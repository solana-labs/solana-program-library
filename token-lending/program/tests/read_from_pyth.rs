#![cfg(feature = "test-bpf")]

mod helpers;

use helpers::*;
use solana_program::msg;
use solana_program_test::*;
use solana_sdk::{account::Account, pubkey::Pubkey};
use spl_token_lending::error::LendingError;
use spl_token_lending::processor::process_instruction;
use std::convert::TryInto;

#[tokio::test]
async fn test_success() {
    let mut test = ProgramTest::new(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(process_instruction),
    );

    let sol_oracle = add_sol_oracle(&mut test);

    let (mut banks_client, _payer, _recent_blockhash) = test.start().await;

    let pyth_product_account: Account = banks_client
        .get_account(sol_oracle.product_pubkey)
        .await
        .unwrap()
        .unwrap();

    let pyth_price_account: Account = banks_client
        .get_account(sol_oracle.price_pubkey)
        .await
        .unwrap()
        .unwrap();

    let pyth_product =
        pyth_client::cast::<pyth_client::Product>(pyth_product_account.data.as_slice());
    let pyth_price = pyth_client::cast::<pyth_client::Price>(pyth_price_account.data.as_slice());

    let mut quote_currency = [0u8; 32];

    let mut pyth_product_attribute_iter = pyth_product.attr[..].iter();
    let mut pyth_product_size = pyth_product.size as usize - pyth_client::PROD_HDR_SIZE;
    while pyth_product_size > 0 {
        let key = get_pyth_product_attribute(&mut pyth_product_attribute_iter);
        let value = get_pyth_product_attribute(&mut pyth_product_attribute_iter);

        if key == "quote_currency" {
            quote_currency[0..value.len()].clone_from_slice(value.as_bytes());
            break;
        }
        pyth_product_size -= 2 + key.len() + value.len();
    }

    if quote_currency == [0u8; 32] {
        panic!("Oracle quote currency not found");
    }

    match &pyth_price.ptype {
        pyth_client::PriceType::Price => {}
        _ => {
            panic!("Oracle price type is invalid");
        }
    }

    if pyth_price.valid_slot < 100 {
        panic!("Oracle price is stale");
    }

    let price: u64 = pyth_price
        .agg
        .price
        .checked_abs()
        .ok_or(LendingError::MathOverflow)
        .unwrap()
        .try_into()
        .map_err(|_| LendingError::MathOverflow)
        .unwrap();

    let decimals: u32 = pyth_price
        .expo
        .checked_abs()
        .ok_or(LendingError::MathOverflow)
        .unwrap()
        .try_into()
        .map_err(|_| LendingError::MathOverflow)
        .unwrap();

    // @FIXME: convert to decimal
    let market_price = price
        .checked_div(
            10u64
                .checked_pow(decimals)
                .ok_or(LendingError::MathOverflow)
                .unwrap(),
        )
        .ok_or(LendingError::MathOverflow)
        .unwrap()
        .checked_mul(10u64.pow(6))
        .ok_or(LendingError::MathOverflow)
        .unwrap();

    msg!("{}", market_price);

    return;
}

fn get_pyth_product_attribute<'a, T>(ite: &mut T) -> String
where
    T: Iterator<Item = &'a u8>,
{
    let mut len = *ite.next().unwrap() as usize;
    let mut val = String::with_capacity(len);
    while len > 0 {
        val.push(*ite.next().unwrap() as char);
        len -= 1;
    }
    return val;
}
