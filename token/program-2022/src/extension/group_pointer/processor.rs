use {
    crate::{
        check_program_account,
        error::TokenError,
        extension::{
            group_pointer::{
                instruction::{
                    GroupPointerInstruction, InitializeInstructionData, UpdateInstructionData,
                },
                GroupPointer,
            },
            StateWithExtensionsMut,
        },
        instruction::{decode_instruction_data, decode_instruction_type},
        processor::Processor,
        state::Mint,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        pubkey::Pubkey,
    },
    spl_pod::optional_keys::OptionalNonZeroPubkey,
};

fn process_initialize(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    authority: &OptionalNonZeroPubkey,
    group_address: &OptionalNonZeroPubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;
    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut mint_data)?;

    if Option::<Pubkey>::from(*authority).is_none()
        && Option::<Pubkey>::from(*group_address).is_none()
    {
        msg!(
            "The group pointer extension requires at least an authority or an address for \
             initialization, neither was provided"
        );
        Err(TokenError::InvalidInstruction)?;
    }

    let extension = mint.init_extension::<GroupPointer>(true)?;
    extension.authority = *authority;
    extension.group_address = *group_address;
    Ok(())
}

fn process_update(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_group_address: &OptionalNonZeroPubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;
    let owner_info = next_account_info(account_info_iter)?;
    let owner_info_data_len = owner_info.data_len();

    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = StateWithExtensionsMut::<Mint>::unpack(&mut mint_data)?;
    let extension = mint.get_extension_mut::<GroupPointer>()?;
    let authority =
        Option::<Pubkey>::from(extension.authority).ok_or(TokenError::NoAuthorityExists)?;

    Processor::validate_owner(
        program_id,
        &authority,
        owner_info,
        owner_info_data_len,
        account_info_iter.as_slice(),
    )?;

    extension.group_address = *new_group_address;
    Ok(())
}

pub(crate) fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    check_program_account(program_id)?;
    match decode_instruction_type(input)? {
        GroupPointerInstruction::Initialize => {
            msg!("GroupPointerInstruction::Initialize");
            let InitializeInstructionData {
                authority,
                group_address,
            } = decode_instruction_data(input)?;
            process_initialize(program_id, accounts, authority, group_address)
        }
        GroupPointerInstruction::Update => {
            msg!("GroupPointerInstruction::Update");
            let UpdateInstructionData { group_address } = decode_instruction_data(input)?;
            process_update(program_id, accounts, group_address)
        }
    }
}
