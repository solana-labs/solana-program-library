//! Token Owner Record Account

use borsh::maybestd::io::Write;
use std::slice::Iter;

use crate::{
    addins::voter_weight::{
        assert_is_valid_voter_weight, get_voter_weight_record_data_for_token_owner_record,
    },
    error::GovernanceError,
    state::{
        enums::GovernanceAccountType, governance::GovernanceConfig, legacy::TokenOwnerRecordV1,
        realm::RealmV2, realm_config::get_realm_config_data_for_realm,
    },
    PROGRAM_AUTHORITY_SEED,
};

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    borsh::try_from_slice_unchecked,
    program_error::ProgramError,
    program_pack::IsInitialized,
    pubkey::Pubkey,
};
use spl_governance_addin_api::voter_weight::VoterWeightAction;
use spl_governance_tools::account::{get_account_data, AccountMaxSize};

/// Governance Token Owner Record
/// Account PDA seeds: ['governance', realm, token_mint, token_owner ]
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct TokenOwnerRecordV2 {
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

    /// The number of votes cast by TokenOwner but not relinquished yet
    /// Every time a vote is cast this number is increased and it's always decreased when relinquishing a vote regardless of the vote state
    pub unrelinquished_votes_count: u32,

    /// The total number of votes cast by the TokenOwner
    /// If TokenOwner withdraws vote while voting is still in progress total_votes_count is decreased  and the vote doesn't count towards the total
    pub total_votes_count: u32,

    /// The number of outstanding proposals the TokenOwner currently owns
    /// The count is increased when TokenOwner creates a proposal
    /// and decreased  once it's either voted on (Succeeded or Defeated) or Cancelled
    /// By default it's restricted to 1 outstanding Proposal per token owner
    pub outstanding_proposal_count: u8,

    /// Reserved space for future versions
    pub reserved: [u8; 7],

    /// A single account that is allowed to operate governance with the deposited governing tokens
    /// It can be delegated to by the governing_token_owner or current governance_delegate
    pub governance_delegate: Option<Pubkey>,

    /// Reserved space for versions v2 and onwards
    /// Note: This space won't be available to v1 accounts until runtime supports resizing
    pub reserved_v2: [u8; 128],
}

impl AccountMaxSize for TokenOwnerRecordV2 {
    fn get_max_size(&self) -> Option<usize> {
        Some(282)
    }
}

impl IsInitialized for TokenOwnerRecordV2 {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::TokenOwnerRecordV2
    }
}

impl TokenOwnerRecordV2 {
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

    /// Asserts TokenOwner has enough tokens to be allowed to create proposal and doesn't have any outstanding proposals
    pub fn assert_can_create_proposal(
        &self,
        realm_data: &RealmV2,
        config: &GovernanceConfig,
        voter_weight: u64,
    ) -> Result<(), ProgramError> {
        let min_weight_to_create_proposal =
            if self.governing_token_mint == realm_data.community_mint {
                config.min_community_weight_to_create_proposal
            } else if Some(self.governing_token_mint) == realm_data.config.council_mint {
                config.min_council_weight_to_create_proposal
            } else {
                return Err(GovernanceError::InvalidGoverningTokenMint.into());
            };

        if voter_weight < min_weight_to_create_proposal {
            return Err(GovernanceError::NotEnoughTokensToCreateProposal.into());
        }

        // The number of outstanding proposals is currently restricted to 10
        // If there is a need to change it in the future then it should be added to realm or governance config
        if self.outstanding_proposal_count >= 10 {
            return Err(GovernanceError::TooManyOutstandingProposals.into());
        }

        Ok(())
    }

    /// Asserts TokenOwner has enough tokens to be allowed to create governance
    pub fn assert_can_create_governance(
        &self,
        realm_data: &RealmV2,
        voter_weight: u64,
    ) -> Result<(), ProgramError> {
        let min_weight_to_create_governance =
            if self.governing_token_mint == realm_data.community_mint {
                realm_data.config.min_community_weight_to_create_governance
            } else if Some(self.governing_token_mint) == realm_data.config.council_mint {
                // For council tokens it's enough to be in possession of any number of tokens
                1
            } else {
                return Err(GovernanceError::InvalidGoverningTokenMint.into());
            };

        if voter_weight < min_weight_to_create_governance {
            return Err(GovernanceError::NotEnoughTokensToCreateGovernance.into());
        }

        Ok(())
    }

    /// Asserts TokenOwner can withdraw tokens from Realm
    pub fn assert_can_withdraw_governing_tokens(&self) -> Result<(), ProgramError> {
        if self.unrelinquished_votes_count > 0 {
            return Err(
                GovernanceError::AllVotesMustBeRelinquishedToWithdrawGoverningTokens.into(),
            );
        }

        if self.outstanding_proposal_count > 0 {
            return Err(
                GovernanceError::AllProposalsMustBeFinalisedToWithdrawGoverningTokens.into(),
            );
        }

        Ok(())
    }

    /// Decreases outstanding_proposal_count
    pub fn decrease_outstanding_proposal_count(&mut self) {
        // Previous versions didn't use the count and it can be already 0
        // TODO: Remove this check once all outstanding proposals on mainnet are resolved
        if self.outstanding_proposal_count != 0 {
            self.outstanding_proposal_count =
                self.outstanding_proposal_count.checked_sub(1).unwrap();
        }
    }

    /// Resolves voter's weight using either the amount deposited into the realm or weight provided by voter weight addin (if configured)
    #[allow(clippy::too_many_arguments)]
    pub fn resolve_voter_weight(
        &self,
        program_id: &Pubkey,
        realm_config_info: &AccountInfo,
        account_info_iter: &mut Iter<AccountInfo>,
        realm: &Pubkey,
        realm_data: &RealmV2,
        weight_action: VoterWeightAction,
        weight_action_target: &Pubkey,
    ) -> Result<u64, ProgramError> {
        // if the realm uses addin for community voter weight then use the externally provided weight
        if realm_data.config.use_community_voter_weight_addin
            && realm_data.community_mint == self.governing_token_mint
        {
            let voter_weight_record_info = next_account_info(account_info_iter)?;

            let realm_config_data =
                get_realm_config_data_for_realm(program_id, realm_config_info, realm)?;

            let voter_weight_record_data = get_voter_weight_record_data_for_token_owner_record(
                &realm_config_data.community_voter_weight_addin.unwrap(),
                voter_weight_record_info,
                self,
            )?;

            assert_is_valid_voter_weight(
                &voter_weight_record_data,
                weight_action,
                weight_action_target,
            )?;

            Ok(voter_weight_record_data.voter_weight)
        } else {
            Ok(self.governing_token_deposit_amount)
        }
    }

    /// Serializes account into the target buffer
    pub fn serialize<W: Write>(self, writer: &mut W) -> Result<(), ProgramError> {
        if self.account_type == GovernanceAccountType::TokenOwnerRecordV2 {
            BorshSerialize::serialize(&self, writer)?
        } else if self.account_type == GovernanceAccountType::TokenOwnerRecordV1 {
            // V1 account can't be resized and we have to translate it back to the original format

            // If reserved_v2 is used it must be individually asses for v1 backward compatibility impact
            if self.reserved_v2 != [0; 128] {
                panic!("Extended data not supported by TokenOwnerRecordV1")
            }

            let token_owner_record_data_v1 = TokenOwnerRecordV1 {
                account_type: self.account_type,
                realm: self.realm,
                governing_token_mint: self.governing_token_mint,
                governing_token_owner: self.governing_token_owner,
                governing_token_deposit_amount: self.governing_token_deposit_amount,
                unrelinquished_votes_count: self.unrelinquished_votes_count,
                total_votes_count: self.total_votes_count,
                outstanding_proposal_count: self.outstanding_proposal_count,
                reserved: self.reserved,
                governance_delegate: self.governance_delegate,
            };

            BorshSerialize::serialize(&token_owner_record_data_v1, writer)?;
        }

        Ok(())
    }
}

/// Returns TokenOwnerRecord PDA address
pub fn get_token_owner_record_address(
    program_id: &Pubkey,
    realm: &Pubkey,
    governing_token_mint: &Pubkey,
    governing_token_owner: &Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &get_token_owner_record_address_seeds(realm, governing_token_mint, governing_token_owner),
        program_id,
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
    program_id: &Pubkey,
    token_owner_record_info: &AccountInfo,
) -> Result<TokenOwnerRecordV2, ProgramError> {
    let account_type: GovernanceAccountType =
        try_from_slice_unchecked(&token_owner_record_info.data.borrow())?;

    // If the account is V1 version then translate to V2
    if account_type == GovernanceAccountType::TokenOwnerRecordV1 {
        let token_owner_record_data_v1 =
            get_account_data::<TokenOwnerRecordV1>(program_id, token_owner_record_info)?;

        return Ok(TokenOwnerRecordV2 {
            account_type,

            realm: token_owner_record_data_v1.realm,
            governing_token_mint: token_owner_record_data_v1.governing_token_mint,
            governing_token_owner: token_owner_record_data_v1.governing_token_owner,
            governing_token_deposit_amount: token_owner_record_data_v1
                .governing_token_deposit_amount,
            unrelinquished_votes_count: token_owner_record_data_v1.unrelinquished_votes_count,
            total_votes_count: token_owner_record_data_v1.total_votes_count,
            outstanding_proposal_count: token_owner_record_data_v1.outstanding_proposal_count,
            reserved: token_owner_record_data_v1.reserved,
            governance_delegate: token_owner_record_data_v1.governance_delegate,

            // Add the extra reserved_v2 padding
            reserved_v2: [0; 128],
        });
    }

    get_account_data::<TokenOwnerRecordV2>(program_id, token_owner_record_info)
}

/// Deserializes TokenOwnerRecord account and checks its PDA against the provided seeds
pub fn get_token_owner_record_data_for_seeds(
    program_id: &Pubkey,
    token_owner_record_info: &AccountInfo,
    token_owner_record_seeds: &[&[u8]],
) -> Result<TokenOwnerRecordV2, ProgramError> {
    let (token_owner_record_address, _) =
        Pubkey::find_program_address(token_owner_record_seeds, program_id);

    if token_owner_record_address != *token_owner_record_info.key {
        return Err(GovernanceError::InvalidTokenOwnerRecordAccountAddress.into());
    }

    get_token_owner_record_data(program_id, token_owner_record_info)
}

/// Deserializes TokenOwnerRecord account and asserts it belongs to the given realm
pub fn get_token_owner_record_data_for_realm(
    program_id: &Pubkey,
    token_owner_record_info: &AccountInfo,
    realm: &Pubkey,
) -> Result<TokenOwnerRecordV2, ProgramError> {
    let token_owner_record_data = get_token_owner_record_data(program_id, token_owner_record_info)?;

    if token_owner_record_data.realm != *realm {
        return Err(GovernanceError::InvalidRealmForTokenOwnerRecord.into());
    }

    Ok(token_owner_record_data)
}

/// Deserializes TokenOwnerRecord account and  asserts it belongs to the given realm and is for the given governing mint
pub fn get_token_owner_record_data_for_realm_and_governing_mint(
    program_id: &Pubkey,
    token_owner_record_info: &AccountInfo,
    realm: &Pubkey,
    governing_token_mint: &Pubkey,
) -> Result<TokenOwnerRecordV2, ProgramError> {
    let token_owner_record_data =
        get_token_owner_record_data_for_realm(program_id, token_owner_record_info, realm)?;

    if token_owner_record_data.governing_token_mint != *governing_token_mint {
        return Err(GovernanceError::InvalidGoverningMintForTokenOwnerRecord.into());
    }

    Ok(token_owner_record_data)
}

///  Deserializes TokenOwnerRecord account and checks its address is the give proposal_owner
pub fn get_token_owner_record_data_for_proposal_owner(
    program_id: &Pubkey,
    token_owner_record_info: &AccountInfo,
    proposal_owner: &Pubkey,
) -> Result<TokenOwnerRecordV2, ProgramError> {
    if token_owner_record_info.key != proposal_owner {
        return Err(GovernanceError::InvalidProposalOwnerAccount.into());
    }

    get_token_owner_record_data(program_id, token_owner_record_info)
}

#[cfg(test)]
mod test {
    use solana_program::borsh::get_packed_len;

    use super::*;

    #[test]
    fn test_max_size() {
        let token_owner_record = TokenOwnerRecordV2 {
            account_type: GovernanceAccountType::TokenOwnerRecordV2,
            realm: Pubkey::new_unique(),
            governing_token_mint: Pubkey::new_unique(),
            governing_token_owner: Pubkey::new_unique(),
            governing_token_deposit_amount: 10,
            governance_delegate: Some(Pubkey::new_unique()),
            unrelinquished_votes_count: 1,
            total_votes_count: 1,
            outstanding_proposal_count: 1,
            reserved: [0; 7],
            reserved_v2: [0; 128],
        };

        let size = get_packed_len::<TokenOwnerRecordV2>();

        assert_eq!(token_owner_record.get_max_size(), Some(size));
    }
}
