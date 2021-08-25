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
async fn fn_dual_swap() {
    let user = Keypair::new();

    let mut pt = helpers::program_test();
    //throw our user account directly onto the chain startup
    pt.add_account(
        user.pubkey(), 
        Account::new(100_000_000_000, 0, &system_program::id())
    );
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let token_a_mint_key = Keypair::new();
    let token_b_mint_key = Keypair::new();
    let token_c_mint_key = Keypair::new();

    //lp1
    let token_a_amount = 600_000_000_000_000;
    let token_b_amount = 500_000_000_000_000;

    let mut swap1 = helpers::create_standard_setup(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        None,
        &token_a_mint_key,
        &token_b_mint_key,
        token_a_amount,
        token_b_amount,
    )
    .await;
    swap1.initialize_swap(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();

    //lp2
    let token_b2_amount = 400_000_000_000_000;
    let token_c_amount = 300_000_000_000_000;

    let mut swap2 = helpers::create_standard_setup(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        //reuse same registry
        Some(swap1.pool_registry_pubkey.clone()),
        //use the same mint as the right side of swap1
        &token_b_mint_key,
        &token_c_mint_key,
        token_b2_amount,
        token_c_amount,
    )
    .await;
    swap2.initialize_swap(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();

    let amount_user_will_swap: u64 = 1_000_000_000;
    let amount_user_expects: u64 = 1_000_000_000;

    //setup our users token account, owned and paid for by user
    let user_token_a = Keypair::new();
    helpers::create_token_account(
        &mut banks_client,
        &user,
        &recent_blockhash,
        &user_token_a,
        &swap1.token_a_mint_key.pubkey(),
        &user.pubkey(),
    )
    .await
    .unwrap();
    //mint tokens to the users account using payer
    helpers::mint_tokens(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &swap1.token_a_mint_key.pubkey(),
        &user_token_a.pubkey(),
        &payer,
        amount_user_will_swap,
    )
    .await
    .unwrap();

    
    /*** now our user does a swap */
    //our test swap will be
    //100,000 A in -> 180,000 B -> 77,142 C out
    swap1.routed_swap(
        &mut banks_client,
        &user,
        &recent_blockhash,
        &swap2,
        &user_token_a.pubkey(),
        None,
        None,
        amount_user_will_swap,
        0,
    )
    .await
    .unwrap();


    // depositing 10% of the current pool amount in token A and B means
    // that our pool tokens will be worth 1 / 10 of the current pool amount
    // let pool_amount = INITIAL_SWAP_POOL_AMOUNT / 10;
    // let deposit_a = token_a_amount / 10;
    // let deposit_b = token_b_amount / 10;

    // let token_account_a = Keypair::new();
    // let token_account_b = Keypair::new();
    // let token_account_pool = Keypair::new();

    // helpers::create_depositor(
    //     &mut banks_client, 
    //     &payer, 
    //     &recent_blockhash,
    //     &depositor,
    //     &token_account_a,
    //     &token_account_b,
    //     &token_account_pool,
    //     &swap.token_a_mint_key.pubkey(),
    //     &swap.token_b_mint_key.pubkey(),
    //     &swap.pool_mint_key.pubkey(),
    //     deposit_a,
    //     deposit_b,
    // ).await;

    // let transaction_error = swap
    //     .deposit_all_token_types(
    //         &mut banks_client, 
    //         &depositor, 
    //         &recent_blockhash,
    //         &depositor,
    //         &token_account_a.pubkey(),
    //         &token_account_b.pubkey(),
    //         &token_account_pool.pubkey(),
    //         pool_amount.try_into().unwrap(),
    //         deposit_a,
    //         deposit_b,
    //     )
    //     .await
    //     .err()
    //     .unwrap();
    // if let TransportError::TransactionError(TransactionError::InstructionError(
    //         _,
    //         InstructionError::InvalidAccountData, 
    //     )) = transaction_error { }
    // else {
    //     panic!("Wrong error occurs while depositing into uninitialized swap")
    // }
    
}
