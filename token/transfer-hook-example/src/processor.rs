//! Program state processor

use {
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        instruction::AccountMeta,
        msg,
        program::invoke_signed,
        program_error::ProgramError,
        pubkey::Pubkey,
        system_instruction,
    },
    spl_tlv_account_resolution::{account::ExtraAccountMeta, state::ExtraAccountMetaList},
    spl_token_2022::{
        extension::{
            transfer_hook::TransferHookAccount, BaseStateWithExtensions, StateWithExtensions,
        },
        state::{Account, Mint},
    },
    spl_transfer_hook_interface::{
        collect_extra_account_metas_signer_seeds,
        error::TransferHookError,
        get_extra_account_metas_address, get_extra_account_metas_address_and_bump_seed,
        instruction::{ExecuteInstruction, TransferHookInstruction},
    },
    spl_type_length_value::state::TlvStateBorrowed,
};

fn check_token_account_is_transferring(account_info: &AccountInfo) -> Result<(), ProgramError> {
    let account_data = account_info.try_borrow_data()?;
    let token_account = StateWithExtensions::<Account>::unpack(&account_data)?;
    let extension = token_account.get_extension::<TransferHookAccount>()?;
    if bool::from(extension.transferring) {
        Ok(())
    } else {
        Err(TransferHookError::ProgramCalledOutsideOfTransfer.into())
    }
}

fn check_extra_meta(
    account_info: &AccountInfo,
    extra_meta: &AccountMeta,
) -> Result<(), ProgramError> {
    if !(&extra_meta.pubkey == account_info.key
        && extra_meta.is_signer == account_info.is_signer
        && extra_meta.is_writable == account_info.is_writable)
    {
        return Err(TransferHookError::IncorrectAccount.into());
    }
    Ok(())
}

fn next_extra_account_meta<'a>(
    extra_account_metas_iter: &mut impl Iterator<Item = &'a ExtraAccountMeta>,
) -> Result<&'a ExtraAccountMeta, ProgramError> {
    extra_account_metas_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)
}

/// Processes an [Execute](enum.TransferHookInstruction.html) instruction.
pub fn process_execute(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let source_account_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let destination_account_info = next_account_info(account_info_iter)?;
    let _authority_info = next_account_info(account_info_iter)?;
    let extra_account_metas_info = next_account_info(account_info_iter)?;

    // Check that the accounts are properly in "transferring" mode
    check_token_account_is_transferring(source_account_info)?;
    check_token_account_is_transferring(destination_account_info)?;

    // For the example program, we just check that the correct pda and validation
    // pubkeys are provided
    let expected_validation_address = get_extra_account_metas_address(mint_info.key, program_id);
    if expected_validation_address != *extra_account_metas_info.key {
        return Err(ProgramError::InvalidSeeds);
    }

    let data = extra_account_metas_info.try_borrow_data()?;
    let state = TlvStateBorrowed::unpack(&data).unwrap();
    let extra_account_metas =
        ExtraAccountMetaList::unpack_with_tlv_state::<ExecuteInstruction>(&state)?;
    let account_metas = extra_account_metas.data();

    // if incorrect number of are provided, error
    if account_metas.len() != account_info_iter.len() {
        return Err(TransferHookError::IncorrectAccount.into());
    }

    // Let's assume that they're provided in the correct order
    let extra_account_metas_iter = &mut account_metas.iter();
    check_extra_meta(
        next_account_info(account_info_iter)?,
        &AccountMeta::try_from(next_extra_account_meta(extra_account_metas_iter)?)?,
    )?;
    check_extra_meta(
        next_account_info(account_info_iter)?,
        &AccountMeta::try_from(next_extra_account_meta(extra_account_metas_iter)?)?,
    )?;
    let next_config = next_extra_account_meta(extra_account_metas_iter)?;
    check_extra_meta(
        next_account_info(account_info_iter)?,
        &AccountMeta {
            pubkey: Pubkey::find_program_address(
                &[b"seed-prefix", source_account_info.key.as_ref()],
                program_id,
            )
            .0,
            is_signer: bool::from(next_config.is_signer),
            is_writable: bool::from(next_config.is_writable),
        },
    )?;
    let next_config = next_extra_account_meta(extra_account_metas_iter)?;
    check_extra_meta(
        next_account_info(account_info_iter)?,
        &AccountMeta {
            pubkey: Pubkey::find_program_address(
                &[&amount.to_le_bytes(), destination_account_info.key.as_ref()],
                program_id,
            )
            .0,
            is_signer: bool::from(next_config.is_signer),
            is_writable: bool::from(next_config.is_writable),
        },
    )?;
    check_extra_meta(
        next_account_info(account_info_iter)?,
        &AccountMeta::try_from(next_extra_account_meta(extra_account_metas_iter)?)?,
    )?;

    Ok(())
}

/// Processes a [InitializeExtraAccountMetaList](enum.TransferHookInstruction.html) instruction.
pub fn process_initialize_extra_account_meta_list(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    extra_account_metas: &[ExtraAccountMeta],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let extra_account_metas_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let _system_program_info = next_account_info(account_info_iter)?;

    // check that the mint authority is valid without fully deserializing
    let mint_data = mint_info.try_borrow_data()?;
    let mint = StateWithExtensions::<Mint>::unpack(&mint_data)?;
    let mint_authority = mint
        .base
        .mint_authority
        .ok_or(TransferHookError::MintHasNoMintAuthority)?;

    // Check signers
    if !authority_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if *authority_info.key != mint_authority {
        return Err(TransferHookError::IncorrectMintAuthority.into());
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
    let length = extra_account_metas.len();
    let account_size = ExtraAccountMetaList::size_of(length)?;
    invoke_signed(
        &system_instruction::allocate(extra_account_metas_info.key, account_size as u64),
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
    ExtraAccountMetaList::init::<ExecuteInstruction>(&mut data, extra_account_metas)?;

    Ok(())
}

/// Processes an [Instruction](enum.Instruction.html).
pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
    let instruction = TransferHookInstruction::unpack(input)?;

    match instruction {
        TransferHookInstruction::Execute { amount } => {
            msg!("Instruction: Execute");
            process_execute(program_id, accounts, amount)
        }
        TransferHookInstruction::InitializeExtraAccountMetaList {
            extra_account_metas,
        } => {
            msg!("Instruction: InitializeExtraAccountMetaList");
            process_initialize_extra_account_meta_list(program_id, accounts, &extra_account_metas)
        }
    }
}
