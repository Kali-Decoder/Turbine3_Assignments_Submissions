# Assignment 8 — Instruction Introspection AMM

**Turbin3 Week 6 Challenge (Option 2)**

A constant-product AMM built from scratch with Anchor, where **withdrawals are split into two instructions** in the same transaction:

1. **`burn_lp`** — burns the user's LP tokens
2. **`payout`** — transfers underlying tokens from the pool vaults, but only after **instruction introspection** verifies that the immediately preceding instruction was a matching `burn_lp` call

This pattern ensures a burn cannot be replayed across unrelated payout calls and that payout always correlates with a burn in the same transaction (relative index `-1`, not absolute index `0`).

## Challenge completed

| Requirement | Status |
| ----------- | ------ |
| AMM program from scratch | ✅ |
| Instruction introspection | ✅ `payout` reads Instructions sysvar |
| Token burn before payout | ✅ separate `burn_lp` + `payout` |
| Tests for all instructions | ✅ 16 tests |
| README | ✅ |

## Instructions

| Instruction | Description |
| ----------- | ----------- |
| `initialize` | Creates pool config PDA, LP mint, and token vaults |
| `deposit` | Adds liquidity; mints LP tokens (constant-product) |
| `swap` | Swaps token A ↔ B with configurable fee |
| `burn_lp` | Burns LP from user ATA (**must precede `payout` in same tx**) |
| `payout` | Pays out pro-rata reserves after introspecting previous `burn_lp` |
| `lock` / `unlock` | Authority-gated pool freeze |

## Instruction introspection design

```
Transaction:
  [0] burn_lp(lp_amount)   ← burns LP, records amount in ix data
  [1] payout(min_a, min_b) ← introspects ix[0]
```

Inside `payout`, the program:

1. Loads the **Instructions sysvar** (`Sysvar1nstructions1111111111111111111111111`)
2. Gets the **current instruction index** via `load_current_index_checked`
3. Loads the **previous instruction** at `index - 1` via `load_instruction_at_checked`
4. Validates:
   - `program_id == introspection_amm`
   - Discriminator matches `burn_lp`
   - `lp_amount` in instruction data
   - Account pubkeys at fixed indices (user, config, mint_lp, user_ata_lp)

See `programs/introspection-amm/src/introspection.rs` for the full verification logic.

## Project layout

```
assignment-8/
├── programs/introspection-amm/
│   └── src/
│       ├── lib.rs
│       ├── introspection.rs      # burn_lp verification via sysvar
│       ├── curve.rs              # AMM math
│       ├── constants.rs
│       ├── error.rs
│       ├── state/config.rs
│       └── instructions/
│           ├── initialize.rs
│           ├── deposit.rs
│           ├── swap.rs
│           ├── burn_lp.rs
│           ├── payout.rs
│           └── lock.rs
├── tests/introspection-amm.ts    # 16 integration tests
└── docs/
    └── test-output.txt           # full passing test log
```

## PDAs

| Account | Seeds |
| ------- | ----- |
| `Config` | `["config", seed.to_le_bytes()]` |
| LP mint | `["lp", config.key()]` |
| Vaults | ATAs owned by `config` for mint A and mint B |

## Setup

- Rust + Cargo
- Solana CLI
- Anchor CLI **0.32.1**
- Node.js + yarn

```bash
cd assignment-8
yarn install
anchor build
anchor test
```

Program ID (localnet): `4BLYz11aMdVuVWQPVsjKoHGEGbnvSfAE3gXFpCx3G95w`

## Test coverage (16 tests)

| Suite | Tests |
| ----- | ----- |
| **initialize** | pool creation, identical mint rejection, invalid fee |
| **deposit** | bootstrap liquidity, zero-amount rejection |
| **swap** | A→B, B→A, slippage rejection |
| **burn_lp + payout** | happy-path withdraw via introspection, payout without burn fails, sandwiched instruction fails, zero burn fails, slippage on payout |
| **lock / unlock** | unauthorized lock, lock blocks burn, unlock restores access |

### Passing test output

All **16 tests pass**. Full log saved at [`docs/test-output.txt`](./docs/test-output.txt):

```
  introspection-amm
    initialize
      ✔ creates config, LP mint, and vaults
      ✔ rejects identical mints
      ✔ rejects invalid fee
    deposit
      ✔ bootstraps liquidity and mints LP
      ✔ rejects zero deposit
    swap
      ✔ swaps A for B
      ✔ swaps B for A
      ✔ rejects excessive slippage
    burn_lp + payout (instruction introspection)
      ✔ withdraws via burn_lp immediately followed by payout
      ✔ rejects payout without a preceding burn_lp
      ✔ rejects payout when burn_lp is not the previous instruction
      ✔ rejects burn_lp with zero amount
      ✔ rejects payout when slippage is too high
    lock / unlock
      ✔ rejects lock from non-authority
      ✔ locks and blocks burn_lp
      ✔ unlocks the pool

  16 passing
```

> **Screenshot for submission:** Run `anchor test` locally and capture the terminal showing `16 passing`. Attach as `docs/tests-passing.png`.

## Example: withdraw with introspection (TypeScript)

```typescript
import { Transaction } from "@solana/web3.js";

const SYSVAR_INSTRUCTIONS = new PublicKey(
  "Sysvar1nstructions1111111111111111111111111"
);

const burnIx = await program.methods
  .burnLp(lpAmount)
  .accountsPartial({ user, mintA, mintB, config, mintLp, userAtaLp, tokenProgram })
  .instruction();

const payoutIx = await program.methods
  .payout(minA, minB)
  .accountsPartial({
    user, mintA, mintB, config, mintLp, vaultA, vaultB,
    userAtaA, userAtaB, userAtaLp,
    instructions: SYSVAR_INSTRUCTIONS,
    tokenProgram, associatedTokenProgram, systemProgram,
  })
  .instruction();

await provider.sendAndConfirm(new Transaction().add(burnIx, payoutIx), [user]);
```

## Key errors

| Error | When |
| ----- | ---- |
| `MissingPreviousInstruction` | `payout` is first instruction in tx |
| `InvalidProgram` | Previous ix is not from this program |
| `InvalidInstruction` | Previous ix discriminator ≠ `burn_lp` |
| `InvalidInstructionData` | `lp_amount` missing or zero in previous ix |
| `InvalidInstructionAccounts` | User/config/mint accounts don't match |

## Curve math

Constant-product swap with fee in basis points (`FEE_DENOMINATOR = 10_000`):

```
amount_in_with_fee = amount_in * (10_000 - fee_bps)
amount_out = (amount_in_with_fee * reserve_out)
             / (reserve_in * 10_000 + amount_in_with_fee)
```

Withdraw uses pro-rata share: `out = lp_amount * reserve / lp_supply` (with `lp_supply` adjusted to pre-burn total during payout).
