//! Swap price calculator based on the Serum market

#![cfg(feature = "serum")]

use crate::curve::calculator::{
    calculate_fee, map_zero_to_none, CurveCalculator, DynPack, SwapResult,
};
use crate::error::SwapError;

use solana_program::{
    account_info::AccountInfo,
    pubkey::Pubkey,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};

use serum_dex::{
    critbit::{LeafNode, Slab, SlabView},
    state::{AccountFlag},
    error::{DexError, DexErrorCode, DexResult},
};

use bytemuck::{cast_mut, cast_slice_mut, try_from_bytes_mut, try_cast_slice_mut, Zeroable, Pod};
use enumflags2::BitFlags;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use std::{
    cell::RefMut,
    convert::TryFrom,
    num::NonZeroU64,
};
use std::mem::{align_of, size_of};

/// Calculator based on observable Serum market
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SerumMarketCurve {
    /// Trade fee numerator
    pub trade_fee_numerator: u64,
    /// Trade fee denominator
    pub trade_fee_denominator: u64,
    /// Owner trade fee numerator
    pub owner_trade_fee_numerator: u64,
    /// Owner trade fee denominator
    pub owner_trade_fee_denominator: u64,
    /// Owner withdraw fee numerator
    pub owner_withdraw_fee_numerator: u64,
    /// Owner withdraw fee denominator
    pub owner_withdraw_fee_denominator: u64,
    /// Host trading fee numerator
    pub host_fee_numerator: u64,
    /// Host trading fee denominator
    pub host_fee_denominator: u64,
    /*
    /// Address of token A mint, for validation
    pub token_a_mint: Pubkey,
    /// Address of token A bids
    pub token_a_bids: Pubkey,
    /// Address of token A asks
    pub token_a_asks: Pubkey,
    /// Address of token B mint, for validation
    pub token_b_mint: Pubkey,
    /// Address of token B bids
    pub token_b_bids: Pubkey,
    /// Address of token B asks
    pub token_b_asks: Pubkey,
    */
}

/// The following is an adaptation of the code in critbit.rs and state.rs from
/// serum dex, specially adapted for unaligned or aligned accounts.  Since
/// token-swap uses `entrypoint` with aligned data structures, and serum uses
/// `entrypoint_deprecated` with unaligned data structures, we need to take
/// cover of both situations just in case.
#[derive(Copy, Clone)]
#[repr(C)]
struct OrderBookStateHeader {
    account_flags: u64, // Initialized, (Bids or Asks)
}
unsafe impl Zeroable for OrderBookStateHeader {}
unsafe impl Pod for OrderBookStateHeader {}

/// Extra padding at the start for `entrypoint_deprecated`
const ACCOUNT_HEAD_PADDING: &[u8; 5] = b"serum";
/// Extra padding at the end for `entrypoint_deprecated`
const ACCOUNT_TAIL_PADDING: &[u8; 7] = b"padding";

fn init_account_padding(data: &mut [u8]) -> DexResult<&mut [u64]> {
    assert!(data.len() >= 12);
    let (head, data, tail) = mut_array_refs![data, 5; ..; 7];
    *head = *ACCOUNT_HEAD_PADDING;
    *tail = *ACCOUNT_TAIL_PADDING;
    try_cast_slice_mut(data).map_err(|_e| DexError::ErrorCode(DexErrorCode::WrongAccountDataAlignment))
}

fn check_account_padding(data: &mut [u8]) -> DexResult<&mut [u64]> {
    assert!(data.len() >= 12);
    let (head, data, tail) = mut_array_refs![data, 5; ..; 7];
    assert_eq!(head, ACCOUNT_HEAD_PADDING);
    assert_eq!(tail, ACCOUNT_TAIL_PADDING);
    try_cast_slice_mut(data).map_err(|_e| DexError::ErrorCode(DexErrorCode::WrongAccountDataAlignment))
}

fn strip_account_padding(padded_data: &mut [u8], init_allowed: bool) -> DexResult<&mut [u64]> {
    if init_allowed {
        init_account_padding(padded_data)
    } else {
        check_account_padding(padded_data)
    }
}

#[inline]
fn remove_slop_mut<T: Pod>(bytes: &mut [u8]) -> &mut [T] {
    let slop = bytes.len() % size_of::<T>();
    let new_len = bytes.len() - slop;
    cast_slice_mut(&mut bytes[..new_len])
}

fn strip_header<'a, H: Pod, D: Pod>(
    account: &'a AccountInfo,
    init_allowed: bool,
) -> DexResult<(RefMut<'a, H>, RefMut<'a, [D]>)> {
    let mut result = Ok(());
    let (header, inner): (RefMut<'a, [H]>, RefMut<'a, [D]>) =
        RefMut::map_split(account.try_borrow_mut_data()?, |padded_data| {
            let dummy_value: (&mut [H], &mut [D]) = (&mut [], &mut []);
            let padded_data: &mut [u8] = *padded_data;

            let u64_result = if (padded_data.as_ptr() as usize) % align_of::<u64>() == 0 {
                try_cast_slice_mut(padded_data).map_err(|_e| DexError::ErrorCode(DexErrorCode::WrongAccountDataAlignment))
            } else {
                strip_account_padding(padded_data, init_allowed)
            };
            let u64_data = match u64_result {
                Ok(u64_data) => u64_data,
                Err(e) => {
                    result = Err(e);
                    return dummy_value;
                }
            };

            let data: &mut [u8] = cast_slice_mut(u64_data);
            let (header_bytes, inner_bytes) = data.split_at_mut(size_of::<H>());
            let header: &mut H;
            let inner: &mut [D];

            header = match try_from_bytes_mut(header_bytes) {
                Ok(h) => h,
                Err(_e) => {
                    result = Err(DexError::ErrorCode(DexErrorCode::InvalidMarketFlags));
                    return dummy_value;
                }
            };
            inner = remove_slop_mut(inner_bytes);

            (std::slice::from_mut(header), inner)
        });
    result?;
    let header = RefMut::map(header, |s| s.first_mut().unwrap_or_else(|| unreachable!()));
    Ok((header, inner))
}

fn unpack_bids<'a>(bids: &'a AccountInfo) -> Result<RefMut<'a, Slab>, ProgramError> {
    let (header, buf) = strip_header::<OrderBookStateHeader, u8>(bids, false)?;
    let flags = BitFlags::from_bits(header.account_flags).unwrap();
    let required_flags = AccountFlag::Initialized | AccountFlag::Bids;
    if flags == required_flags {
        Ok(RefMut::map(buf, Slab::new))
    } else {
        Err(SwapError::InvalidOrderbook.into())
    }
}

fn unpack_asks<'a>(asks: &'a AccountInfo) -> Result<RefMut<'a, Slab>, ProgramError> {
    let (header, buf) = strip_header::<OrderBookStateHeader, u8>(asks, false)?;
    let flags = BitFlags::from_bits(header.account_flags).unwrap();
    let required_flags = AccountFlag::Initialized | AccountFlag::Asks;
    if flags == required_flags {
        Ok(RefMut::map(buf, Slab::new))
    } else {
        Err(SwapError::InvalidOrderbook.into())
    }
}

impl CurveCalculator for SerumMarketCurve {
    /// Constant product swap ensures x * y = constant
    fn swap(
        &self,
        source_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
        accounts: &[AccountInfo],
    ) -> Option<SwapResult> {
        // get the price for
        // debit the fee to calculate the amount swapped
        let new_source_amount = swap_source_amount;
        let new_destination_amount = swap_destination_amount;
        let amount_swapped = 0;
        let trade_fee = 0;
        let owner_fee = 0;
        Some(SwapResult {
            new_source_amount,
            new_destination_amount,
            amount_swapped,
            trade_fee,
            owner_fee,
        })
    }

    /// Calculate the withdraw fee in pool tokens
    fn owner_withdraw_fee(&self, pool_tokens: u128) -> Option<u128> {
        calculate_fee(
            pool_tokens,
            u128::try_from(self.owner_withdraw_fee_numerator).ok()?,
            u128::try_from(self.owner_withdraw_fee_denominator).ok()?,
        )
    }

    /// Calculate the trading fee in trading tokens
    fn trading_fee(&self, trading_tokens: u128) -> Option<u128> {
        calculate_fee(
            trading_tokens,
            u128::try_from(self.trade_fee_numerator).ok()?,
            u128::try_from(self.trade_fee_denominator).ok()?,
        )
    }

    /// Calculate the host fee based on the owner fee, only used in production
    /// situations where a program is hosted by multiple frontends
    fn host_fee(&self, owner_fee: u128) -> Option<u128> {
        calculate_fee(
            owner_fee,
            u128::try_from(self.host_fee_numerator).ok()?,
            u128::try_from(self.host_fee_denominator).ok()?,
        )
    }

    /// Validate mints provided in the swap instruction, ensuring
    /// that the curve and swap instruction are aligned. For example, if the
    /// swap goes from token A to token B, and the associated curve accounts
    /// are provided in the order required for a swap from token A to token B.
    /// This prevents an attack of declaring a swap from token A to token B,
    /// but providing reference accounts for a token B to token A swap.
    fn validate_swap_accounts(
        &self,
        _source_mint: &Pubkey,
        _destination_mint: &Pubkey,
        _curve_accounts: &[AccountInfo],
    ) -> Result<(), SwapError> {
        Ok(())
    }
}

/// IsInitialized is required to use `Pack::pack` and `Pack::unpack`
impl IsInitialized for SerumMarketCurve {
    fn is_initialized(&self) -> bool {
        true
    }
}
impl Sealed for SerumMarketCurve {}
impl Pack for SerumMarketCurve {
    const LEN: usize = 64;
    fn unpack_from_slice(input: &[u8]) -> Result<SerumMarketCurve, ProgramError> {
        let input = array_ref![input, 0, 64];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
        ) = array_refs![input, 8, 8, 8, 8, 8, 8, 8, 8];
        Ok(Self {
            trade_fee_numerator: u64::from_le_bytes(*trade_fee_numerator),
            trade_fee_denominator: u64::from_le_bytes(*trade_fee_denominator),
            owner_trade_fee_numerator: u64::from_le_bytes(*owner_trade_fee_numerator),
            owner_trade_fee_denominator: u64::from_le_bytes(*owner_trade_fee_denominator),
            owner_withdraw_fee_numerator: u64::from_le_bytes(*owner_withdraw_fee_numerator),
            owner_withdraw_fee_denominator: u64::from_le_bytes(*owner_withdraw_fee_denominator),
            host_fee_numerator: u64::from_le_bytes(*host_fee_numerator),
            host_fee_denominator: u64::from_le_bytes(*host_fee_denominator),
        })
    }

    fn pack_into_slice(&self, output: &mut [u8]) {
        (self as &dyn DynPack).pack_into_slice(output);
    }
}

impl DynPack for SerumMarketCurve {
    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, 64];
        let (
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
        ) = mut_array_refs![output, 8, 8, 8, 8, 8, 8, 8, 8];
        *trade_fee_numerator = self.trade_fee_numerator.to_le_bytes();
        *trade_fee_denominator = self.trade_fee_denominator.to_le_bytes();
        *owner_trade_fee_numerator = self.owner_trade_fee_numerator.to_le_bytes();
        *owner_trade_fee_denominator = self.owner_trade_fee_denominator.to_le_bytes();
        *owner_withdraw_fee_numerator = self.owner_withdraw_fee_numerator.to_le_bytes();
        *owner_withdraw_fee_denominator = self.owner_withdraw_fee_denominator.to_le_bytes();
        *host_fee_numerator = self.host_fee_numerator.to_le_bytes();
        *host_fee_denominator = self.host_fee_denominator.to_le_bytes();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serum_dex::fees::FeeTier;

    const MAX_PRICE: u128 = 10001;
    const MIN_PRICE: u128 = 400;

    fn to_account_info<'a>(key: &'a Pubkey, lamports: &'a mut u64, data: &'a mut [u8],) -> AccountInfo<'a> {
        AccountInfo::new(
            key,
            false,
            false,
            lamports,
            data,
            key,
            false,
            1,)
    }

    fn fill_orderbook(slab: &mut Slab) {
        let mut owner_slot = 1;
        let price = 10000 << 64;
        let owner = [1, 1, 1, 1];
        let quantity = 123;
        let fee_tier = FeeTier::Base;
        let mut order_id = 1;

        let leaf = LeafNode::new(owner_slot, &price, &owner, quantity, fee_tier, order_id);
        slab.insert_leaf(&leaf).unwrap();
        let price = MAX_PRICE << 64;
        owner_slot += 1;
        order_id += 1;
        let leaf = LeafNode::new(owner_slot, &price, &owner, quantity, fee_tier, order_id);
        slab.insert_leaf(&leaf).unwrap();
        let price = MIN_PRICE << 64;
        owner_slot += 1;
        order_id += 1;
        let leaf = LeafNode::new(owner_slot, &price, &owner, quantity, fee_tier, order_id);
        slab.insert_leaf(&leaf).unwrap();
    }

    fn check_min_max(slab: &Slab) {
        let max_node_handle = slab.find_max().unwrap();
        let max_ref = slab.get(max_node_handle).unwrap().as_leaf().unwrap();
        let price = max_ref.price();
        assert_eq!(price, NonZeroU64::new(MAX_PRICE as u64).unwrap());

        let min_node_handle = slab.find_min().unwrap();
        let min_ref = slab.get(min_node_handle).unwrap().as_leaf().unwrap();
        let price = min_ref.price();
        assert_eq!(price, NonZeroU64::new(MIN_PRICE as u64).unwrap());
    }

    #[test]
    fn unaligned_orderbook_unpack_empty() {
        let mut bytes = vec![0u8; 10_000];
        let slab = Slab::new(&mut bytes);
        assert_eq!(slab.find_min(), None);
        assert_eq!(slab.find_max(), None);

        {
            let mut ob_data = vec![0u8; 10_007]; // 3 padding + 5 + header (8) + size + 7
            let ob_view = init_account_padding(&mut ob_data[3..]).unwrap();
            const OB_HEADER_WORDS: usize = size_of::<OrderBookStateHeader>() / size_of::<u64>();
            assert!(ob_view.len() > OB_HEADER_WORDS);
            let (hdr_array, slab_words) = mut_array_refs![ob_view, OB_HEADER_WORDS; .. ;];
            let ob_hdr: &mut OrderBookStateHeader = cast_mut(hdr_array);
            *ob_hdr = OrderBookStateHeader {
                account_flags: (AccountFlag::Initialized | AccountFlag::Bids).bits(),
            };
            let _slab = Slab::new(cast_slice_mut(slab_words));
            let key = Pubkey::new_unique();
            let mut lamports = 0;
            let account_info = to_account_info(&key, &mut lamports, &mut ob_data[3..]);
            let bids = unpack_bids(&account_info).unwrap();
            assert_eq!(bids.find_min(), None);
            assert_eq!(bids.find_max(), None);
        }

        {
            let mut ob_data = vec![0u8; 10_007]; // 3 padding + 5 + header (8) + size + 7
            let ob_view = init_account_padding(&mut ob_data[3..]).unwrap();
            const OB_HEADER_WORDS: usize = size_of::<OrderBookStateHeader>() / size_of::<u64>();
            assert!(ob_view.len() > OB_HEADER_WORDS);
            let (hdr_array, slab_words) = mut_array_refs![ob_view, OB_HEADER_WORDS; .. ;];
            let ob_hdr: &mut OrderBookStateHeader = cast_mut(hdr_array);
            *ob_hdr = OrderBookStateHeader {
                account_flags: (AccountFlag::Initialized | AccountFlag::Asks).bits(),
            };
            let _slab = Slab::new(cast_slice_mut(slab_words));
            let key = Pubkey::new_unique();
            let mut lamports = 0;
            let account_info = to_account_info(&key, &mut lamports, &mut ob_data[3..]);
            let asks = unpack_asks(&account_info).unwrap();
            assert_eq!(asks.find_min(), None);
            assert_eq!(asks.find_max(), None);
        }
    }

    #[test]
    fn aligned_orderbook_unpack_empty() {
        let mut bytes = vec![0u8; 10_000];
        let slab = Slab::new(&mut bytes);
        assert_eq!(slab.find_min(), None);
        assert_eq!(slab.find_max(), None);

        {
            let mut ob_data = vec![0u8; 10_000]; // header (8) + size
            let (hdr_array, slab_words) = mut_array_refs![&mut ob_data, size_of::<OrderBookStateHeader>(); .. ;];
            let ob_hdr: &mut OrderBookStateHeader = cast_mut(hdr_array);
            *ob_hdr = OrderBookStateHeader {
                account_flags: (AccountFlag::Initialized | AccountFlag::Bids).bits(),
            };
            let _slab = Slab::new(cast_slice_mut(slab_words));
            let key = Pubkey::new_unique();
            let mut lamports = 0;
            let account_info = to_account_info(&key, &mut lamports, &mut ob_data);
            let bids = unpack_bids(&account_info).unwrap();
            assert_eq!(bids.find_min(), None);
            assert_eq!(bids.find_max(), None);
        }

        {
            let mut ob_data = vec![0u8; 10_000]; // header (8) + size
            let (hdr_array, slab_words) = mut_array_refs![&mut ob_data, size_of::<OrderBookStateHeader>(); .. ;];
            let ob_hdr: &mut OrderBookStateHeader = cast_mut(hdr_array);
            *ob_hdr = OrderBookStateHeader {
                account_flags: (AccountFlag::Initialized | AccountFlag::Asks).bits(),
            };
            let _slab = Slab::new(cast_slice_mut(slab_words));
            let key = Pubkey::new_unique();
            let mut lamports = 0;
            let account_info = to_account_info(&key, &mut lamports, &mut ob_data);
            let asks = unpack_asks(&account_info).unwrap();
            assert_eq!(asks.find_min(), None);
            assert_eq!(asks.find_max(), None);
        }
    }

    #[test]
    fn unaligned_orderbook_unpack_filled() {
        let mut bytes = vec![0u8; 10_000];
        let mut slab = Slab::new(&mut bytes);

        fill_orderbook(&mut slab);

        let mut ob_data = vec![0u8; 10_023]; // 3 padding + 5 + header (8) + size + 7
        let ob_view = init_account_padding(&mut ob_data[3..]).unwrap();
        const OB_HEADER_WORDS: usize = size_of::<OrderBookStateHeader>() / size_of::<u64>();
        assert!(ob_view.len() > OB_HEADER_WORDS);
        let (hdr_array, slab_words) = mut_array_refs![ob_view, OB_HEADER_WORDS; .. ;];
        let ob_hdr: &mut OrderBookStateHeader = cast_mut(hdr_array);
        *ob_hdr = OrderBookStateHeader {
            account_flags: (AccountFlag::Initialized | AccountFlag::Bids).bits(),
        };
        let slab_data: &mut [u8] = cast_slice_mut(slab_words);
        slab_data.clone_from_slice(&bytes);
        let _slab = Slab::new(cast_slice_mut(slab_words));
        let key = Pubkey::new_unique();
        let mut lamports = 0;
        let account_info = to_account_info(&key, &mut lamports, &mut ob_data[3..]);
        let unpacked = unpack_bids(&account_info).unwrap();
        check_min_max(&unpacked);
    }

    #[test]
    fn aligned_orderbook_unpack_filled() {
        let mut bytes = vec![0u8; 10_000];
        let mut slab = Slab::new(&mut bytes);

        fill_orderbook(&mut slab);

        let mut ob_data = vec![0u8; 10_008]; // header (8) + size
        let (hdr_array, slab_words) = mut_array_refs![&mut ob_data, size_of::<OrderBookStateHeader>(); .. ;];
        let ob_hdr: &mut OrderBookStateHeader = cast_mut(hdr_array);
        *ob_hdr = OrderBookStateHeader {
            account_flags: (AccountFlag::Initialized | AccountFlag::Bids).bits(),
        };
        let slab_data: &mut [u8] = cast_slice_mut(slab_words);
        slab_data.clone_from_slice(&bytes);
        let slab = Slab::new(cast_slice_mut(slab_words));
        let key = Pubkey::new_unique();
        let mut lamports = 0;
        let account_info = to_account_info(&key, &mut lamports, &mut ob_data);
        let unpacked = unpack_bids(&account_info).unwrap();
        check_min_max(&unpacked);
    }

    #[test]
    fn orderbook_find_max_min() {
        let mut bytes = vec![0u8; 80_000];
        let mut slab = Slab::new(&mut bytes);

        assert_eq!(slab.find_min(), None);
        assert_eq!(slab.find_max(), None);

        fill_orderbook(&mut slab);
        check_min_max(&slab);
    }

    #[test]
    #[ignore]
    fn swap_no_fee() {
        let swap_source_amount: u128 = 1000;
        let swap_destination_amount: u128 = 50000;
        let source_amount: u128 = 100;
        let curve = SerumMarketCurve::default();
        let result = curve
            .swap(source_amount, swap_source_amount, swap_destination_amount, &[])
            .unwrap();
        assert_eq!(result.new_source_amount, 1100);
        assert_eq!(result.amount_swapped, 4546);
        assert_eq!(result.new_destination_amount, 45454);
    }

    #[test]
    fn pack_curve() {
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 4;
        let owner_trade_fee_numerator = 2;
        let owner_trade_fee_denominator = 5;
        let owner_withdraw_fee_numerator = 4;
        let owner_withdraw_fee_denominator = 10;
        let host_fee_numerator = 4;
        let host_fee_denominator = 10;
        let curve = SerumMarketCurve {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
        };

        let mut packed = [0u8; SerumMarketCurve::LEN];
        Pack::pack_into_slice(&curve, &mut packed[..]);
        let unpacked = SerumMarketCurve::unpack(&packed).unwrap();
        assert_eq!(curve, unpacked);

        let mut packed = vec![];
        packed.extend_from_slice(&trade_fee_numerator.to_le_bytes());
        packed.extend_from_slice(&trade_fee_denominator.to_le_bytes());
        packed.extend_from_slice(&owner_trade_fee_numerator.to_le_bytes());
        packed.extend_from_slice(&owner_trade_fee_denominator.to_le_bytes());
        packed.extend_from_slice(&owner_withdraw_fee_numerator.to_le_bytes());
        packed.extend_from_slice(&owner_withdraw_fee_denominator.to_le_bytes());
        packed.extend_from_slice(&host_fee_numerator.to_le_bytes());
        packed.extend_from_slice(&host_fee_denominator.to_le_bytes());
        let unpacked = SerumMarketCurve::unpack(&packed).unwrap();
        assert_eq!(curve, unpacked);
    }
}
