use anchor_lang::prelude::*;
use anchor_lang::InstructionData;
use anchor_lang::solana_program::sysvar::instructions::{
    load_current_index_checked, load_instruction_at_checked, ID as INSTRUCTIONS_SYSVAR_ID,
};

use crate::error::AmmError;

fn burn_lp_discriminator() -> [u8; 8] {
    let data = crate::instruction::BurnLp { lp_amount: 0 }.data();
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&data[..8]);
    disc
}

/// Validates that the instruction immediately before the current one is a matching
/// `burn_lp` call from this program with the expected accounts and amount.
pub fn verify_previous_burn_lp(
    instructions: &AccountInfo,
    user: &Pubkey,
    config: &Pubkey,
    mint_lp: &Pubkey,
    user_ata_lp: &Pubkey,
) -> Result<u64> {
    require_keys_eq!(*instructions.key, INSTRUCTIONS_SYSVAR_ID);

    let current_index = load_current_index_checked(instructions)?;
    require!(current_index > 0, AmmError::MissingPreviousInstruction);

    let prev_ix = load_instruction_at_checked(current_index as usize - 1, instructions)?;
    let expected_disc = burn_lp_discriminator();

    require_keys_eq!(prev_ix.program_id, crate::ID, AmmError::InvalidProgram);
    require!(
        prev_ix.data.len() >= 16,
        AmmError::InvalidInstructionData
    );
    require!(
        prev_ix.data[0..8] == expected_disc,
        AmmError::InvalidInstruction
    );

    let lp_amount = u64::from_le_bytes(
        prev_ix.data[8..16]
            .try_into()
            .map_err(|_| error!(AmmError::InvalidInstructionData))?,
    );
    require!(lp_amount > 0, AmmError::ZeroAmount);

    require!(prev_ix.accounts.len() >= 6, AmmError::InvalidInstructionAccounts);
    require_keys_eq!(prev_ix.accounts[0].pubkey, *user, AmmError::InvalidInstructionAccounts);
    require!(
        prev_ix.accounts[0].is_signer,
        AmmError::InvalidInstructionAccounts
    );
    require_keys_eq!(prev_ix.accounts[3].pubkey, *config, AmmError::InvalidInstructionAccounts);
    require_keys_eq!(prev_ix.accounts[4].pubkey, *mint_lp, AmmError::InvalidInstructionAccounts);
    require_keys_eq!(
        prev_ix.accounts[5].pubkey,
        *user_ata_lp,
        AmmError::InvalidInstructionAccounts
    );

    Ok(lp_amount)
}
