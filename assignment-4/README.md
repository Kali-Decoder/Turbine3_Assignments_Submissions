# Anchor Escrow — Kali-Decoder Submission

This is my **Turbin3 Assignment 4** submission for **Kali-Decoder**.

It is a two-party SPL token escrow on Solana. The maker locks a `deposit` of token A into a program-owned vault and names the price (`receive` of token B). A taker who pays the price gets token A; otherwise, the maker can refund the escrow.

The Anchor workspace lives in `escrow/`. All commands below assume you run them from that folder.

## What this program does

| Instruction | Who calls it | What it does |
|---|---|---|
| `make` | Maker | Creates the `Escrow` PDA, opens the vault ATA, transfers the deposit into the vault, and stores the price to receive. |
| `take` | Taker | Pays the maker, receives the vault balance, and closes both the vault and the `Escrow` account. |
| `refund` | Maker | Returns the vault balance to the maker and closes both the vault and the `Escrow` account. |

The `Escrow` PDA is derived from:

```text
[b"escrow", maker.key().as_ref(), seed.to_le_bytes().as_ref()]
```

That allows one maker to run multiple escrows at the same time by changing the seed.

## Project layout

```text
escrow/
  programs/escrow/
    src/lib.rs          program entrypoint
    src/state.rs        escrow state
    src/error.rs        error codes
    src/instructions/   make / take / refund

  litesvm-tests/
    src/lib.rs          shared test harness
    tests/escrow.rs     integration tests
```

## Setup

Before running anything, make sure you have:

- Rust `1.89.0`
- Anchor CLI `0.32.1`
- Solana CLI installed
- `yarn` available

If you need to match the pinned Anchor version:

```bash
cargo install --git https://github.com/coral-xyz/anchor avm --force
avm install 0.32.1
avm use 0.32.1
```

## Install dependencies

```bash
cd /Users/nikku.jr.dev/Downloads/Turbin3_assignments/assignment-4/escrow
yarn install
```

## Compile the contract

```bash
RUSTUP_TOOLCHAIN=1.89.0 anchor build
```

This builds the on-chain program and creates the deploy artifacts in `target/deploy/`.

If you change the program ID, run:

```bash
anchor keys sync
```

## Run the tests

```bash
RUSTUP_TOOLCHAIN=1.89.0 anchor test
```

That runs the LiteSVM test crate through `Anchor.toml`.

If you prefer the underlying command directly from `escrow/`:

```bash
RUSTUP_TOOLCHAIN=1.89.0 cargo test -p escrow-litesvm-tests --tests -- --nocapture
```

## Deploy the contract

For a local deployment:

```bash
solana-test-validator
anchor deploy
```

## My assignment details

These are the values from my **Kali-Decoder** submission:

- Program Id: `DtdLAran5oTruCmrvFXKGskXeYvo5EfceQebq4MbWnyZ`
- Signature: `28N4ACjGjiXaXwikVyNWWvx6rB4YVdYgMVAj4zwyBYwkuBwPiSadvLtRyVzpBxRqADgrUPEbVXNRUQ9pL1Vn51oe`
- Status: confirmed on-chain
- Rust toolchain: `1.89.0`
- Anchor CLI: `0.32.1`

## Notes

- `anchor build` must succeed before the tests can load the program binary.
- Always prefix build/test commands with `RUSTUP_TOOLCHAIN=1.89.0` if Anchor or Cargo falls back to `rustc 1.88.0`.
- If you see a `rustc 1.88.0` error, run `rustup override set 1.89.0` inside `escrow/` and open a fresh shell before rebuilding.
- The tests use LiteSVM, so they run without a live validator once the program is built.
- This README is written for the Kali-Decoder submission and does not use the old Janhavi branding.

## Result

When everything is working, you should see the escrow tests pass and the program compile cleanly for submission.
