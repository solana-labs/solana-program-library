use {
    crate::{
        check_program_account,
        error::TokenError,
        extension::{
            group_member_pointer::{
                instruction::{
                    GroupMemberPointerInstruction, InitializeInstructionData, UpdateInstructionData,
                },
                GroupMemberPointer,
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
        pubkey::Pubkey,
    },
    spl_pod::optional_keys::OptionalNonZeroPubkey,
};

fn process_initialize(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    authority: &OptionalNonZeroPubkey,
    member_address: &OptionalNonZeroPubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;
    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack_uninitialized(&mut mint_data)?;

    if Option::<Pubkey>::from(*authority).is_none()
        && Option::<Pubkey>::from(*member_address).is_none()
    {
        msg!(
            "The group member pointer extension requires at least an authority or an address for \
            initialization, neither was provided"
        );
        Err(TokenError::InvalidInstruction)?;
    }

    let extension = mint.init_extension::<GroupMemberPointer>(true)?;
    extension.authority = *authority;
    extension.member_address = *member_address;
    Ok(())
}

fn process_update(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_member_address: &OptionalNonZeroPubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;
    let owner_info = next_account_info(account_info_iter)?;
    let owner_info_data_len = owner_info.data_len();

    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack(&mut mint_data)?;
    let extension = mint.get_extension_mut::<GroupMemberPointer>()?;
    let authority =
        Option::<Pubkey>::from(extension.authority).ok_or(TokenError::NoAuthorityExists)?;

    Processor::validate_owner(
        program_id,
        &authority,
        owner_info,
        owner_info_data_len,
        account_info_iter.as_slice(),
    )?;

    extension.member_address = *new_member_address;
    Ok(())
}

pub(crate) fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    check_program_account(program_id)?;
    match decode_instruction_type(input)? {
        GroupMemberPointerInstruction::Initialize => {
            msg!("GroupMemberPointerInstruction::Initialize");
            let InitializeInstructionData {
                authority,
                member_address,
            } = decode_instruction_data(input)?;
            process_initialize(program_id, accounts, authority, member_address)
        }
        GroupMemberPointerInstruction::Update => {
            msg!("GroupMemberPointerInstruction::Update");
            let UpdateInstructionData { member_address } = decode_instruction_data(input)?;
            process_update(program_id, accounts, member_address)
        }
    }
}
