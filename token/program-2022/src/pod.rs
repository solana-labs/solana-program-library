//! Rewrites of the base state types represented as Pods

#[cfg(test)]
use {
    crate::state::{Account, Mint, Multisig},
    solana_program::program_option::COption,
};
use {
    crate::{instruction::MAX_SIGNERS, state::PackedSizeOf},
    bytemuck::{Pod, Zeroable},
    solana_program::{program_pack::IsInitialized, pubkey::Pubkey},
    spl_pod::{
        bytemuck::pod_get_packed_len,
        primitives::{PodBool, PodU64},
    },
};

/// [Mint] data stored as a Pod type
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
    /// If `true`, this structure has been initialized
    pub is_initialized: PodBool,
    /// Optional authority to freeze token accounts.
    pub freeze_authority: PodCOption<Pubkey>,
}
impl IsInitialized for PodMint {
    fn is_initialized(&self) -> bool {
        self.is_initialized.into()
    }
}
impl PackedSizeOf for PodMint {
    const SIZE_OF: usize = pod_get_packed_len::<Self>();
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

/// [Account] data stored as a Pod type
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
    /// The account's [AccountState], stored as a u8
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
    const SIZE_OF: usize = pod_get_packed_len::<Self>();
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
            is_native: account.is_native.map(PodU64::from_primitive).into(),
            delegated_amount: account.delegated_amount.into(),
            close_authority: account.close_authority.into(),
        }
    }
}

/// [Multisig] data stored as a Pod type
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct PodMultisig {
    /// Number of signers required
    pub m: u8,
    /// Number of valid signers
    pub n: u8,
    /// If `true`, this structure has been initialized
    pub is_initialized: PodBool,
    /// Signer public keys
    pub signers: [Pubkey; MAX_SIGNERS],
}
impl IsInitialized for PodMultisig {
    fn is_initialized(&self) -> bool {
        self.is_initialized.into()
    }
}
impl PackedSizeOf for PodMultisig {
    const SIZE_OF: usize = pod_get_packed_len::<Self>();
}
#[cfg(test)]
impl From<Multisig> for PodMultisig {
    fn from(multisig: Multisig) -> Self {
        Self {
            m: multisig.m,
            n: multisig.n,
            is_initialized: multisig.is_initialized.into(),
            signers: multisig.signers,
        }
    }
}

/// COption<T> stored as a Pod type
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct PodCOption<T: Pod + Default> {
    option: [u8; 4],
    value: T,
}
impl<T: Pod + Default> PodCOption<T> {
    /// Represents that no value is stored in the option, like `Option::None`
    pub const NONE: [u8; 4] = [0; 4];
    /// Represents that some value is stored in the option, like
    /// `Option::Some(v)`
    pub const SOME: [u8; 4] = [1, 0, 0, 0];
}
#[cfg(test)]
impl<T: Pod + Default> From<COption<T>> for PodCOption<T> {
    fn from(opt: COption<T>) -> Self {
        match opt {
            COption::None => Self {
                option: Self::NONE,
                value: T::default(),
            },
            COption::Some(v) => Self {
                option: Self::SOME,
                value: v,
            },
        }
    }
}

#[cfg(test)]
mod test {
    use {
        super::*,
        crate::state::test::{
            TEST_ACCOUNT, TEST_ACCOUNT_SLICE, TEST_MINT, TEST_MINT_SLICE, TEST_MULTISIG,
            TEST_MULTISIG_SLICE,
        },
        spl_pod::bytemuck::pod_from_bytes,
    };
    #[test]
    fn pod_mint_to_mint_equality() {
        let pod_mint = pod_from_bytes::<PodMint>(TEST_MINT_SLICE).unwrap();
        assert_eq!(*pod_mint, PodMint::from(TEST_MINT));
    }

    #[test]
    fn pod_account_to_account_equality() {
        let pod_account = pod_from_bytes::<PodAccount>(TEST_ACCOUNT_SLICE).unwrap();
        assert_eq!(*pod_account, PodAccount::from(TEST_ACCOUNT));
    }

    #[test]
    fn pod_multisig_to_multisig_equality() {
        let pod_multisig = pod_from_bytes::<PodMultisig>(TEST_MULTISIG_SLICE).unwrap();
        assert_eq!(*pod_multisig, PodMultisig::from(TEST_MULTISIG));
    }
}
