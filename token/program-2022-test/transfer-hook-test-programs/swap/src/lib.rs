//! Program implementation

use {
    solana_account_info::{next_account_info, AccountInfo},
    solana_program_error::ProgramResult,
    solana_pubkey::Pubkey,
    spl_token_2022::onchain,
};

solana_program_entrypoint::entrypoint!(process_instruction);
fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let source_a_account_info = next_account_info(account_info_iter)?;
    let mint_a_info = next_account_info(account_info_iter)?;
    let destination_a_account_info = next_account_info(account_info_iter)?;
    let authority_a_info = next_account_info(account_info_iter)?;
    let token_program_a_info = next_account_info(account_info_iter)?;

    let source_b_account_info = next_account_info(account_info_iter)?;
    let mint_b_info = next_account_info(account_info_iter)?;
    let destination_b_account_info = next_account_info(account_info_iter)?;
    let authority_b_info = next_account_info(account_info_iter)?;
    let token_program_b_info = next_account_info(account_info_iter)?;

    let remaining_accounts = account_info_iter.as_slice();

    onchain::invoke_transfer_checked(
        token_program_a_info.key,
        source_a_account_info.clone(),
        mint_a_info.clone(),
        destination_a_account_info.clone(),
        authority_a_info.clone(),
        remaining_accounts,
        1,
        9,
        &[],
    )?;

    onchain::invoke_transfer_checked(
        token_program_b_info.key,
        source_b_account_info.clone(),
        mint_b_info.clone(),
        destination_b_account_info.clone(),
        authority_b_info.clone(),
        remaining_accounts,
        1,
        9,
        &[],
    )?;

    Ok(())
}
