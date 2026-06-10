use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::{
    constants::{LISTING_SEED, MARKETPLACE_SEED},
    state::{Listing, Marketplace},
};

/// Lists an NFT for sale. The NFT is moved from the maker's token account into
/// a vault ATA owned by the listing PDA, where it is escrowed until the listing
/// is sold or delisted.
///
/// `payment_mint` selects the denomination:
///   * `None`       -> priced in SOL (settled via `purchase`).
///   * `Some(mint)` -> priced in that SPL token (settled via `buy_with_token`).
#[derive(Accounts)]
pub struct List<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
        seeds = [MARKETPLACE_SEED, marketplace.name.as_bytes()],
        bump = marketplace.bump,
    )]
    pub marketplace: Account<'info, Marketplace>,

    /// The NFT mint being listed.
    pub maker_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = maker_mint,
        associated_token::authority = maker,
        associated_token::token_program = token_program,
    )]
    pub maker_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init,
        payer = maker,
        space = 8 + Listing::INIT_SPACE,
        seeds = [LISTING_SEED, marketplace.key().as_ref(), maker_mint.key().as_ref()],
        bump,
    )]
    pub listing: Account<'info, Listing>,

    #[account(
        init,
        payer = maker,
        associated_token::mint = maker_mint,
        associated_token::authority = listing,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn list_handler(ctx: Context<List>, price: u64, payment_mint: Option<Pubkey>) -> Result<()> {
    // Escrow the NFT into the vault.
    let cpi_accounts = TransferChecked {
        from: ctx.accounts.maker_ata.to_account_info(),
        mint: ctx.accounts.maker_mint.to_account_info(),
        to: ctx.accounts.vault.to_account_info(),
        authority: ctx.accounts.maker.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
    transfer_checked(cpi_ctx, 1, ctx.accounts.maker_mint.decimals)?;

    ctx.accounts.listing.set_inner(Listing {
        maker: ctx.accounts.maker.key(),
        maker_mint: ctx.accounts.maker_mint.key(),
        price,
        payment_mint,
        bump: ctx.bumps.listing,
    });

    Ok(())
}
