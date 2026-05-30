use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{
    self, Mint, MintTo, TokenAccount, TokenInterface, TransferChecked,
};

use crate::constants::{CONFIG_SEED, LP_SEED};
use crate::curve::{initial_lp_amount, lp_from_deposit, quote_b_for_a};
use crate::error::AmmError;
use crate::state::Config;

#[derive(Accounts)]
pub struct Deposit<'info> {
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
        init_if_needed,
        payer = user,
        associated_token::mint = mint_lp,
        associated_token::authority = user,
    )]
    pub user_ata_lp: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> Deposit<'info> {
    fn transfer_in(
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

    fn mint_lp(&self, amount: u64) -> Result<()> {
        let seed_bytes = self.config.seed.to_le_bytes();
        let signer_seeds: &[&[&[u8]]] = &[&[
            CONFIG_SEED,
            seed_bytes.as_ref(),
            &[self.config.config_bump],
        ]];
        let cpi_accounts = MintTo {
            mint: self.mint_lp.to_account_info(),
            to: self.user_ata_lp.to_account_info(),
            authority: self.config.to_account_info(),
        };
        let ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        token_interface::mint_to(ctx, amount)
    }
}

pub fn deposit_handler(
    ctx: Context<Deposit>,
    amount_a: u64,
    max_b: u64,
    min_lp: u64,
) -> Result<()> {
    require!(!ctx.accounts.config.locked, AmmError::PoolLocked);
    require!(amount_a > 0, AmmError::ZeroAmount);

    let reserve_a = ctx.accounts.vault_a.amount;
    let reserve_b = ctx.accounts.vault_b.amount;
    let lp_supply = ctx.accounts.mint_lp.supply;

    let (deposit_b, lp_amount) = if lp_supply == 0 {
        // Bootstrap: caller's `max_b` defines the initial price, lp = sqrt(a * b).
        require!(max_b > 0, AmmError::ZeroAmount);
        let lp = initial_lp_amount(amount_a, max_b)?;
        (max_b, lp)
    } else {
        let needed_b = quote_b_for_a(amount_a, reserve_a, reserve_b)?;
        require!(needed_b <= max_b, AmmError::SlippageExceeded);
        let lp = lp_from_deposit(amount_a, reserve_a, lp_supply)?;
        (needed_b, lp)
    };

    require!(lp_amount >= min_lp, AmmError::SlippageExceeded);

    ctx.accounts.transfer_in(
        &ctx.accounts.user_ata_a,
        &ctx.accounts.vault_a,
        &ctx.accounts.mint_a,
        amount_a,
    )?;
    ctx.accounts.transfer_in(
        &ctx.accounts.user_ata_b,
        &ctx.accounts.vault_b,
        &ctx.accounts.mint_b,
        deposit_b,
    )?;
    ctx.accounts.mint_lp(lp_amount)?;

    Ok(())
}
