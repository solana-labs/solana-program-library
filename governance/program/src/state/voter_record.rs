//! Voter Record Account

use crate::{
    error::GovernanceError,
    id,
    tools::account::{deserialize_account, AccountMaxSize},
    PROGRAM_AUTHORITY_SEED,
};

use super::enums::{GovernanceAccountType, GoverningTokenType};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, program_pack::IsInitialized,
    pubkey::Pubkey,
};

/// Governance Voter Record
/// Account PDA seeds: ['governance', realm, token_mint, token_owner ]
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct VoterRecord {
    /// Governance account type
    pub account_type: GovernanceAccountType,

    /// The Realm the VoterRecord belongs to
    pub realm: Pubkey,

    /// The type of the Governing Token the VoteRecord is for
    pub token_type: GoverningTokenType,

    /// The owner (either single or multisig) of the deposited governing SPL Tokens
    /// This is who can authorize a withdrawal
    pub token_owner: Pubkey,

    /// The amount of governing tokens deposited into the Realm
    /// This amount is the voter weight used when voting on proposals
    pub token_deposit_amount: u64,

    /// A single account that is allowed to operate governance with the deposited governing tokens
    /// It's delegated to by the governing token owner or current vote_authority
    pub vote_authority: Option<Pubkey>,

    /// The number of active votes cast by voter
    pub active_votes_count: u8,

    /// The total number of votes cast by the voter
    pub total_votes_count: u8,
}

impl AccountMaxSize for VoterRecord {
    fn get_max_size(&self) -> Option<usize> {
        Some(109)
    }
}

impl IsInitialized for VoterRecord {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::VoterRecord
    }
}

/// Returns VoteRecord PDA address
pub fn get_voter_record_address(
    realm: &Pubkey,
    governing_token_mint: &Pubkey,
    governing_token_owner: &Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &get_voter_record_address_seeds(realm, governing_token_mint, governing_token_owner),
        &id(),
    )
    .0
}

/// Returns VoterRecord PDA seeds
pub fn get_voter_record_address_seeds<'a>(
    realm: &'a Pubkey,
    governing_token_mint: &'a Pubkey,
    governing_token_owner: &'a Pubkey,
) -> [&'a [u8]; 4] {
    [
        PROGRAM_AUTHORITY_SEED,
        realm.as_ref(),
        governing_token_mint.as_ref(),
        governing_token_owner.as_ref(),
    ]
}

/// Deserializes VoterRecord and checks account PDA and  owner program
pub fn deserialize_voter_record(
    voter_record_info: &AccountInfo,
    voter_record_seeds: &[&[u8]],
) -> Result<VoterRecord, ProgramError> {
    let (voter_record_address, _) = Pubkey::find_program_address(voter_record_seeds, &id());

    if voter_record_address != *voter_record_info.key {
        return Err(GovernanceError::InvalidVoterAccountAddress.into());
    }

    deserialize_account::<VoterRecord>(voter_record_info, &id())
}

#[cfg(test)]
mod test {
    use solana_program::borsh::get_packed_len;

    use super::*;

    #[test]
    fn test_max_size() {
        let vote_record = VoterRecord {
            account_type: GovernanceAccountType::VoterRecord,
            realm: Pubkey::new_unique(),
            token_type: GoverningTokenType::Community,
            token_owner: Pubkey::new_unique(),
            token_deposit_amount: 10,
            vote_authority: Some(Pubkey::new_unique()),
            active_votes_count: 1,
            total_votes_count: 1,
        };

        let size = get_packed_len::<VoterRecord>();

        assert_eq!(vote_record.get_max_size(), Some(size));
    }
}
