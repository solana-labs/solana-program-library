//! Instruction types

// Remove the following `allow` when `Redelegate` is removed, required to avoid
// warnings from uses of deprecated types during trait derivations.
#![allow(deprecated)]
#![allow(clippy::too_many_arguments)]

use {
    crate::{
        find_deposit_authority_program_address, find_ephemeral_stake_program_address,
        find_stake_program_address, find_transient_stake_program_address,
        find_withdraw_authority_program_address,
        inline_mpl_token_metadata::{self, pda::find_metadata_account},
        state::{Fee, FeeType, StakePool, ValidatorList, ValidatorStakeInfo},
        MAX_VALIDATORS_TO_UPDATE,
    },
    borsh::{BorshDeserialize, BorshSchema, BorshSerialize},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
        stake,
        stake_history::Epoch,
        system_program, sysvar,
    },
    std::num::NonZeroU32,
};

/// Defines which validator vote account is set during the
/// `SetPreferredValidator` instruction
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize, BorshSchema)]
pub enum PreferredValidatorType {
    /// Set preferred validator for deposits
    Deposit,
    /// Set preferred validator for withdraws
    Withdraw,
}

/// Defines which authority to update in the `SetFundingAuthority`
/// instruction
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize, BorshSchema)]
pub enum FundingType {
    /// Sets the stake deposit authority
    StakeDeposit,
    /// Sets the SOL deposit authority
    SolDeposit,
    /// Sets the SOL withdraw authority
    SolWithdraw,
}

/// Instructions supported by the StakePool program.
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum StakePoolInstruction {
    ///   Initializes a new StakePool.
    ///
    ///   0. `[w]` New StakePool to create.
    ///   1. `[s]` Manager
    ///   2. `[]` Staker
    ///   3. `[]` Stake pool withdraw authority
    ///   4. `[w]` Uninitialized validator stake list storage account
    ///   5. `[]` Reserve stake account must be initialized, have zero balance,
    ///      and staker / withdrawer authority set to pool withdraw authority.
    ///   6. `[]` Pool token mint. Must have zero supply, owned by withdraw
    ///      authority.
    ///   7. `[]` Pool account to deposit the generated fee for manager.
    ///   8. `[]` Token program id
    ///   9. `[]` (Optional) Deposit authority that must sign all deposits.
    ///      Defaults to the program address generated using
    ///      `find_deposit_authority_program_address`, making deposits
    ///      permissionless.
    Initialize {
        /// Fee assessed as percentage of perceived rewards
        fee: Fee,
        /// Fee charged per withdrawal as percentage of withdrawal
        withdrawal_fee: Fee,
        /// Fee charged per deposit as percentage of deposit
        deposit_fee: Fee,
        /// Percentage [0-100] of deposit_fee that goes to referrer
        referral_fee: u8,
        /// Maximum expected number of validators
        max_validators: u32,
    },

    ///   (Staker only) Adds stake account delegated to validator to the pool's
    ///   list of managed validators.
    ///
    ///   The stake account will have the rent-exempt amount plus
    ///   `max(
    ///     crate::MINIMUM_ACTIVE_STAKE,
    ///     solana_program::stake::tools::get_minimum_delegation()
    ///   )`.
    ///   It is funded from the stake pool reserve.
    ///
    ///   0. `[w]` Stake pool
    ///   1. `[s]` Staker
    ///   2. `[w]` Reserve stake account
    ///   3. `[]` Stake pool withdraw authority
    ///   4. `[w]` Validator stake list storage account
    ///   5. `[w]` Stake account to add to the pool
    ///   6. `[]` Validator this stake account will be delegated to
    ///   7. `[]` Rent sysvar
    ///   8. `[]` Clock sysvar
    ///   9. '[]' Stake history sysvar
    ///  10. '[]' Stake config sysvar
    ///  11. `[]` System program
    ///  12. `[]` Stake program
    ///
    ///  userdata: optional non-zero u32 seed used for generating the validator
    ///  stake address
    AddValidatorToPool(u32),

    ///   (Staker only) Removes validator from the pool, deactivating its stake
    ///
    ///   Only succeeds if the validator stake account has the minimum of
    ///   `max(crate::MINIMUM_ACTIVE_STAKE,
    /// solana_program::stake::tools::get_minimum_delegation())`.   plus the
    /// rent-exempt amount.
    ///
    ///   0. `[w]` Stake pool
    ///   1. `[s]` Staker
    ///   2. `[]` Stake pool withdraw authority
    ///   3. `[w]` Validator stake list storage account
    ///   4. `[w]` Stake account to remove from the pool
    ///   5. `[w]` Transient stake account, to deactivate if necessary
    ///   6. `[]` Sysvar clock
    ///   7. `[]` Stake program id,
    RemoveValidatorFromPool,

    /// NOTE: This instruction has been deprecated since version 0.7.0. Please
    /// use `DecreaseValidatorStakeWithReserve` instead.
    ///
    /// (Staker only) Decrease active stake on a validator, eventually moving it
    /// to the reserve
    ///
    /// Internally, this instruction splits a validator stake account into its
    /// corresponding transient stake account and deactivates it.
    ///
    /// In order to rebalance the pool without taking custody, the staker needs
    /// a way of reducing the stake on a stake account. This instruction splits
    /// some amount of stake, up to the total activated stake, from the
    /// canonical validator stake account, into its "transient" stake
    /// account.
    ///
    /// The instruction only succeeds if the transient stake account does not
    /// exist. The amount of lamports to move must be at least rent-exemption
    /// plus `max(crate::MINIMUM_ACTIVE_STAKE,
    /// solana_program::stake::tools::get_minimum_delegation())`.
    ///
    ///  0. `[]` Stake pool
    ///  1. `[s]` Stake pool staker
    ///  2. `[]` Stake pool withdraw authority
    ///  3. `[w]` Validator list
    ///  4. `[w]` Canonical stake account to split from
    ///  5. `[w]` Transient stake account to receive split
    ///  6. `[]` Clock sysvar
    ///  7. `[]` Rent sysvar
    ///  8. `[]` System program
    ///  9. `[]` Stake program
    DecreaseValidatorStake {
        /// amount of lamports to split into the transient stake account
        lamports: u64,
        /// seed used to create transient stake account
        transient_stake_seed: u64,
    },

    /// (Staker only) Increase stake on a validator from the reserve account
    ///
    /// Internally, this instruction splits reserve stake into a transient stake
    /// account and delegate to the appropriate validator.
    /// `UpdateValidatorListBalance` will do the work of merging once it's
    /// ready.
    ///
    /// This instruction only succeeds if the transient stake account does not
    /// exist. The minimum amount to move is rent-exemption plus
    /// `max(crate::MINIMUM_ACTIVE_STAKE,
    /// solana_program::stake::tools::get_minimum_delegation())`.
    ///
    ///  0. `[]` Stake pool
    ///  1. `[s]` Stake pool staker
    ///  2. `[]` Stake pool withdraw authority
    ///  3. `[w]` Validator list
    ///  4. `[w]` Stake pool reserve stake
    ///  5. `[w]` Transient stake account
    ///  6. `[]` Validator stake account
    ///  7. `[]` Validator vote account to delegate to
    ///  8. '[]' Clock sysvar
    ///  9. '[]' Rent sysvar
    /// 10. `[]` Stake History sysvar
    /// 11. `[]` Stake Config sysvar
    /// 12. `[]` System program
    /// 13. `[]` Stake program
    ///
    /// userdata: amount of lamports to increase on the given validator.
    ///
    /// The actual amount split into the transient stake account is:
    /// `lamports + stake_rent_exemption`.
    ///
    /// The rent-exemption of the stake account is withdrawn back to the
    /// reserve after it is merged.
    IncreaseValidatorStake {
        /// amount of lamports to increase on the given validator
        lamports: u64,
        /// seed used to create transient stake account
        transient_stake_seed: u64,
    },

    /// (Staker only) Set the preferred deposit or withdraw stake account for
    /// the stake pool
    ///
    /// In order to avoid users abusing the stake pool as a free conversion
    /// between SOL staked on different validators, the staker can force all
    /// deposits and/or withdraws to go to one chosen account, or unset that
    /// account.
    ///
    /// 0. `[w]` Stake pool
    /// 1. `[s]` Stake pool staker
    /// 2. `[]` Validator list
    ///
    /// Fails if the validator is not part of the stake pool.
    SetPreferredValidator {
        /// Affected operation (deposit or withdraw)
        validator_type: PreferredValidatorType,
        /// Validator vote account that deposits or withdraws must go through,
        /// unset with None
        validator_vote_address: Option<Pubkey>,
    },

    ///  Updates balances of validator and transient stake accounts in the pool
    ///
    ///  While going through the pairs of validator and transient stake
    ///  accounts, if the transient stake is inactive, it is merged into the
    ///  reserve stake account. If the transient stake is active and has
    ///  matching credits observed, it is merged into the canonical
    ///  validator stake account. In all other states, nothing is done, and
    ///  the balance is simply added to the canonical stake account balance.
    ///
    ///  0. `[]` Stake pool
    ///  1. `[]` Stake pool withdraw authority
    ///  2. `[w]` Validator stake list storage account
    ///  3. `[w]` Reserve stake account
    ///  4. `[]` Sysvar clock
    ///  5. `[]` Sysvar stake history
    ///  6. `[]` Stake program
    ///  7. ..7+2N ` [] N pairs of validator and transient stake accounts
    UpdateValidatorListBalance {
        /// Index to start updating on the validator list
        start_index: u32,
        /// If true, don't try merging transient stake accounts into the reserve
        /// or validator stake account.  Useful for testing or if a
        /// particular stake account is in a bad state, but we still
        /// want to update
        no_merge: bool,
    },

    ///   Updates total pool balance based on balances in the reserve and
    ///   validator list
    ///
    ///   0. `[w]` Stake pool
    ///   1. `[]` Stake pool withdraw authority
    ///   2. `[w]` Validator stake list storage account
    ///   3. `[]` Reserve stake account
    ///   4. `[w]` Account to receive pool fee tokens
    ///   5. `[w]` Pool mint account
    ///   6. `[]` Pool token program
    UpdateStakePoolBalance,

    ///   Cleans up validator stake account entries marked as `ReadyForRemoval`
    ///
    ///   0. `[]` Stake pool
    ///   1. `[w]` Validator stake list storage account
    CleanupRemovedValidatorEntries,

    ///   Deposit some stake into the pool. The output is a "pool" token
    ///   representing ownership into the pool. Inputs are converted to the
    ///   current ratio.
    ///
    ///   0. `[w]` Stake pool
    ///   1. `[w]` Validator stake list storage account
    ///   2. `[s]/[]` Stake pool deposit authority
    ///   3. `[]` Stake pool withdraw authority
    ///   4. `[w]` Stake account to join the pool (withdraw authority for the
    ///      stake account should be first set to the stake pool deposit
    ///      authority)
    ///   5. `[w]` Validator stake account for the stake account to be merged
    ///      with
    ///   6. `[w]` Reserve stake account, to withdraw rent exempt reserve
    ///   7. `[w]` User account to receive pool tokens
    ///   8. `[w]` Account to receive pool fee tokens
    ///   9. `[w]` Account to receive a portion of pool fee tokens as referral
    ///      fees
    ///   10. `[w]` Pool token mint account
    ///   11. '[]' Sysvar clock account
    ///   12. '[]' Sysvar stake history account
    ///   13. `[]` Pool token program id,
    ///   14. `[]` Stake program id,
    DepositStake,

    ///   Withdraw the token from the pool at the current ratio.
    ///
    ///   Succeeds if the stake account has enough SOL to cover the desired
    ///   amount of pool tokens, and if the withdrawal keeps the total
    ///   staked amount above the minimum of rent-exempt amount + `max(
    ///     crate::MINIMUM_ACTIVE_STAKE,
    ///     solana_program::stake::tools::get_minimum_delegation()
    ///   )`.
    ///
    ///   When allowing withdrawals, the order of priority goes:
    ///
    ///   * preferred withdraw validator stake account (if set)
    ///   * validator stake accounts
    ///   * transient stake accounts
    ///   * reserve stake account OR totally remove validator stake accounts
    ///
    ///   A user can freely withdraw from a validator stake account, and if they
    ///   are all at the minimum, then they can withdraw from transient stake
    ///   accounts, and if they are all at minimum, then they can withdraw from
    ///   the reserve or remove any validator from the pool.
    ///
    ///   0. `[w]` Stake pool
    ///   1. `[w]` Validator stake list storage account
    ///   2. `[]` Stake pool withdraw authority
    ///   3. `[w]` Validator or reserve stake account to split
    ///   4. `[w]` Uninitialized stake account to receive withdrawal
    ///   5. `[]` User account to set as a new withdraw authority
    ///   6. `[s]` User transfer authority, for pool token account
    ///   7. `[w]` User account with pool tokens to burn from
    ///   8. `[w]` Account to receive pool fee tokens
    ///   9. `[w]` Pool token mint account
    ///  10. `[]` Sysvar clock account (required)
    ///  11. `[]` Pool token program id
    ///  12. `[]` Stake program id,
    ///
    ///  userdata: amount of pool tokens to withdraw
    WithdrawStake(u64),

    ///  (Manager only) Update manager
    ///
    ///  0. `[w]` StakePool
    ///  1. `[s]` Manager
    ///  2. `[s]` New manager
    ///  3. `[]` New manager fee account
    SetManager,

    ///  (Manager only) Update fee
    ///
    ///  0. `[w]` StakePool
    ///  1. `[s]` Manager
    SetFee {
        /// Type of fee to update and value to update it to
        fee: FeeType,
    },

    ///  (Manager or staker only) Update staker
    ///
    ///  0. `[w]` StakePool
    ///  1. `[s]` Manager or current staker
    ///  2. '[]` New staker pubkey
    SetStaker,

    ///   Deposit SOL directly into the pool's reserve account. The output is a
    ///   "pool" token representing ownership into the pool. Inputs are
    ///   converted to the current ratio.
    ///
    ///   0. `[w]` Stake pool
    ///   1. `[]` Stake pool withdraw authority
    ///   2. `[w]` Reserve stake account, to deposit SOL
    ///   3. `[s]` Account providing the lamports to be deposited into the pool
    ///   4. `[w]` User account to receive pool tokens
    ///   5. `[w]` Account to receive fee tokens
    ///   6. `[w]` Account to receive a portion of fee as referral fees
    ///   7. `[w]` Pool token mint account
    ///   8. `[]` System program account
    ///   9. `[]` Token program id
    ///  10. `[s]` (Optional) Stake pool sol deposit authority.
    DepositSol(u64),

    ///  (Manager only) Update SOL deposit, stake deposit, or SOL withdrawal
    /// authority.
    ///
    ///  0. `[w]` StakePool
    ///  1. `[s]` Manager
    ///  2. '[]` New authority pubkey or none
    SetFundingAuthority(FundingType),

    ///   Withdraw SOL directly from the pool's reserve account. Fails if the
    ///   reserve does not have enough SOL.
    ///
    ///   0. `[w]` Stake pool
    ///   1. `[]` Stake pool withdraw authority
    ///   2. `[s]` User transfer authority, for pool token account
    ///   3. `[w]` User account to burn pool tokens
    ///   4. `[w]` Reserve stake account, to withdraw SOL
    ///   5. `[w]` Account receiving the lamports from the reserve, must be a
    ///      system account
    ///   6. `[w]` Account to receive pool fee tokens
    ///   7. `[w]` Pool token mint account
    ///   8. '[]' Clock sysvar
    ///   9. '[]' Stake history sysvar
    ///  10. `[]` Stake program account
    ///  11. `[]` Token program id
    ///  12. `[s]` (Optional) Stake pool sol withdraw authority
    WithdrawSol(u64),

    /// Create token metadata for the stake-pool token in the
    /// metaplex-token program
    /// 0. `[]` Stake pool
    /// 1. `[s]` Manager
    /// 2. `[]` Stake pool withdraw authority
    /// 3. `[]` Pool token mint account
    /// 4. `[s, w]` Payer for creation of token metadata account
    /// 5. `[w]` Token metadata account
    /// 6. `[]` Metadata program id
    /// 7. `[]` System program id
    CreateTokenMetadata {
        /// Token name
        name: String,
        /// Token symbol e.g. stkSOL
        symbol: String,
        /// URI of the uploaded metadata of the spl-token
        uri: String,
    },
    /// Update token metadata for the stake-pool token in the
    /// metaplex-token program
    ///
    /// 0. `[]` Stake pool
    /// 1. `[s]` Manager
    /// 2. `[]` Stake pool withdraw authority
    /// 3. `[w]` Token metadata account
    /// 4. `[]` Metadata program id
    UpdateTokenMetadata {
        /// Token name
        name: String,
        /// Token symbol e.g. stkSOL
        symbol: String,
        /// URI of the uploaded metadata of the spl-token
        uri: String,
    },

    /// (Staker only) Increase stake on a validator again in an epoch.
    ///
    /// Works regardless if the transient stake account exists.
    ///
    /// Internally, this instruction splits reserve stake into an ephemeral
    /// stake account, activates it, then merges or splits it into the
    /// transient stake account delegated to the appropriate validator.
    /// `UpdateValidatorListBalance` will do the work of merging once it's
    /// ready.
    ///
    /// The minimum amount to move is rent-exemption plus
    /// `max(crate::MINIMUM_ACTIVE_STAKE,
    /// solana_program::stake::tools::get_minimum_delegation())`.
    ///
    ///  0. `[]` Stake pool
    ///  1. `[s]` Stake pool staker
    ///  2. `[]` Stake pool withdraw authority
    ///  3. `[w]` Validator list
    ///  4. `[w]` Stake pool reserve stake
    ///  5. `[w]` Uninitialized ephemeral stake account to receive stake
    ///  6. `[w]` Transient stake account
    ///  7. `[]` Validator stake account
    ///  8. `[]` Validator vote account to delegate to
    ///  9. '[]' Clock sysvar
    /// 10. `[]` Stake History sysvar
    /// 11. `[]` Stake Config sysvar
    /// 12. `[]` System program
    /// 13. `[]` Stake program
    ///
    /// userdata: amount of lamports to increase on the given validator.
    ///
    /// The actual amount split into the transient stake account is:
    /// `lamports + stake_rent_exemption`.
    ///
    /// The rent-exemption of the stake account is withdrawn back to the
    /// reserve after it is merged.
    IncreaseAdditionalValidatorStake {
        /// amount of lamports to increase on the given validator
        lamports: u64,
        /// seed used to create transient stake account
        transient_stake_seed: u64,
        /// seed used to create ephemeral account.
        ephemeral_stake_seed: u64,
    },

    /// (Staker only) Decrease active stake again from a validator, eventually
    /// moving it to the reserve
    ///
    /// Works regardless if the transient stake account already exists.
    ///
    /// Internally, this instruction:
    ///  * withdraws rent-exempt reserve lamports from the reserve into the
    ///    ephemeral stake
    ///  * splits a validator stake account into an ephemeral stake account
    ///  * deactivates the ephemeral account
    ///  * merges or splits the ephemeral account into the transient stake
    ///    account delegated to the appropriate validator
    ///
    ///  The amount of lamports to move must be at least
    /// `max(crate::MINIMUM_ACTIVE_STAKE,
    /// solana_program::stake::tools::get_minimum_delegation())`.
    ///
    ///  0. `[]` Stake pool
    ///  1. `[s]` Stake pool staker
    ///  2. `[]` Stake pool withdraw authority
    ///  3. `[w]` Validator list
    ///  4. `[w]` Reserve stake account, to fund rent exempt reserve
    ///  5. `[w]` Canonical stake account to split from
    ///  6. `[w]` Uninitialized ephemeral stake account to receive stake
    ///  7. `[w]` Transient stake account
    ///  8. `[]` Clock sysvar
    ///  9. '[]' Stake history sysvar
    /// 10. `[]` System program
    /// 11. `[]` Stake program
    DecreaseAdditionalValidatorStake {
        /// amount of lamports to split into the transient stake account
        lamports: u64,
        /// seed used to create transient stake account
        transient_stake_seed: u64,
        /// seed used to create ephemeral account.
        ephemeral_stake_seed: u64,
    },

    /// (Staker only) Decrease active stake on a validator, eventually moving it
    /// to the reserve
    ///
    /// Internally, this instruction:
    /// * withdraws enough lamports to make the transient account rent-exempt
    /// * splits from a validator stake account into a transient stake account
    /// * deactivates the transient stake account
    ///
    /// In order to rebalance the pool without taking custody, the staker needs
    /// a way of reducing the stake on a stake account. This instruction splits
    /// some amount of stake, up to the total activated stake, from the
    /// canonical validator stake account, into its "transient" stake
    /// account.
    ///
    /// The instruction only succeeds if the transient stake account does not
    /// exist. The amount of lamports to move must be at least rent-exemption
    /// plus `max(crate::MINIMUM_ACTIVE_STAKE,
    /// solana_program::stake::tools::get_minimum_delegation())`.
    ///
    ///  0. `[]` Stake pool
    ///  1. `[s]` Stake pool staker
    ///  2. `[]` Stake pool withdraw authority
    ///  3. `[w]` Validator list
    ///  4. `[w]` Reserve stake account, to fund rent exempt reserve
    ///  5. `[w]` Canonical stake account to split from
    ///  6. `[w]` Transient stake account to receive split
    ///  7. `[]` Clock sysvar
    ///  8. '[]' Stake history sysvar
    ///  9. `[]` System program
    /// 10. `[]` Stake program
    DecreaseValidatorStakeWithReserve {
        /// amount of lamports to split into the transient stake account
        lamports: u64,
        /// seed used to create transient stake account
        transient_stake_seed: u64,
    },

    /// (Staker only) Redelegate active stake on a validator, eventually moving
    /// it to another
    ///
    /// Internally, this instruction splits a validator stake account into its
    /// corresponding transient stake account, redelegates it to an ephemeral
    /// stake account, then merges that stake into the destination transient
    /// stake account.
    ///
    /// In order to rebalance the pool without taking custody, the staker needs
    /// a way of reducing the stake on a stake account. This instruction splits
    /// some amount of stake, up to the total activated stake, from the
    /// canonical validator stake account, into its "transient" stake
    /// account.
    ///
    /// The instruction only succeeds if the source transient stake account and
    /// ephemeral stake account do not exist.
    ///
    /// The amount of lamports to move must be at least rent-exemption plus the
    /// minimum delegation amount. Rent-exemption plus minimum delegation
    /// is required for the destination ephemeral stake account.
    ///
    /// The rent-exemption for the source transient account comes from the stake
    /// pool reserve, if needed.
    ///
    /// The amount that arrives at the destination validator in the end is
    /// `redelegate_lamports - rent_exemption` if the destination transient
    /// account does *not* exist, and `redelegate_lamports` if the destination
    /// transient account already exists. The `rent_exemption` is not activated
    /// when creating the destination transient stake account, but if it already
    /// exists, then the full amount is delegated.
    ///
    ///  0. `[]` Stake pool
    ///  1. `[s]` Stake pool staker
    ///  2. `[]` Stake pool withdraw authority
    ///  3. `[w]` Validator list
    ///  4. `[w]` Reserve stake account, to withdraw rent exempt reserve
    ///  5. `[w]` Source canonical stake account to split from
    ///  6. `[w]` Source transient stake account to receive split and be
    ///     redelegated
    ///  7. `[w]` Uninitialized ephemeral stake account to receive redelegation
    ///  8. `[w]` Destination transient stake account to receive ephemeral stake
    ///     by merge
    ///  9. `[]` Destination stake account to receive transient stake after
    ///     activation
    /// 10. `[]` Destination validator vote account
    /// 11. `[]` Clock sysvar
    /// 12. `[]` Stake History sysvar
    /// 13. `[]` Stake Config sysvar
    /// 14. `[]` System program
    /// 15. `[]` Stake program
    #[deprecated(
        since = "2.0.0",
        note = "The stake redelegate instruction used in this will not be enabled."
    )]
    Redelegate {
        /// Amount of lamports to redelegate
        #[allow(dead_code)] // but it's not
        lamports: u64,
        /// Seed used to create source transient stake account
        #[allow(dead_code)] // but it's not
        source_transient_stake_seed: u64,
        /// Seed used to create destination ephemeral account.
        #[allow(dead_code)] // but it's not
        ephemeral_stake_seed: u64,
        /// Seed used to create destination transient stake account. If there is
        /// already transient stake, this must match the current seed, otherwise
        /// it can be anything
        #[allow(dead_code)] // but it's not
        destination_transient_stake_seed: u64,
    },

    ///   Deposit some stake into the pool, with a specified slippage
    ///   constraint. The output is a "pool" token representing ownership
    ///   into the pool. Inputs are converted at the current ratio.
    ///
    ///   0. `[w]` Stake pool
    ///   1. `[w]` Validator stake list storage account
    ///   2. `[s]/[]` Stake pool deposit authority
    ///   3. `[]` Stake pool withdraw authority
    ///   4. `[w]` Stake account to join the pool (withdraw authority for the
    ///      stake account should be first set to the stake pool deposit
    ///      authority)
    ///   5. `[w]` Validator stake account for the stake account to be merged
    ///      with
    ///   6. `[w]` Reserve stake account, to withdraw rent exempt reserve
    ///   7. `[w]` User account to receive pool tokens
    ///   8. `[w]` Account to receive pool fee tokens
    ///   9. `[w]` Account to receive a portion of pool fee tokens as referral
    ///      fees
    ///   10. `[w]` Pool token mint account
    ///   11. '[]' Sysvar clock account
    ///   12. '[]' Sysvar stake history account
    ///   13. `[]` Pool token program id,
    ///   14. `[]` Stake program id,
    DepositStakeWithSlippage {
        /// Minimum amount of pool tokens that must be received
        minimum_pool_tokens_out: u64,
    },

    ///   Withdraw the token from the pool at the current ratio, specifying a
    ///   minimum expected output lamport amount.
    ///
    ///   Succeeds if the stake account has enough SOL to cover the desired
    ///   amount of pool tokens, and if the withdrawal keeps the total
    ///   staked amount above the minimum of rent-exempt amount + `max(
    ///     crate::MINIMUM_ACTIVE_STAKE,
    ///     solana_program::stake::tools::get_minimum_delegation()
    ///   )`.
    ///
    ///   0. `[w]` Stake pool
    ///   1. `[w]` Validator stake list storage account
    ///   2. `[]` Stake pool withdraw authority
    ///   3. `[w]` Validator or reserve stake account to split
    ///   4. `[w]` Uninitialized stake account to receive withdrawal
    ///   5. `[]` User account to set as a new withdraw authority
    ///   6. `[s]` User transfer authority, for pool token account
    ///   7. `[w]` User account with pool tokens to burn from
    ///   8. `[w]` Account to receive pool fee tokens
    ///   9. `[w]` Pool token mint account
    ///  10. `[]` Sysvar clock account (required)
    ///  11. `[]` Pool token program id
    ///  12. `[]` Stake program id,
    ///
    ///  userdata: amount of pool tokens to withdraw
    WithdrawStakeWithSlippage {
        /// Pool tokens to burn in exchange for lamports
        pool_tokens_in: u64,
        /// Minimum amount of lamports that must be received
        minimum_lamports_out: u64,
    },

    ///   Deposit SOL directly into the pool's reserve account, with a
    ///   specified slippage constraint. The output is a "pool" token
    ///   representing ownership into the pool. Inputs are converted at the
    ///   current ratio.
    ///
    ///   0. `[w]` Stake pool
    ///   1. `[]` Stake pool withdraw authority
    ///   2. `[w]` Reserve stake account, to deposit SOL
    ///   3. `[s]` Account providing the lamports to be deposited into the pool
    ///   4. `[w]` User account to receive pool tokens
    ///   5. `[w]` Account to receive fee tokens
    ///   6. `[w]` Account to receive a portion of fee as referral fees
    ///   7. `[w]` Pool token mint account
    ///   8. `[]` System program account
    ///   9. `[]` Token program id
    ///  10. `[s]` (Optional) Stake pool sol deposit authority.
    DepositSolWithSlippage {
        /// Amount of lamports to deposit into the reserve
        lamports_in: u64,
        /// Minimum amount of pool tokens that must be received
        minimum_pool_tokens_out: u64,
    },

    ///   Withdraw SOL directly from the pool's reserve account. Fails if the
    ///   reserve does not have enough SOL or if the slippage constraint is not
    ///   met.
    ///
    ///   0. `[w]` Stake pool
    ///   1. `[]` Stake pool withdraw authority
    ///   2. `[s]` User transfer authority, for pool token account
    ///   3. `[w]` User account to burn pool tokens
    ///   4. `[w]` Reserve stake account, to withdraw SOL
    ///   5. `[w]` Account receiving the lamports from the reserve, must be a
    ///      system account
    ///   6. `[w]` Account to receive pool fee tokens
    ///   7. `[w]` Pool token mint account
    ///   8. '[]' Clock sysvar
    ///   9. '[]' Stake history sysvar
    ///  10. `[]` Stake program account
    ///  11. `[]` Token program id
    ///  12. `[s]` (Optional) Stake pool sol withdraw authority
    WithdrawSolWithSlippage {
        /// Pool tokens to burn in exchange for lamports
        pool_tokens_in: u64,
        /// Minimum amount of lamports that must be received
        minimum_lamports_out: u64,
    },
}

/// Creates an 'initialize' instruction.
pub fn initialize(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    manager: &Pubkey,
    staker: &Pubkey,
    stake_pool_withdraw_authority: &Pubkey,
    validator_list: &Pubkey,
    reserve_stake: &Pubkey,
    pool_mint: &Pubkey,
    manager_pool_account: &Pubkey,
    token_program_id: &Pubkey,
    deposit_authority: Option<Pubkey>,
    fee: Fee,
    withdrawal_fee: Fee,
    deposit_fee: Fee,
    referral_fee: u8,
    max_validators: u32,
) -> Instruction {
    let init_data = StakePoolInstruction::Initialize {
        fee,
        withdrawal_fee,
        deposit_fee,
        referral_fee,
        max_validators,
    };
    let data = borsh::to_vec(&init_data).unwrap();
    let mut accounts = vec![
        AccountMeta::new(*stake_pool, false),
        AccountMeta::new_readonly(*manager, true),
        AccountMeta::new_readonly(*staker, false),
        AccountMeta::new_readonly(*stake_pool_withdraw_authority, false),
        AccountMeta::new(*validator_list, false),
        AccountMeta::new_readonly(*reserve_stake, false),
        AccountMeta::new(*pool_mint, false),
        AccountMeta::new(*manager_pool_account, false),
        AccountMeta::new_readonly(*token_program_id, false),
    ];
    if let Some(deposit_authority) = deposit_authority {
        accounts.push(AccountMeta::new_readonly(deposit_authority, true));
    }
    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

/// Creates `AddValidatorToPool` instruction (add new validator stake account to
/// the pool)
pub fn add_validator_to_pool(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    staker: &Pubkey,
    reserve: &Pubkey,
    stake_pool_withdraw: &Pubkey,
    validator_list: &Pubkey,
    stake: &Pubkey,
    validator: &Pubkey,
    seed: Option<NonZeroU32>,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*stake_pool, false),
        AccountMeta::new_readonly(*staker, true),
        AccountMeta::new(*reserve, false),
        AccountMeta::new_readonly(*stake_pool_withdraw, false),
        AccountMeta::new(*validator_list, false),
        AccountMeta::new(*stake, false),
        AccountMeta::new_readonly(*validator, false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::stake_history::id(), false),
        #[allow(deprecated)]
        AccountMeta::new_readonly(stake::config::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(stake::program::id(), false),
    ];
    let data = borsh::to_vec(&StakePoolInstruction::AddValidatorToPool(
        seed.map(|s| s.get()).unwrap_or(0),
    ))
    .unwrap();
    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

/// Creates `RemoveValidatorFromPool` instruction (remove validator stake
/// account from the pool)
pub fn remove_validator_from_pool(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    staker: &Pubkey,
    stake_pool_withdraw: &Pubkey,
    validator_list: &Pubkey,
    stake_account: &Pubkey,
    transient_stake_account: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*stake_pool, false),
        AccountMeta::new_readonly(*staker, true),
        AccountMeta::new_readonly(*stake_pool_withdraw, false),
        AccountMeta::new(*validator_list, false),
        AccountMeta::new(*stake_account, false),
        AccountMeta::new(*transient_stake_account, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(stake::program::id(), false),
    ];
    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&StakePoolInstruction::RemoveValidatorFromPool).unwrap(),
    }
}

/// Creates `DecreaseValidatorStake` instruction (rebalance from validator
/// account to transient account)
#[deprecated(
    since = "0.7.0",
    note = "please use `decrease_validator_stake_with_reserve`"
)]
pub fn decrease_validator_stake(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    staker: &Pubkey,
    stake_pool_withdraw_authority: &Pubkey,
    validator_list: &Pubkey,
    validator_stake: &Pubkey,
    transient_stake: &Pubkey,
    lamports: u64,
    transient_stake_seed: u64,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new_readonly(*stake_pool, false),
        AccountMeta::new_readonly(*staker, true),
        AccountMeta::new_readonly(*stake_pool_withdraw_authority, false),
        AccountMeta::new(*validator_list, false),
        AccountMeta::new(*validator_stake, false),
        AccountMeta::new(*transient_stake, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(stake::program::id(), false),
    ];
    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&StakePoolInstruction::DecreaseValidatorStake {
            lamports,
            transient_stake_seed,
        })
        .unwrap(),
    }
}

/// Creates `DecreaseAdditionalValidatorStake` instruction (rebalance from
/// validator account to transient account)
pub fn decrease_additional_validator_stake(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    staker: &Pubkey,
    stake_pool_withdraw_authority: &Pubkey,
    validator_list: &Pubkey,
    reserve_stake: &Pubkey,
    validator_stake: &Pubkey,
    ephemeral_stake: &Pubkey,
    transient_stake: &Pubkey,
    lamports: u64,
    transient_stake_seed: u64,
    ephemeral_stake_seed: u64,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new_readonly(*stake_pool, false),
        AccountMeta::new_readonly(*staker, true),
        AccountMeta::new_readonly(*stake_pool_withdraw_authority, false),
        AccountMeta::new(*validator_list, false),
        AccountMeta::new(*reserve_stake, false),
        AccountMeta::new(*validator_stake, false),
        AccountMeta::new(*ephemeral_stake, false),
        AccountMeta::new(*transient_stake, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::stake_history::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(stake::program::id(), false),
    ];
    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&StakePoolInstruction::DecreaseAdditionalValidatorStake {
            lamports,
            transient_stake_seed,
            ephemeral_stake_seed,
        })
        .unwrap(),
    }
}

/// Creates `DecreaseValidatorStakeWithReserve` instruction (rebalance from
/// validator account to transient account)
pub fn decrease_validator_stake_with_reserve(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    staker: &Pubkey,
    stake_pool_withdraw_authority: &Pubkey,
    validator_list: &Pubkey,
    reserve_stake: &Pubkey,
    validator_stake: &Pubkey,
    transient_stake: &Pubkey,
    lamports: u64,
    transient_stake_seed: u64,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new_readonly(*stake_pool, false),
        AccountMeta::new_readonly(*staker, true),
        AccountMeta::new_readonly(*stake_pool_withdraw_authority, false),
        AccountMeta::new(*validator_list, false),
        AccountMeta::new(*reserve_stake, false),
        AccountMeta::new(*validator_stake, false),
        AccountMeta::new(*transient_stake, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::stake_history::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(stake::program::id(), false),
    ];
    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&StakePoolInstruction::DecreaseValidatorStakeWithReserve {
            lamports,
            transient_stake_seed,
        })
        .unwrap(),
    }
}

/// Creates `IncreaseValidatorStake` instruction (rebalance from reserve account
/// to transient account)
pub fn increase_validator_stake(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    staker: &Pubkey,
    stake_pool_withdraw_authority: &Pubkey,
    validator_list: &Pubkey,
    reserve_stake: &Pubkey,
    transient_stake: &Pubkey,
    validator_stake: &Pubkey,
    validator: &Pubkey,
    lamports: u64,
    transient_stake_seed: u64,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new_readonly(*stake_pool, false),
        AccountMeta::new_readonly(*staker, true),
        AccountMeta::new_readonly(*stake_pool_withdraw_authority, false),
        AccountMeta::new(*validator_list, false),
        AccountMeta::new(*reserve_stake, false),
        AccountMeta::new(*transient_stake, false),
        AccountMeta::new_readonly(*validator_stake, false),
        AccountMeta::new_readonly(*validator, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(sysvar::stake_history::id(), false),
        #[allow(deprecated)]
        AccountMeta::new_readonly(stake::config::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(stake::program::id(), false),
    ];
    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&StakePoolInstruction::IncreaseValidatorStake {
            lamports,
            transient_stake_seed,
        })
        .unwrap(),
    }
}

/// Creates `IncreaseAdditionalValidatorStake` instruction (rebalance from
/// reserve account to transient account)
pub fn increase_additional_validator_stake(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    staker: &Pubkey,
    stake_pool_withdraw_authority: &Pubkey,
    validator_list: &Pubkey,
    reserve_stake: &Pubkey,
    ephemeral_stake: &Pubkey,
    transient_stake: &Pubkey,
    validator_stake: &Pubkey,
    validator: &Pubkey,
    lamports: u64,
    transient_stake_seed: u64,
    ephemeral_stake_seed: u64,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new_readonly(*stake_pool, false),
        AccountMeta::new_readonly(*staker, true),
        AccountMeta::new_readonly(*stake_pool_withdraw_authority, false),
        AccountMeta::new(*validator_list, false),
        AccountMeta::new(*reserve_stake, false),
        AccountMeta::new(*ephemeral_stake, false),
        AccountMeta::new(*transient_stake, false),
        AccountMeta::new_readonly(*validator_stake, false),
        AccountMeta::new_readonly(*validator, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::stake_history::id(), false),
        #[allow(deprecated)]
        AccountMeta::new_readonly(stake::config::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(stake::program::id(), false),
    ];
    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&StakePoolInstruction::IncreaseAdditionalValidatorStake {
            lamports,
            transient_stake_seed,
            ephemeral_stake_seed,
        })
        .unwrap(),
    }
}

/// Creates `Redelegate` instruction (rebalance from one validator account to
/// another)
#[deprecated(
    since = "2.0.0",
    note = "The stake redelegate instruction used in this will not be enabled."
)]
pub fn redelegate(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    staker: &Pubkey,
    stake_pool_withdraw_authority: &Pubkey,
    validator_list: &Pubkey,
    reserve_stake: &Pubkey,
    source_validator_stake: &Pubkey,
    source_transient_stake: &Pubkey,
    ephemeral_stake: &Pubkey,
    destination_transient_stake: &Pubkey,
    destination_validator_stake: &Pubkey,
    validator: &Pubkey,
    lamports: u64,
    source_transient_stake_seed: u64,
    ephemeral_stake_seed: u64,
    destination_transient_stake_seed: u64,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new_readonly(*stake_pool, false),
        AccountMeta::new_readonly(*staker, true),
        AccountMeta::new_readonly(*stake_pool_withdraw_authority, false),
        AccountMeta::new(*validator_list, false),
        AccountMeta::new(*reserve_stake, false),
        AccountMeta::new(*source_validator_stake, false),
        AccountMeta::new(*source_transient_stake, false),
        AccountMeta::new(*ephemeral_stake, false),
        AccountMeta::new(*destination_transient_stake, false),
        AccountMeta::new_readonly(*destination_validator_stake, false),
        AccountMeta::new_readonly(*validator, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::stake_history::id(), false),
        #[allow(deprecated)]
        AccountMeta::new_readonly(stake::config::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(stake::program::id(), false),
    ];
    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&StakePoolInstruction::Redelegate {
            lamports,
            source_transient_stake_seed,
            ephemeral_stake_seed,
            destination_transient_stake_seed,
        })
        .unwrap(),
    }
}

/// Creates `SetPreferredDepositValidator` instruction
pub fn set_preferred_validator(
    program_id: &Pubkey,
    stake_pool_address: &Pubkey,
    staker: &Pubkey,
    validator_list_address: &Pubkey,
    validator_type: PreferredValidatorType,
    validator_vote_address: Option<Pubkey>,
) -> Instruction {
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*stake_pool_address, false),
            AccountMeta::new_readonly(*staker, true),
            AccountMeta::new_readonly(*validator_list_address, false),
        ],
        data: borsh::to_vec(&StakePoolInstruction::SetPreferredValidator {
            validator_type,
            validator_vote_address,
        })
        .unwrap(),
    }
}

/// Create an `AddValidatorToPool` instruction given an existing stake pool and
/// vote account
pub fn add_validator_to_pool_with_vote(
    program_id: &Pubkey,
    stake_pool: &StakePool,
    stake_pool_address: &Pubkey,
    vote_account_address: &Pubkey,
    seed: Option<NonZeroU32>,
) -> Instruction {
    let pool_withdraw_authority =
        find_withdraw_authority_program_address(program_id, stake_pool_address).0;
    let (stake_account_address, _) =
        find_stake_program_address(program_id, vote_account_address, stake_pool_address, seed);
    add_validator_to_pool(
        program_id,
        stake_pool_address,
        &stake_pool.staker,
        &stake_pool.reserve_stake,
        &pool_withdraw_authority,
        &stake_pool.validator_list,
        &stake_account_address,
        vote_account_address,
        seed,
    )
}

/// Create an `RemoveValidatorFromPool` instruction given an existing stake pool
/// and vote account
pub fn remove_validator_from_pool_with_vote(
    program_id: &Pubkey,
    stake_pool: &StakePool,
    stake_pool_address: &Pubkey,
    vote_account_address: &Pubkey,
    validator_stake_seed: Option<NonZeroU32>,
    transient_stake_seed: u64,
) -> Instruction {
    let pool_withdraw_authority =
        find_withdraw_authority_program_address(program_id, stake_pool_address).0;
    let (stake_account_address, _) = find_stake_program_address(
        program_id,
        vote_account_address,
        stake_pool_address,
        validator_stake_seed,
    );
    let (transient_stake_account, _) = find_transient_stake_program_address(
        program_id,
        vote_account_address,
        stake_pool_address,
        transient_stake_seed,
    );
    remove_validator_from_pool(
        program_id,
        stake_pool_address,
        &stake_pool.staker,
        &pool_withdraw_authority,
        &stake_pool.validator_list,
        &stake_account_address,
        &transient_stake_account,
    )
}

/// Create an `IncreaseValidatorStake` instruction given an existing stake pool
/// and vote account
pub fn increase_validator_stake_with_vote(
    program_id: &Pubkey,
    stake_pool: &StakePool,
    stake_pool_address: &Pubkey,
    vote_account_address: &Pubkey,
    lamports: u64,
    validator_stake_seed: Option<NonZeroU32>,
    transient_stake_seed: u64,
) -> Instruction {
    let pool_withdraw_authority =
        find_withdraw_authority_program_address(program_id, stake_pool_address).0;
    let (transient_stake_address, _) = find_transient_stake_program_address(
        program_id,
        vote_account_address,
        stake_pool_address,
        transient_stake_seed,
    );
    let (validator_stake_address, _) = find_stake_program_address(
        program_id,
        vote_account_address,
        stake_pool_address,
        validator_stake_seed,
    );

    increase_validator_stake(
        program_id,
        stake_pool_address,
        &stake_pool.staker,
        &pool_withdraw_authority,
        &stake_pool.validator_list,
        &stake_pool.reserve_stake,
        &transient_stake_address,
        &validator_stake_address,
        vote_account_address,
        lamports,
        transient_stake_seed,
    )
}

/// Create an `IncreaseAdditionalValidatorStake` instruction given an existing
/// stake pool and vote account
pub fn increase_additional_validator_stake_with_vote(
    program_id: &Pubkey,
    stake_pool: &StakePool,
    stake_pool_address: &Pubkey,
    vote_account_address: &Pubkey,
    lamports: u64,
    validator_stake_seed: Option<NonZeroU32>,
    transient_stake_seed: u64,
    ephemeral_stake_seed: u64,
) -> Instruction {
    let pool_withdraw_authority =
        find_withdraw_authority_program_address(program_id, stake_pool_address).0;
    let (ephemeral_stake_address, _) =
        find_ephemeral_stake_program_address(program_id, stake_pool_address, ephemeral_stake_seed);
    let (transient_stake_address, _) = find_transient_stake_program_address(
        program_id,
        vote_account_address,
        stake_pool_address,
        transient_stake_seed,
    );
    let (validator_stake_address, _) = find_stake_program_address(
        program_id,
        vote_account_address,
        stake_pool_address,
        validator_stake_seed,
    );

    increase_additional_validator_stake(
        program_id,
        stake_pool_address,
        &stake_pool.staker,
        &pool_withdraw_authority,
        &stake_pool.validator_list,
        &stake_pool.reserve_stake,
        &ephemeral_stake_address,
        &transient_stake_address,
        &validator_stake_address,
        vote_account_address,
        lamports,
        transient_stake_seed,
        ephemeral_stake_seed,
    )
}

/// Create a `DecreaseValidatorStake` instruction given an existing stake pool
/// and vote account
pub fn decrease_validator_stake_with_vote(
    program_id: &Pubkey,
    stake_pool: &StakePool,
    stake_pool_address: &Pubkey,
    vote_account_address: &Pubkey,
    lamports: u64,
    validator_stake_seed: Option<NonZeroU32>,
    transient_stake_seed: u64,
) -> Instruction {
    let pool_withdraw_authority =
        find_withdraw_authority_program_address(program_id, stake_pool_address).0;
    let (validator_stake_address, _) = find_stake_program_address(
        program_id,
        vote_account_address,
        stake_pool_address,
        validator_stake_seed,
    );
    let (transient_stake_address, _) = find_transient_stake_program_address(
        program_id,
        vote_account_address,
        stake_pool_address,
        transient_stake_seed,
    );
    decrease_validator_stake_with_reserve(
        program_id,
        stake_pool_address,
        &stake_pool.staker,
        &pool_withdraw_authority,
        &stake_pool.validator_list,
        &stake_pool.reserve_stake,
        &validator_stake_address,
        &transient_stake_address,
        lamports,
        transient_stake_seed,
    )
}

/// Create a `IncreaseAdditionalValidatorStake` instruction given an existing
/// stake pool, valiator list and vote account
pub fn increase_additional_validator_stake_with_list(
    program_id: &Pubkey,
    stake_pool: &StakePool,
    validator_list: &ValidatorList,
    stake_pool_address: &Pubkey,
    vote_account_address: &Pubkey,
    lamports: u64,
    ephemeral_stake_seed: u64,
) -> Result<Instruction, ProgramError> {
    let validator_info = validator_list
        .find(vote_account_address)
        .ok_or(ProgramError::InvalidInstructionData)?;
    let transient_stake_seed = u64::from(validator_info.transient_seed_suffix);
    let validator_stake_seed = NonZeroU32::new(validator_info.validator_seed_suffix.into());
    Ok(increase_additional_validator_stake_with_vote(
        program_id,
        stake_pool,
        stake_pool_address,
        vote_account_address,
        lamports,
        validator_stake_seed,
        transient_stake_seed,
        ephemeral_stake_seed,
    ))
}

/// Create a `DecreaseAdditionalValidatorStake` instruction given an existing
/// stake pool, valiator list and vote account
pub fn decrease_additional_validator_stake_with_list(
    program_id: &Pubkey,
    stake_pool: &StakePool,
    validator_list: &ValidatorList,
    stake_pool_address: &Pubkey,
    vote_account_address: &Pubkey,
    lamports: u64,
    ephemeral_stake_seed: u64,
) -> Result<Instruction, ProgramError> {
    let validator_info = validator_list
        .find(vote_account_address)
        .ok_or(ProgramError::InvalidInstructionData)?;
    let transient_stake_seed = u64::from(validator_info.transient_seed_suffix);
    let validator_stake_seed = NonZeroU32::new(validator_info.validator_seed_suffix.into());
    Ok(decrease_additional_validator_stake_with_vote(
        program_id,
        stake_pool,
        stake_pool_address,
        vote_account_address,
        lamports,
        validator_stake_seed,
        transient_stake_seed,
        ephemeral_stake_seed,
    ))
}

/// Create a `DecreaseAdditionalValidatorStake` instruction given an existing
/// stake pool and vote account
pub fn decrease_additional_validator_stake_with_vote(
    program_id: &Pubkey,
    stake_pool: &StakePool,
    stake_pool_address: &Pubkey,
    vote_account_address: &Pubkey,
    lamports: u64,
    validator_stake_seed: Option<NonZeroU32>,
    transient_stake_seed: u64,
    ephemeral_stake_seed: u64,
) -> Instruction {
    let pool_withdraw_authority =
        find_withdraw_authority_program_address(program_id, stake_pool_address).0;
    let (validator_stake_address, _) = find_stake_program_address(
        program_id,
        vote_account_address,
        stake_pool_address,
        validator_stake_seed,
    );
    let (ephemeral_stake_address, _) =
        find_ephemeral_stake_program_address(program_id, stake_pool_address, ephemeral_stake_seed);
    let (transient_stake_address, _) = find_transient_stake_program_address(
        program_id,
        vote_account_address,
        stake_pool_address,
        transient_stake_seed,
    );
    decrease_additional_validator_stake(
        program_id,
        stake_pool_address,
        &stake_pool.staker,
        &pool_withdraw_authority,
        &stake_pool.validator_list,
        &stake_pool.reserve_stake,
        &validator_stake_address,
        &ephemeral_stake_address,
        &transient_stake_address,
        lamports,
        transient_stake_seed,
        ephemeral_stake_seed,
    )
}

/// Creates `UpdateValidatorListBalance` instruction (update validator stake
/// account balances)
#[deprecated(
    since = "1.1.0",
    note = "please use `update_validator_list_balance_chunk`"
)]
pub fn update_validator_list_balance(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    stake_pool_withdraw_authority: &Pubkey,
    validator_list_address: &Pubkey,
    reserve_stake: &Pubkey,
    validator_list: &ValidatorList,
    validator_vote_accounts: &[Pubkey],
    start_index: u32,
    no_merge: bool,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new_readonly(*stake_pool, false),
        AccountMeta::new_readonly(*stake_pool_withdraw_authority, false),
        AccountMeta::new(*validator_list_address, false),
        AccountMeta::new(*reserve_stake, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::stake_history::id(), false),
        AccountMeta::new_readonly(stake::program::id(), false),
    ];
    accounts.append(
        &mut validator_vote_accounts
            .iter()
            .flat_map(|vote_account_address| {
                let validator_stake_info = validator_list.find(vote_account_address);
                if let Some(validator_stake_info) = validator_stake_info {
                    let (validator_stake_account, _) = find_stake_program_address(
                        program_id,
                        vote_account_address,
                        stake_pool,
                        NonZeroU32::new(validator_stake_info.validator_seed_suffix.into()),
                    );
                    let (transient_stake_account, _) = find_transient_stake_program_address(
                        program_id,
                        vote_account_address,
                        stake_pool,
                        validator_stake_info.transient_seed_suffix.into(),
                    );
                    vec![
                        AccountMeta::new(validator_stake_account, false),
                        AccountMeta::new(transient_stake_account, false),
                    ]
                } else {
                    vec![]
                }
            })
            .collect::<Vec<AccountMeta>>(),
    );
    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&StakePoolInstruction::UpdateValidatorListBalance {
            start_index,
            no_merge,
        })
        .unwrap(),
    }
}

/// Creates an `UpdateValidatorListBalance` instruction (update validator stake
/// account balances) to update `validator_list[start_index..start_index +
/// len]`.
///
/// Returns `Err(ProgramError::InvalidInstructionData)` if:
/// - `start_index..start_index + len` is out of bounds for
///   `validator_list.validators`
pub fn update_validator_list_balance_chunk(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    stake_pool_withdraw_authority: &Pubkey,
    validator_list_address: &Pubkey,
    reserve_stake: &Pubkey,
    validator_list: &ValidatorList,
    len: usize,
    start_index: usize,
    no_merge: bool,
) -> Result<Instruction, ProgramError> {
    let mut accounts = vec![
        AccountMeta::new_readonly(*stake_pool, false),
        AccountMeta::new_readonly(*stake_pool_withdraw_authority, false),
        AccountMeta::new(*validator_list_address, false),
        AccountMeta::new(*reserve_stake, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::stake_history::id(), false),
        AccountMeta::new_readonly(stake::program::id(), false),
    ];
    let validator_list_subslice = validator_list
        .validators
        .get(start_index..start_index.saturating_add(len))
        .ok_or(ProgramError::InvalidInstructionData)?;
    accounts.extend(validator_list_subslice.iter().flat_map(
        |ValidatorStakeInfo {
             vote_account_address,
             validator_seed_suffix,
             transient_seed_suffix,
             ..
         }| {
            let (validator_stake_account, _) = find_stake_program_address(
                program_id,
                vote_account_address,
                stake_pool,
                NonZeroU32::new((*validator_seed_suffix).into()),
            );
            let (transient_stake_account, _) = find_transient_stake_program_address(
                program_id,
                vote_account_address,
                stake_pool,
                (*transient_seed_suffix).into(),
            );
            [
                AccountMeta::new(validator_stake_account, false),
                AccountMeta::new(transient_stake_account, false),
            ]
        },
    ));
    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&StakePoolInstruction::UpdateValidatorListBalance {
            start_index: start_index.try_into().unwrap(),
            no_merge,
        })
        .unwrap(),
    })
}

/// Creates `UpdateValidatorListBalance` instruction (update validator stake
/// account balances)
///
/// Returns `None` if all validators in the given chunk has already been updated
/// for this epoch, returns the required instruction otherwise.
pub fn update_stale_validator_list_balance_chunk(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    stake_pool_withdraw_authority: &Pubkey,
    validator_list_address: &Pubkey,
    reserve_stake: &Pubkey,
    validator_list: &ValidatorList,
    len: usize,
    start_index: usize,
    no_merge: bool,
    current_epoch: Epoch,
) -> Result<Option<Instruction>, ProgramError> {
    let validator_list_subslice = validator_list
        .validators
        .get(start_index..start_index.saturating_add(len))
        .ok_or(ProgramError::InvalidInstructionData)?;
    if validator_list_subslice.iter().all(|info| {
        let last_update_epoch: u64 = info.last_update_epoch.into();
        last_update_epoch >= current_epoch
    }) {
        return Ok(None);
    }
    update_validator_list_balance_chunk(
        program_id,
        stake_pool,
        stake_pool_withdraw_authority,
        validator_list_address,
        reserve_stake,
        validator_list,
        len,
        start_index,
        no_merge,
    )
    .map(Some)
}

/// Creates `UpdateStakePoolBalance` instruction (pool balance from the stake
/// account list balances)
pub fn update_stake_pool_balance(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    withdraw_authority: &Pubkey,
    validator_list_storage: &Pubkey,
    reserve_stake: &Pubkey,
    manager_fee_account: &Pubkey,
    stake_pool_mint: &Pubkey,
    token_program_id: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*stake_pool, false),
        AccountMeta::new_readonly(*withdraw_authority, false),
        AccountMeta::new(*validator_list_storage, false),
        AccountMeta::new_readonly(*reserve_stake, false),
        AccountMeta::new(*manager_fee_account, false),
        AccountMeta::new(*stake_pool_mint, false),
        AccountMeta::new_readonly(*token_program_id, false),
    ];
    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&StakePoolInstruction::UpdateStakePoolBalance).unwrap(),
    }
}

/// Creates `CleanupRemovedValidatorEntries` instruction (removes entries from
/// the validator list)
pub fn cleanup_removed_validator_entries(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    validator_list_storage: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new_readonly(*stake_pool, false),
        AccountMeta::new(*validator_list_storage, false),
    ];
    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&StakePoolInstruction::CleanupRemovedValidatorEntries).unwrap(),
    }
}

/// Creates all `UpdateValidatorListBalance` and `UpdateStakePoolBalance`
/// instructions for fully updating a stake pool each epoch
pub fn update_stake_pool(
    program_id: &Pubkey,
    stake_pool: &StakePool,
    validator_list: &ValidatorList,
    stake_pool_address: &Pubkey,
    no_merge: bool,
) -> (Vec<Instruction>, Vec<Instruction>) {
    let (withdraw_authority, _) =
        find_withdraw_authority_program_address(program_id, stake_pool_address);

    let update_list_instructions = validator_list
        .validators
        .chunks(MAX_VALIDATORS_TO_UPDATE)
        .enumerate()
        .map(|(i, chunk)| {
            // unwrap-safety: chunk len and offset are derived
            update_validator_list_balance_chunk(
                program_id,
                stake_pool_address,
                &withdraw_authority,
                &stake_pool.validator_list,
                &stake_pool.reserve_stake,
                validator_list,
                chunk.len(),
                i.saturating_mul(MAX_VALIDATORS_TO_UPDATE),
                no_merge,
            )
            .unwrap()
        })
        .collect();

    let final_instructions = vec![
        update_stake_pool_balance(
            program_id,
            stake_pool_address,
            &withdraw_authority,
            &stake_pool.validator_list,
            &stake_pool.reserve_stake,
            &stake_pool.manager_fee_account,
            &stake_pool.pool_mint,
            &stake_pool.token_program_id,
        ),
        cleanup_removed_validator_entries(
            program_id,
            stake_pool_address,
            &stake_pool.validator_list,
        ),
    ];
    (update_list_instructions, final_instructions)
}

/// Creates the `UpdateValidatorListBalance` instructions only for validators on
/// `validator_list` that have not been updated for this epoch, and the
/// `UpdateStakePoolBalance` instruction for fully updating the stake pool.
///
/// Basically same as [`update_stake_pool`], but skips validators that are
/// already updated for this epoch
pub fn update_stale_stake_pool(
    program_id: &Pubkey,
    stake_pool: &StakePool,
    validator_list: &ValidatorList,
    stake_pool_address: &Pubkey,
    no_merge: bool,
    current_epoch: Epoch,
) -> (Vec<Instruction>, Vec<Instruction>) {
    let (withdraw_authority, _) =
        find_withdraw_authority_program_address(program_id, stake_pool_address);

    let update_list_instructions = validator_list
        .validators
        .chunks(MAX_VALIDATORS_TO_UPDATE)
        .enumerate()
        .filter_map(|(i, chunk)| {
            // unwrap-safety: chunk len and offset are derived
            update_stale_validator_list_balance_chunk(
                program_id,
                stake_pool_address,
                &withdraw_authority,
                &stake_pool.validator_list,
                &stake_pool.reserve_stake,
                validator_list,
                chunk.len(),
                i.saturating_mul(MAX_VALIDATORS_TO_UPDATE),
                no_merge,
                current_epoch,
            )
            .unwrap()
        })
        .collect();

    let final_instructions = vec![
        update_stake_pool_balance(
            program_id,
            stake_pool_address,
            &withdraw_authority,
            &stake_pool.validator_list,
            &stake_pool.reserve_stake,
            &stake_pool.manager_fee_account,
            &stake_pool.pool_mint,
            &stake_pool.token_program_id,
        ),
        cleanup_removed_validator_entries(
            program_id,
            stake_pool_address,
            &stake_pool.validator_list,
        ),
    ];
    (update_list_instructions, final_instructions)
}

fn deposit_stake_internal(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    validator_list_storage: &Pubkey,
    stake_pool_deposit_authority: Option<&Pubkey>,
    stake_pool_withdraw_authority: &Pubkey,
    deposit_stake_address: &Pubkey,
    deposit_stake_withdraw_authority: &Pubkey,
    validator_stake_account: &Pubkey,
    reserve_stake_account: &Pubkey,
    pool_tokens_to: &Pubkey,
    manager_fee_account: &Pubkey,
    referrer_pool_tokens_account: &Pubkey,
    pool_mint: &Pubkey,
    token_program_id: &Pubkey,
    minimum_pool_tokens_out: Option<u64>,
) -> Vec<Instruction> {
    let mut instructions = vec![];
    let mut accounts = vec![
        AccountMeta::new(*stake_pool, false),
        AccountMeta::new(*validator_list_storage, false),
    ];
    if let Some(stake_pool_deposit_authority) = stake_pool_deposit_authority {
        accounts.push(AccountMeta::new_readonly(
            *stake_pool_deposit_authority,
            true,
        ));
        instructions.extend_from_slice(&[
            stake::instruction::authorize(
                deposit_stake_address,
                deposit_stake_withdraw_authority,
                stake_pool_deposit_authority,
                stake::state::StakeAuthorize::Staker,
                None,
            ),
            stake::instruction::authorize(
                deposit_stake_address,
                deposit_stake_withdraw_authority,
                stake_pool_deposit_authority,
                stake::state::StakeAuthorize::Withdrawer,
                None,
            ),
        ]);
    } else {
        let stake_pool_deposit_authority =
            find_deposit_authority_program_address(program_id, stake_pool).0;
        accounts.push(AccountMeta::new_readonly(
            stake_pool_deposit_authority,
            false,
        ));
        instructions.extend_from_slice(&[
            stake::instruction::authorize(
                deposit_stake_address,
                deposit_stake_withdraw_authority,
                &stake_pool_deposit_authority,
                stake::state::StakeAuthorize::Staker,
                None,
            ),
            stake::instruction::authorize(
                deposit_stake_address,
                deposit_stake_withdraw_authority,
                &stake_pool_deposit_authority,
                stake::state::StakeAuthorize::Withdrawer,
                None,
            ),
        ]);
    };

    accounts.extend_from_slice(&[
        AccountMeta::new_readonly(*stake_pool_withdraw_authority, false),
        AccountMeta::new(*deposit_stake_address, false),
        AccountMeta::new(*validator_stake_account, false),
        AccountMeta::new(*reserve_stake_account, false),
        AccountMeta::new(*pool_tokens_to, false),
        AccountMeta::new(*manager_fee_account, false),
        AccountMeta::new(*referrer_pool_tokens_account, false),
        AccountMeta::new(*pool_mint, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::stake_history::id(), false),
        AccountMeta::new_readonly(*token_program_id, false),
        AccountMeta::new_readonly(stake::program::id(), false),
    ]);
    instructions.push(
        if let Some(minimum_pool_tokens_out) = minimum_pool_tokens_out {
            Instruction {
                program_id: *program_id,
                accounts,
                data: borsh::to_vec(&StakePoolInstruction::DepositStakeWithSlippage {
                    minimum_pool_tokens_out,
                })
                .unwrap(),
            }
        } else {
            Instruction {
                program_id: *program_id,
                accounts,
                data: borsh::to_vec(&StakePoolInstruction::DepositStake).unwrap(),
            }
        },
    );
    instructions
}

/// Creates instructions required to deposit into a stake pool, given a stake
/// account owned by the user.
pub fn deposit_stake(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    validator_list_storage: &Pubkey,
    stake_pool_withdraw_authority: &Pubkey,
    deposit_stake_address: &Pubkey,
    deposit_stake_withdraw_authority: &Pubkey,
    validator_stake_account: &Pubkey,
    reserve_stake_account: &Pubkey,
    pool_tokens_to: &Pubkey,
    manager_fee_account: &Pubkey,
    referrer_pool_tokens_account: &Pubkey,
    pool_mint: &Pubkey,
    token_program_id: &Pubkey,
) -> Vec<Instruction> {
    deposit_stake_internal(
        program_id,
        stake_pool,
        validator_list_storage,
        None,
        stake_pool_withdraw_authority,
        deposit_stake_address,
        deposit_stake_withdraw_authority,
        validator_stake_account,
        reserve_stake_account,
        pool_tokens_to,
        manager_fee_account,
        referrer_pool_tokens_account,
        pool_mint,
        token_program_id,
        None,
    )
}

/// Creates instructions to deposit into a stake pool with slippage
pub fn deposit_stake_with_slippage(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    validator_list_storage: &Pubkey,
    stake_pool_withdraw_authority: &Pubkey,
    deposit_stake_address: &Pubkey,
    deposit_stake_withdraw_authority: &Pubkey,
    validator_stake_account: &Pubkey,
    reserve_stake_account: &Pubkey,
    pool_tokens_to: &Pubkey,
    manager_fee_account: &Pubkey,
    referrer_pool_tokens_account: &Pubkey,
    pool_mint: &Pubkey,
    token_program_id: &Pubkey,
    minimum_pool_tokens_out: u64,
) -> Vec<Instruction> {
    deposit_stake_internal(
        program_id,
        stake_pool,
        validator_list_storage,
        None,
        stake_pool_withdraw_authority,
        deposit_stake_address,
        deposit_stake_withdraw_authority,
        validator_stake_account,
        reserve_stake_account,
        pool_tokens_to,
        manager_fee_account,
        referrer_pool_tokens_account,
        pool_mint,
        token_program_id,
        Some(minimum_pool_tokens_out),
    )
}

/// Creates instructions required to deposit into a stake pool, given a stake
/// account owned by the user. The difference with `deposit()` is that a deposit
/// authority must sign this instruction, which is required for private pools.
pub fn deposit_stake_with_authority(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    validator_list_storage: &Pubkey,
    stake_pool_deposit_authority: &Pubkey,
    stake_pool_withdraw_authority: &Pubkey,
    deposit_stake_address: &Pubkey,
    deposit_stake_withdraw_authority: &Pubkey,
    validator_stake_account: &Pubkey,
    reserve_stake_account: &Pubkey,
    pool_tokens_to: &Pubkey,
    manager_fee_account: &Pubkey,
    referrer_pool_tokens_account: &Pubkey,
    pool_mint: &Pubkey,
    token_program_id: &Pubkey,
) -> Vec<Instruction> {
    deposit_stake_internal(
        program_id,
        stake_pool,
        validator_list_storage,
        Some(stake_pool_deposit_authority),
        stake_pool_withdraw_authority,
        deposit_stake_address,
        deposit_stake_withdraw_authority,
        validator_stake_account,
        reserve_stake_account,
        pool_tokens_to,
        manager_fee_account,
        referrer_pool_tokens_account,
        pool_mint,
        token_program_id,
        None,
    )
}

/// Creates instructions required to deposit into a stake pool with slippage,
/// given a stake account owned by the user. The difference with `deposit()` is
/// that a deposit authority must sign this instruction, which is required for
/// private pools.
pub fn deposit_stake_with_authority_and_slippage(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    validator_list_storage: &Pubkey,
    stake_pool_deposit_authority: &Pubkey,
    stake_pool_withdraw_authority: &Pubkey,
    deposit_stake_address: &Pubkey,
    deposit_stake_withdraw_authority: &Pubkey,
    validator_stake_account: &Pubkey,
    reserve_stake_account: &Pubkey,
    pool_tokens_to: &Pubkey,
    manager_fee_account: &Pubkey,
    referrer_pool_tokens_account: &Pubkey,
    pool_mint: &Pubkey,
    token_program_id: &Pubkey,
    minimum_pool_tokens_out: u64,
) -> Vec<Instruction> {
    deposit_stake_internal(
        program_id,
        stake_pool,
        validator_list_storage,
        Some(stake_pool_deposit_authority),
        stake_pool_withdraw_authority,
        deposit_stake_address,
        deposit_stake_withdraw_authority,
        validator_stake_account,
        reserve_stake_account,
        pool_tokens_to,
        manager_fee_account,
        referrer_pool_tokens_account,
        pool_mint,
        token_program_id,
        Some(minimum_pool_tokens_out),
    )
}

/// Creates instructions required to deposit SOL directly into a stake pool.
fn deposit_sol_internal(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    stake_pool_withdraw_authority: &Pubkey,
    reserve_stake_account: &Pubkey,
    lamports_from: &Pubkey,
    pool_tokens_to: &Pubkey,
    manager_fee_account: &Pubkey,
    referrer_pool_tokens_account: &Pubkey,
    pool_mint: &Pubkey,
    token_program_id: &Pubkey,
    sol_deposit_authority: Option<&Pubkey>,
    lamports_in: u64,
    minimum_pool_tokens_out: Option<u64>,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new(*stake_pool, false),
        AccountMeta::new_readonly(*stake_pool_withdraw_authority, false),
        AccountMeta::new(*reserve_stake_account, false),
        AccountMeta::new(*lamports_from, true),
        AccountMeta::new(*pool_tokens_to, false),
        AccountMeta::new(*manager_fee_account, false),
        AccountMeta::new(*referrer_pool_tokens_account, false),
        AccountMeta::new(*pool_mint, false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(*token_program_id, false),
    ];
    if let Some(sol_deposit_authority) = sol_deposit_authority {
        accounts.push(AccountMeta::new_readonly(*sol_deposit_authority, true));
    }
    if let Some(minimum_pool_tokens_out) = minimum_pool_tokens_out {
        Instruction {
            program_id: *program_id,
            accounts,
            data: borsh::to_vec(&StakePoolInstruction::DepositSolWithSlippage {
                lamports_in,
                minimum_pool_tokens_out,
            })
            .unwrap(),
        }
    } else {
        Instruction {
            program_id: *program_id,
            accounts,
            data: borsh::to_vec(&StakePoolInstruction::DepositSol(lamports_in)).unwrap(),
        }
    }
}

/// Creates instruction to deposit SOL directly into a stake pool.
pub fn deposit_sol(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    stake_pool_withdraw_authority: &Pubkey,
    reserve_stake_account: &Pubkey,
    lamports_from: &Pubkey,
    pool_tokens_to: &Pubkey,
    manager_fee_account: &Pubkey,
    referrer_pool_tokens_account: &Pubkey,
    pool_mint: &Pubkey,
    token_program_id: &Pubkey,
    lamports_in: u64,
) -> Instruction {
    deposit_sol_internal(
        program_id,
        stake_pool,
        stake_pool_withdraw_authority,
        reserve_stake_account,
        lamports_from,
        pool_tokens_to,
        manager_fee_account,
        referrer_pool_tokens_account,
        pool_mint,
        token_program_id,
        None,
        lamports_in,
        None,
    )
}

/// Creates instruction to deposit SOL directly into a stake pool with slippage
/// constraint.
pub fn deposit_sol_with_slippage(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    stake_pool_withdraw_authority: &Pubkey,
    reserve_stake_account: &Pubkey,
    lamports_from: &Pubkey,
    pool_tokens_to: &Pubkey,
    manager_fee_account: &Pubkey,
    referrer_pool_tokens_account: &Pubkey,
    pool_mint: &Pubkey,
    token_program_id: &Pubkey,
    lamports_in: u64,
    minimum_pool_tokens_out: u64,
) -> Instruction {
    deposit_sol_internal(
        program_id,
        stake_pool,
        stake_pool_withdraw_authority,
        reserve_stake_account,
        lamports_from,
        pool_tokens_to,
        manager_fee_account,
        referrer_pool_tokens_account,
        pool_mint,
        token_program_id,
        None,
        lamports_in,
        Some(minimum_pool_tokens_out),
    )
}

/// Creates instruction required to deposit SOL directly into a stake pool.
/// The difference with `deposit_sol()` is that a deposit
/// authority must sign this instruction.
pub fn deposit_sol_with_authority(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    sol_deposit_authority: &Pubkey,
    stake_pool_withdraw_authority: &Pubkey,
    reserve_stake_account: &Pubkey,
    lamports_from: &Pubkey,
    pool_tokens_to: &Pubkey,
    manager_fee_account: &Pubkey,
    referrer_pool_tokens_account: &Pubkey,
    pool_mint: &Pubkey,
    token_program_id: &Pubkey,
    lamports_in: u64,
) -> Instruction {
    deposit_sol_internal(
        program_id,
        stake_pool,
        stake_pool_withdraw_authority,
        reserve_stake_account,
        lamports_from,
        pool_tokens_to,
        manager_fee_account,
        referrer_pool_tokens_account,
        pool_mint,
        token_program_id,
        Some(sol_deposit_authority),
        lamports_in,
        None,
    )
}

/// Creates instruction to deposit SOL directly into a stake pool with slippage
/// constraint.
pub fn deposit_sol_with_authority_and_slippage(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    sol_deposit_authority: &Pubkey,
    stake_pool_withdraw_authority: &Pubkey,
    reserve_stake_account: &Pubkey,
    lamports_from: &Pubkey,
    pool_tokens_to: &Pubkey,
    manager_fee_account: &Pubkey,
    referrer_pool_tokens_account: &Pubkey,
    pool_mint: &Pubkey,
    token_program_id: &Pubkey,
    lamports_in: u64,
    minimum_pool_tokens_out: u64,
) -> Instruction {
    deposit_sol_internal(
        program_id,
        stake_pool,
        stake_pool_withdraw_authority,
        reserve_stake_account,
        lamports_from,
        pool_tokens_to,
        manager_fee_account,
        referrer_pool_tokens_account,
        pool_mint,
        token_program_id,
        Some(sol_deposit_authority),
        lamports_in,
        Some(minimum_pool_tokens_out),
    )
}

fn withdraw_stake_internal(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    validator_list_storage: &Pubkey,
    stake_pool_withdraw: &Pubkey,
    stake_to_split: &Pubkey,
    stake_to_receive: &Pubkey,
    user_stake_authority: &Pubkey,
    user_transfer_authority: &Pubkey,
    user_pool_token_account: &Pubkey,
    manager_fee_account: &Pubkey,
    pool_mint: &Pubkey,
    token_program_id: &Pubkey,
    pool_tokens_in: u64,
    minimum_lamports_out: Option<u64>,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*stake_pool, false),
        AccountMeta::new(*validator_list_storage, false),
        AccountMeta::new_readonly(*stake_pool_withdraw, false),
        AccountMeta::new(*stake_to_split, false),
        AccountMeta::new(*stake_to_receive, false),
        AccountMeta::new_readonly(*user_stake_authority, false),
        AccountMeta::new_readonly(*user_transfer_authority, true),
        AccountMeta::new(*user_pool_token_account, false),
        AccountMeta::new(*manager_fee_account, false),
        AccountMeta::new(*pool_mint, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(*token_program_id, false),
        AccountMeta::new_readonly(stake::program::id(), false),
    ];
    if let Some(minimum_lamports_out) = minimum_lamports_out {
        Instruction {
            program_id: *program_id,
            accounts,
            data: borsh::to_vec(&StakePoolInstruction::WithdrawStakeWithSlippage {
                pool_tokens_in,
                minimum_lamports_out,
            })
            .unwrap(),
        }
    } else {
        Instruction {
            program_id: *program_id,
            accounts,
            data: borsh::to_vec(&StakePoolInstruction::WithdrawStake(pool_tokens_in)).unwrap(),
        }
    }
}

/// Creates a 'WithdrawStake' instruction.
pub fn withdraw_stake(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    validator_list_storage: &Pubkey,
    stake_pool_withdraw: &Pubkey,
    stake_to_split: &Pubkey,
    stake_to_receive: &Pubkey,
    user_stake_authority: &Pubkey,
    user_transfer_authority: &Pubkey,
    user_pool_token_account: &Pubkey,
    manager_fee_account: &Pubkey,
    pool_mint: &Pubkey,
    token_program_id: &Pubkey,
    pool_tokens_in: u64,
) -> Instruction {
    withdraw_stake_internal(
        program_id,
        stake_pool,
        validator_list_storage,
        stake_pool_withdraw,
        stake_to_split,
        stake_to_receive,
        user_stake_authority,
        user_transfer_authority,
        user_pool_token_account,
        manager_fee_account,
        pool_mint,
        token_program_id,
        pool_tokens_in,
        None,
    )
}

/// Creates a 'WithdrawStakeWithSlippage' instruction.
pub fn withdraw_stake_with_slippage(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    validator_list_storage: &Pubkey,
    stake_pool_withdraw: &Pubkey,
    stake_to_split: &Pubkey,
    stake_to_receive: &Pubkey,
    user_stake_authority: &Pubkey,
    user_transfer_authority: &Pubkey,
    user_pool_token_account: &Pubkey,
    manager_fee_account: &Pubkey,
    pool_mint: &Pubkey,
    token_program_id: &Pubkey,
    pool_tokens_in: u64,
    minimum_lamports_out: u64,
) -> Instruction {
    withdraw_stake_internal(
        program_id,
        stake_pool,
        validator_list_storage,
        stake_pool_withdraw,
        stake_to_split,
        stake_to_receive,
        user_stake_authority,
        user_transfer_authority,
        user_pool_token_account,
        manager_fee_account,
        pool_mint,
        token_program_id,
        pool_tokens_in,
        Some(minimum_lamports_out),
    )
}

fn withdraw_sol_internal(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    stake_pool_withdraw_authority: &Pubkey,
    user_transfer_authority: &Pubkey,
    pool_tokens_from: &Pubkey,
    reserve_stake_account: &Pubkey,
    lamports_to: &Pubkey,
    manager_fee_account: &Pubkey,
    pool_mint: &Pubkey,
    token_program_id: &Pubkey,
    sol_withdraw_authority: Option<&Pubkey>,
    pool_tokens_in: u64,
    minimum_lamports_out: Option<u64>,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new(*stake_pool, false),
        AccountMeta::new_readonly(*stake_pool_withdraw_authority, false),
        AccountMeta::new_readonly(*user_transfer_authority, true),
        AccountMeta::new(*pool_tokens_from, false),
        AccountMeta::new(*reserve_stake_account, false),
        AccountMeta::new(*lamports_to, false),
        AccountMeta::new(*manager_fee_account, false),
        AccountMeta::new(*pool_mint, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::stake_history::id(), false),
        AccountMeta::new_readonly(stake::program::id(), false),
        AccountMeta::new_readonly(*token_program_id, false),
    ];
    if let Some(sol_withdraw_authority) = sol_withdraw_authority {
        accounts.push(AccountMeta::new_readonly(*sol_withdraw_authority, true));
    }
    if let Some(minimum_lamports_out) = minimum_lamports_out {
        Instruction {
            program_id: *program_id,
            accounts,
            data: borsh::to_vec(&StakePoolInstruction::WithdrawSolWithSlippage {
                pool_tokens_in,
                minimum_lamports_out,
            })
            .unwrap(),
        }
    } else {
        Instruction {
            program_id: *program_id,
            accounts,
            data: borsh::to_vec(&StakePoolInstruction::WithdrawSol(pool_tokens_in)).unwrap(),
        }
    }
}

/// Creates instruction required to withdraw SOL directly from a stake pool.
pub fn withdraw_sol(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    stake_pool_withdraw_authority: &Pubkey,
    user_transfer_authority: &Pubkey,
    pool_tokens_from: &Pubkey,
    reserve_stake_account: &Pubkey,
    lamports_to: &Pubkey,
    manager_fee_account: &Pubkey,
    pool_mint: &Pubkey,
    token_program_id: &Pubkey,
    pool_tokens_in: u64,
) -> Instruction {
    withdraw_sol_internal(
        program_id,
        stake_pool,
        stake_pool_withdraw_authority,
        user_transfer_authority,
        pool_tokens_from,
        reserve_stake_account,
        lamports_to,
        manager_fee_account,
        pool_mint,
        token_program_id,
        None,
        pool_tokens_in,
        None,
    )
}

/// Creates instruction required to withdraw SOL directly from a stake pool with
/// slippage constraints.
pub fn withdraw_sol_with_slippage(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    stake_pool_withdraw_authority: &Pubkey,
    user_transfer_authority: &Pubkey,
    pool_tokens_from: &Pubkey,
    reserve_stake_account: &Pubkey,
    lamports_to: &Pubkey,
    manager_fee_account: &Pubkey,
    pool_mint: &Pubkey,
    token_program_id: &Pubkey,
    pool_tokens_in: u64,
    minimum_lamports_out: u64,
) -> Instruction {
    withdraw_sol_internal(
        program_id,
        stake_pool,
        stake_pool_withdraw_authority,
        user_transfer_authority,
        pool_tokens_from,
        reserve_stake_account,
        lamports_to,
        manager_fee_account,
        pool_mint,
        token_program_id,
        None,
        pool_tokens_in,
        Some(minimum_lamports_out),
    )
}

/// Creates instruction required to withdraw SOL directly from a stake pool.
/// The difference with `withdraw_sol()` is that the sol withdraw authority
/// must sign this instruction.
pub fn withdraw_sol_with_authority(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    sol_withdraw_authority: &Pubkey,
    stake_pool_withdraw_authority: &Pubkey,
    user_transfer_authority: &Pubkey,
    pool_tokens_from: &Pubkey,
    reserve_stake_account: &Pubkey,
    lamports_to: &Pubkey,
    manager_fee_account: &Pubkey,
    pool_mint: &Pubkey,
    token_program_id: &Pubkey,
    pool_tokens_in: u64,
) -> Instruction {
    withdraw_sol_internal(
        program_id,
        stake_pool,
        stake_pool_withdraw_authority,
        user_transfer_authority,
        pool_tokens_from,
        reserve_stake_account,
        lamports_to,
        manager_fee_account,
        pool_mint,
        token_program_id,
        Some(sol_withdraw_authority),
        pool_tokens_in,
        None,
    )
}

/// Creates instruction required to withdraw SOL directly from a stake pool with
/// a slippage constraint.
/// The difference with `withdraw_sol()` is that the sol withdraw authority
/// must sign this instruction.
pub fn withdraw_sol_with_authority_and_slippage(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    sol_withdraw_authority: &Pubkey,
    stake_pool_withdraw_authority: &Pubkey,
    user_transfer_authority: &Pubkey,
    pool_tokens_from: &Pubkey,
    reserve_stake_account: &Pubkey,
    lamports_to: &Pubkey,
    manager_fee_account: &Pubkey,
    pool_mint: &Pubkey,
    token_program_id: &Pubkey,
    pool_tokens_in: u64,
    minimum_lamports_out: u64,
) -> Instruction {
    withdraw_sol_internal(
        program_id,
        stake_pool,
        stake_pool_withdraw_authority,
        user_transfer_authority,
        pool_tokens_from,
        reserve_stake_account,
        lamports_to,
        manager_fee_account,
        pool_mint,
        token_program_id,
        Some(sol_withdraw_authority),
        pool_tokens_in,
        Some(minimum_lamports_out),
    )
}

/// Creates a 'set manager' instruction.
pub fn set_manager(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    manager: &Pubkey,
    new_manager: &Pubkey,
    new_fee_receiver: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*stake_pool, false),
        AccountMeta::new_readonly(*manager, true),
        AccountMeta::new_readonly(*new_manager, true),
        AccountMeta::new_readonly(*new_fee_receiver, false),
    ];
    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&StakePoolInstruction::SetManager).unwrap(),
    }
}

/// Creates a 'set fee' instruction.
pub fn set_fee(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    manager: &Pubkey,
    fee: FeeType,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*stake_pool, false),
        AccountMeta::new_readonly(*manager, true),
    ];
    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&StakePoolInstruction::SetFee { fee }).unwrap(),
    }
}

/// Creates a 'set staker' instruction.
pub fn set_staker(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    set_staker_authority: &Pubkey,
    new_staker: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*stake_pool, false),
        AccountMeta::new_readonly(*set_staker_authority, true),
        AccountMeta::new_readonly(*new_staker, false),
    ];
    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&StakePoolInstruction::SetStaker).unwrap(),
    }
}

/// Creates a 'SetFundingAuthority' instruction.
pub fn set_funding_authority(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    manager: &Pubkey,
    new_sol_deposit_authority: Option<&Pubkey>,
    funding_type: FundingType,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new(*stake_pool, false),
        AccountMeta::new_readonly(*manager, true),
    ];
    if let Some(auth) = new_sol_deposit_authority {
        accounts.push(AccountMeta::new_readonly(*auth, false))
    }
    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&StakePoolInstruction::SetFundingAuthority(funding_type)).unwrap(),
    }
}

/// Creates an instruction to update metadata in the mpl token metadata program
/// account for the pool token
pub fn update_token_metadata(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    manager: &Pubkey,
    pool_mint: &Pubkey,
    name: String,
    symbol: String,
    uri: String,
) -> Instruction {
    let (stake_pool_withdraw_authority, _) =
        find_withdraw_authority_program_address(program_id, stake_pool);
    let (token_metadata, _) = find_metadata_account(pool_mint);

    let accounts = vec![
        AccountMeta::new_readonly(*stake_pool, false),
        AccountMeta::new_readonly(*manager, true),
        AccountMeta::new_readonly(stake_pool_withdraw_authority, false),
        AccountMeta::new(token_metadata, false),
        AccountMeta::new_readonly(inline_mpl_token_metadata::id(), false),
    ];

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&StakePoolInstruction::UpdateTokenMetadata { name, symbol, uri })
            .unwrap(),
    }
}

/// Creates an instruction to create metadata using the mpl token metadata
/// program for the pool token
pub fn create_token_metadata(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    manager: &Pubkey,
    pool_mint: &Pubkey,
    payer: &Pubkey,
    name: String,
    symbol: String,
    uri: String,
) -> Instruction {
    let (stake_pool_withdraw_authority, _) =
        find_withdraw_authority_program_address(program_id, stake_pool);
    let (token_metadata, _) = find_metadata_account(pool_mint);

    let accounts = vec![
        AccountMeta::new_readonly(*stake_pool, false),
        AccountMeta::new_readonly(*manager, true),
        AccountMeta::new_readonly(stake_pool_withdraw_authority, false),
        AccountMeta::new_readonly(*pool_mint, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new(token_metadata, false),
        AccountMeta::new_readonly(inline_mpl_token_metadata::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&StakePoolInstruction::CreateTokenMetadata { name, symbol, uri })
            .unwrap(),
    }
}
