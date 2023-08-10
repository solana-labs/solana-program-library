//! Program state processor

use {
    crate::{error::FeatureGateError, instruction::FeatureGateInstruction},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        feature::Feature,
        msg,
        program::invoke,
        program_error::ProgramError,
        pubkey::Pubkey,
        system_instruction, system_program,
    },
};

/// Processes an [Activate](enum.FeatureGateInstruction.html) instruction.
pub fn process_activate(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let feature_info = next_account_info(account_info_iter)?;
    let _system_program_info = next_account_info(account_info_iter)?;

    if !feature_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if feature_info.owner != &system_program::id() {
        return Err(FeatureGateError::FeatureNotSystemAccount.into());
    }

    invoke(
        &system_instruction::allocate(feature_info.key, Feature::size_of() as u64),
        &[feature_info.clone()],
    )?;
    invoke(
        &system_instruction::assign(feature_info.key, program_id),
        &[feature_info.clone()],
    )?;

    Ok(())
}

/// Processes an [revoke](enum.FeatureGateInstruction.html) instruction.
pub fn process_revoke(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let feature_info = next_account_info(account_info_iter)?;
    let destination_info = next_account_info(account_info_iter)?;

    if !feature_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if feature_info.owner != program_id {
        return Err(ProgramError::IllegalOwner);
    }

    if Feature::from_account_info(feature_info)?
        .activated_at
        .is_some()
    {
        return Err(FeatureGateError::FeatureNotInactive.into());
    }

    let new_destination_lamports = feature_info
        .lamports()
        .checked_add(destination_info.lamports())
        .ok_or::<ProgramError>(FeatureGateError::Overflow.into())?;

    **feature_info.try_borrow_mut_lamports()? = 0;
    **destination_info.try_borrow_mut_lamports()? = new_destination_lamports;

    feature_info.realloc(0, true)?;
    feature_info.assign(&system_program::id());

    Ok(())
}

/// Processes an [Instruction](enum.Instruction.html).
pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
    let instruction = FeatureGateInstruction::unpack(input)?;
    match instruction {
        FeatureGateInstruction::Activate => {
            msg!("Instruction: Activate");
            process_activate(program_id, accounts)
        }
        FeatureGateInstruction::RevokePendingActivation => {
            msg!("Instruction: RevokePendingActivation");
            process_revoke(program_id, accounts)
        }
    }
}
