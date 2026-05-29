use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use anchor_vault::{
    accounts as vault_accounts, instruction as vault_ix, VaultState, ID as PROGRAM_ID,
};
use litesvm::LiteSVM;
#[allow(deprecated)]
use solana_sdk::system_program;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

const SOL: u64 = 1_000_000_000;

fn so_path() -> String {
    format!(
        "{}/../target/deploy/anchor_vault.so",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn new_svm() -> LiteSVM {
    let mut svm = LiteSVM::new();
    svm.add_program_from_file(PROGRAM_ID, so_path())
        .expect("run `anchor build` first");
    svm
}

fn pdas(user: &Pubkey) -> (Pubkey, Pubkey) {
    let (state, _) = Pubkey::find_program_address(&[b"state", user.as_ref()], &PROGRAM_ID);
    let (vault, _) = Pubkey::find_program_address(&[b"vault", state.as_ref()], &PROGRAM_ID);
    (state, vault)
}

fn send_tx(svm: &mut LiteSVM, payer: &Keypair, ix: Instruction) {
    let bh = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[payer], bh);
    svm.send_transaction(tx).unwrap();
}

fn initialize(svm: &mut LiteSVM, user: &Keypair, state: Pubkey, vault: Pubkey) {
    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vault_accounts::Initialize {
            user: user.pubkey(),
            state,
            vault,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: vault_ix::Initialize {}.data(),
    };
    send_tx(svm, user, ix);
}

#[test]
fn init_writes_both_bumps() {
    let mut svm = new_svm();
    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 100 * SOL).unwrap();
    let (state, vault) = pdas(&user.pubkey());

    initialize(&mut svm, &user, state, vault);

    let acc = svm.get_account(&state).expect("state account exists");
    assert_eq!(acc.owner, PROGRAM_ID);

    let parsed = VaultState::try_deserialize(&mut acc.data.as_slice()).unwrap();
    let (_, sb) = Pubkey::find_program_address(&[b"state", user.pubkey().as_ref()], &PROGRAM_ID);
    let (_, vb) = Pubkey::find_program_address(&[b"vault", state.as_ref()], &PROGRAM_ID);
    assert_eq!(parsed.state_bump, sb);
    assert_eq!(parsed.vault_bump, vb);
}

#[test]
fn deposit_credits_vault() {
    let mut svm = new_svm();
    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 100 * SOL).unwrap();
    let (state, vault) = pdas(&user.pubkey());
    initialize(&mut svm, &user, state, vault);

    let amount = 2 * SOL;
    let before = svm.get_balance(&vault).unwrap_or(0);

    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vault_accounts::Payment {
            user: user.pubkey(),
            state,
            vault,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: vault_ix::Deposit { amount }.data(),
    };
    send_tx(&mut svm, &user, ix);

    assert_eq!(svm.get_balance(&vault).unwrap(), before + amount);
}

#[test]
fn withdraw_pays_user_back() {
    let mut svm = new_svm();
    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 100 * SOL).unwrap();
    let (state, vault) = pdas(&user.pubkey());
    initialize(&mut svm, &user, state, vault);

    // deposit 5 SOL
    let deposit_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vault_accounts::Payment {
            user: user.pubkey(),
            state,
            vault,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: vault_ix::Deposit { amount: 5 * SOL }.data(),
    };
    send_tx(&mut svm, &user, deposit_ix);

    let user_before = svm.get_balance(&user.pubkey()).unwrap();
    let vault_before = svm.get_balance(&vault).unwrap();

    // pull 2 SOL back out
    let withdraw_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vault_accounts::Payment {
            user: user.pubkey(),
            state,
            vault,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: vault_ix::Withdraw { amount: 2 * SOL }.data(),
    };
    send_tx(&mut svm, &user, withdraw_ix);

    let user_after = svm.get_balance(&user.pubkey()).unwrap();
    let vault_after = svm.get_balance(&vault).unwrap();

    assert_eq!(vault_after, vault_before - 2 * SOL);
    // user got back ~2 SOL, minus the tx fee
    let gained = user_after - user_before;
    assert!(gained <= 2 * SOL);
    assert!(gained > 2 * SOL - 10_000);
}

#[test]
fn close_drains_everything() {
    let mut svm = new_svm();
    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 100 * SOL).unwrap();
    let (state, vault) = pdas(&user.pubkey());
    initialize(&mut svm, &user, state, vault);

    let deposit_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vault_accounts::Payment {
            user: user.pubkey(),
            state,
            vault,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: vault_ix::Deposit { amount: 3 * SOL }.data(),
    };
    send_tx(&mut svm, &user, deposit_ix);

    let close_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vault_accounts::Close {
            user: user.pubkey(),
            state,
            vault,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: vault_ix::Close {}.data(),
    };
    send_tx(&mut svm, &user, close_ix);

    let state_acc = svm.get_account(&state);
    let closed = match state_acc {
        None => true,
        Some(a) => a.lamports == 0 && a.data.is_empty(),
    };
    assert!(closed);
    assert_eq!(svm.get_balance(&vault).unwrap_or(0), 0);
}
