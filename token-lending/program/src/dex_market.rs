//! Dex market used for simulating trades

use crate::{error::LendingError, math::Decimal, state::Reserve};
use arrayref::{array_refs, mut_array_refs};
use serum_dex::critbit::Slab;
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};
use std::{cell::RefMut, collections::VecDeque, convert::TryFrom};

/// Side of the dex market order book
#[derive(PartialEq)]
enum Side {
    Bid,
    Ask,
}

/// Input currency for trade simulator
#[derive(PartialEq)]
enum Currency {
    Base,
    Quote,
}

/// The role of the trader
#[derive(PartialEq)]
pub enum Role {
    /// Trade as the taker. Buy from asks and Sell to bids.
    Taker,
    /// Trade as the maker. Sell as bidder and Buy as asker.
    Maker,
}

/// Dex market order
struct Order {
    price: u64,
    quantity: u64,
}

/// Dex market orders used for simulating trades
enum Orders<'a, 'b: 'a> {
    DexMarket(DexMarketOrders<'a, 'b>),
    Cached(VecDeque<Order>),
    None,
}

impl Iterator for Orders<'_, '_> {
    type Item = Order;

    fn next(&mut self) -> Option<Order> {
        match self {
            Orders::DexMarket(dex_market_orders) => {
                let leaf_node = match dex_market_orders.side {
                    Side::Bid => dex_market_orders
                        .heap
                        .as_mut()
                        .and_then(|heap| heap.remove_max()),
                    Side::Ask => dex_market_orders
                        .heap
                        .as_mut()
                        .and_then(|heap| heap.remove_min()),
                }?;

                Some(Order {
                    price: leaf_node.price().get(),
                    quantity: leaf_node.quantity(),
                })
            }
            Orders::Cached(orders) => orders.pop_front(),
            _ => None,
        }
    }
}

/// Trade simulator
pub struct TradeSimulator<'a, 'b: 'a> {
    input_currency: Currency,
    input_lots: u64,
    output_lots: u64,
    orders: Orders<'a, 'b>,
}

impl<'a, 'b: 'a> TradeSimulator<'a, 'b> {
    /// Create a new TradeSimulator
    pub fn new(
        dex_market_info: &AccountInfo,
        dex_market_orders: &AccountInfo,
        memory: &'a AccountInfo<'b>,
        input_reserve: &Reserve,
        role: Role,
    ) -> Result<Self, ProgramError> {
        let dex_market = DexMarket::new(dex_market_info);
        let dex_market_orders = DexMarketOrders::new(&dex_market, dex_market_orders, memory)?;
        let (input_currency, input_lots, output_lots) =
            if input_reserve.liquidity_mint == dex_market.base_mint {
                (Currency::Base, dex_market.base_lots, dex_market.quote_lots)
            } else {
                (Currency::Quote, dex_market.quote_lots, dex_market.base_lots)
            };

        let order_book_side = if role == Role::Taker {
            if input_currency == Currency::Base {
                Side::Ask
            } else {
                Side::Bid
            }
        } else {
            if input_currency == Currency::Base {
                Side::Bid
            } else {
                Side::Ask
            }
        };

        if order_book_side != dex_market_orders.side {
            return Err(LendingError::DexInvalidOrderBookSide.into());
        }

        Ok(Self {
            input_currency,
            input_lots,
            output_lots,
            orders: Orders::DexMarket(dex_market_orders),
        })
    }

    /// Simulate a trade
    pub fn simulate_trade(
        &mut self,
        amount: Decimal,
        cache_orders: bool,
    ) -> Result<Decimal, ProgramError> {
        let input_quantity: Decimal = amount / self.input_lots;
        let output_quantity = self.exchange_with_order_book(cache_orders, input_quantity)?;
        Ok(output_quantity * self.output_lots)
    }

    /// Calculate output quantity from input using order book depth
    fn exchange_with_order_book(
        &mut self,
        cache_orders: bool,
        mut input_quantity: Decimal,
    ) -> Result<Decimal, ProgramError> {
        let mut output_quantity = Decimal::zero();
        let mut orders = std::mem::replace(&mut self.orders, Orders::None).into_iter();

        if cache_orders {
            self.orders = Orders::Cached(VecDeque::new())
        }

        let zero = Decimal::zero();
        while input_quantity > zero {
            let next_order = orders
                .next()
                .ok_or_else(|| ProgramError::from(LendingError::DexOrderBookError))?;

            let next_order_price = next_order.price;
            let base_quantity = next_order.quantity;

            let (filled, output) = if self.input_currency == Currency::Base {
                let filled = input_quantity.min(Decimal::from(base_quantity));
                (filled, filled * next_order_price)
            } else {
                let quote_quantity = base_quantity as u128 * next_order_price as u128;
                let filled = input_quantity.min(Decimal::from(quote_quantity));
                (filled, filled / next_order_price)
            };

            input_quantity -= filled;
            output_quantity += output;

            if let Orders::Cached(orders) = &mut self.orders {
                orders.push_back(next_order);
            }
        }

        Ok(output_quantity)
    }
}

/// Dex market order account info
pub struct DexMarketOrders<'a, 'b: 'a> {
    heap: Option<RefMut<'a, Slab>>,
    memory: &'a AccountInfo<'b>,
    side: Side,
}

impl<'a, 'b: 'a> DexMarketOrders<'a, 'b> {
    /// Create a new DexMarketOrders
    pub fn new(
        dex_market: &DexMarket,
        orders: &AccountInfo,
        memory: &'a AccountInfo<'b>,
    ) -> Result<Self, ProgramError> {
        let side = match orders.key {
            key if key == &dex_market.bids => Side::Bid,
            key if key == &dex_market.asks => Side::Ask,
            _ => return Err(LendingError::DexInvalidOrderBookSide.into()),
        };

        if memory.data_len() < orders.data_len() {
            return Err(LendingError::MemoryTooSmall.into());
        }

        let mut memory_data = memory.data.borrow_mut();
        fast_copy(&orders.data.borrow(), &mut memory_data);
        let heap = Some(RefMut::map(memory_data, |bytes| {
            // strip padding and header
            let start = 5 + 8;
            let end = bytes.len() - 7;
            Slab::new(&mut bytes[start..end])
        }));

        Ok(Self { heap, memory, side })
    }
}

impl Drop for DexMarketOrders<'_, '_> {
    fn drop(&mut self) {
        self.heap.take();
        fast_set(&mut self.memory.data.borrow_mut(), 0);
    }
}

/// Offset for dex market base mint
pub const BASE_MINT_OFFSET: usize = 6;
/// Offset for dex market quote mint
pub const QUOTE_MINT_OFFSET: usize = 10;

const BIDS_OFFSET: usize = 35;
const ASKS_OFFSET: usize = 39;

/// Dex market info
pub struct DexMarket {
    base_mint: Pubkey,
    bids: Pubkey,
    asks: Pubkey,
    base_lots: u64,
    quote_lots: u64,
}

impl DexMarket {
    /// Create a new DexMarket
    pub fn new(dex_market_info: &AccountInfo) -> Self {
        let dex_market_data = dex_market_info.data.borrow();
        let base_mint = Self::pubkey_at_offset(&dex_market_data, BASE_MINT_OFFSET);
        let bids = Self::pubkey_at_offset(&dex_market_data, BIDS_OFFSET);
        let asks = Self::pubkey_at_offset(&dex_market_data, ASKS_OFFSET);
        let base_lots = Self::base_lots(&dex_market_data);
        let quote_lots = Self::quote_lots(&dex_market_data);

        Self {
            base_mint,
            bids,
            asks,
            base_lots,
            quote_lots,
        }
    }

    fn base_lots(data: &[u8]) -> u64 {
        let count_start = 5 + 43 * 8;
        let count_end = count_start + 8;
        u64::from_le_bytes(<[u8; 8]>::try_from(&data[count_start..count_end]).unwrap())
    }

    fn quote_lots(data: &[u8]) -> u64 {
        let count_start = 5 + 44 * 8;
        let count_end = count_start + 8;
        u64::from_le_bytes(<[u8; 8]>::try_from(&data[count_start..count_end]).unwrap())
    }

    /// Get pubkey located at offset
    pub fn pubkey_at_offset(data: &[u8], offset: usize) -> Pubkey {
        let count_start = 5 + offset * 8;
        let count_end = count_start + 32;
        Pubkey::new(&data[count_start..count_end])
    }
}

/// A more efficient `copy_from_slice` implementation.
fn fast_copy(mut src: &[u8], mut dst: &mut [u8]) {
    const COPY_SIZE: usize = 512;
    while src.len() >= COPY_SIZE {
        #[allow(clippy::ptr_offset_with_cast)]
        let (src_word, src_rem) = array_refs![src, COPY_SIZE; ..;];
        #[allow(clippy::ptr_offset_with_cast)]
        let (dst_word, dst_rem) = mut_array_refs![dst, COPY_SIZE; ..;];
        *dst_word = *src_word;
        src = src_rem;
        dst = dst_rem;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(src.as_ptr(), dst.as_mut_ptr(), src.len());
    }
}

/// A stack and instruction efficient memset
fn fast_set(mut dst: &mut [u8], val: u8) {
    const SET_SIZE: usize = 1024;
    while dst.len() >= SET_SIZE {
        #[allow(clippy::ptr_offset_with_cast)]
        let (dst_word, dst_rem) = mut_array_refs![dst, SET_SIZE; ..;];
        *dst_word = [val; SET_SIZE];
        dst = dst_rem;
    }
    unsafe {
        std::ptr::write_bytes(dst.as_mut_ptr(), val, dst.len());
    }
}
