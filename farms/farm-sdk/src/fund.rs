//! Solana Fund

use {
    crate::{pack::*, string::ArrayString64, traits::*},
    arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs},
    num_enum::TryFromPrimitive,
    serde::{Deserialize, Serialize},
    serde_json::to_string,
    solana_program::{clock::UnixTimestamp, program_error::ProgramError, pubkey::Pubkey},
};

pub const DISCRIMINATOR_FUND_CUSTODY: u64 = 15979585294446943865;
pub const DISCRIMINATOR_FUND_VAULT: u64 = 10084386823844633785;
pub const DISCRIMINATOR_FUND_USER_REQUESTS: u64 = 13706702285134686038;

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum FundType {
    General,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
pub struct Fund {
    #[serde(
        serialize_with = "as64_serialize",
        deserialize_with = "as64_deserialize"
    )]
    pub name: ArrayString64,
    pub version: u16,
    pub fund_type: FundType,
    pub official: bool,
    #[serde(skip_serializing, skip_deserializing)]
    pub refdb_index: Option<u32>,
    #[serde(skip_serializing, skip_deserializing)]
    pub refdb_counter: u16,
    pub metadata_bump: u8,
    pub authority_bump: u8,
    pub fund_token_bump: u8,
    pub multisig_bump: u8,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub fund_program_id: Pubkey,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub fund_authority: Pubkey,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub fund_manager: Pubkey,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub fund_token_ref: Pubkey,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub info_account: Pubkey,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub multisig_account: Pubkey,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub vaults_assets_info: Pubkey,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub custodies_assets_info: Pubkey,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub description_account: Pubkey,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FundUserAction {
    pub time: UnixTimestamp,
    pub amount: u64,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FundUserInfo {
    pub virtual_tokens_balance: u64,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FundUserRequests {
    pub discriminator: u64,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub fund_ref: Pubkey,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub token_ref: Pubkey,
    pub deposit_request: FundUserAction,
    pub last_deposit: FundUserAction,
    pub withdrawal_request: FundUserAction,
    pub last_withdrawal: FundUserAction,
    #[serde(
        serialize_with = "as64_serialize",
        deserialize_with = "as64_deserialize"
    )]
    pub deny_reason: ArrayString64,
    pub bump: u8,
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum FundAssetType {
    Vault,
    Custody,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct FundAssets {
    pub asset_type: FundAssetType,
    pub target_hash: u64,
    pub current_hash: u64,
    pub current_cycle: u64,
    pub current_assets_usd: f64,
    pub cycle_start_time: UnixTimestamp,
    pub cycle_end_time: UnixTimestamp,
    pub bump: u8,
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum FundCustodyType {
    DepositWithdraw,
    Trading,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
pub struct FundCustody {
    pub discriminator: u64,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub fund_ref: Pubkey,
    pub custody_id: u32,
    pub custody_type: FundCustodyType,
    pub is_vault_token: bool,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub token_ref: Pubkey,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub address: Pubkey,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub fees_address: Pubkey,
    pub bump: u8,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct FundCustodyWithBalance {
    pub discriminator: u64,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub fund_ref: Pubkey,
    #[serde(
        serialize_with = "as64_serialize",
        deserialize_with = "as64_deserialize"
    )]
    pub fund_name: ArrayString64,
    pub custody_id: u32,
    pub custody_type: FundCustodyType,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub token_ref: Pubkey,
    #[serde(
        serialize_with = "as64_serialize",
        deserialize_with = "as64_deserialize"
    )]
    pub token_name: ArrayString64,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub address: Pubkey,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub fees_address: Pubkey,
    pub balance: f64,
    pub fees_balance: f64,
    pub bump: u8,
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum FundVaultType {
    Pool,
    Farm,
    Vault,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
pub struct FundVault {
    pub discriminator: u64,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub fund_ref: Pubkey,
    pub vault_id: u32,
    pub vault_type: FundVaultType,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub vault_ref: Pubkey,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub router_program_id: Pubkey,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub underlying_pool_id: Pubkey,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub underlying_pool_ref: Pubkey,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub underlying_lp_token_mint: Pubkey,
    pub lp_balance: u64,
    pub balance_update_time: UnixTimestamp,
    pub bump: u8,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
pub struct FundSchedule {
    pub start_time: UnixTimestamp,
    pub end_time: UnixTimestamp,
    pub approval_required: bool,
    pub min_amount_usd: f64,
    pub max_amount_usd: f64,
    pub fee: f64,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
pub struct FundAssetsTrackingConfig {
    pub assets_limit_usd: f64,
    pub max_update_age_sec: u64,
    pub max_price_error: f64,
    pub max_price_age_sec: u64,
    pub issue_virtual_tokens: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
pub struct FundInfo {
    pub deposit_schedule: FundSchedule,
    pub withdrawal_schedule: FundSchedule,
    pub assets_config: FundAssetsTrackingConfig,
    pub virtual_tokens_supply: u64,
    pub amount_invested_usd: f64,
    pub amount_removed_usd: f64,
    pub current_assets_usd: f64,
    pub assets_update_time: UnixTimestamp,
    pub admin_action_time: UnixTimestamp,
    pub last_trade_time: UnixTimestamp,
    pub liquidation_start_time: UnixTimestamp,
    pub liquidation_amount_usd: f64,
    pub liquidation_amount_tokens: u64,
}

impl Named for Fund {
    fn name(&self) -> ArrayString64 {
        self.name
    }
}

impl Versioned for Fund {
    fn version(&self) -> u16 {
        self.version
    }
}

impl FundUserAction {
    pub const LEN: usize = 16;
}

impl Packed for FundUserAction {
    fn get_size(&self) -> usize {
        Self::LEN
    }

    fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, Self::LEN)?;

        let output = array_mut_ref![output, 0, FundUserAction::LEN];

        let (time_out, amount_out) = mut_array_refs![output, 8, 8];

        *time_out = self.time.to_le_bytes();
        *amount_out = self.amount.to_le_bytes();

        Ok(Self::LEN)
    }

    fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; Self::LEN] = [0; Self::LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidAccountData)
        }
    }

    fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        check_data_len(input, Self::LEN)?;

        let input = array_ref![input, 0, FundUserAction::LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (time, amount) = array_refs![input, 8, 8];

        Ok(Self {
            time: i64::from_le_bytes(*time),
            amount: u64::from_le_bytes(*amount),
        })
    }
}

impl FundUserRequests {
    pub const LEN: usize = 201;
}

impl Packed for FundUserRequests {
    fn get_size(&self) -> usize {
        Self::LEN
    }

    fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, Self::LEN)?;

        let output = array_mut_ref![output, 0, FundUserRequests::LEN];

        let (
            discriminator_out,
            fund_ref_out,
            token_ref_out,
            deposit_request_out,
            last_deposit_out,
            withdrawal_request_out,
            last_withdrawal_out,
            deny_reason_out,
            bump_out,
        ) = mut_array_refs![output, 8, 32, 32, 16, 16, 16, 16, 64, 1];

        *discriminator_out = self.discriminator.to_le_bytes();
        fund_ref_out.copy_from_slice(self.fund_ref.as_ref());
        token_ref_out.copy_from_slice(self.token_ref.as_ref());
        self.deposit_request.pack(deposit_request_out)?;
        self.last_deposit.pack(last_deposit_out)?;
        self.withdrawal_request.pack(withdrawal_request_out)?;
        self.last_withdrawal.pack(last_withdrawal_out)?;
        pack_array_string64(&self.deny_reason, deny_reason_out);
        bump_out[0] = self.bump;

        Ok(Self::LEN)
    }

    fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; Self::LEN] = [0; Self::LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidAccountData)
        }
    }

    fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        check_data_len(input, Self::LEN)?;

        let input = array_ref![input, 0, FundUserRequests::LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            discriminator,
            fund_ref,
            token_ref,
            deposit_request,
            last_deposit,
            withdrawal_request,
            last_withdrawal,
            deny_reason,
            bump,
        ) = array_refs![input, 8, 32, 32, 16, 16, 16, 16, 64, 1];

        Ok(Self {
            discriminator: u64::from_le_bytes(*discriminator),
            fund_ref: Pubkey::new_from_array(*fund_ref),
            token_ref: Pubkey::new_from_array(*token_ref),
            deposit_request: FundUserAction::unpack(deposit_request)?,
            last_deposit: FundUserAction::unpack(last_deposit)?,
            withdrawal_request: FundUserAction::unpack(withdrawal_request)?,
            last_withdrawal: FundUserAction::unpack(last_withdrawal)?,
            deny_reason: unpack_array_string64(deny_reason)?,
            bump: bump[0],
        })
    }
}

impl FundAssets {
    pub const LEN: usize = 50;
}

impl Packed for FundAssets {
    fn get_size(&self) -> usize {
        Self::LEN
    }

    fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, Self::LEN)?;

        let output = array_mut_ref![output, 0, FundAssets::LEN];

        let (
            fund_asset_type_out,
            target_hash_out,
            current_hash_out,
            current_cycle_out,
            current_assets_usd_out,
            cycle_start_time_out,
            cycle_end_time_out,
            bump_out,
        ) = mut_array_refs![output, 1, 8, 8, 8, 8, 8, 8, 1];

        fund_asset_type_out[0] = self.asset_type as u8;
        *target_hash_out = self.target_hash.to_le_bytes();
        *current_hash_out = self.current_hash.to_le_bytes();
        *current_cycle_out = self.current_cycle.to_le_bytes();
        *current_assets_usd_out = self.current_assets_usd.to_le_bytes();
        *cycle_start_time_out = self.cycle_start_time.to_le_bytes();
        *cycle_end_time_out = self.cycle_end_time.to_le_bytes();
        bump_out[0] = self.bump;

        Ok(Self::LEN)
    }

    fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; Self::LEN] = [0; Self::LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidAccountData)
        }
    }

    fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        check_data_len(input, Self::LEN)?;

        let input = array_ref![input, 0, FundAssets::LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            asset_type,
            target_hash,
            current_hash,
            current_cycle,
            current_assets_usd,
            cycle_start_time,
            cycle_end_time,
            bump,
        ) = array_refs![input, 1, 8, 8, 8, 8, 8, 8, 1];

        Ok(Self {
            asset_type: FundAssetType::try_from_primitive(asset_type[0])
                .or(Err(ProgramError::InvalidInstructionData))?,
            target_hash: u64::from_le_bytes(*target_hash),
            current_hash: u64::from_le_bytes(*current_hash),
            current_cycle: u64::from_le_bytes(*current_cycle),
            current_assets_usd: f64::from_le_bytes(*current_assets_usd),
            cycle_start_time: i64::from_le_bytes(*cycle_start_time),
            cycle_end_time: i64::from_le_bytes(*cycle_end_time),
            bump: bump[0],
        })
    }
}

impl FundCustody {
    pub const LEN: usize = 143;
}

impl Packed for FundCustody {
    fn get_size(&self) -> usize {
        Self::LEN
    }

    fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, Self::LEN)?;

        let output = array_mut_ref![output, 0, FundCustody::LEN];

        let (
            discriminator_out,
            fund_ref_out,
            custody_id_out,
            custody_type_out,
            is_vault_token_out,
            token_ref_out,
            address_out,
            fees_address_out,
            bump_out,
        ) = mut_array_refs![output, 8, 32, 4, 1, 1, 32, 32, 32, 1];

        *discriminator_out = self.discriminator.to_le_bytes();
        fund_ref_out.copy_from_slice(self.fund_ref.as_ref());
        *custody_id_out = self.custody_id.to_le_bytes();
        custody_type_out[0] = self.custody_type as u8;
        is_vault_token_out[0] = self.is_vault_token as u8;
        token_ref_out.copy_from_slice(self.token_ref.as_ref());
        address_out.copy_from_slice(self.address.as_ref());
        fees_address_out.copy_from_slice(self.fees_address.as_ref());
        bump_out[0] = self.bump;

        Ok(Self::LEN)
    }

    fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; Self::LEN] = [0; Self::LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidAccountData)
        }
    }

    fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        check_data_len(input, Self::LEN)?;

        let input = array_ref![input, 0, FundCustody::LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            discriminator,
            fund_ref,
            custody_id,
            custody_type,
            is_vault_token,
            token_ref,
            address,
            fees_address,
            bump,
        ) = array_refs![input, 8, 32, 4, 1, 1, 32, 32, 32, 1];

        Ok(Self {
            discriminator: u64::from_le_bytes(*discriminator),
            fund_ref: Pubkey::new_from_array(*fund_ref),
            custody_id: u32::from_le_bytes(*custody_id),
            custody_type: FundCustodyType::try_from_primitive(custody_type[0])
                .or(Err(ProgramError::InvalidAccountData))?,
            is_vault_token: unpack_bool(is_vault_token)?,
            token_ref: Pubkey::new_from_array(*token_ref),
            address: Pubkey::new_from_array(*address),
            fees_address: Pubkey::new_from_array(*fees_address),
            bump: bump[0],
        })
    }
}

impl FundVault {
    pub const LEN: usize = 222;
}

impl Packed for FundVault {
    fn get_size(&self) -> usize {
        Self::LEN
    }

    fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, Self::LEN)?;

        let output = array_mut_ref![output, 0, FundVault::LEN];

        let (
            discriminator_out,
            fund_ref_out,
            vault_id_out,
            vault_type_out,
            vault_ref_out,
            router_program_id_out,
            underlying_pool_id_out,
            underlying_pool_ref_out,
            underlying_lp_token_mint_out,
            lp_balance_out,
            balance_update_time_out,
            bump_out,
        ) = mut_array_refs![output, 8, 32, 4, 1, 32, 32, 32, 32, 32, 8, 8, 1];

        *discriminator_out = self.discriminator.to_le_bytes();
        fund_ref_out.copy_from_slice(self.fund_ref.as_ref());
        *vault_id_out = self.vault_id.to_le_bytes();
        vault_type_out[0] = self.vault_type as u8;
        vault_ref_out.copy_from_slice(self.vault_ref.as_ref());
        router_program_id_out.copy_from_slice(self.router_program_id.as_ref());
        underlying_pool_id_out.copy_from_slice(self.underlying_pool_id.as_ref());
        underlying_pool_ref_out.copy_from_slice(self.underlying_pool_ref.as_ref());
        underlying_lp_token_mint_out.copy_from_slice(self.underlying_lp_token_mint.as_ref());
        *lp_balance_out = self.lp_balance.to_le_bytes();
        *balance_update_time_out = self.balance_update_time.to_le_bytes();
        bump_out[0] = self.bump;

        Ok(Self::LEN)
    }

    fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; Self::LEN] = [0; Self::LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidAccountData)
        }
    }

    fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        check_data_len(input, Self::LEN)?;

        let input = array_ref![input, 0, FundVault::LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            discriminator,
            fund_ref,
            vault_id,
            vault_type,
            vault_ref,
            router_program_id,
            underlying_pool_id,
            underlying_pool_ref,
            underlying_lp_token_mint,
            lp_balance,
            balance_update_time,
            bump,
        ) = array_refs![input, 8, 32, 4, 1, 32, 32, 32, 32, 32, 8, 8, 1];

        Ok(Self {
            discriminator: u64::from_le_bytes(*discriminator),
            fund_ref: Pubkey::new_from_array(*fund_ref),
            vault_id: u32::from_le_bytes(*vault_id),
            vault_type: FundVaultType::try_from_primitive(vault_type[0])
                .or(Err(ProgramError::InvalidAccountData))?,
            vault_ref: Pubkey::new_from_array(*vault_ref),
            router_program_id: Pubkey::new_from_array(*router_program_id),
            underlying_pool_id: Pubkey::new_from_array(*underlying_pool_id),
            underlying_pool_ref: Pubkey::new_from_array(*underlying_pool_ref),
            underlying_lp_token_mint: Pubkey::new_from_array(*underlying_lp_token_mint),
            lp_balance: u64::from_le_bytes(*lp_balance),
            balance_update_time: i64::from_le_bytes(*balance_update_time),
            bump: bump[0],
        })
    }
}

impl Fund {
    pub const LEN: usize = 367;
}

impl Packed for Fund {
    fn get_size(&self) -> usize {
        Self::LEN
    }

    fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, Self::LEN)?;

        let output = array_mut_ref![output, 0, Fund::LEN];

        let (
            name_out,
            version_out,
            fund_type_out,
            official_out,
            refdb_index_out,
            refdb_counter_out,
            metadata_bump_out,
            authority_bump_out,
            fund_token_bump_out,
            multisig_bump_out,
            fund_program_id_out,
            fund_authority_out,
            fund_manager_out,
            fund_token_ref_out,
            info_account_out,
            multisig_account_out,
            vaults_assets_info_out,
            custodies_assets_info_out,
            description_account_out,
        ) = mut_array_refs![
            output, 64, 2, 1, 1, 5, 2, 1, 1, 1, 1, 32, 32, 32, 32, 32, 32, 32, 32, 32
        ];

        pack_array_string64(&self.name, name_out);
        *version_out = self.version.to_le_bytes();
        fund_type_out[0] = self.fund_type as u8;
        official_out[0] = self.official as u8;
        pack_option_u32(self.refdb_index, refdb_index_out);
        *refdb_counter_out = self.refdb_counter.to_le_bytes();
        metadata_bump_out[0] = self.metadata_bump as u8;
        authority_bump_out[0] = self.authority_bump as u8;
        fund_token_bump_out[0] = self.fund_token_bump as u8;
        multisig_bump_out[0] = self.multisig_bump as u8;
        fund_program_id_out.copy_from_slice(self.fund_program_id.as_ref());
        fund_authority_out.copy_from_slice(self.fund_authority.as_ref());
        fund_manager_out.copy_from_slice(self.fund_manager.as_ref());
        fund_token_ref_out.copy_from_slice(self.fund_token_ref.as_ref());
        info_account_out.copy_from_slice(self.info_account.as_ref());
        multisig_account_out.copy_from_slice(self.multisig_account.as_ref());
        vaults_assets_info_out.copy_from_slice(self.vaults_assets_info.as_ref());
        custodies_assets_info_out.copy_from_slice(self.custodies_assets_info.as_ref());
        description_account_out.copy_from_slice(self.description_account.as_ref());

        Ok(Self::LEN)
    }

    fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; Self::LEN] = [0; Self::LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidAccountData)
        }
    }

    fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        check_data_len(input, Self::LEN)?;

        let input = array_ref![input, 0, Fund::LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            name,
            version,
            fund_type,
            official,
            refdb_index,
            refdb_counter,
            metadata_bump,
            authority_bump,
            fund_token_bump,
            multisig_bump,
            fund_program_id,
            fund_authority,
            fund_manager,
            fund_token_ref,
            info_account,
            multisig_account,
            vaults_assets_info,
            custodies_assets_info,
            description_account,
        ) = array_refs![input, 64, 2, 1, 1, 5, 2, 1, 1, 1, 1, 32, 32, 32, 32, 32, 32, 32, 32, 32];

        Ok(Self {
            name: unpack_array_string64(name)?,
            version: u16::from_le_bytes(*version),
            fund_type: FundType::try_from_primitive(fund_type[0])
                .or(Err(ProgramError::InvalidAccountData))?,
            official: unpack_bool(official)?,
            refdb_index: unpack_option_u32(refdb_index)?,
            refdb_counter: u16::from_le_bytes(*refdb_counter),
            metadata_bump: metadata_bump[0],
            authority_bump: authority_bump[0],
            fund_token_bump: fund_token_bump[0],
            multisig_bump: multisig_bump[0],
            fund_program_id: Pubkey::new_from_array(*fund_program_id),
            fund_authority: Pubkey::new_from_array(*fund_authority),
            fund_manager: Pubkey::new_from_array(*fund_manager),
            fund_token_ref: Pubkey::new_from_array(*fund_token_ref),
            info_account: Pubkey::new_from_array(*info_account),
            multisig_account: Pubkey::new_from_array(*multisig_account),
            vaults_assets_info: Pubkey::new_from_array(*vaults_assets_info),
            custodies_assets_info: Pubkey::new_from_array(*custodies_assets_info),
            description_account: Pubkey::new_from_array(*description_account),
        })
    }
}

impl std::fmt::Display for FundType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            FundType::General => write!(f, "General"),
        }
    }
}

impl std::fmt::Display for Fund {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", to_string(&self).unwrap())
    }
}

impl std::fmt::Display for FundUserInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", to_string(&self).unwrap())
    }
}

impl std::fmt::Display for FundUserRequests {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", to_string(&self).unwrap())
    }
}

impl std::fmt::Display for FundInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", to_string(&self).unwrap())
    }
}

impl std::fmt::Display for FundAssets {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", to_string(&self).unwrap())
    }
}

impl std::fmt::Display for FundAssetType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            FundAssetType::Vault => write!(f, "Vault"),
            FundAssetType::Custody => write!(f, "Custody"),
        }
    }
}

impl std::str::FromStr for FundAssetType {
    type Err = ProgramError;

    fn from_str(s: &str) -> Result<Self, ProgramError> {
        match s.to_lowercase().as_str() {
            "vault" => Ok(FundAssetType::Vault),
            "custody" => Ok(FundAssetType::Custody),
            _ => Err(ProgramError::InvalidArgument),
        }
    }
}

impl std::fmt::Display for FundCustody {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", to_string(&self).unwrap())
    }
}

impl std::fmt::Display for FundCustodyWithBalance {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", to_string(&self).unwrap())
    }
}

impl std::fmt::Display for FundCustodyType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            FundCustodyType::DepositWithdraw => write!(f, "DepositWithdraw"),
            FundCustodyType::Trading => write!(f, "Trading"),
        }
    }
}

impl std::str::FromStr for FundCustodyType {
    type Err = ProgramError;

    fn from_str(s: &str) -> Result<Self, ProgramError> {
        match s.to_lowercase().as_str() {
            "depositwithdraw" => Ok(FundCustodyType::DepositWithdraw),
            "trading" => Ok(FundCustodyType::Trading),
            _ => Err(ProgramError::InvalidArgument),
        }
    }
}

impl std::fmt::Display for FundVault {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", to_string(&self).unwrap())
    }
}

impl std::fmt::Display for FundVaultType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            FundVaultType::Pool => write!(f, "Pool"),
            FundVaultType::Farm => write!(f, "Farm"),
            FundVaultType::Vault => write!(f, "Vault"),
        }
    }
}

impl std::str::FromStr for FundVaultType {
    type Err = ProgramError;

    fn from_str(s: &str) -> Result<Self, ProgramError> {
        match s.to_lowercase().as_str() {
            "pool" => Ok(FundVaultType::Pool),
            "farm" => Ok(FundVaultType::Farm),
            "vault" => Ok(FundVaultType::Vault),
            _ => Err(ProgramError::InvalidArgument),
        }
    }
}
