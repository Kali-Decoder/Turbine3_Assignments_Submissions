use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        close_account, mint_to, transfer_checked, CloseAccount, Mint, MintTo, TokenAccount,
        TokenInterface, TransferChecked,
    },
};

use crate::{
    constants::{LISTING_SEED, MARKETPLACE_SEED, REWARDS_SEED, REWARD_AMOUNT},
    error::MarketplaceError,
    state::{Listing, Marketplace},
    util::split_price,
};

/// Buys a token-denominated listing. The buyer pays the listed price in the
/// listing's `payment_mint` (e.g. USDC). Payment is split between the maker and
/// the treasury — which, for token sales, is an ATA owned by the marketplace
/// PDA rather than a SystemAccount. Settlement (NFT release, rewards, vault
/// close) mirrors the SOL `purchase` flow.
#[derive(Accounts)]
pub struct BuyWithToken<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    /// The seller, who receives token proceeds and rent refunds.
    #[account(mut)]
    pub maker: SystemAccount<'info>,

    #[account(
        seeds = [MARKETPLACE_SEED, marketplace.name.as_bytes()],
        bump = marketplace.bump,
    )]
    pub marketplace: Box<Account<'info, Marketplace>>,

    /// The SPL token the listing is priced in.
    pub payment_mint: Box<InterfaceAccount<'info, Mint>>,

    pub maker_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = payment_mint,
        associated_token::authority = buyer,
        associated_token::token_program = token_program,
    )]
    pub buyer_payment_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = payment_mint,
        associated_token::authority = maker,
        associated_token::token_program = token_program,
    )]
    pub maker_payment_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Token treasury: an ATA owned by the marketplace PDA.
    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = payment_mint,
        associated_token::authority = marketplace,
        associated_token::token_program = token_program,
    )]
    pub treasury_payment_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = maker_mint,
        associated_token::authority = buyer,
        associated_token::token_program = token_program,
    )]
    pub buyer_nft_ata: Box<InterfaceAccount<'info, TokenAccount>>,

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

pub fn buy_with_token_handler(ctx: Context<BuyWithToken>) -> Result<()> {
    // The listing must be token-denominated, and the supplied mint must match.
    match ctx.accounts.listing.payment_mint {
        None => return err!(MarketplaceError::NotTokenListing),
        Some(mint) => require_keys_eq!(
            mint,
            ctx.accounts.payment_mint.key(),
            MarketplaceError::PaymentMintMismatch
        ),
    }

    let (seller_amount, fee) = split_price(ctx.accounts.listing.price, ctx.accounts.marketplace.fee)?;
    let payment_decimals = ctx.accounts.payment_mint.decimals;

    // Pay the seller in the payment token.
    transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.buyer_payment_ata.to_account_info(),
                mint: ctx.accounts.payment_mint.to_account_info(),
                to: ctx.accounts.maker_payment_ata.to_account_info(),
                authority: ctx.accounts.buyer.to_account_info(),
            },
        ),
        seller_amount,
        payment_decimals,
    )?;

    // Pay the marketplace fee into the token treasury.
    if fee > 0 {
        transfer_checked(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.buyer_payment_ata.to_account_info(),
                    mint: ctx.accounts.payment_mint.to_account_info(),
                    to: ctx.accounts.treasury_payment_ata.to_account_info(),
                    authority: ctx.accounts.buyer.to_account_info(),
                },
            ),
            fee,
            payment_decimals,
        )?;
    }

    settle_nft_and_rewards(&ctx)?;

    Ok(())
}

/// Releases the escrowed NFT to the buyer, mints reward tokens, and closes the
/// vault.
fn settle_nft_and_rewards(ctx: &Context<BuyWithToken>) -> Result<()> {
    let marketplace_key = ctx.accounts.marketplace.key();
    let maker_mint_key = ctx.accounts.maker_mint.key();
    let listing_seeds: &[&[&[u8]]] = &[&[
        LISTING_SEED,
        marketplace_key.as_ref(),
        maker_mint_key.as_ref(),
        &[ctx.accounts.listing.bump],
    ]];

    transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.vault.to_account_info(),
                mint: ctx.accounts.maker_mint.to_account_info(),
                to: ctx.accounts.buyer_nft_ata.to_account_info(),
                authority: ctx.accounts.listing.to_account_info(),
            },
            listing_seeds,
        ),
        1,
        ctx.accounts.maker_mint.decimals,
    )?;

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
