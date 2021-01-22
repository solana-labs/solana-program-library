//! FIXME copied from the solana stake program

use serde_derive::{Deserialize, Serialize};
use solana_program::{
    clock::{Epoch, UnixTimestamp},
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
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
    ///    must be `<= StakeAccount.lamports - staked_lamports`.
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
