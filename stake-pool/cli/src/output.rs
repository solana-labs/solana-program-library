use std::fmt::{Display, Formatter};
use solana_cli_output::{QuietDisplay, VerboseDisplay};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::stake::state::Lockup;
use serde::{Serialize, Deserialize};
use spl_stake_pool::state::{AccountType, Fee, StakePool, StakeStatus, ValidatorList, ValidatorStakeInfo};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliStakePools {
    pub pools: Vec<CliStakePool>
}

impl Display for CliStakePools {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
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
    pub status: String,
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
            status: match v.status {
                StakeStatus::Active => "Active",
                StakeStatus::DeactivatingTransient => "DeactivatingTransient",
                StakeStatus::ReadyForRemoval => "ReadyForRemoval",
            }.to_string(),
            vote_account_address: v.vote_account_address.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum CliStakePoolValidatorStakeStatus{
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
        Self {
            address: s.0.to_string(),
            manager: s.1.manager.to_string(),
            staker: s.1.staker.to_string(),
            stake_deposit_authority: s.1.stake_deposit_authority.to_string(),
            stake_withdraw_bump_seed: s.1.stake_withdraw_bump_seed,
            max_validators: s.2.header.max_validators,
            validator_list: s.2.validators.into_iter().map(|x| CliStakePoolValidator::from(x)).collect(),
            reserve_stake: s.1.reserve_stake.to_string(),
            pool_mint: s.1.pool_mint.to_string(),
            manager_fee_account: s.1.manager_fee_account.to_string(),
            token_program_id: s.1.token_program_id.to_string(),
            total_lamports: s.1.total_lamports,
            pool_token_supply: s.1.pool_token_supply,
            last_update_epoch: s.1.last_update_epoch,
            lockup: CliStakePoolLockup::from(s.1.lockup),
            epoch_fee: CliStakePoolFee::from(s.1.epoch_fee),
            next_epoch_fee: s.1.next_epoch_fee.map(|x| CliStakePoolFee::from(x)),
            preferred_deposit_validator_vote_address: s.1.preferred_deposit_validator_vote_address.map(|x| x.to_string()),
            preferred_withdraw_validator_vote_address: s.1.preferred_withdraw_validator_vote_address.map(|x| x.to_string()),
            stake_deposit_fee: CliStakePoolFee::from(s.1.stake_deposit_fee),
            stake_withdrawal_fee: CliStakePoolFee::from(s.1.stake_withdrawal_fee),
            next_stake_withdrawal_fee: s.1.next_sol_withdrawal_fee.map(|x| CliStakePoolFee::from(x)),
            stake_referral_fee: s.1.stake_referral_fee,
            sol_deposit_authority: s.1.sol_deposit_authority.map(|x| x.to_string()),
            sol_deposit_fee: CliStakePoolFee::from(s.1.stake_deposit_fee),
            sol_referral_fee: s.1.sol_referral_fee,
            sol_withdraw_authority: s.1.sol_deposit_authority.map(|x| x.to_string()),
            sol_withdrawal_fee: CliStakePoolFee::from(s.1.sol_withdrawal_fee),
            next_sol_withdrawal_fee: s.1.next_sol_withdrawal_fee.map(|x| CliStakePoolFee::from(x)),
            last_epoch_pool_token_supply: s.1.last_epoch_pool_token_supply,
            last_epoch_total_lamports: s.1.last_epoch_total_lamports,
        }
    }
}