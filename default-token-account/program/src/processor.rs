//! Program state processor

use crate::{
    error::DefaultTokenAccountError, //, get_default_token_account_address_and_bump_seed,
    instruction::DefaultTokenAccountInstruction,
    *,
};
use solana_program::program::{invoke, invoke_signed};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    info,
    log::sol_log_compute_units,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

/// Process a DefaultTokenAccountInstruction::Create instruction
fn process_create(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let default_token_account_info = next_account_info(account_info_iter)?;
    let user_wallet_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let funder_info = next_account_info(account_info_iter)?;
    let sysvar_rent = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;

    let (default_token_account_address, bump_seed) =
        get_default_token_account_address_and_bump_seed(
            program_id,
            &token_program.key,
            &mint_info.key,
            &user_wallet_info.key,
        );

    if default_token_account_address != *default_token_account_info.key {
        return Err(DefaultTokenAccountError::InvalidDefaultTokenAccountAddress.into());
    }

    let default_token_account_signer_seeds: &[&[_]] = &[
        &token_program.key.to_bytes(),
        &mint_info.key.to_bytes(),
        &user_wallet_info.key.to_bytes(),
        &[bump_seed],
    ];

    // Fund the default token account with the minimum balance to be rent exempt
    let rent = &Rent::from_account_info(sysvar_rent)?;
    let required_lamports = rent
        .minimum_balance(spl_token::state::Account::LEN)
        .max(1)
        .saturating_sub(default_token_account_info.lamports());

    if required_lamports > 0 {
        invoke(
            &system_instruction::transfer(
                &funder_info.key,
                default_token_account_info.key,
                required_lamports,
            ),
            &[
                funder_info.clone(),
                default_token_account_info.clone(),
                system_program.clone(),
            ],
        )?;
    }

    sol_log_compute_units();

    // Allocate space for the default token account
    invoke_signed(
        &system_instruction::allocate(
            default_token_account_info.key,
            spl_token::state::Account::LEN as u64,
        ),
        &[default_token_account_info.clone(), system_program.clone()],
        &[default_token_account_signer_seeds],
    )?;

    // Assign the default token account to the SPL Token program
    invoke_signed(
        &system_instruction::assign(default_token_account_info.key, &spl_token::id()),
        &[default_token_account_info.clone(), system_program.clone()],
        &[default_token_account_signer_seeds],
    )?;

    // Initialize the default token account
    invoke(
        &spl_token::instruction::initialize_account(
            &spl_token::id(),
            default_token_account_info.key,
            mint_info.key,
            user_wallet_info.key,
        )?,
        &[
            default_token_account_info.clone(),
            mint_info.clone(),
            user_wallet_info.clone(),
            sysvar_rent.clone(),
            token_program.clone(),
        ],
    )
}

/// Process a DefaultTokenAccountInstruction::Exists instruction
fn process_exists(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let default_token_account_info = next_account_info(account_info_iter)?;
    let user_wallet_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;

    let default_token_account_address = get_default_token_account_address(
        program_id,
        &token_program.key,
        &mint_info.key,
        &user_wallet_info.key,
    );

    if default_token_account_address != *default_token_account_info.key {
        return Err(DefaultTokenAccountError::InvalidDefaultTokenAccountAddress.into());
    }

    let default_token_account =
        spl_token::state::Account::unpack(&default_token_account_info.data.borrow())?;

    if user_wallet_info.key != &default_token_account.owner {
        return Err(DefaultTokenAccountError::TokenOwnerMismatch.into());
    }

    Ok(())
}

/// Processes a DefaultTokenAccountInstruction
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = DefaultTokenAccountInstruction::unpack(input)?;
    match instruction {
        DefaultTokenAccountInstruction::Create => {
            info!("Create");
            process_create(program_id, accounts)
        }
        DefaultTokenAccountInstruction::Exists => {
            info!("Exists");
            process_exists(program_id, accounts)
        }
    }
}
