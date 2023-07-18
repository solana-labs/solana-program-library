//! Program instruction processor

use {
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program::invoke_signed,
        program_error::ProgramError,
        program_pack::Pack,
        pubkey::Pubkey,
    },
    spl_token::{
        instruction::transfer_checked,
        state::{Account, Mint},
    },
};

/// Instruction processor
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    // Create an iterator to safely reference accounts in the slice
    let account_info_iter = &mut accounts.iter();

    // As part of the program specification the instruction gives:
    let source_info = next_account_info(account_info_iter)?; // 1.
    let mint_info = next_account_info(account_info_iter)?; // 2.
    let destination_info = next_account_info(account_info_iter)?; // 3.
    let authority_info = next_account_info(account_info_iter)?; // 4.
    let token_program_info = next_account_info(account_info_iter)?; // 5.

    // In order to transfer from the source account, owned by the program-derived
    // address, we must have the correct address and seeds.
    let (expected_authority, bump_seed) = Pubkey::find_program_address(&[b"authority"], program_id);
    if expected_authority != *authority_info.key {
        return Err(ProgramError::InvalidSeeds);
    }

    // The program transfers everything out of its account, so extract that from
    // the account data.
    let source_account = Account::unpack(&source_info.try_borrow_data()?)?;
    let amount = source_account.amount;

    // The program uses `transfer_checked`, which requires the number of decimals
    // in the mint, so extract that from the account data too.
    let mint = Mint::unpack(&mint_info.try_borrow_data()?)?;
    let decimals = mint.decimals;

    // Invoke the transfer
    msg!("Attempting to transfer {} tokens", amount);
    invoke_signed(
        &transfer_checked(
            token_program_info.key,
            source_info.key,
            mint_info.key,
            destination_info.key,
            authority_info.key,
            &[], // no multisig allowed
            amount,
            decimals,
        )
        .unwrap(),
        &[
            source_info.clone(),
            mint_info.clone(),
            destination_info.clone(),
            authority_info.clone(),
            token_program_info.clone(), // not required, but better for clarity
        ],
        &[&[b"authority", &[bump_seed]]],
    )
}
