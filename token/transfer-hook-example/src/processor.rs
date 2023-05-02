//! Program state processor

use {
    crate::inline_spl_token,
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program::invoke_signed,
        program_error::ProgramError,
        pubkey::Pubkey,
        system_instruction,
    },
    spl_tlv_account_resolution::state::ExtraAccountMetas,
    spl_transfer_hook_interface::{
        collect_extra_account_metas_signer_seeds,
        error::TransferHookError,
        get_extra_account_metas_address, get_extra_account_metas_address_and_bump_seed,
        instruction::{ExecuteInstruction, TransferHookInstruction},
    },
    spl_type_length_value::state::TlvStateBorrowed,
};

/// Processes an [Execute](enum.TransferHookInstruction.html) instruction.
pub fn process_execute(
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
    let extra_account_metas =
        ExtraAccountMetas::unpack_with_tlv_state::<ExecuteInstruction>(&state)?;

    // if incorrect number of are provided, error
    let extra_account_infos = account_info_iter.as_slice();
    let account_metas = extra_account_metas.data();
    if extra_account_infos.len() != account_metas.len() {
        return Err(TransferHookError::IncorrectAccount.into());
    }

    // Let's assume that they're provided in the correct order
    for (i, account_info) in extra_account_infos.iter().enumerate() {
        if &account_metas[i] != account_info {
            return Err(TransferHookError::IncorrectAccount.into());
        }
    }

    Ok(())
}

/// Processes a [InitializeExtraAccountMetas](enum.TransferHookInstruction.html) instruction.
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
    let mint_authority = mint_authority.ok_or(TransferHookError::MintHasNoMintAuthority)?;

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
    let extra_account_infos = account_info_iter.as_slice();
    let length = extra_account_infos.len();
    let account_size = ExtraAccountMetas::size_of(length)?;
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
    ExtraAccountMetas::init_with_account_infos::<ExecuteInstruction>(
        &mut data,
        extra_account_infos,
    )?;

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
        TransferHookInstruction::InitializeExtraAccountMetas => {
            msg!("Instruction: InitializeExtraAccountMetas");
            process_initialize_extra_account_metas(program_id, accounts)
        }
    }
}
