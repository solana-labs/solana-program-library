//! FIXME copied from the solana stake program

use {
    borsh::{BorshDeserialize, BorshSchema, BorshSerialize},
    serde_derive::{Deserialize, Serialize},
    solana_program::{
        clock::{Epoch, UnixTimestamp},
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
        stake,
    },
};

/// FIXME copied from the stake program, once https://github.com/solana-labs/solana/pull/20784
/// lands this can be removed
#[derive(
    BorshSerialize,
    BorshDeserialize,
    BorshSchema,
    Default,
    Debug,
    Serialize,
    Deserialize,
    PartialEq,
    Clone,
    Copy,
)]
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

/// FIXME copied from stake program
/// Checks if two active delegations are mergeable, required since we cannot recover
/// from a CPI error.
pub fn active_delegations_can_merge(
    stake: &stake::state::Delegation,
    source: &stake::state::Delegation,
) -> Result<(), ProgramError> {
    if stake.voter_pubkey != source.voter_pubkey {
        msg!("Unable to merge due to voter mismatch");
        Err(ProgramError::InvalidAccountData)
    } else if (stake.warmup_cooldown_rate - source.warmup_cooldown_rate).abs() < f64::EPSILON
        && stake.deactivation_epoch == Epoch::MAX
        && source.deactivation_epoch == Epoch::MAX
    {
        Ok(())
    } else {
        msg!("Unable to merge due to stake deactivation");
        Err(ProgramError::InvalidAccountData)
    }
}

/// FIXME copied from stake program
/// Checks if two active stakes are mergeable, required since we cannot recover
/// from a CPI error.
pub fn active_stakes_can_merge(
    stake: &stake::state::Stake,
    source: &stake::state::Stake,
) -> Result<(), ProgramError> {
    active_delegations_can_merge(&stake.delegation, &source.delegation)?;

    if stake.credits_observed == source.credits_observed {
        Ok(())
    } else {
        msg!("Unable to merge due to credits observed mismatch");
        Err(ProgramError::InvalidAccountData)
    }
}
