use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

declare_id!("BFp47D3DdwLjNqJVt7ztxqn37F2Gq7espARVBsQ1YU5W");

#[program]
pub mod anchor_vault {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.state.state_bump = ctx.bumps.state;
        ctx.accounts.state.vault_bump = ctx.bumps.vault;
        Ok(())
    }

    pub fn deposit(ctx: Context<Payment>, amount: u64) -> Result<()> {
        let cpi_accounts = Transfer {
            from: ctx.accounts.user.to_account_info(),
            to: ctx.accounts.vault.to_account_info(),
        };
        let cpi = CpiContext::new(ctx.accounts.system_program.to_account_info(), cpi_accounts);
        transfer(cpi, amount)
    }

    pub fn withdraw(ctx: Context<Payment>, amount: u64) -> Result<()> {
        let state_key = ctx.accounts.state.key();
        let seeds: &[&[u8]] = &[
            b"vault",
            state_key.as_ref(),
            &[ctx.accounts.state.vault_bump],
        ];
        let signer = &[seeds];

        let cpi_accounts = Transfer {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.user.to_account_info(),
        };
        let cpi = CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            cpi_accounts,
            signer,
        );
        transfer(cpi, amount)
    }

    pub fn close(ctx: Context<Close>) -> Result<()> {
        let state_key = ctx.accounts.state.key();
        let bump = ctx.accounts.state.vault_bump;
        let seeds: &[&[u8]] = &[b"vault", state_key.as_ref(), &[bump]];
        let signer = &[seeds];

        let lamports = ctx.accounts.vault.lamports();
        let cpi_accounts = Transfer {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.user.to_account_info(),
        };
        let cpi = CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            cpi_accounts,
            signer,
        );
        transfer(cpi, lamports)
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init,
        payer = user,
        seeds = [b"state", user.key().as_ref()],
        bump,
        space = 8 + VaultState::INIT_SPACE,
    )]
    pub state: Account<'info, VaultState>,

    #[account(
        mut,
        seeds = [b"vault", state.key().as_ref()],
        bump,
    )]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Payment<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [b"state", user.key().as_ref()],
        bump = state.state_bump,
    )]
    pub state: Account<'info, VaultState>,

    #[account(
        mut,
        seeds = [b"vault", state.key().as_ref()],
        bump = state.vault_bump,
    )]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Close<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        close = user,
        seeds = [b"state", user.key().as_ref()],
        bump = state.state_bump,
    )]
    pub state: Account<'info, VaultState>,

    #[account(
        mut,
        seeds = [b"vault", state.key().as_ref()],
        bump = state.vault_bump,
    )]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[account]
#[derive(InitSpace)]
pub struct VaultState {
    pub state_bump: u8,
    pub vault_bump: u8,
}
