//! Program state processor

use {
    crate::{
        error::FeatureGateError, feature_id::derive_feature_id, instruction::FeatureGateInstruction,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        feature::Feature,
        msg,
        program::{invoke, invoke_signed},
        program_error::ProgramError,
        pubkey::Pubkey,
        system_instruction, system_program,
    },
};

/// Processes an [ActivateFeature](enum.FeatureGateInstruction.html)
/// instruction.
pub fn process_activate_feature(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let feature_info = next_account_info(account_info_iter)?;
    let _system_program_info = next_account_info(account_info_iter)?;

    if !feature_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if feature_info.owner != &system_program::id() {
        return Err(FeatureGateError::InvalidFeatureAccount.into());
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

/// Processes an
/// [ActivateFeatureWithAuthority](enum.FeatureGateInstruction.html)
/// instruction.
pub fn process_activate_feature_with_authority(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    nonce: u16,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let feature_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let _system_program_info = next_account_info(account_info_iter)?;

    if !authority_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if feature_info.owner != &system_program::id() {
        return Err(FeatureGateError::InvalidFeatureAccount.into());
    }

    let (feature_id, feature_id_bump) = derive_feature_id(authority_info.key, nonce)?;

    if feature_info.key != &feature_id {
        return Err(FeatureGateError::IncorrectFeatureId.into());
    }

    invoke_signed(
        &system_instruction::allocate(feature_info.key, Feature::size_of() as u64),
        &[feature_info.clone()],
        &[&[
            b"feature",
            &nonce.to_le_bytes(),
            authority_info.key.as_ref(),
            &[feature_id_bump],
        ]],
    )?;
    invoke_signed(
        &system_instruction::assign(feature_info.key, program_id),
        &[feature_info.clone()],
        &[&[
            b"feature",
            &nonce.to_le_bytes(),
            authority_info.key.as_ref(),
            &[feature_id_bump],
        ]],
    )?;

    Ok(())
}

/// Processes a [RevokePendingActivation](enum.FeatureGateInstruction.html)
/// instruction.
pub fn process_revoke_pending_activation(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let feature_info = next_account_info(account_info_iter)?;
    let destination_info = next_account_info(account_info_iter)?;

    if !feature_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // This will also check the program ID
    if Feature::from_account_info(feature_info)?
        .activated_at
        .is_some()
    {
        return Err(FeatureGateError::FeatureAlreadyActivated.into());
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
        FeatureGateInstruction::ActivateFeature => {
            msg!("Instruction: ActivateFeature");
            process_activate_feature(program_id, accounts)
        }
        FeatureGateInstruction::ActivateFeatureWithAuthority { nonce } => {
            msg!("Instruction: ActivateFeatureWithAuthority");
            process_activate_feature_with_authority(program_id, accounts, nonce)
        }
        FeatureGateInstruction::RevokePendingActivation => {
            msg!("Instruction: RevokePendingActivation");
            process_revoke_pending_activation(program_id, accounts)
        }
    }
}
