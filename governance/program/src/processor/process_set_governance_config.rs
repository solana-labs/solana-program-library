//! Program state processor

use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

use crate::{
    error::GovernanceError,
    state::governance::{assert_is_valid_governance_config, get_governance_data, GovernanceConfig},
};

/// Processes SetGovernanceConfig instruction
pub fn process_set_governance_config(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    config: GovernanceConfig,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let governance_info = next_account_info(account_info_iter)?; // 0

    // Only governance PDA via a proposal can authorize change to its own config
    if !governance_info.is_signer {
        return Err(GovernanceError::GovernancePdaMustSign.into());
    }

    assert_is_valid_governance_config(&config)?;

    let mut governance_data = get_governance_data(program_id, governance_info)?;
    governance_data.config = config;

    governance_data.serialize(&mut *governance_info.data.borrow_mut())?;

    Ok(())
}
