//! FIXME copied from the solana stake program

use {
    borsh::{BorshDeserialize, BorshSchema, BorshSerialize},
    serde_derive::{Deserialize, Serialize},
    solana_program::{
        clock::{Epoch, UnixTimestamp},
        pubkey::Pubkey,
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
