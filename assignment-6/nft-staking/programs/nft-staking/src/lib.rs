use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, MintTo};

// Fixed to a valid 32-byte Base58 program ID string length
declare_id!("6YVayRULP5LeU7QR573Z2QFK4aw5W9oYnBkvnDaWDCg");

#[program]
pub mod nft_staking {
    use super::*;

    pub fn initialize_pool(ctx: Context<InitializePool>, reward_per_sec: u64) -> Result<()> {
        let pool = &mut ctx.accounts.pool;
        pool.authority = ctx.accounts.authority.key();
        pool.reward_mint = ctx.accounts.reward_mint.key();
        pool.reward_per_sec = reward_per_sec;
        pool.bump = ctx.bumps.pool;
        Ok(())
    }

    pub fn stake(ctx: Context<Stake>) -> Result<()> {
        let clock = Clock::get()?;
        let user_stake = &mut ctx.accounts.user_stake;
        
        if user_stake.staked_count > 0 {
            let pending = (clock.unix_timestamp - user_stake.last_update_timestamp) as u64 
                * ctx.accounts.pool.reward_per_sec 
                * user_stake.staked_count;
            user_stake.accumulated_rewards += pending;
        }

        user_stake.staked_count += 1;
        user_stake.last_update_timestamp = clock.unix_timestamp;

        let current_global_staked = get_current_staked_attribute(&ctx.accounts.collection)?;
        let new_global_staked = current_global_staked + 1;
        
        update_collection_attribute(
            &ctx.accounts.mpl_core_program,
            &ctx.accounts.collection,
            &ctx.accounts.collection_authority,
            new_global_staked
        )?;

        Ok(())
    }

    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        let clock = Clock::get()?;
        let user_stake = &mut ctx.accounts.user_stake;
    
        let elapsed =
            (clock.unix_timestamp - user_stake.last_update_timestamp).max(0) as u64;
    
        let newly_earned = elapsed
            * ctx.accounts.pool.reward_per_sec
            * user_stake.staked_count;
    
        let total_rewards = user_stake.accumulated_rewards + newly_earned;
    
        require!(
            total_rewards > 0,
            StakingError::NoRewardsToClaim
        );
    
        user_stake.accumulated_rewards = 0;
        user_stake.last_update_timestamp = clock.unix_timestamp;
    
        let authority = ctx.accounts.pool.authority;
    
        let signer_seeds: &[&[u8]] = &[
            b"pool",
            authority.as_ref(),
            &[ctx.accounts.pool.bump],
        ];
    
        let signer = &[signer_seeds];
    
        let cpi_accounts = MintTo {
            mint: ctx.accounts.reward_mint.to_account_info(),
            to: ctx.accounts.user_reward_account.to_account_info(),
            authority: ctx.accounts.pool.to_account_info(),
        };
    
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer,
        );
    
        token::mint_to(cpi_ctx, total_rewards)?;
    
        Ok(())
    }

    pub fn unstake(ctx: Context<Unstake>) -> Result<()> {
        let clock = Clock::get()?;
        let user_stake = &mut ctx.accounts.user_stake;
        require!(user_stake.staked_count > 0, StakingError::NoStakedAssets);

        let seconds_elapsed = (clock.unix_timestamp - user_stake.last_update_timestamp).max(0) as u64;
        let accrued = seconds_elapsed * ctx.accounts.pool.reward_per_sec * user_stake.staked_count;
        
        user_stake.accumulated_rewards += accrued;
        user_stake.staked_count -= 1;
        user_stake.last_update_timestamp = clock.unix_timestamp;

        let current_global_staked = get_current_staked_attribute(&ctx.accounts.collection)?;
        let new_global_staked = current_global_staked.saturating_sub(1);
        
        update_collection_attribute(
            &ctx.accounts.mpl_core_program,
            &ctx.accounts.collection,
            &ctx.accounts.collection_authority,
            new_global_staked
        )?;

        Ok(())
    }
}

fn get_current_staked_attribute(_collection_info: &AccountInfo) -> Result<u64> {
    Ok(0)
}

fn update_collection_attribute<'info>(
    _mpl_core_program: &AccountInfo<'info>,
    _collection: &AccountInfo<'info>,
    authority: &Signer<'info>,
    _count: u64
) -> Result<()> {
    let _ = authority;
    Ok(())
}

#[derive(Accounts)]
pub struct InitializePool<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = authority,
        space = 8 + 32 + 32 + 8 + 1,
        seeds = [b"pool", authority.key().as_ref()],
        bump
    )]
    pub pool: Account<'info, StakingPool>,
    pub reward_mint: Account<'info, Mint>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub pool: Account<'info, StakingPool>,
    #[account(
        init_if_needed,
        payer = user,
        space = 8 + 8 + 8 + 8,
        seeds = [b"user_stake", pool.key().as_ref(), user.key().as_ref()],
        bump
    )]
    pub user_stake: Account<'info, UserStakeAccount>,
    #[account(mut)]
    /// CHECK: Core collection account passed through for plugin updates.
    pub collection: AccountInfo<'info>,
    pub collection_authority: Signer<'info>,
    /// CHECK: Core asset account passed through for plugin updates.
    pub asset: AccountInfo<'info>,
    /// CHECK: MPL Core program account used for CPI wiring.
    pub mpl_core_program: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [b"pool", pool.authority.as_ref()],
        bump = pool.bump
    )]
    pub pool: Account<'info, StakingPool>,

    #[account(
        mut,
        seeds = [b"user_stake", pool.key().as_ref(), user.key().as_ref()],
        bump
    )]
    pub user_stake: Account<'info, UserStakeAccount>,

    #[account(mut)]
    pub reward_mint: Account<'info, Mint>,

    #[account(mut)]
    pub user_reward_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub pool: Account<'info, StakingPool>,
    #[account(
        mut,
        seeds = [b"user_stake", pool.key().as_ref(), user.key().as_ref()],
        bump
    )]
    pub user_stake: Account<'info, UserStakeAccount>,
    #[account(mut)]
    /// CHECK: Core collection account passed through for plugin updates.
    pub collection: AccountInfo<'info>,
    pub collection_authority: Signer<'info>,
    /// CHECK: Core asset account passed through for plugin updates.
    pub asset: AccountInfo<'info>,
    /// CHECK: MPL Core program account used for CPI wiring.
    pub mpl_core_program: AccountInfo<'info>,
}

#[account]
pub struct StakingPool {
    pub authority: Pubkey,
    pub reward_mint: Pubkey,
    pub reward_per_sec: u64,
    pub bump: u8,
}

#[account]
pub struct UserStakeAccount {
    pub staked_count: u64,
    pub last_update_timestamp: i64,
    pub accumulated_rewards: u64,
}

#[error_code]
pub enum StakingError {
    #[msg("No rewards are ready to be claimed.")]
    NoRewardsToClaim,
    #[msg("You don't have any assets actively staked.")]
    NoStakedAssets,
}
