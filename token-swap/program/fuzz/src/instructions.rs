use {
    arbitrary::Arbitrary,
    honggfuzz::fuzz,
    spl_math::precise_number::PreciseNumber,
    spl_token::error::TokenError,
    spl_token_swap::{
        curve::{
            base::{CurveType, SwapCurve},
            calculator::TradeDirection,
            constant_price::ConstantPriceCurve,
            constant_product::ConstantProductCurve,
            fees::Fees,
            offset::OffsetCurve,
            stable::StableCurve,
        },
        error::SwapError,
        instruction::{
            DepositAllTokenTypes, DepositSingleTokenTypeExactAmountIn, Swap, WithdrawAllTokenTypes,
            WithdrawSingleTokenTypeExactAmountOut,
        },
    },
    spl_token_swap_fuzz::{
        native_account_data::NativeAccountData,
        native_token::{get_token_balance, transfer},
        native_token_swap::NativeTokenSwap,
    },
    std::collections::{HashMap, HashSet},
};

#[derive(Debug, Arbitrary, Clone)]
struct FuzzData {
    curve_type: CurveType,
    instructions: Vec<FuzzInstruction>,
}

#[derive(Debug, Arbitrary, Clone)]
enum FuzzInstruction {
    Swap {
        token_a_id: AccountId,
        token_b_id: AccountId,
        trade_direction: TradeDirection,
        instruction: Swap,
    },
    DepositAllTokenTypes {
        token_a_id: AccountId,
        token_b_id: AccountId,
        pool_token_id: AccountId,
        instruction: DepositAllTokenTypes,
    },
    WithdrawAllTokenTypes {
        token_a_id: AccountId,
        token_b_id: AccountId,
        pool_token_id: AccountId,
        instruction: WithdrawAllTokenTypes,
    },
    DepositSingleTokenTypeExactAmountIn {
        token_account_id: AccountId,
        trade_direction: TradeDirection,
        pool_token_id: AccountId,
        instruction: DepositSingleTokenTypeExactAmountIn,
    },
    WithdrawSingleTokenTypeExactAmountOut {
        token_account_id: AccountId,
        trade_direction: TradeDirection,
        pool_token_id: AccountId,
        instruction: WithdrawSingleTokenTypeExactAmountOut,
    },
}

/// Use u8 as an account id to simplify the address space and re-use accounts
/// more often.
type AccountId = u8;

const INITIAL_SWAP_TOKEN_A_AMOUNT: u64 = 100_000_000_000;
const INITIAL_SWAP_TOKEN_B_AMOUNT: u64 = 300_000_000_000;

const INITIAL_USER_TOKEN_A_AMOUNT: u64 = 1_000_000_000;
const INITIAL_USER_TOKEN_B_AMOUNT: u64 = 3_000_000_000;

fn main() {
    loop {
        fuzz!(|fuzz_data: FuzzData| { run_fuzz(fuzz_data) });
    }
}

fn run_fuzz(fuzz_data: FuzzData) {
    let trade_fee_numerator = 25;
    let trade_fee_denominator = 10000;
    let owner_trade_fee_numerator = 5;
    let owner_trade_fee_denominator = 10000;
    let owner_withdraw_fee_numerator = 30;
    let owner_withdraw_fee_denominator = 10000;
    let host_fee_numerator = 1;
    let host_fee_denominator = 5;
    let fees = Fees {
        trade_fee_numerator,
        trade_fee_denominator,
        owner_trade_fee_numerator,
        owner_trade_fee_denominator,
        owner_withdraw_fee_numerator,
        owner_withdraw_fee_denominator,
        host_fee_numerator,
        host_fee_denominator,
    };
    let swap_curve = get_swap_curve(fuzz_data.curve_type);
    let mut token_swap = NativeTokenSwap::new(
        fees,
        swap_curve.clone(),
        INITIAL_SWAP_TOKEN_A_AMOUNT,
        INITIAL_SWAP_TOKEN_B_AMOUNT,
    );

    // keep track of all accounts, including swap accounts
    let mut token_a_accounts: HashMap<AccountId, NativeAccountData> = HashMap::new();
    let mut token_b_accounts: HashMap<AccountId, NativeAccountData> = HashMap::new();
    let mut pool_accounts: HashMap<AccountId, NativeAccountData> = HashMap::new();

    // add all the pool and token accounts that will be needed
    for fuzz_instruction in &fuzz_data.instructions {
        let (token_a_id, token_b_id, pool_token_id) = match fuzz_instruction.clone() {
            FuzzInstruction::Swap {
                token_a_id,
                token_b_id,
                ..
            } => (Some(token_a_id), Some(token_b_id), None),

            FuzzInstruction::DepositAllTokenTypes {
                token_a_id,
                token_b_id,
                pool_token_id,
                ..
            } => (Some(token_a_id), Some(token_b_id), Some(pool_token_id)),

            FuzzInstruction::WithdrawAllTokenTypes {
                token_a_id,
                token_b_id,
                pool_token_id,
                ..
            } => (Some(token_a_id), Some(token_b_id), Some(pool_token_id)),

            FuzzInstruction::DepositSingleTokenTypeExactAmountIn {
                token_account_id,
                trade_direction,
                pool_token_id,
                ..
            } => match trade_direction {
                TradeDirection::AtoB => (Some(token_account_id), None, Some(pool_token_id)),
                TradeDirection::BtoA => (None, Some(token_account_id), Some(pool_token_id)),
            },

            FuzzInstruction::WithdrawSingleTokenTypeExactAmountOut {
                token_account_id,
                trade_direction,
                pool_token_id,
                ..
            } => match trade_direction {
                TradeDirection::AtoB => (Some(token_account_id), None, Some(pool_token_id)),
                TradeDirection::BtoA => (None, Some(token_account_id), Some(pool_token_id)),
            },
        };
        if let Some(token_a_id) = token_a_id {
            token_a_accounts
                .entry(token_a_id)
                .or_insert_with(|| token_swap.create_token_a_account(INITIAL_USER_TOKEN_A_AMOUNT));
        }
        if let Some(token_b_id) = token_b_id {
            token_b_accounts
                .entry(token_b_id)
                .or_insert_with(|| token_swap.create_token_b_account(INITIAL_USER_TOKEN_B_AMOUNT));
        }
        if let Some(pool_token_id) = pool_token_id {
            pool_accounts
                .entry(pool_token_id)
                .or_insert_with(|| token_swap.create_pool_account());
        }
    }

    let pool_tokens = [&token_swap.pool_token_account, &token_swap.pool_fee_account]
        .iter()
        .map(|&x| get_token_balance(x))
        .sum::<u64>() as u128;
    let initial_pool_token_amount =
        pool_tokens + pool_accounts.values().map(get_token_balance).sum::<u64>() as u128;
    let initial_swap_token_a_amount = get_token_balance(&token_swap.token_a_account) as u128;
    let initial_swap_token_b_amount = get_token_balance(&token_swap.token_b_account) as u128;

    // to ensure that we never create or remove base tokens
    let before_total_token_a =
        INITIAL_SWAP_TOKEN_A_AMOUNT + get_total_token_a_amount(&fuzz_data.instructions);
    let before_total_token_b =
        INITIAL_SWAP_TOKEN_B_AMOUNT + get_total_token_b_amount(&fuzz_data.instructions);

    for fuzz_instruction in fuzz_data.instructions {
        run_fuzz_instruction(
            fuzz_instruction,
            &mut token_swap,
            &mut token_a_accounts,
            &mut token_b_accounts,
            &mut pool_accounts,
        );
    }

    // Omit fees intentionally, because fees in the form of pool tokens can
    // dilute the value of the pool.  For example, if we perform a small swap
    // whose fee is worth less than 1 pool token, we may round up to 1 pool
    // token and mint it as the fee.  Depending on the size of the pool, this
    // fee can actually reduce the value of pool tokens.
    let pool_token_amount =
        pool_tokens + pool_accounts.values().map(get_token_balance).sum::<u64>() as u128;
    let swap_token_a_amount = get_token_balance(&token_swap.token_a_account) as u128;
    let swap_token_b_amount = get_token_balance(&token_swap.token_b_account) as u128;

    let initial_pool_value = swap_curve
        .calculator
        .normalized_value(initial_swap_token_a_amount, initial_swap_token_b_amount)
        .unwrap();
    let pool_value = swap_curve
        .calculator
        .normalized_value(swap_token_a_amount, swap_token_b_amount)
        .unwrap();

    let pool_token_amount = PreciseNumber::new(pool_token_amount).unwrap();
    let initial_pool_token_amount = PreciseNumber::new(initial_pool_token_amount).unwrap();
    assert!(initial_pool_value
        .checked_div(&initial_pool_token_amount)
        .unwrap()
        .less_than_or_equal(&pool_value.checked_div(&pool_token_amount).unwrap()));

    // check total token a and b amounts
    let after_total_token_a = token_a_accounts
        .values()
        .map(get_token_balance)
        .sum::<u64>()
        + get_token_balance(&token_swap.token_a_account);
    assert_eq!(before_total_token_a, after_total_token_a);
    let after_total_token_b = token_b_accounts
        .values()
        .map(get_token_balance)
        .sum::<u64>()
        + get_token_balance(&token_swap.token_b_account);
    assert_eq!(before_total_token_b, after_total_token_b);

    // Final check to make sure that withdrawing everything works
    //
    // First, transfer all pool tokens to the fee account to avoid withdrawal
    // fees and a potential crash when withdrawing just 1 pool token.
    let mut fee_account = token_swap.pool_fee_account.clone();
    for mut pool_account in pool_accounts.values_mut() {
        let pool_token_amount = get_token_balance(pool_account);
        if pool_token_amount > 0 {
            transfer(&mut pool_account, &mut fee_account, pool_token_amount);
        }
    }
    let mut pool_account = token_swap.pool_token_account.clone();
    let pool_token_amount = get_token_balance(&pool_account);
    transfer(&mut pool_account, &mut fee_account, pool_token_amount);

    // Withdraw everything once again
    let mut withdrawn_token_a_account = token_swap.create_token_a_account(0);
    let mut withdrawn_token_b_account = token_swap.create_token_b_account(0);
    token_swap
        .withdraw_all(
            &mut fee_account,
            &mut withdrawn_token_a_account,
            &mut withdrawn_token_b_account,
        )
        .unwrap();

    let after_total_token_a = token_a_accounts
        .values()
        .map(get_token_balance)
        .sum::<u64>()
        + get_token_balance(&withdrawn_token_a_account)
        + get_token_balance(&token_swap.token_a_account);
    assert_eq!(before_total_token_a, after_total_token_a);
    let after_total_token_b = token_b_accounts
        .values()
        .map(get_token_balance)
        .sum::<u64>()
        + get_token_balance(&withdrawn_token_b_account)
        + get_token_balance(&token_swap.token_b_account);
    assert_eq!(before_total_token_b, after_total_token_b);
}

fn run_fuzz_instruction(
    fuzz_instruction: FuzzInstruction,
    token_swap: &mut NativeTokenSwap,
    token_a_accounts: &mut HashMap<AccountId, NativeAccountData>,
    token_b_accounts: &mut HashMap<AccountId, NativeAccountData>,
    pool_accounts: &mut HashMap<AccountId, NativeAccountData>,
) {
    let result = match fuzz_instruction {
        FuzzInstruction::Swap {
            token_a_id,
            token_b_id,
            trade_direction,
            instruction,
        } => {
            let mut token_a_account = token_a_accounts.get_mut(&token_a_id).unwrap();
            let mut token_b_account = token_b_accounts.get_mut(&token_b_id).unwrap();
            match trade_direction {
                TradeDirection::AtoB => {
                    token_swap.swap_a_to_b(&mut token_a_account, &mut token_b_account, instruction)
                }
                TradeDirection::BtoA => {
                    token_swap.swap_b_to_a(&mut token_b_account, &mut token_a_account, instruction)
                }
            }
        }
        FuzzInstruction::DepositAllTokenTypes {
            token_a_id,
            token_b_id,
            pool_token_id,
            instruction,
        } => {
            let mut token_a_account = token_a_accounts.get_mut(&token_a_id).unwrap();
            let mut token_b_account = token_b_accounts.get_mut(&token_b_id).unwrap();
            let mut pool_account = pool_accounts.get_mut(&pool_token_id).unwrap();
            token_swap.deposit_all_token_types(
                &mut token_a_account,
                &mut token_b_account,
                &mut pool_account,
                instruction,
            )
        }
        FuzzInstruction::WithdrawAllTokenTypes {
            token_a_id,
            token_b_id,
            pool_token_id,
            instruction,
        } => {
            let mut token_a_account = token_a_accounts.get_mut(&token_a_id).unwrap();
            let mut token_b_account = token_b_accounts.get_mut(&token_b_id).unwrap();
            let mut pool_account = pool_accounts.get_mut(&pool_token_id).unwrap();
            token_swap.withdraw_all_token_types(
                &mut pool_account,
                &mut token_a_account,
                &mut token_b_account,
                instruction,
            )
        }
        FuzzInstruction::DepositSingleTokenTypeExactAmountIn {
            token_account_id,
            trade_direction,
            pool_token_id,
            instruction,
        } => {
            let mut source_token_account = match trade_direction {
                TradeDirection::AtoB => token_a_accounts.get_mut(&token_account_id).unwrap(),
                TradeDirection::BtoA => token_b_accounts.get_mut(&token_account_id).unwrap(),
            };
            let mut pool_account = pool_accounts.get_mut(&pool_token_id).unwrap();
            token_swap.deposit_single_token_type_exact_amount_in(
                &mut source_token_account,
                &mut pool_account,
                instruction,
            )
        }
        FuzzInstruction::WithdrawSingleTokenTypeExactAmountOut {
            token_account_id,
            trade_direction,
            pool_token_id,
            instruction,
        } => {
            let mut destination_token_account = match trade_direction {
                TradeDirection::AtoB => token_a_accounts.get_mut(&token_account_id).unwrap(),
                TradeDirection::BtoA => token_b_accounts.get_mut(&token_account_id).unwrap(),
            };
            let mut pool_account = pool_accounts.get_mut(&pool_token_id).unwrap();
            token_swap.withdraw_single_token_type_exact_amount_out(
                &mut pool_account,
                &mut destination_token_account,
                instruction,
            )
        }
    };
    result
        .map_err(|e| {
            if !(e == SwapError::CalculationFailure.into()
                || e == SwapError::ConversionFailure.into()
                || e == SwapError::FeeCalculationFailure.into()
                || e == SwapError::ExceededSlippage.into()
                || e == SwapError::ZeroTradingTokens.into()
                || e == SwapError::UnsupportedCurveOperation.into()
                || e == TokenError::InsufficientFunds.into())
            {
                println!("{:?}", e);
                Err(e).unwrap()
            }
        })
        .ok();
}

fn get_total_token_a_amount(fuzz_instructions: &[FuzzInstruction]) -> u64 {
    let mut token_a_ids = HashSet::new();
    for fuzz_instruction in fuzz_instructions.iter() {
        match fuzz_instruction {
            FuzzInstruction::Swap { token_a_id, .. } => token_a_ids.insert(token_a_id),
            FuzzInstruction::DepositAllTokenTypes { token_a_id, .. } => {
                token_a_ids.insert(token_a_id)
            }
            FuzzInstruction::WithdrawAllTokenTypes { token_a_id, .. } => {
                token_a_ids.insert(token_a_id)
            }
            FuzzInstruction::DepositSingleTokenTypeExactAmountIn {
                token_account_id,
                trade_direction,
                ..
            } => match trade_direction {
                TradeDirection::AtoB => token_a_ids.insert(token_account_id),
                _ => false,
            },
            FuzzInstruction::WithdrawSingleTokenTypeExactAmountOut {
                token_account_id,
                trade_direction,
                ..
            } => match trade_direction {
                TradeDirection::AtoB => token_a_ids.insert(token_account_id),
                _ => false,
            },
        };
    }
    (token_a_ids.len() as u64) * INITIAL_USER_TOKEN_A_AMOUNT
}

fn get_total_token_b_amount(fuzz_instructions: &[FuzzInstruction]) -> u64 {
    let mut token_b_ids = HashSet::new();
    for fuzz_instruction in fuzz_instructions.iter() {
        match fuzz_instruction {
            FuzzInstruction::Swap { token_b_id, .. } => token_b_ids.insert(token_b_id),
            FuzzInstruction::DepositAllTokenTypes { token_b_id, .. } => {
                token_b_ids.insert(token_b_id)
            }
            FuzzInstruction::WithdrawAllTokenTypes { token_b_id, .. } => {
                token_b_ids.insert(token_b_id)
            }
            FuzzInstruction::DepositSingleTokenTypeExactAmountIn {
                token_account_id,
                trade_direction,
                ..
            } => match trade_direction {
                TradeDirection::BtoA => token_b_ids.insert(token_account_id),
                _ => false,
            },
            FuzzInstruction::WithdrawSingleTokenTypeExactAmountOut {
                token_account_id,
                trade_direction,
                ..
            } => match trade_direction {
                TradeDirection::BtoA => token_b_ids.insert(token_account_id),
                _ => false,
            },
        };
    }
    (token_b_ids.len() as u64) * INITIAL_USER_TOKEN_B_AMOUNT
}

fn get_swap_curve(curve_type: CurveType) -> SwapCurve {
    SwapCurve {
        curve_type,
        calculator: match curve_type {
            CurveType::ConstantProduct => Box::new(ConstantProductCurve),
            CurveType::ConstantPrice => Box::new(ConstantPriceCurve {
                token_b_price: 10_000_000,
            }),
            CurveType::Stable => Box::new(StableCurve { amp: 100 }),
            CurveType::Offset => Box::new(OffsetCurve {
                token_b_offset: 100_000_000_000,
            }),
        },
    }
}
