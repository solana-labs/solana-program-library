//! Defines useful math utils

#![allow(clippy::assign_op_pattern)]
#![allow(clippy::ptr_offset_with_cast)]
#![allow(clippy::unknown_clippy_lints)]
#![allow(clippy::manual_range_contains)]

use uint::construct_uint;

construct_uint! {
    pub struct U256(4);
}

impl U256 {
    /// Returns selt to the power of b
    pub fn checked_u8_power(&self, b: u8) -> Option<U256> {
        let mut result = *self;
        for _ in 1..b {
            result = result.checked_mul(*self)?;
        }
        Some(result)
    }

    /// Returns self multiplied by b
    pub fn checked_u8_mul(&self, b: u8) -> Option<U256> {
        let mut result = *self;
        for _ in 1..b {
            result = result.checked_add(*self)?;
        }
        Some(result)
    }

    /// Returns true of values differ not more than by 1
    pub fn almost_equal(&self, b: &U256) -> Option<bool> {
        if self > b {
            Some(self.checked_sub(*b)? <= U256::one())
        } else {
            Some(b.checked_sub(*self)? <= U256::one())
        }
    }
}
