//! Program state processor

use {
    crate::{
        collect_extra_account_metas_signer_seeds,
        error::PermissionedTransferError,
        get_extra_account_metas_address, get_extra_account_metas_address_and_bump_seed,
        inline_spl_token,
        instruction::PermissionedTransferInstruction,
        pod::PodAccountMeta,
        state::ExtraAccountMetas,
        tlv::{get_base_len, TlvState, TlvStateBorrowed, TlvStateMut},
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
    let extra_account_metas_info = next_account_info(account_info_iter)?;

    // For the example program, we just check that the correct pda and validation
    // pubkeys are provided
    let expected_validation_address = get_extra_account_metas_address(mint_info.key, program_id);
    if expected_validation_address != *extra_account_metas_info.key {
        return Err(ProgramError::InvalidSeeds);
    }

    let data = extra_account_metas_info.try_borrow_data()?;
    let state = TlvStateBorrowed::unpack(&data).unwrap();
    let bytes = state.get_bytes::<ExtraAccountMetas>()?;
    let extra_account_metas = ExtraAccountMetas::unpack(bytes)?;

    // if incorrect number of are provided, error
    let extra_account_infos = account_info_iter.as_slice();
    let account_metas = extra_account_metas.data();
    if extra_account_infos.len() != account_metas.len() {
        return Err(PermissionedTransferError::IncorrectAccount.into());
    }

    // Let's assume that they're provided in the correct order
    for (i, account_info) in extra_account_infos.iter().enumerate() {
        if &account_metas[i] != account_info {
            return Err(PermissionedTransferError::IncorrectAccount.into());
        }
    }

    Ok(())
}

/// Processes a [InitializeExtraAccountMetas](enum.PermissionedTransferInstruction.html) instruction.
pub fn process_initialize_extra_account_metas(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let extra_account_metas_info = next_account_info(account_info_iter)?;
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
        get_extra_account_metas_address_and_bump_seed(mint_info.key, program_id);
    if expected_validation_address != *extra_account_metas_info.key {
        return Err(ProgramError::InvalidSeeds);
    }

    // Create the account
    let bump_seed = [bump_seed];
    let signer_seeds = collect_extra_account_metas_signer_seeds(mint_info.key, &bump_seed);
    let extra_account_infos = account_info_iter.as_slice();
    let length = extra_account_infos.len();
    let tlv_size = ExtraAccountMetas::byte_size_of(length)?;
    let account_size = get_base_len()
        .checked_add(tlv_size)
        .ok_or(PermissionedTransferError::CalculationFailure)? as u64;
    invoke_signed(
        &system_instruction::allocate(extra_account_metas_info.key, account_size),
        &[extra_account_metas_info.clone()],
        &[&signer_seeds],
    )?;
    invoke_signed(
        &system_instruction::assign(extra_account_metas_info.key, program_id),
        &[extra_account_metas_info.clone()],
        &[&signer_seeds],
    )?;

    // Write the data
    let mut data = extra_account_metas_info.try_borrow_mut_data()?;
    let mut state = TlvStateMut::unpack(&mut data).unwrap();
    let bytes = state.allocate::<ExtraAccountMetas>(tlv_size)?;
    let mut extra_account_metas = ExtraAccountMetas::init(bytes)?;
    for account_info in extra_account_infos {
        extra_account_metas.push(PodAccountMeta::from(account_info))?;
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
        PermissionedTransferInstruction::InitializeExtraAccountMetas => {
            msg!("Instruction: InitializeExtraAccountMetas");
            process_initialize_extra_account_metas(program_id, accounts)
        }
    }
}
