#![cfg(feature = "test-bpf")]

mod helpers;

use {
    solana_program::{hash::Hash, pubkey::Pubkey},
    solana_program_test::{tokio, BanksClient},
    solana_sdk::{
        instruction::InstructionError,
        signature::{Keypair, Signer},
        transaction::TransactionError,
        transport::TransportError,
    },
    spl_token_swap::{
        curve::{
            base::{CurveType, SwapCurve},
            constant_product::ConstantProductCurve,
            fees::Fees,
        },
        error::SwapError,
        id,
    },
};

#[tokio::test]
async fn fn_test_initialize_pass() {
    let (mut banks_client, payer, recent_blockhash) = helpers::program_test().start().await;

    let mut swap = create_standard_setup(&mut banks_client, &payer, &recent_blockhash).await;

    // create valid swap
    swap.initialize_swap(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();
}

#[tokio::test]
async fn fn_test_initialize_fail_wrong_swap_acct() {
    let (mut banks_client, payer, recent_blockhash) = helpers::program_test().start().await;

    let mut swap = create_standard_setup(&mut banks_client, &payer, &recent_blockhash).await;

    // wrong pda for swap account
    {
        swap.swap_pubkey = Pubkey::new_unique();

        let transaction_error = swap
            .initialize_swap(&mut banks_client, &payer, &recent_blockhash)
            .await
            .err()
            .unwrap();
        match transaction_error {
            TransportError::TransactionError(TransactionError::InstructionError(
                _,
                InstructionError::Custom(error_index),
            )) => {
                assert_eq!(error_index, SwapError::InvalidProgramAddress as u32);
            }
            _ => {
                panic!("Wrong error occurs while trying wrong pda for swap account")
            }
        }
    }
}

#[tokio::test]
async fn fn_test_initialize_fail_wrong_nonce() {
    let (mut banks_client, payer, recent_blockhash) = helpers::program_test().start().await;

    let mut swap = create_standard_setup(&mut banks_client, &payer, &recent_blockhash).await;

    // wrong nonce for authority_key
    {
        swap.nonce -= 1;

        let transaction_error = swap
            .initialize_swap(&mut banks_client, &payer, &recent_blockhash)
            .await
            .err()
            .unwrap();
        match transaction_error {
            TransportError::TransactionError(TransactionError::InstructionError(
                _,
                InstructionError::Custom(error_index),
            )) => {
                assert_eq!(error_index, SwapError::InvalidProgramAddress as u32);
            }
            _ => {
                panic!("Wrong error occurs while trying wrong nonce for authority_key")
            }
        }
    }
}

#[tokio::test]
async fn fn_test_initialize_fail_uninit_token_a() {
    let (mut banks_client, payer, recent_blockhash) = helpers::program_test().start().await;

    let mut swap = create_standard_setup(&mut banks_client, &payer, &recent_blockhash).await;

    // uninitialized token a account
    {
        let token_a_key = Keypair::new();

        helpers::create_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &token_a_key,
            &spl_token::id(),
            0,
        )
        .await
        .unwrap();

        swap.token_a_key = token_a_key;

        let transaction_error = swap
            .initialize_swap(&mut banks_client, &payer, &recent_blockhash)
            .await
            .err()
            .unwrap();
        match transaction_error {
            TransportError::TransactionError(TransactionError::InstructionError(
                _,
                InstructionError::Custom(error_index),
            )) => {
                assert_eq!(error_index, SwapError::ExpectedAccount as u32);
            }
            _ => {
                panic!("Wrong error occurs while trying uninitialized token a account")
            }
        }
    }
}

#[tokio::test]
async fn fn_test_initialize_fail_uninit_token_b() {
    let (mut banks_client, payer, recent_blockhash) = helpers::program_test().start().await;

    let mut swap = create_standard_setup(&mut banks_client, &payer, &recent_blockhash).await;

    // uninitialized token b account
    {
        let token_b_key = Keypair::new();

        helpers::create_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &token_b_key,
            &spl_token::id(),
            0,
        )
        .await
        .unwrap();

        swap.token_b_key = token_b_key;

        let transaction_error = swap
            .initialize_swap(&mut banks_client, &payer, &recent_blockhash)
            .await
            .err()
            .unwrap();
        match transaction_error {
            TransportError::TransactionError(TransactionError::InstructionError(
                _,
                InstructionError::Custom(error_index),
            )) => {
                assert_eq!(error_index, SwapError::ExpectedAccount as u32);
            }
            _ => {
                panic!("Wrong error occurs while trying uninitialized token b account")
            }
        }
    }
}

#[tokio::test]
async fn fn_test_initialize_fail_uninit_pool_mint() {
    let (mut banks_client, payer, recent_blockhash) = helpers::program_test().start().await;

    let mut swap = create_standard_setup(&mut banks_client, &payer, &recent_blockhash).await;

    // uninitialized pool mint
    {
        let pool_mint_key = Keypair::new();

        helpers::create_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &pool_mint_key,
            &spl_token::id(),
            0,
        )
        .await
        .unwrap();

        swap.pool_mint_key = pool_mint_key;

        let transaction_error = swap
            .initialize_swap(&mut banks_client, &payer, &recent_blockhash)
            .await
            .err()
            .unwrap();
        match transaction_error {
            TransportError::TransactionError(TransactionError::InstructionError(
                _,
                InstructionError::Custom(error_index),
            )) => {
                assert_eq!(error_index, SwapError::ExpectedMint as u32);
            }
            _ => {
                panic!("Wrong error occurs while trying uninitialized pool mint")
            }
        }
    }
}

async fn create_standard_setup(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
) -> helpers::TokenSwapAccounts {
    //create the registry
    let pool_registry_seed = "poolregistry";
    let pool_registry_key =
        Pubkey::create_with_seed(&payer.pubkey(), &pool_registry_seed, &id()).unwrap();
    helpers::create_pool_registry(
        banks_client,
        &payer,
        &recent_blockhash,
        &pool_registry_key,
        &payer,
    )
    .await
    .unwrap();

    let fees = Fees {
        trade_fee_numerator: 1,
        trade_fee_denominator: 2,
        owner_trade_fee_numerator: 1,
        owner_trade_fee_denominator: 10,
        owner_withdraw_fee_numerator: 1,
        owner_withdraw_fee_denominator: 5,
    };

    let swap_curve = SwapCurve {
        curve_type: CurveType::ConstantProduct,
        calculator: Box::new(ConstantProductCurve {}),
    };
    let token_a_amount = 1000;
    let token_b_amount = 2000;

    let swap = helpers::TokenSwapAccounts::new(
        banks_client,
        &payer,
        &recent_blockhash,
        pool_registry_key,
        fees,
        swap_curve,
        token_a_amount,
        token_b_amount,
    )
    .await;

    swap
}
