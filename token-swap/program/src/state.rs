//! State transition types

use crate::curve::{base::SwapCurve, fees::Fees};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use bytemuck::{from_bytes_mut, Pod, Zeroable};
use enum_dispatch::enum_dispatch;
use solana_program::{
    account_info::AccountInfo,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};
use std::sync::Arc;
use std::cell::RefMut;

/// Max number of accounts in registry
const MAX_REGISTRY_SIZE: usize = ((2 * 1024 * 1024) / 32) - 1;

/// Pool Registry
#[derive(Copy, Clone)]
#[repr(packed)]
pub struct PoolRegistry {
    /// Track if registry has been created
    pub is_initialized: bool,
    /// Current size of the registry array
    pub registry_size: u32,
    /// Array of pubkeys
    pub accounts: [Pubkey; MAX_REGISTRY_SIZE],
}
unsafe impl Zeroable for PoolRegistry {}
unsafe impl Pod for PoolRegistry {}

impl PoolRegistry {
    #[inline]
    /// Loads the registry byte blob into a struct
    pub fn load<'a>(
        registry_account: &'a AccountInfo,
        program_id: &Pubkey,
    ) -> Result<RefMut<'a, Self>, ProgramError> {
        if registry_account.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }
        let account_data: RefMut<'a, [u8]>;
        let state: RefMut<'a, Self>;

        account_data = RefMut::map(registry_account.try_borrow_mut_data()?, |data| *data);
        state = RefMut::map(account_data, |data| from_bytes_mut(data));

        Ok(state)
    }

    /// Adds a pubkey to the registry
    pub fn append(&mut self, key: &Pubkey) {
        self.accounts[PoolRegistry::index_of(self.registry_size)] = *key;
        self.registry_size += 1;
    }

    /// Remove the item at the provided index by replacing it with the last item in
    /// the registry and decreasing the size by 1. If the index provided IS the last
    /// item, the size is simply decreased by 1.
    /// Also clears out the last item in the list after (or if) moving.
    pub fn remove(&mut self, index: u32) -> Result<(), ProgramError> {
        if index >= self.registry_size {
            return Err(ProgramError::InvalidArgument);
        }

        let last_index = PoolRegistry::index_of(self.registry_size - 1);

        if index != self.registry_size - 1 {
            let last = self.accounts[last_index];
            self.accounts[PoolRegistry::index_of(index)] = last;
        }

        self.accounts[last_index] = Pubkey::default();
        self.registry_size -= 1;

        Ok(())
    }

    /// Gets a key by index
    pub fn index_of(counter: u32) -> usize {
        std::convert::TryInto::try_into(counter).unwrap()
    }
}

/// Trait representing access to program state across all versions
#[enum_dispatch]
pub trait SwapState {
    /// Is the swap initialized, with data written to it
    fn is_initialized(&self) -> bool;
    /// Bump seed used to generate the program address / authority
    fn nonce(&self) -> u8;
    /// Token program ID associated with the swap
    fn token_program_id(&self) -> &Pubkey;
    /// Address of token A liquidity account
    fn token_a_account(&self) -> &Pubkey;
    /// Address of token B liquidity account
    fn token_b_account(&self) -> &Pubkey;
    /// Address of pool token mint
    fn pool_mint(&self) -> &Pubkey;

    /// Address of token A mint
    fn token_a_mint(&self) -> &Pubkey;
    /// Address of token B mint
    fn token_b_mint(&self) -> &Pubkey;

    /// Address of pool fee account
    fn pool_fee_account(&self) -> &Pubkey;

    /// Fees associated with swap
    fn fees(&self) -> &Fees;
    /// Curve associated with swap
    fn swap_curve(&self) -> &SwapCurve;

    /// Bump seed used to generate the pool program address / authority
    fn pool_nonce(&self) -> u8;
}

/// All versions of SwapState
#[enum_dispatch(SwapState)]
pub enum SwapVersion {
    /// Latest version, used for all new swaps
    SwapV1,
}

/// SwapVersion does not implement program_pack::Pack because there are size
/// checks on pack and unpack that would break backwards compatibility, so
/// special implementations are provided here
impl SwapVersion {
    /// Size of the latest version of the SwapState
    pub const LATEST_LEN: usize = 1 + SwapV1::LEN; // add one for the version enum

    /// Pack a swap into a byte array, based on its version
    pub fn pack(src: Self, dst: &mut [u8]) -> Result<(), ProgramError> {
        match src {
            Self::SwapV1(swap_info) => {
                dst[0] = 1;
                SwapV1::pack(swap_info, &mut dst[1..])
            }
        }
    }

    /// Unpack the swap account based on its version, returning the result as a
    /// SwapState trait object
    pub fn unpack(input: &[u8]) -> Result<Arc<dyn SwapState>, ProgramError> {
        let (&version, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidAccountData)?;
        match version {
            1 => Ok(Arc::new(SwapV1::unpack(rest)?)),
            _ => Err(ProgramError::UninitializedAccount),
        }
    }

    /// Special check to be done before any instruction processing, works for
    /// all versions
    pub fn is_initialized(input: &[u8]) -> bool {
        match Self::unpack(input) {
            Ok(swap) => swap.is_initialized(),
            Err(_) => false,
        }
    }
}

/// Program states.
#[repr(C)]
#[derive(Debug, Default, PartialEq)]
pub struct SwapV1 {
    /// Initialized state.
    pub is_initialized: bool,
    /// Nonce used in program address.
    /// The program address is created deterministically with the nonce,
    /// swap program id, and swap account pubkey.  This program address has
    /// authority over the swap's token A account, token B account, and pool
    /// token mint.
    pub nonce: u8,

    /// Program ID of the tokens being exchanged.
    pub token_program_id: Pubkey,

    /// Token A
    pub token_a: Pubkey,
    /// Token B
    pub token_b: Pubkey,

    /// Pool tokens are issued when A or B tokens are deposited.
    /// Pool tokens can be withdrawn back to the original A or B token.
    pub pool_mint: Pubkey,

    /// Mint information for token A
    pub token_a_mint: Pubkey,
    /// Mint information for token B
    pub token_b_mint: Pubkey,

    /// Pool token account to receive trading and / or withdrawal fees
    pub pool_fee_account: Pubkey,

    /// All fee information
    pub fees: Fees,

    /// Swap curve parameters, to be unpacked and used by the SwapCurve, which
    /// calculates swaps, deposits, and withdrawals
    pub swap_curve: SwapCurve,

    /// Nonce used in pool program address.
    /// The program address is created deterministically with the nonce,
    /// swap program id, mint A, mint B, and curve type. This program address has
    /// authority over the pool account.
    pub pool_nonce: u8,
}

impl SwapState for SwapV1 {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }

    fn nonce(&self) -> u8 {
        self.nonce
    }

    fn token_program_id(&self) -> &Pubkey {
        &self.token_program_id
    }

    fn token_a_account(&self) -> &Pubkey {
        &self.token_a
    }

    fn token_b_account(&self) -> &Pubkey {
        &self.token_b
    }

    fn pool_mint(&self) -> &Pubkey {
        &self.pool_mint
    }

    fn token_a_mint(&self) -> &Pubkey {
        &self.token_a_mint
    }

    fn token_b_mint(&self) -> &Pubkey {
        &self.token_b_mint
    }

    fn pool_fee_account(&self) -> &Pubkey {
        &self.pool_fee_account
    }

    fn fees(&self) -> &Fees {
        &self.fees
    }

    fn swap_curve(&self) -> &SwapCurve {
        &self.swap_curve
    }

    fn pool_nonce(&self) -> u8 {
        self.pool_nonce
    }
}

impl Sealed for SwapV1 {}
impl IsInitialized for SwapV1 {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for SwapV1 {
    const LEN: usize = 308;

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, 308];
        let (
            is_initialized,
            nonce,
            token_program_id,
            token_a,
            token_b,
            pool_mint,
            token_a_mint,
            token_b_mint,
            pool_fee_account,
            fees,
            swap_curve,
            pool_nonce,
        ) = mut_array_refs![output, 1, 1, 32, 32, 32, 32, 32, 32, 32, 48, 33, 1];
        is_initialized[0] = self.is_initialized as u8;
        nonce[0] = self.nonce;
        token_program_id.copy_from_slice(self.token_program_id.as_ref());
        token_a.copy_from_slice(self.token_a.as_ref());
        token_b.copy_from_slice(self.token_b.as_ref());
        pool_mint.copy_from_slice(self.pool_mint.as_ref());
        token_a_mint.copy_from_slice(self.token_a_mint.as_ref());
        token_b_mint.copy_from_slice(self.token_b_mint.as_ref());
        pool_fee_account.copy_from_slice(self.pool_fee_account.as_ref());
        self.fees.pack_into_slice(&mut fees[..]);
        self.swap_curve.pack_into_slice(&mut swap_curve[..]);
        pool_nonce[0] = self.pool_nonce;
    }

    /// Unpacks a byte buffer into a [SwapV1](struct.SwapV1.html).
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, 308];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            is_initialized,
            nonce,
            token_program_id,
            token_a,
            token_b,
            pool_mint,
            token_a_mint,
            token_b_mint,
            pool_fee_account,
            fees,
            swap_curve,
            pool_nonce,
        ) = array_refs![input, 1, 1, 32, 32, 32, 32, 32, 32, 32, 48, 33, 1];
        Ok(Self {
            is_initialized: match is_initialized {
                [0] => false,
                [1] => true,
                _ => return Err(ProgramError::InvalidAccountData),
            },
            nonce: nonce[0],
            token_program_id: Pubkey::new_from_array(*token_program_id),
            token_a: Pubkey::new_from_array(*token_a),
            token_b: Pubkey::new_from_array(*token_b),
            pool_mint: Pubkey::new_from_array(*pool_mint),
            token_a_mint: Pubkey::new_from_array(*token_a_mint),
            token_b_mint: Pubkey::new_from_array(*token_b_mint),
            pool_fee_account: Pubkey::new_from_array(*pool_fee_account),
            fees: Fees::unpack_from_slice(fees)?,
            swap_curve: SwapCurve::unpack_from_slice(swap_curve)?,
            pool_nonce: pool_nonce[0],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::stable::StableCurve;

    use std::convert::TryInto;

    use bytemuck::try_zeroed_box;

    const TEST_FEES: Fees = Fees {
        trade_fee_numerator: 1,
        trade_fee_denominator: 4,
        owner_trade_fee_numerator: 3,
        owner_trade_fee_denominator: 10,
        owner_withdraw_fee_numerator: 2,
        owner_withdraw_fee_denominator: 7,
    };

    const TEST_NONCE: u8 = 255;
    const TEST_TOKEN_PROGRAM_ID: Pubkey = Pubkey::new_from_array([1u8; 32]);
    const TEST_TOKEN_A: Pubkey = Pubkey::new_from_array([2u8; 32]);
    const TEST_TOKEN_B: Pubkey = Pubkey::new_from_array([3u8; 32]);
    const TEST_POOL_MINT: Pubkey = Pubkey::new_from_array([4u8; 32]);
    const TEST_TOKEN_A_MINT: Pubkey = Pubkey::new_from_array([5u8; 32]);
    const TEST_TOKEN_B_MINT: Pubkey = Pubkey::new_from_array([6u8; 32]);
    const TEST_POOL_FEE_ACCOUNT: Pubkey = Pubkey::new_from_array([7u8; 32]);

    const TEST_CURVE_TYPE: u8 = 2;
    const TEST_AMP: u64 = 1;
    const TEST_CURVE: StableCurve = StableCurve { amp: TEST_AMP };
    const TEST_POOL_NONCE: u8 = 250;

    #[test]
    fn pool_registry_pack() {
        let mut pool_registry: Box<PoolRegistry> = try_zeroed_box().unwrap();
        pool_registry.append(&TEST_TOKEN_A);
        pool_registry.append(&TEST_TOKEN_B);
        let regsize_ref = std::ptr::addr_of!(pool_registry.registry_size);
        let registry_size = unsafe { regsize_ref.read_unaligned() };
        assert!(!pool_registry.is_initialized);
        assert_eq!(registry_size, 2);
        assert_eq!(pool_registry.accounts[0], TEST_TOKEN_A);
        assert_eq!(pool_registry.accounts[1], TEST_TOKEN_B);
    }

    #[test]
    fn pool_registry_remove() {
        let mut pool_registry: Box<PoolRegistry> = try_zeroed_box().unwrap();
        pool_registry.append(&Pubkey::new_unique());
        pool_registry.append(&Pubkey::new_unique());
        let mid = Pubkey::new_unique();
        pool_registry.append(&mid);
        pool_registry.append(&Pubkey::new_unique());
        let last = Pubkey::new_unique();
        pool_registry.append(&last);

        assert_eq!(pool_registry.accounts[2], mid);
        assert_eq!(pool_registry.accounts[4], last);
        let regsize_ref = std::ptr::addr_of!(pool_registry.registry_size);
        let registry_size = unsafe { regsize_ref.read_unaligned() };
        assert_eq!(registry_size, 5u32);

        pool_registry.remove(2).unwrap();

        assert_eq!(pool_registry.accounts[2], last);
        let regsize_ref = std::ptr::addr_of!(pool_registry.registry_size);
        let registry_size = unsafe { regsize_ref.read_unaligned() };
        assert_eq!(registry_size, 4u32);
    }

    #[test]
    fn swap_version_pack() {
        let curve_type = TEST_CURVE_TYPE.try_into().unwrap();
        let calculator = Arc::new(TEST_CURVE);
        let swap_curve = SwapCurve {
            curve_type,
            calculator,
        };
        let swap_info = SwapVersion::SwapV1(SwapV1 {
            is_initialized: true,
            nonce: TEST_NONCE,
            token_program_id: TEST_TOKEN_PROGRAM_ID,
            token_a: TEST_TOKEN_A,
            token_b: TEST_TOKEN_B,
            pool_mint: TEST_POOL_MINT,
            token_a_mint: TEST_TOKEN_A_MINT,
            token_b_mint: TEST_TOKEN_B_MINT,
            pool_fee_account: TEST_POOL_FEE_ACCOUNT,
            fees: TEST_FEES,
            swap_curve: swap_curve.clone(),
            pool_nonce: TEST_POOL_NONCE,
        });

        let mut packed = [0u8; SwapVersion::LATEST_LEN];
        SwapVersion::pack(swap_info, &mut packed).unwrap();
        let unpacked = SwapVersion::unpack(&packed).unwrap();

        assert!(unpacked.is_initialized());
        assert_eq!(unpacked.nonce(), TEST_NONCE);
        assert_eq!(*unpacked.token_program_id(), TEST_TOKEN_PROGRAM_ID);
        assert_eq!(*unpacked.token_a_account(), TEST_TOKEN_A);
        assert_eq!(*unpacked.token_b_account(), TEST_TOKEN_B);
        assert_eq!(*unpacked.pool_mint(), TEST_POOL_MINT);
        assert_eq!(*unpacked.token_a_mint(), TEST_TOKEN_A_MINT);
        assert_eq!(*unpacked.token_b_mint(), TEST_TOKEN_B_MINT);
        assert_eq!(*unpacked.pool_fee_account(), TEST_POOL_FEE_ACCOUNT);
        assert_eq!(*unpacked.fees(), TEST_FEES);
        assert_eq!(*unpacked.swap_curve(), swap_curve);
        assert_eq!(unpacked.pool_nonce(), TEST_POOL_NONCE);
    }

    #[test]
    fn swap_v1_pack() {
        let curve_type = TEST_CURVE_TYPE.try_into().unwrap();
        let calculator = Arc::new(TEST_CURVE);
        let swap_curve = SwapCurve {
            curve_type,
            calculator,
        };
        let swap_info = SwapV1 {
            is_initialized: true,
            nonce: TEST_NONCE,
            token_program_id: TEST_TOKEN_PROGRAM_ID,
            token_a: TEST_TOKEN_A,
            token_b: TEST_TOKEN_B,
            pool_mint: TEST_POOL_MINT,
            token_a_mint: TEST_TOKEN_A_MINT,
            token_b_mint: TEST_TOKEN_B_MINT,
            pool_fee_account: TEST_POOL_FEE_ACCOUNT,
            fees: TEST_FEES,
            swap_curve,
            pool_nonce: TEST_POOL_NONCE,
        };

        let mut packed = [0u8; SwapV1::LEN];
        SwapV1::pack_into_slice(&swap_info, &mut packed);
        let unpacked = SwapV1::unpack(&packed).unwrap();
        assert_eq!(swap_info, unpacked);

        let mut packed = vec![1u8, TEST_NONCE];
        packed.extend_from_slice(&TEST_TOKEN_PROGRAM_ID.to_bytes());
        packed.extend_from_slice(&TEST_TOKEN_A.to_bytes());
        packed.extend_from_slice(&TEST_TOKEN_B.to_bytes());
        packed.extend_from_slice(&TEST_POOL_MINT.to_bytes());
        packed.extend_from_slice(&TEST_TOKEN_A_MINT.to_bytes());
        packed.extend_from_slice(&TEST_TOKEN_B_MINT.to_bytes());
        packed.extend_from_slice(&TEST_POOL_FEE_ACCOUNT.to_bytes());
        packed.extend_from_slice(&TEST_FEES.trade_fee_numerator.to_le_bytes());
        packed.extend_from_slice(&TEST_FEES.trade_fee_denominator.to_le_bytes());
        packed.extend_from_slice(&TEST_FEES.owner_trade_fee_numerator.to_le_bytes());
        packed.extend_from_slice(&TEST_FEES.owner_trade_fee_denominator.to_le_bytes());
        packed.extend_from_slice(&TEST_FEES.owner_withdraw_fee_numerator.to_le_bytes());
        packed.extend_from_slice(&TEST_FEES.owner_withdraw_fee_denominator.to_le_bytes());
        packed.push(TEST_CURVE_TYPE);
        packed.extend_from_slice(&TEST_AMP.to_le_bytes());
        packed.extend_from_slice(&[0u8; 24]);
        packed.push(TEST_POOL_NONCE);
        let unpacked = SwapV1::unpack(&packed).unwrap();
        assert_eq!(swap_info, unpacked);

        let packed = [0u8; SwapV1::LEN];
        let swap_info: SwapV1 = Default::default();
        let unpack_unchecked = SwapV1::unpack_unchecked(&packed).unwrap();
        assert_eq!(unpack_unchecked, swap_info);
        let err = SwapV1::unpack(&packed).unwrap_err();
        assert_eq!(err, ProgramError::UninitializedAccount);
    }
}
