//! The curve.fi invariant calculator.

use crate::{curve::math::U256, error::SwapError};
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};

use crate::curve::calculator::{CurveCalculator, DynPack, SwapWithoutFeesResult, TradeDirection};
use arrayref::{array_mut_ref, array_ref};
use std::convert::TryFrom;

const N_COINS: u8 = 2;
const N_COINS_SQUARED: u8 = 4;

/// StableCurve struct implementing CurveCalculator
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StableCurve {
    /// Amplifier constant
    pub amp: u64,
}

/// d = (leverage * sum_x + d_product * n_coins) * initial_d / ((leverage - 1) * initial_d + (n_coins + 1) * d_product)
fn calculate_step(initial_d: &U256, leverage: u64, sum_x: u128, d_product: &U256) -> Option<U256> {
    let leverage_mul = U256::from(leverage).checked_mul(sum_x.into())?;
    let d_p_mul = d_product.checked_u8_mul(N_COINS)?;

    let l_val = leverage_mul.checked_add(d_p_mul)?.checked_mul(*initial_d)?;

    let leverage_sub = initial_d.checked_mul((leverage.checked_sub(1)?).into())?;
    let n_coins_sum = d_product.checked_u8_mul(N_COINS.checked_add(1)?)?;

    let r_val = leverage_sub.checked_add(n_coins_sum)?;

    l_val.checked_div(r_val)
}

/// Compute stable swap invariant (D)
/// Equation:
/// A * sum(x_i) * n**n + D = A * D * n**n + D**(n+1) / (n**n * prod(x_i))
fn compute_d(leverage: u64, amount_a: u128, amount_b: u128) -> Option<u128> {
    let amount_a_times_coins = U256::from(amount_a).checked_u8_mul(N_COINS)?;
    let amount_b_times_coins = U256::from(amount_b).checked_u8_mul(N_COINS)?;
    let sum_x = amount_a.checked_add(amount_b)?; // sum(x_i), a.k.a S
    if sum_x == 0 {
        Some(0)
    } else {
        let mut d_previous: U256;
        let mut d: U256 = sum_x.into();

        // Newton's method to approximate D
        for _ in 0..32 {
            let mut d_product = d;
            d_product = d_product
                .checked_mul(d)?
                .checked_div(amount_a_times_coins)?;
            d_product = d_product
                .checked_mul(d)?
                .checked_div(amount_b_times_coins)?;
            d_previous = d;
            //d = (leverage * sum_x + d_p * n_coins) * d / ((leverage - 1) * d + (n_coins + 1) * d_p);
            d = calculate_step(&d, leverage, sum_x, &d_product)?;
            // Equality with the precision of 1
            if d.almost_equal(&d_previous)? {
                break;
            }
        }
        u128::try_from(d).ok()
    }
}

/// Compute swap amount `y` in proportion to `x`
/// Solve for y:
/// y**2 + y * (sum' - (A*n**n - 1) * D / (A * n**n)) = D ** (n + 1) / (n ** (2 * n) * prod' * A)
/// y**2 + b*y = c
fn compute_new_destination_amount(
    leverage: u64,
    new_source_amount: u128,
    d_val: u128,
) -> Option<u128> {
    // Upscale to U256
    let leverage: U256 = leverage.into();
    let new_source_amount: U256 = new_source_amount.into();
    let d_val: U256 = d_val.into();

    // sum' = prod' = x
    // c =  D ** (n + 1) / (n ** (2 * n) * prod' * A)
    let c = d_val
        .checked_u8_power(N_COINS.checked_add(1)?)?
        .checked_div(
            new_source_amount
                .checked_u8_mul(N_COINS_SQUARED)?
                .checked_mul(leverage)?,
        )?;

    // b = sum' - (A*n**n - 1) * D / (A * n**n)
    let b = new_source_amount.checked_add(d_val.checked_div(leverage)?)?;

    // Solve for y by approximating: y**2 + b*y = c
    let mut y_prev: U256;
    let mut y = d_val;
    for _ in 0..32 {
        y_prev = y;
        y = (y.checked_u8_power(2)?.checked_add(c)?)
            .checked_div(y.checked_u8_mul(2)?.checked_add(b)?.checked_sub(d_val)?)?;
        if y.almost_equal(&y_prev)? {
            break;
        }
    }
    u128::try_from(y).ok()
}

impl CurveCalculator for StableCurve {
    /// Stable curve
    fn swap_without_fees(
        &self,
        source_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
        _trade_direction: TradeDirection,
    ) -> Option<SwapWithoutFeesResult> {
        let leverage = self.amp.checked_mul(N_COINS as u64)?;

        let new_source_amount = swap_source_amount.checked_add(source_amount)?;
        let new_destination_amount = compute_new_destination_amount(
            leverage,
            new_source_amount,
            compute_d(leverage, swap_source_amount, swap_destination_amount)?,
        )?;

        let amount_swapped = swap_destination_amount.checked_sub(new_destination_amount)?;

        Some(SwapWithoutFeesResult {
            source_amount_swapped: source_amount,
            destination_amount_swapped: amount_swapped,
        })
    }

    fn validate(&self) -> Result<(), SwapError> {
        // TODO are all amps valid?
        Ok(())
    }
}

/// IsInitialized is required to use `Pack::pack` and `Pack::unpack`
impl IsInitialized for StableCurve {
    fn is_initialized(&self) -> bool {
        true
    }
}
impl Sealed for StableCurve {}
impl Pack for StableCurve {
    const LEN: usize = 8;
    fn pack_into_slice(&self, output: &mut [u8]) {
        (self as &dyn DynPack).pack_into_slice(output);
    }

    fn unpack_from_slice(input: &[u8]) -> Result<StableCurve, ProgramError> {
        let amp = array_ref![input, 0, 8];
        Ok(Self {
            amp: u64::from_le_bytes(*amp),
        })
    }
}

impl DynPack for StableCurve {
    fn pack_into_slice(&self, output: &mut [u8]) {
        let amp = array_mut_ref![output, 0, 8];
        *amp = self.amp.to_le_bytes();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::calculator::INITIAL_SWAP_POOL_AMOUNT;
    use sim::StableSwapModel;

    #[test]
    fn initial_pool_amount() {
        let amp = 1;
        let calculator = StableCurve { amp };
        assert_eq!(calculator.new_pool_supply(), INITIAL_SWAP_POOL_AMOUNT);
    }

    fn check_pool_token_rate(token_a: u128, deposit: u128, supply: u128, expected_a: u128) {
        let amp = 1;
        let calculator = StableCurve { amp };
        let results = calculator
            .pool_tokens_to_trading_tokens(deposit, supply, token_a)
            .unwrap();
        assert_eq!(results, expected_a);
    }

    #[test]
    fn trading_token_conversion() {
        check_pool_token_rate(2, 5, 10, 1);
        check_pool_token_rate(10, 5, 10, 5);
        check_pool_token_rate(5, 5, 10, 2);
        check_pool_token_rate(5, 5, 10, 2);
    }

    #[test]
    fn fail_trading_token_conversion() {
        let amp = 1;
        let calculator = StableCurve { amp };
        let results = calculator.pool_tokens_to_trading_tokens(5, 10, u128::MAX);
        assert!(results.is_none());
    }

    #[test]
    fn constant_product_swap_no_fee() {
        const POOL_AMOUNTS: &[u128] = &[
            100,
            10_000,
            1_000_000,
            100_000_000,
            10_000_000_000,
            1_000_000_000_000,
            100_000_000_000_000,
            10_000_000_000_000_000,
            1_000_000_000_000_000_000,
        ];
        const SWAP_AMOUNTS: &[u128] = &[
            100,
            1_000,
            10_000,
            100_000,
            1_000_000,
            10_000_000,
            100_000_000,
            1_000_000_000,
            10_000_000_000,
            100_000_000_000,
        ];
        const AMP_FACTORS: &[u64] = &[1, 10, 20, 50, 75, 100, 125, 150];

        for swap_source_amount in POOL_AMOUNTS {
            for swap_destination_amount in POOL_AMOUNTS {
                for source_amount in SWAP_AMOUNTS {
                    for amp in AMP_FACTORS {
                        let curve = StableCurve { amp: *amp };

                        if *source_amount >= *swap_source_amount {
                            continue;
                        }

                        println!(
                            "trying: source_amount={}, swap_source_amount={}, swap_destination_amount={}",
                            source_amount, swap_source_amount, swap_destination_amount
                        );

                        let model: StableSwapModel = StableSwapModel::new(
                            curve.amp.into(),
                            vec![*swap_source_amount, *swap_destination_amount],
                            N_COINS,
                        );

                        let result = curve.swap_without_fees(
                            *source_amount,
                            *swap_source_amount,
                            *swap_destination_amount,
                            TradeDirection::AtoB,
                        );

                        let result = result.unwrap();
                        let sim_result = model.sim_exchange(0, 1, *source_amount);

                        println!(
                            "result={}, sim_result={}",
                            result.destination_amount_swapped, sim_result
                        );
                        let diff =
                            (sim_result as i128 - result.destination_amount_swapped as i128).abs();

                        assert!(
                            diff <= 1,
                            "result={}, sim_result={}, amp={}, source_amount={}, swap_source_amount={}, swap_destination_amount={}",
                            result.destination_amount_swapped,
                            sim_result,
                            amp,
                            source_amount,
                            swap_source_amount,
                            swap_destination_amount
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn pack_curve() {
        let amp = 1;
        let curve = StableCurve { amp };

        let mut packed = [0u8; StableCurve::LEN];
        Pack::pack_into_slice(&curve, &mut packed[..]);
        let unpacked = StableCurve::unpack(&packed).unwrap();
        assert_eq!(curve, unpacked);

        let mut packed = vec![];
        packed.extend_from_slice(&amp.to_le_bytes());
        let unpacked = StableCurve::unpack(&packed).unwrap();
        assert_eq!(curve, unpacked);
    }
}
