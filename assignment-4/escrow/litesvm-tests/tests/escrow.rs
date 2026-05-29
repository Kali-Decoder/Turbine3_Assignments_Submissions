use escrow_litesvm_tests::*;
use solana_sdk::signature::{Keypair, Signer};

const DECIMALS_A: u8 = 6;
const DECIMALS_B: u8 = 9;

#[allow(dead_code)]
struct Parties {
    maker: Keypair,
    taker: Keypair,
    mint_a: solana_sdk::pubkey::Pubkey,
    mint_b: solana_sdk::pubkey::Pubkey,
    mint_authority: Keypair,
}

fn bootstrap(env: &mut Env, maker_a: u64, taker_b: u64) -> Parties {
    let maker = Keypair::new();
    let taker = Keypair::new();
    env.fund(&maker.pubkey(), 10_000_000_000);
    env.fund(&taker.pubkey(), 10_000_000_000);

    let mint_authority = Keypair::new();
    env.fund(&mint_authority.pubkey(), 1_000_000_000);

    let mint_a = create_mint(env, &mint_authority.pubkey(), DECIMALS_A);
    let mint_b = create_mint(env, &mint_authority.pubkey(), DECIMALS_B);

    if maker_a > 0 {
        let maker_ata_a = create_ata(env, &maker.pubkey(), &mint_a);
        mint_to_account(env, &mint_a, &mint_authority, &maker_ata_a, maker_a);
    }
    if taker_b > 0 {
        let taker_ata_b = create_ata(env, &taker.pubkey(), &mint_b);
        mint_to_account(env, &mint_b, &mint_authority, &taker_ata_b, taker_b);
    }

    Parties {
        maker,
        taker,
        mint_a,
        mint_b,
        mint_authority,
    }
}

#[test]
fn make_locks_deposit_in_vault() {
    let mut env = Env::new();
    let p = bootstrap(&mut env, 1_000_000, 0);

    let seed = 42u64;
    let deposit = 750_000u64;
    let receive = 2_000_000u64;

    let ix = make_ix(
        &p.maker.pubkey(),
        &p.mint_a,
        &p.mint_b,
        seed,
        deposit,
        receive,
    );
    env.send(&[ix], &[&p.maker]).unwrap();

    let (escrow_pk, _) = escrow_pda(&p.maker.pubkey(), seed);
    let vault = vault_pda(&escrow_pk, &p.mint_a);
    assert_eq!(balance_of(&env, &vault), deposit);

    let maker_ata_a = spl_associated_token_account::get_associated_token_address_with_program_id(
        &p.maker.pubkey(),
        &p.mint_a,
        &spl_token::ID,
    );
    assert_eq!(balance_of(&env, &maker_ata_a), 1_000_000 - deposit);

    let escrow_data = env.svm.get_account(&escrow_pk).unwrap().data;
    let escrow = decode_escrow(&escrow_data);
    assert_eq!(escrow.seed, seed);
    assert_eq!(escrow.maker, p.maker.pubkey());
    assert_eq!(escrow.mint_a, p.mint_a);
    assert_eq!(escrow.mint_b, p.mint_b);
    assert_eq!(escrow.receive, receive);
}

#[test]
fn make_rejects_zero_deposit() {
    let mut env = Env::new();
    let p = bootstrap(&mut env, 1_000_000, 0);

    let ix = make_ix(&p.maker.pubkey(), &p.mint_a, &p.mint_b, 1, 0, 1_000);
    let err = env.send(&[ix], &[&p.maker]).unwrap_err();
    assert!(
        format!("{:?}", err).contains("ZeroAmount"),
        "expected ZeroAmount in error, got: {:?}",
        err
    );
}

#[test]
fn make_rejects_zero_receive() {
    let mut env = Env::new();
    let p = bootstrap(&mut env, 1_000_000, 0);

    let ix = make_ix(&p.maker.pubkey(), &p.mint_a, &p.mint_b, 2, 100, 0);
    env.send(&[ix], &[&p.maker]).unwrap_err();
}

#[test]
fn take_swaps_balances_and_closes_escrow() {
    let mut env = Env::new();
    let p = bootstrap(&mut env, 5_000_000, 4_000_000);

    let seed = 7u64;
    let deposit = 3_000_000u64;
    let receive = 2_500_000u64;

    let make = make_ix(
        &p.maker.pubkey(),
        &p.mint_a,
        &p.mint_b,
        seed,
        deposit,
        receive,
    );
    env.send(&[make], &[&p.maker]).unwrap();

    let take = take_ix(
        &p.taker.pubkey(),
        &p.maker.pubkey(),
        &p.mint_a,
        &p.mint_b,
        seed,
    );
    env.send(&[take], &[&p.taker]).unwrap();

    let taker_ata_a = spl_associated_token_account::get_associated_token_address_with_program_id(
        &p.taker.pubkey(),
        &p.mint_a,
        &spl_token::ID,
    );
    let maker_ata_b = spl_associated_token_account::get_associated_token_address_with_program_id(
        &p.maker.pubkey(),
        &p.mint_b,
        &spl_token::ID,
    );
    let taker_ata_b = spl_associated_token_account::get_associated_token_address_with_program_id(
        &p.taker.pubkey(),
        &p.mint_b,
        &spl_token::ID,
    );

    assert_eq!(balance_of(&env, &taker_ata_a), deposit);
    assert_eq!(balance_of(&env, &maker_ata_b), receive);
    assert_eq!(balance_of(&env, &taker_ata_b), 4_000_000 - receive);

    let (escrow_pk, _) = escrow_pda(&p.maker.pubkey(), seed);
    let vault = vault_pda(&escrow_pk, &p.mint_a);
    assert!(env.svm.get_account(&escrow_pk).is_none() || env.svm.get_account(&escrow_pk).unwrap().lamports == 0);
    assert!(env.svm.get_account(&vault).is_none() || env.svm.get_account(&vault).unwrap().lamports == 0);
}

#[test]
fn take_fails_when_taker_lacks_funds() {
    let mut env = Env::new();
    let p = bootstrap(&mut env, 5_000_000, 100);

    let seed = 11u64;
    let make = make_ix(
        &p.maker.pubkey(),
        &p.mint_a,
        &p.mint_b,
        seed,
        1_000_000,
        500_000,
    );
    env.send(&[make], &[&p.maker]).unwrap();

    let take = take_ix(
        &p.taker.pubkey(),
        &p.maker.pubkey(),
        &p.mint_a,
        &p.mint_b,
        seed,
    );
    env.send(&[take], &[&p.taker]).unwrap_err();
}

#[test]
fn refund_returns_deposit_and_closes_escrow() {
    let mut env = Env::new();
    let p = bootstrap(&mut env, 1_500_000, 0);

    let seed = 99u64;
    let deposit = 900_000u64;
    let make = make_ix(
        &p.maker.pubkey(),
        &p.mint_a,
        &p.mint_b,
        seed,
        deposit,
        1,
    );
    env.send(&[make], &[&p.maker]).unwrap();

    let refund = refund_ix(&p.maker.pubkey(), &p.mint_a, seed);
    env.send(&[refund], &[&p.maker]).unwrap();

    let maker_ata_a = spl_associated_token_account::get_associated_token_address_with_program_id(
        &p.maker.pubkey(),
        &p.mint_a,
        &spl_token::ID,
    );
    assert_eq!(balance_of(&env, &maker_ata_a), 1_500_000);

    let (escrow_pk, _) = escrow_pda(&p.maker.pubkey(), seed);
    let vault = vault_pda(&escrow_pk, &p.mint_a);
    assert!(env.svm.get_account(&escrow_pk).is_none() || env.svm.get_account(&escrow_pk).unwrap().lamports == 0);
    assert!(env.svm.get_account(&vault).is_none() || env.svm.get_account(&vault).unwrap().lamports == 0);
}

#[test]
fn refund_rejects_non_maker() {
    let mut env = Env::new();
    let p = bootstrap(&mut env, 1_000_000, 0);

    let seed = 5u64;
    let make = make_ix(&p.maker.pubkey(), &p.mint_a, &p.mint_b, seed, 100_000, 200_000);
    env.send(&[make], &[&p.maker]).unwrap();

    let imposter = Keypair::new();
    env.fund(&imposter.pubkey(), 1_000_000_000);

    // The PDA seeds include the maker, so the imposter derives a different
    // (non-existent) escrow account; the tx must fail.
    let ix = refund_ix(&imposter.pubkey(), &p.mint_a, seed);
    env.send(&[ix], &[&imposter]).unwrap_err();
}

#[test]
fn maker_can_run_multiple_escrows_with_different_seeds() {
    let mut env = Env::new();
    let p = bootstrap(&mut env, 5_000_000, 0);

    for (seed, deposit) in [(1u64, 100_000u64), (2, 200_000), (3, 300_000)] {
        let ix = make_ix(
            &p.maker.pubkey(),
            &p.mint_a,
            &p.mint_b,
            seed,
            deposit,
            1,
        );
        env.send(&[ix], &[&p.maker]).unwrap();

        let (escrow_pk, _) = escrow_pda(&p.maker.pubkey(), seed);
        let vault = vault_pda(&escrow_pk, &p.mint_a);
        assert_eq!(balance_of(&env, &vault), deposit);
    }
}

fn decode_escrow(data: &[u8]) -> escrow::state::Escrow {
    use anchor_lang::AccountDeserialize;
    let mut slice = data;
    escrow::state::Escrow::try_deserialize(&mut slice).unwrap()
}
