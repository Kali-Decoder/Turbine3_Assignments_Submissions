use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked};
use solana_program::sysvar::instructions;

use crate::constants::{CONFIG_SEED, LP_SEED};
use crate::curve::withdraw_amount;
use crate::error::AmmError;
use crate::introspection::verify_previous_burn_lp;
use crate::state::Config;

/// Pays out underlying tokens after verifying the previous instruction in this
/// transaction was a matching `burn_lp` call (instruction introspection).
#[derive(Accounts)]
pub struct Payout<'info> {
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
        associated_token::mint = mint_a,
        associated_token::authority = config,
    )]
    pub vault_a: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = config,
    )]
    pub vault_b: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = user,
    )]
    pub user_ata_a: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = user,
    )]
    pub user_ata_b: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_lp,
        associated_token::authority = user,
    )]
    pub user_ata_lp: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: Instructions sysvar validated by address constraint.
    #[account(address = instructions::ID)]
    pub instructions: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> Payout<'info> {
    fn transfer_out(
        &self,
        from: &InterfaceAccount<'info, TokenAccount>,
        to: &InterfaceAccount<'info, TokenAccount>,
        mint: &InterfaceAccount<'info, Mint>,
        amount: u64,
    ) -> Result<()> {
        let seed_bytes = self.config.seed.to_le_bytes();
        let signer_seeds: &[&[&[u8]]] = &[&[
            CONFIG_SEED,
            seed_bytes.as_ref(),
            &[self.config.config_bump],
        ]];
        token_interface::transfer_checked(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                TransferChecked {
                    from: from.to_account_info(),
                    to: to.to_account_info(),
                    mint: mint.to_account_info(),
                    authority: self.config.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
            mint.decimals,
        )
    }
}

pub fn payout_handler(ctx: Context<Payout>, min_a: u64, min_b: u64) -> Result<()> {
    require!(!ctx.accounts.config.locked, AmmError::PoolLocked);

    let lp_amount = verify_previous_burn_lp(
        &ctx.accounts.instructions.to_account_info(),
        &ctx.accounts.user.key(),
        &ctx.accounts.config.key(),
        &ctx.accounts.mint_lp.key(),
        &ctx.accounts.user_ata_lp.key(),
    )?;

    // LP was already burned in the previous instruction; use pre-burn supply + amount.
    let lp_supply = ctx
        .accounts
        .mint_lp
        .supply
        .checked_add(lp_amount)
        .ok_or(AmmError::Overflow)?;

    let reserve_a = ctx.accounts.vault_a.amount;
    let reserve_b = ctx.accounts.vault_b.amount;

    let out_a = withdraw_amount(lp_amount, reserve_a, lp_supply)?;
    let out_b = withdraw_amount(lp_amount, reserve_b, lp_supply)?;

    require!(out_a >= min_a, AmmError::SlippageExceeded);
    require!(out_b >= min_b, AmmError::SlippageExceeded);

    ctx.accounts.transfer_out(
        &ctx.accounts.vault_a,
        &ctx.accounts.user_ata_a,
        &ctx.accounts.mint_a,
        out_a,
    )?;
    ctx.accounts.transfer_out(
        &ctx.accounts.vault_b,
        &ctx.accounts.user_ata_b,
        &ctx.accounts.mint_b,
        out_b,
    )?;

    Ok(())
}
