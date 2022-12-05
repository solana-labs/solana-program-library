//! Proposal deposit account

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};
use spl_governance_tools::{account::AccountMaxSize, error::GovernanceToolsError};

use crate::error::GovernanceError;

/// Proposal deposit account
/// The account has no data and is used to limit spam of proposals
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct ProposalDeposit {}

impl AccountMaxSize for ProposalDeposit {
    fn get_max_size(&self) -> Option<usize> {
        Some(0)
    }
}

/// Returns ProposalDeposit PDA seeds
pub fn get_proposal_deposit_address_seeds<'a>(
    proposal: &'a Pubkey,
    proposal_deposit_payer: &'a Pubkey,
) -> [&'a [u8]; 3] {
    [
        b"proposal-deposit",
        proposal.as_ref(),
        proposal_deposit_payer.as_ref(),
    ]
}

/// Returns ProposalDeposit PDA address
pub fn get_proposal_deposit_address(
    program_id: &Pubkey,
    proposal: &Pubkey,
    proposal_deposit_payer: &Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &get_proposal_deposit_address_seeds(proposal, proposal_deposit_payer),
        program_id,
    )
    .0
}

/// Asserts the given ProposalDeposit account address is derived from to the Proposal and the deposit payer
pub fn assert_is_valid_proposal_deposit_account(
    program_id: &Pubkey,
    proposal_deposit_info: &AccountInfo,
    proposal: &Pubkey,
    proposal_deposit_payer: &Pubkey,
) -> Result<(), ProgramError> {
    if proposal_deposit_info.owner != program_id {
        return Err(GovernanceToolsError::InvalidAccountOwner.into());
    }

    let proposal_deposit_address =
        get_proposal_deposit_address(program_id, proposal, proposal_deposit_payer);

    if *proposal_deposit_info.key != proposal_deposit_address {
        return Err(GovernanceError::InvalidProposalDepositAccountAddress.into());
    }

    Ok(())
}
