//! Proposal deposit account

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, program_pack::IsInitialized,
    pubkey::Pubkey,
};
use spl_governance_tools::account::{get_account_data, AccountMaxSize};

use crate::{error::GovernanceError, state::enums::GovernanceAccountType};

/// Proposal deposit account
/// The account is used to limit spam of proposals
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct ProposalDeposit {
    /// Governance account type
    pub account_type: GovernanceAccountType,

    /// The Proposal the deposit belongs to
    pub proposal: Pubkey,

    /// The account which payed for the deposit
    pub deposit_payer: Pubkey,

    /// Reserved
    pub reserved: [u8; 64],
}

impl AccountMaxSize for ProposalDeposit {
    fn get_max_size(&self) -> Option<usize> {
        Some(1 + 32 + 32 + 64)
    }
}

impl IsInitialized for ProposalDeposit {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::ProposalDeposit
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

/// Deserializes ProposalDeposit account and checks owner program and account type
pub fn get_proposal_deposit_data(
    program_id: &Pubkey,
    proposal_deposit_info: &AccountInfo,
) -> Result<ProposalDeposit, ProgramError> {
    get_account_data::<ProposalDeposit>(program_id, proposal_deposit_info)
}

/// Deserializes ProposalDeposit account
/// 1) Checks owner program and account type
/// 2) Asserts it belongs to the given Proposal and deposit Payer
pub fn get_proposal_deposit_data_for_proposal_and_deposit_payer(
    program_id: &Pubkey,
    proposal_deposit_info: &AccountInfo,
    proposal: &Pubkey,
    proposal_deposit_payer: &Pubkey,
) -> Result<ProposalDeposit, ProgramError> {
    let proposal_deposit_data = get_proposal_deposit_data(program_id, proposal_deposit_info)?;

    if proposal_deposit_data.proposal != *proposal {
        return Err(GovernanceError::InvalidProposalForProposalDeposit.into());
    }

    if proposal_deposit_data.deposit_payer != *proposal_deposit_payer {
        return Err(GovernanceError::InvalidDepositPayerForProposalDeposit.into());
    }

    Ok(proposal_deposit_data)
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_max_size() {
        // Arrange
        let proposal_deposit_data = ProposalDeposit {
            account_type: GovernanceAccountType::ProposalDeposit,
            proposal: Pubkey::new_unique(),
            deposit_payer: Pubkey::new_unique(),
            reserved: [0; 64],
        };

        // Act
        let size = proposal_deposit_data.try_to_vec().unwrap().len();

        // Assert
        assert_eq!(proposal_deposit_data.get_max_size(), Some(size));
    }
}
