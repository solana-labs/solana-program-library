use crate::state::{pack_decimal, unpack_decimal};
use solana_program::msg;
use solana_program::program_pack::IsInitialized;
use solana_program::{program_error::ProgramError, slot_history::Slot};

use crate::{
    error::LendingError,
    math::{Decimal, TryAdd, TryDiv, TryMul, TrySub},
};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::program_pack::{Pack, Sealed};

/// Sliding Window Rate limiter
/// guarantee: at any point, the outflow between [cur_slot - slot.window_duration, cur_slot]
/// is less than 2x max_outflow.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimiter {
    /// configuration parameters
    pub config: RateLimiterConfig,

    // state
    /// prev qty is the sum of all outflows from [window_start - config.window_duration, window_start)
    prev_qty: Decimal,
    /// window_start is the start of the current window
    window_start: Slot,
    /// cur qty is the sum of all outflows from [window_start, window_start + config.window_duration)
    cur_qty: Decimal,
}

/// Lending market configuration parameters
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RateLimiterConfig {
    /// Rate limiter window size in slots
    pub window_duration: u64,
    /// Rate limiter param. Max outflow of tokens in a window
    pub max_outflow: u64,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            window_duration: 1,
            max_outflow: u64::MAX,
        }
    }
}

impl RateLimiter {
    /// initialize rate limiter
    pub fn new(config: RateLimiterConfig, cur_slot: u64) -> Self {
        let slot_start = cur_slot / config.window_duration * config.window_duration;
        Self {
            config,
            prev_qty: Decimal::zero(),
            window_start: slot_start,
            cur_qty: Decimal::zero(),
        }
    }

    /// update rate limiter with new quantity. errors if rate limit has been reached
    pub fn update(&mut self, cur_slot: u64, qty: Decimal) -> Result<(), ProgramError> {
        if cur_slot < self.window_start {
            msg!("Current slot is less than window start, which is impossible");
            return Err(LendingError::InvalidAccountInput.into());
        }

        // rate limiter is disabled if window duration == 0. this is here because we don't want to
        // brick borrows/withdraws in permissionless pools on program upgrade.
        if self.config.window_duration == 0 {
            return Ok(());
        }

        // floor wrt window duration
        let cur_slot_start = cur_slot / self.config.window_duration * self.config.window_duration;

        // update prev window, current window
        match cur_slot_start.cmp(&(self.window_start + self.config.window_duration)) {
            // |<-prev window->|<-cur window (cur_slot is in here)->|
            std::cmp::Ordering::Less => (),

            // |<-prev window->|<-cur window->| (cur_slot is in here) |
            std::cmp::Ordering::Equal => {
                self.prev_qty = self.cur_qty;
                self.window_start = cur_slot_start;
                self.cur_qty = Decimal::zero();
            }

            // |<-prev window->|<-cur window->|<-cur window + 1->| ... | (cur_slot is in here) |
            std::cmp::Ordering::Greater => {
                self.prev_qty = Decimal::zero();
                self.window_start = cur_slot_start;
                self.cur_qty = Decimal::zero();
            }
        };

        // assume the prev_window's outflow is even distributed across the window
        // this isn't true, but it's a good enough approximation
        let prev_weight = Decimal::from(self.config.window_duration)
            .try_sub(Decimal::from(cur_slot - self.window_start + 1))?
            .try_div(self.config.window_duration)?;
        let cur_outflow = prev_weight.try_mul(self.prev_qty)?.try_add(self.cur_qty)?;

        if cur_outflow.try_add(qty)? > Decimal::from(self.config.max_outflow) {
            Err(LendingError::OutflowRateLimitExceeded.into())
        } else {
            self.cur_qty = self.cur_qty.try_add(qty)?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_rate_limiter() {
        let mut rate_limiter = RateLimiter::new(
            RateLimiterConfig {
                window_duration: 10,
                max_outflow: 100,
            },
            10,
        );

        assert_eq!(
            rate_limiter.update(9, Decimal::from(1u64)),
            Err(LendingError::InvalidAccountInput.into())
        );

        // case 1: no prev window, all quantity is taken up in first slot
        assert_eq!(
            rate_limiter.update(10, Decimal::from(101u64)),
            Err(LendingError::OutflowRateLimitExceeded.into())
        );
        assert_eq!(rate_limiter.update(10, Decimal::from(100u64)), Ok(()));
        for i in 11..20 {
            assert_eq!(
                rate_limiter.update(i, Decimal::from(1u64)),
                Err(LendingError::OutflowRateLimitExceeded.into())
            );
        }

        // case 2: prev window qty affects cur window's allowed qty. exactly 10 qty frees up every
        // slot.
        for i in 20..30 {
            assert_eq!(
                rate_limiter.update(i, Decimal::from(11u64)),
                Err(LendingError::OutflowRateLimitExceeded.into())
            );

            assert_eq!(rate_limiter.update(i, Decimal::from(10u64)), Ok(()));

            assert_eq!(
                rate_limiter.update(i, Decimal::from(1u64)),
                Err(LendingError::OutflowRateLimitExceeded.into())
            );
        }

        // case 3: new slot is so far ahead, prev window is dropped
        assert_eq!(rate_limiter.update(100, Decimal::from(10u64)), Ok(()));
        for i in 101..109 {
            assert_eq!(rate_limiter.update(i, Decimal::from(10u64)), Ok(()));
        }
        println!("{:#?}", rate_limiter);
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(
            RateLimiterConfig {
                window_duration: 1,
                max_outflow: u64::MAX,
            },
            1,
        )
    }
}

impl Sealed for RateLimiter {}

impl IsInitialized for RateLimiter {
    fn is_initialized(&self) -> bool {
        true
    }
}

/// Size of RateLimiter when packed into account
pub const RATE_LIMITER_LEN: usize = 56;
impl Pack for RateLimiter {
    const LEN: usize = RATE_LIMITER_LEN;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, RATE_LIMITER_LEN];
        let (
            config_max_outflow_dst,
            config_window_duration_dst,
            prev_qty_dst,
            window_start_dst,
            cur_qty_dst,
        ) = mut_array_refs![dst, 8, 8, 16, 8, 16];
        *config_max_outflow_dst = self.config.max_outflow.to_le_bytes();
        *config_window_duration_dst = self.config.window_duration.to_le_bytes();
        pack_decimal(self.prev_qty, prev_qty_dst);
        *window_start_dst = self.window_start.to_le_bytes();
        pack_decimal(self.cur_qty, cur_qty_dst);
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, RATE_LIMITER_LEN];
        let (
            config_max_outflow_src,
            config_window_duration_src,
            prev_qty_src,
            window_start_src,
            cur_qty_src,
        ) = array_refs![src, 8, 8, 16, 8, 16];

        Ok(Self {
            config: RateLimiterConfig {
                max_outflow: u64::from_le_bytes(*config_max_outflow_src),
                window_duration: u64::from_le_bytes(*config_window_duration_src),
            },
            prev_qty: unpack_decimal(prev_qty_src),
            window_start: u64::from_le_bytes(*window_start_src),
            cur_qty: unpack_decimal(cur_qty_src),
        })
    }
}
