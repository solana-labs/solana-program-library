use {
    crate::{
        check_program_account,
        extension::{
            confidential_transfer::{instruction::*, *},
            StateWithExtensions, StateWithExtensionsMut,
        },
        id,
        processor::Processor,
        state::{Account, Mint},
        tools::account::create_pda_account,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program_error::ProgramError,
        program_pack::Pack,
        pubkey::Pubkey,
        sysvar::{rent::Rent, Sysvar},
    },
};

/// Processes an [InitializeAuditor] instruction.
fn process_initialize_auditor(
    accounts: &[AccountInfo],
    auditor: &ConfidentialTransferAuditor,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_info = next_account_info(account_info_iter)?;

    check_program_account(mint_info.owner)?;
    let mint_data = &mut mint_info.data.borrow_mut();
    let mut mint = StateWithExtensionsMut::<Mint>::unpack_uninitialized(mint_data)?;
    *mint.init_extension::<ConfidentialTransferAuditor>()? = *auditor;

    Ok(())
}

/// Processes a [ConfigureOmnibusAccount] instruction.
fn process_configure_omnibus_account(accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let funder_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let omnibus_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;

    check_program_account(mint_info.owner)?;
    let _mint = StateWithExtensions::<Mint>::unpack(&mint_info.data.borrow())?;

    let (omnibus_token_address, omnibus_token_bump_seed) =
        get_omnibus_token_address_with_seed(mint_info.key);

    if omnibus_token_address != *omnibus_info.key {
        msg!("Error: Omnibus token address does not match derivation");
        return Err(ProgramError::InvalidArgument);
    }

    let omnibus_token_account_signer_seeds: &[&[_]] = &[
        &mint_info.key.to_bytes(),
        br"confidential_transfer_omnibus",
        &[omnibus_token_bump_seed],
    ];

    create_pda_account(
        funder_info,
        &Rent::get()?,
        Account::get_packed_len(),
        &id(),
        system_program_info,
        omnibus_info,
        omnibus_token_account_signer_seeds,
    )?;

    Processor::process(
        &id(),
        &[omnibus_info.clone(), mint_info.clone()],
        &crate::instruction::initialize_account3(
            &id(),
            omnibus_info.key,
            mint_info.key,
            omnibus_info.key,
        )?
        .data,
    )
}

/// Processes an [UpdateAuditor] instruction.
fn process_update_auditor(
    accounts: &[AccountInfo],
    new_auditor: &ConfidentialTransferAuditor,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let new_authority_info = next_account_info(account_info_iter)?;

    check_program_account(mint_info.owner)?;
    let mint_data = &mut mint_info.data.borrow_mut();
    let mut mint = StateWithExtensionsMut::<Mint>::unpack(mint_data)?;
    let auditor = mint.get_extension_mut::<ConfidentialTransferAuditor>()?;

    if authority_info.is_signer
        && (new_authority_info.is_signer || *new_authority_info.key == Pubkey::default())
    {
        if new_auditor.authority == *new_authority_info.key {
            *auditor = *new_auditor;
            Ok(())
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    } else {
        Err(ProgramError::MissingRequiredSignature)
    }
}

/// Processes an [ApproveAccount] instruction.
fn process_approve_account(accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let account_to_approve_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;

    check_program_account(account_to_approve_info.owner)?;
    let account_to_approve_data = &mut account_to_approve_info.data.borrow_mut();
    let mut account_to_approve = StateWithExtensionsMut::<Mint>::unpack(account_to_approve_data)?;

    check_program_account(mint_info.owner)?;
    let mint_data = &mint_info.data.borrow_mut();
    let mint = StateWithExtensions::<Mint>::unpack(mint_data)?;
    let auditor = mint.get_extension::<ConfidentialTransferAuditor>()?;

    if authority_info.is_signer && *authority_info.key == auditor.authority {
        let mut confidential_transfer_state =
            account_to_approve.get_extension_mut::<ConfidentialTransferState>()?;
        confidential_transfer_state.approved = true.into();
        Ok(())
    } else {
        Err(ProgramError::MissingRequiredSignature)
    }
}

pub(crate) fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    check_program_account(program_id)?;

    match decode_instruction_type(input)? {
        ConfidentialTransferInstruction::InitializeAuditor => {
            msg!("ConfidentialTransferInstruction::InitializeAuditor");
            process_initialize_auditor(
                accounts,
                decode_instruction_data::<ConfidentialTransferAuditor>(input)?,
            )
        }
        ConfidentialTransferInstruction::ConfigureOmnibusAccount => {
            msg!("ConfidentialTransferInstruction::ConfigureOmnibusAccount");
            process_configure_omnibus_account(accounts)
        }
        ConfidentialTransferInstruction::UpdateAuditor => {
            msg!("ConfidentialTransferInstruction::UpdateAuditor");
            process_update_auditor(
                accounts,
                decode_instruction_data::<ConfidentialTransferAuditor>(input)?,
            )
        }
        ConfidentialTransferInstruction::ApproveAccount => {
            msg!("ConfidentialTransferInstruction::ApproveAccount");
            process_approve_account(accounts)
        } // TODO: add remaining `zk_token_program::processor.rs` here
    }
}
