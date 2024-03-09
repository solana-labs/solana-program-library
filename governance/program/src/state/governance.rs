//! Governance Account
use {
    crate::{
        error::GovernanceError,
        state::{
            enums::{GovernanceAccountType, VoteThreshold, VoteTipping},
            legacy::{is_governance_v1_account_type, GovernanceV1},
            realm::{assert_is_valid_realm, RealmV2},
            vote_record::VoteKind,
        },
        tools::structs::Reserved119,
    },
    borsh::{io::Write, BorshDeserialize, BorshSchema, BorshSerialize},
    solana_program::{
        account_info::AccountInfo, program_error::ProgramError, program_pack::IsInitialized,
        pubkey::Pubkey, rent::Rent,
    },
    spl_governance_tools::{
        account::{
            assert_is_valid_account_of_types, extend_account_size, get_account_data,
            get_account_type, AccountMaxSize,
        },
        error::GovernanceToolsError,
    },
};

/// Governance config
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct GovernanceConfig {
    /// The type of the vote threshold used for community vote
    /// Note: In the current version only YesVotePercentage and Disabled
    /// thresholds are supported
    pub community_vote_threshold: VoteThreshold,

    /// Minimum community weight a governance token owner must possess to be
    /// able to create a proposal
    pub min_community_weight_to_create_proposal: u64,

    /// The wait time in seconds before transactions can be executed after
    /// proposal is successfully voted on
    pub transactions_hold_up_time: u32,

    /// The base voting time in seconds for proposal to be open for voting
    /// Voting is unrestricted during the base voting time and any vote types
    /// can be cast The base voting time can be extend by optional cool off
    /// time when only negative votes (Veto and Deny) are allowed
    pub voting_base_time: u32,

    /// Conditions under which a Community vote will complete early
    pub community_vote_tipping: VoteTipping,

    /// The type of the vote threshold used for council vote
    /// Note: In the current version only YesVotePercentage and Disabled
    /// thresholds are supported
    pub council_vote_threshold: VoteThreshold,

    /// The threshold for Council Veto votes
    pub council_veto_vote_threshold: VoteThreshold,

    /// Minimum council weight a governance token owner must possess to be able
    /// to create a proposal
    pub min_council_weight_to_create_proposal: u64,

    /// Conditions under which a Council vote will complete early
    pub council_vote_tipping: VoteTipping,

    /// The threshold for Community Veto votes
    pub community_veto_vote_threshold: VoteThreshold,

    /// Voting cool of time
    pub voting_cool_off_time: u32,

    /// The number of active proposals exempt from the Proposal security deposit
    pub deposit_exempt_proposal_count: u8,
}

/// The default number of active proposals exempt from security deposit
pub const DEFAULT_DEPOSIT_EXEMPT_PROPOSAL_COUNT: u8 = 10;

/// Security deposit is paid when a Proposal is created and can be refunded
/// after voting ends or the Proposals is cancelled
pub const SECURITY_DEPOSIT_BASE_LAMPORTS: u64 = 100_000_000; // 0.1 SOL

/// Governance Account
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct GovernanceV2 {
    /// Account type. It can be Uninitialized, Governance, ProgramGovernance,
    /// TokenGovernance or MintGovernance
    pub account_type: GovernanceAccountType,

    /// Governance Realm
    pub realm: Pubkey,

    /// The seed used to create Governance account PDA
    ///
    /// Note: For the legacy asset specific Governance accounts
    /// the seed by convention is:
    /// MintGovernance -> mint address
    /// TokenAccountGovernance -> token account address
    /// ProgramGovernance -> program address
    pub governance_seed: Pubkey,

    /// Reserved space for future versions
    pub reserved1: u32,

    /// Governance config
    pub config: GovernanceConfig,

    /// Reserved space for versions v2 and onwards
    /// Note 1: V1 accounts must be resized before using this space
    /// Note 2: The reserved space should be used from the end to also allow the
    /// config to grow if needed
    pub reserved_v2: Reserved119,

    /// The number of required signatories for proposals in the Governance
    pub required_signatories_count: u8,

    /// The number of active proposals where active means Draft, SigningOff or
    /// Voting state
    ///
    /// Note: The counter was introduced in program V3 and didn't exist in
    /// program V1 & V2 If the program is upgraded from program V1 or V2
    /// while there are any outstanding active proposals the counter won't
    /// be accurate until all proposals are transitioned to an inactive final
    /// state and the counter reset
    pub active_proposal_count: u64,
}

impl AccountMaxSize for GovernanceV2 {
    fn get_max_size(&self) -> Option<usize> {
        Some(236)
    }
}

/// Checks if the given account type is one of the Governance V2 account types
pub fn is_governance_v2_account_type(account_type: &GovernanceAccountType) -> bool {
    match account_type {
        GovernanceAccountType::GovernanceV2
        | GovernanceAccountType::ProgramGovernanceV2
        | GovernanceAccountType::MintGovernanceV2
        | GovernanceAccountType::TokenGovernanceV2 => true,
        GovernanceAccountType::Uninitialized
        | GovernanceAccountType::RealmV1
        | GovernanceAccountType::RealmV2
        | GovernanceAccountType::RealmConfig
        | GovernanceAccountType::TokenOwnerRecordV1
        | GovernanceAccountType::TokenOwnerRecordV2
        | GovernanceAccountType::GovernanceV1
        | GovernanceAccountType::ProgramGovernanceV1
        | GovernanceAccountType::MintGovernanceV1
        | GovernanceAccountType::TokenGovernanceV1
        | GovernanceAccountType::ProposalV1
        | GovernanceAccountType::ProposalV2
        | GovernanceAccountType::SignatoryRecordV1
        | GovernanceAccountType::SignatoryRecordV2
        | GovernanceAccountType::ProposalInstructionV1
        | GovernanceAccountType::ProposalTransactionV2
        | GovernanceAccountType::VoteRecordV1
        | GovernanceAccountType::VoteRecordV2
        | GovernanceAccountType::ProgramMetadata
        | GovernanceAccountType::ProposalDeposit
        | GovernanceAccountType::RequiredSignatory => false,
    }
}

/// Returns GovernanceV2 type for given GovernanceV1 type or None if the given
/// account type is not GovernanceV1
pub fn try_get_governance_v2_type_for_v1(
    account_type: &GovernanceAccountType,
) -> Option<GovernanceAccountType> {
    match account_type {
        GovernanceAccountType::GovernanceV1 => Some(GovernanceAccountType::GovernanceV2),
        GovernanceAccountType::ProgramGovernanceV1 => {
            Some(GovernanceAccountType::ProgramGovernanceV2)
        }
        GovernanceAccountType::MintGovernanceV1 => Some(GovernanceAccountType::MintGovernanceV2),
        GovernanceAccountType::TokenGovernanceV1 => Some(GovernanceAccountType::TokenGovernanceV2),
        GovernanceAccountType::Uninitialized
        | GovernanceAccountType::RealmV1
        | GovernanceAccountType::RealmV2
        | GovernanceAccountType::RealmConfig
        | GovernanceAccountType::TokenOwnerRecordV1
        | GovernanceAccountType::TokenOwnerRecordV2
        | GovernanceAccountType::GovernanceV2
        | GovernanceAccountType::ProgramGovernanceV2
        | GovernanceAccountType::MintGovernanceV2
        | GovernanceAccountType::TokenGovernanceV2
        | GovernanceAccountType::ProposalV1
        | GovernanceAccountType::ProposalV2
        | GovernanceAccountType::SignatoryRecordV1
        | GovernanceAccountType::SignatoryRecordV2
        | GovernanceAccountType::ProposalInstructionV1
        | GovernanceAccountType::ProposalTransactionV2
        | GovernanceAccountType::VoteRecordV1
        | GovernanceAccountType::VoteRecordV2
        | GovernanceAccountType::ProgramMetadata
        | GovernanceAccountType::ProposalDeposit
        | GovernanceAccountType::RequiredSignatory => None,
    }
}

/// Checks if the given account type is on of the Governance account types of
/// any version
pub fn is_governance_account_type(account_type: &GovernanceAccountType) -> bool {
    is_governance_v1_account_type(account_type) || is_governance_v2_account_type(account_type)
}

impl IsInitialized for GovernanceV2 {
    fn is_initialized(&self) -> bool {
        is_governance_v2_account_type(&self.account_type)
    }
}

impl GovernanceV2 {
    /// Returns Governance PDA seeds
    pub fn get_governance_address_seeds(&self) -> Result<[&[u8]; 3], ProgramError> {
        let seeds = match self.account_type {
            GovernanceAccountType::GovernanceV1 | GovernanceAccountType::GovernanceV2 => {
                get_governance_address_seeds(&self.realm, &self.governance_seed)
            }
            GovernanceAccountType::ProgramGovernanceV1
            | GovernanceAccountType::ProgramGovernanceV2 => {
                get_program_governance_address_seeds(&self.realm, &self.governance_seed)
            }
            GovernanceAccountType::MintGovernanceV1 | GovernanceAccountType::MintGovernanceV2 => {
                get_mint_governance_address_seeds(&self.realm, &self.governance_seed)
            }
            GovernanceAccountType::TokenGovernanceV1 | GovernanceAccountType::TokenGovernanceV2 => {
                get_token_governance_address_seeds(&self.realm, &self.governance_seed)
            }
            GovernanceAccountType::Uninitialized
            | GovernanceAccountType::RealmV1
            | GovernanceAccountType::TokenOwnerRecordV1
            | GovernanceAccountType::ProposalV1
            | GovernanceAccountType::SignatoryRecordV1
            | GovernanceAccountType::VoteRecordV1
            | GovernanceAccountType::ProposalInstructionV1
            | GovernanceAccountType::RealmConfig
            | GovernanceAccountType::VoteRecordV2
            | GovernanceAccountType::ProposalTransactionV2
            | GovernanceAccountType::ProposalV2
            | GovernanceAccountType::ProgramMetadata
            | GovernanceAccountType::ProposalDeposit
            | GovernanceAccountType::RealmV2
            | GovernanceAccountType::TokenOwnerRecordV2
            | GovernanceAccountType::SignatoryRecordV2
            | GovernanceAccountType::RequiredSignatory => {
                return Err(GovernanceToolsError::InvalidAccountType.into())
            }
        };

        Ok(seeds)
    }

    /// Serializes account into the target buffer
    pub fn serialize<W: Write>(self, writer: W) -> Result<(), ProgramError> {
        if is_governance_v2_account_type(&self.account_type) {
            borsh::to_writer(writer, &self)?
        } else if is_governance_v1_account_type(&self.account_type) {
            // V1 account can't be resized and we have to translate it back to the original
            // format

            // If reserved_v2 is used it must be individually assessed for GovernanceV1
            // account backward compatibility impact
            if self.reserved_v2 != Reserved119::default() {
                panic!("Extended data not supported by GovernanceV1")
            }

            // Note: active_proposal_count is not preserved on GovernanceV1 account until
            // it's migrated to GovernanceV2 during Proposal creation

            let governance_data_v1 = GovernanceV1 {
                account_type: self.account_type,
                realm: self.realm,
                governance_seed: self.governance_seed,
                proposals_count: 0,
                config: self.config,
            };

            borsh::to_writer(writer, &governance_data_v1)?
        }

        Ok(())
    }

    /// Serializes Governance accounts as GovernanceV2
    /// If the account is GovernanceV1 then it changes its type to GovernanceV2
    /// and resizes account data Note: It supports all the specialized
    /// Governance account types (Governance, ProgramGovernance, MintGovernance
    /// and TokenGovernance)
    pub fn serialize_as_governance_v2<'a>(
        mut self,
        governance_info: &AccountInfo<'a>,
        payer_info: &AccountInfo<'a>,
        system_info: &AccountInfo<'a>,
        rent: &Rent,
    ) -> Result<(), ProgramError> {
        // If the Governance account is GovernanceV1 reallocate its size and change type
        // to GovernanceV2
        if let Some(governance_v2_type) = try_get_governance_v2_type_for_v1(&self.account_type) {
            // Change type to GovernanceV2
            // Note: Only type change is required because the account data was translated to
            // GovernanceV2 during deserialisation
            self.account_type = governance_v2_type;

            extend_account_size(
                governance_info,
                payer_info,
                self.get_max_size().unwrap(),
                rent,
                system_info,
            )?;
        }

        self.serialize(&mut governance_info.data.borrow_mut()[..])
    }

    /// Asserts the provided voting population represented by the given
    /// governing_token_mint can cast the given vote type on proposals for
    /// the Governance
    pub fn assert_governing_token_mint_can_vote(
        &self,
        realm_data: &RealmV2,
        vote_governing_token_mint: &Pubkey,
        vote_kind: &VoteKind,
    ) -> Result<(), ProgramError> {
        // resolve_vote_threshold() asserts the vote threshold exists for the given
        // governing_token_mint and is not disabled
        let _ = self.resolve_vote_threshold(realm_data, vote_governing_token_mint, vote_kind)?;

        Ok(())
    }

    /// Resolves VoteThreshold for the given realm, governing token and Vote
    /// kind
    pub fn resolve_vote_threshold(
        &self,
        realm_data: &RealmV2,
        vote_governing_token_mint: &Pubkey,
        vote_kind: &VoteKind,
    ) -> Result<VoteThreshold, ProgramError> {
        let vote_threshold = if realm_data.community_mint == *vote_governing_token_mint {
            match vote_kind {
                VoteKind::Electorate => &self.config.community_vote_threshold,
                VoteKind::Veto => &self.config.community_veto_vote_threshold,
            }
        } else if realm_data.config.council_mint == Some(*vote_governing_token_mint) {
            match vote_kind {
                VoteKind::Electorate => &self.config.council_vote_threshold,
                VoteKind::Veto => &self.config.council_veto_vote_threshold,
            }
        } else {
            return Err(GovernanceError::InvalidGoverningTokenMint.into());
        };

        if *vote_threshold == VoteThreshold::Disabled {
            return Err(GovernanceError::GoverningTokenMintNotAllowedToVote.into());
        }

        Ok(vote_threshold.clone())
    }

    /// Returns VoteTipping for the given governing_token_mint
    pub fn get_vote_tipping(
        &self,
        realm_data: &RealmV2,
        governing_token_mint: &Pubkey,
    ) -> Result<&VoteTipping, ProgramError> {
        let vote_tipping = if *governing_token_mint == realm_data.community_mint {
            &self.config.community_vote_tipping
        } else if Some(*governing_token_mint) == realm_data.config.council_mint {
            &self.config.council_vote_tipping
        } else {
            return Err(GovernanceError::InvalidGoverningTokenMint.into());
        };

        Ok(vote_tipping)
    }

    /// Returns the required deposit amount for creating Nth Proposal based on
    /// the number of active proposals where N equals to
    /// active_proposal_count - deposit_exempt_proposal_count The deposit is
    /// not paid unless there are more active Proposal than the exempt amount
    ///
    /// Note: The exact deposit paid for Nth Proposal is
    /// N*SECURITY_DEPOSIT_BASE_LAMPORTS + min_rent_for(ProposalDeposit)
    ///
    /// Note: Although the deposit amount paid for Nth proposal is linear the
    /// total deposit amount required to create N proposals is sum of arithmetic
    /// series Dn = N*r + d*N*(N+1)/2
    // where:
    // Dn - The total deposit amount required to create N proposals
    // N = active_proposal_count - deposit_exempt_proposal_count
    // d = SECURITY_DEPOSIT_BASE_LAMPORTS
    // r = min rent amount for ProposalDeposit
    pub fn get_proposal_deposit_amount(&self) -> u64 {
        self.active_proposal_count
            .saturating_sub(self.config.deposit_exempt_proposal_count as u64)
            .saturating_mul(SECURITY_DEPOSIT_BASE_LAMPORTS)
    }
}

/// Deserializes Governance account and checks owner program
pub fn get_governance_data(
    program_id: &Pubkey,
    governance_info: &AccountInfo,
) -> Result<GovernanceV2, ProgramError> {
    let account_type: GovernanceAccountType = get_account_type(program_id, governance_info)?;

    // If the account is V1 version then translate to V2
    let mut governance_data = if is_governance_v1_account_type(&account_type) {
        let governance_data_v1 = get_account_data::<GovernanceV1>(program_id, governance_info)?;

        GovernanceV2 {
            account_type,
            realm: governance_data_v1.realm,
            governance_seed: governance_data_v1.governance_seed,
            reserved1: 0,
            config: governance_data_v1.config,
            reserved_v2: Reserved119::default(),
            required_signatories_count: 0,
            // GovernanceV1 layout doesn't support active_proposal_count
            // For any legacy GovernanceV1 account it's not preserved until the account layout is
            // migrated to GovernanceV2 in CreateProposal
            active_proposal_count: 0,
        }
    } else {
        get_account_data::<GovernanceV2>(program_id, governance_info)?
    };

    // In previous versions of spl-gov (< 3) we had
    // config.proposal_cool_off_time:u32 which was unused and always 0
    // In version 3.0.0 proposal_cool_off_time was replaced with
    // council_vote_threshold:VoteThreshold and
    // council_veto_vote_threshold:VoteThreshold If we read a legacy account
    // then council_vote_threshold == VoteThreshold::YesVotePercentage(0)
    //
    // Note: assert_is_valid_governance_config() prevents setting
    // council_vote_threshold to VoteThreshold::YesVotePercentage(0) which gives
    // as guarantee that it is a legacy account layout set with
    // proposal_cool_off_time = 0
    //
    // Note: All the settings below are one time config migration from program V1 &
    // V2 account data to V3
    if governance_data.config.council_vote_threshold == VoteThreshold::YesVotePercentage(0) {
        // Set council_vote_threshold to community_vote_threshold which was used for
        // both council and community thresholds before
        governance_data.config.council_vote_threshold =
            governance_data.config.community_vote_threshold.clone();

        // The assumption here is that council should have Veto vote enabled by default
        // and equal to council_vote_threshold
        governance_data.config.council_veto_vote_threshold =
            governance_data.config.council_vote_threshold.clone();

        // For legacy accounts default Council VoteTipping to the Community
        governance_data.config.council_vote_tipping =
            governance_data.config.community_vote_tipping.clone();

        // For legacy accounts set the community Veto threshold to Disabled
        governance_data.config.community_veto_vote_threshold = VoteThreshold::Disabled;

        // Reset voting_cool_off_time and deposit_exempt_proposal_count  previously used
        // for voting_proposal_count
        governance_data.config.voting_cool_off_time = 0;
        governance_data.config.deposit_exempt_proposal_count =
            DEFAULT_DEPOSIT_EXEMPT_PROPOSAL_COUNT;

        // Reset reserved space previously used for proposal_count
        governance_data.reserved1 = 0;
    }

    Ok(governance_data)
}

/// Deserializes Governance account, checks owner program and asserts governance
/// belongs to the given ream
pub fn get_governance_data_for_realm(
    program_id: &Pubkey,
    governance_info: &AccountInfo,
    realm: &Pubkey,
) -> Result<GovernanceV2, ProgramError> {
    let governance_data = get_governance_data(program_id, governance_info)?;

    if governance_data.realm != *realm {
        return Err(GovernanceError::InvalidRealmForGovernance.into());
    }

    Ok(governance_data)
}

/// Checks the given account is a governance account and belongs to the given
/// realm
pub fn assert_governance_for_realm(
    program_id: &Pubkey,
    governance_info: &AccountInfo,
    realm: &Pubkey,
) -> Result<(), ProgramError> {
    get_governance_data_for_realm(program_id, governance_info, realm)?;
    Ok(())
}

/// Returns legacy ProgramGovernance PDA seeds
pub fn get_program_governance_address_seeds<'a>(
    realm: &'a Pubkey,
    governed_program: &'a Pubkey,
) -> [&'a [u8]; 3] {
    // 'program-governance' prefix ensures uniqueness of the PDA
    // Note: Only the current program upgrade authority can create an account with
    // this PDA using CreateProgramGovernance instruction
    [
        b"program-governance",
        realm.as_ref(),
        governed_program.as_ref(),
    ]
}

/// Returns legacy ProgramGovernance PDA address
pub fn get_program_governance_address<'a>(
    program_id: &Pubkey,
    realm: &'a Pubkey,
    governed_program: &'a Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &get_program_governance_address_seeds(realm, governed_program),
        program_id,
    )
    .0
}

/// Returns legacy MintGovernance PDA seeds
pub fn get_mint_governance_address_seeds<'a>(
    realm: &'a Pubkey,
    governed_mint: &'a Pubkey,
) -> [&'a [u8]; 3] {
    // 'mint-governance' prefix ensures uniqueness of the PDA
    // Note: Only the current mint authority can create an account with this PDA
    // using CreateMintGovernance instruction
    [b"mint-governance", realm.as_ref(), governed_mint.as_ref()]
}

/// Returns legacy MintGovernance PDA address
pub fn get_mint_governance_address<'a>(
    program_id: &Pubkey,
    realm: &'a Pubkey,
    governed_mint: &'a Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &get_mint_governance_address_seeds(realm, governed_mint),
        program_id,
    )
    .0
}

/// Returns legacy TokenGovernance PDA seeds
pub fn get_token_governance_address_seeds<'a>(
    realm: &'a Pubkey,
    governed_token_account: &'a Pubkey,
) -> [&'a [u8]; 3] {
    // 'token-governance' prefix ensures uniqueness of the PDA
    // Note: Only the current token account owner can create an account with this
    // PDA using CreateTokenGovernance instruction
    [
        b"token-governance",
        realm.as_ref(),
        governed_token_account.as_ref(),
    ]
}

/// Returns legacy TokenGovernance PDA address
pub fn get_token_governance_address<'a>(
    program_id: &Pubkey,
    realm: &'a Pubkey,
    governed_token_account: &'a Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &get_token_governance_address_seeds(realm, governed_token_account),
        program_id,
    )
    .0
}

/// Returns Governance PDA seeds
pub fn get_governance_address_seeds<'a>(
    realm: &'a Pubkey,
    governance_seed: &'a Pubkey,
) -> [&'a [u8]; 3] {
    [
        b"account-governance",
        realm.as_ref(),
        governance_seed.as_ref(),
    ]
}

/// Returns Governance PDA address
pub fn get_governance_address<'a>(
    program_id: &Pubkey,
    realm: &'a Pubkey,
    governance_seed: &'a Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &get_governance_address_seeds(realm, governance_seed),
        program_id,
    )
    .0
}

/// Checks whether the Governance account exists, is initialized and owned by
/// the Governance program
pub fn assert_is_valid_governance(
    program_id: &Pubkey,
    governance_info: &AccountInfo,
) -> Result<(), ProgramError> {
    assert_is_valid_account_of_types(program_id, governance_info, is_governance_account_type)
}

/// Validates args supplied to create governance account
pub fn assert_valid_create_governance_args(
    program_id: &Pubkey,
    governance_config: &GovernanceConfig,
    realm_info: &AccountInfo,
) -> Result<(), ProgramError> {
    assert_is_valid_realm(program_id, realm_info)?;

    assert_is_valid_governance_config(governance_config)?;

    Ok(())
}

/// Validates governance config parameters
pub fn assert_is_valid_governance_config(
    governance_config: &GovernanceConfig,
) -> Result<(), ProgramError> {
    assert_is_valid_vote_threshold(&governance_config.community_vote_threshold)?;
    assert_is_valid_vote_threshold(&governance_config.community_veto_vote_threshold)?;

    assert_is_valid_vote_threshold(&governance_config.council_vote_threshold)?;
    assert_is_valid_vote_threshold(&governance_config.council_veto_vote_threshold)?;

    // Setting both thresholds to Disabled is not allowed, however we might
    // reconsider it as a way to disable Governance permanently
    if governance_config.community_vote_threshold == VoteThreshold::Disabled
        && governance_config.council_vote_threshold == VoteThreshold::Disabled
    {
        return Err(GovernanceError::AtLeastOneVoteThresholdRequired.into());
    }

    // Make u8::MAX invalid value in case we would like to use the magic number as
    // Disabled value in the future
    if governance_config.deposit_exempt_proposal_count == u8::MAX {
        return Err(GovernanceError::InvalidDepositExemptProposalCount.into());
    }

    Ok(())
}

/// Asserts the provided vote_threshold is valid
pub fn assert_is_valid_vote_threshold(vote_threshold: &VoteThreshold) -> Result<(), ProgramError> {
    match *vote_threshold {
        VoteThreshold::YesVotePercentage(yes_vote_threshold_percentage) => {
            if !(1..=100).contains(&yes_vote_threshold_percentage) {
                return Err(GovernanceError::InvalidVoteThresholdPercentage.into());
            }
        }
        VoteThreshold::QuorumPercentage(_) => {
            return Err(GovernanceError::VoteThresholdTypeNotSupported.into());
        }
        VoteThreshold::Disabled => {}
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use {super::*, solana_program::clock::Epoch};

    fn create_test_governance_config() -> GovernanceConfig {
        GovernanceConfig {
            community_vote_threshold: VoteThreshold::YesVotePercentage(60),
            min_community_weight_to_create_proposal: 5,
            transactions_hold_up_time: 10,
            voting_base_time: 5,
            community_vote_tipping: VoteTipping::Strict,
            council_vote_threshold: VoteThreshold::YesVotePercentage(60),
            council_veto_vote_threshold: VoteThreshold::YesVotePercentage(50),
            min_council_weight_to_create_proposal: 1,
            council_vote_tipping: VoteTipping::Strict,
            community_veto_vote_threshold: VoteThreshold::YesVotePercentage(40),
            voting_cool_off_time: 2,
            deposit_exempt_proposal_count: 0,
        }
    }

    fn create_test_governance() -> GovernanceV2 {
        GovernanceV2 {
            account_type: GovernanceAccountType::GovernanceV2,
            realm: Pubkey::new_unique(),
            governance_seed: Pubkey::new_unique(),
            reserved1: 0,
            config: create_test_governance_config(),
            reserved_v2: Reserved119::default(),
            active_proposal_count: 10,
            required_signatories_count: 0,
        }
    }

    fn create_test_v1_governance() -> GovernanceV1 {
        GovernanceV1 {
            account_type: GovernanceAccountType::GovernanceV1,
            realm: Pubkey::new_unique(),
            governance_seed: Pubkey::new_unique(),
            proposals_count: 10,
            config: create_test_governance_config(),
        }
    }

    #[test]
    fn test_max_governance_size() {
        // Arrange
        let governance_data = create_test_governance();

        // Act
        let size = borsh::to_vec(&governance_data).unwrap().len();

        // Assert
        assert_eq!(governance_data.get_max_size(), Some(size));
    }

    #[test]
    fn test_v1_governance_size() {
        // Arrange
        let governance = create_test_v1_governance();

        // Act
        let size = borsh::to_vec(&governance).unwrap().len();

        // Assert
        assert_eq!(108, size);
    }

    #[test]
    fn test_deserialize_legacy_governance_account_without_council_vote_thresholds() {
        // Arrange

        // Legacy GovernanceV2 with
        // 1) config.community_vote_threshold = YesVotePercentage(10)
        // 2) config.proposal_cool_off_time = 0
        let mut account_data = [
            18, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 10, 10, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 100, 0,
            0, 0, 1, //
            // proposal_cool_off_time:
            0, 0, 0, 0, // ---------
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let program_id = Pubkey::new_unique();

        let info_key = Pubkey::new_unique();
        let mut lamports = 10u64;

        let governance_info = AccountInfo::new(
            &info_key,
            false,
            false,
            &mut lamports,
            &mut account_data[..],
            &program_id,
            false,
            Epoch::default(),
        );

        // Act
        let governance = get_governance_data(&program_id, &governance_info).unwrap();

        // Assert
        assert_eq!(
            governance.config.community_vote_threshold,
            governance.config.council_vote_threshold
        );

        assert_eq!(
            governance.config.council_vote_threshold,
            governance.config.council_veto_vote_threshold
        );

        assert_eq!(
            governance.config.council_vote_tipping,
            governance.config.community_vote_tipping
        );

        assert_eq!(
            governance.config.deposit_exempt_proposal_count,
            DEFAULT_DEPOSIT_EXEMPT_PROPOSAL_COUNT
        );
        assert_eq!(governance.config.voting_cool_off_time, 0);
    }

    #[test]
    fn test_assert_config_invalid_with_council_zero_yes_vote_threshold() {
        // Arrange
        let mut governance_config = create_test_governance_config();
        governance_config.council_vote_threshold = VoteThreshold::YesVotePercentage(0);

        // Act
        let err = assert_is_valid_governance_config(&governance_config)
            .err()
            .unwrap();

        // Assert
        assert_eq!(err, GovernanceError::InvalidVoteThresholdPercentage.into());
    }

    #[test]
    fn test_migrate_governance_config_from_legacy_data_to_program_v3() {
        // Arrange
        let mut governance_legacy_data = create_test_governance();

        governance_legacy_data.config.community_vote_threshold =
            VoteThreshold::YesVotePercentage(60);

        // council_vote_threshold == YesVotePercentage(0) indicates legacy account from
        // V1 & V2 program versions
        governance_legacy_data.config.council_vote_threshold = VoteThreshold::YesVotePercentage(0);

        governance_legacy_data.config.council_veto_vote_threshold =
            VoteThreshold::YesVotePercentage(0);
        governance_legacy_data.config.council_vote_tipping = VoteTipping::Disabled;
        governance_legacy_data.config.community_veto_vote_threshold =
            VoteThreshold::YesVotePercentage(0);
        governance_legacy_data.config.voting_cool_off_time = 1;
        governance_legacy_data.config.voting_base_time = 36000;

        let mut legacy_data = vec![];
        governance_legacy_data.serialize(&mut legacy_data).unwrap();

        let program_id = Pubkey::new_unique();

        let info_key = Pubkey::new_unique();
        let mut lamports = 10u64;

        let legacy_account_info = AccountInfo::new(
            &info_key,
            false,
            false,
            &mut lamports,
            &mut legacy_data[..],
            &program_id,
            false,
            Epoch::default(),
        );
        // Act
        let governance_program_v3 = get_governance_data(&program_id, &legacy_account_info).unwrap();

        // Assert
        assert_eq!(
            governance_program_v3.config.council_vote_threshold,
            VoteThreshold::YesVotePercentage(60)
        );

        assert_eq!(
            governance_program_v3.config.council_veto_vote_threshold,
            VoteThreshold::YesVotePercentage(60)
        );

        assert_eq!(
            governance_program_v3.config.community_veto_vote_threshold,
            VoteThreshold::Disabled
        );

        assert_eq!(
            governance_program_v3.config.council_vote_tipping,
            VoteTipping::Strict
        );

        assert_eq!(governance_program_v3.config.voting_cool_off_time, 0);

        assert_eq!(
            governance_program_v3.config.deposit_exempt_proposal_count,
            DEFAULT_DEPOSIT_EXEMPT_PROPOSAL_COUNT
        );
    }

    #[test]
    fn test_assert_config_invalid_with_community_zero_yes_vote_threshold() {
        // Arrange
        let mut governance_config = create_test_governance_config();
        governance_config.community_vote_threshold = VoteThreshold::YesVotePercentage(0);

        // Act
        let err = assert_is_valid_governance_config(&governance_config)
            .err()
            .unwrap();

        // Assert
        assert_eq!(err, GovernanceError::InvalidVoteThresholdPercentage.into());
    }

    #[test]
    fn test_assert_config_invalid_with_all_vote_thresholds_disabled() {
        // Arrange
        let mut governance_config = create_test_governance_config();
        governance_config.community_vote_threshold = VoteThreshold::Disabled;
        governance_config.council_vote_threshold = VoteThreshold::Disabled;

        // Act
        let err = assert_is_valid_governance_config(&governance_config)
            .err()
            .unwrap();

        // Assert
        assert_eq!(err, GovernanceError::AtLeastOneVoteThresholdRequired.into());
    }

    #[test]
    fn test_assert_config_invalid_with_council_zero_yes_veto_vote_threshold() {
        // Arrange
        let mut governance_config = create_test_governance_config();
        governance_config.council_veto_vote_threshold = VoteThreshold::YesVotePercentage(0);

        // Act
        let err = assert_is_valid_governance_config(&governance_config)
            .err()
            .unwrap();

        // Assert
        assert_eq!(err, GovernanceError::InvalidVoteThresholdPercentage.into());
    }

    #[test]
    fn test_assert_config_invalid_with_community_zero_yes_veto_vote_threshold() {
        // Arrange
        let mut governance_config = create_test_governance_config();
        governance_config.community_veto_vote_threshold = VoteThreshold::YesVotePercentage(0);

        // Act
        let err = assert_is_valid_governance_config(&governance_config)
            .err()
            .unwrap();

        // Assert
        assert_eq!(err, GovernanceError::InvalidVoteThresholdPercentage.into());
    }

    #[test]
    fn test_get_proposal_deposit_amount_for_exempt_proposal() {
        // Arrange
        let mut governance_data = create_test_governance();

        governance_data.active_proposal_count = 10;
        governance_data.config.deposit_exempt_proposal_count = 10;

        // Act
        let deposit_amount = governance_data.get_proposal_deposit_amount();

        // Assert
        assert_eq!(deposit_amount, 0);
    }

    #[test]
    fn test_get_proposal_deposit_amount_for_non_exempt_proposal() {
        // Arrange
        let mut governance_data = create_test_governance();

        governance_data.active_proposal_count = 100;
        governance_data.config.deposit_exempt_proposal_count = 10;

        // Act
        let deposit_amount = governance_data.get_proposal_deposit_amount();

        // Assert
        assert_eq!(deposit_amount, SECURITY_DEPOSIT_BASE_LAMPORTS * 90);
    }

    #[test]
    fn test_get_proposal_deposit_amount_without_exempt_proposal() {
        // Arrange
        let mut governance_data = create_test_governance();

        governance_data.active_proposal_count = 10;
        governance_data.config.deposit_exempt_proposal_count = 0;

        // Act
        let deposit_amount = governance_data.get_proposal_deposit_amount();

        // Assert
        assert_eq!(deposit_amount, SECURITY_DEPOSIT_BASE_LAMPORTS * 10);
    }

    #[test]
    fn test_assert_config_invalid_with_max_deposit_exempt_proposal_count() {
        // Arrange
        let mut governance_config = create_test_governance_config();
        governance_config.deposit_exempt_proposal_count = u8::MAX;

        // Act
        let err = assert_is_valid_governance_config(&governance_config)
            .err()
            .unwrap();

        // Assert
        assert_eq!(
            err,
            GovernanceError::InvalidDepositExemptProposalCount.into()
        );
    }
}
