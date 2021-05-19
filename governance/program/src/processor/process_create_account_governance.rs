//! Program state processor
//use crate::utils::assert_program_upgrade_authority;

use crate::{
    state::account_governance::AccountGovernance,
    state::{
        account_governance::get_account_governance_address_seeds, enums::GovernanceAccountType,
    },
    tools::account::create_and_serialize_account_signed,
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};

/// Processes CreateAccountGovernance instruction
#[allow(clippy::too_many_arguments)]
pub fn process_create_account_governance(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    realm: &Pubkey,
    governed_account: &Pubkey,
    vote_threshold: u8,
    min_instruction_hold_up_time: u64,
    max_voting_time: u64,
    token_threshold_to_create_proposal: u8,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let account_governance_info = next_account_info(account_info_iter)?; // 0
    let payer_info = next_account_info(account_info_iter)?; // 1
    let system_info = next_account_info(account_info_iter)?; // 2

    let rent_sysvar_info = next_account_info(account_info_iter)?; // 3
    let rent = &Rent::from_account_info(rent_sysvar_info)?;

    let account_governance_data = AccountGovernance {
        account_type: GovernanceAccountType::AccountGovernance,
        realm: *realm,
        vote_threshold,
        token_threshold_to_create_proposal,
        min_instruction_hold_up_time,
        governed_account: *governed_account,
        max_voting_time,
        proposal_count: 0,
    };

    create_and_serialize_account_signed::<AccountGovernance>(
        payer_info,
        &account_governance_info,
        &account_governance_data,
        &get_account_governance_address_seeds(realm, governed_account),
        program_id,
        system_info,
        rent,
    )?;

    Ok(())
}
