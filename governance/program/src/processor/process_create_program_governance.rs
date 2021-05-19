//! Program state processor
//use crate::utils::assert_program_upgrade_authority;

use crate::{
    state::account_governance::AccountGovernance,
    state::{
        account_governance::get_program_governance_address_seeds, enums::GovernanceAccountType,
    },
    tools::{
        account::create_and_serialize_account_signed, bpf_loader::assert_program_upgrade_authority,
    },
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};

/// Processes CreateProgramGovernance instruction
#[allow(clippy::too_many_arguments)]
pub fn process_create_program_governance(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    realm: &Pubkey,
    governed_program: &Pubkey,
    vote_threshold: u8,
    min_instruction_hold_up_time: u64,
    max_voting_time: u64,
    token_threshold_to_create_proposal: u8,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let program_governance_info = next_account_info(account_info_iter)?; // 0

    let governed_program_data_info = next_account_info(account_info_iter)?; // 1
    let governed_program_upgrade_authority_info = next_account_info(account_info_iter)?; // 2

    let payer_info = next_account_info(account_info_iter)?; // 3
    let _bpf_upgrade_loader_account_info = next_account_info(account_info_iter)?; // 4

    let system_info = next_account_info(account_info_iter)?; // 5

    let rent_sysvar_info = next_account_info(account_info_iter)?; // 6
    let rent = &Rent::from_account_info(rent_sysvar_info)?;

    // Assert current program upgrade authority signed the transaction as a temp. workaround until we can set_upgrade_authority via CPI.
    // Even though it doesn't transfer authority to the governance at the creation time it prevents from creating governance for programs owned by somebody else
    // After governance is created upgrade authority can be transferred to governance using CLI call.

    assert_program_upgrade_authority(
        &program_governance_info.key,
        governed_program,
        governed_program_data_info,
        governed_program_upgrade_authority_info,
    )?;

    // TODO: Uncomment once PR to allow set_upgrade_authority via CPI calls is released  https://github.com/solana-labs/solana/pull/16676
    // let set_upgrade_authority_ix = bpf_loader_upgradeable::set_upgrade_authority(
    //     &governed_program_account_info.key,
    //     &governed_program_upgrade_authority_account_info.key,
    //     Some(&governance_key),
    // );

    // let accounts = &[
    //     payer_account_info.clone(),
    //     bpf_upgrade_loader_account_info.clone(),
    //     governed_program_upgrade_authority_account_info.clone(),
    //     governance_account_info.clone(),
    //     governed_program_data_account_info.clone(),
    // ];
    // invoke(&set_upgrade_authority_ix, accounts)?;

    let program_governance_data = AccountGovernance {
        account_type: GovernanceAccountType::AccountGovernance,
        realm: *realm,
        vote_threshold,
        token_threshold_to_create_proposal,
        min_instruction_hold_up_time,
        governed_account: *governed_program,
        max_voting_time,
        proposal_count: 0,
    };

    create_and_serialize_account_signed::<AccountGovernance>(
        payer_info,
        &program_governance_info,
        &program_governance_data,
        &get_program_governance_address_seeds(realm, governed_program),
        program_id,
        system_info,
        rent,
    )?;

    Ok(())
}
