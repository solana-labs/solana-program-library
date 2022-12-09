#![allow(clippy::integer_arithmetic)]
//! Program instruction processor

use {
    crate::find_manager_registration_address_internal,
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        program::{invoke, invoke_signed},
        program_error::ProgramError,
        pubkey::{Pubkey, PUBKEY_BYTES},
        rent::Rent,
        system_instruction, system_program,
        sysvar::Sysvar,
    },
    spl_token_2022::{
        check_spl_token_program_account, extension::StateWithExtensions, state::Mint,
    },
};

/// Instruction processor, writes the pubkey of the managing program to the
/// registration address, so that anyone can look up the managing program from
/// the mint address using `crate::find_manager_registration_address()`, and
/// read the managing program address from there.
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let payer_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let mint_authority_info = next_account_info(account_info_iter)?;
    let manager_registration_info = next_account_info(account_info_iter)?;
    let manager_program_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;

    if system_program_info.key != &system_program::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Check mint is actually a mint
    check_spl_token_program_account(mint_info.owner)?;
    let mint_data = mint_info.try_borrow_data()?;
    let mint = StateWithExtensions::<Mint>::unpack(&mint_data)?;

    // Check mint authority is correct and signed
    if !mint.base.mint_authority.contains(mint_authority_info.key) {
        return Err(ProgramError::InvalidAccountData);
    }
    if !mint_authority_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Check manager registration account address is correct
    let (expected_registration_address, bump_seed) =
        find_manager_registration_address_internal(program_id, mint_info.key);
    if &expected_registration_address != manager_registration_info.key {
        return Err(ProgramError::InvalidSeeds);
    }
    let signer_seeds = [mint_info.key.as_ref(), &[bump_seed]];

    // Create the account with 32 bytes, and write 32 bytes
    invoke_signed(
        &system_instruction::allocate(manager_registration_info.key, PUBKEY_BYTES as u64),
        &[
            manager_registration_info.clone(),
            system_program_info.clone(),
        ],
        &[&signer_seeds],
    )?;
    invoke_signed(
        &system_instruction::assign(manager_registration_info.key, program_id),
        &[
            manager_registration_info.clone(),
            system_program_info.clone(),
        ],
        &[&signer_seeds],
    )?;
    let rent = Rent::get()?;
    let rent_exemption = rent.minimum_balance(PUBKEY_BYTES);
    let transfer_amount = rent_exemption.saturating_sub(manager_registration_info.try_lamports()?);
    if transfer_amount > 0 {
        invoke(
            &system_instruction::transfer(
                payer_info.key,
                manager_registration_info.key,
                transfer_amount,
            ),
            &[
                payer_info.clone(),
                manager_registration_info.clone(),
                system_program_info.clone(),
            ],
        )?;
    }
    let mut data = manager_registration_info.try_borrow_mut_data()?;
    data.copy_from_slice(manager_program_info.key.as_ref());

    Ok(())
}
