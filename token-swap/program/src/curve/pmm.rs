//! Proactive Market Making Algorithm

use {
    crate::{
        curve::calculator::{
            CurveCalculator, DynPack, RoundDirection, SwapWithoutFeesResult, TradeDirection,
            TradingTokenResult,
        },
        error::SwapError,
    },
    arrayref::{array_mut_ref, array_ref},
    solana_program::{
        program_error::ProgramError,
        program_pack::{IsInitialized, Pack, Sealed},
    },
    spl_math::precise_number::PreciseNumber,
    std::mem,
};

const BILLION: u128 = 1_000_000_000;

/// An implementation of the Proactive Market Maker algorithm as a curve
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PMMCurve {
    /// The mid price of the buy/sell requests.
    pub mid_price: u64,
    /// The liquidity parameter gives the flexibility to handle different market
    /// situations. It's akin to the slope in the depth chart. Parts per million.
    pub liquidity_parameter_ppb: u32,
    /// The initial number of base tokens in the inventory.
    pub base_regression_target: u64,
    /// The initial number of quote tokens in the inventory.
    pub quote_regression_target: u64,
}

/// Method to calculate the parameter R which determines where we are from the
/// equilibrium point where the current supply is equal to the initial supply.
pub fn calculate_r(
    base_amount: u128,
    base_regression_target: u128,
    quote_amount: u128,
    quote_regression_target: u128,
    liquidity_parameter_ppb: u128,
) -> Option<PreciseNumber> {
    let base_amount = PreciseNumber::new(base_amount)?;
    let base_regression_target = PreciseNumber::new(base_regression_target)?;
    let quote_amount = PreciseNumber::new(quote_amount)?;
    let quote_regression_target = PreciseNumber::new(quote_regression_target)?;
    let liquidity_parameter =
        PreciseNumber::new(liquidity_parameter_ppb)?.checked_div(&PreciseNumber::new(BILLION)?)?;

    return if base_amount.less_than(&base_regression_target) {
        // R = 1 - k + (B_0/B)^2 * k

        let ratio = base_regression_target.checked_div(&base_amount)?;
        let ratio_squared = ratio.checked_mul(&ratio)?;

        Some(
            PreciseNumber::new(1)?
                .checked_sub(&liquidity_parameter)?
                .checked_add(&ratio_squared.checked_mul(&liquidity_parameter)?)?,
        )
    } else if quote_amount.less_than(&quote_regression_target) {
        // R = 1/(1 - k + (Q_0/Q)^2 * k)

        let ratio = quote_regression_target.checked_div(&quote_amount)?;
        let ratio_squared = ratio.checked_mul(&ratio)?;

        Some(
            PreciseNumber::new(1)?.checked_div(
                &PreciseNumber::new(1)?
                    .checked_sub(&liquidity_parameter)?
                    .checked_add(&ratio_squared.checked_mul(&liquidity_parameter)?)?,
            )?,
        )
    } else {
        Some(PreciseNumber::new(1)?)
    };
}

/// Method to calculate the average transaction price which is the integral of
/// the marginal price.
pub fn general_integral(
    token_regression_target: u128,
    token_balance_1: u128,
    token_balance_2: u128,
    mid_price: u128,
    liquidity_parameter_ppb: u128,
) -> Option<u128> {
    // i * (1 - k + k * ((B_0)^2 / (B_1 * B_2)))

    let base_regression_target = PreciseNumber::new(token_regression_target)?;
    let base_balance_1 = PreciseNumber::new(token_balance_1)?;
    let base_balance_2 = PreciseNumber::new(token_balance_2)?;
    let mid_price = PreciseNumber::new(mid_price)?;
    let liquidity_parameter =
        PreciseNumber::new(liquidity_parameter_ppb)?.checked_div(&PreciseNumber::new(BILLION)?)?;

    let fair_amount = mid_price.checked_mul(&base_balance_1.checked_sub(&base_balance_2)?)?;
    let penalty_factor = base_regression_target
        .checked_mul(&base_regression_target)?
        .checked_div(&base_balance_1)?
        .checked_div(&base_balance_2)?
        .ceiling()?;
    let penalty = liquidity_parameter.checked_mul(&penalty_factor)?;

    Some(
        fair_amount
            .checked_mul(
                &PreciseNumber::new(1)?
                    .checked_sub(&liquidity_parameter)?
                    .checked_add(&penalty)?,
            )?
            .to_imprecise()?,
    )
}

/// Method to calculate the price when there is a shortage of quote tokens and
/// only the number of base tokens to buy or sell are given.
pub fn solve_quadratic_function_for_trade(
    quote_regression_target: u128,
    quote_balance_1: u128,
    mid_price_mul_delta_base: u128,
    delta_b_sign: bool,
    liquidity_parameter_ppb: u128,
) -> Option<u128> {
    // General Quadratic Equation Formula on this Equation:
    // (1 - k) * (Q_2)^2 + (((k * (Q_0)^2) / Q_1)) - Q_1 + k * Q_1 - i * dB) * (Q_2)^2

    let quote_regression_target = PreciseNumber::new(quote_regression_target)?;
    let quote_balance_1 = PreciseNumber::new(quote_balance_1)?;
    let mid_price_mul_delta_base = PreciseNumber::new(mid_price_mul_delta_base)?;
    let liquidity_parameter =
        PreciseNumber::new(liquidity_parameter_ppb)?.checked_div(&PreciseNumber::new(BILLION)?)?;

    let mut b_factor = liquidity_parameter
        .checked_mul(&quote_regression_target)?
        .checked_mul(&quote_regression_target)?
        .checked_div(&quote_balance_1)?;
    let mut b = PreciseNumber::new(1)?
        .checked_sub(&liquidity_parameter)?
        .checked_mul(&quote_balance_1)?;

    if delta_b_sign {
        b = b.checked_add(&mid_price_mul_delta_base)?;
    } else {
        b_factor = b_factor.checked_add(&mid_price_mul_delta_base)?;
    }

    let minus_b_sign = if b.greater_than_or_equal(&b_factor) {
        b = b.checked_sub(&b_factor)?;
        true
    } else {
        b = b_factor.checked_sub(&b)?;
        false
    };

    let square_root = PreciseNumber::new(1)?
        .checked_sub(&liquidity_parameter)?
        .checked_mul(&PreciseNumber::new(4)?)?
        .checked_mul(
            &liquidity_parameter
                .checked_mul(&quote_regression_target)?
                .checked_mul(&quote_regression_target)?,
        )?;
    let square_root = b.checked_mul(&b)?.checked_add(&square_root)?.sqrt()?;

    let denominator = PreciseNumber::new(1)?
        .checked_sub(&liquidity_parameter)?
        .checked_mul(&PreciseNumber::new(2)?)?;

    let numerator = if minus_b_sign {
        b.checked_add(&square_root)?
    } else {
        square_root.checked_sub(&b)?
    };

    if delta_b_sign {
        Some(
            numerator
                .checked_div(&denominator)?
                .floor()?
                .to_imprecise()?,
        )
    } else {
        Some(
            numerator
                .checked_div(&denominator)?
                .ceiling()?
                .to_imprecise()?,
        )
    }
}

/// Method to calculate the regression target at a certain oracle price.
pub fn solve_quadratic_function_for_target(
    token_balance_1: u128,
    liquidity_parameter_ppb: u128,
    fair_amount: u128,
) -> Option<u128> {
    // B_0 = B_1 + B_1 * ((sqrt(1 + (4 * k * dQ / B_1 * i)) - 1) / 2 * k)

    let token_balance_1 = PreciseNumber::new(token_balance_1)?;
    let liquidity_parameter =
        PreciseNumber::new(liquidity_parameter_ppb)?.checked_div(&PreciseNumber::new(BILLION)?)?;
    let fair_amount = PreciseNumber::new(fair_amount)?;

    let sqrt = liquidity_parameter
        .checked_mul(&fair_amount)?
        .checked_mul(&PreciseNumber::new(4)?)?
        .checked_div(&token_balance_1)?
        .ceiling()?;
    let sqrt = sqrt.checked_add(&PreciseNumber::new(1)?)?.sqrt()?;
    let premium = sqrt
        .checked_sub(&PreciseNumber::new(1)?)?
        .checked_div(&liquidity_parameter.checked_mul(&PreciseNumber::new(2)?)?)?
        .ceiling()?;

    Some(
        token_balance_1
            .checked_mul(&PreciseNumber::new(1)?.checked_add(&premium)?)?
            .to_imprecise()?,
    )
}

/// Helper method to integrate when R is > 1.
pub fn r_above_integrate(
    mid_price: u128,
    liquidity_parameter_ppb: u128,
    base_regression_target: u128,
    base_balance_1: u128,
    base_balance_2: u128,
) -> Option<u128> {
    general_integral(
        base_regression_target,
        base_balance_1,
        base_balance_2,
        mid_price,
        liquidity_parameter_ppb,
    )
}

/// Method to calculate the number of quote tokens to trade for amount base
/// tokens at equilibrium.
pub fn r_one_sell_base_token(
    mid_price: u128,
    liquidity_parameter_ppb: u128,
    amount: u128,
    target_quote_amount: u128,
) -> Option<u128> {
    let mid_price = PreciseNumber::new(mid_price)?;

    let quote_balance_2 = solve_quadratic_function_for_trade(
        target_quote_amount,
        target_quote_amount,
        mid_price
            .checked_mul(&PreciseNumber::new(amount)?)?
            .to_imprecise()?,
        false,
        liquidity_parameter_ppb,
    )?;

    Some(
        PreciseNumber::new(target_quote_amount)?
            .checked_sub(&PreciseNumber::new(quote_balance_2)?)?
            .to_imprecise()?,
    )
}

/// Method to calculate the number of quote tokens to trade for amount base
/// tokens at equilibrium.
pub fn r_one_buy_base_token(
    mid_price: u128,
    liquidity_parameter_ppb: u128,
    amount: u128,
    target_base_amount: u128,
) -> Option<u128> {
    if amount < target_base_amount {
        return None;
    }

    let base_balance_2 = PreciseNumber::new(target_base_amount)?
        .checked_sub(&PreciseNumber::new(amount)?)?
        .to_imprecise()?;

    Some(r_above_integrate(
        mid_price,
        liquidity_parameter_ppb,
        target_base_amount,
        target_base_amount,
        base_balance_2,
    )?)
}

/// Method to calculate the number of quote tokens to trade for amount base
/// tokens when R < 1.
pub fn r_below_sell_base_token(
    mid_price: u128,
    liquidity_parameter_ppb: u128,
    amount: u128,
    quote_balance: u128,
    target_quote_amount: u128,
) -> Option<u128> {
    let mid_price = PreciseNumber::new(mid_price)?;

    let quote_balance_2 = solve_quadratic_function_for_trade(
        target_quote_amount,
        quote_balance,
        mid_price
            .checked_mul(&PreciseNumber::new(amount)?)?
            .to_imprecise()?,
        false,
        liquidity_parameter_ppb,
    )?;

    Some(
        PreciseNumber::new(quote_balance)?
            .checked_sub(&PreciseNumber::new(quote_balance_2)?)?
            .to_imprecise()?,
    )
}

/// Method to calculate the number of quote tokens to trade for amount base
/// tokens when R < 1.
pub fn r_below_buy_base_token(
    mid_price: u128,
    liquidity_parameter_ppb: u128,
    amount: u128,
    quote_balance: u128,
    target_quote_amount: u128,
) -> Option<u128> {
    let mid_price = PreciseNumber::new(mid_price)?;

    let quote_balance_2 = solve_quadratic_function_for_trade(
        target_quote_amount,
        quote_balance,
        mid_price
            .checked_mul(&PreciseNumber::new(amount)?)?
            .ceiling()?
            .to_imprecise()?,
        true,
        liquidity_parameter_ppb,
    )?;

    Some(
        PreciseNumber::new(quote_balance_2)?
            .checked_sub(&PreciseNumber::new(quote_balance)?)?
            .to_imprecise()?,
    )
}

/// Method to calculate the number of quote tokens to trade for amount base
/// tokens when R < 1.
pub fn r_below_back_to_one(
    mid_price: u128,
    base_balance: u128,
    target_base_amount: u128,
    quote_balance: u128,
    liquidity_parameter_ppb: u128,
) -> Option<u128> {
    let sparse_base =
        PreciseNumber::new(base_balance)?.checked_sub(&PreciseNumber::new(target_base_amount)?)?;
    let mid_price = PreciseNumber::new(mid_price)?;
    let fair_amount = sparse_base.checked_mul(&mid_price)?;

    let new_target_quote = solve_quadratic_function_for_target(
        quote_balance,
        liquidity_parameter_ppb,
        fair_amount.to_imprecise()?,
    )?;

    Some(
        PreciseNumber::new(new_target_quote)?
            .checked_sub(&PreciseNumber::new(quote_balance)?)?
            .to_imprecise()?,
    )
}

/// Method to calculate the number of quote tokens to trade for amount base
/// tokens when R > 1.
pub fn r_above_sell_base_token(
    mid_price: u128,
    liquidity_parameter_ppb: u128,
    amount: u128,
    base_balance: u128,
    target_base_amount: u128,
) -> Option<u128> {
    let base_balance_1 = PreciseNumber::new(base_balance)?
        .checked_add(&PreciseNumber::new(amount)?)?
        .to_imprecise()?;

    Some(r_above_integrate(
        mid_price,
        liquidity_parameter_ppb,
        target_base_amount,
        base_balance_1,
        base_balance,
    )?)
}

/// Method to calculate the number of quote tokens to trade for amount base
/// tokens when R > 1.
pub fn r_above_buy_base_token(
    mid_price: u128,
    liquidity_parameter_ppb: u128,
    amount: u128,
    base_balance: u128,
    target_base_amount: u128,
) -> Option<u128> {
    if amount < base_balance {
        return None;
    }

    let base_balance_2 = PreciseNumber::new(base_balance)?
        .checked_sub(&PreciseNumber::new(amount)?)?
        .to_imprecise()?;

    Some(r_above_integrate(
        mid_price,
        liquidity_parameter_ppb,
        target_base_amount,
        base_balance,
        base_balance_2,
    )?)
}

/// Method to calculate the number of quote tokens to trade for amount base
/// tokens when R > 1.
pub fn r_above_back_to_one(
    mid_price: u128,
    quote_balance: u128,
    target_quote_amount: u128,
    base_balance: u128,
    liquidity_parameter_ppb: u128,
) -> Option<u128> {
    let sparse_quote = PreciseNumber::new(quote_balance)?
        .checked_sub(&PreciseNumber::new(target_quote_amount)?)?;
    let mid_price = PreciseNumber::new(mid_price)?;
    let fair_amount = sparse_quote.checked_div(&mid_price)?.floor()?;

    let new_target_base = solve_quadratic_function_for_target(
        base_balance,
        liquidity_parameter_ppb,
        fair_amount.to_imprecise()?,
    )?;

    Some(
        PreciseNumber::new(new_target_base)?
            .checked_sub(&PreciseNumber::new(base_balance)?)?
            .to_imprecise()?,
    )
}

/// Calculate base and quote target tokens.
pub fn get_expected_target(
    mid_price: u128,
    base_balance: u128,
    quote_balance: u128,
    target_base_amount: u128,
    target_quote_amount: u128,
    liquidity_parameter_ppb: u128,
) -> Option<(u128, u128)> {
    let r = calculate_r(
        base_balance,
        target_base_amount,
        quote_balance,
        target_quote_amount,
        liquidity_parameter_ppb,
    )?;

    return if r == PreciseNumber::new(1)? {
        Some((target_base_amount, target_quote_amount))
    } else if r.less_than(&PreciseNumber::new(1)?) {
        let pay_quote_token = r_below_back_to_one(
            mid_price,
            base_balance,
            target_base_amount,
            quote_balance,
            liquidity_parameter_ppb,
        )?;

        Some((
            target_base_amount,
            PreciseNumber::new(quote_balance)?
                .checked_add(&PreciseNumber::new(pay_quote_token)?)?
                .to_imprecise()?,
        ))
    } else {
        let pay_base_token = r_above_back_to_one(
            mid_price,
            base_balance,
            target_base_amount,
            quote_balance,
            liquidity_parameter_ppb,
        )?;

        Some((
            PreciseNumber::new(base_balance)?
                .checked_add(&PreciseNumber::new(pay_base_token)?)?
                .to_imprecise()?,
            target_quote_amount,
        ))
    };
}

/// Query the sell price for base token.
pub fn query_sell_base_token(
    mid_price: u128,
    base_balance: u128,
    quote_balance: u128,
    target_base_amount: u128,
    target_quote_amount: u128,
    liquidity_parameter_ppb: u128,
    amount: u128,
) -> Option<u128> {
    let (new_base_target, new_quote_target) = get_expected_target(
        mid_price,
        base_balance,
        quote_balance,
        target_base_amount,
        target_quote_amount,
        liquidity_parameter_ppb,
    )?;

    let r = calculate_r(
        base_balance,
        target_base_amount,
        quote_balance,
        target_quote_amount,
        liquidity_parameter_ppb,
    )?;

    if r == PreciseNumber::new(1)? {
        Some(r_one_sell_base_token(
            mid_price,
            liquidity_parameter_ppb,
            amount,
            new_quote_target,
        )?)
    } else if r.less_than(&PreciseNumber::new(1)?) {
        let back_to_one_pay_base = PreciseNumber::new(new_base_target)?
            .checked_sub(&PreciseNumber::new(base_balance)?)?
            .to_imprecise()?;
        let back_to_one_receive_quote = PreciseNumber::new(quote_balance)?
            .checked_sub(&PreciseNumber::new(new_quote_target)?)?
            .to_imprecise()?;

        if amount < back_to_one_pay_base {
            let receive_quote = r_above_sell_base_token(
                mid_price,
                liquidity_parameter_ppb,
                amount,
                base_balance,
                target_base_amount,
            )?;

            return if receive_quote > back_to_one_receive_quote {
                Some(back_to_one_receive_quote)
            } else {
                Some(receive_quote)
            };
        } else if amount == back_to_one_pay_base {
            return Some(back_to_one_receive_quote);
        } else {
            return Some(
                PreciseNumber::new(back_to_one_receive_quote)?
                    .checked_add(&PreciseNumber::new(r_one_sell_base_token(
                        mid_price,
                        liquidity_parameter_ppb,
                        PreciseNumber::new(amount)?
                            .checked_sub(&PreciseNumber::new(back_to_one_pay_base)?)?
                            .to_imprecise()?,
                        new_quote_target,
                    )?)?)?
                    .to_imprecise()?,
            );
        }
    } else {
        Some(r_below_sell_base_token(
            mid_price,
            liquidity_parameter_ppb,
            amount,
            quote_balance,
            new_quote_target,
        )?)
    }
}

/// Query the buy price for base token.
pub fn query_buy_base_token(
    mid_price: u128,
    base_balance: u128,
    quote_balance: u128,
    target_base_amount: u128,
    target_quote_amount: u128,
    liquidity_parameter_ppb: u128,
    amount: u128,
) -> Option<u128> {
    let (new_base_target, new_quote_target) = get_expected_target(
        mid_price,
        base_balance,
        quote_balance,
        target_base_amount,
        target_quote_amount,
        liquidity_parameter_ppb,
    )?;

    let r = calculate_r(
        base_balance,
        target_base_amount,
        quote_balance,
        target_quote_amount,
        liquidity_parameter_ppb,
    )?;

    if r == PreciseNumber::new(1)? {
        Some(r_one_buy_base_token(
            mid_price,
            liquidity_parameter_ppb,
            amount,
            new_base_target,
        )?)
    } else if r.less_than(&PreciseNumber::new(1)?) {
        let back_to_one_pay_quote = PreciseNumber::new(new_quote_target)?
            .checked_sub(&PreciseNumber::new(quote_balance)?)?
            .to_imprecise()?;
        let back_to_one_receive_base = PreciseNumber::new(base_balance)?
            .checked_sub(&PreciseNumber::new(new_base_target)?)?
            .to_imprecise()?;

        if amount < back_to_one_receive_base {
            Some(r_below_buy_base_token(
                mid_price,
                liquidity_parameter_ppb,
                amount,
                quote_balance,
                new_quote_target,
            )?)
        } else if amount == back_to_one_receive_base {
            return Some(back_to_one_pay_quote);
        } else {
            return Some(
                PreciseNumber::new(back_to_one_pay_quote)?
                    .checked_add(&PreciseNumber::new(r_one_buy_base_token(
                        mid_price,
                        liquidity_parameter_ppb,
                        PreciseNumber::new(amount)?
                            .checked_sub(&PreciseNumber::new(back_to_one_receive_base)?)?
                            .to_imprecise()?,
                        new_base_target,
                    )?)?)?
                    .to_imprecise()?,
            );
        }
    } else {
        Some(r_above_buy_base_token(
            mid_price,
            liquidity_parameter_ppb,
            amount,
            base_balance,
            new_base_target,
        )?)
    }
}

impl CurveCalculator for PMMCurve {
    fn swap_without_fees(
        &self,
        source_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
        trade_direction: TradeDirection,
    ) -> Option<SwapWithoutFeesResult> {
        match trade_direction {
            TradeDirection::AtoB => {
                let receive_quote = query_sell_base_token(
                    self.mid_price.into(),
                    swap_source_amount,
                    swap_destination_amount,
                    self.base_regression_target.into(),
                    self.quote_regression_target.into(),
                    self.liquidity_parameter_ppb.into(),
                    source_amount,
                )?;

                Some(SwapWithoutFeesResult {
                    source_amount_swapped: swap_source_amount,
                    destination_amount_swapped: receive_quote,
                })
            }
            TradeDirection::BtoA => {
                let pay_quote = query_buy_base_token(
                    self.mid_price.into(),
                    swap_destination_amount,
                    swap_source_amount,
                    self.base_regression_target.into(),
                    self.quote_regression_target.into(),
                    self.liquidity_parameter_ppb.into(),
                    source_amount,
                )?;

                Some(SwapWithoutFeesResult {
                    source_amount_swapped: swap_source_amount,
                    destination_amount_swapped: pay_quote,
                })
            }
        }
    }

    fn pool_tokens_to_trading_tokens(
        &self,
        _pool_tokens: u128,
        _pool_token_supply: u128,
        _swap_token_a_amount: u128,
        _swap_token_b_amount: u128,
        _round_direction: RoundDirection,
    ) -> Option<TradingTokenResult> {
        unimplemented!()
    }

    fn deposit_single_token_type(
        &self,
        _source_amount: u128,
        _swap_token_a_amount: u128,
        _swap_token_b_amount: u128,
        _pool_supply: u128,
        _trade_direction: TradeDirection,
    ) -> Option<u128> {
        unimplemented!()
    }

    fn withdraw_single_token_type_exact_out(
        &self,
        _source_amount: u128,
        _swap_token_a_amount: u128,
        _swap_token_b_amount: u128,
        _pool_supply: u128,
        _trade_direction: TradeDirection,
    ) -> Option<u128> {
        unimplemented!()
    }

    fn validate(&self) -> Result<(), SwapError> {
        if u128::from(self.liquidity_parameter_ppb) > BILLION {
            Err(SwapError::InvalidCurve)
        } else {
            Ok(())
        }
    }

    fn validate_supply(&self, token_a_amount: u64, token_b_amount: u64) -> Result<(), SwapError> {
        if token_a_amount == 0 {
            return Err(SwapError::EmptySupply);
        }
        if token_b_amount == 0 {
            return Err(SwapError::EmptySupply);
        }
        Ok(())
    }

    fn allows_deposits(&self) -> bool {
        true
    }

    fn normalized_value(
        &self,
        swap_token_a_amount: u128,
        swap_token_b_amount: u128,
    ) -> Option<PreciseNumber> {
        let swap_token_a_amount = PreciseNumber::new(swap_token_a_amount)?;
        let swap_token_b_amount = PreciseNumber::new(swap_token_b_amount)?;
        let liquidity_parameter = PreciseNumber::new(self.liquidity_parameter_ppb.into())?
            .checked_div(&PreciseNumber::new(BILLION)?)?;

        liquidity_parameter
            .checked_mul(&swap_token_a_amount.checked_mul(&swap_token_b_amount)?)?
            .sqrt()?
            .checked_add(
                &PreciseNumber::new(1)?
                    .checked_sub(&liquidity_parameter)?
                    .checked_mul(
                        &swap_token_a_amount
                            .checked_add(&swap_token_b_amount)?
                            .checked_div(&PreciseNumber::new(2)?)?,
                    )?,
            )
    }
}

impl IsInitialized for PMMCurve {
    fn is_initialized(&self) -> bool {
        true
    }
}
impl Sealed for PMMCurve {}
impl Pack for PMMCurve {
    const LEN: usize = 4 * mem::size_of::<u64>();

    fn pack_into_slice(&self, output: &mut [u8]) {
        (self as &dyn DynPack).pack_into_slice(output);
    }

    fn unpack_from_slice(input: &[u8]) -> Result<PMMCurve, ProgramError> {
        let mid_price = array_ref![input, 0, 8];
        let liquidity_parameter_ppb = array_ref![input, 8, 4];
        let base_regression_target = array_ref![input, 12, 8];
        let quote_regression_target = array_ref![input, 20, 8];

        Ok(Self {
            mid_price: u64::from_le_bytes(*mid_price),
            liquidity_parameter_ppb: u32::from_le_bytes(*liquidity_parameter_ppb),
            base_regression_target: u64::from_le_bytes(*base_regression_target),
            quote_regression_target: u64::from_le_bytes(*quote_regression_target),
        })
    }
}

impl DynPack for PMMCurve {
    fn pack_into_slice(&self, output: &mut [u8]) {
        let mid_price = array_mut_ref![output, 0, 8];
        *mid_price = self.mid_price.to_le_bytes();

        let liquidity_parameter_ppb = array_mut_ref![output, 8, 4];
        *liquidity_parameter_ppb = self.liquidity_parameter_ppb.to_le_bytes();

        let base_regression_target = array_mut_ref![output, 12, 8];
        *base_regression_target = self.base_regression_target.to_le_bytes();

        let quote_regression_target = array_mut_ref![output, 20, 8];
        *quote_regression_target = self.quote_regression_target.to_le_bytes();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::calculator::INITIAL_SWAP_POOL_AMOUNT;

    #[test]
    fn initial_pool_amount() {
        let calculator = PMMCurve {
            mid_price: 1000,
            liquidity_parameter_ppb: 1_000_000_000,
            base_regression_target: 1000,
            quote_regression_target: 1000,
        };
        assert_eq!(calculator.new_pool_supply(), INITIAL_SWAP_POOL_AMOUNT);
    }

    #[test]
    fn pack_constant_product_curve() {
        let curve = PMMCurve {
            mid_price: 1000,
            liquidity_parameter_ppb: 1_000_000_000,
            base_regression_target: 1000,
            quote_regression_target: 1000,
        };

        let mut packed = [0u8; PMMCurve::LEN];
        Pack::pack_into_slice(&curve, &mut packed[..]);
        let unpacked = PMMCurve::unpack(&packed).unwrap();
        assert_eq!(curve, unpacked);

        let packed = vec![];
        let unpacked = PMMCurve::unpack(&packed).unwrap();
        assert_eq!(curve, unpacked);
    }

    #[test]
    fn swap_liquidity_parameter_amm() {
        let curve = PMMCurve {
            mid_price: 1000,
            liquidity_parameter_ppb: 1_000_000_000,
            base_regression_target: 1000,
            quote_regression_target: 1000,
        };

        let invariant = curve.base_regression_target * curve.quote_regression_target;

        let swapped_amount = curve
            .swap_without_fees(
                10,
                curve.base_regression_target.into(),
                curve.quote_regression_target.into(),
                TradeDirection::AtoB,
            )
            .unwrap();

        let new_invariant: u128 = (u128::from(curve.base_regression_target)
            + swapped_amount.source_amount_swapped)
            * (u128::from(curve.quote_regression_target)
                - swapped_amount.destination_amount_swapped);

        assert_eq!(new_invariant, invariant.into());
    }
}
