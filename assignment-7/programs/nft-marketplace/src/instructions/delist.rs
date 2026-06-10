use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        close_account, transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface,
        TransferChecked,
    },
};

use crate::{
    constants::{LISTING_SEED, MARKETPLACE_SEED},
    state::{Listing, Marketplace},
};

/// Cancels a listing: returns the escrowed NFT to the maker, closes the vault
/// and closes the listing account, refunding rent to the maker.
#[derive(Accounts)]
pub struct Delist<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
        seeds = [MARKETPLACE_SEED, marketplace.name.as_bytes()],
        bump = marketplace.bump,
    )]
    pub marketplace: Account<'info, Marketplace>,

    pub maker_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init_if_needed,
        payer = maker,
        associated_token::mint = maker_mint,
        associated_token::authority = maker,
        associated_token::token_program = token_program,
    )]
    pub maker_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        close = maker,
        has_one = maker,
        has_one = maker_mint,
        seeds = [LISTING_SEED, marketplace.key().as_ref(), maker_mint.key().as_ref()],
        bump = listing.bump,
    )]
    pub listing: Account<'info, Listing>,

    #[account(
        mut,
        associated_token::mint = maker_mint,
        associated_token::authority = listing,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn delist_handler(ctx: Context<Delist>) -> Result<()> {
    let marketplace_key = ctx.accounts.marketplace.key();
    let maker_mint_key = ctx.accounts.maker_mint.key();
    let signer_seeds: &[&[&[u8]]] = &[&[
        LISTING_SEED,
        marketplace_key.as_ref(),
        maker_mint_key.as_ref(),
        &[ctx.accounts.listing.bump],
    ]];

    // Return the NFT to the maker.
    let transfer_accounts = TransferChecked {
        from: ctx.accounts.vault.to_account_info(),
        mint: ctx.accounts.maker_mint.to_account_info(),
        to: ctx.accounts.maker_ata.to_account_info(),
        authority: ctx.accounts.listing.to_account_info(),
    };
    let transfer_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        transfer_accounts,
        signer_seeds,
    );
    transfer_checked(transfer_ctx, 1, ctx.accounts.maker_mint.decimals)?;

    // Close the now-empty vault, refunding its rent to the maker.
    let close_accounts = CloseAccount {
        account: ctx.accounts.vault.to_account_info(),
        destination: ctx.accounts.maker.to_account_info(),
        authority: ctx.accounts.listing.to_account_info(),
    };
    let close_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        close_accounts,
        signer_seeds,
    );
    close_account(close_ctx)?;

    Ok(())
}
