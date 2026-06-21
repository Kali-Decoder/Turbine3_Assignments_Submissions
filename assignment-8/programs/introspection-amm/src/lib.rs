pub mod constants;
pub mod curve;
pub mod error;
pub mod instructions;
pub mod introspection;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("4BLYz11aMdVuVWQPVsjKoHGEGbnvSfAE3gXFpCx3G95w");

#[program]
pub mod introspection_amm {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        seed: u64,
        fee_bps: u16,
        authority: Option<Pubkey>,
    ) -> Result<()> {
        initialize_handler(ctx, seed, fee_bps, authority)
    }

    pub fn deposit(
        ctx: Context<Deposit>,
        amount_a: u64,
        max_b: u64,
        min_lp: u64,
    ) -> Result<()> {
        deposit_handler(ctx, amount_a, max_b, min_lp)
    }

    pub fn swap(
        ctx: Context<Swap>,
        amount_in: u64,
        min_amount_out: u64,
        a_to_b: bool,
    ) -> Result<()> {
        swap_handler(ctx, amount_in, min_amount_out, a_to_b)
    }

    /// Burns LP tokens. Must be followed immediately by `payout` in the same tx.
    pub fn burn_lp(ctx: Context<BurnLp>, lp_amount: u64) -> Result<()> {
        burn_lp_handler(ctx, lp_amount)
    }

    /// Pays out pool tokens after introspecting the preceding `burn_lp` instruction.
    pub fn payout(ctx: Context<Payout>, min_a: u64, min_b: u64) -> Result<()> {
        payout_handler(ctx, min_a, min_b)
    }

    pub fn lock(ctx: Context<SetLock>) -> Result<()> {
        lock_handler(ctx)
    }

    pub fn unlock(ctx: Context<SetLock>) -> Result<()> {
        unlock_handler(ctx)
    }
}
