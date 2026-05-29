use anchor_lang::prelude::*;

pub mod error;
pub mod instructions;
pub mod state;

use instructions::*;

pub(crate) use instructions::make::__client_accounts_make;
pub(crate) use instructions::refund::__client_accounts_refund;
pub(crate) use instructions::take::__client_accounts_take;

declare_id!("C2oi9cyMqb3VkRFJTxV7ePsViZBhPY7vhPLxZ6uNyPjK");

#[program]
pub mod escrow {
    use super::*;

    pub fn make(ctx: Context<Make>, seed: u64, deposit: u64, receive: u64) -> Result<()> {
        instructions::make::handler(ctx, seed, deposit, receive)
    }

    pub fn take(ctx: Context<Take>) -> Result<()> {
        instructions::take::handler(ctx)
    }

    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        instructions::refund::handler(ctx)
    }
}
