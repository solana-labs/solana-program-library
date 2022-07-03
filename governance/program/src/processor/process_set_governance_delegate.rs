//! Program state processor

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

use crate::state::token_owner_record::get_token_owner_record_data;

/// Processes SetGovernanceDelegate instruction
pub fn process_set_governance_delegate(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_governance_delegate: &Option<Pubkey>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let governance_authority_info = next_account_info(account_info_iter)?; // 0
    let token_owner_record_info = next_account_info(account_info_iter)?; // 1

    let mut token_owner_record_data =
        get_token_owner_record_data(program_id, token_owner_record_info)?;

    token_owner_record_data.assert_token_owner_or_delegate_is_signer(governance_authority_info)?;

    token_owner_record_data.governance_delegate = *new_governance_delegate;
    token_owner_record_data.serialize(&mut *token_owner_record_info.data.borrow_mut())?;

    Ok(())
}
