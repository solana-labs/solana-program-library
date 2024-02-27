//! Rewrites of the base state types represented as Pods

use {
    crate::{instruction::MAX_SIGNERS, state::PackedSizeOf},
    bytemuck::{Pod, Zeroable},
    solana_program::{program_pack::IsInitialized, pubkey::Pubkey},
    spl_pod::primitives::PodU64,
};

/// Mint data stored as a Pod type
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct PodMint {
    /// Optional authority used to mint new tokens. The mint authority may only
    /// be provided during mint creation. If no mint authority is present
    /// then the mint has a fixed supply and no further tokens may be
    /// minted.
    pub mint_authority: PodCOption<Pubkey>,
    /// Total supply of tokens.
    pub supply: PodU64,
    /// Number of base 10 digits to the right of the decimal place.
    pub decimals: u8,
    /// Is not 0 if this structure has been initialized
    pub is_initialized: u8,
    /// Optional authority to freeze token accounts.
    pub freeze_authority: PodCOption<Pubkey>,
}
impl IsInitialized for PodMint {
    fn is_initialized(&self) -> bool {
        self.is_initialized != 0
    }
}
impl PackedSizeOf for PodMint {
    const SIZE_OF: usize = std::mem::size_of::<Self>();
}

/// Account data stored as a Pod type
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct PodAccount {
    /// The mint associated with this account
    pub mint: Pubkey,
    /// The owner of this account.
    pub owner: Pubkey,
    /// The amount of tokens this account holds.
    pub amount: PodU64,
    /// If `delegate` is `Some` then `delegated_amount` represents
    /// the amount authorized by the delegate
    pub delegate: PodCOption<Pubkey>,
    /// The account's state, stored as u8
    pub state: u8,
    /// If is_some, this is a native token, and the value logs the rent-exempt
    /// reserve. An Account is required to be rent-exempt, so the value is
    /// used by the Processor to ensure that wrapped SOL accounts do not
    /// drop below this threshold.
    pub is_native: PodCOption<PodU64>,
    /// The amount delegated
    pub delegated_amount: PodU64,
    /// Optional authority to close the account.
    pub close_authority: PodCOption<Pubkey>,
}
impl IsInitialized for PodAccount {
    fn is_initialized(&self) -> bool {
        self.state != 0
    }
}
impl PackedSizeOf for PodAccount {
    const SIZE_OF: usize = std::mem::size_of::<Self>();
}

/// Multisignature data.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct PodMultisig {
    /// Number of signers required
    pub m: u8,
    /// Number of valid signers
    pub n: u8,
    /// If not 0, this structure has been initialized
    pub is_initialized: u8,
    /// Signer public keys
    pub signers: [Pubkey; MAX_SIGNERS],
}
impl IsInitialized for PodMultisig {
    fn is_initialized(&self) -> bool {
        self.is_initialized != 0
    }
}
impl PackedSizeOf for PodMultisig {
    const SIZE_OF: usize = std::mem::size_of::<Self>();
}

/// COption<T> stored as a Pod type
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct PodCOption<T: Pod> {
    option: [u8; 4],
    value: T,
}
