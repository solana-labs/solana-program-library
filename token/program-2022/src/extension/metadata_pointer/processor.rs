use {
    crate::{
        check_program_account,
        error::TokenError,
        extension::{
            metadata_pointer::{
                instruction::{
                    InitializeInstructionData, MetadataPointerInstruction, UpdateInstructionData,
                },
                MetadataPointer,
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
    metadata_address: &OptionalNonZeroPubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;
    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack_uninitialized(&mut mint_data)?;

    let extension = mint.init_extension::<MetadataPointer>(true)?;
    extension.authority = *authority;

    if Option::<Pubkey>::from(*authority).is_none()
        && Option::<Pubkey>::from(*metadata_address).is_none()
    {
        msg!("The metadata pointer extension requires at least an authority or an address for initialization, neither was provided");
        Err(TokenError::InvalidInstruction)?;
    }
    extension.metadata_address = *metadata_address;
    Ok(())
}

fn process_update(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_metadata_address: &OptionalNonZeroPubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;
    let owner_info = next_account_info(account_info_iter)?;
    let owner_info_data_len = owner_info.data_len();

    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack(&mut mint_data)?;
    let extension = mint.get_extension_mut::<MetadataPointer>()?;
    let authority =
        Option::<Pubkey>::from(extension.authority).ok_or(TokenError::NoAuthorityExists)?;

    Processor::validate_owner(
        program_id,
        &authority,
        owner_info,
        owner_info_data_len,
        account_info_iter.as_slice(),
    )?;

    extension.metadata_address = *new_metadata_address;
    Ok(())
}

pub(crate) fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    check_program_account(program_id)?;
    match decode_instruction_type(input)? {
        MetadataPointerInstruction::Initialize => {
            msg!("MetadataPointerInstruction::Initialize");
            let InitializeInstructionData {
                authority,
                metadata_address,
            } = decode_instruction_data(input)?;
            process_initialize(program_id, accounts, authority, metadata_address)
        }
        MetadataPointerInstruction::Update => {
            msg!("MetadataPointerInstruction::Update");
            let UpdateInstructionData { metadata_address } = decode_instruction_data(input)?;
            process_update(program_id, accounts, metadata_address)
        }
    }
}
