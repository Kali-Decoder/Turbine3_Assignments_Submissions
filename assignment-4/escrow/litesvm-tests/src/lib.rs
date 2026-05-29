use anchor_lang::{InstructionData, ToAccountMetas};
use litesvm::{types::TransactionResult, LiteSVM};
use solana_sdk::{
    instruction::Instruction,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use solana_system_interface::{instruction as system_instruction, program as system_program};
use spl_associated_token_account::{
    get_associated_token_address_with_program_id,
    instruction::create_associated_token_account,
};
use spl_token::{
    instruction::{initialize_mint2, mint_to},
    state::{Account as TokenAccount, Mint as TokenMint},
};

const PROGRAM_SO: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../target/deploy/escrow.so"
));

pub struct Env {
    pub svm: LiteSVM,
    pub payer: Keypair,
}

impl Env {
    pub fn new() -> Self {
        let mut svm = LiteSVM::new();
        svm.add_program(escrow::ID, PROGRAM_SO)
            .expect("failed to load escrow program");
        let payer = Keypair::new();
        svm.airdrop(&payer.pubkey(), 100_000_000_000).unwrap();
        Self { svm, payer }
    }

    pub fn fund(&mut self, account: &Pubkey, lamports: u64) {
        self.svm.airdrop(account, lamports).unwrap();
    }

    pub fn send(&mut self, ixs: &[Instruction], signers: &[&Keypair]) -> TransactionResult {
        let payer_pk = self.payer.pubkey();
        let mut all: Vec<&Keypair> = vec![&self.payer];
        all.extend_from_slice(signers);
        let tx = Transaction::new_signed_with_payer(
            ixs,
            Some(&payer_pk),
            &all,
            self.svm.latest_blockhash(),
        );
        self.svm.send_transaction(tx)
    }
}

pub fn create_mint(env: &mut Env, authority: &Pubkey, decimals: u8) -> Pubkey {
    let mint = Keypair::new();
    let rent = env
        .svm
        .minimum_balance_for_rent_exemption(TokenMint::LEN);

    let create_ix = system_instruction::create_account(
        &env.payer.pubkey(),
        &mint.pubkey(),
        rent,
        TokenMint::LEN as u64,
        &spl_token::ID,
    );
    let init_ix =
        initialize_mint2(&spl_token::ID, &mint.pubkey(), authority, None, decimals).unwrap();

    env.send(&[create_ix, init_ix], &[&mint]).unwrap();
    mint.pubkey()
}

pub fn create_ata(env: &mut Env, owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    let ata = get_associated_token_address_with_program_id(owner, mint, &spl_token::ID);
    if env.svm.get_account(&ata).is_some_and(|a| !a.data.is_empty()) {
        return ata;
    }
    let ix = create_associated_token_account(&env.payer.pubkey(), owner, mint, &spl_token::ID);
    env.send(&[ix], &[]).unwrap();
    ata
}

pub fn mint_to_account(
    env: &mut Env,
    mint: &Pubkey,
    authority: &Keypair,
    to: &Pubkey,
    amount: u64,
) {
    let ix = mint_to(&spl_token::ID, mint, to, &authority.pubkey(), &[], amount).unwrap();
    env.send(&[ix], &[authority]).unwrap();
}

pub fn balance_of(env: &Env, ata: &Pubkey) -> u64 {
    let acc = env.svm.get_account(ata).expect("token account missing");
    TokenAccount::unpack(&acc.data).unwrap().amount
}

pub fn escrow_pda(maker: &Pubkey, seed: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"escrow", maker.as_ref(), &seed.to_le_bytes()],
        &escrow::ID,
    )
}

pub fn vault_pda(escrow_account: &Pubkey, mint_a: &Pubkey) -> Pubkey {
    get_associated_token_address_with_program_id(escrow_account, mint_a, &spl_token::ID)
}

pub fn make_ix(
    maker: &Pubkey,
    mint_a: &Pubkey,
    mint_b: &Pubkey,
    seed: u64,
    deposit: u64,
    receive: u64,
) -> Instruction {
    let (escrow_pk, _) = escrow_pda(maker, seed);
    let vault = vault_pda(&escrow_pk, mint_a);
    let maker_ata_a =
        get_associated_token_address_with_program_id(maker, mint_a, &spl_token::ID);

    let accounts = escrow::accounts::Make {
        maker: *maker,
        mint_a: *mint_a,
        mint_b: *mint_b,
        maker_ata_a,
        escrow: escrow_pk,
        vault,
        system_program: system_program::ID,
        token_program: spl_token::ID,
        associated_token_program: spl_associated_token_account::ID,
    }
    .to_account_metas(None);

    Instruction {
        program_id: escrow::ID,
        accounts,
        data: escrow::instruction::Make {
            seed,
            deposit,
            receive,
        }
        .data(),
    }
}

pub fn take_ix(
    taker: &Pubkey,
    maker: &Pubkey,
    mint_a: &Pubkey,
    mint_b: &Pubkey,
    seed: u64,
) -> Instruction {
    let (escrow_pk, _) = escrow_pda(maker, seed);
    let vault = vault_pda(&escrow_pk, mint_a);

    let accounts = escrow::accounts::Take {
        taker: *taker,
        maker: *maker,
        mint_a: *mint_a,
        mint_b: *mint_b,
        taker_ata_a: get_associated_token_address_with_program_id(taker, mint_a, &spl_token::ID),
        taker_ata_b: get_associated_token_address_with_program_id(taker, mint_b, &spl_token::ID),
        maker_ata_b: get_associated_token_address_with_program_id(maker, mint_b, &spl_token::ID),
        escrow: escrow_pk,
        vault,
        system_program: system_program::ID,
        token_program: spl_token::ID,
        associated_token_program: spl_associated_token_account::ID,
    }
    .to_account_metas(None);

    Instruction {
        program_id: escrow::ID,
        accounts,
        data: escrow::instruction::Take {}.data(),
    }
}

pub fn refund_ix(maker: &Pubkey, mint_a: &Pubkey, seed: u64) -> Instruction {
    let (escrow_pk, _) = escrow_pda(maker, seed);
    let vault = vault_pda(&escrow_pk, mint_a);

    let accounts = escrow::accounts::Refund {
        maker: *maker,
        mint_a: *mint_a,
        maker_ata_a: get_associated_token_address_with_program_id(
            maker,
            mint_a,
            &spl_token::ID,
        ),
        escrow: escrow_pk,
        vault,
        token_program: spl_token::ID,
        associated_token_program: spl_associated_token_account::ID,
    }
    .to_account_metas(None);

    Instruction {
        program_id: escrow::ID,
        accounts,
        data: escrow::instruction::Refund {}.data(),
    }
}
