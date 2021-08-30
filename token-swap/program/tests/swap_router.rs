#![cfg(feature = "test-bpf")]

mod helpers;

use {
    solana_program_test::tokio,
    solana_sdk::{
        account::Account,
        instruction::InstructionError,
        signature::{Keypair, Signer},
        system_program,
        transaction::TransactionError,
        transport::TransportError,
    },
    spl_token_swap::error::SwapError,
};

const POOL_TOKEN_A_AMOUNT: u64 = 700_000_000_000_000;
const POOL_TOKEN_B_AMOUNT: u64 = 600_000_000_000_000;
const POOL_TOKEN_B2_AMOUNT: u64 = 300_000_000_000_000;
const POOL_TOKEN_C_AMOUNT: u64 = 400_000_000_000_000;
const USER_TOKEN_A_BAL: u64 = 200_000;
const USER_WILL_SWAP: u64 = 99_999;
const USER_WILL_EXPECT: u64 = 114_286;
//const USER_WILL_RECEIVE: u64 = 112_463;
//if 1/2 router discount used fees
const USER_WILL_RECEIVE: u64 = 113_694; //after 0.6% fee

#[tokio::test]
async fn fn_swap_router_create_b_c() {
    let user = Keypair::new();

    let mut pt = helpers::program_test();
    //throw our user account directly onto the chain startup
    pt.add_account(
        user.pubkey(),
        Account::new(100_000_000_000, 0, &system_program::id()),
    );
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let token_a_mint_key = Keypair::new();
    let token_b_mint_key = Keypair::new();
    let token_c_mint_key = Keypair::new();

    //lp1
    let mut swap1 = helpers::create_standard_setup(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        None,
        &token_a_mint_key,
        &token_b_mint_key,
        POOL_TOKEN_A_AMOUNT,
        POOL_TOKEN_B_AMOUNT,
    )
    .await;
    swap1
        .initialize_swap(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();

    //lp2
    let mut swap2 = helpers::create_standard_setup(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        //reuse same registry
        Some(swap1.pool_registry_pubkey.clone()),
        //use the same mint as the right side of swap1
        &token_b_mint_key,
        &token_c_mint_key,
        POOL_TOKEN_B2_AMOUNT,
        POOL_TOKEN_C_AMOUNT,
    )
    .await;
    swap2
        .initialize_swap(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();

    //our test swap will be
    //100,000 A in -> 85,714 B -> 114,286 C out (excluding fees)
    let amount_user_will_have: u64 = USER_TOKEN_A_BAL;
    let amount_user_will_swap: u64 = USER_WILL_SWAP;
    let mut amount_user_expects: u64 = USER_WILL_EXPECT;
    let amount_user_actually_gets: u64 = USER_WILL_RECEIVE; //after fees

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
        amount_user_will_have,
    )
    .await
    .unwrap();

    //swap without a high enough slippage
    {
        //swap ins
        let transaction_error = swap1
            .routed_swap(
                &mut banks_client,
                &user,
                &recent_blockhash,
                &swap2,
                &user_token_a.pubkey(),
                None,
                None,
                amount_user_will_swap,
                amount_user_expects,
            )
            .await
            .err()
            .unwrap();

        if let TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(err),
        )) = transaction_error
        {
            if err as u32 != SwapError::ExceededSlippage as u32 {
                panic!("Did not find the expected failure due to slippage (received other error)")
            }
        } else {
            panic!("Did not find the expected failure due to slippage")
        }
    }

    //allow it some slippage
    amount_user_expects = amount_user_expects 
        - (amount_user_expects as f32 * 0.016) as u64; //fees - 0.5% trade, 0.3% owner. * 2 for 2 pools
        -(amount_user_expects as f32 * 0.005) as u64; //0.5% slippage

    {
    swap1
        .routed_swap(
            &mut banks_client,
            &user,
            &recent_blockhash,
            &swap2,
            &user_token_a.pubkey(),
            None,
            None,
            amount_user_will_swap,
            amount_user_expects,
        )
        .await
        .unwrap();
    }


    //verify balances
    let user_token_c = spl_associated_token_account::get_associated_token_address(
        &user.pubkey(), 
        &token_c_mint_key.pubkey(),
    );

    let user_token_a_bal = helpers::get_token_balance(&mut banks_client, &user_token_a.pubkey()).await;
    assert_eq!(user_token_a_bal, amount_user_will_have - amount_user_will_swap);
    let user_token_c_bal = helpers::get_token_balance(&mut banks_client, &user_token_c).await;
    assert_eq!(user_token_c_bal, amount_user_actually_gets);

    //verify b account doesnt exist
    let user_token_b = spl_associated_token_account::get_associated_token_address(
        &user.pubkey(), 
        &token_b_mint_key.pubkey(),
    );
    let is = banks_client.get_account(user_token_b).await.unwrap();
    assert_eq!(is, None);
}

#[tokio::test]
async fn fn_swap_router_create_b() {
    let user = Keypair::new();

    let mut pt = helpers::program_test();
    //throw our user account directly onto the chain startup
    pt.add_account(
        user.pubkey(),
        Account::new(100_000_000_000, 0, &system_program::id()),
    );
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let token_a_mint_key = Keypair::new();
    let token_b_mint_key = Keypair::new();
    let token_c_mint_key = Keypair::new();
    
    //lp1
    let mut swap1 = helpers::create_standard_setup(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        None,
        &token_a_mint_key,
        &token_b_mint_key,
        POOL_TOKEN_A_AMOUNT,
        POOL_TOKEN_B_AMOUNT,
    )
    .await;
    swap1
        .initialize_swap(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();

    //lp2
    let mut swap2 = helpers::create_standard_setup(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        //reuse same registry
        Some(swap1.pool_registry_pubkey.clone()),
        //use the same mint as the right side of swap1
        &token_b_mint_key,
        &token_c_mint_key,
        POOL_TOKEN_B2_AMOUNT,
        POOL_TOKEN_C_AMOUNT,
    )
    .await;
    swap2
        .initialize_swap(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();

    //our test swap will be
    //100,000 A in -> 85,714 B -> 114,286 C out (excluding fees)
    let amount_user_will_have: u64 = USER_TOKEN_A_BAL;
    let amount_user_will_swap: u64 = USER_WILL_SWAP;
    let mut amount_user_expects: u64 = USER_WILL_EXPECT;
    let amount_user_actually_gets: u64 = USER_WILL_RECEIVE; //after fees

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
        amount_user_will_have,
    )
    .await
    .unwrap();

    //create token b account
    let user_token_b = Keypair::new();
    helpers::create_token_account(
        &mut banks_client,
        &user,
        &recent_blockhash,
        &user_token_b,
        &swap1.token_b_mint_key.pubkey(),
        &user.pubkey(),
    )
    .await
    .unwrap();

    //allow it some slippage
    amount_user_expects = amount_user_expects 
        - (amount_user_expects as f32 * 0.016) as u64; //fees - 0.5% trade, 0.3% owner. * 2 for 2 pools
        -(amount_user_expects as f32 * 0.005) as u64; //0.5% slippage

    {
    swap1
        .routed_swap(
            &mut banks_client,
            &user,
            &recent_blockhash,
            &swap2,
            &user_token_a.pubkey(),
            Some(&user_token_b.pubkey()),
            None,
            amount_user_will_swap,
            amount_user_expects,
        )
        .await
        .unwrap();
    }


    //verify balances
    let user_token_c = spl_associated_token_account::get_associated_token_address(
        &user.pubkey(), 
        &token_c_mint_key.pubkey(),
    );

    let user_token_a_bal = helpers::get_token_balance(&mut banks_client, &user_token_a.pubkey()).await;
    assert_eq!(user_token_a_bal, amount_user_will_have - amount_user_will_swap);
    let user_token_c_bal = helpers::get_token_balance(&mut banks_client, &user_token_c).await;
    assert_eq!(user_token_c_bal, amount_user_actually_gets);

    //verify b account doesnt exist anymore
    let user_token_b = spl_associated_token_account::get_associated_token_address(
        &user.pubkey(), 
        &token_b_mint_key.pubkey(),
    );
    let is = banks_client.get_account(user_token_b).await.unwrap();
    assert_eq!(is, None);
}

#[tokio::test]
async fn fn_swap_router_reuse_all() {
    let user = Keypair::new();

    let mut pt = helpers::program_test();
    //throw our user account directly onto the chain startup
    pt.add_account(
        user.pubkey(),
        Account::new(100_000_000_000, 0, &system_program::id()),
    );
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let token_a_mint_key = Keypair::new();
    let token_b_mint_key = Keypair::new();
    let token_c_mint_key = Keypair::new();

    //lp1
    let mut swap1 = helpers::create_standard_setup(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        None,
        &token_a_mint_key,
        &token_b_mint_key,
        POOL_TOKEN_A_AMOUNT,
        POOL_TOKEN_B_AMOUNT,
    )
    .await;
    swap1
        .initialize_swap(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();

    //lp2
    let mut swap2 = helpers::create_standard_setup(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        //reuse same registry
        Some(swap1.pool_registry_pubkey.clone()),
        //use the same mint as the right side of swap1
        &token_b_mint_key,
        &token_c_mint_key,
        POOL_TOKEN_B2_AMOUNT,
        POOL_TOKEN_C_AMOUNT,
    )
    .await;
    swap2
        .initialize_swap(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();
        
    //our test swap will be
    //100,000 A in -> 85,714 B -> 114,286 C out (excluding fees)
    let amount_user_had_token_b: u64 = 999_234_432;
    let amount_user_had_token_c: u64 = 123_345_345;
    let amount_user_will_have: u64 = USER_TOKEN_A_BAL;
    let amount_user_will_swap: u64 = USER_WILL_SWAP;
    let mut amount_user_expects: u64 = USER_WILL_EXPECT;
    let amount_user_actually_gets: u64 = USER_WILL_RECEIVE; //after fees

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
        amount_user_will_have,
    )
    .await
    .unwrap();

    //create token b account
    let user_token_b = Keypair::new();
    helpers::create_token_account(
        &mut banks_client,
        &user,
        &recent_blockhash,
        &user_token_b,
        &swap1.token_b_mint_key.pubkey(),
        &user.pubkey(),
    )
    .await
    .unwrap();
    //mint tokens to the users account using payer
    helpers::mint_tokens(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &swap1.token_b_mint_key.pubkey(),
        &user_token_b.pubkey(),
        &payer,
        amount_user_had_token_b,
    )
    .await
    .unwrap();

    //create token c account
    let user_token_c = Keypair::new();
    helpers::create_token_account(
        &mut banks_client,
        &user,
        &recent_blockhash,
        &user_token_c,
        &swap2.token_b_mint_key.pubkey(),
        &user.pubkey(),
    )
    .await
    .unwrap();
    //mint tokens to the users account using payer
    helpers::mint_tokens(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &swap2.token_b_mint_key.pubkey(),
        &user_token_c.pubkey(),
        &payer,
        amount_user_had_token_c,
    )
    .await
    .unwrap();

    //allow it some slippage
    amount_user_expects = amount_user_expects 
        - (amount_user_expects as f32 * 0.016) as u64; //fees - 0.5% trade, 0.3% owner. * 2 for 2 pools
        -(amount_user_expects as f32 * 0.005) as u64; //0.5% slippage

    {
    swap1
        .routed_swap(
            &mut banks_client,
            &user,
            &recent_blockhash,
            &swap2,
            &user_token_a.pubkey(),
            Some(&user_token_b.pubkey()),
            Some(&user_token_c.pubkey()),
            amount_user_will_swap,
            amount_user_expects,
        )
        .await
        .unwrap();
    }


    //assert that unswapped amount remains
    let user_token_a_bal = helpers::get_token_balance(&mut banks_client, &user_token_a.pubkey()).await;
    assert_eq!(user_token_a_bal, amount_user_will_have - amount_user_will_swap);


    //assert that prior balances remain in place
    let user_token_b_bal = helpers::get_token_balance(&mut banks_client, &user_token_b.pubkey()).await;
    assert_eq!(user_token_b_bal, amount_user_had_token_b);

    let user_token_c_bal = helpers::get_token_balance(&mut banks_client, &user_token_c.pubkey()).await;
    assert_eq!(user_token_c_bal, amount_user_had_token_c + amount_user_actually_gets);
}