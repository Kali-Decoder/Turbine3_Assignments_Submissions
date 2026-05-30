use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{
    self, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::constants::CONFIG_SEED;
use crate::curve::swap_output;
use crate::error::AmmError;
use crate::state::Config;

#[derive(Accounts)]
pub struct Swap<'info> {
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

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> Swap<'info> {
    fn transfer_user_to_vault(
        &self,
        from: &InterfaceAccount<'info, TokenAccount>,
        to: &InterfaceAccount<'info, TokenAccount>,
        mint: &InterfaceAccount<'info, Mint>,
        amount: u64,
    ) -> Result<()> {
        let cpi_accounts = TransferChecked {
            from: from.to_account_info(),
            to: to.to_account_info(),
            mint: mint.to_account_info(),
            authority: self.user.to_account_info(),
        };
        let ctx = CpiContext::new(self.token_program.to_account_info(), cpi_accounts);
        token_interface::transfer_checked(ctx, amount, mint.decimals)
    }

    fn transfer_vault_to_user(
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
        let cpi_accounts = TransferChecked {
            from: from.to_account_info(),
            to: to.to_account_info(),
            mint: mint.to_account_info(),
            authority: self.config.to_account_info(),
        };
        let ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        token_interface::transfer_checked(ctx, amount, mint.decimals)
    }
}

/// `a_to_b = true` means swap mint_a in, receive mint_b.
pub fn swap_handler(
    ctx: Context<Swap>,
    amount_in: u64,
    min_amount_out: u64,
    a_to_b: bool,
) -> Result<()> {
    require!(!ctx.accounts.config.locked, AmmError::PoolLocked);
    require!(amount_in > 0, AmmError::ZeroAmount);

    let reserve_a = ctx.accounts.vault_a.amount;
    let reserve_b = ctx.accounts.vault_b.amount;
    let fee_bps = ctx.accounts.config.fee_bps;

    let amount_out = if a_to_b {
        swap_output(amount_in, reserve_a, reserve_b, fee_bps)?
    } else {
        swap_output(amount_in, reserve_b, reserve_a, fee_bps)?
    };

    require!(amount_out >= min_amount_out, AmmError::SlippageExceeded);
    require!(amount_out > 0, AmmError::ZeroAmount);

    if a_to_b {
        ctx.accounts.transfer_user_to_vault(
            &ctx.accounts.user_ata_a,
            &ctx.accounts.vault_a,
            &ctx.accounts.mint_a,
            amount_in,
        )?;
        ctx.accounts.transfer_vault_to_user(
            &ctx.accounts.vault_b,
            &ctx.accounts.user_ata_b,
            &ctx.accounts.mint_b,
            amount_out,
        )?;
    } else {
        ctx.accounts.transfer_user_to_vault(
            &ctx.accounts.user_ata_b,
            &ctx.accounts.vault_b,
            &ctx.accounts.mint_b,
            amount_in,
        )?;
        ctx.accounts.transfer_vault_to_user(
            &ctx.accounts.vault_a,
            &ctx.accounts.user_ata_a,
            &ctx.accounts.mint_a,
            amount_out,
        )?;
    }

    Ok(())
}
