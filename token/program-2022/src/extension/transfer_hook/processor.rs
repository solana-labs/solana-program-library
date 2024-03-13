use {
    crate::{
        check_program_account,
        error::TokenError,
        extension::{
            transfer_hook::{
                instruction::{
                    InitializeInstructionData, TransferHookInstruction, UpdateInstructionData,
                },
                TransferHook,
            },
            BaseStateWithExtensionsMut, PodStateWithExtensionsMut,
        },
        instruction::{decode_instruction_data, decode_instruction_type},
        pod::PodMint,
        processor::Processor,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    spl_pod::optional_keys::OptionalNonZeroPubkey,
};

fn process_initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    authority: &OptionalNonZeroPubkey,
    transfer_hook_program_id: &OptionalNonZeroPubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;
    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack_uninitialized(&mut mint_data)?;

    let extension = mint.init_extension::<TransferHook>(true)?;
    extension.authority = *authority;

    if let Some(transfer_hook_program_id) = Option::<Pubkey>::from(*transfer_hook_program_id) {
        if transfer_hook_program_id == *program_id {
            return Err(ProgramError::IncorrectProgramId);
        }
    } else if Option::<Pubkey>::from(*authority).is_none() {
        msg!("The transfer hook extension requires at least an authority or a program id for initialization, neither was provided");
        Err(TokenError::InvalidInstruction)?;
    }
    extension.program_id = *transfer_hook_program_id;
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
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack(&mut mint_data)?;
    let extension = mint.get_extension_mut::<TransferHook>()?;
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

    extension.program_id = *new_program_id;
    Ok(())
}

pub(crate) fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    check_program_account(program_id)?;
    match decode_instruction_type(input)? {
        TransferHookInstruction::Initialize => {
            msg!("TransferHookInstruction::Initialize");
            let InitializeInstructionData {
                authority,
                program_id: transfer_hook_program_id,
            } = decode_instruction_data(input)?;
            process_initialize(program_id, accounts, authority, transfer_hook_program_id)
        }
        TransferHookInstruction::Update => {
            msg!("TransferHookInstruction::Update");
            let UpdateInstructionData {
                program_id: transfer_hook_program_id,
            } = decode_instruction_data(input)?;
            process_update(program_id, accounts, transfer_hook_program_id)
        }
    }
}
