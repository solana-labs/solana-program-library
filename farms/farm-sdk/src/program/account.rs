//! Common accounts management functions

use {
    crate::{math, pack::check_data_len},
    arrayref::array_ref,
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program::invoke,
        program_error::ProgramError, program_pack::Pack, pubkey::Pubkey,
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

/// Returns Token account mint.
/// Extrats mint field without unpacking entire struct.
pub fn get_token_account_mint(token_account: &AccountInfo) -> Result<Pubkey, ProgramError> {
    let data = token_account.try_borrow_data()?;
    check_data_len(&data, spl_token::state::Account::get_packed_len())?;
    let mint = array_ref![data, 0, 32];

    Ok(Pubkey::new_from_array(*mint))
}

pub fn get_balance_increase(
    account: &AccountInfo,
    previous_balance: u64,
) -> Result<u64, ProgramError> {
    let balance = get_token_balance(account)?;
    if balance >= previous_balance {
        // safe to use unchecked sub
        Ok(balance - previous_balance)
    } else {
        msg!(
            "Error: Balance decrease was not expected. Account: {}",
            account.key
        );
        Err(ProgramError::Custom(1001))
    }
}

pub fn get_balance_decrease(
    account: &AccountInfo,
    previous_balance: u64,
) -> Result<u64, ProgramError> {
    let balance = get_token_balance(account)?;
    if balance <= previous_balance {
        // safe to use unchecked sub
        Ok(previous_balance - balance)
    } else {
        msg!(
            "Error: Balance increase was not expected. Account: {}",
            account.key
        );
        Err(ProgramError::Custom(1002))
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
        Err(ProgramError::Custom(1003))
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
        Err(ProgramError::Custom(1004))
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

/// Returns token pair ratio, uses decimals instead of mints, optimized for on-chain.
pub fn get_token_ratio_with_decimals(
    token_a_balance: u64,
    token_b_balance: u64,
    token_a_decimals: u8,
    token_b_decimals: u8,
) -> Result<f64, ProgramError> {
    if token_a_balance == 0 || token_b_balance == 0 {
        return Ok(0.0);
    }

    let mut ratio = token_b_balance as f64 / token_a_balance as f64;
    match token_a_decimals.cmp(&token_b_decimals) {
        Ordering::Greater => {
            for _ in 0..(token_a_decimals - token_b_decimals) {
                ratio *= 10.0;
            }
        }
        Ordering::Less => {
            for _ in 0..(token_b_decimals - token_a_decimals) {
                ratio /= 10.0;
            }
        }
        Ordering::Equal => {}
    }

    Ok(ratio)
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
    let mut ui_amount = amount;
    for _ in 0..decimals {
        ui_amount /= 10;
    }
    ui_amount as f64
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
            let mut new_amount = amount as f64;
            // safe to use unchecked sub
            for _ in 0..(new_decimals - original_decimals) {
                new_amount *= 10.0;
            }
            math::checked_as_u64(new_amount)
        }
        Ordering::Less => {
            let mut new_amount = amount;
            // safe to use unchecked sub
            for _ in 0..(original_decimals - new_decimals) {
                new_amount /= 10;
            }
            Ok(new_amount)
        }
        Ordering::Equal => Ok(amount),
    }
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
    )?;
    Ok(())
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

pub fn close_system_account<'a, 'b>(
    receiving_account: &'a AccountInfo<'b>,
    target_account: &'a AccountInfo<'b>,
    authority_account: &Pubkey,
) -> ProgramResult {
    if *target_account.owner != *authority_account {
        return Err(ProgramError::IllegalOwner);
    }
    let cur_balance = target_account.try_lamports()?;
    **receiving_account.try_borrow_mut_lamports()? = receiving_account
        .try_lamports()?
        .checked_add(cur_balance)
        .ok_or(ProgramError::InsufficientFunds)?;
    **target_account.try_borrow_mut_lamports()? = target_account
        .try_lamports()?
        .checked_sub(cur_balance)
        .ok_or(ProgramError::InsufficientFunds)?;

    if target_account.data_len() > 1000 {
        target_account.try_borrow_mut_data()?[..1000].fill(0);
    } else {
        target_account.try_borrow_mut_data()?.fill(0);
    }

    Ok(())
}

pub fn close_token_account<'a, 'b>(
    receiving_account: &'a AccountInfo<'b>,
    target_account: &'a AccountInfo<'b>,
    authority_account: &'a AccountInfo<'b>,
) -> ProgramResult {
    invoke(
        &spl_token::instruction::close_account(
            &spl_token::id(),
            receiving_account.key,
            target_account.key,
            authority_account.key,
            &[],
        )?,
        &[
            target_account.clone(),
            receiving_account.clone(),
            authority_account.clone(),
        ],
    )?;
    Ok(())
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
