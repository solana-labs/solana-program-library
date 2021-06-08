//! Token Owner Record Account

use crate::{
    error::GovernanceError,
    id,
    tools::account::{get_account_data, AccountMaxSize},
    PROGRAM_AUTHORITY_SEED,
};

use crate::state::enums::GovernanceAccountType;

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, program_pack::IsInitialized,
    pubkey::Pubkey,
};

/// Governance Token Owner Record
/// Account PDA seeds: ['governance', realm, token_mint, token_owner ]
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct TokenOwnerRecord {
    /// Governance account type
    pub account_type: GovernanceAccountType,

    /// The Realm the TokenOwnerRecord belongs to
    pub realm: Pubkey,

    /// Governing Token Mint the TokenOwnerRecord holds deposit for
    pub governing_token_mint: Pubkey,

    /// The owner (either single or multisig) of the deposited governing SPL Tokens
    /// This is who can authorize a withdrawal of the tokens
    pub governing_token_owner: Pubkey,

    /// The amount of governing tokens deposited into the Realm
    /// This amount is the voter weight used when voting on proposals
    pub governing_token_deposit_amount: u64,

    /// A single account that is allowed to operate governance with the deposited governing tokens
    /// It can be delegated to by the governing_token_owner or current governance_delegate
    pub governance_delegate: Option<Pubkey>,

    /// The number of votes cast by TokenOwner but not relinquished yet
    /// Every time a vote is cast this number is increased and it's always decreased when relinquishing a vote regardless of the vote state
    pub unrelinquished_votes_count: u32,

    /// The total number of votes cast by the TokenOwner
    /// If TokenOwner withdraws vote while voting is still in progress total_votes_count is decreased  and the vote doesn't count towards the total
    pub total_votes_count: u32,
}

impl AccountMaxSize for TokenOwnerRecord {
    fn get_max_size(&self) -> Option<usize> {
        Some(146)
    }
}

impl IsInitialized for TokenOwnerRecord {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::TokenOwnerRecord
    }
}

impl TokenOwnerRecord {
    /// Checks whether the provided Governance Authority signed transaction
    pub fn assert_token_owner_or_delegate_is_signer(
        &self,
        governance_authority_info: &AccountInfo,
    ) -> Result<(), ProgramError> {
        if governance_authority_info.is_signer {
            if &self.governing_token_owner == governance_authority_info.key {
                return Ok(());
            }

            if let Some(governance_delegate) = self.governance_delegate {
                if &governance_delegate == governance_authority_info.key {
                    return Ok(());
                }
            };
        }

        Err(GovernanceError::GoverningTokenOwnerOrDelegateMustSign.into())
    }
}

/// Returns TokenOwnerRecord PDA address
pub fn get_token_owner_record_address(
    realm: &Pubkey,
    governing_token_mint: &Pubkey,
    governing_token_owner: &Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &get_token_owner_record_address_seeds(realm, governing_token_mint, governing_token_owner),
        &id(),
    )
    .0
}

/// Returns TokenOwnerRecord PDA seeds
pub fn get_token_owner_record_address_seeds<'a>(
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

/// Deserializes TokenOwnerRecord account and checks owner program
pub fn get_token_owner_record_data(
    token_owner_record_info: &AccountInfo,
) -> Result<TokenOwnerRecord, ProgramError> {
    get_account_data::<TokenOwnerRecord>(token_owner_record_info, &id())
}

/// Deserializes TokenOwnerRecord account and checks its PDA against the provided seeds
pub fn get_token_owner_record_data_for_seeds(
    token_owner_record_info: &AccountInfo,
    token_owner_record_seeds: &[&[u8]],
) -> Result<TokenOwnerRecord, ProgramError> {
    let (token_owner_record_address, _) =
        Pubkey::find_program_address(token_owner_record_seeds, &id());

    if token_owner_record_address != *token_owner_record_info.key {
        return Err(GovernanceError::InvalidTokenOwnerRecordAccountAddress.into());
    }

    get_token_owner_record_data(token_owner_record_info)
}

/// Deserializes TokenOwnerRecord account and checks that its PDA matches the given realm and governing mint
pub fn get_token_owner_record_data_for_realm_and_governing_mint(
    token_owner_record_info: &AccountInfo,
    realm: &Pubkey,
    governing_token_mint: &Pubkey,
) -> Result<TokenOwnerRecord, ProgramError> {
    let token_owner_record_data = get_token_owner_record_data(token_owner_record_info)?;

    if token_owner_record_data.governing_token_mint != *governing_token_mint {
        return Err(GovernanceError::InvalidGoverningMintForTokenOwnerRecord.into());
    }

    if token_owner_record_data.realm != *realm {
        return Err(GovernanceError::InvalidRealmForTokenOwnerRecord.into());
    }

    Ok(token_owner_record_data)
}

///  Deserializes TokenOwnerRecord account and checks its address is the give proposal_owner
pub fn get_token_owner_record_data_for_proposal_owner(
    token_owner_record_info: &AccountInfo,
    proposal_owner: &Pubkey,
) -> Result<TokenOwnerRecord, ProgramError> {
    if token_owner_record_info.key != proposal_owner {
        return Err(GovernanceError::InvalidProposalOwnerAccount.into());
    }

    get_token_owner_record_data(token_owner_record_info)
}

#[cfg(test)]
mod test {
    use solana_program::borsh::get_packed_len;

    use super::*;

    #[test]
    fn test_max_size() {
        let token_owner_record = TokenOwnerRecord {
            account_type: GovernanceAccountType::TokenOwnerRecord,
            realm: Pubkey::new_unique(),
            governing_token_mint: Pubkey::new_unique(),
            governing_token_owner: Pubkey::new_unique(),
            governing_token_deposit_amount: 10,
            governance_delegate: Some(Pubkey::new_unique()),
            unrelinquished_votes_count: 1,
            total_votes_count: 1,
        };

        let size = get_packed_len::<TokenOwnerRecord>();

        assert_eq!(token_owner_record.get_max_size(), Some(size));
    }
}
