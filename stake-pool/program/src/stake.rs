//! FIXME copied from the solana stake program

use serde_derive::{Deserialize, Serialize};
use solana_program::{
    clock::{Epoch, UnixTimestamp},
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    stake_history::StakeHistory,
    system_instruction, sysvar,
};
use std::str::FromStr;

solana_program::declare_id!("Stake11111111111111111111111111111111111111");

const STAKE_CONFIG: &str = "StakeConfig11111111111111111111111111111111";

/// FIXME copied from solana stake program
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum StakeInstruction {
    /// Initialize a stake with lockup and authorization information
    ///
    /// # Account references
    ///   0. [WRITE] Uninitialized stake account
    ///   1. [] Rent sysvar
    ///
    /// Authorized carries pubkeys that must sign staker transactions
    ///   and withdrawer transactions.
    /// Lockup carries information about withdrawal restrictions
    Initialize(Authorized, Lockup),

    /// Authorize a key to manage stake or withdrawal
    ///
    /// # Account references
    ///   0. [WRITE] Stake account to be updated
    ///   1. [] (reserved for future use) Clock sysvar
    ///   2. [SIGNER] The stake or withdraw authority
    Authorize(Pubkey, StakeAuthorize),

    /// Delegate a stake to a particular vote account
    ///
    /// # Account references
    ///   0. [WRITE] Initialized stake account to be delegated
    ///   1. [] Vote account to which this stake will be delegated
    ///   2. [] Clock sysvar
    ///   3. [] Stake history sysvar that carries stake warmup/cooldown history
    ///   4. [] Address of config account that carries stake config
    ///   5. [SIGNER] Stake authority
    ///
    /// The entire balance of the staking account is staked.  DelegateStake
    ///   can be called multiple times, but re-delegation is delayed
    ///   by one epoch
    DelegateStake,

    /// Split u64 tokens and stake off a stake account into another stake account.
    ///
    /// # Account references
    ///   0. [WRITE] Stake account to be split; must be in the Initialized or Stake state
    ///   1. [WRITE] Uninitialized stake account that will take the split-off amount
    ///   2. [SIGNER] Stake authority
    Split(u64),

    /// Withdraw unstaked lamports from the stake account
    ///
    /// # Account references
    ///   0. [WRITE] Stake account from which to withdraw
    ///   1. [WRITE] Recipient account
    ///   2. [] Clock sysvar
    ///   3. [] Stake history sysvar that carries stake warmup/cooldown history
    ///   4. [SIGNER] Withdraw authority
    ///   5. Optional: [SIGNER] Lockup authority, if before lockup expiration
    ///
    /// The u64 is the portion of the stake account balance to be withdrawn,
    ///    must be `<= ValidatorStakeAccount.lamports - staked_lamports`.
    Withdraw(u64),

    /// Deactivates the stake in the account
    ///
    /// # Account references
    ///   0. [WRITE] Delegated stake account
    ///   1. [] Clock sysvar
    ///   2. [SIGNER] Stake authority
    Deactivate,

    /// Set stake lockup
    ///
    /// # Account references
    ///   0. [WRITE] Initialized stake account
    ///   1. [SIGNER] Lockup authority
    SetLockupNOTUSED,

    /// Merge two stake accounts. Both accounts must be deactivated and have identical lockup and
    /// authority keys.
    ///
    /// # Account references
    ///   0. [WRITE] Destination stake account for the merge
    ///   1. [WRITE] Source stake account for to merge.  This account will be drained
    ///   2. [] Clock sysvar
    ///   3. [] Stake history sysvar that carries stake warmup/cooldown history
    ///   4. [SIGNER] Stake authority
    Merge,

    /// Authorize a key to manage stake or withdrawal with a derived key
    ///
    /// # Account references
    ///   0. [WRITE] Stake account to be updated
    ///   1. [SIGNER] Base key of stake or withdraw authority
    AuthorizeWithSeedNOTUSED,
}

/// FIXME copied from the stake program
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
#[allow(clippy::large_enum_variant)]
pub enum StakeState {
    /// FIXME copied from the stake program
    Uninitialized,
    /// FIXME copied from the stake program
    Initialized(Meta),
    /// FIXME copied from the stake program
    Stake(Meta, Stake),
    /// FIXME copied from the stake program
    RewardsPool,
}

/// FIXME copied from the stake program
#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub struct Meta {
    /// FIXME copied from the stake program
    pub rent_exempt_reserve: u64,
    /// FIXME copied from the stake program
    pub authorized: Authorized,
    /// FIXME copied from the stake program
    pub lockup: Lockup,
}

/// FIXME copied from the stake program
#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub struct Stake {
    /// FIXME copied from the stake program
    pub delegation: Delegation,
    /// credits observed is credits from vote account state when delegated or redeemed
    pub credits_observed: u64,
}

/// FIXME copied from the stake program
#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub struct Delegation {
    /// to whom the stake is delegated
    pub voter_pubkey: Pubkey,
    /// activated stake amount, set at delegate() time
    pub stake: u64,
    /// epoch at which this stake was activated, std::Epoch::MAX if is a bootstrap stake
    pub activation_epoch: Epoch,
    /// epoch the stake was deactivated, std::Epoch::MAX if not deactivated
    pub deactivation_epoch: Epoch,
    /// how much stake we can activate per-epoch as a fraction of currently effective stake
    pub warmup_cooldown_rate: f64,
}

/// FIXME copied from the stake program
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum StakeAuthorize {
    /// FIXME copied from the stake program
    Staker,
    /// FIXME copied from the stake program
    Withdrawer,
}
/// FIXME copied from the stake program
#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub struct Authorized {
    /// FIXME copied from the stake program
    pub staker: Pubkey,
    /// FIXME copied from the stake program
    pub withdrawer: Pubkey,
}

/// FIXME copied from the stake program
#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub struct Lockup {
    /// UnixTimestamp at which this stake will allow withdrawal, unless the
    ///   transaction is signed by the custodian
    pub unix_timestamp: UnixTimestamp,
    /// epoch height at which this stake will allow withdrawal, unless the
    ///   transaction is signed by the custodian
    pub epoch: Epoch,
    /// custodian signature on a transaction exempts the operation from
    ///  lockup constraints
    pub custodian: Pubkey,
}

/// FIXME copied from the stake program
impl StakeState {
    /// Get Delegation
    pub fn delegation(&self) -> Option<Delegation> {
        match self {
            StakeState::Stake(_meta, stake) => Some(stake.delegation),
            _ => None,
        }
    }
}

/// FIXME copied from the stake program
impl Delegation {
    /// Create new Delegation
    pub fn new(
        voter_pubkey: &Pubkey,
        stake: u64,
        activation_epoch: Epoch,
        warmup_cooldown_rate: f64,
    ) -> Self {
        Self {
            voter_pubkey: *voter_pubkey,
            stake,
            activation_epoch,
            warmup_cooldown_rate,
            ..Delegation::default()
        }
    }
    /// Check if it bootstrap
    pub fn is_bootstrap(&self) -> bool {
        self.activation_epoch == std::u64::MAX
    }

    /// Return tuple (effective, activating, deactivating) stake
    #[allow(clippy::comparison_chain)]
    pub fn stake_activating_and_deactivating(
        &self,
        target_epoch: Epoch,
        history: Option<&StakeHistory>,
        fix_stake_deactivate: bool,
    ) -> (u64, u64, u64) {
        let delegated_stake = self.stake;

        // first, calculate an effective and activating stake
        let (effective_stake, activating_stake) =
            self.stake_and_activating(target_epoch, history, fix_stake_deactivate);

        // then de-activate some portion if necessary
        if target_epoch < self.deactivation_epoch {
            // not deactivated
            (effective_stake, activating_stake, 0)
        } else if target_epoch == self.deactivation_epoch {
            // can only deactivate what's activated
            (effective_stake, 0, effective_stake.min(delegated_stake))
        } else if let Some((history, mut prev_epoch, mut prev_cluster_stake)) =
            history.and_then(|history| {
                history
                    .get(&self.deactivation_epoch)
                    .map(|cluster_stake_at_deactivation_epoch| {
                        (
                            history,
                            self.deactivation_epoch,
                            cluster_stake_at_deactivation_epoch,
                        )
                    })
            })
        {
            // target_epoch > self.deactivation_epoch

            // loop from my deactivation epoch until the target epoch
            // current effective stake is updated using its previous epoch's cluster stake
            let mut current_epoch;
            let mut current_effective_stake = effective_stake;
            loop {
                current_epoch = prev_epoch + 1;
                // if there is no deactivating stake at prev epoch, we should have been
                // fully undelegated at this moment
                if prev_cluster_stake.deactivating == 0 {
                    break;
                }

                // I'm trying to get to zero, how much of the deactivation in stake
                //   this account is entitled to take
                let weight =
                    current_effective_stake as f64 / prev_cluster_stake.deactivating as f64;

                // portion of newly not-effective cluster stake I'm entitled to at current epoch
                let newly_not_effective_cluster_stake =
                    prev_cluster_stake.effective as f64 * self.warmup_cooldown_rate;
                let newly_not_effective_stake =
                    ((weight * newly_not_effective_cluster_stake) as u64).max(1);

                current_effective_stake =
                    current_effective_stake.saturating_sub(newly_not_effective_stake);
                if current_effective_stake == 0 {
                    break;
                }

                if current_epoch >= target_epoch {
                    break;
                }
                if let Some(current_cluster_stake) = history.get(&current_epoch) {
                    prev_epoch = current_epoch;
                    prev_cluster_stake = current_cluster_stake;
                } else {
                    break;
                }
            }

            // deactivating stake should equal to all of currently remaining effective stake
            (current_effective_stake, 0, current_effective_stake)
        } else {
            // no history or I've dropped out of history, so assume fully deactivated
            (0, 0, 0)
        }
    }

    // returned tuple is (effective, activating) stake
    fn stake_and_activating(
        &self,
        target_epoch: Epoch,
        history: Option<&StakeHistory>,
        fix_stake_deactivate: bool,
    ) -> (u64, u64) {
        let delegated_stake = self.stake;

        if self.is_bootstrap() {
            // fully effective immediately
            (delegated_stake, 0)
        } else if fix_stake_deactivate && self.activation_epoch == self.deactivation_epoch {
            // activated but instantly deactivated; no stake at all regardless of target_epoch
            // this must be after the bootstrap check and before all-is-activating check
            (0, 0)
        } else if target_epoch == self.activation_epoch {
            // all is activating
            (0, delegated_stake)
        } else if target_epoch < self.activation_epoch {
            // not yet enabled
            (0, 0)
        } else if let Some((history, mut prev_epoch, mut prev_cluster_stake)) =
            history.and_then(|history| {
                history
                    .get(&self.activation_epoch)
                    .map(|cluster_stake_at_activation_epoch| {
                        (
                            history,
                            self.activation_epoch,
                            cluster_stake_at_activation_epoch,
                        )
                    })
            })
        {
            // target_epoch > self.activation_epoch

            // loop from my activation epoch until the target epoch summing up my entitlement
            // current effective stake is updated using its previous epoch's cluster stake
            let mut current_epoch;
            let mut current_effective_stake = 0;
            loop {
                current_epoch = prev_epoch + 1;
                // if there is no activating stake at prev epoch, we should have been
                // fully effective at this moment
                if prev_cluster_stake.activating == 0 {
                    break;
                }

                // how much of the growth in stake this account is
                //  entitled to take
                let remaining_activating_stake = delegated_stake - current_effective_stake;
                let weight =
                    remaining_activating_stake as f64 / prev_cluster_stake.activating as f64;

                // portion of newly effective cluster stake I'm entitled to at current epoch
                let newly_effective_cluster_stake =
                    prev_cluster_stake.effective as f64 * self.warmup_cooldown_rate;
                let newly_effective_stake =
                    ((weight * newly_effective_cluster_stake) as u64).max(1);

                current_effective_stake += newly_effective_stake;
                if current_effective_stake >= delegated_stake {
                    current_effective_stake = delegated_stake;
                    break;
                }

                if current_epoch >= target_epoch || current_epoch >= self.deactivation_epoch {
                    break;
                }
                if let Some(current_cluster_stake) = history.get(&current_epoch) {
                    prev_epoch = current_epoch;
                    prev_cluster_stake = current_cluster_stake;
                } else {
                    break;
                }
            }

            (
                current_effective_stake,
                delegated_stake - current_effective_stake,
            )
        } else {
            // no history or I've dropped out of history, so assume fully effective
            (delegated_stake, 0)
        }
    }
}

/// FIXME copied from the stake program
pub fn split_only(
    stake_pubkey: &Pubkey,
    authorized_pubkey: &Pubkey,
    lamports: u64,
    split_stake_pubkey: &Pubkey,
) -> Instruction {
    let account_metas = vec![
        AccountMeta::new(*stake_pubkey, false),
        AccountMeta::new(*split_stake_pubkey, false),
        AccountMeta::new_readonly(*authorized_pubkey, true),
    ];

    Instruction::new(id(), &StakeInstruction::Split(lamports), account_metas)
}

/// FIXME copied from the stake program
pub fn authorize(
    stake_pubkey: &Pubkey,
    authorized_pubkey: &Pubkey,
    new_authorized_pubkey: &Pubkey,
    stake_authorize: StakeAuthorize,
) -> Instruction {
    let account_metas = vec![
        AccountMeta::new(*stake_pubkey, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(*authorized_pubkey, true),
    ];

    Instruction::new(
        id(),
        &StakeInstruction::Authorize(*new_authorized_pubkey, stake_authorize),
        account_metas,
    )
}

/// FIXME copied from the stake program
pub fn merge(
    destination_stake_pubkey: &Pubkey,
    source_stake_pubkey: &Pubkey,
    authorized_pubkey: &Pubkey,
) -> Instruction {
    let account_metas = vec![
        AccountMeta::new(*destination_stake_pubkey, false),
        AccountMeta::new(*source_stake_pubkey, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::stake_history::id(), false),
        AccountMeta::new_readonly(*authorized_pubkey, true),
    ];

    Instruction::new(id(), &StakeInstruction::Merge, account_metas)
}

/// FIXME copied from the stake program
pub fn create_account(
    from_pubkey: &Pubkey,
    stake_pubkey: &Pubkey,
    authorized: &Authorized,
    lockup: &Lockup,
    lamports: u64,
) -> Vec<Instruction> {
    vec![
        system_instruction::create_account(
            from_pubkey,
            stake_pubkey,
            lamports,
            std::mem::size_of::<StakeState>() as u64,
            &id(),
        ),
        initialize(stake_pubkey, authorized, lockup),
    ]
}

/// FIXME copied from the stake program
pub fn initialize(stake_pubkey: &Pubkey, authorized: &Authorized, lockup: &Lockup) -> Instruction {
    Instruction::new(
        id(),
        &StakeInstruction::Initialize(*authorized, *lockup),
        vec![
            AccountMeta::new(*stake_pubkey, false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
    )
}

/// FIXME copied from the stake program
pub fn delegate_stake(
    stake_pubkey: &Pubkey,
    authorized_pubkey: &Pubkey,
    vote_pubkey: &Pubkey,
) -> Instruction {
    let account_metas = vec![
        AccountMeta::new(*stake_pubkey, false),
        AccountMeta::new_readonly(*vote_pubkey, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::stake_history::id(), false),
        AccountMeta::new_readonly(Pubkey::from_str(STAKE_CONFIG).unwrap(), false),
        AccountMeta::new_readonly(*authorized_pubkey, true),
    ];
    Instruction::new(id(), &StakeInstruction::DelegateStake, account_metas)
}
