use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::constants::{CONFIG_SEED, FEE_DENOMINATOR, LP_SEED};
use crate::error::AmmError;
use crate::state::Config;

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub initializer: Signer<'info>,

    pub mint_a: Box<InterfaceAccount<'info, Mint>>,

    #[account(constraint = mint_a.key() != mint_b.key() @ AmmError::IdenticalMints)]
    pub mint_b: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        init,
        payer = initializer,
        seeds = [CONFIG_SEED, seed.to_le_bytes().as_ref()],
        bump,
        space = 8 + Config::INIT_SPACE,
    )]
    pub config: Box<Account<'info, Config>>,

    #[account(
        init,
        payer = initializer,
        seeds = [LP_SEED, config.key().as_ref()],
        bump,
        mint::decimals = 6,
        mint::authority = config,
    )]
    pub mint_lp: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        init,
        payer = initializer,
        associated_token::mint = mint_a,
        associated_token::authority = config,
    )]
    pub vault_a: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init,
        payer = initializer,
        associated_token::mint = mint_b,
        associated_token::authority = config,
    )]
    pub vault_b: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn initialize_handler(
    ctx: Context<Initialize>,
    seed: u64,
    fee_bps: u16,
    authority: Option<Pubkey>,
) -> Result<()> {
    require!((fee_bps as u64) < FEE_DENOMINATOR, AmmError::InvalidFee);

    ctx.accounts.config.set_inner(Config {
        seed,
        authority,
        mint_a: ctx.accounts.mint_a.key(),
        mint_b: ctx.accounts.mint_b.key(),
        fee_bps,
        locked: false,
        config_bump: ctx.bumps.config,
        lp_bump: ctx.bumps.mint_lp,
    });

    Ok(())
}
