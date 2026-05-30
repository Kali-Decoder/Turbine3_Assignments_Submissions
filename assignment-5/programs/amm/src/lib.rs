pub mod constants;
pub mod curve;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("4DmfmgZHzg7aTC11qaZGc7WsbiA7hjtgLU4TpePrSB3v");

#[program]
pub mod amm {
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

    pub fn withdraw(
        ctx: Context<Withdraw>,
        lp_amount: u64,
        min_a: u64,
        min_b: u64,
    ) -> Result<()> {
        withdraw_handler(ctx, lp_amount, min_a, min_b)
    }

    pub fn swap(
        ctx: Context<Swap>,
        amount_in: u64,
        min_amount_out: u64,
        a_to_b: bool,
    ) -> Result<()> {
        swap_handler(ctx, amount_in, min_amount_out, a_to_b)
    }

    pub fn lock(ctx: Context<SetLock>) -> Result<()> {
        lock_handler(ctx)
    }

    pub fn unlock(ctx: Context<SetLock>) -> Result<()> {
        unlock_handler(ctx)
    }
}
