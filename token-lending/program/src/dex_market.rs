//! Dex market used for simulating trades

use crate::{
    error::LendingError,
    math::{Decimal, TryAdd, TryDiv, TryMul, TrySub},
    state::TokenConverter,
};
use arrayref::{array_refs, mut_array_refs};
use serum_dex::critbit::{Slab, SlabView};
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};
use std::{cell::RefMut, convert::TryFrom};

/// Side of the dex market order book
#[derive(Clone, Copy, PartialEq)]
enum Side {
    Bid,
    Ask,
}

/// Market currency
#[derive(Clone, Copy, PartialEq)]
enum Currency {
    Base,
    Quote,
}

impl Currency {
    fn opposite(&self) -> Self {
        match self {
            Currency::Base => Currency::Quote,
            Currency::Quote => Currency::Base,
        }
    }
}

/// Trade action for trade simulator
#[derive(PartialEq)]
pub enum TradeAction {
    /// Sell tokens
    Sell,
    /// Buy tokens
    Buy,
}

/// Dex market order
struct Order {
    price: u64,
    quantity: u64,
}

/// Trade simulator
pub struct TradeSimulator<'a> {
    dex_market: DexMarket,
    orders: DexMarketOrders<'a>,
    orders_side: Side,
    quote_token_mint: &'a Pubkey,
    buy_token_mint: &'a Pubkey,
    sell_token_mint: &'a Pubkey,
}

impl<'a> TokenConverter for TradeSimulator<'a> {
    fn best_price(&mut self, token_mint: &Pubkey) -> Result<Decimal, ProgramError> {
        let action = if token_mint == self.buy_token_mint {
            TradeAction::Buy
        } else {
            TradeAction::Sell
        };

        let currency = if token_mint == self.quote_token_mint {
            Currency::Quote
        } else {
            Currency::Base
        };

        let order_book_side = match (action, currency) {
            (TradeAction::Buy, Currency::Base) => Side::Ask,
            (TradeAction::Sell, Currency::Quote) => Side::Ask,
            (TradeAction::Buy, Currency::Quote) => Side::Bid,
            (TradeAction::Sell, Currency::Base) => Side::Bid,
        };
        if order_book_side != self.orders_side {
            return Err(LendingError::DexInvalidOrderBookSide.into());
        }

        let best_order_price = self
            .orders
            .best_order_price()
            .ok_or(LendingError::TradeSimulationError)?;

        let input_token = Decimal::one().try_div(self.dex_market.get_lots(currency))?;
        let output_token_price = if currency == Currency::Base {
            input_token.try_mul(best_order_price)
        } else {
            input_token.try_div(best_order_price)
        }?;
        Ok(output_token_price.try_mul(self.dex_market.get_lots(currency.opposite()))?)
    }

    fn convert(
        self,
        from_amount: Decimal,
        from_token_mint: &Pubkey,
    ) -> Result<Decimal, ProgramError> {
        let action = if from_token_mint == self.buy_token_mint {
            TradeAction::Buy
        } else {
            TradeAction::Sell
        };

        self.simulate_trade(action, from_amount)
    }
}

impl<'a> TradeSimulator<'a> {
    /// Create a new TradeSimulator
    pub fn new(
        dex_market_info: &AccountInfo,
        dex_market_orders: &AccountInfo,
        memory: &'a AccountInfo,
        quote_token_mint: &'a Pubkey,
        buy_token_mint: &'a Pubkey,
        sell_token_mint: &'a Pubkey,
    ) -> Result<Self, ProgramError> {
        let dex_market = DexMarket::new(dex_market_info);
        let orders = DexMarketOrders::new(&dex_market, dex_market_orders, memory)?;
        let orders_side = orders.side;

        Ok(Self {
            dex_market,
            orders,
            orders_side,
            quote_token_mint,
            buy_token_mint,
            sell_token_mint,
        })
    }

    /// Simulate a trade
    pub fn simulate_trade(
        mut self,
        action: TradeAction,
        quantity: Decimal,
    ) -> Result<Decimal, ProgramError> {
        let token_mint = match action {
            TradeAction::Buy => self.buy_token_mint,
            TradeAction::Sell => self.sell_token_mint,
        };

        let currency = if token_mint == self.quote_token_mint {
            Currency::Quote
        } else {
            Currency::Base
        };

        let order_book_side = match (action, currency) {
            (TradeAction::Buy, Currency::Base) => Side::Ask,
            (TradeAction::Sell, Currency::Quote) => Side::Ask,
            (TradeAction::Buy, Currency::Quote) => Side::Bid,
            (TradeAction::Sell, Currency::Base) => Side::Bid,
        };

        if order_book_side != self.orders_side {
            return Err(LendingError::DexInvalidOrderBookSide.into());
        }

        let input_quantity: Decimal = quantity.try_div(self.dex_market.get_lots(currency))?;
        let output_quantity = self.exchange_with_order_book(input_quantity, currency)?;
        Ok(output_quantity.try_mul(self.dex_market.get_lots(currency.opposite()))?)
    }

    /// Exchange tokens by filling orders
    fn exchange_with_order_book(
        &mut self,
        mut input_quantity: Decimal,
        currency: Currency,
    ) -> Result<Decimal, ProgramError> {
        let mut output_quantity = Decimal::zero();

        let zero = Decimal::zero();
        while input_quantity > zero {
            let next_order = self
                .orders
                .next()
                .ok_or_else(|| ProgramError::from(LendingError::TradeSimulationError))?;

            let next_order_price = next_order.price;
            let base_quantity = next_order.quantity;

            let (filled, output) = if currency == Currency::Base {
                let filled = input_quantity.min(Decimal::from(base_quantity));
                (filled, filled.try_mul(next_order_price)?)
            } else {
                let quote_quantity = Decimal::from(base_quantity).try_mul(next_order_price)?;
                let filled = input_quantity.min(quote_quantity);
                (filled, filled.try_div(next_order_price)?)
            };

            input_quantity = input_quantity.try_sub(filled)?;
            output_quantity = output_quantity.try_add(output)?;
        }

        Ok(output_quantity)
    }
}

/// Dex market order account info
struct DexMarketOrders<'a> {
    heap: Option<RefMut<'a, Slab>>,
    side: Side,
}

impl<'a> DexMarketOrders<'a> {
    /// Create a new DexMarketOrders
    fn new(
        dex_market: &DexMarket,
        orders: &AccountInfo,
        memory: &'a AccountInfo,
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

        Ok(Self { heap, side })
    }

    fn best_order_price(&mut self) -> Option<u64> {
        let side = self.side;
        self.heap.as_mut().and_then(|heap| {
            let handle = match side {
                Side::Bid => heap.find_max(),
                Side::Ask => heap.find_min(),
            }?;

            Some(heap.get_mut(handle)?.as_leaf_mut()?.price().get())
        })
    }
}

impl Iterator for DexMarketOrders<'_> {
    type Item = Order;

    fn next(&mut self) -> Option<Order> {
        let leaf_node = match self.side {
            Side::Bid => self.heap.as_mut().and_then(|heap| heap.remove_max()),
            Side::Ask => self.heap.as_mut().and_then(|heap| heap.remove_min()),
        }?;

        Some(Order {
            price: leaf_node.price().get(),
            quantity: leaf_node.quantity(),
        })
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
    bids: Pubkey,
    asks: Pubkey,
    base_lots: u64,
    quote_lots: u64,
}

impl DexMarket {
    /// Create a new DexMarket
    fn new(dex_market_info: &AccountInfo) -> Self {
        let dex_market_data = dex_market_info.data.borrow();
        let bids = Self::pubkey_at_offset(&dex_market_data, BIDS_OFFSET);
        let asks = Self::pubkey_at_offset(&dex_market_data, ASKS_OFFSET);
        let base_lots = Self::base_lots(&dex_market_data);
        let quote_lots = Self::quote_lots(&dex_market_data);

        Self {
            bids,
            asks,
            base_lots,
            quote_lots,
        }
    }

    fn get_lots(&self, currency: Currency) -> u64 {
        match currency {
            Currency::Base => self.base_lots,
            Currency::Quote => self.quote_lots,
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
