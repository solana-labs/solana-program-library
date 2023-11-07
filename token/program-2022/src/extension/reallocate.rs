use {
    crate::{
        error::TokenError,
        extension::{
            set_account_type, AccountType, BaseStateWithExtensions, ExtensionType,
            StateWithExtensions, StateWithExtensionsMut,
        },
        processor::Processor,
        state::Account,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program::invoke,
        program_option::COption,
        pubkey::Pubkey,
        system_instruction,
        sysvar::{rent::Rent, Sysvar},
    },
};

/// Processes a [Reallocate](enum.TokenInstruction.html) instruction
pub fn process_reallocate(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_extension_types: Vec<ExtensionType>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let token_account_info = next_account_info(account_info_iter)?;
    let payer_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let authority_info_data_len = authority_info.data_len();

    // check that account is the right type and validate owner
    let (mut current_extension_types, native_token_amount) = {
        let token_account = token_account_info.data.borrow();
        let account = StateWithExtensions::<Account>::unpack(&token_account)?;
        Processor::validate_owner(
            program_id,
            &account.base.owner,
            authority_info,
            authority_info_data_len,
            account_info_iter.as_slice(),
        )?;
        let native_token_amount = account.base.is_native().then_some(account.base.amount);
        (account.get_extension_types()?, native_token_amount)
    };

    // check that all desired extensions are for the right account type
    if new_extension_types
        .iter()
        .any(|extension_type| extension_type.get_account_type() != AccountType::Account)
    {
        return Err(TokenError::InvalidState.into());
    }
    // ExtensionType::try_calculate_account_len() dedupes types, so just a dumb
    // concatenation is fine here
    current_extension_types.extend_from_slice(&new_extension_types);
    let needed_account_len =
        ExtensionType::try_calculate_account_len::<Account>(&current_extension_types)?;

    // if account is already large enough, return early
    if token_account_info.data_len() >= needed_account_len {
        return Ok(());
    }

    // reallocate
    msg!(
        "account needs realloc, +{:?} bytes",
        needed_account_len - token_account_info.data_len()
    );
    token_account_info.realloc(needed_account_len, false)?;

    // if additional lamports needed to remain rent-exempt, transfer them
    let rent = Rent::get()?;
    let new_rent_exempt_reserve = rent.minimum_balance(needed_account_len);

    let current_lamport_reserve = token_account_info
        .lamports()
        .checked_sub(native_token_amount.unwrap_or(0))
        .ok_or(TokenError::Overflow)?;
    let lamports_diff = new_rent_exempt_reserve.saturating_sub(current_lamport_reserve);
    if lamports_diff > 0 {
        invoke(
            &system_instruction::transfer(payer_info.key, token_account_info.key, lamports_diff),
            &[
                payer_info.clone(),
                token_account_info.clone(),
                system_program_info.clone(),
            ],
        )?;
    }

    // set account_type, if needed
    let mut token_account_data = token_account_info.data.borrow_mut();
    set_account_type::<Account>(&mut token_account_data)?;

    // sync the rent exempt reserve for native accounts
    if let Some(native_token_amount) = native_token_amount {
        let mut token_account = StateWithExtensionsMut::<Account>::unpack(&mut token_account_data)?;
        // sanity check that there are enough lamports to cover the token amount
        // and the rent exempt reserve
        let minimum_lamports = new_rent_exempt_reserve
            .checked_add(native_token_amount)
            .ok_or(TokenError::Overflow)?;
        if token_account_info.lamports() < minimum_lamports {
            return Err(TokenError::InvalidState.into());
        }
        token_account.base.is_native = COption::Some(new_rent_exempt_reserve);
        token_account.pack_base();
    }

    Ok(())
}
