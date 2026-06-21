use anchor_lang::prelude::*;

use crate::constants::CONFIG_SEED;
use crate::error::AmmError;
use crate::state::Config;

#[derive(Accounts)]
pub struct SetLock<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [CONFIG_SEED, config.seed.to_le_bytes().as_ref()],
        bump = config.config_bump,
    )]
    pub config: Box<Account<'info, Config>>,
}

pub fn lock_handler(ctx: Context<SetLock>) -> Result<()> {
    let cfg = &ctx.accounts.config;
    require!(
        cfg.authority
            .map(|a| a == ctx.accounts.authority.key())
            .unwrap_or(false),
        AmmError::Unauthorized
    );
    ctx.accounts.config.locked = true;
    Ok(())
}

pub fn unlock_handler(ctx: Context<SetLock>) -> Result<()> {
    let cfg = &ctx.accounts.config;
    require!(
        cfg.authority
            .map(|a| a == ctx.accounts.authority.key())
            .unwrap_or(false),
        AmmError::Unauthorized
    );
    ctx.accounts.config.locked = false;
    Ok(())
}
