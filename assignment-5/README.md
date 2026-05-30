# AMM — Kali-Decoder Submission

This is my **Turbin3 Assignment 5** submission for **Kali-Decoder**.

It is a constant-product Automated Market Maker built with Anchor. The program lets anyone initialize a two-mint pool, deposit liquidity in exchange for LP tokens, swap between the two sides with a configurable fee, withdraw a pro-rata share of reserves, and optionally lock or unlock trading through an authority-gated flag.

The on-chain program lives in `programs/amm/`, and the integration tests live in `tests/amm.ts`.

## What this program does

| Instruction | What it does |
| --- | --- |
| `initialize(seed, fee_bps, authority)` | Creates the `Config` PDA, the LP mint PDA, and both vault associated token accounts owned by the config PDA. The fee must be below `10_000` bps. |
| `deposit(amount_a, max_b, min_lp)` | Deposits `amount_a` of mint A and the matching amount of mint B into the vaults, then mints LP tokens. The first deposit bootstraps the pool with `sqrt(a * b)` LP. |
| `withdraw(lp_amount, min_a, min_b)` | Burns LP tokens and returns a proportional share of both vaults, while enforcing slippage floors. |
| `swap(amount_in, min_amount_out, a_to_b)` | Swaps one side of the pool for the other using the constant-product curve and the configured fee. |
| `lock()` / `unlock()` | Toggles the pool lock flag. When locked, `deposit`, `withdraw`, and `swap` are all blocked. |

## Repository layout

```text
programs/amm/
  src/
    lib.rs              program entrypoint
    constants.rs        PDA seeds and fee denominator
    curve.rs            LP and swap math + unit tests
    error.rs            custom AmmError codes
    state/config.rs     pool configuration account
    instructions/
      initialize.rs     pool setup
      deposit.rs        add liquidity
      withdraw.rs       remove liquidity
      swap.rs           token swaps
      lock.rs           authority-gated lock controls
tests/amm.ts            TypeScript integration tests
docs/                   passing test output and screenshots
```

## Important PDAs

| PDA | Seeds |
| --- | --- |
| `Config` | `[b"config", seed.to_le_bytes()]` |
| LP mint | `[b"lp", config.key()]` |
| `vault_a`, `vault_b` | Associated token accounts owned by `config` for mint A and mint B |

`Config` stores the pool seed, both mint addresses, the fee, optional authority, lock flag, and both bumps so the instructions do not need to recompute them on every call.

## Curve math

The AMM uses a constant-product curve. For reserves `(rA, rB)` and input `amount_in` on side A, the quoted output on side B is:

```text
amount_in_with_fee = amount_in * (FEE_DENOMINATOR - fee_bps)
amount_out         = (amount_in_with_fee * rB)
                     / (rA * FEE_DENOMINATOR + amount_in_with_fee)
```

`FEE_DENOMINATOR` is `10_000`, so a `fee_bps` value of `30` gives the standard `0.30%` fee.

## Errors

All custom errors live in `programs/amm/src/error.rs`:

`PoolLocked`, `InvalidFee`, `ZeroAmount`, `SlippageExceeded`, `Overflow`, `EmptyReserves`, `Unauthorized`, `IdenticalMints`

## Setup

Before running anything, make sure you have:

- Rust and Cargo
- Solana CLI
- Anchor CLI `0.32.1`
- Node.js and `yarn`

The workspace already pins the program to `Anchor 0.32.1` and uses `yarn` for the TypeScript side.

## Install dependencies

From the repository root:

```bash
cd /Users/nikku.jr.dev/Downloads/Turbin3_assignments/assignment-5
yarn install
anchor build
```

## Run the tests

```bash
anchor test
```

That boots a local validator, deploys the program, and runs the TypeScript suite in `tests/amm.ts`.

You can also run the pure Rust math tests on their own:

```bash
cargo test --manifest-path programs/amm/Cargo.toml --lib
```

## Test coverage

The test suite covers the full pool lifecycle:

- pool initialization
- identical-mint rejection
- fee validation
- initial liquidity bootstrap
- proportional second deposits
- deposit slippage and zero-amount rejection
- swaps in both directions
- swap slippage and zero-amount rejection
- authority checks for locking
- lock and unlock behavior
- withdrawals with slippage checks
- zero-amount withdraw rejection

Passing artifacts are saved in `docs/`:

- `docs/tests-passing.png`
- `docs/anchor-tests-passing.png`
- `docs/cargo-tests-passing.png`
- `docs/test-output.txt`

## Build and deployment

The declared program id is:

- `4DmfmgZHzg7aTC11qaZGc7WsbiA7hjtgLU4TpePrSB3v`

It is set in `programs/amm/src/lib.rs` and mirrored in `Anchor.toml`.

### Localnet

`anchor test` already handles the local validator workflow. If you want to keep the validator alive after the test run, use:

```bash
anchor test --detach
```

### Deploying manually

Build first, then deploy:

```bash
anchor build
anchor deploy
```

If you fork this repository and change the program id, update both `programs/amm/src/lib.rs` and `Anchor.toml` before deploying again.

## Submission details

This README is written for the **Kali-Decoder** submission of **Assignment 5**.

If you want, I can also add a short “my deployment” section once you have a confirmed on-chain signature to include.
