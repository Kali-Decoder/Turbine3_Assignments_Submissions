use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};
use anchor_spl::token_interface::Mint;

use crate::{
    constants::{LISTING_SEED, MARKETPLACE_SEED, OFFER_SEED},
    error::MarketplaceError,
    state::{Listing, Marketplace, Offer},
};

/// Makes a SOL offer on a listed NFT at a price the buyer chooses, instead of
/// paying the listed price. The offered lamports are escrowed in the Offer PDA
/// until the maker accepts or the buyer cancels.
#[derive(Accounts)]
pub struct MakeOffer<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(
        seeds = [MARKETPLACE_SEED, marketplace.name.as_bytes()],
        bump = marketplace.bump,
    )]
    pub marketplace: Account<'info, Marketplace>,

    pub maker_mint: InterfaceAccount<'info, Mint>,

    /// The listing must exist for the NFT to be offered on.
    #[account(
        has_one = maker_mint,
        seeds = [LISTING_SEED, marketplace.key().as_ref(), maker_mint.key().as_ref()],
        bump = listing.bump,
    )]
    pub listing: Account<'info, Listing>,

    #[account(
        init,
        payer = buyer,
        space = 8 + Offer::INIT_SPACE,
        seeds = [OFFER_SEED, maker_mint.key().as_ref(), buyer.key().as_ref()],
        bump,
    )]
    pub offer: Account<'info, Offer>,

    pub system_program: Program<'info, System>,
}

pub fn make_offer_handler(ctx: Context<MakeOffer>, amount: u64) -> Result<()> {
    require!(amount > 0, MarketplaceError::ZeroOffer);

    // Escrow the offered SOL into the Offer PDA.
    transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.buyer.to_account_info(),
                to: ctx.accounts.offer.to_account_info(),
            },
        ),
        amount,
    )?;

    ctx.accounts.offer.set_inner(Offer {
        buyer: ctx.accounts.buyer.key(),
        maker_mint: ctx.accounts.maker_mint.key(),
        amount,
        bump: ctx.bumps.offer,
    });

    Ok(())
}
