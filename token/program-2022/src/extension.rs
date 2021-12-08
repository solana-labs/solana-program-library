use solana_program::{clock::Epoch, program_option::COption, pubkey::Pubkey};

/// Extensions that can be applied to mints or accounts.  Mint extensions must only be
/// applied to mint accounts, and account extensions must only be applied to token holding
/// accounts.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Extension {
    /// Used as padding if the account size would otherwise be 355, same as a multisig
    Uninitialized,
    /// Includes a transfer fee and accompanying authorities to withdraw and set the fee
    MintTransferFee,
    /// Includes withheld transfer fees
    AccountTransferFee,
    /// Includes an optional mint close authority
    MintCloseAuthority,
}

/// Type-Length-Value Entry, used to encapsulate all extensions contained within an account
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TlvEntry<V> {
    pub extension: Extension,
    pub length: u32,
    pub value: V,
}

/// Close authority extension data for mints.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MintCloseAuthority {
    /// Optional authority to close the mint
    pub close_authority: COption<Pubkey>,
}

/// Transfer fee information
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TransferFee {
    /// First epoch where the transfer fee takes effect
    pub epoch: Epoch,
    /// Amount of transfer collected as fees, expressed as basis points of the
    /// transfer amount, ie. increments of 0.01%
    pub transfer_fee_basis_points: u16,
    /// Maximum fee assessed on transfers, expressed as an amount of tokens
    pub maximum_fee: u64,
}

/// Transfer fee extension data for mints.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MintTransferFee {
    /// Optional authority to set the fee
    pub transfer_fee_config_authority: COption<Pubkey>,
    /// Withdraw from mint instructions must be signed by this key
    pub withheld_withdraw_authority: COption<Pubkey>,
    /// Withheld transfer fee tokens that have been moved to the mint for withdrawal
    pub withheld_amount: u64,
    /// Older transfer fee, used if the current epoch < new_transfer_fee.epoch
    pub older_transfer_fee: TransferFee,
    /// Newer transfer fee, used if the current epoch >= new_transfer_fee.epoch
    pub newer_transfer_fee: TransferFee,
}

/// Transfer fee extension data for accounts.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AccountTransferFee {
    /// Amount withheld during transfers, to be harvested to the mint
    pub withheld_amount: u64,
}
