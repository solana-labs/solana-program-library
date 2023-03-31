//! Program state processor

use {
    crate::{
        collect_validate_state_signer_seeds,
        error::PermissionedTransferError,
        get_validate_state_address, get_validate_state_address_and_bump_seed, inline_spl_token,
        instruction::PermissionedTransferInstruction,
        state::{ValidationPubkeys, MAX_NUM_KEYS},
        tlv::{TlvState, TlvStateBorrowed, TlvStateMut},
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program::invoke_signed,
        program_error::ProgramError,
        pubkey::Pubkey,
        system_instruction,
    },
    std::mem::size_of,
};

/// Processes a [Validate](enum.PermissionedTransferInstruction.html) instruction.
pub fn process_validate(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let _source_account_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let _destination_account_info = next_account_info(account_info_iter)?;
    let _authority_info = next_account_info(account_info_iter)?;
    let validation_pubkeys_info = next_account_info(account_info_iter)?;

    // For the example program, we just check that the correct pda and validation
    // pubkeys are provided
    let expected_validation_address = get_validate_state_address(mint_info.key, program_id);
    if expected_validation_address != *validation_pubkeys_info.key {
        return Err(ProgramError::InvalidSeeds);
    }

    let data = validation_pubkeys_info.try_borrow_data()?;
    let state = TlvStateBorrowed::unpack(&data).unwrap();
    let validation_pubkeys = state.get_value::<ValidationPubkeys>()?;

    // if too many keys are provided, error
    if account_info_iter.count() > validation_pubkeys.length as usize {
        return Err(PermissionedTransferError::TooManyPubkeys.into());
    }

    // Let's assume that they're provided in the correct order
    for (i, account_info) in account_info_iter.enumerate() {
        if *account_info.key != validation_pubkeys.pubkeys[i] {
            return Err(PermissionedTransferError::IncorrectAccount.into());
        }
    }

    Ok(())
}

/// Processes a [InitializeValidationPubkeys](enum.PermissionedTransferInstruction.html) instruction.
pub fn process_initialize_validation_pubkeys(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let validation_pubkeys_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let _system_program_info = next_account_info(account_info_iter)?;

    // check that the mint authority is valid without fully deserializing
    let mint_authority = inline_spl_token::get_mint_authority(&mint_info.try_borrow_data()?)?;
    let mint_authority = mint_authority.ok_or(PermissionedTransferError::MintHasNoMintAuthority)?;

    // Check signers
    if !authority_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if *authority_info.key != mint_authority {
        return Err(PermissionedTransferError::IncorrectMintAuthority.into());
    }

    // Check validation account
    let (expected_validation_address, bump_seed) =
        get_validate_state_address_and_bump_seed(mint_info.key, program_id);
    if expected_validation_address != *validation_pubkeys_info.key {
        return Err(ProgramError::InvalidSeeds);
    }

    // Create the account
    let bump_seed = [bump_seed];
    let signer_seeds = collect_validate_state_signer_seeds(mint_info.key, &bump_seed);
    const VALIDATION_SIZE: u64 = size_of::<ValidationPubkeys>() as u64;
    invoke_signed(
        &system_instruction::allocate(validation_pubkeys_info.key, VALIDATION_SIZE),
        &[validation_pubkeys_info.clone()],
        &[&signer_seeds],
    )?;
    invoke_signed(
        &system_instruction::assign(validation_pubkeys_info.key, program_id),
        &[validation_pubkeys_info.clone()],
        &[&signer_seeds],
    )?;

    // Write the data
    let mut data = validation_pubkeys_info.try_borrow_mut_data()?;
    let mut state = TlvStateMut::unpack(&mut data).unwrap();
    let mut validation_pubkeys = state.init_value::<ValidationPubkeys>(false)?;
    let length = account_info_iter.count();
    if length > MAX_NUM_KEYS {
        return Err(PermissionedTransferError::TooManyPubkeys.into());
    }
    validation_pubkeys.length = length
        .try_into()
        .map_err(|_| ProgramError::from(PermissionedTransferError::CalculationFailure))?;
    for (i, account_info) in account_info_iter.enumerate() {
        validation_pubkeys.pubkeys[i] = *account_info.key;
    }

    Ok(())
}

/// Processes an [Instruction](enum.Instruction.html).
pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
    let instruction = PermissionedTransferInstruction::unpack(input)?;

    match instruction {
        PermissionedTransferInstruction::Validate { amount } => {
            msg!("Instruction: Validate");
            process_validate(program_id, accounts, amount)
        }
        PermissionedTransferInstruction::InitializeValidationPubkeys => {
            msg!("Instruction: InitializeValidationPubkeys");
            process_initialize_validation_pubkeys(program_id, accounts)
        }
    }
}
