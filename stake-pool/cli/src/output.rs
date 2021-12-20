use {
    serde::{Deserialize, Serialize},
    solana_cli_output::{QuietDisplay, VerboseDisplay},
    solana_sdk::native_token::Sol,
    solana_sdk::{pubkey::Pubkey, stake::state::Lockup},
    spl_stake_pool::state::{Fee, StakePool, StakeStatus, ValidatorList, ValidatorStakeInfo},
    std::fmt::{Display, Formatter, Result, Write},
};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliStakePools {
    pub pools: Vec<CliStakePool>,
}

impl Display for CliStakePools {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        for pool in &self.pools {
            writeln!(
                f,
                "Address: {}\tManager: {}\tLamports: {}\tPool tokens: {}\tValidators: {}",
                pool.address,
                pool.manager,
                pool.total_lamports,
                pool.pool_token_supply,
                pool.validator_list.len()
            )?;
        }
        writeln!(f, "Total number of pools: {}", &self.pools.len())?;
        Ok(())
    }
}

impl QuietDisplay for CliStakePools {}
impl VerboseDisplay for CliStakePools {}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliStakePool {
    pub address: String,
    pub pool_withdraw_authority: String,
    pub manager: String,
    pub staker: String,
    pub stake_deposit_authority: String,
    pub stake_withdraw_bump_seed: u8,
    pub max_validators: u32,
    pub validator_list: Vec<CliStakePoolValidator>,
    pub validator_list_storage_account: String,
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
    pub details: Option<CliStakePoolDetails>,
}

impl QuietDisplay for CliStakePool {}
impl VerboseDisplay for CliStakePool {
    fn write_str(&self, w: &mut dyn Write) -> Result {
        writeln!(w, "Stake Pool Info")?;
        writeln!(w, "===============")?;
        writeln!(w, "Stake Pool: {}", &self.address)?;
        writeln!(
            w,
            "Validator List: {}",
            &self.validator_list_storage_account
        )?;
        writeln!(w, "Manager: {}", &self.manager)?;
        writeln!(w, "Staker: {}", &self.staker)?;
        writeln!(w, "Depositor: {}", &self.stake_deposit_authority)?;
        writeln!(
            w,
            "SOL Deposit Authority: {}",
            &self
                .sol_deposit_authority
                .as_ref()
                .unwrap_or(&"None".to_string())
        )?;
        writeln!(
            w,
            "SOL Withdraw Authority: {}",
            &self
                .sol_withdraw_authority
                .as_ref()
                .unwrap_or(&"None".to_string())
        )?;
        writeln!(w, "Withdraw Authority: {}", &self.pool_withdraw_authority)?;
        writeln!(w, "Pool Token Mint: {}", &self.pool_mint)?;
        writeln!(w, "Fee Account: {}", &self.manager_fee_account)?;
        match &self.preferred_deposit_validator_vote_address {
            None => {}
            Some(s) => {
                writeln!(w, "Preferred Deposit Validator: {}", s)?;
            }
        }
        match &self.preferred_withdraw_validator_vote_address {
            None => {}
            Some(s) => {
                writeln!(w, "Preferred Withraw Validator: {}", s)?;
            }
        }
        writeln!(w, "Epoch Fee: {} of epoch rewards", &self.epoch_fee)?;
        if let Some(next_epoch_fee) = &self.next_epoch_fee {
            writeln!(w, "Next Epoch Fee: {} of epoch rewards", next_epoch_fee)?;
        }
        writeln!(
            w,
            "Stake Withdrawal Fee: {} of withdrawal amount",
            &self.stake_withdrawal_fee
        )?;
        if let Some(next_stake_withdrawal_fee) = &self.next_stake_withdrawal_fee {
            writeln!(
                w,
                "Next Stake Withdrawal Fee: {} of withdrawal amount",
                next_stake_withdrawal_fee
            )?;
        }
        writeln!(
            w,
            "SOL Withdrawal Fee: {} of withdrawal amount",
            &self.sol_withdrawal_fee
        )?;
        if let Some(next_sol_withdrawal_fee) = &self.next_sol_withdrawal_fee {
            writeln!(
                w,
                "Next SOL Withdrawal Fee: {} of withdrawal amount",
                next_sol_withdrawal_fee
            )?;
        }
        writeln!(
            w,
            "Stake Deposit Fee: {} of deposit amount",
            &self.stake_deposit_fee
        )?;
        writeln!(
            w,
            "SOL Deposit Fee: {} of deposit amount",
            &self.sol_deposit_fee
        )?;
        writeln!(
            w,
            "Stake Deposit Referral Fee: {}% of Stake Deposit Fee",
            &self.stake_referral_fee
        )?;
        writeln!(
            w,
            "SOL Deposit Referral Fee: {}% of SOL Deposit Fee",
            &self.sol_referral_fee
        )?;
        writeln!(w)?;
        writeln!(w, "Stake Accounts")?;
        writeln!(w, "--------------")?;
        match &self.details {
            None => {}
            Some(details) => {
                VerboseDisplay::write_str(details, w)?;
            }
        }
        Ok(())
    }
}

impl Display for CliStakePool {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "Stake Pool: {}", &self.address)?;
        writeln!(
            f,
            "Validator List: {}",
            &self.validator_list_storage_account
        )?;
        writeln!(f, "Pool Token Mint: {}", &self.pool_mint)?;
        match &self.preferred_deposit_validator_vote_address {
            None => {}
            Some(s) => {
                writeln!(f, "Preferred Deposit Validator: {}", s)?;
            }
        }
        match &self.preferred_withdraw_validator_vote_address {
            None => {}
            Some(s) => {
                writeln!(f, "Preferred Withraw Validator: {}", s)?;
            }
        }
        writeln!(f, "Epoch Fee: {} of epoch rewards", &self.epoch_fee)?;
        writeln!(
            f,
            "Stake Withdrawal Fee: {} of withdrawal amount",
            &self.stake_withdrawal_fee
        )?;
        writeln!(
            f,
            "SOL Withdrawal Fee: {} of withdrawal amount",
            &self.sol_withdrawal_fee
        )?;
        writeln!(
            f,
            "Stake Deposit Fee: {} of deposit amount",
            &self.stake_deposit_fee
        )?;
        writeln!(
            f,
            "SOL Deposit Fee: {} of deposit amount",
            &self.sol_deposit_fee
        )?;
        writeln!(
            f,
            "Stake Deposit Referral Fee: {}% of Stake Deposit Fee",
            &self.stake_referral_fee
        )?;
        writeln!(
            f,
            "SOL Deposit Referral Fee: {}% of SOL Deposit Fee",
            &self.sol_referral_fee
        )?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliStakePoolDetails {
    pub reserve_stake_account_address: String,
    pub reserve_stake_lamports: u64,
    pub minimum_reserve_stake_balance: u64,
    pub stake_accounts: Vec<CliStakePoolStakeAccountInfo>,
    pub total_lamports: u64,
    pub total_pool_tokens: f64,
    pub current_number_of_validators: u32,
    pub max_number_of_validators: u32,
    pub update_required: bool,
}

impl Display for CliStakePoolDetails {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(
            f,
            "Reserve Account: {}\tAvailable Balance: {}",
            &self.reserve_stake_account_address,
            Sol(self.reserve_stake_lamports - self.minimum_reserve_stake_balance),
        )?;
        for stake_account in &self.stake_accounts {
            writeln!(
                f,
                "Vote Account: {}\tBalance: {}\tLast Update Epoch: {}",
                stake_account.vote_account_address,
                Sol(stake_account.validator_lamports),
                stake_account.validator_last_update_epoch,
            )?;
        }
        writeln!(
            f,
            "Total Pool Stake: {} {}",
            Sol(self.total_lamports),
            if self.update_required {
                " [UPDATE REQUIRED]"
            } else {
                ""
            },
        )?;
        writeln!(f, "Total Pool Tokens: {}", &self.total_pool_tokens,)?;
        writeln!(
            f,
            "Current Number of Validators: {}",
            &self.current_number_of_validators,
        )?;
        writeln!(
            f,
            "Max Number of Validators: {}",
            &self.max_number_of_validators,
        )?;
        Ok(())
    }
}

impl QuietDisplay for CliStakePoolDetails {}
impl VerboseDisplay for CliStakePoolDetails {
    fn write_str(&self, w: &mut dyn Write) -> Result {
        writeln!(w)?;
        writeln!(w, "Stake Accounts")?;
        writeln!(w, "--------------")?;
        writeln!(
            w,
            "Reserve Account: {}\tAvailable Balance: {}",
            &self.reserve_stake_account_address,
            Sol(self.reserve_stake_lamports - self.minimum_reserve_stake_balance),
        )?;
        for stake_account in &self.stake_accounts {
            writeln!(
                w,
                "Vote Account: {}\tStake Account: {}\tActive Balance: {}\tTransient Stake Account: {}\tTransient Balance: {}\tLast Update Epoch: {}{}",
                stake_account.vote_account_address,
                stake_account.stake_account_address,
                Sol(stake_account.validator_active_stake_lamports),
                stake_account.validator_transient_stake_account_address,
                Sol(stake_account.validator_transient_stake_lamports),
                stake_account.validator_last_update_epoch,
                if stake_account.update_required {
                    " [UPDATE REQUIRED]"
                } else {
                    ""
                },
            )?;
        }
        writeln!(
            w,
            "Total Pool Stake: {} {}",
            Sol(self.total_lamports),
            if self.update_required {
                " [UPDATE REQUIRED]"
            } else {
                ""
            },
        )?;
        writeln!(w, "Total Pool Tokens: {}", &self.total_pool_tokens,)?;
        writeln!(
            w,
            "Current Number of Validators: {}",
            &self.current_number_of_validators,
        )?;
        writeln!(
            w,
            "Max Number of Validators: {}",
            &self.max_number_of_validators,
        )?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliStakePoolStakeAccountInfo {
    pub vote_account_address: String,
    pub stake_account_address: String,
    pub validator_active_stake_lamports: u64,
    pub validator_last_update_epoch: u64,
    pub validator_lamports: u64,
    pub validator_transient_stake_account_address: String,
    pub validator_transient_stake_lamports: u64,
    pub update_required: bool,
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
        match s {
            StakeStatus::Active => CliStakePoolValidatorStakeStatus::Active,
            StakeStatus::DeactivatingTransient => {
                CliStakePoolValidatorStakeStatus::DeactivatingTransient
            }
            StakeStatus::ReadyForRemoval => CliStakePoolValidatorStakeStatus::ReadyForRemoval,
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
            custodian: l.custodian.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliStakePoolFee {
    pub denominator: u64,
    pub numerator: u64,
}

impl Display for CliStakePoolFee {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}/{}", &self.numerator, &self.denominator)
    }
}

impl From<Fee> for CliStakePoolFee {
    fn from(f: Fee) -> Self {
        Self {
            denominator: f.denominator,
            numerator: f.numerator,
        }
    }
}

impl From<(Pubkey, StakePool, ValidatorList, Pubkey)> for CliStakePool {
    fn from(s: (Pubkey, StakePool, ValidatorList, Pubkey)) -> Self {
        let (address, stake_pool, validator_list, pool_withdraw_authority) = s;
        Self {
            address: address.to_string(),
            pool_withdraw_authority: pool_withdraw_authority.to_string(),
            manager: stake_pool.manager.to_string(),
            staker: stake_pool.staker.to_string(),
            stake_deposit_authority: stake_pool.stake_deposit_authority.to_string(),
            stake_withdraw_bump_seed: stake_pool.stake_withdraw_bump_seed,
            max_validators: validator_list.header.max_validators,
            validator_list: validator_list
                .validators
                .into_iter()
                .map(CliStakePoolValidator::from)
                .collect(),
            validator_list_storage_account: stake_pool.validator_list.to_string(),
            reserve_stake: stake_pool.reserve_stake.to_string(),
            pool_mint: stake_pool.pool_mint.to_string(),
            manager_fee_account: stake_pool.manager_fee_account.to_string(),
            token_program_id: stake_pool.token_program_id.to_string(),
            total_lamports: stake_pool.total_lamports,
            pool_token_supply: stake_pool.pool_token_supply,
            last_update_epoch: stake_pool.last_update_epoch,
            lockup: CliStakePoolLockup::from(stake_pool.lockup),
            epoch_fee: CliStakePoolFee::from(stake_pool.epoch_fee),
            next_epoch_fee: stake_pool.next_epoch_fee.map(CliStakePoolFee::from),
            preferred_deposit_validator_vote_address: stake_pool
                .preferred_deposit_validator_vote_address
                .map(|x| x.to_string()),
            preferred_withdraw_validator_vote_address: stake_pool
                .preferred_withdraw_validator_vote_address
                .map(|x| x.to_string()),
            stake_deposit_fee: CliStakePoolFee::from(stake_pool.stake_deposit_fee),
            stake_withdrawal_fee: CliStakePoolFee::from(stake_pool.stake_withdrawal_fee),
            next_stake_withdrawal_fee: stake_pool
                .next_stake_withdrawal_fee
                .map(CliStakePoolFee::from),
            stake_referral_fee: stake_pool.stake_referral_fee,
            sol_deposit_authority: stake_pool.sol_deposit_authority.map(|x| x.to_string()),
            sol_deposit_fee: CliStakePoolFee::from(stake_pool.stake_deposit_fee),
            sol_referral_fee: stake_pool.sol_referral_fee,
            sol_withdraw_authority: stake_pool.sol_deposit_authority.map(|x| x.to_string()),
            sol_withdrawal_fee: CliStakePoolFee::from(stake_pool.sol_withdrawal_fee),
            next_sol_withdrawal_fee: stake_pool
                .next_sol_withdrawal_fee
                .map(CliStakePoolFee::from),
            last_epoch_pool_token_supply: stake_pool.last_epoch_pool_token_supply,
            last_epoch_total_lamports: stake_pool.last_epoch_total_lamports,
            details: None,
        }
    }
}
