use {
    crate::{
        check_program_account,
        error::TokenError,
        extension::{
            scaled_ui_amount::{
                instruction::{
                    InitializeInstructionData, ScaledUiAmountMintInstruction,
                    UpdateMultiplierInstructionData,
                },
                PodF64, ScaledUiAmountConfig, UnixTimestamp,
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

fn try_validate_multiplier(multiplier: &PodF64) -> ProgramResult {
    let float_multiplier = f64::from(*multiplier);
    if float_multiplier.is_sign_positive() && float_multiplier.is_normal() {
        Ok(())
    } else {
        Err(TokenError::InvalidScale.into())
    }
}

fn process_initialize(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    authority: &OptionalNonZeroPubkey,
    multiplier: &PodF64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;
    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack_uninitialized(&mut mint_data)?;

    let extension = mint.init_extension::<ScaledUiAmountConfig>(true)?;
    extension.authority = *authority;
    try_validate_multiplier(multiplier)?;
    extension.multiplier = *multiplier;
    extension.new_multiplier_effective_timestamp = 0.into();
    extension.new_multiplier = *multiplier;
    Ok(())
}

fn process_update_multiplier(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_multiplier: &PodF64,
    effective_timestamp: &UnixTimestamp,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;
    let owner_info = next_account_info(account_info_iter)?;
    let owner_info_data_len = owner_info.data_len();

    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack(&mut mint_data)?;
    let extension = mint.get_extension_mut::<ScaledUiAmountConfig>()?;
    let authority =
        Option::<Pubkey>::from(extension.authority).ok_or(TokenError::NoAuthorityExists)?;

    Processor::validate_owner(
        program_id,
        &authority,
        owner_info,
        owner_info_data_len,
        account_info_iter.as_slice(),
    )?;

    try_validate_multiplier(new_multiplier)?;
    let clock = Clock::get()?;
    extension.new_multiplier = *new_multiplier;
    let int_effective_timestamp = i64::from(*effective_timestamp);
    // just floor it to 0
    if int_effective_timestamp < 0 {
        extension.new_multiplier_effective_timestamp = 0.into();
    } else {
        extension.new_multiplier_effective_timestamp = *effective_timestamp;
    }
    // if the new effective timestamp has already passed, also set the old
    // multiplier, just to be clear
    if clock.unix_timestamp >= int_effective_timestamp {
        extension.multiplier = *new_multiplier;
    }
    Ok(())
}

pub(crate) fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    check_program_account(program_id)?;
    match decode_instruction_type(input)? {
        ScaledUiAmountMintInstruction::Initialize => {
            msg!("ScaledUiAmountMintInstruction::Initialize");
            let InitializeInstructionData {
                authority,
                multiplier,
            } = decode_instruction_data(input)?;
            process_initialize(program_id, accounts, authority, multiplier)
        }
        ScaledUiAmountMintInstruction::UpdateMultiplier => {
            msg!("ScaledUiAmountMintInstruction::UpdateScale");
            let UpdateMultiplierInstructionData {
                effective_timestamp,
                multiplier,
            } = decode_instruction_data(input)?;
            process_update_multiplier(program_id, accounts, multiplier, effective_timestamp)
        }
    }
}
