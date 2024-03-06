//! State transition types

use {
    crate::{
        extension::AccountType,
        generic_token_account::{is_initialized_account, GenericTokenAccount},
        instruction::MAX_SIGNERS,
    },
    arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs},
    num_enum::{IntoPrimitive, TryFromPrimitive},
    solana_program::{
        program_error::ProgramError,
        program_option::COption,
        program_pack::{IsInitialized, Pack, Sealed},
        pubkey::Pubkey,
    },
};

/// Simplified version of the `Pack` trait which only gives the size of the
/// packed struct. Useful when a function doesn't need a type to implement all
/// of `Pack`, but a size is still needed.
pub trait PackedSizeOf {
    /// The packed size of the struct
    const SIZE_OF: usize;
}

/// Mint data.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Mint {
    /// Optional authority used to mint new tokens. The mint authority may only
    /// be provided during mint creation. If no mint authority is present
    /// then the mint has a fixed supply and no further tokens may be
    /// minted.
    pub mint_authority: COption<Pubkey>,
    /// Total supply of tokens.
    pub supply: u64,
    /// Number of base 10 digits to the right of the decimal place.
    pub decimals: u8,
    /// Is `true` if this structure has been initialized
    pub is_initialized: bool,
    /// Optional authority to freeze token accounts.
    pub freeze_authority: COption<Pubkey>,
}
impl Sealed for Mint {}
impl IsInitialized for Mint {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
impl Pack for Mint {
    const LEN: usize = 82;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, 82];
        let (mint_authority, supply, decimals, is_initialized, freeze_authority) =
            array_refs![src, 36, 8, 1, 1, 36];
        let mint_authority = unpack_coption_key(mint_authority)?;
        let supply = u64::from_le_bytes(*supply);
        let decimals = decimals[0];
        let is_initialized = match is_initialized {
            [0] => false,
            [1] => true,
            _ => return Err(ProgramError::InvalidAccountData),
        };
        let freeze_authority = unpack_coption_key(freeze_authority)?;
        Ok(Mint {
            mint_authority,
            supply,
            decimals,
            is_initialized,
            freeze_authority,
        })
    }
    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, 82];
        let (
            mint_authority_dst,
            supply_dst,
            decimals_dst,
            is_initialized_dst,
            freeze_authority_dst,
        ) = mut_array_refs![dst, 36, 8, 1, 1, 36];
        let &Mint {
            ref mint_authority,
            supply,
            decimals,
            is_initialized,
            ref freeze_authority,
        } = self;
        pack_coption_key(mint_authority, mint_authority_dst);
        *supply_dst = supply.to_le_bytes();
        decimals_dst[0] = decimals;
        is_initialized_dst[0] = is_initialized as u8;
        pack_coption_key(freeze_authority, freeze_authority_dst);
    }
}
impl PackedSizeOf for Mint {
    const SIZE_OF: usize = Self::LEN;
}

/// Account data.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Account {
    /// The mint associated with this account
    pub mint: Pubkey,
    /// The owner of this account.
    pub owner: Pubkey,
    /// The amount of tokens this account holds.
    pub amount: u64,
    /// If `delegate` is `Some` then `delegated_amount` represents
    /// the amount authorized by the delegate
    pub delegate: COption<Pubkey>,
    /// The account's state
    pub state: AccountState,
    /// If is_some, this is a native token, and the value logs the rent-exempt
    /// reserve. An Account is required to be rent-exempt, so the value is
    /// used by the Processor to ensure that wrapped SOL accounts do not
    /// drop below this threshold.
    pub is_native: COption<u64>,
    /// The amount delegated
    pub delegated_amount: u64,
    /// Optional authority to close the account.
    pub close_authority: COption<Pubkey>,
}
impl Account {
    /// Checks if account is frozen
    pub fn is_frozen(&self) -> bool {
        self.state == AccountState::Frozen
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
impl Sealed for Account {}
impl IsInitialized for Account {
    fn is_initialized(&self) -> bool {
        self.state != AccountState::Uninitialized
    }
}
impl Pack for Account {
    const LEN: usize = 165;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, 165];
        let (mint, owner, amount, delegate, state, is_native, delegated_amount, close_authority) =
            array_refs![src, 32, 32, 8, 36, 1, 12, 8, 36];
        Ok(Account {
            mint: Pubkey::new_from_array(*mint),
            owner: Pubkey::new_from_array(*owner),
            amount: u64::from_le_bytes(*amount),
            delegate: unpack_coption_key(delegate)?,
            state: AccountState::try_from_primitive(state[0])
                .or(Err(ProgramError::InvalidAccountData))?,
            is_native: unpack_coption_u64(is_native)?,
            delegated_amount: u64::from_le_bytes(*delegated_amount),
            close_authority: unpack_coption_key(close_authority)?,
        })
    }
    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, 165];
        let (
            mint_dst,
            owner_dst,
            amount_dst,
            delegate_dst,
            state_dst,
            is_native_dst,
            delegated_amount_dst,
            close_authority_dst,
        ) = mut_array_refs![dst, 32, 32, 8, 36, 1, 12, 8, 36];
        let &Account {
            ref mint,
            ref owner,
            amount,
            ref delegate,
            state,
            ref is_native,
            delegated_amount,
            ref close_authority,
        } = self;
        mint_dst.copy_from_slice(mint.as_ref());
        owner_dst.copy_from_slice(owner.as_ref());
        *amount_dst = amount.to_le_bytes();
        pack_coption_key(delegate, delegate_dst);
        state_dst[0] = state as u8;
        pack_coption_u64(is_native, is_native_dst);
        *delegated_amount_dst = delegated_amount.to_le_bytes();
        pack_coption_key(close_authority, close_authority_dst);
    }
}
impl PackedSizeOf for Account {
    const SIZE_OF: usize = Self::LEN;
}

/// Account state.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, IntoPrimitive, TryFromPrimitive)]
pub enum AccountState {
    /// Account is not yet initialized
    #[default]
    Uninitialized,
    /// Account is initialized; the account owner and/or delegate may perform
    /// permitted operations on this account
    Initialized,
    /// Account has been frozen by the mint freeze authority. Neither the
    /// account owner nor the delegate are able to perform operations on
    /// this account.
    Frozen,
}

/// Multisignature data.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Multisig {
    /// Number of signers required
    pub m: u8,
    /// Number of valid signers
    pub n: u8,
    /// Is `true` if this structure has been initialized
    pub is_initialized: bool,
    /// Signer public keys
    pub signers: [Pubkey; MAX_SIGNERS],
}
impl Sealed for Multisig {}
impl IsInitialized for Multisig {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
impl Pack for Multisig {
    const LEN: usize = 355;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, 355];
        #[allow(clippy::ptr_offset_with_cast)]
        let (m, n, is_initialized, signers_flat) = array_refs![src, 1, 1, 1, 32 * MAX_SIGNERS];
        let mut result = Multisig {
            m: m[0],
            n: n[0],
            is_initialized: match is_initialized {
                [0] => false,
                [1] => true,
                _ => return Err(ProgramError::InvalidAccountData),
            },
            signers: [Pubkey::new_from_array([0u8; 32]); MAX_SIGNERS],
        };
        for (src, dst) in signers_flat.chunks(32).zip(result.signers.iter_mut()) {
            *dst = Pubkey::try_from(src).map_err(|_| ProgramError::InvalidAccountData)?;
        }
        Ok(result)
    }
    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, 355];
        #[allow(clippy::ptr_offset_with_cast)]
        let (m, n, is_initialized, signers_flat) = mut_array_refs![dst, 1, 1, 1, 32 * MAX_SIGNERS];
        *m = [self.m];
        *n = [self.n];
        *is_initialized = [self.is_initialized as u8];
        for (i, src) in self.signers.iter().enumerate() {
            let dst_array = array_mut_ref![signers_flat, 32 * i, 32];
            dst_array.copy_from_slice(src.as_ref());
        }
    }
}
impl PackedSizeOf for Multisig {
    const SIZE_OF: usize = Self::LEN;
}

// Helpers
pub(crate) fn pack_coption_key(src: &COption<Pubkey>, dst: &mut [u8; 36]) {
    let (tag, body) = mut_array_refs![dst, 4, 32];
    match src {
        COption::Some(key) => {
            *tag = [1, 0, 0, 0];
            body.copy_from_slice(key.as_ref());
        }
        COption::None => {
            *tag = [0; 4];
        }
    }
}
pub(crate) fn unpack_coption_key(src: &[u8; 36]) -> Result<COption<Pubkey>, ProgramError> {
    let (tag, body) = array_refs![src, 4, 32];
    match *tag {
        [0, 0, 0, 0] => Ok(COption::None),
        [1, 0, 0, 0] => Ok(COption::Some(Pubkey::new_from_array(*body))),
        _ => Err(ProgramError::InvalidAccountData),
    }
}
fn pack_coption_u64(src: &COption<u64>, dst: &mut [u8; 12]) {
    let (tag, body) = mut_array_refs![dst, 4, 8];
    match src {
        COption::Some(amount) => {
            *tag = [1, 0, 0, 0];
            *body = amount.to_le_bytes();
        }
        COption::None => {
            *tag = [0; 4];
        }
    }
}
fn unpack_coption_u64(src: &[u8; 12]) -> Result<COption<u64>, ProgramError> {
    let (tag, body) = array_refs![src, 4, 8];
    match *tag {
        [0, 0, 0, 0] => Ok(COption::None),
        [1, 0, 0, 0] => Ok(COption::Some(u64::from_le_bytes(*body))),
        _ => Err(ProgramError::InvalidAccountData),
    }
}

// `spl_token_program_2022::extension::AccountType::Account` ordinal value
const ACCOUNTTYPE_ACCOUNT: u8 = AccountType::Account as u8;
impl GenericTokenAccount for Account {
    fn valid_account_data(account_data: &[u8]) -> bool {
        // Use spl_token::state::Account::valid_account_data once possible
        account_data.len() == Account::LEN && is_initialized_account(account_data)
            || (account_data.len() > Account::LEN
                && account_data.len() != Multisig::LEN
                && ACCOUNTTYPE_ACCOUNT == account_data[Account::LEN]
                && is_initialized_account(account_data))
    }
}

#[cfg(test)]
pub(crate) mod test {
    use {super::*, crate::generic_token_account::ACCOUNT_INITIALIZED_INDEX};

    pub const TEST_MINT: Mint = Mint {
        mint_authority: COption::Some(Pubkey::new_from_array([1; 32])),
        supply: 42,
        decimals: 7,
        is_initialized: true,
        freeze_authority: COption::Some(Pubkey::new_from_array([2; 32])),
    };
    pub const TEST_MINT_SLICE: &[u8] = &[
        1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 42, 0, 0, 0, 0, 0, 0, 0, 7, 1, 1, 0, 0, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
        2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
    ];

    pub const TEST_ACCOUNT: Account = Account {
        mint: Pubkey::new_from_array([1; 32]),
        owner: Pubkey::new_from_array([2; 32]),
        amount: 3,
        delegate: COption::Some(Pubkey::new_from_array([4; 32])),
        state: AccountState::Frozen,
        is_native: COption::Some(5),
        delegated_amount: 6,
        close_authority: COption::Some(Pubkey::new_from_array([7; 32])),
    };
    pub const TEST_ACCOUNT_SLICE: &[u8] = &[
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
        2, 2, 2, 2, 3, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 2, 1, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0,
        0, 6, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
        7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    ];
    pub const TEST_MULTISIG: Multisig = Multisig {
        m: 1,
        n: 11,
        is_initialized: true,
        signers: [
            Pubkey::new_from_array([1; 32]),
            Pubkey::new_from_array([2; 32]),
            Pubkey::new_from_array([3; 32]),
            Pubkey::new_from_array([4; 32]),
            Pubkey::new_from_array([5; 32]),
            Pubkey::new_from_array([6; 32]),
            Pubkey::new_from_array([7; 32]),
            Pubkey::new_from_array([8; 32]),
            Pubkey::new_from_array([9; 32]),
            Pubkey::new_from_array([10; 32]),
            Pubkey::new_from_array([11; 32]),
        ],
    };
    pub const TEST_MULTISIG_SLICE: &[u8] = &[
        1, 11, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
        2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
        3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
        5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
        6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
        7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
        8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
        9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 10, 10, 10, 10, 10, 10, 10,
        10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
        10, 10, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11,
        11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11,
    ];

    #[test]
    fn test_pack_unpack() {
        // Mint
        let check = TEST_MINT;
        let mut packed = vec![0; Mint::get_packed_len() + 1];
        assert_eq!(
            Err(ProgramError::InvalidAccountData),
            Mint::pack(check, &mut packed)
        );
        let mut packed = vec![0; Mint::get_packed_len() - 1];
        assert_eq!(
            Err(ProgramError::InvalidAccountData),
            Mint::pack(check, &mut packed)
        );
        let mut packed = vec![0; Mint::get_packed_len()];
        Mint::pack(check, &mut packed).unwrap();
        assert_eq!(packed, TEST_MINT_SLICE);
        let unpacked = Mint::unpack(&packed).unwrap();
        assert_eq!(unpacked, check);

        // Account
        let check = TEST_ACCOUNT;
        let mut packed = vec![0; Account::get_packed_len() + 1];
        assert_eq!(
            Err(ProgramError::InvalidAccountData),
            Account::pack(check, &mut packed)
        );
        let mut packed = vec![0; Account::get_packed_len() - 1];
        assert_eq!(
            Err(ProgramError::InvalidAccountData),
            Account::pack(check, &mut packed)
        );
        let mut packed = vec![0; Account::get_packed_len()];
        Account::pack(check, &mut packed).unwrap();
        let expect = TEST_ACCOUNT_SLICE;
        assert_eq!(packed, expect);
        let unpacked = Account::unpack(&packed).unwrap();
        assert_eq!(unpacked, check);

        // Multisig
        let check = TEST_MULTISIG;
        let mut packed = vec![0; Multisig::get_packed_len() + 1];
        assert_eq!(
            Err(ProgramError::InvalidAccountData),
            Multisig::pack(check, &mut packed)
        );
        let mut packed = vec![0; Multisig::get_packed_len() - 1];
        assert_eq!(
            Err(ProgramError::InvalidAccountData),
            Multisig::pack(check, &mut packed)
        );
        let mut packed = vec![0; Multisig::get_packed_len()];
        Multisig::pack(check, &mut packed).unwrap();
        let expect = TEST_MULTISIG_SLICE;
        assert_eq!(packed, expect);
        let unpacked = Multisig::unpack(&packed).unwrap();
        assert_eq!(unpacked, check);
    }

    #[test]
    fn test_unpack_token_owner() {
        // Account data length < Account::LEN, unpack will not return a key
        let src: [u8; 12] = [0; 12];
        let result = Account::unpack_account_owner(&src);
        assert_eq!(result, Option::None);

        // The right account data size and initialized, unpack will return some key
        let mut src: [u8; Account::LEN] = [0; Account::LEN];
        src[ACCOUNT_INITIALIZED_INDEX] = AccountState::Initialized as u8;
        let result = Account::unpack_account_owner(&src);
        assert!(result.is_some());

        // The right account data size and frozen, unpack will return some key
        src[ACCOUNT_INITIALIZED_INDEX] = AccountState::Frozen as u8;
        let result = Account::unpack_account_owner(&src);
        assert!(result.is_some());

        // Account data length > account data size, but not a valid extension,
        // unpack will not return a key
        let mut src: [u8; Account::LEN + 5] = [0; Account::LEN + 5];
        src[ACCOUNT_INITIALIZED_INDEX] = AccountState::Initialized as u8;
        let result = Account::unpack_account_owner(&src);
        assert_eq!(result, Option::None);

        // Account data length > account data size with a valid extension and
        // initialized, expect some key returned
        let mut src: [u8; Account::LEN + 5] = [0; Account::LEN + 5];
        src[Account::LEN] = AccountType::Account as u8;
        src[ACCOUNT_INITIALIZED_INDEX] = AccountState::Initialized as u8;
        let result = Account::unpack_account_owner(&src);
        assert!(result.is_some());

        // Account data length > account data size with a valid extension but
        // uninitialized, expect None
        src[ACCOUNT_INITIALIZED_INDEX] = AccountState::Uninitialized as u8;
        let result = Account::unpack_account_owner(&src);
        assert!(result.is_none());

        // Account data length is multi-sig data size with a valid extension and
        // initialized, expect none
        let mut src: [u8; Multisig::LEN] = [0; Multisig::LEN];
        src[ACCOUNT_INITIALIZED_INDEX] = AccountState::Initialized as u8;
        src[Account::LEN] = AccountType::Account as u8;
        let result = Account::unpack_account_owner(&src);
        assert!(result.is_none());
    }

    #[test]
    fn test_unpack_token_mint() {
        // Account data length < Account::LEN, unpack will not return a key
        let src: [u8; 12] = [0; 12];
        let result = Account::unpack_account_mint(&src);
        assert_eq!(result, Option::None);

        // The right account data size and initialized, unpack will return some key
        let mut src: [u8; Account::LEN] = [0; Account::LEN];
        src[ACCOUNT_INITIALIZED_INDEX] = AccountState::Initialized as u8;
        let result = Account::unpack_account_mint(&src);
        assert!(result.is_some());

        // The right account data size and frozen, unpack will return some key
        src[ACCOUNT_INITIALIZED_INDEX] = AccountState::Frozen as u8;
        let result = Account::unpack_account_mint(&src);
        assert!(result.is_some());

        // Account data length > account data size, but not a valid extension,
        // unpack will not return a key
        let mut src: [u8; Account::LEN + 5] = [0; Account::LEN + 5];
        src[ACCOUNT_INITIALIZED_INDEX] = AccountState::Initialized as u8;
        let result = Account::unpack_account_mint(&src);
        assert_eq!(result, Option::None);

        // Account data length > account data size with a valid extension and
        // initialized, expect some key returned
        let mut src: [u8; Account::LEN + 5] = [0; Account::LEN + 5];
        src[ACCOUNT_INITIALIZED_INDEX] = AccountState::Initialized as u8;
        src[Account::LEN] = AccountType::Account as u8;
        let result = Account::unpack_account_mint(&src);
        assert!(result.is_some());

        // Account data length > account data size with a valid extension but
        // uninitialized, expect none
        src[ACCOUNT_INITIALIZED_INDEX] = AccountState::Uninitialized as u8;
        let result = Account::unpack_account_mint(&src);
        assert!(result.is_none());

        // Account data length is multi-sig data size with a valid extension and
        // initialized, expect none
        let mut src: [u8; Multisig::LEN] = [0; Multisig::LEN];
        src[ACCOUNT_INITIALIZED_INDEX] = AccountState::Initialized as u8;
        src[Account::LEN] = AccountType::Account as u8;
        let result = Account::unpack_account_mint(&src);
        assert!(result.is_none());
    }
}
