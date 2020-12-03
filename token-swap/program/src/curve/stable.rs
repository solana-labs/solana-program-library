//! The curve.fi invariant calculator.

use crate::curve::math::U256;
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};

use crate::curve::calculator::{calculate_fee, CurveCalculator, DynPack, SwapWithoutFeesResult};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use std::convert::TryFrom;

const N_COINS: u8 = 2;
const N_COINS_SQUARED: u8 = 4;

/// StableCurve struct implementing CurveCalculator
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StableCurve {
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

    /// Calculate the owner trading fee in trading tokens
    fn owner_trading_fee(&self, trading_tokens: u128) -> Option<u128> {
        calculate_fee(
            trading_tokens,
            u128::try_from(self.owner_trade_fee_numerator).ok()?,
            u128::try_from(self.owner_trade_fee_denominator).ok()?,
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
}

/// IsInitialized is required to use `Pack::pack` and `Pack::unpack`
impl IsInitialized for StableCurve {
    fn is_initialized(&self) -> bool {
        true
    }
}
impl Sealed for StableCurve {}
impl Pack for StableCurve {
    const LEN: usize = 72;
    fn pack_into_slice(&self, output: &mut [u8]) {
        (self as &dyn DynPack).pack_into_slice(output);
    }

    fn unpack_from_slice(input: &[u8]) -> Result<StableCurve, ProgramError> {
        let input = array_ref![input, 0, 72];
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
            amp,
        ) = array_refs![input, 8, 8, 8, 8, 8, 8, 8, 8, 8];
        Ok(Self {
            trade_fee_numerator: u64::from_le_bytes(*trade_fee_numerator),
            trade_fee_denominator: u64::from_le_bytes(*trade_fee_denominator),
            owner_trade_fee_numerator: u64::from_le_bytes(*owner_trade_fee_numerator),
            owner_trade_fee_denominator: u64::from_le_bytes(*owner_trade_fee_denominator),
            owner_withdraw_fee_numerator: u64::from_le_bytes(*owner_withdraw_fee_numerator),
            owner_withdraw_fee_denominator: u64::from_le_bytes(*owner_withdraw_fee_denominator),
            host_fee_numerator: u64::from_le_bytes(*host_fee_numerator),
            host_fee_denominator: u64::from_le_bytes(*host_fee_denominator),
            amp: u64::from_le_bytes(*amp),
        })
    }
}

impl DynPack for StableCurve {
    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, 72];
        let (
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
            amp,
        ) = mut_array_refs![output, 8, 8, 8, 8, 8, 8, 8, 8, 8];
        *trade_fee_numerator = self.trade_fee_numerator.to_le_bytes();
        *trade_fee_denominator = self.trade_fee_denominator.to_le_bytes();
        *owner_trade_fee_numerator = self.owner_trade_fee_numerator.to_le_bytes();
        *owner_trade_fee_denominator = self.owner_trade_fee_denominator.to_le_bytes();
        *owner_withdraw_fee_numerator = self.owner_withdraw_fee_numerator.to_le_bytes();
        *owner_withdraw_fee_denominator = self.owner_withdraw_fee_denominator.to_le_bytes();
        *host_fee_numerator = self.host_fee_numerator.to_le_bytes();
        *host_fee_denominator = self.host_fee_denominator.to_le_bytes();
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
        let trade_fee_numerator = 0;
        let trade_fee_denominator = 1;
        let owner_trade_fee_numerator = 0;
        let owner_trade_fee_denominator = 1;
        let owner_withdraw_fee_numerator = 0;
        let owner_withdraw_fee_denominator = 1;
        let host_fee_numerator = 0;
        let host_fee_denominator = 1;
        let amp = 1;
        let calculator = StableCurve {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
            amp,
        };
        assert_eq!(calculator.new_pool_supply(), INITIAL_SWAP_POOL_AMOUNT);
    }

    fn check_pool_token_rate(token_a: u128, deposit: u128, supply: u128, expected: Option<u128>) {
        let trade_fee_numerator = 0;
        let trade_fee_denominator = 1;
        let owner_trade_fee_numerator = 0;
        let owner_trade_fee_denominator = 1;
        let owner_withdraw_fee_numerator = 0;
        let owner_withdraw_fee_denominator = 1;
        let host_fee_numerator = 0;
        let host_fee_denominator = 1;
        let amp = 1;
        let calculator = StableCurve {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
            amp,
        };
        assert_eq!(
            calculator.pool_tokens_to_trading_tokens(deposit, supply, token_a),
            expected
        );
    }

    #[test]
    fn trading_token_conversion() {
        check_pool_token_rate(2, 5, 10, Some(1));
        check_pool_token_rate(10, 5, 10, Some(5));
        check_pool_token_rate(5, 5, 10, Some(2));
        check_pool_token_rate(5, 5, 10, Some(2));
        check_pool_token_rate(u128::MAX, 5, 10, None);
    }

    #[test]
    fn stable_swap_calculation_trade_fee() {
        let swap_source_amount = 1_000;
        let swap_destination_amount = 50_000;
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 1_000;
        let owner_trade_fee_numerator = 0;
        let owner_trade_fee_denominator = 0;
        let owner_withdraw_fee_numerator = 0;
        let owner_withdraw_fee_denominator = 0;
        let host_fee_numerator = 0;
        let host_fee_denominator = 0;
        let source_amount = 100;
        let amp = 1;
        let curve = StableCurve {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
            amp,
        };
        let result = curve
            .swap(source_amount, swap_source_amount, swap_destination_amount)
            .unwrap();
        assert_eq!(result.new_source_amount, 1_100);
        assert_eq!(result.destination_amount_swapped, 2_063);
        assert_eq!(result.new_destination_amount, 47_937);
        assert_eq!(result.trade_fee, 1);
        assert_eq!(result.owner_fee, 0);
    }

    #[test]
    fn stable_swap_calculation_owner_fee() {
        let swap_source_amount = 1000;
        let swap_destination_amount = 50000;
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 100;
        let owner_trade_fee_numerator = 2;
        let owner_trade_fee_denominator = 100;
        let owner_withdraw_fee_numerator = 2;
        let owner_withdraw_fee_denominator = 100;
        let host_fee_numerator = 2;
        let host_fee_denominator = 100;
        let source_amount: u128 = 100;
        let amp = 1;
        let curve = StableCurve {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
            amp,
        };
        let result = curve
            .swap(source_amount, swap_source_amount, swap_destination_amount)
            .unwrap();
        assert_eq!(result.new_source_amount, 1100);
        assert_eq!(result.destination_amount_swapped, 2024);
        assert_eq!(result.new_destination_amount, 47976);
        assert_eq!(result.trade_fee, 1);
        assert_eq!(result.owner_fee, 2);
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
                        let curve = StableCurve {
                            amp: *amp,
                            ..Default::default()
                        };

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

                        let result = curve.swap(
                            *source_amount,
                            *swap_source_amount,
                            *swap_destination_amount,
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
    fn pack_constant_product_curve() {
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 4;
        let owner_trade_fee_numerator = 2;
        let owner_trade_fee_denominator = 5;
        let owner_withdraw_fee_numerator = 4;
        let owner_withdraw_fee_denominator = 10;
        let host_fee_numerator = 4;
        let host_fee_denominator = 10;
        let amp = 1;
        let curve = StableCurve {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
            amp,
        };

        let mut packed = [0u8; StableCurve::LEN];
        Pack::pack_into_slice(&curve, &mut packed[..]);
        let unpacked = StableCurve::unpack(&packed).unwrap();
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
        packed.extend_from_slice(&amp.to_le_bytes());
        let unpacked = StableCurve::unpack(&packed).unwrap();
        assert_eq!(curve, unpacked);
    }
}
