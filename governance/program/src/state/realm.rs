//! Realm Account

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, program_pack::IsInitialized,
    pubkey::Pubkey,
};
use spl_governance_tools::account::{assert_is_valid_account, get_account_data, AccountMaxSize};

use crate::{
    error::GovernanceError,
    state::enums::{GovernanceAccountType, MintMaxVoteWeightSource},
    PROGRAM_AUTHORITY_SEED,
};

/// Realm Config instruction args
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct RealmConfigArgs {
    /// Indicates whether council_mint should be used
    /// If yes then council_mint account must also be passed to the instruction
    pub use_council_mint: bool,

    /// Min number of community tokens required to create a governance
    pub min_community_tokens_to_create_governance: u64,

    /// The source used for community mint max vote weight source
    pub community_mint_max_vote_weight_source: MintMaxVoteWeightSource,

    /// Indicates whether an external addin program should be used to provide community voters weights
    /// If yes then the voters weight program account must be passed to the instruction
    pub use_community_voter_weight_addin: bool,
}

/// Realm Config defining Realm parameters.
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct RealmConfig {
    /// Indicates whether an external addin program should be used to provide voters weights for the community mint
    pub use_community_voter_weight_addin: bool,

    /// Reserved space for future versions
    pub reserved: [u8; 7],

    /// Min number of community tokens required to create a governance
    pub min_community_tokens_to_create_governance: u64,

    /// The source used for community mint max vote weight source
    pub community_mint_max_vote_weight_source: MintMaxVoteWeightSource,

    /// Optional council mint
    pub council_mint: Option<Pubkey>,
}

/// Governance Realm Account
/// Account PDA seeds" ['governance', name]
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct Realm {
    /// Governance account type
    pub account_type: GovernanceAccountType,

    /// Community mint
    pub community_mint: Pubkey,

    /// Configuration of the Realm
    pub config: RealmConfig,

    /// Reserved space for future versions
    pub reserved: [u8; 8],

    /// Realm authority. The authority must sign transactions which update the realm config
    /// The authority should be transferred to Realm Governance to make the Realm self governed through proposals
    pub authority: Option<Pubkey>,

    /// Governance Realm name
    pub name: String,
}

impl AccountMaxSize for Realm {
    fn get_max_size(&self) -> Option<usize> {
        Some(self.name.len() + 136)
    }
}

impl IsInitialized for Realm {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::Realm
    }
}

impl Realm {
    /// Asserts the given mint is either Community or Council mint of the Realm
    pub fn assert_is_valid_governing_token_mint(
        &self,
        governing_token_mint: &Pubkey,
    ) -> Result<(), ProgramError> {
        if self.community_mint == *governing_token_mint {
            return Ok(());
        }

        if self.config.council_mint == Some(*governing_token_mint) {
            return Ok(());
        }

        Err(GovernanceError::InvalidGoverningTokenMint.into())
    }

    /// Asserts the given governing token mint and holding accounts are valid for the realm
    pub fn assert_is_valid_governing_token_mint_and_holding(
        &self,
        program_id: &Pubkey,
        realm: &Pubkey,
        governing_token_mint: &Pubkey,
        governing_token_holding: &Pubkey,
    ) -> Result<(), ProgramError> {
        self.assert_is_valid_governing_token_mint(governing_token_mint)?;

        let governing_token_holding_address =
            get_governing_token_holding_address(program_id, realm, governing_token_mint);

        if governing_token_holding_address != *governing_token_holding {
            return Err(GovernanceError::InvalidGoverningTokenHoldingAccount.into());
        }

        Ok(())
    }

    /// Asserts the given governing token can be deposited into the realm
    pub fn asset_governing_tokens_deposits_allowed(
        &self,
        governing_token_mint: &Pubkey,
    ) -> Result<(), ProgramError> {
        // If the deposit is for the community token and the realm uses community voter weight addin then panic
        if self.config.use_community_voter_weight_addin
            && self.community_mint == *governing_token_mint
        {
            return Err(GovernanceError::GoverningTokenDepositsNotAllowed.into());
        }

        Ok(())
    }
}

/// Checks whether realm account exists, is initialized and  owned by Governance program
pub fn assert_is_valid_realm(
    program_id: &Pubkey,
    realm_info: &AccountInfo,
) -> Result<(), ProgramError> {
    assert_is_valid_account(realm_info, GovernanceAccountType::Realm, program_id)
}

/// Deserializes account and checks owner program
pub fn get_realm_data(
    program_id: &Pubkey,
    realm_info: &AccountInfo,
) -> Result<Realm, ProgramError> {
    get_account_data::<Realm>(program_id, realm_info)
}

/// Deserializes account and checks the given authority is Realm's authority
pub fn get_realm_data_for_authority(
    program_id: &Pubkey,
    realm_info: &AccountInfo,
    realm_authority: &Pubkey,
) -> Result<Realm, ProgramError> {
    let realm_data = get_account_data::<Realm>(program_id, realm_info)?;

    if realm_data.authority.is_none() {
        return Err(GovernanceError::RealmHasNoAuthority.into());
    }

    if realm_data.authority.unwrap() != *realm_authority {
        return Err(GovernanceError::InvalidAuthorityForRealm.into());
    }

    Ok(realm_data)
}

/// Deserializes Ream account and asserts the given governing_token_mint is either Community or Council mint of the Realm
pub fn get_realm_data_for_governing_token_mint(
    program_id: &Pubkey,
    realm_info: &AccountInfo,
    governing_token_mint: &Pubkey,
) -> Result<Realm, ProgramError> {
    let realm_data = get_realm_data(program_id, realm_info)?;

    realm_data.assert_is_valid_governing_token_mint(governing_token_mint)?;

    Ok(realm_data)
}

/// Returns Realm PDA seeds
pub fn get_realm_address_seeds(name: &str) -> [&[u8]; 2] {
    [PROGRAM_AUTHORITY_SEED, name.as_bytes()]
}

/// Returns Realm PDA address
pub fn get_realm_address(program_id: &Pubkey, name: &str) -> Pubkey {
    Pubkey::find_program_address(&get_realm_address_seeds(name), program_id).0
}

/// Returns Realm Token Holding PDA seeds
pub fn get_governing_token_holding_address_seeds<'a>(
    realm: &'a Pubkey,
    governing_token_mint: &'a Pubkey,
) -> [&'a [u8]; 3] {
    [
        PROGRAM_AUTHORITY_SEED,
        realm.as_ref(),
        governing_token_mint.as_ref(),
    ]
}

/// Returns Realm Token Holding PDA address
pub fn get_governing_token_holding_address(
    program_id: &Pubkey,
    realm: &Pubkey,
    governing_token_mint: &Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &get_governing_token_holding_address_seeds(realm, governing_token_mint),
        program_id,
    )
    .0
}

/// Asserts given realm config args are correct
pub fn assert_valid_realm_config_args(config_args: &RealmConfigArgs) -> Result<(), ProgramError> {
    match config_args.community_mint_max_vote_weight_source {
        MintMaxVoteWeightSource::SupplyFraction(fraction) => {
            if !(1..=MintMaxVoteWeightSource::SUPPLY_FRACTION_BASE).contains(&fraction) {
                return Err(GovernanceError::InvalidMaxVoteWeightSupplyFraction.into());
            }
        }
        MintMaxVoteWeightSource::Absolute(_) => {
            return Err(GovernanceError::MintMaxVoteWeightSourceNotSupported.into())
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {

    use crate::{
        instruction::GovernanceInstruction,
        state::legacy::{GovernanceInstructionV1, RealmConfigV1, RealmV1},
    };
    use solana_program::borsh::try_from_slice_unchecked;

    use super::*;

    #[test]
    fn test_max_size() {
        let realm = Realm {
            account_type: GovernanceAccountType::Realm,
            community_mint: Pubkey::new_unique(),
            reserved: [0; 8],

            authority: Some(Pubkey::new_unique()),
            name: "test-realm".to_string(),
            config: RealmConfig {
                council_mint: Some(Pubkey::new_unique()),
                use_community_voter_weight_addin: false,
                reserved: [0; 7],

                community_mint_max_vote_weight_source: MintMaxVoteWeightSource::Absolute(100),
                min_community_tokens_to_create_governance: 10,
            },
        };

        let size = realm.try_to_vec().unwrap().len();

        assert_eq!(realm.get_max_size(), Some(size));
    }

    #[test]
    fn test_deserialize_v2_realm_account_from_v1() {
        // Arrange
        let realm_v1 = RealmV1 {
            account_type: GovernanceAccountType::Realm,
            community_mint: Pubkey::new_unique(),
            config: RealmConfigV1 {
                council_mint: Some(Pubkey::new_unique()),
                reserved: [0; 8],
                community_mint_max_vote_weight_source: MintMaxVoteWeightSource::Absolute(100),
                min_community_tokens_to_create_governance: 10,
            },
            reserved: [0; 8],
            authority: Some(Pubkey::new_unique()),
            name: "test-realm-v1".to_string(),
        };

        let mut realm_v1_data = vec![];
        realm_v1.serialize(&mut realm_v1_data).unwrap();

        // Act
        let realm_v2: Realm = try_from_slice_unchecked(&realm_v1_data).unwrap();

        // Assert
        assert!(!realm_v2.config.use_community_voter_weight_addin);
        assert_eq!(realm_v2.account_type, GovernanceAccountType::Realm);
        assert_eq!(
            realm_v2.config.min_community_tokens_to_create_governance,
            realm_v1.config.min_community_tokens_to_create_governance,
        );
    }

    #[test]
    fn test_deserialize_v1_create_realm_instruction_from_v2() {
        // Arrange
        let create_realm_ix_v2 = GovernanceInstruction::CreateRealm {
            name: "test-realm".to_string(),
            config_args: RealmConfigArgs {
                use_council_mint: true,
                min_community_tokens_to_create_governance: 100,
                community_mint_max_vote_weight_source:
                    MintMaxVoteWeightSource::FULL_SUPPLY_FRACTION,
                use_community_voter_weight_addin: false,
            },
        };

        let mut create_realm_ix_data = vec![];
        create_realm_ix_v2
            .serialize(&mut create_realm_ix_data)
            .unwrap();

        // Act
        let create_realm_ix_v1: GovernanceInstructionV1 =
            try_from_slice_unchecked(&create_realm_ix_data).unwrap();

        // Assert
        if let GovernanceInstructionV1::CreateRealm { name, config_args } = create_realm_ix_v1 {
            assert_eq!("test-realm", name);
            assert_eq!(
                MintMaxVoteWeightSource::FULL_SUPPLY_FRACTION,
                config_args.community_mint_max_vote_weight_source
            );
        } else {
            panic!("Can't deserialize v1 CreateRealm instruction from v2");
        }
    }
}
