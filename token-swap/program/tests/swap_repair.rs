#![cfg(any(test, feature = "test-bpf"))]

mod helpers;

use spl_token_swap::state::SwapVersion;
use {
    solana_program_test::tokio,
    solana_sdk::{
        pubkey::Pubkey,
        account::Account,
        signature::{Keypair, Signer},
        transaction::TransactionError,
        instruction::InstructionError,
        system_program,
    },
    spl_token_swap::error::SwapError,
};

const POOL_TOKEN_A_AMOUNT: u64 = 700_000_000;
const POOL_TOKEN_B_AMOUNT: u64 = 600_000_000;
const POOL_TOKEN_B2_AMOUNT: u64 = 300_000_000;
const POOL_TOKEN_C_AMOUNT: u64 = 400_000_000;
const USER_TOKEN_A_BAL: u64 = 20_00_000;
const USER_WILL_SWAP: u64 = 100_000;
//const USER_WILL_EXPECT: u64 = 114_286;
//const USER_WILL_RECEIVE: u64 = 113_646; 

/// For unit testing, we need to use a owner key when generating ATAs
pub const TEST_OWNER_KEY: &str = "5Cebzty8iwgAUx9jyfZVAT2iMvXBECLwEVgT6T8KYmvS";

#[tokio::test]
async fn fn_swap_repair() {
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

    //grab the atas for later use
    let user_token_c = spl_associated_token_account::get_associated_token_address(
        &user.pubkey(), 
        &token_c_mint_key.pubkey(),
    );

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
        USER_TOKEN_A_BAL,
    )
    .await
    .unwrap();

    //simple swap should work
    swap1
        .routed_swap(
            &mut banks_client,
            &user,
            &recent_blockhash,
            &swap2,
            &user_token_a.pubkey(),
            None,
            None,
            USER_WILL_SWAP,
            0, //not testing exacts, just testing fee paying
        )
        .await
        .unwrap();

    let fee_bal = helpers::get_token_balance(&mut banks_client, &swap1.pool_fee_key.pubkey()).await;
    assert!(fee_bal > 0);

    //empty the fee account
    helpers::burn_tokens(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &swap1.pool_mint_key.pubkey(),
        &swap1.pool_fee_key.pubkey(),
        &payer,
        fee_bal,
    )
    .await
    .unwrap();

    //pull a G and close the fee token account
    helpers::close_token_account(
        &mut banks_client, 
        &payer, 
        &recent_blockhash, 
        &swap1.pool_fee_key.pubkey(), 
        &payer,
    )
    .await
    .unwrap();

    //simple swap should still work, because we now ignore missing fee accounts
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
            USER_WILL_SWAP,
            0, //not testing exacts, just testing fee paying
        )
        .await
        .unwrap();
    }

    //assert the fee account still doesnt exist, of course, because G closed it
    assert!(banks_client.get_account(swap1.pool_fee_key.pubkey()).await.unwrap().is_none());
    
    //create the ata for the fee address
    let ata = helpers::create_associated_token_account(
        &mut banks_client, 
        &payer, 
        &recent_blockhash, 
        &TEST_OWNER_KEY.parse::<Pubkey>().unwrap(),
        &swap1.pool_mint_key.pubkey(),
    )
    .await
    .unwrap();

    //repair the swap_pool
    swap1.repair(
        &mut banks_client,
        &user,
        &recent_blockhash,
        &ata,
    )
    .await
    .unwrap();

    //verify the token swap account was updated with the new key
    let swap_account = banks_client.get_account(swap1.swap_pubkey).await.unwrap().unwrap();
    let swap_account = SwapVersion::unpack(swap_account.data.as_ref()).unwrap();
    assert!(swap_account.pool_fee_account() == &ata);

    //verify balance of new fee token account is 0 (why the hell it wouldn't be I don't know)
    let fee_bal = helpers::get_token_balance(&mut banks_client, &ata).await;
    assert!(fee_bal == 0);

    //simple swap should now pay fee account
    {
        swap1
            .routed_swap(
                &mut banks_client,
                &user,
                &recent_blockhash,
                &swap2,
                &user_token_a.pubkey(),
                None,
                Some(&user_token_c),
                USER_WILL_SWAP,
                0, //not testing exacts, just testing fee paying
            )
            .await
            .unwrap();
    }

    //verify that fees work again!
    let fee_bal = helpers::get_token_balance(&mut banks_client, &ata).await;
    assert!(fee_bal > 0);
}

#[tokio::test]
async fn fn_swap_repair_failures() {
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
        USER_TOKEN_A_BAL,
    )
    .await
    .unwrap();

    //simple swap should work
    swap1
        .routed_swap(
            &mut banks_client,
            &user,
            &recent_blockhash,
            &swap2,
            &user_token_a.pubkey(),
            None,
            None,
            USER_WILL_SWAP,
            0, //not testing exacts, just testing fee paying
        )
        .await
        .unwrap();

    let fee_bal = helpers::get_token_balance(&mut banks_client, &swap1.pool_fee_key.pubkey()).await;
    assert!(fee_bal > 0);

    //empty the fee account
    helpers::burn_tokens(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &swap1.pool_mint_key.pubkey(),
        &swap1.pool_fee_key.pubkey(),
        &payer,
        fee_bal,
    )
    .await
    .unwrap();

    let owner = TEST_OWNER_KEY.parse::<Pubkey>().unwrap();

    //create a random user account and try to make it the fee address while the old address exists
    let k = Keypair::new();
    helpers::create_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &k,
        &swap1.pool_mint_key.pubkey(),
        &user.pubkey(),
    )
    .await
    .unwrap();
    //fail to repair the swap_pool 
    {
        let res = swap1.repair(
            &mut banks_client,
            &user,
            &recent_blockhash,
            &k.pubkey(),
        )
            .await
            .unwrap_err()
            .unwrap();

        //invalidinput because old fee address exists
        assert_eq!(
            TransactionError::InstructionError(0, InstructionError::Custom(SwapError::InvalidInput as u32)),
            res);
    }

    //create an owner account but not ata and try to make the fee address while the old address exists
    let k = Keypair::new();
    helpers::create_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &k,
        &swap1.pool_mint_key.pubkey(),
        &owner,
    )
    .await
    .unwrap();
    //fail to repair the swap_pool 
    {
        let res = swap1.repair(
            &mut banks_client,
            &user,
            &recent_blockhash,
            &k.pubkey(),
        )
            .await
            .unwrap_err()
            .unwrap();
            
        //invalidinput because old fee address exists
        assert_eq!(
            TransactionError::InstructionError(0, InstructionError::Custom(SwapError::InvalidInput as u32)),
            res);
    }

    //pull a G and close the fee token account
    helpers::close_token_account(
        &mut banks_client, 
        &payer, 
        &recent_blockhash, 
        &swap1.pool_fee_key.pubkey(), 
        &payer,
    )
    .await
    .unwrap();

    //simple swap should still work, because we now ignore missing fee accounts
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
            USER_WILL_SWAP,
            0, //not testing exacts, just testing fee paying
        )
        .await
        .unwrap();
    }

    //assert the fee account still doesnt exist, of course, because G closed it
    assert!(banks_client.get_account(swap1.pool_fee_key.pubkey()).await.unwrap().is_none());

    //create a random user account and try to make it the fee address
    let k = Keypair::new();
    helpers::create_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &k,
        &swap1.pool_mint_key.pubkey(),
        &user.pubkey(),
    )
    .await
    .unwrap();
    //fail to repair the swap_pool 
    {
        let res = swap1.repair(
            &mut banks_client,
            &user,
            &recent_blockhash,
            &k.pubkey(),
        )
            .await
            .unwrap_err()
            .unwrap();

        assert_eq!(
            TransactionError::InstructionError(0, InstructionError::Custom(SwapError::InvalidOwner as u32)),
            res);
    }

    //create an owner account but not ata and try to make the fee address
    let k = Keypair::new();
    helpers::create_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &k,
        &swap1.pool_mint_key.pubkey(),
        &owner,
    )
    .await
    .unwrap();
    //fail to repair the swap_pool 
    {
        let res = swap1.repair(
            &mut banks_client,
            &user,
            &recent_blockhash,
            &k.pubkey(),
        )
            .await
            .unwrap_err()
            .unwrap();
            
        assert_eq!(
            TransactionError::InstructionError(0, InstructionError::Custom(SwapError::IncorrectFeeAccount as u32)),
            res);
    }
    
    //create the ata for the fee address
    let ata = helpers::create_associated_token_account(
        &mut banks_client, 
        &payer, 
        &recent_blockhash, 
        &TEST_OWNER_KEY.parse::<Pubkey>().unwrap(),
        &swap1.pool_mint_key.pubkey(),
    )
    .await
    .unwrap();

    //pass the valid ata, but a old fee account that is empty but doesn't match whats recorded on swap acct
    let k = Keypair::new();
    //fail to repair the swap_pool 
    {
        let res = swap1.repair_override_old_fee(
            &mut banks_client,
            &user,
            &recent_blockhash,
            &ata,
            Some(k.pubkey()),
        )
            .await
            .unwrap_err()
            .unwrap();
            
        assert_eq!(
            TransactionError::InstructionError(0, InstructionError::Custom(SwapError::IncorrectFeeAccount as u32)),
            res);
    }

    //repair the swap_pool properly
    swap1.repair(
        &mut banks_client,
        &user,
        &recent_blockhash,
        &ata,
    )
    .await
    .unwrap();

    //verify the token swap account was updated with the new key
    let swap_account = banks_client.get_account(swap1.swap_pubkey).await.unwrap().unwrap();
    let swap_account = SwapVersion::unpack(swap_account.data.as_ref()).unwrap();
    assert!(swap_account.pool_fee_account() == &ata);
}
