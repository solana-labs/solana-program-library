//! Program state processor

use crate::utils::assert_program_upgrade_authority;
use crate::{
    error::GovernanceError, state::governance::Governance, utils::create_account_raw,
    PROGRAM_AUTHORITY_SEED,
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    // program::invoke, bpf_loader_upgradeable,
    pubkey::Pubkey,
};

/// Create empty Governance
pub fn process_create_empty_governance(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let governance_account_info = next_account_info(account_info_iter)?; // 0
    let governed_program_account_info = next_account_info(account_info_iter)?; // 1
    let governed_program_data_account_info = next_account_info(account_info_iter)?; // 2
    let governed_program_upgrade_authority_account_info = next_account_info(account_info_iter)?; // 3
    let payer_account_info = next_account_info(account_info_iter)?; // 4
    let system_account_info = next_account_info(account_info_iter)?; // 5
    let _bpf_upgrade_loader_account_info = next_account_info(account_info_iter)?; // 6

    let mut seeds = vec![
        PROGRAM_AUTHORITY_SEED,
        governed_program_account_info.key.as_ref(),
    ];
    let (governance_key, bump_seed) = Pubkey::find_program_address(&seeds[..], program_id);

    if governance_account_info.key != &governance_key {
        return Err(GovernanceError::InvalidGovernanceKey.into());
    }
    let bump = &[bump_seed];
    seeds.push(bump);
    let authority_signer_seeds = &seeds[..];

    create_account_raw::<Governance>(
        &[
            payer_account_info.clone(),
            governance_account_info.clone(),
            system_account_info.clone(),
        ],
        &governance_key,
        payer_account_info.key,
        program_id,
        authority_signer_seeds,
    )?;

    // Assert current program upgrade authority signed the transaction as a temp. workaround until we can set_upgrade_authority via CPI.
    // Even though it doesn't transfer authority to the governance at the creation time it prevents from creating governance for programs owned by somebody else
    // After governance is created upgrade authority can be transferred to governance using CLI call.
    assert_program_upgrade_authority(
        &governance_key,
        governed_program_account_info.key,
        governed_program_data_account_info,
        governed_program_upgrade_authority_account_info,
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

    Ok(())
}
