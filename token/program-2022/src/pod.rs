//! Rewrites of the base state types represented as Pods

#[cfg(test)]
use {
    crate::state::{Account, Mint},
    solana_program::program_option::COption,
};
use {
    crate::{instruction::MAX_SIGNERS, state::PackedSizeOf},
    bytemuck::{Pod, Zeroable},
    solana_program::{program_pack::IsInitialized, pubkey::Pubkey},
    spl_pod::primitives::{PodBool, PodU64},
};

/// Mint data stored as a Pod type
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct PodMint {
    /// Optional authority used to mint new tokens. The mint authority may only
    /// be provided during mint creation. If no mint authority is present
    /// then the mint has a fixed supply and no further tokens may be
    /// minted.
    pub mint_authority: PodCOptionPubkey,
    /// Total supply of tokens.
    pub supply: PodU64,
    /// Number of base 10 digits to the right of the decimal place.
    pub decimals: u8,
    /// Is not 0 if this structure has been initialized
    pub is_initialized: PodBool,
    /// Optional authority to freeze token accounts.
    pub freeze_authority: PodCOptionPubkey,
}
impl IsInitialized for PodMint {
    fn is_initialized(&self) -> bool {
        self.is_initialized.into()
    }
}
impl PackedSizeOf for PodMint {
    const SIZE_OF: usize = std::mem::size_of::<Self>();
}
// All of the #[cfg(test)] items here are used for easier testing, mostly in
// src/extension/mod.rs
#[cfg(test)]
impl PartialEq<&PodMint> for Mint {
    fn eq(&self, pod: &&PodMint) -> bool {
        PodMint::from(*self) == **pod
    }
}
#[cfg(test)]
impl PartialEq<Mint> for &PodMint {
    fn eq(&self, mint: &crate::state::Mint) -> bool {
        mint.eq(self)
    }
}
#[cfg(test)]
impl PartialEq<&mut PodMint> for Mint {
    fn eq(&self, pod: &&mut PodMint) -> bool {
        PodMint::from(*self) == **pod
    }
}
#[cfg(test)]
impl PartialEq<Mint> for &mut PodMint {
    fn eq(&self, mint: &Mint) -> bool {
        mint.eq(self)
    }
}
#[cfg(test)]
impl From<Mint> for PodMint {
    fn from(mint: Mint) -> Self {
        Self {
            mint_authority: mint.mint_authority.into(),
            supply: mint.supply.into(),
            decimals: mint.decimals,
            is_initialized: mint.is_initialized.into(),
            freeze_authority: mint.freeze_authority.into(),
        }
    }
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
    pub delegate: PodCOptionPubkey,
    /// The account's state, stored as u8
    pub state: u8,
    /// If is_some, this is a native token, and the value logs the rent-exempt
    /// reserve. An Account is required to be rent-exempt, so the value is
    /// used by the Processor to ensure that wrapped SOL accounts do not
    /// drop below this threshold.
    pub is_native: PodCOptionU64,
    /// The amount delegated
    pub delegated_amount: PodU64,
    /// Optional authority to close the account.
    pub close_authority: PodCOptionPubkey,
}
impl IsInitialized for PodAccount {
    fn is_initialized(&self) -> bool {
        self.state != 0
    }
}
impl PackedSizeOf for PodAccount {
    const SIZE_OF: usize = std::mem::size_of::<Self>();
}
#[cfg(test)]
impl PartialEq<PodAccount> for Account {
    fn eq(&self, pod: &PodAccount) -> bool {
        PodAccount::from(*self) == *pod
    }
}
#[cfg(test)]
impl PartialEq<Account> for PodAccount {
    fn eq(&self, account: &Account) -> bool {
        account.eq(self)
    }
}
#[cfg(test)]
impl PartialEq<&mut PodAccount> for Account {
    fn eq(&self, pod: &&mut PodAccount) -> bool {
        PodAccount::from(*self) == **pod
    }
}
#[cfg(test)]
impl PartialEq<Account> for &mut PodAccount {
    fn eq(&self, account: &Account) -> bool {
        account.eq(self)
    }
}
#[cfg(test)]
impl From<Account> for PodAccount {
    fn from(account: Account) -> Self {
        Self {
            mint: account.mint,
            owner: account.owner,
            amount: account.amount.into(),
            delegate: account.delegate.into(),
            state: account.state.into(),
            is_native: account.is_native.into(),
            delegated_amount: account.delegated_amount.into(),
            close_authority: account.close_authority.into(),
        }
    }
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

/// COption<Pubkey> stored as a Pod type
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct PodCOptionPubkey {
    option: [u8; 4],
    value: Pubkey,
}
impl PodCOptionPubkey {
    /// Create an option with a value, corresponds to Option::Some(value)
    pub const fn some(value: Pubkey) -> Self {
        Self {
            option: [1, 0, 0, 0],
            value,
        }
    }

    /// Create an option without a value, corresponds to Option::None
    pub const fn none() -> Self {
        Self {
            option: [0, 0, 0, 0],
            value: Pubkey::new_from_array([0; 32]),
        }
    }
}
#[cfg(test)]
impl From<PodCOptionPubkey> for COption<Pubkey> {
    fn from(pod: PodCOptionPubkey) -> Self {
        if pod.option == [0, 0, 0, 0] {
            COption::None
        } else {
            COption::Some(pod.value)
        }
    }
}
#[cfg(test)]
impl From<COption<Pubkey>> for PodCOptionPubkey {
    fn from(opt: COption<Pubkey>) -> Self {
        match opt {
            COption::None => Self::default(),
            COption::Some(pk) => Self {
                option: [1, 0, 0, 0],
                value: pk,
            },
        }
    }
}

/// COption<u64> stored as a Pod type
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct PodCOptionU64 {
    option: [u8; 4],
    value: PodU64,
}
impl PodCOptionU64 {
    /// Create an option with a value, corresponds to Option::Some(value)
    pub const fn some(value: u64) -> Self {
        Self {
            option: [1, 0, 0, 0],
            value: PodU64::from_primitive(value),
        }
    }
    /// Create an option without a value, corresponds to Option::None
    pub const fn none() -> Self {
        Self {
            option: [0, 0, 0, 0],
            value: PodU64::from_primitive(0),
        }
    }
}
#[cfg(test)]
impl From<PodCOptionU64> for COption<u64> {
    fn from(pod: PodCOptionU64) -> Self {
        if pod.option == [0, 0, 0, 0] {
            COption::None
        } else {
            COption::Some(pod.value.into())
        }
    }
}
#[cfg(test)]
impl From<COption<u64>> for PodCOptionU64 {
    fn from(opt: COption<u64>) -> Self {
        match opt {
            COption::None => Self::default(),
            COption::Some(v) => Self {
                option: [1, 0, 0, 0],
                value: v.into(),
            },
        }
    }
}

#[cfg(test)]
pub(crate) const TEST_MINT: PodMint = PodMint {
    mint_authority: PodCOptionPubkey::some(Pubkey::new_from_array([1; 32])),
    supply: PodU64::from_primitive(42),
    decimals: 7,
    is_initialized: PodBool::from_bool(true),
    freeze_authority: PodCOptionPubkey::some(Pubkey::new_from_array([2; 32])),
};
#[cfg(test)]
pub(crate) const TEST_ACCOUNT: PodAccount = PodAccount {
    mint: Pubkey::new_from_array([1; 32]),
    owner: Pubkey::new_from_array([2; 32]),
    amount: PodU64::from_primitive(3),
    delegate: PodCOptionPubkey::some(Pubkey::new_from_array([4; 32])),
    state: 2,
    is_native: PodCOptionU64::some(5),
    delegated_amount: PodU64::from_primitive(6),
    close_authority: PodCOptionPubkey::some(Pubkey::new_from_array([7; 32])),
};
