use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Burn, Mint, TokenAccount, TokenInterface};

use crate::constants::{CONFIG_SEED, LP_SEED};
use crate::error::AmmError;
use crate::state::Config;

/// Burns LP tokens from the user. Must be immediately followed by `payout` in
/// the same transaction so the program can introspect this instruction.
#[derive(Accounts)]
pub struct BurnLp<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    pub mint_a: Box<InterfaceAccount<'info, Mint>>,
    pub mint_b: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        seeds = [CONFIG_SEED, config.seed.to_le_bytes().as_ref()],
        bump = config.config_bump,
        has_one = mint_a,
        has_one = mint_b,
    )]
    pub config: Box<Account<'info, Config>>,

    #[account(
        mut,
        seeds = [LP_SEED, config.key().as_ref()],
        bump = config.lp_bump,
    )]
    pub mint_lp: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = mint_lp,
        associated_token::authority = user,
    )]
    pub user_ata_lp: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn burn_lp_handler(ctx: Context<BurnLp>, lp_amount: u64) -> Result<()> {
    require!(!ctx.accounts.config.locked, AmmError::PoolLocked);
    require!(lp_amount > 0, AmmError::ZeroAmount);

    token_interface::burn(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                mint: ctx.accounts.mint_lp.to_account_info(),
                from: ctx.accounts.user_ata_lp.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        lp_amount,
    )?;

    Ok(())
}
