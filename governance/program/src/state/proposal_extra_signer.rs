//! Proposal extra account
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::pubkey::Pubkey;
use spl_governance_tools::account::AccountMaxSize;

/// ProposalExtraAccount (one per proposal)
/// This account aims to solve the problem of calling `system_instruction::create_account` from a proposal
/// Typically, when we create an account outside of a program, we call `system_instruction::create_account`
/// and both the newly created account and the funder need to sign.
/// In the context of spl-governance, we already have a funder (`NativeTreasury`), `ProposalExtraAccount`
/// is intended to be used as the newly created account.

#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct ProposalExtraAccount {}

impl AccountMaxSize for ProposalExtraAccount {
    fn get_max_size(&self) -> Option<usize> {
        Some(0)
    }
}
/// Returns ProposalExtraAccount PDA seeds
pub fn get_proposal_extra_account_seeds(proposal: &Pubkey) -> [&[u8]; 2] {
    [b"proposal_extra_account", proposal.as_ref()]
}

/// Returns ProposalExtraAccount PDA address
pub fn get_proposal_extra_account_address(program_id: &Pubkey, proposal: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&get_proposal_extra_account_seeds(proposal), program_id).0
}
