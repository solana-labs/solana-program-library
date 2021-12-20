//! Solana Vault

use {
    crate::{pack::*, string::ArrayString64, traits::*},
    arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs},
    num_enum::TryFromPrimitive,
    serde::{Deserialize, Serialize},
    serde_json::to_string,
    solana_program::{clock::UnixTimestamp, program_error::ProgramError, pubkey::Pubkey},
};

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum VaultType {
    AmmStake,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
pub enum VaultStrategy {
    StakeLpCompoundRewards {
        #[serde(
            deserialize_with = "pubkey_deserialize",
            serialize_with = "pubkey_serialize"
        )]
        pool_id_ref: Pubkey,
        #[serde(
            deserialize_with = "pubkey_deserialize",
            serialize_with = "pubkey_serialize"
        )]
        farm_id_ref: Pubkey,
        #[serde(
            deserialize_with = "pubkey_deserialize",
            serialize_with = "pubkey_serialize"
        )]
        lp_token_custody: Pubkey,
        #[serde(
            deserialize_with = "pubkey_deserialize",
            serialize_with = "pubkey_serialize"
        )]
        token_a_custody: Pubkey,
        #[serde(
            deserialize_with = "optional_pubkey_deserialize",
            serialize_with = "optional_pubkey_serialize"
        )]
        token_b_custody: Option<Pubkey>,
        #[serde(
            deserialize_with = "pubkey_deserialize",
            serialize_with = "pubkey_serialize"
        )]
        token_a_reward_custody: Pubkey,
        #[serde(
            deserialize_with = "optional_pubkey_deserialize",
            serialize_with = "optional_pubkey_serialize"
        )]
        token_b_reward_custody: Option<Pubkey>,
        #[serde(
            deserialize_with = "pubkey_deserialize",
            serialize_with = "pubkey_serialize"
        )]
        vault_stake_info: Pubkey,
    },
    DynamicHedge,
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum VaultStrategyType {
    StakeLpCompoundRewards,
    DynamicHedge,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
pub struct Vault {
    #[serde(
        serialize_with = "as64_serialize",
        deserialize_with = "as64_deserialize"
    )]
    pub name: ArrayString64,
    pub version: u16,
    pub vault_type: VaultType,
    pub official: bool,
    #[serde(skip_serializing, skip_deserializing)]
    pub refdb_index: Option<u32>,
    #[serde(skip_serializing, skip_deserializing)]
    pub refdb_counter: u16,
    pub metadata_bump: u8,
    pub authority_bump: u8,
    pub vault_token_bump: u8,
    pub lock_required: bool,
    pub unlock_required: bool,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub vault_program_id: Pubkey,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub vault_authority: Pubkey,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub vault_token_ref: Pubkey,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub info_account: Pubkey,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub admin_account: Pubkey,
    #[serde(
        deserialize_with = "optional_pubkey_deserialize",
        serialize_with = "optional_pubkey_serialize"
    )]
    pub fees_account_a: Option<Pubkey>,
    #[serde(
        deserialize_with = "optional_pubkey_deserialize",
        serialize_with = "optional_pubkey_serialize"
    )]
    pub fees_account_b: Option<Pubkey>,
    pub strategy: VaultStrategy,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UserInfo {
    pub last_deposit_time: UnixTimestamp,
    pub last_withdrawal_time: UnixTimestamp,
    pub tokens_a_added: u64,
    pub tokens_b_added: u64,
    pub tokens_a_removed: u64,
    pub tokens_b_removed: u64,
    pub lp_tokens_debt: u64,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
pub struct VaultInfo {
    pub crank_time: UnixTimestamp,
    pub crank_step: u64,
    pub tokens_a_added: u64,
    pub tokens_b_added: u64,
    pub tokens_a_removed: u64,
    pub tokens_b_removed: u64,
    pub tokens_a_rewards: u64,
    pub tokens_b_rewards: u64,
    pub stake_balance: f64,
    pub deposit_allowed: bool,
    pub withdrawal_allowed: bool,
    pub min_crank_interval: u64,
    pub fee: f64,
    pub external_fee: f64,
}

impl Named for Vault {
    fn name(&self) -> ArrayString64 {
        self.name
    }
}

impl Versioned for Vault {
    fn version(&self) -> u16 {
        self.version
    }
}

impl Vault {
    pub const MAX_LEN: usize = 565;
    pub const STAKE_LP_COMPOUND_REWARDS_LEN: usize = 565;
    pub const DYNAMIC_HEDGE_LEN: usize = 1;

    pub fn get_size(&self) -> usize {
        match self.strategy {
            VaultStrategy::StakeLpCompoundRewards { .. } => Vault::STAKE_LP_COMPOUND_REWARDS_LEN,
            VaultStrategy::DynamicHedge { .. } => Vault::DYNAMIC_HEDGE_LEN,
        }
    }

    pub fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        match self.strategy {
            VaultStrategy::StakeLpCompoundRewards { .. } => {
                self.pack_stake_lp_compound_rewards(output)
            }
            VaultStrategy::DynamicHedge { .. } => Err(ProgramError::UnsupportedSysvar),
        }
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; Vault::MAX_LEN] = [0; Vault::MAX_LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidAccountData)
        }
    }

    pub fn unpack(input: &[u8]) -> Result<Vault, ProgramError> {
        check_data_len(input, 1)?;
        let strategy_type = VaultStrategyType::try_from_primitive(input[0])
            .or(Err(ProgramError::InvalidAccountData))?;
        match strategy_type {
            VaultStrategyType::StakeLpCompoundRewards => {
                Vault::unpack_stake_lp_compound_rewards(input)
            }
            VaultStrategyType::DynamicHedge { .. } => Err(ProgramError::UnsupportedSysvar),
        }
    }

    fn pack_stake_lp_compound_rewards(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, Vault::STAKE_LP_COMPOUND_REWARDS_LEN)?;

        if let VaultStrategy::StakeLpCompoundRewards {
            pool_id_ref,
            farm_id_ref,
            lp_token_custody,
            token_a_custody,
            token_b_custody,
            token_a_reward_custody,
            token_b_reward_custody,
            vault_stake_info,
        } = self.strategy
        {
            let output = array_mut_ref![output, 0, Vault::STAKE_LP_COMPOUND_REWARDS_LEN];

            let (
                strategy_type_out,
                name_out,
                version_out,
                vault_type_out,
                official_out,
                refdb_index_out,
                refdb_counter_out,
                metadata_bump_out,
                authority_bump_out,
                vault_token_bump_out,
                lock_required_out,
                unlock_required_out,
                vault_program_id_out,
                vault_authority_out,
                vault_token_ref_out,
                vault_info_account_out,
                admin_account_out,
                fees_account_a_out,
                fees_account_b_out,
                pool_id_ref_out,
                farm_id_ref_out,
                lp_token_custody_out,
                token_a_custody_out,
                token_b_custody_out,
                token_a_reward_custody_out,
                token_b_reward_custody_out,
                vault_stake_info_out,
            ) = mut_array_refs![
                output, 1, 64, 2, 1, 1, 5, 2, 1, 1, 1, 1, 1, 32, 32, 32, 32, 32, 33, 33, 32, 32,
                32, 32, 33, 32, 33, 32
            ];

            strategy_type_out[0] = VaultStrategyType::StakeLpCompoundRewards as u8;

            pack_array_string64(&self.name, name_out);
            *version_out = self.version.to_le_bytes();
            vault_type_out[0] = self.vault_type as u8;
            official_out[0] = self.official as u8;
            pack_option_u32(self.refdb_index, refdb_index_out);
            *refdb_counter_out = self.refdb_counter.to_le_bytes();
            metadata_bump_out[0] = self.metadata_bump as u8;
            authority_bump_out[0] = self.authority_bump as u8;
            vault_token_bump_out[0] = self.vault_token_bump as u8;
            lock_required_out[0] = self.lock_required as u8;
            unlock_required_out[0] = self.unlock_required as u8;
            vault_program_id_out.copy_from_slice(self.vault_program_id.as_ref());
            vault_authority_out.copy_from_slice(self.vault_authority.as_ref());
            vault_token_ref_out.copy_from_slice(self.vault_token_ref.as_ref());
            vault_info_account_out.copy_from_slice(self.info_account.as_ref());
            admin_account_out.copy_from_slice(self.admin_account.as_ref());
            pack_option_key(&self.fees_account_a, fees_account_a_out);
            pack_option_key(&self.fees_account_b, fees_account_b_out);
            pool_id_ref_out.copy_from_slice(pool_id_ref.as_ref());
            farm_id_ref_out.copy_from_slice(farm_id_ref.as_ref());
            lp_token_custody_out.copy_from_slice(lp_token_custody.as_ref());
            token_a_custody_out.copy_from_slice(token_a_custody.as_ref());
            pack_option_key(&token_b_custody, token_b_custody_out);
            token_a_reward_custody_out.copy_from_slice(token_a_reward_custody.as_ref());
            pack_option_key(&token_b_reward_custody, token_b_reward_custody_out);
            vault_stake_info_out.copy_from_slice(vault_stake_info.as_ref());

            Ok(Vault::STAKE_LP_COMPOUND_REWARDS_LEN)
        } else {
            Err(ProgramError::InvalidAccountData)
        }
    }

    fn unpack_stake_lp_compound_rewards(input: &[u8]) -> Result<Vault, ProgramError> {
        check_data_len(input, Vault::STAKE_LP_COMPOUND_REWARDS_LEN)?;

        let input = array_ref![input, 1, Vault::STAKE_LP_COMPOUND_REWARDS_LEN - 1];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            name,
            version,
            vault_type,
            official,
            refdb_index,
            refdb_counter,
            metadata_bump,
            authority_bump,
            vault_token_bump,
            lock_required,
            unlock_required,
            vault_program_id,
            vault_authority,
            vault_token_ref,
            info_account,
            admin_account,
            fees_account_a,
            fees_account_b,
            pool_id_ref,
            farm_id_ref,
            lp_token_custody,
            token_a_custody,
            token_b_custody,
            token_a_reward_custody,
            token_b_reward_custody,
            vault_stake_info,
        ) = array_refs![
            input, 64, 2, 1, 1, 5, 2, 1, 1, 1, 1, 1, 32, 32, 32, 32, 32, 33, 33, 32, 32, 32, 32,
            33, 32, 33, 32
        ];

        Ok(Self {
            name: unpack_array_string64(name)?,
            version: u16::from_le_bytes(*version),
            vault_type: VaultType::try_from_primitive(vault_type[0])
                .or(Err(ProgramError::InvalidAccountData))?,
            official: unpack_bool(official)?,
            refdb_index: unpack_option_u32(refdb_index)?,
            refdb_counter: u16::from_le_bytes(*refdb_counter),
            metadata_bump: metadata_bump[0],
            authority_bump: authority_bump[0],
            vault_token_bump: vault_token_bump[0],
            lock_required: unpack_bool(lock_required)?,
            unlock_required: unpack_bool(unlock_required)?,
            vault_program_id: Pubkey::new_from_array(*vault_program_id),
            vault_authority: Pubkey::new_from_array(*vault_authority),
            vault_token_ref: Pubkey::new_from_array(*vault_token_ref),
            info_account: Pubkey::new_from_array(*info_account),
            admin_account: Pubkey::new_from_array(*admin_account),
            fees_account_a: unpack_option_key(fees_account_a)?,
            fees_account_b: unpack_option_key(fees_account_b)?,
            strategy: VaultStrategy::StakeLpCompoundRewards {
                pool_id_ref: Pubkey::new_from_array(*pool_id_ref),
                farm_id_ref: Pubkey::new_from_array(*farm_id_ref),
                lp_token_custody: Pubkey::new_from_array(*lp_token_custody),
                token_a_custody: Pubkey::new_from_array(*token_a_custody),
                token_b_custody: unpack_option_key(token_b_custody)?,
                token_a_reward_custody: Pubkey::new_from_array(*token_a_reward_custody),
                token_b_reward_custody: unpack_option_key(token_b_reward_custody)?,
                vault_stake_info: Pubkey::new_from_array(*vault_stake_info),
            },
        })
    }
}

impl std::fmt::Display for VaultStrategyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            VaultStrategyType::StakeLpCompoundRewards => write!(f, "StakeLpCompoundRewards"),
            VaultStrategyType::DynamicHedge => write!(f, "DynamicHedge"),
        }
    }
}

impl std::fmt::Display for VaultType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            VaultType::AmmStake => write!(f, "AmmStake"),
        }
    }
}

impl std::fmt::Display for Vault {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", to_string(&self).unwrap())
    }
}

impl std::fmt::Display for UserInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", to_string(&self).unwrap())
    }
}

impl std::fmt::Display for VaultInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", to_string(&self).unwrap())
    }
}
