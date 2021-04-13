//! Proactive market making curve implementation

/**
 * Proactive market making curve implementation (interview question for solana-labs)
 * Incomplete!
 * see https://github.com/solana-labs/solana-program-library/blob/master/token-swap/proposals/ProactiveMarketMaking.md
 * and https://dodoex.github.io/docs/docs/pmmDetails/
 */


use {
    crate::{
        curve::calculator::{
            map_zero_to_none, CurveCalculator, DynPack, RoundDirection, SwapWithoutFeesResult,
            TradeDirection, TradingTokenResult,
        },
        error::SwapError,
    },
    solana_program::{
        program_error::ProgramError,
        program_pack::{IsInitialized, Pack, Sealed},
    },
    spl_math::{checked_ceil_div::CheckedCeilDiv, precise_number::PreciseNumber},
};


/// PMMCurve struct implementing CurveCalculator
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PMMCurve {
    /// base token regression target
    pub b_0: u128,
    /// quote token regression target
    pub q_0: u128,
    /// the market price provided by an oracle
    pub i: f64,
    /// a parameter in the range (0, 1)
    pub k: f64,
}


fn p_margin(i: f64, b: u128, b_0: u128, q: u128, q_0: u128, k: f64) -> f64 {
    let mut r = 1.0;
    if b < b_0 {
        r = 1.0 - k + (b_0 as f64/b as f64).powf(2.0) * k;
    } else
    if q < q_0 {
        r = 1.0 / (1.0 - k + (q_0 as f64/q as f64).powf(2.0) * k)
    }
    return i*r;
}


impl CurveCalculator for PMMCurve {
    /// Constant product swap ensures x * y = constant
    fn swap_without_fees(
        &self,
        source_amount: u128,
        swap_source_amount: u128, // b
        swap_destination_amount: u128,  // q
        trade_direction: TradeDirection,
    ) -> Option<SwapWithoutFeesResult> {
        let b = swap_source_amount;
        let q = swap_destination_amount;
        // if bid:quote is ETH:BTC, price is the quantity of ETH equal to one BTC
        let p = p_margin(self.i, b, self.b_0, q, self.q_0, self.k);
        let source_amount_swapped = source_amount;
        let destination_amount_swapped = (source_amount as f64 / p) as u128;
        return Some(SwapWithoutFeesResult {
            source_amount_swapped,
            destination_amount_swapped,
        });
    }

    /// The constant product implementation is a simple ratio calculation for how many
    /// trading tokens correspond to a certain number of pool tokens
    fn pool_tokens_to_trading_tokens(
        &self,
        pool_tokens: u128,
        pool_token_supply: u128,
        swap_token_a_amount: u128,
        swap_token_b_amount: u128,
        round_direction: RoundDirection,
    ) -> Option<TradingTokenResult> {
        return None;
    }

    /// Get the amount of pool tokens for the given amount of token A or B.
    fn trading_tokens_to_pool_tokens(
        &self,
        source_amount: u128,
        swap_token_a_amount: u128,
        swap_token_b_amount: u128,
        pool_supply: u128,
        trade_direction: TradeDirection,
        round_direction: RoundDirection,
    ) -> Option<u128> {
        return None;
    }

    fn normalized_value(
        &self,
        swap_token_a_amount: u128,
        swap_token_b_amount: u128,
    ) -> Option<PreciseNumber> {
        return None;
    }

    fn validate(&self) -> Result<(), SwapError> {
        Ok(())
    }
}

/// IsInitialized is required to use `Pack::pack` and `Pack::unpack`
impl IsInitialized for PMMCurve {
    fn is_initialized(&self) -> bool {
        true
    }
}
impl Sealed for PMMCurve {}
/*impl Pack for PMMCurve {
    const LEN: usize = 0;
    fn pack_into_slice(&self, output: &mut [u8]) {
        (self as &dyn DynPack).pack_into_slice(output);
    }

    fn unpack_from_slice(_input: &[u8]) -> Result<PMMCurve, ProgramError> {
        Ok(Self {})
    }
}// */

impl DynPack for PMMCurve {
    fn pack_into_slice(&self, _output: &mut [u8]) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::calculator::{
        test::{
            check_curve_value_from_swap, check_pool_token_conversion,
            check_pool_value_from_deposit, check_pool_value_from_withdraw, total_and_intermediate,
            CONVERSION_BASIS_POINTS_GUARANTEE,
        },
        RoundDirection, INITIAL_SWAP_POOL_AMOUNT,
    };
    use proptest::prelude::*;


    #[test]
    fn test_constant_price() {
        // if k=0, price should always match the oracle's price
        assert_eq!(p_margin(15.0, 100, 100, 100, 100, 0.0), 15.0);
    }

    #[test]
    fn test_constant_price_swap() {
        // if k=0, price should always match the oracle's price
        let calculator = PMMCurve {b_0:100, q_0:100, i:15.0, k:0.0};
        let x = calculator.swap_without_fees(15, 100, 100, TradeDirection::AtoB).unwrap();
        assert_eq!(x.source_amount_swapped, 15);
        assert_eq!(x.destination_amount_swapped, 1);
    }
    
    
    #[test]
    fn test_constant_price_small_b() {
        // b < b_0
        assert_eq!(p_margin(15.0, 1, 100, 1, 1, 0.0), 15.0);
    }
    
    #[test]
    fn test_constant_price_small_b_swap() {
        // b < b_0
        let calculator = PMMCurve {b_0:100, q_0:1, i:15.0, k:0.0};
        let x = calculator.swap_without_fees(15, 1, 1, TradeDirection::AtoB).unwrap();
        assert_eq!(x.source_amount_swapped, 15);
        assert_eq!(x.destination_amount_swapped, 1);
    }
    
    #[test]
    fn test_constant_price_small_q() {
        // q < q_0
        assert_eq!(p_margin(15.0, 1, 1, 1, 100, 0.0), 15.0);
    }
    
    #[test]
    fn test_constant_price_zeros() {
        // should not divide by zero
        assert_eq!(p_margin(15.0, 0, 0, 0, 0, 0.0), 15.0);
    }
    
    #[test]
    fn test_pmm_equiv_to_amm_base() {
        // k=1, so price should scale linearly according to AMM
        assert_eq!(p_margin(15.0, 1, 1, 1, 1, 1.0), 15.0);
    }
    
    #[test]
    fn test_pmm_equiv_to_amm_small_b() {
        // b < b_0
        assert_eq!(p_margin(15.0, 1, 2, 0, 0, 1.0), 60.0);
    }
    
    #[test]
    fn test_pmm_equiv_to_amm_small_q() {
        // q < q_0
        assert_eq!(p_margin(16.0, 0, 0, 1, 2, 1.0), 4.0);
    }
    
    #[test]
    fn test_nonlinear_small_b() {
        assert_eq!(p_margin(10.0, 2, 4, 0, 0, 0.5), 25.0);
    }
    
    #[test]
    fn test_nonlinear_small_b_swap() {
        let calculator = PMMCurve {b_0:4, q_0:0, i:10.0, k:0.5};
        let x = calculator.swap_without_fees(25, 2, 0, TradeDirection::AtoB).unwrap();
        assert_eq!(x.source_amount_swapped, 25);
        assert_eq!(x.destination_amount_swapped, 1);
    }
    
    #[test]
    fn test_nonlinear_small_b_swap_2() {
        let calculator = PMMCurve {b_0:4, q_0:0, i:10.0, k:0.5};
        let x = calculator.swap_without_fees(50, 2, 0, TradeDirection::AtoB).unwrap();
        assert_eq!(x.source_amount_swapped, 50);
        assert_eq!(x.destination_amount_swapped, 2);
    }
    
    #[test]
    fn test_nonlinear_small_q() {
        assert_eq!(p_margin(10.0, 0, 0, 2, 4, 0.5), 4.0);
    }

    #[test]
    fn test_nonlinear_small_q_swap() {
        let calculator = PMMCurve {b_0:0, q_0:4, i:10.0, k:0.5};
        let x = calculator.swap_without_fees(16, 0, 2, TradeDirection::AtoB).unwrap();
        assert_eq!(x.source_amount_swapped, 16);
        assert_eq!(x.destination_amount_swapped, 4);
    }

}
