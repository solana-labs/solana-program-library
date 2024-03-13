use {
    crate::{
        check_program_account,
        error::TokenError,
        extension::{
            interest_bearing_mint::{
                instruction::{InitializeInstructionData, InterestBearingMintInstruction},
                BasisPoints, InterestBearingConfig,
            },
            BaseStateWithExtensionsMut, PodStateWithExtensionsMut,
        },
        instruction::{decode_instruction_data, decode_instruction_type},
        pod::PodMint,
        processor::Processor,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        clock::Clock,
        entrypoint::ProgramResult,
        msg,
        pubkey::Pubkey,
        sysvar::Sysvar,
    },
    spl_pod::optional_keys::OptionalNonZeroPubkey,
};

fn process_initialize(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    rate_authority: &OptionalNonZeroPubkey,
    rate: &BasisPoints,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;
    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack_uninitialized(&mut mint_data)?;

    let clock = Clock::get()?;
    let extension = mint.init_extension::<InterestBearingConfig>(true)?;
    extension.rate_authority = *rate_authority;
    extension.initialization_timestamp = clock.unix_timestamp.into();
    extension.last_update_timestamp = clock.unix_timestamp.into();
    // There is no validation on the rate, since ridiculous values are *technically*
    // possible!
    extension.pre_update_average_rate = *rate;
    extension.current_rate = *rate;
    Ok(())
}

fn process_update_rate(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_rate: &BasisPoints,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;
    let owner_info = next_account_info(account_info_iter)?;
    let owner_info_data_len = owner_info.data_len();

    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack(&mut mint_data)?;
    let extension = mint.get_extension_mut::<InterestBearingConfig>()?;
    let rate_authority =
        Option::<Pubkey>::from(extension.rate_authority).ok_or(TokenError::NoAuthorityExists)?;

    Processor::validate_owner(
        program_id,
        &rate_authority,
        owner_info,
        owner_info_data_len,
        account_info_iter.as_slice(),
    )?;

    let clock = Clock::get()?;
    let new_average_rate = extension
        .time_weighted_average_rate(clock.unix_timestamp)
        .ok_or(TokenError::Overflow)?;
    extension.pre_update_average_rate = new_average_rate.into();
    extension.last_update_timestamp = clock.unix_timestamp.into();
    // There is no validation on the rate, since ridiculous values are *technically*
    // possible!
    extension.current_rate = *new_rate;
    Ok(())
}

pub(crate) fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    check_program_account(program_id)?;
    match decode_instruction_type(input)? {
        InterestBearingMintInstruction::Initialize => {
            msg!("InterestBearingMintInstruction::Initialize");
            let InitializeInstructionData {
                rate_authority,
                rate,
            } = decode_instruction_data(input)?;
            process_initialize(program_id, accounts, rate_authority, rate)
        }
        InterestBearingMintInstruction::UpdateRate => {
            msg!("InterestBearingMintInstruction::UpdateRate");
            let new_rate = decode_instruction_data(input)?;
            process_update_rate(program_id, accounts, new_rate)
        }
    }
}
