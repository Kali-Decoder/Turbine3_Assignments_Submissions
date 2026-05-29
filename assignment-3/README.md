# anchor_vault — Kali-Decoder Submission

Week 3 homework for **Kali-Decoder**. This is a small Anchor program where each user can open a personal SOL vault, deposit into it, withdraw from it, and close it.

If you're new to Anchor, the goal of this README is to walk you through the program end to end so you actually understand *why* each piece is there, not just copy it. Tests are in Rust using LiteSVM, and this write-up is formatted for the **Kali-Decoder** submission.

---

## Stuff you should know first

A quick refresher on the Solana concepts this program uses. Skip if you're already comfortable.

- **Account.** Everything on Solana is an account. An account has an owner (a program), some lamports (SOL), and some data bytes.
- **Lamport.** 1 SOL = 1,000,000,000 lamports. The chain only talks in lamports.
- **System Program.** A built-in program at address `11111111111111111111111111111111`. It's the one that creates accounts and moves lamports between System-owned accounts. Anytime you "transfer SOL", you're calling the System Program.
- **PDA (Program Derived Address).** An address that isn't a real keypair. It's derived deterministically from some seeds + a program ID. You can't sign for it with a private key, but the program that "owns" it can sign for it by passing the seeds (this is called *signer seeds*). PDAs are how programs control accounts.
- **Bump.** PDAs are found by trying numbers 255, 254, 253... until one gives you an address that's not on the ed25519 curve. That winning number is the bump. We save it so we don't have to recompute next time.
- **CPI (Cross-Program Invocation).** When one program calls another. Our vault program calls the System Program via CPI to actually move lamports.
- **Rent.** Accounts have to keep a minimum SOL balance to stay alive. When you close an account, that rent gets refunded.

## What this program does

Two PDAs per user.

```
state PDA   seeds: [b"state", user_pubkey]
vault PDA   seeds: [b"vault", state_pubkey]
```

`state` is owned by our program and only stores the two bumps (`state_bump`, `vault_bump`). It's tiny on purpose. Storing the bumps means we don't have to call `find_program_address` (which is expensive on-chain) every time we need them.

`vault` is owned by the **System Program**, not ours. This is the trick that makes the whole thing simple. Because the vault is a plain System account, the System Program is happy to move lamports in and out of it. We just need someone who can sign for it. The user can sign for *into* the vault (it's just a normal transfer to that address). For *out of* the vault, the vault PDA itself signs, using its bump.

The four instructions:

| Instruction | What happens |
|---|---|
| `initialize` | Creates the `state` account, saves both bumps inside it. |
| `deposit(amount)` | CPI to System Program: transfer `amount` lamports from user to vault. User signs. |
| `withdraw(amount)` | CPI to System Program: transfer `amount` lamports from vault to user. Vault PDA signs using its bump seeds. |
| `close` | Drains the vault entirely back to the user, then closes the `state` account so the user gets the rent back too. |

## Walking through the program

Open `programs/anchor_vault/src/lib.rs` while you read this.

### The account types

```rust
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init,
        payer = user,
        seeds = [b"state", user.key().as_ref()],
        bump,
        space = 8 + VaultState::INIT_SPACE,
    )]
    pub state: Account<'info, VaultState>,

    #[account(
        mut,
        seeds = [b"vault", state.key().as_ref()],
        bump,
    )]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}
```

Things to notice:

- `Signer<'info>` means we're verifying this account signed the transaction. Marked `mut` because the user pays rent and pays the tx fee, so their balance changes.
- `init` does three things: creates the account, has `payer` cover rent, and sets the program (our program) as the owner. `space = 8 + VaultState::INIT_SPACE` is because every Anchor account starts with an 8-byte discriminator (Anchor uses this to know what type the bytes represent), then `INIT_SPACE` for the actual fields. The `#[derive(InitSpace)]` macro on `VaultState` computes that for us.
- `seeds = [...]` + `bump` (with no value) tells Anchor "this should be a PDA, find the canonical bump yourself and pass it to me as `ctx.bumps.state`".
- `SystemAccount<'info>` means "any account currently owned by the System Program". The vault doesn't exist yet at init time, but that's fine, we're not initializing it here. We just declare its address so we *could* deposit to it later. It exists implicitly the moment lamports land in it.
- `Program<'info, System>` is a typed reference to the System Program. Anchor will check the address matches `11111111111111111111111111111111` automatically.

The `Payment` and `Close` structs look similar but pass `bump = state.state_bump` instead of letting Anchor recompute. That's the speedup we get from having stored the bump.

### initialize

```rust
pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
    ctx.accounts.state.state_bump = ctx.bumps.state;
    ctx.accounts.state.vault_bump = ctx.bumps.vault;
    Ok(())
}
```

All it does is save both bumps into the freshly created state account. Anchor took care of actually creating the account, paying rent, and setting up the discriminator.

### deposit

```rust
pub fn deposit(ctx: Context<Payment>, amount: u64) -> Result<()> {
    let cpi_accounts = Transfer {
        from: ctx.accounts.user.to_account_info(),
        to: ctx.accounts.vault.to_account_info(),
    };
    let cpi = CpiContext::new(ctx.accounts.system_program.to_account_info(), cpi_accounts);
    transfer(cpi, amount)
}
```

This is a CPI to the System Program's `transfer`. `CpiContext::new` is for cases where the signer is already on the outer transaction (the user signed the tx, so they don't need to sign again for the inner call). That's why no signer seeds here.

### withdraw

```rust
pub fn withdraw(ctx: Context<Payment>, amount: u64) -> Result<()> {
    let state_key = ctx.accounts.state.key();
    let seeds: &[&[u8]] = &[
        b"vault",
        state_key.as_ref(),
        &[ctx.accounts.state.vault_bump],
    ];
    let signer = &[seeds];

    let cpi_accounts = Transfer {
        from: ctx.accounts.vault.to_account_info(),
        to: ctx.accounts.user.to_account_info(),
    };
    let cpi = CpiContext::new_with_signer(
        ctx.accounts.system_program.to_account_info(),
        cpi_accounts,
        signer,
    );
    transfer(cpi, amount)
}
```

Same idea as deposit but the direction is reversed, so the *vault* is the one losing lamports. Since the vault is a PDA and no human signed for it, we need to provide signer seeds so the runtime can verify that the program is allowed to sign for that exact address.

The shape `&[seeds]` looks weird because `with_signer` takes a slice of seed-sets, one per PDA signer. We only have one, so it's a slice with one element.

The order of seeds matters: `[b"vault", state_key, bump]` is the same order as the `seeds = [b"vault", state.key().as_ref()]` annotation (with the bump appended at the end). If you mismatch them, the runtime will refuse the CPI.

### close

```rust
pub fn close(ctx: Context<Close>) -> Result<()> {
    let state_key = ctx.accounts.state.key();
    let bump = ctx.accounts.state.vault_bump;
    let seeds: &[&[u8]] = &[b"vault", state_key.as_ref(), &[bump]];
    let signer = &[seeds];

    let lamports = ctx.accounts.vault.lamports();
    let cpi_accounts = Transfer {
        from: ctx.accounts.vault.to_account_info(),
        to: ctx.accounts.user.to_account_info(),
    };
    let cpi = CpiContext::new_with_signer(
        ctx.accounts.system_program.to_account_info(),
        cpi_accounts,
        signer,
    );
    transfer(cpi, lamports)
}
```

Two things happen on close. The function body drains whatever's in the vault back to the user. The `state` account doesn't need its own code, because of this constraint on the accounts struct:

```rust
#[account(
    mut,
    close = user,
    ...
)]
pub state: Account<'info, VaultState>,
```

`close = user` tells Anchor: "after this instruction finishes, zero out this account's data, set its owner back to the System Program, and send all its lamports (the rent we paid at init) to `user`". So the user gets both their deposited SOL and their rent back.

## How the tests work

Tests live in `tests-litesvm/tests/vault.rs`. They use [LiteSVM](https://github.com/LiteSVM/litesvm), which is an in-process Solana runtime. No validator, no localnet, no `anchor test` overhead. Each test runs in well under a second.

The flow:

1. `new_svm()` boots a fresh LiteSVM and loads our compiled `.so` file into it at our program's address.
2. We make a new user keypair and airdrop 100 SOL to them.
3. We derive the two PDAs with `Pubkey::find_program_address` (same seeds we used in the program).
4. We build an `Instruction` by hand. Anchor's `#[program]` macro generated two helpful modules for us:
   - `anchor_vault::accounts::Initialize { ... }` is a struct with the right fields, which we call `.to_account_metas(None)` on to get the list of account metas.
   - `anchor_vault::instruction::Initialize {}` is the instruction's *data* struct, which we call `.data()` on to get the serialized bytes (discriminator + args).
5. We send the transaction through LiteSVM and assert on balances / account state.

We're not using `anchor-client` or parsing the IDL. The program crate is added as a dependency with the `no-entrypoint` feature, which gives us those generated types but skips the on-chain entrypoint code.

The four tests cover one instruction each:

- `init_writes_both_bumps` — calls initialize, deserializes the `state` account, checks the bumps match what `find_program_address` returns.
- `deposit_credits_vault` — deposits 2 SOL, checks the vault's balance went up by exactly 2 SOL.
- `withdraw_pays_user_back` — deposits 5, withdraws 2, checks the vault dropped by exactly 2 SOL and the user got ~2 SOL back (less the ~5000 lamport tx fee).
- `close_drains_everything` — deposits 3 SOL, closes, checks the state account is gone and the vault is at 0.

## Setup

You'll need:

- [Rust](https://rustup.rs/) (the repo pins `1.89.0` via `rust-toolchain.toml`, but a newer stable is also fine)
- [Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools), 2.x or 3.x
- [Anchor CLI](https://www.anchor-lang.com/docs/installation) `0.32.1` (other versions might compile but the test crate is pinned for 0.32.1)

What I ran with:

```
anchor-cli 0.32.1
solana-cli 3.1.14
rustc 1.95.0
litesvm 0.7.1
```

If `anchor --version` shows something else, install the matching version with `avm`:

```
cargo install --git https://github.com/coral-xyz/anchor avm --force
avm install 0.32.1
avm use 0.32.1
```

## Compile the contract

```
anchor build
```

This compiles the program to BPF and drops two files into `target/deploy/`:

- `anchor_vault.so` — the compiled program. The tests load this directly.
- `anchor_vault-keypair.json` — the program's deploy keypair. Its public key is what `declare_id!(...)` is set to in `lib.rs`.

If you ever change `declare_id!` and re-build, run `anchor keys sync` to keep them aligned.

## Deploy the contract

For a local deployment, start a validator in one terminal and deploy from another:

```bash
solana-test-validator
anchor deploy
```

If you change the program id or rebuild the program, run `anchor build` again before deploying.

If you want to deploy to a different cluster, update the provider cluster in `Anchor.toml` first, then run `anchor deploy` again.

## My deployment

These are the values from my successful deployment for **Kali-Decoder**:

- Program Id: `AmGUxUaTS89qUjB3mXzX6HXHKwZaC5fcSKxEeQYDeDnT`
- Signature: `5yMX9rs8GLEuFiueRt6mQTXq23xFDU5W9v1e8Z7zKQKYhcQJW6eZ8Q2UhtGCuNsQ1ANLTQJBPFocbtsKtvKvsxUH`
- Status: confirmed on-chain

## Run the tests

```
cargo test -p tests-litesvm --tests
```

You should see:

```
running 4 tests
test init_writes_both_bumps ... ok
test deposit_credits_vault ... ok
test withdraw_pays_user_back ... ok
test close_drains_everything ... ok

test result: ok. 4 passed; 0 failed
```

Screenshot of this on my machine:

![Tests passing](docs/tests-passing.png)

## Common things that trip beginners up

- **Wrong bump on signer seeds.** If you compute the bump with one seed order in the program and a different order in `with_signer`, the CPI fails with `Cross-program invocation with unauthorized signer`. The seeds in `with_signer` must match the `seeds = [...]` on the accounts struct, exactly, with the bump appended at the end.
- **Forgetting the discriminator in `space`.** Every Anchor account needs `8 + (size of fields)`. If you forget the `8`, you'll silently overwrite the discriminator and deserialization breaks.
- **`mut` on the wrong accounts.** Any account whose lamports or data change has to be marked `mut`. User pays fees -> mut. Vault gets/loses lamports -> mut. State is being created/written/closed -> mut.
- **Mixing solana-sdk v2 and v3.** Anchor 0.32 pulls in solana v2.x crates. Newer LiteSVM (0.8+) pulls in v3.x. They don't interop, so the test crate pins `litesvm = "0.7"` and `solana-sdk = "2.3"`. If you upgrade Anchor to 1.0+, you can bump LiteSVM too.
- **Vault is a SystemAccount, not an Account.** Don't try to make it `Account<'info, Something>`. If our program owned the vault, the System Program would refuse to transfer lamports out of it, because programs can only debit accounts they own. By keeping it System-owned, the System Program is happy to move lamports in and out.

## Layout

```
.
├── Anchor.toml                     anchor config
├── Cargo.toml                      workspace: program + test crate
├── programs/
│   └── anchor_vault/
│       ├── Cargo.toml
│       └── src/lib.rs              the program
└── tests-litesvm/
    ├── Cargo.toml
    ├── src/lib.rs                  empty, just so the crate has a lib target
    └── tests/vault.rs              4 integration tests
```

## Where to go next

If you want to extend this for practice:

- Add an authority field to `VaultState` so anyone can deposit but only the owner can withdraw.
- Swap SOL for an SPL token (you'd use `anchor-spl` and a token vault instead of a SystemAccount).
- Add a per-vault withdraw limit and an error type for over-withdraw.
- Add a fuzz test that does random sequences of deposits / withdraws and checks the balance invariant.
