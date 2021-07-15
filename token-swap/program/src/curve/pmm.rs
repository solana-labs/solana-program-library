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
    /// quote token regression target
    pub q_0: u128,
    /// the market price provided by an oracle
    pub i: f64,
    /// a parameter in the range (0, 1)
    pub k: f64,
}


fn calc_q_2(delta_b: i128, q_0: u128, q_1: u128, i: f64, k: f64) -> u128 {
    // does not handle k=1.0 (divide by zero)
    // what should the behavior be?
    if delta_b == 0 { return q_1 }
    let a = 1.0 - k;
    let q_2_sq = f64::powf(q_0 as f64, 2.0);
    let b = k * q_2_sq / q_1 as f64 - q_1 as f64 + k * q_1 as f64 - i * delta_b  as f64;
    let c = -k * q_2_sq;
    let q_2 = (-b + f64::sqrt(f64::powf(b,2.0) - 4.0*a*c)) / (2.0*a);
    return q_2 as u128;
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
        let q_1 = swap_destination_amount;
        let delta_b = source_amount as i128 - swap_source_amount as i128; // not sure about this.  other way around?
        let q_2 = calc_q_2(delta_b, self.q_0, q_1, self.i, self.k);
        let receive_q = if delta_b < 0 {q_1 - q_2} else {q_2 - q_1};
        let source_amount_swapped = delta_b.abs() as u128;
        let destination_amount_swapped = receive_q;
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


    // calc_q_2(delta_b: u128, q_0: u128, q_1: u128, k: f64) -> u128


    #[test]
    fn test_delta_b_is_zero() {
        let delta_b = 0;
        assert_eq!(calc_q_2(delta_b, 100, 200, 100.0, 0.5), 200);
        assert_eq!(calc_q_2(delta_b, 100, 200, 100.0, 1.0), 200);
    }


    #[test]
    fn test_k_is_0() {
        // When k = 0, the price is the determining factor
        let q_0 = 100;
        let q_1 = 200;
        let delta_b = -20;
        let mut i = 1.0;
        let k = 0.0;
        assert_eq!(q_1 - calc_q_2(delta_b, q_0, q_1, i, k), 20);
        assert_eq!(q_1 - calc_q_2(delta_b, 0, q_1, i, k), 20);
        assert_eq!(q_1-100 - calc_q_2(delta_b, q_0, q_1-100, i, k), 20); // q_1 == q_0
        assert_eq!(q_1-101 - calc_q_2(delta_b, q_0, q_1-101, i, k), 20); // q_1 < q_0, but not sure if this will ever happen?
        assert_eq!(q_1 - calc_q_2(delta_b-10, q_0, q_1, i, k), 30);
        i = 2.0;
        assert_eq!(q_1 - calc_q_2(delta_b, q_0, q_1, i, k), 40);
        assert_eq!(q_1 - calc_q_2(delta_b-1, q_0, q_1, i, k), 42);
        i = 0.5;
        assert_eq!(q_1 - calc_q_2(delta_b, q_0, q_1, i, k), 10);
        assert_eq!(q_1 - calc_q_2(delta_b+2, q_0, q_1, i, k), 9);
    }


    #[test]
    fn test_k_sensitivity() {
        // as k increases, you get more of your quote token for your base token.
        // i think this is correct per the trading formula, as q_2 is calculated 
        // w/ 2*(1-k) in the denominator, but it seems counter intuitive to me.
        let q_0 = 100;
        let q_1 = 200;
        let delta_b = -20;
        let i = 1.0;
        assert_eq!(q_1 - calc_q_2(delta_b, q_0, q_1, i, 0.0), 20);
        assert_eq!(q_1 - calc_q_2(delta_b, q_0, q_1, i, 0.1), 22);
        assert_eq!(q_1 - calc_q_2(delta_b, q_0, q_1, i, 0.5), 31);
        assert_eq!(q_1 - calc_q_2(delta_b, q_0, q_1, i, 0.9), 50);
        // what should it do when k=1, where q_2 is undefined?
        // assert_eq!(q_1 - calc_q_2(delta_b, q_0, q_1, i, 1.0), ???);
    }
    
    
    #[test]
    fn test_swap_without_fees() {
        let calculator = PMMCurve {q_0:100, i:1.0, k:0.5};
        let x = calculator.swap_without_fees(100, 120, 200, TradeDirection::AtoB).unwrap();
        assert_eq!(x.source_amount_swapped, 20);
        assert_eq!(x.destination_amount_swapped, 31);
    }
    

}
