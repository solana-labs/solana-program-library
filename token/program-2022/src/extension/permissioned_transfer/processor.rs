use {
    crate::{
        check_program_account,
        error::TokenError,
        extension::{
            permissioned_transfer::{
                instruction::{
                    InitializeInstructionData, PermissionedTransferInstruction,
                    UpdateInstructionData,
                },
                PermissionedTransfer,
            },
            StateWithExtensionsMut,
        },
        instruction::{decode_instruction_data, decode_instruction_type},
        pod::OptionalNonZeroPubkey,
        processor::Processor,
        state::Mint,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
    },
};

fn process_initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    authority: &OptionalNonZeroPubkey,
    permissioned_transfer_program_id: &OptionalNonZeroPubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;
    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut mint_data)?;

    let extension = mint.init_extension::<PermissionedTransfer>(true)?;
    extension.authority = *authority;

    if let Some(permissioned_transfer_program_id) =
        Option::<Pubkey>::from(*permissioned_transfer_program_id)
    {
        if permissioned_transfer_program_id == *program_id {
            return Err(ProgramError::IncorrectProgramId);
        }
    }
    extension.permissioned_transfer_program_id = *permissioned_transfer_program_id;
    Ok(())
}

fn process_update(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_program_id: &OptionalNonZeroPubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;
    let owner_info = next_account_info(account_info_iter)?;
    let owner_info_data_len = owner_info.data_len();

    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = StateWithExtensionsMut::<Mint>::unpack(&mut mint_data)?;
    let extension = mint.get_extension_mut::<PermissionedTransfer>()?;
    let authority =
        Option::<Pubkey>::from(extension.authority).ok_or(TokenError::NoAuthorityExists)?;

    Processor::validate_owner(
        program_id,
        &authority,
        owner_info,
        owner_info_data_len,
        account_info_iter.as_slice(),
    )?;

    if let Some(new_program_id) = Option::<Pubkey>::from(*new_program_id) {
        if new_program_id == *program_id {
            return Err(ProgramError::IncorrectProgramId);
        }
    }

    extension.permissioned_transfer_program_id = *new_program_id;
    Ok(())
}

pub(crate) fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    check_program_account(program_id)?;
    match decode_instruction_type(input)? {
        PermissionedTransferInstruction::Initialize => {
            msg!("PermissionedTransferInstruction::Initialize");
            let InitializeInstructionData {
                authority,
                permissioned_transfer_program_id,
            } = decode_instruction_data(input)?;
            process_initialize(
                program_id,
                accounts,
                authority,
                permissioned_transfer_program_id,
            )
        }
        PermissionedTransferInstruction::Update => {
            msg!("PermissionedTransferInstruction::Update");
            let UpdateInstructionData {
                permissioned_transfer_program_id,
            } = decode_instruction_data(input)?;
            process_update(program_id, accounts, permissioned_transfer_program_id)
        }
    }
}
