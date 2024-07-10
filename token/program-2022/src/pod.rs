//! Rewrites of the base state types represented as Pods

#[cfg(test)]
use crate::state::{Account, Mint, Multisig};
use {
    crate::{
        instruction::MAX_SIGNERS,
        state::{AccountState, PackedSizeOf},
    },
    bytemuck::{Pod, Zeroable},
    solana_program::{
        program_error::ProgramError, program_option::COption, program_pack::IsInitialized,
        pubkey::Pubkey,
    },
    spl_pod::{
        bytemuck::pod_get_packed_len,
        optional_keys::OptionalNonZeroPubkey,
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
impl PodAccount {
    /// Checks if account is frozen
    pub fn is_frozen(&self) -> bool {
        self.state == AccountState::Frozen as u8
    }
    /// Checks if account is native
    pub fn is_native(&self) -> bool {
        self.is_native.is_some()
    }
    /// Checks if a token Account's owner is the system_program or the
    /// incinerator
    pub fn is_owned_by_system_program_or_incinerator(&self) -> bool {
        solana_program::system_program::check_id(&self.owner)
            || solana_program::incinerator::check_id(&self.owner)
    }
}
impl IsInitialized for PodAccount {
    fn is_initialized(&self) -> bool {
        self.state == AccountState::Initialized as u8 || self.state == AccountState::Frozen as u8
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
pub struct PodCOption<T>
where
    T: Pod + Default,
{
    pub(crate) option: [u8; 4],
    pub(crate) value: T,
}
impl<T> PodCOption<T>
where
    T: Pod + Default,
{
    /// Represents that no value is stored in the option, like `Option::None`
    pub const NONE: [u8; 4] = [0; 4];
    /// Represents that some value is stored in the option, like
    /// `Option::Some(v)`
    pub const SOME: [u8; 4] = [1, 0, 0, 0];

    /// Create a PodCOption equivalent of `Option::None`
    ///
    /// This could be made `const` by using `std::mem::zeroed`, but that would
    /// require `unsafe` code, which is prohibited at the crate level.
    pub fn none() -> Self {
        Self {
            option: Self::NONE,
            value: T::default(),
        }
    }

    /// Create a PodCOption equivalent of `Option::Some(value)`
    pub const fn some(value: T) -> Self {
        Self {
            option: Self::SOME,
            value,
        }
    }

    /// Get the underlying value or another provided value if it isn't set,
    /// equivalent of `Option::unwrap_or`
    pub fn unwrap_or(self, default: T) -> T {
        if self.option == Self::NONE {
            default
        } else {
            self.value
        }
    }

    /// Checks to see if a value is set, equivalent of `Option::is_some`
    pub fn is_some(&self) -> bool {
        self.option == Self::SOME
    }

    /// Checks to see if no value is set, equivalent of `Option::is_none`
    pub fn is_none(&self) -> bool {
        self.option == Self::NONE
    }

    /// Converts the option into a Result, similar to `Option::ok_or`
    pub fn ok_or<E>(self, error: E) -> Result<T, E> {
        match self {
            Self {
                option: Self::SOME,
                value,
            } => Ok(value),
            _ => Err(error),
        }
    }
}
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
impl TryFrom<PodCOption<Pubkey>> for OptionalNonZeroPubkey {
    type Error = ProgramError;
    fn try_from(p: PodCOption<Pubkey>) -> Result<Self, Self::Error> {
        match p {
            PodCOption {
                option: PodCOption::<Pubkey>::SOME,
                value,
            } if value == Pubkey::default() => Err(ProgramError::InvalidArgument),
            PodCOption {
                option: PodCOption::<Pubkey>::SOME,
                value,
            } => Ok(Self(value)),
            PodCOption {
                option: PodCOption::<Pubkey>::NONE,
                value: _,
            } => Ok(Self(Pubkey::default())),
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
pub mod test {
    use {
        super::*,
        crate::state::{
            test::{
                TEST_ACCOUNT, TEST_ACCOUNT_SLICE, TEST_MINT, TEST_MINT_SLICE, TEST_MULTISIG,
                TEST_MULTISIG_SLICE,
            },
            AccountState,
        },
        spl_pod::bytemuck::pod_from_bytes,
    };

    pub const TEST_POD_MINT: PodMint = PodMint {
        mint_authority: PodCOption::some(Pubkey::new_from_array([1; 32])),
        supply: PodU64::from_primitive(42),
        decimals: 7,
        is_initialized: PodBool::from_bool(true),
        freeze_authority: PodCOption::some(Pubkey::new_from_array([2; 32])),
    };
    pub const TEST_POD_ACCOUNT: PodAccount = PodAccount {
        mint: Pubkey::new_from_array([1; 32]),
        owner: Pubkey::new_from_array([2; 32]),
        amount: PodU64::from_primitive(3),
        delegate: PodCOption::some(Pubkey::new_from_array([4; 32])),
        state: AccountState::Frozen as u8,
        is_native: PodCOption::some(PodU64::from_primitive(5)),
        delegated_amount: PodU64::from_primitive(6),
        close_authority: PodCOption::some(Pubkey::new_from_array([7; 32])),
    };

    #[test]
    fn pod_mint_to_mint_equality() {
        let pod_mint = pod_from_bytes::<PodMint>(TEST_MINT_SLICE).unwrap();
        assert_eq!(*pod_mint, PodMint::from(TEST_MINT));
        assert_eq!(*pod_mint, TEST_POD_MINT);
    }

    #[test]
    fn pod_account_to_account_equality() {
        let pod_account = pod_from_bytes::<PodAccount>(TEST_ACCOUNT_SLICE).unwrap();
        assert_eq!(*pod_account, PodAccount::from(TEST_ACCOUNT));
        assert_eq!(*pod_account, TEST_POD_ACCOUNT);
    }

    #[test]
    fn pod_multisig_to_multisig_equality() {
        let pod_multisig = pod_from_bytes::<PodMultisig>(TEST_MULTISIG_SLICE).unwrap();
        assert_eq!(*pod_multisig, PodMultisig::from(TEST_MULTISIG));
    }
}
