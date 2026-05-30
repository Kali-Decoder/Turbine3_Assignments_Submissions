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
    pub config: Account<'info, Config>,
}

pub fn lock_handler(ctx: Context<SetLock>) -> Result<()> {
    enforce_authority(&ctx.accounts.config, &ctx.accounts.authority)?;
    ctx.accounts.config.locked = true;
    Ok(())
}

pub fn unlock_handler(ctx: Context<SetLock>) -> Result<()> {
    enforce_authority(&ctx.accounts.config, &ctx.accounts.authority)?;
    ctx.accounts.config.locked = false;
    Ok(())
}

fn enforce_authority(config: &Config, signer: &Signer) -> Result<()> {
    match config.authority {
        Some(expected) => {
            require_keys_eq!(expected, signer.key(), AmmError::Unauthorized);
            Ok(())
        }
        None => err!(AmmError::Unauthorized),
    }
}
