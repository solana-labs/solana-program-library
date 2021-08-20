//started writing these to mimic the unit tests - but found that simply mocking the init 
//in the unit tests followed the pattern there and allowed all other existing tests to function

#![cfg(feature = "test-bpf")]

mod helpers;

use {
    solana_program_test::{tokio},
    solana_sdk::{
        instruction::InstructionError,
        signature::{Keypair, Signer},
        transaction::TransactionError,
        transport::TransportError,
        account::Account,
        system_program,
    },
    spl_token_swap::{
        curve::{
            calculator::INITIAL_SWAP_POOL_AMOUNT,
        },
    },
    std::convert::TryInto,
};

#[tokio::test]
async fn fn_test_deposit_swap_not_init() {
    let depositor = Keypair::new();

    let mut pt = helpers::program_test();
    //throw our depositor account directly onto the chain startup
    pt.add_account(
        depositor.pubkey(), 
        Account::new(100_000_000, 0, &system_program::id())
    );
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let token_a_amount = 1000;
    let token_b_amount = 9000;
    let mut swap = helpers::create_standard_setup(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        token_a_amount,
        token_b_amount,
    )
    .await;

    // depositing 10% of the current pool amount in token A and B means
    // that our pool tokens will be worth 1 / 10 of the current pool amount
    let pool_amount = INITIAL_SWAP_POOL_AMOUNT / 10;
    let deposit_a = token_a_amount / 10;
    let deposit_b = token_b_amount / 10;

    let token_account_a = Keypair::new();
    let token_account_b = Keypair::new();
    let token_account_pool = Keypair::new();

    helpers::create_depositor(
        &mut banks_client, 
        &payer, 
        &recent_blockhash,
        &depositor,
        &token_account_a,
        &token_account_b,
        &token_account_pool,
        &swap.token_a_mint_key.pubkey(),
        &swap.token_b_mint_key.pubkey(),
        &swap.pool_mint_key.pubkey(),
        deposit_a,
        deposit_b,
    ).await;

    let transaction_error = swap
        .deposit_all_token_types(
            &mut banks_client, 
            &depositor, 
            &recent_blockhash,
            &depositor,
            &token_account_a.pubkey(),
            &token_account_b.pubkey(),
            &token_account_pool.pubkey(),
            pool_amount.try_into().unwrap(),
            deposit_a,
            deposit_b,
        )
        .await
        .err()
        .unwrap();
    if let TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::InvalidAccountData, 
        )) = transaction_error { }
    else {
        panic!("Wrong error occurs while depositing into uninitialized swap")
    }
    
}
