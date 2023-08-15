//! Instruction types

use {
    borsh::{BorshDeserialize, BorshSerialize},
    spl_discriminator::SplDiscriminate,
    spl_token_group_interface::state::SplTokenGroup,
    spl_type_length_value::SplBorshVariableLenPack,
};

/// A token `Collection`.
///
/// Group:      `Collection`
/// Members:    `Member`
#[derive(
    BorshDeserialize,
    BorshSerialize,
    Clone,
    Debug,
    PartialEq,
    SplBorshVariableLenPack,
    SplDiscriminate,
)]
#[discriminator_hash_input("spl_token_group_example:collection")]
pub struct Collection {
    /// The `Collection`'s creation slot
    pub creation_date: String,
}

impl SplTokenGroup for Collection {}

/// Token `Edition`s.
///
/// Group:      `Edition`
/// Members:    `Original` | `Reprint`
#[derive(
    BorshDeserialize,
    BorshSerialize,
    Clone,
    Debug,
    PartialEq,
    SplBorshVariableLenPack,
    SplDiscriminate,
)]
#[discriminator_hash_input("spl_token_group_example:edition")]
pub struct Edition {
    /// The `Edition`'s line
    pub line: EditionLine,
    /// The `Edition`'s membership level
    pub membership_level: MembershipLevel,
}

impl SplTokenGroup for Edition {}

/// The `Edition`'s line.
///
/// Note: This data is simply for demonstration purposes.
#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, PartialEq)]
pub enum EditionLine {
    /// The `Edition` is an original
    Original,
    /// The `Edition` is a "gold" reprint
    Gold,
    /// The `Edition` is a "silver" reprint
    Silver,
    /// The `Edition` is a "bronze" reprint
    Bronze,
}

/// The `Edition`'s membership level.
///
/// Note: This data is simply for demonstration purposes.
#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, PartialEq)]
pub enum MembershipLevel {
    /// The `Edition` is for "ultimate" members
    Ultimate,
    /// The `Edition` is for "premium" members
    Premium,
    /// The `Edition` is for "standard" members
    Standard,
}
