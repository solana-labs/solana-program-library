//! State transition types

use crate::instruction::MAX_SIGNERS;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use num_enum::TryFromPrimitive;
use solana_program::{
    info,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

/// Account data.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Account {
    /// The owner of this account.
    pub owner: Pubkey,
    /// The account's state
    pub state: AccountState,
}
impl Account {
}
impl Sealed for Account {}
impl IsInitialized for Account {
    fn is_initialized(&self) -> bool {
        self.state != AccountState::Uninitialized
    }
}
impl Pack for Account {
    const LEN: usize = 33; // the sum of the size of each of the components
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        info!("unpacking");
        let src = array_ref![src, 0, 33]; // should match LEN (Todo why does the compiler complain when I use LEN here?)
        let (owner, state) =
            array_refs![src, 32, 1];

        info!("State byte");
        info!(state[0], 0,0,0,0);

        Ok(Account {
            owner: Pubkey::new_from_array(*owner),
            state: AccountState::try_from_primitive(state[0])
                .or(Err(ProgramError::InvalidAccountData))?,
        })
    }
    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, 33]; // should match LEN (Todo why does the compiler complain when I use LEN here?)
        let (
            owner_dst,
            state_dst,
        ) = mut_array_refs![dst, 32, 1];
        let &Account {
            ref owner,
            state,
        } = self;
        owner_dst.copy_from_slice(owner.as_ref());
        state_dst[0] = state as u8;
    }
}

/// Account state.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, TryFromPrimitive)]
pub enum AccountState {
    /// Account is not yet initialized
    Uninitialized,
    /// Account is initialized; the account owner and/or delegate may perform permitted operations
    /// on this account
    Initialized,
}

impl Default for AccountState {
    fn default() -> Self {
        AccountState::Uninitialized
    }
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
            *dst = Pubkey::new(src);
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
