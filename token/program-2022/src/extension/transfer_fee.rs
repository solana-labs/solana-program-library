use {
    crate::{
        extension::{AccountType, Extension, ExtensionType},
        pod::*,
    },
    bytemuck::{Pod, Zeroable},
};

/// Transfer fee information
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct TransferFee {
    /// First epoch where the transfer fee takes effect
    pub epoch: PodU64, // Epoch,
    /// Maximum fee assessed on transfers, expressed as an amount of tokens
    pub maximum_fee: PodU64,
    /// Amount of transfer collected as fees, expressed as basis points of the
    /// transfer amount, ie. increments of 0.01%
    pub transfer_fee_basis_points: PodU16,
}

/// Transfer fee extension data for mints.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct MintTransferFee {
    /// Optional authority to set the fee
    pub transfer_fee_config_authority: OptionalNonZeroPubkey,
    /// Withdraw from mint instructions must be signed by this key
    pub withheld_withdraw_authority: OptionalNonZeroPubkey,
    /// Withheld transfer fee tokens that have been moved to the mint for withdrawal
    pub withheld_amount: PodU64,
    /// Older transfer fee, used if the current epoch < new_transfer_fee.epoch
    pub older_transfer_fee: TransferFee,
    /// Newer transfer fee, used if the current epoch >= new_transfer_fee.epoch
    pub newer_transfer_fee: TransferFee,
}
impl Extension for MintTransferFee {
    const TYPE: ExtensionType = ExtensionType::MintTransferFee;
    const ACCOUNT_TYPE: AccountType = AccountType::Mint;
}

/// Transfer fee extension data for accounts.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct AccountTransferFee {
    /// Amount withheld during transfers, to be harvested to the mint
    pub withheld_amount: PodU64,
}
impl Extension for AccountTransferFee {
    const TYPE: ExtensionType = ExtensionType::AccountTransferFee;
    const ACCOUNT_TYPE: AccountType = AccountType::Account;
}

#[cfg(test)]
pub(crate) mod test {
    use {super::*, solana_program::pubkey::Pubkey, std::convert::TryFrom};

    pub(crate) fn test_mint_transfer_fee() -> MintTransferFee {
        MintTransferFee {
            transfer_fee_config_authority: OptionalNonZeroPubkey::try_from(Some(Pubkey::new(
                &[10; 32],
            )))
            .unwrap(),
            withheld_withdraw_authority: OptionalNonZeroPubkey::try_from(Some(Pubkey::new(
                &[11; 32],
            )))
            .unwrap(),
            withheld_amount: PodU64::from(u64::MAX),
            older_transfer_fee: TransferFee {
                epoch: PodU64::from(1),
                maximum_fee: PodU64::from(10),
                transfer_fee_basis_points: PodU16::from(100),
            },
            newer_transfer_fee: TransferFee {
                epoch: PodU64::from(100),
                maximum_fee: PodU64::from(5_000),
                transfer_fee_basis_points: PodU16::from(1),
            },
        }
    }
}
