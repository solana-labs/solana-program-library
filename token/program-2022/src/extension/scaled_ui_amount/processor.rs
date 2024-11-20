use {
    crate::{
        check_program_account,
        error::TokenError,
        extension::{
            scaled_ui_amount::{
                instruction::{InitializeInstructionData, ScaledUiAmountMintInstruction},
                PodF64, ScaledUiAmountConfig,
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

fn try_validate_scale(scale: &PodF64) -> ProgramResult {
    let float_scale = f64::from(*scale);
    if float_scale.is_sign_positive() && float_scale.is_normal() {
        Ok(())
    } else {
        Err(TokenError::InvalidScale.into())
    }
}

fn process_initialize(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    authority: &OptionalNonZeroPubkey,
    scale: &PodF64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;
    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack_uninitialized(&mut mint_data)?;

    let extension = mint.init_extension::<ScaledUiAmountConfig>(true)?;
    extension.authority = *authority;
    try_validate_scale(scale)?;
    extension.scale = *scale;
    Ok(())
}

fn process_update_scale(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_scale: &PodF64,
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

    try_validate_scale(new_scale)?;
    extension.scale = *new_scale;
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
            let InitializeInstructionData { authority, scale } = decode_instruction_data(input)?;
            process_initialize(program_id, accounts, authority, scale)
        }
        ScaledUiAmountMintInstruction::UpdateScale => {
            msg!("ScaledUiAmountMintInstruction::UpdateScale");
            let new_scale = decode_instruction_data(input)?;
            process_update_scale(program_id, accounts, new_scale)
        }
    }
}
