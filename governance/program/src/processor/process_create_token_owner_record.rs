//! Program state processor

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};
use spl_governance_tools::account::create_and_serialize_account_signed;

use crate::{
    error::GovernanceError,
    state::{
        enums::GovernanceAccountType,
        realm::get_realm_data,
        token_owner_record::{get_token_owner_record_address_seeds, TokenOwnerRecordV2},
    },
};

/// Processes CreateTokenOwnerRecord instruction
pub fn process_create_token_owner_record(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let realm_info = next_account_info(account_info_iter)?; // 0
    let governing_token_owner_info = next_account_info(account_info_iter)?; // 1
    let token_owner_record_info = next_account_info(account_info_iter)?; // 2
    let governing_token_mint_info = next_account_info(account_info_iter)?; // 3
    let payer_info = next_account_info(account_info_iter)?; // 4
    let system_info = next_account_info(account_info_iter)?; // 5
    let rent = Rent::get()?;

    let realm_data = get_realm_data(program_id, realm_info)?;
    realm_data.assert_is_valid_governing_token_mint(governing_token_mint_info.key)?;

    if !token_owner_record_info.data_is_empty() {
        return Err(GovernanceError::TokenOwnerRecordAlreadyExists.into());
    }

    let token_owner_record_data = TokenOwnerRecordV2 {
        account_type: GovernanceAccountType::TokenOwnerRecordV2,
        realm: *realm_info.key,
        governing_token_owner: *governing_token_owner_info.key,
        governing_token_deposit_amount: 0,
        governing_token_mint: *governing_token_mint_info.key,
        governance_delegate: None,
        unrelinquished_votes_count: 0,
        total_votes_count: 0,
        outstanding_proposal_count: 0,
        reserved: [0; 7],
        reserved_v2: [0; 128],
    };

    create_and_serialize_account_signed(
        payer_info,
        token_owner_record_info,
        &token_owner_record_data,
        &get_token_owner_record_address_seeds(
            realm_info.key,
            governing_token_mint_info.key,
            governing_token_owner_info.key,
        ),
        program_id,
        system_info,
        &rent,
    )
}
