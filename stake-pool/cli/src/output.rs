use {
    std::fmt::{Display, Formatter},
    solana_cli_output::{QuietDisplay, VerboseDisplay},
    solana_sdk::{pubkey::Pubkey, stake::state::Lockup},
    serde::{Serialize, Deserialize},
    spl_stake_pool::state::{Fee, StakePool, StakeStatus, ValidatorList, ValidatorStakeInfo},
};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliStakePools {
    pub pools: Vec<CliStakePool>
}

impl Display for CliStakePools {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for pool in &self.pools {
            writeln!(f,
                     "Address: {}\tManager: {}\tLamports: {}\tPool tokens: {}\tValidators: {}",
                     pool.address,
                     pool.manager,
                     pool.total_lamports,
                     pool.pool_token_supply,
                     pool.validator_list.len()
            ).ok();
        }
        writeln!(f, "Total number of pools: {}", &self.pools.len())
    }
}

impl QuietDisplay for CliStakePools {}
impl VerboseDisplay for CliStakePools {}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliStakePool {
    pub address: String,
    pub manager: String,
    pub staker: String,
    pub stake_deposit_authority: String,
    pub stake_withdraw_bump_seed: u8,
    pub max_validators: u32,
    pub validator_list: Vec<CliStakePoolValidator>,
    pub reserve_stake: String,
    pub pool_mint: String,
    pub manager_fee_account: String,
    pub token_program_id: String,
    pub total_lamports: u64,
    pub pool_token_supply: u64,
    pub last_update_epoch: u64,
    pub lockup: CliStakePoolLockup,
    pub epoch_fee: CliStakePoolFee,
    pub next_epoch_fee: Option<CliStakePoolFee>,
    pub preferred_deposit_validator_vote_address: Option<String>,
    pub preferred_withdraw_validator_vote_address: Option<String>,
    pub stake_deposit_fee: CliStakePoolFee,
    pub stake_withdrawal_fee: CliStakePoolFee,
    pub next_stake_withdrawal_fee: Option<CliStakePoolFee>,
    pub stake_referral_fee: u8,
    pub sol_deposit_authority: Option<String>,
    pub sol_deposit_fee: CliStakePoolFee,
    pub sol_referral_fee: u8,
    pub sol_withdraw_authority: Option<String>,
    pub sol_withdrawal_fee: CliStakePoolFee,
    pub next_sol_withdrawal_fee: Option<CliStakePoolFee>,
    pub last_epoch_pool_token_supply: u64,
    pub last_epoch_total_lamports: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliStakePoolValidator {
    pub active_stake_lamports: u64,
    pub transient_stake_lamports: u64,
    pub last_update_epoch: u64,
    pub transient_seed_suffix_start: u64,
    pub transient_seed_suffix_end: u64,
    pub status: CliStakePoolValidatorStakeStatus,
    pub vote_account_address: String,
}

impl From<ValidatorStakeInfo> for CliStakePoolValidator {
    fn from(v: ValidatorStakeInfo) -> Self {
        Self {
            active_stake_lamports: v.active_stake_lamports,
            transient_stake_lamports: v.transient_stake_lamports,
            last_update_epoch: v.last_update_epoch,
            transient_seed_suffix_start: v.transient_seed_suffix_start,
            transient_seed_suffix_end: v.transient_seed_suffix_end,
            status: CliStakePoolValidatorStakeStatus::from(v.status),
            vote_account_address: v.vote_account_address.to_string(),
        }
    }
}

impl From<StakeStatus> for CliStakePoolValidatorStakeStatus {
    fn from(s: StakeStatus) -> CliStakePoolValidatorStakeStatus {
        return match s {
            StakeStatus::Active => CliStakePoolValidatorStakeStatus::Active,
            StakeStatus::DeactivatingTransient => CliStakePoolValidatorStakeStatus::DeactivatingTransient,
            StakeStatus::ReadyForRemoval => CliStakePoolValidatorStakeStatus::ReadyForRemoval
        }
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) enum CliStakePoolValidatorStakeStatus {
    Active,
    DeactivatingTransient,
    ReadyForRemoval,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliStakePoolLockup {
    pub unix_timestamp: i64,
    pub epoch: u64,
    pub custodian: String,
}

impl From<Lockup> for CliStakePoolLockup {
    fn from(l: Lockup) -> Self {
        Self {
            unix_timestamp: l.unix_timestamp,
            epoch: l.epoch,
            custodian: l.custodian.to_string()
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliStakePoolFee {
    pub denominator: u64,
    pub numerator: u64,
}

impl From<Fee> for CliStakePoolFee {
    fn from(f: Fee) -> Self {
        Self {
            denominator: f.denominator,
            numerator: f.numerator
        }
    }
}

impl From<(Pubkey, StakePool, ValidatorList)> for CliStakePool {
    fn from(s: (Pubkey, StakePool, ValidatorList)) -> Self {
        let (pubkey, stake_pool, validator_list) = s;
        Self {
            address: pubkey.to_string(),
            manager: stake_pool.manager.to_string(),
            staker: stake_pool.staker.to_string(),
            stake_deposit_authority: stake_pool.stake_deposit_authority.to_string(),
            stake_withdraw_bump_seed: stake_pool.stake_withdraw_bump_seed,
            max_validators: validator_list.header.max_validators,
            validator_list: validator_list.validators.into_iter().map(|x| CliStakePoolValidator::from(x)).collect(),
            reserve_stake: stake_pool.reserve_stake.to_string(),
            pool_mint: stake_pool.pool_mint.to_string(),
            manager_fee_account: stake_pool.manager_fee_account.to_string(),
            token_program_id: stake_pool.token_program_id.to_string(),
            total_lamports: stake_pool.total_lamports,
            pool_token_supply: stake_pool.pool_token_supply,
            last_update_epoch: stake_pool.last_update_epoch,
            lockup: CliStakePoolLockup::from(stake_pool.lockup),
            epoch_fee: CliStakePoolFee::from(stake_pool.epoch_fee),
            next_epoch_fee: stake_pool.next_epoch_fee.map(|x| CliStakePoolFee::from(x)),
            preferred_deposit_validator_vote_address: stake_pool.preferred_deposit_validator_vote_address.map(|x| x.to_string()),
            preferred_withdraw_validator_vote_address: stake_pool.preferred_withdraw_validator_vote_address.map(|x| x.to_string()),
            stake_deposit_fee: CliStakePoolFee::from(stake_pool.stake_deposit_fee),
            stake_withdrawal_fee: CliStakePoolFee::from(stake_pool.stake_withdrawal_fee),
            next_stake_withdrawal_fee: stake_pool.next_sol_withdrawal_fee.map(|x| CliStakePoolFee::from(x)),
            stake_referral_fee: stake_pool.stake_referral_fee,
            sol_deposit_authority: stake_pool.sol_deposit_authority.map(|x| x.to_string()),
            sol_deposit_fee: CliStakePoolFee::from(stake_pool.stake_deposit_fee),
            sol_referral_fee: stake_pool.sol_referral_fee,
            sol_withdraw_authority: stake_pool.sol_deposit_authority.map(|x| x.to_string()),
            sol_withdrawal_fee: CliStakePoolFee::from(stake_pool.sol_withdrawal_fee),
            next_sol_withdrawal_fee: stake_pool.next_sol_withdrawal_fee.map(|x| CliStakePoolFee::from(x)),
            last_epoch_pool_token_supply: stake_pool.last_epoch_pool_token_supply,
            last_epoch_total_lamports: stake_pool.last_epoch_total_lamports,
        }
    }
}