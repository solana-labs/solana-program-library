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
        rent::Rent,
        system_instruction, system_program,
        sysvar::Sysvar,
    },
};

fn fund_feature_account<'a>(
    feature_info: &AccountInfo<'a>,
    payer_info: &AccountInfo<'a>,
    space: u64,
) -> ProgramResult {
    let rent = Rent::get()?;

    // Just in case the account already has some lamports
    let required_lamports = rent
        .minimum_balance(space as usize)
        .max(1)
        .saturating_sub(feature_info.lamports());

    if required_lamports > 0 {
        invoke(
            &system_instruction::transfer(payer_info.key, feature_info.key, required_lamports),
            &[payer_info.clone(), feature_info.clone()],
        )?;
    }

    Ok(())
}

/// Activates a feature account with a keypair.
fn activate_feature_with_keypair<'a>(
    program_id: &Pubkey,
    feature_info: &AccountInfo<'a>,
    payer_info: &AccountInfo<'a>,
) -> ProgramResult {
    let space = Feature::size_of() as u64;

    fund_feature_account(feature_info, payer_info, space)?;

    invoke(
        &system_instruction::allocate(feature_info.key, space),
        &[feature_info.clone()],
    )?;

    invoke(
        &system_instruction::assign(feature_info.key, program_id),
        &[feature_info.clone()],
    )?;

    Ok(())
}

/// Activates a feature account with an authority.
fn activate_feature_with_authority<'a>(
    program_id: &Pubkey,
    feature_info: &AccountInfo<'a>,
    payer_info: &AccountInfo<'a>,
    authority_info: &AccountInfo<'a>,
    nonce: u16,
) -> ProgramResult {
    let (feature_id, feature_id_bump) = derive_feature_id(authority_info.key, nonce)?;

    if feature_info.key != &feature_id {
        return Err(FeatureGateError::IncorrectFeatureId.into());
    }

    let space = Feature::size_of() as u64;

    fund_feature_account(feature_info, payer_info, space)?;

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

/// Processes an [ActivateFeature](enum.FeatureGateInstruction.html)
/// instruction.
pub fn process_activate_feature(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    nonce: Option<u16>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let feature_info = next_account_info(account_info_iter)?;
    let payer_info = next_account_info(account_info_iter)?;
    let _system_program_info = next_account_info(account_info_iter)?;

    // Check if activation is being done by feature keypair or by authority
    if let Ok(authority_info) = next_account_info(account_info_iter) {
        // A nonce should be provided if an authority has been provided
        let nonce = nonce.ok_or(FeatureGateError::MissingNonce)?;

        if !authority_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if feature_info.owner != &system_program::id() {
            return Err(FeatureGateError::InvalidFeatureAccount.into());
        }

        activate_feature_with_authority(
            program_id,
            feature_info,
            payer_info,
            authority_info,
            nonce,
        )?;
    } else {
        if !feature_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if feature_info.owner != &system_program::id() {
            return Err(FeatureGateError::InvalidFeatureAccount.into());
        }

        activate_feature_with_keypair(program_id, feature_info, payer_info)?;
    }

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
        FeatureGateInstruction::ActivateFeature { nonce } => {
            msg!("Instruction: ActivateFeature");
            process_activate_feature(program_id, accounts, nonce)
        }
        FeatureGateInstruction::RevokePendingActivation => {
            msg!("Instruction: RevokePendingActivation");
            process_revoke_pending_activation(program_id, accounts)
        }
    }
}
