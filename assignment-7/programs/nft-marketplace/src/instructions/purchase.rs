use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        close_account, mint_to, transfer_checked, CloseAccount, Mint, MintTo, TokenAccount,
        TokenInterface, TransferChecked,
    },
};

use crate::{
    constants::{LISTING_SEED, MARKETPLACE_SEED, REWARDS_SEED, REWARD_AMOUNT, TREASURY_SEED},
    error::MarketplaceError,
    state::{Listing, Marketplace},
    util::split_price,
};

/// Buys a SOL-denominated listing. The buyer pays the listed price in SOL,
/// split between the maker (proceeds) and the treasury (fee). The NFT is
/// released to the buyer, reward tokens are minted to them, and the listing and
/// vault are closed.
#[derive(Accounts)]
pub struct Purchase<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    /// The seller, who receives the SOL proceeds and rent refunds.
    #[account(mut)]
    pub maker: SystemAccount<'info>,

    #[account(
        seeds = [MARKETPLACE_SEED, marketplace.name.as_bytes()],
        bump = marketplace.bump,
    )]
    pub marketplace: Box<Account<'info, Marketplace>>,

    #[account(
        mut,
        seeds = [TREASURY_SEED, marketplace.key().as_ref()],
        bump = marketplace.treasury_bump,
    )]
    pub treasury: SystemAccount<'info>,

    pub maker_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = maker_mint,
        associated_token::authority = buyer,
        associated_token::token_program = token_program,
    )]
    pub buyer_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = maker_mint,
        associated_token::authority = listing,
        associated_token::token_program = token_program,
    )]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [REWARDS_SEED, marketplace.key().as_ref()],
        bump = marketplace.rewards_bump,
        mint::token_program = token_program,
    )]
    pub rewards_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = rewards_mint,
        associated_token::authority = buyer,
        associated_token::token_program = token_program,
    )]
    pub buyer_rewards_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        close = maker,
        has_one = maker,
        has_one = maker_mint,
        seeds = [LISTING_SEED, marketplace.key().as_ref(), maker_mint.key().as_ref()],
        bump = listing.bump,
    )]
    pub listing: Box<Account<'info, Listing>>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn purchase_handler(ctx: Context<Purchase>) -> Result<()> {
    let listing = &ctx.accounts.listing;
    require!(listing.payment_mint.is_none(), MarketplaceError::NotSolListing);

    let (seller_amount, fee) = split_price(listing.price, ctx.accounts.marketplace.fee)?;

    // Pay the seller.
    transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.buyer.to_account_info(),
                to: ctx.accounts.maker.to_account_info(),
            },
        ),
        seller_amount,
    )?;

    // Pay the marketplace fee into the treasury.
    if fee > 0 {
        transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.buyer.to_account_info(),
                    to: ctx.accounts.treasury.to_account_info(),
                },
            ),
            fee,
        )?;
    }

    settle_nft_and_rewards(&ctx)?;

    Ok(())
}

/// Releases the escrowed NFT to the buyer, mints reward tokens, and closes the
/// vault. Shared between SOL and token purchases.
fn settle_nft_and_rewards(ctx: &Context<Purchase>) -> Result<()> {
    let marketplace_key = ctx.accounts.marketplace.key();
    let maker_mint_key = ctx.accounts.maker_mint.key();
    let listing_seeds: &[&[&[u8]]] = &[&[
        LISTING_SEED,
        marketplace_key.as_ref(),
        maker_mint_key.as_ref(),
        &[ctx.accounts.listing.bump],
    ]];

    // NFT: vault -> buyer.
    transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.vault.to_account_info(),
                mint: ctx.accounts.maker_mint.to_account_info(),
                to: ctx.accounts.buyer_ata.to_account_info(),
                authority: ctx.accounts.listing.to_account_info(),
            },
            listing_seeds,
        ),
        1,
        ctx.accounts.maker_mint.decimals,
    )?;

    // Reward tokens: marketplace -> buyer.
    let name = ctx.accounts.marketplace.name.clone();
    let marketplace_seeds: &[&[&[u8]]] =
        &[&[MARKETPLACE_SEED, name.as_bytes(), &[ctx.accounts.marketplace.bump]]];
    mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.rewards_mint.to_account_info(),
                to: ctx.accounts.buyer_rewards_ata.to_account_info(),
                authority: ctx.accounts.marketplace.to_account_info(),
            },
            marketplace_seeds,
        ),
        REWARD_AMOUNT,
    )?;

    // Close the empty vault, refunding rent to the maker.
    close_account(CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        CloseAccount {
            account: ctx.accounts.vault.to_account_info(),
            destination: ctx.accounts.maker.to_account_info(),
            authority: ctx.accounts.listing.to_account_info(),
        },
        listing_seeds,
    ))?;

    Ok(())
}
