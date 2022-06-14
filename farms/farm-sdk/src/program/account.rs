//! Common accounts management functions

use {
    crate::{
        error::FarmError,
        id::zero,
        math,
        pack::check_data_len,
        program::clock,
        token::{OraclePrice, OracleType},
        traits::Packed,
    },
    arrayref::{array_ref, array_refs},
    pyth_client::{PriceStatus, PriceType},
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program::invoke,
        program_error::ProgramError, program_pack::Pack, pubkey::Pubkey, system_instruction,
        sysvar, sysvar::Sysvar,
    },
    spl_token::state::{Account, Mint},
    std::cmp::Ordering,
};

/// Returns Token Mint supply.
/// Extrats supply field without unpacking entire struct.
pub fn get_token_supply(token_mint: &AccountInfo) -> Result<u64, ProgramError> {
    let data = token_mint.try_borrow_data()?;
    check_data_len(&data, spl_token::state::Mint::get_packed_len())?;
    let supply = array_ref![data, 36, 8];

    Ok(u64::from_le_bytes(*supply))
}

/// Returns Token decimals.
/// Extrats decimals field without unpacking entire struct.
pub fn get_token_decimals(token_mint: &AccountInfo) -> Result<u8, ProgramError> {
    let data = token_mint.try_borrow_data()?;
    check_data_len(&data, spl_token::state::Mint::get_packed_len())?;
    let decimals = array_ref![data, 44, 1];

    Ok(decimals[0])
}

/// Returns Tokens balance.
/// Extrats balance field without unpacking entire struct.
pub fn get_token_balance(token_account: &AccountInfo) -> Result<u64, ProgramError> {
    let data = token_account.try_borrow_data()?;
    check_data_len(&data, spl_token::state::Account::get_packed_len())?;
    let amount = array_ref![data, 64, 8];

    Ok(u64::from_le_bytes(*amount))
}

/// Returns Token account owner.
/// Extrats owner field without unpacking entire struct.
pub fn get_token_account_owner(token_account: &AccountInfo) -> Result<Pubkey, ProgramError> {
    let data = token_account.try_borrow_data()?;
    check_data_len(&data, spl_token::state::Account::get_packed_len())?;
    let owner = array_ref![data, 32, 32];

    Ok(Pubkey::new_from_array(*owner))
}

/// Checks Token account owner
pub fn check_token_account_owner(
    token_account: &AccountInfo,
    expected_owner: &Pubkey,
) -> Result<bool, ProgramError> {
    Ok(token_account.owner == &spl_token::id()
        && get_token_account_owner(token_account)? == *expected_owner)
}

/// Checks Token account owner
pub fn check_token_account_owner_or_zero(
    token_account: &AccountInfo,
    expected_owner: &Pubkey,
) -> Result<bool, ProgramError> {
    Ok(token_account.key == &zero::id()
        || (token_account.owner == &spl_token::id()
            && get_token_account_owner(token_account)? == *expected_owner))
}

/// Returns Token account mint.
/// Extrats mint field without unpacking entire struct.
pub fn get_token_account_mint(token_account: &AccountInfo) -> Result<Pubkey, ProgramError> {
    let data = token_account.try_borrow_data()?;
    check_data_len(&data, spl_token::state::Account::get_packed_len())?;
    let mint = array_ref![data, 0, 32];

    Ok(Pubkey::new_from_array(*mint))
}

/// Returns Mint authority.
/// Extrats authority field without unpacking entire struct.
pub fn get_mint_authority(token_mint: &AccountInfo) -> Result<Option<Pubkey>, ProgramError> {
    let data = token_mint.try_borrow_data()?;
    check_data_len(&data, spl_token::state::Mint::get_packed_len())?;

    let data = array_ref![data, 0, 36];
    let (tag, authority) = array_refs![data, 4, 32];
    match *tag {
        [0, 0, 0, 0] => Ok(None),
        [1, 0, 0, 0] => Ok(Some(Pubkey::new_from_array(*authority))),
        _ => Err(ProgramError::InvalidAccountData),
    }
}

/// Checks mint authority
pub fn check_mint_authority(
    mint_account: &AccountInfo,
    expected_authority: Option<Pubkey>,
) -> Result<bool, ProgramError> {
    Ok(mint_account.owner == &spl_token::id()
        && get_mint_authority(mint_account)? == expected_authority)
}

pub fn is_empty(account: &AccountInfo) -> Result<bool, ProgramError> {
    Ok(account.data_is_empty() || account.try_lamports()? == 0)
}

pub fn exists(account: &AccountInfo) -> Result<bool, ProgramError> {
    Ok(account.try_lamports()? > 0)
}

pub fn get_balance_increase(
    account: &AccountInfo,
    previous_balance: u64,
) -> Result<u64, ProgramError> {
    let balance = get_token_balance(account)?;
    if let Some(res) = balance.checked_sub(previous_balance) {
        Ok(res)
    } else {
        msg!(
            "Error: Balance decrease was not expected. Account: {}",
            account.key
        );
        Err(FarmError::UnexpectedBalanceDecrease.into())
    }
}

pub fn get_balance_decrease(
    account: &AccountInfo,
    previous_balance: u64,
) -> Result<u64, ProgramError> {
    let balance = get_token_balance(account)?;
    if let Some(res) = previous_balance.checked_sub(balance) {
        Ok(res)
    } else {
        msg!(
            "Error: Balance increase was not expected. Account: {}",
            account.key
        );
        Err(FarmError::UnexpectedBalanceIncrease.into())
    }
}

pub fn check_tokens_spent(
    account: &AccountInfo,
    previous_balance: u64,
    max_amount_spent: u64,
) -> Result<u64, ProgramError> {
    let tokens_spent = get_balance_decrease(account, previous_balance)?;
    if tokens_spent > max_amount_spent {
        msg!(
            "Error: Invoked program overspent. Account: {}, max expected: {}, actual: {}",
            account.key,
            max_amount_spent,
            tokens_spent
        );
        Err(FarmError::ProgramOverspent.into())
    } else {
        Ok(tokens_spent)
    }
}

pub fn check_tokens_received(
    account: &AccountInfo,
    previous_balance: u64,
    min_amount_received: u64,
) -> Result<u64, ProgramError> {
    let tokens_received = get_balance_increase(account, previous_balance)?;
    if tokens_received < min_amount_received {
        msg!(
            "Error: Not enough tokens returned by invoked program. Account: {}, min expected: {}, actual: {}",
            account.key,
            min_amount_received,
            tokens_received
        );
        Err(FarmError::ProgramInsufficientTransfer.into())
    } else {
        Ok(tokens_received)
    }
}

/// Returns Token Mint data.
pub fn get_token_mint(token_mint: &AccountInfo) -> Result<Mint, ProgramError> {
    let data = token_mint.try_borrow_data()?;
    Mint::unpack(&data)
}

/// Returns Token Account data.
pub fn get_token_account(token_account: &AccountInfo) -> Result<Account, ProgramError> {
    let data = token_account.try_borrow_data()?;
    Account::unpack(&data)
}

/// Returns token pair ratio, optimized for on-chain.
pub fn get_token_ratio<'a, 'b>(
    token_a_balance: u64,
    token_b_balance: u64,
    token_a_mint: &'a AccountInfo<'b>,
    token_b_mint: &'a AccountInfo<'b>,
) -> Result<f64, ProgramError> {
    get_token_ratio_with_decimals(
        token_a_balance,
        token_b_balance,
        get_token_decimals(token_a_mint)?,
        get_token_decimals(token_b_mint)?,
    )
}

/// Returns token pair ratio, uses decimals instead of mints
pub fn get_token_ratio_with_decimals(
    token_a_balance: u64,
    token_b_balance: u64,
    token_a_decimals: u8,
    token_b_decimals: u8,
) -> Result<f64, ProgramError> {
    if token_a_balance == 0 || token_b_balance == 0 {
        return Ok(0.0);
    }

    Ok(token_b_balance as f64 / token_a_balance as f64
        * math::checked_powi(10.0, token_a_decimals as i32 - token_b_decimals as i32)?)
}

/// Returns token pair ratio
pub fn get_token_pair_ratio<'a, 'b>(
    token_a_account: &'a AccountInfo<'b>,
    token_b_account: &'a AccountInfo<'b>,
) -> Result<f64, ProgramError> {
    let token_a_balance = get_token_balance(token_a_account)?;
    let token_b_balance = get_token_balance(token_b_account)?;
    if token_a_balance == 0 || token_b_balance == 0 {
        return Ok(0.0);
    }
    Ok(token_b_balance as f64 / token_a_balance as f64)
}

pub fn to_ui_amount(amount: u64, decimals: u8) -> f64 {
    let mut ui_amount = amount as f64;
    for _ in 0..decimals {
        ui_amount /= 10.0;
    }
    ui_amount
}

pub fn to_token_amount(ui_amount: f64, decimals: u8) -> Result<u64, ProgramError> {
    let mut amount = ui_amount;
    for _ in 0..decimals {
        amount *= 10.0;
    }
    math::checked_as_u64(amount)
}

pub fn to_amount_with_new_decimals(
    amount: u64,
    original_decimals: u8,
    new_decimals: u8,
) -> Result<u64, ProgramError> {
    match new_decimals.cmp(&original_decimals) {
        Ordering::Greater => {
            let exponent = new_decimals.checked_sub(original_decimals).unwrap();
            math::checked_mul(amount, math::checked_pow(10u64, exponent as usize)?)
        }
        Ordering::Less => {
            let exponent = original_decimals.checked_sub(new_decimals).unwrap();
            math::checked_div(amount, math::checked_pow(10u64, exponent as usize)?)
        }
        Ordering::Equal => Ok(amount),
    }
}

pub fn init_token_account<'a, 'b>(
    funding_account: &'a AccountInfo<'b>,
    target_account: &'a AccountInfo<'b>,
    mint_account: &'a AccountInfo<'b>,
    owner_account: &'a AccountInfo<'b>,
    rent_program: &'a AccountInfo<'b>,
    seed: &str,
) -> ProgramResult {
    if exists(target_account)? {
        if !check_token_account_owner(target_account, owner_account.key)? {
            return Err(ProgramError::IllegalOwner);
        }
        if target_account.data_len() != spl_token::state::Account::get_packed_len()
            || mint_account.key != &get_token_account_mint(target_account)?
        {
            return Err(ProgramError::InvalidAccountData);
        }
        return Ok(());
    }

    init_system_account(
        funding_account,
        target_account,
        &spl_token::id(),
        seed,
        spl_token::state::Account::get_packed_len(),
    )?;

    invoke(
        &spl_token::instruction::initialize_account(
            &spl_token::id(),
            target_account.key,
            mint_account.key,
            owner_account.key,
        )?,
        &[
            target_account.clone(),
            mint_account.clone(),
            owner_account.clone(),
            rent_program.clone(),
        ],
    )
}

pub fn close_token_account<'a, 'b>(
    receiving_account: &'a AccountInfo<'b>,
    target_account: &'a AccountInfo<'b>,
    authority_account: &'a AccountInfo<'b>,
) -> ProgramResult {
    if !exists(target_account)? {
        return Ok(());
    }

    invoke(
        &spl_token::instruction::close_account(
            &spl_token::id(),
            target_account.key,
            receiving_account.key,
            authority_account.key,
            &[],
        )?,
        &[
            target_account.clone(),
            receiving_account.clone(),
            authority_account.clone(),
        ],
    )
}

pub fn transfer_sol_from_owned<'a, 'b>(
    program_owned_source_account: &'a AccountInfo<'b>,
    destination_account: &'a AccountInfo<'b>,
    amount: u64,
) -> ProgramResult {
    **destination_account.try_borrow_mut_lamports()? = destination_account
        .try_lamports()?
        .checked_add(amount)
        .ok_or(ProgramError::InsufficientFunds)?;
    let source_balance = program_owned_source_account.try_lamports()?;
    if source_balance < amount {
        msg!(
            "Error: Not enough funds to withdraw {} lamports from {}",
            amount,
            program_owned_source_account.key
        );
        return Err(ProgramError::InsufficientFunds);
    }
    **program_owned_source_account.try_borrow_mut_lamports()? = source_balance
        .checked_sub(amount)
        .ok_or(ProgramError::InsufficientFunds)?;

    Ok(())
}

pub fn transfer_sol<'a, 'b>(
    source_account: &'a AccountInfo<'b>,
    destination_account: &'a AccountInfo<'b>,
    amount: u64,
) -> ProgramResult {
    if source_account.try_lamports()? < amount {
        msg!(
            "Error: Not enough funds to withdraw {} lamports from {}",
            amount,
            source_account.key
        );
        return Err(ProgramError::InsufficientFunds);
    }
    invoke(
        &system_instruction::transfer(source_account.key, destination_account.key, amount),
        &[source_account.clone(), destination_account.clone()],
    )
}

pub fn transfer_tokens<'a, 'b>(
    source_account: &'a AccountInfo<'b>,
    destination_account: &'a AccountInfo<'b>,
    authority_account: &'a AccountInfo<'b>,
    amount: u64,
) -> ProgramResult {
    invoke(
        &spl_token::instruction::transfer(
            &spl_token::id(),
            source_account.key,
            destination_account.key,
            authority_account.key,
            &[],
            amount,
        )?,
        &[
            source_account.clone(),
            destination_account.clone(),
            authority_account.clone(),
        ],
    )
}

pub fn burn_tokens<'a, 'b>(
    from_token_account: &'a AccountInfo<'b>,
    mint_account: &'a AccountInfo<'b>,
    authority_account: &'a AccountInfo<'b>,
    amount: u64,
) -> ProgramResult {
    invoke(
        &spl_token::instruction::burn(
            &spl_token::id(),
            from_token_account.key,
            mint_account.key,
            authority_account.key,
            &[],
            amount,
        )?,
        &[
            from_token_account.clone(),
            mint_account.clone(),
            authority_account.clone(),
        ],
    )
}

pub fn approve_delegate<'a, 'b>(
    source_account: &'a AccountInfo<'b>,
    delegate_account: &'a AccountInfo<'b>,
    authority_account: &'a AccountInfo<'b>,
    amount: u64,
) -> ProgramResult {
    invoke(
        &spl_token::instruction::approve(
            &spl_token::id(),
            source_account.key,
            delegate_account.key,
            authority_account.key,
            &[],
            amount,
        )?,
        &[
            source_account.clone(),
            delegate_account.clone(),
            authority_account.clone(),
        ],
    )
}

pub fn revoke_delegate<'a, 'b>(
    source_account: &'a AccountInfo<'b>,
    authority_account: &'a AccountInfo<'b>,
) -> ProgramResult {
    invoke(
        &spl_token::instruction::revoke(
            &spl_token::id(),
            source_account.key,
            authority_account.key,
            &[],
        )?,
        &[source_account.clone(), authority_account.clone()],
    )
}

pub fn init_system_account<'a, 'b>(
    funding_account: &'a AccountInfo<'b>,
    target_account: &'a AccountInfo<'b>,
    owner_key: &Pubkey,
    seed: &str,
    data_size: usize,
) -> ProgramResult {
    if exists(target_account)? {
        if target_account.owner != owner_key {
            return Err(ProgramError::IllegalOwner);
        }
        if target_account.data_len() != data_size {
            return Err(ProgramError::InvalidAccountData);
        }
        return Ok(());
    }

    let derived_account = Pubkey::create_with_seed(funding_account.key, seed, owner_key)?;
    if target_account.key != &derived_account {
        return Err(ProgramError::InvalidSeeds);
    }

    let min_balance = sysvar::rent::Rent::get()
        .unwrap()
        .minimum_balance(data_size);
    invoke(
        &system_instruction::create_account_with_seed(
            funding_account.key,
            target_account.key,
            funding_account.key,
            seed,
            min_balance,
            data_size as u64,
            owner_key,
        ),
        &[funding_account.clone(), target_account.clone()],
    )
}

pub fn close_system_account<'a, 'b>(
    receiving_account: &'a AccountInfo<'b>,
    target_account: &'a AccountInfo<'b>,
    authority_account: &Pubkey,
) -> ProgramResult {
    if *target_account.owner != *authority_account {
        return Err(ProgramError::IllegalOwner);
    }
    let cur_balance = target_account.try_lamports()?;
    transfer_sol_from_owned(target_account, receiving_account, cur_balance)?;

    if target_account.data_len() > 2000 {
        target_account.try_borrow_mut_data()?[..2000].fill(0);
    } else {
        target_account.try_borrow_mut_data()?.fill(0);
    }

    Ok(())
}

pub fn get_oracle_price(
    oracle_type: OracleType,
    oracle_account: &AccountInfo,
    max_price_error: f64,
    max_price_age_sec: u64,
) -> Result<OraclePrice, ProgramError> {
    match oracle_type {
        OracleType::Pyth => get_pyth_price(oracle_account, max_price_error, max_price_age_sec),
        _ => Err(ProgramError::UnsupportedSysvar),
    }
}

pub fn get_pyth_price(
    pyth_price_info: &AccountInfo,
    max_price_error: f64,
    max_price_age_sec: u64,
) -> Result<OraclePrice, ProgramError> {
    if is_empty(pyth_price_info)? {
        msg!("Error: Invalid Pyth oracle account");
        return Err(FarmError::OracleInvalidAccount.into());
    }

    let pyth_price_data = &pyth_price_info.try_borrow_data()?;
    let pyth_price = pyth_client::load_price(pyth_price_data)?;

    if !matches!(pyth_price.agg.status, PriceStatus::Trading)
        || !matches!(pyth_price.ptype, PriceType::Price)
    {
        msg!("Error: Pyth oracle price has invalid state");
        return Err(FarmError::OracleInvalidState.into());
    }

    let last_update_age_sec = math::checked_mul(
        math::checked_sub(clock::get_slot()?, pyth_price.valid_slot)?,
        solana_program::clock::DEFAULT_MS_PER_SLOT,
    )? / 1000;
    if last_update_age_sec > max_price_age_sec {
        msg!("Error: Pyth oracle price is stale");
        return Err(FarmError::OracleStalePrice.into());
    }

    if pyth_price.agg.price <= 0
        || pyth_price.agg.conf as f64 / pyth_price.agg.price as f64 > max_price_error
    {
        msg!("Error: Pyth oracle price is out of bounds");
        return Err(FarmError::OracleInvalidPrice.into());
    }

    Ok(OraclePrice {
        // price is i64 and > 0 per check above
        price: pyth_price.agg.price as u64,
        exponent: pyth_price.expo,
    })
}

// Converts token amount to USD using price oracle
pub fn get_asset_value_usd(
    amount: u64,
    decimals: u8,
    oracle_type: OracleType,
    oracle_account: &AccountInfo,
    max_price_error: f64,
    max_price_age_sec: u64,
) -> Result<f64, ProgramError> {
    if amount == 0 {
        return Ok(0.0);
    }
    let oracle_price = get_oracle_price(
        oracle_type,
        oracle_account,
        max_price_error,
        max_price_age_sec,
    )?;

    Ok(amount as f64 * oracle_price.price as f64
        / math::checked_powi(10.0, decimals as i32 - oracle_price.exponent)?)
}

// Converts USD amount to tokens using price oracle
pub fn get_asset_value_tokens(
    usd_amount: f64,
    token_decimals: u8,
    oracle_type: OracleType,
    oracle_account: &AccountInfo,
    max_price_error: f64,
    max_price_age_sec: u64,
) -> Result<u64, ProgramError> {
    if usd_amount == 0.0 {
        return Ok(0);
    }
    let oracle_price = get_oracle_price(
        oracle_type,
        oracle_account,
        max_price_error,
        max_price_age_sec,
    )?;

    // oracle_price.price guaranteed > 0
    math::checked_as_u64(
        usd_amount as f64 / oracle_price.price as f64
            * math::checked_powi(10.0, token_decimals as i32 - oracle_price.exponent)?,
    )
}

pub fn unpack<T: Packed>(account: &AccountInfo, name: &str) -> Result<T, ProgramError> {
    if let Ok(object) = T::unpack(&account.try_borrow_data()?) {
        Ok(object)
    } else {
        msg!("Error: Failed to load {} metadata", name);
        Err(ProgramError::InvalidAccountData)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spl_token::state::{Account, Mint};

    #[test]
    fn test_mint_supply_offset() {
        let mint = Mint {
            supply: 1234567891011,
            ..Mint::default()
        };
        let mut packed: [u8; 82] = [0; 82];
        Mint::pack(mint, &mut packed).unwrap();

        let supply = array_ref![packed, 36, 8];
        assert_eq!(1234567891011, u64::from_le_bytes(*supply));
    }

    #[test]
    fn test_mint_decimals_offset() {
        let mint = Mint {
            decimals: 123,
            ..Mint::default()
        };
        let mut packed: [u8; 82] = [0; 82];
        Mint::pack(mint, &mut packed).unwrap();

        let decimals = array_ref![packed, 44, 1];
        assert_eq!(123, decimals[0]);
    }

    #[test]
    fn test_account_amount_offset() {
        let account = Account {
            amount: 1234567891011,
            ..Account::default()
        };
        let mut packed: [u8; 165] = [0; 165];
        Account::pack(account, &mut packed).unwrap();

        let amount = array_ref![packed, 64, 8];
        assert_eq!(1234567891011, u64::from_le_bytes(*amount));
    }
}
