use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenInterface};

use crate::{
    constants::{MARKETPLACE_SEED, MAX_NAME_LEN, REWARDS_SEED, TREASURY_SEED},
    error::MarketplaceError,
    state::Marketplace,
};

/// Creates a new marketplace together with its SOL treasury (a PDA) and its
/// rewards mint (a PDA whose mint authority is the marketplace itself).
#[derive(Accounts)]
#[instruction(name: String)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + Marketplace::INIT_SPACE,
        seeds = [MARKETPLACE_SEED, name.as_bytes()],
        bump,
    )]
    pub marketplace: Account<'info, Marketplace>,

    /// SOL treasury PDA. Holds marketplace fees collected on SOL sales.
    #[account(
        seeds = [TREASURY_SEED, marketplace.key().as_ref()],
        bump,
    )]
    pub treasury: SystemAccount<'info>,

    /// Rewards mint, controlled by the marketplace PDA.
    #[account(
        init,
        payer = admin,
        seeds = [REWARDS_SEED, marketplace.key().as_ref()],
        bump,
        mint::decimals = 6,
        mint::authority = marketplace,
        mint::token_program = token_program,
    )]
    pub rewards_mint: InterfaceAccount<'info, Mint>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn initialize_handler(ctx: Context<Initialize>, name: String, fee: u16) -> Result<()> {
    require!(
        !name.is_empty() && name.len() <= MAX_NAME_LEN,
        MarketplaceError::InvalidNameLength
    );
    require!(fee <= 10_000, MarketplaceError::InvalidFee);

    ctx.accounts.marketplace.set_inner(Marketplace {
        admin: ctx.accounts.admin.key(),
        fee,
        bump: ctx.bumps.marketplace,
        treasury_bump: ctx.bumps.treasury,
        rewards_bump: ctx.bumps.rewards_mint,
        name,
    });

    Ok(())
}
