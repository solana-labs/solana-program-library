#![cfg(feature = "test-bpf")]

mod helpers;

use {
    bytemuck::from_bytes,
    solana_program::{program_pack::Pack, pubkey::Pubkey},
    solana_program_test::{tokio, ProgramTestBanksClientExt},
    solana_sdk::{
        account::ReadableAccount,
        instruction::InstructionError,
        signature::{Keypair, Signer},
        transaction::{Transaction, TransactionError},
        transport::TransportError,
    },
    spl_token_swap::{
        curve::{
            base::{CurveType, SwapCurve},
            constant_price::ConstantPriceCurve,
            fees::Fees,
            offset::OffsetCurve,
        },
        error::SwapError,
        id, instruction,
        state::{PoolRegistry, SwapVersion},
    },
    std::sync::Arc,
};

#[tokio::test]
async fn fn_test_initialize_fail_wrong_swap_acct() {
    let (mut banks_client, payer, recent_blockhash) = helpers::program_test().start().await;

    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    let mut swap = helpers::create_standard_setup(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        None,
        &mint_a,
        &mint_b,
        1000,
        2000,
    )
    .await;

    // wrong pda for swap account
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

#[tokio::test]
async fn fn_test_initialize_fail_wrong_nonce() {
    let (mut banks_client, payer, recent_blockhash) = helpers::program_test().start().await;

    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    let mut swap = helpers::create_standard_setup(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        None,
        &mint_a,
        &mint_b,
        1000,
        2000,
    )
    .await;

    // wrong nonce for authority_key
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

#[tokio::test]
async fn fn_test_initialize_fail_uninit_token_a() {
    let (mut banks_client, payer, recent_blockhash) = helpers::program_test().start().await;

    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    let mut swap = helpers::create_standard_setup(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        None,
        &mint_a,
        &mint_b,
        1000,
        2000,
    )
    .await;

    // uninitialized token a account
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

#[tokio::test]
async fn fn_test_initialize_fail_uninit_token_b() {
    let (mut banks_client, payer, recent_blockhash) = helpers::program_test().start().await;

    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    let mut swap = helpers::create_standard_setup(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        None,
        &mint_a,
        &mint_b,
        1000,
        2000,
    )
    .await;

    // uninitialized token b account
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

#[tokio::test]
async fn fn_test_initialize_fail_uninit_pool_mint() {
    let (mut banks_client, payer, recent_blockhash) = helpers::program_test().start().await;

    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    let mut swap = helpers::create_standard_setup(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        None,
        &mint_a,
        &mint_b,
        1000,
        2000,
    )
    .await;

    // uninitialized pool mint
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

#[tokio::test]
async fn fn_test_initialize_fail_token_a_wrong_owner() {
    let (mut banks_client, payer, recent_blockhash) = helpers::program_test().start().await;

    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    let mut swap = helpers::create_standard_setup(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        None,
        &mint_a,
        &mint_b,
        1000,
        2000,
    )
    .await;

    // token A account owner is not swap authority
    let new_account = Keypair::new();

    helpers::create_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &new_account,
        &swap.token_a_mint_key.pubkey(),
        &new_account.pubkey(), //new token account managed by random
    )
    .await
    .unwrap();
    helpers::mint_tokens(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &swap.token_a_mint_key.pubkey(),
        &new_account.pubkey(),
        &payer,
        1000,
    )
    .await
    .unwrap();

    swap.token_a_key = new_account;

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
            assert_eq!(error_index, SwapError::InvalidOwner as u32);
        }
        _ => {
            panic!("Wrong error occurs while trying token A account owner is not swap authority")
        }
    }
}

#[tokio::test]
async fn fn_test_initialize_fail_token_b_wrong_owner() {
    let (mut banks_client, payer, recent_blockhash) = helpers::program_test().start().await;

    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    let mut swap = helpers::create_standard_setup(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        None,
        &mint_a,
        &mint_b,
        1000,
        2000,
    )
    .await;

    // token B account owner is not swap authority
    let new_account = Keypair::new();

    helpers::create_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &new_account,
        &swap.token_b_mint_key.pubkey(),
        &new_account.pubkey(), //new token account managed by random
    )
    .await
    .unwrap();
    helpers::mint_tokens(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &swap.token_b_mint_key.pubkey(),
        &new_account.pubkey(),
        &payer,
        1000,
    )
    .await
    .unwrap();

    swap.token_b_key = new_account;

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
            assert_eq!(error_index, SwapError::InvalidOwner as u32);
        }
        _ => {
            panic!("Wrong error occurs while trying token B account owner is not swap authority")
        }
    }
}

#[tokio::test]
async fn fn_test_initialize_fail_pool_token_owner_is_swap_authority() {
    let (mut banks_client, payer, recent_blockhash) = helpers::program_test().start().await;

    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    let mut swap = helpers::create_standard_setup(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        None,
        &mint_a,
        &mint_b,
        1000,
        2000,
    )
    .await;

    // pool token account owner is swap authority
    //change the pool token account owner
    helpers::change_token_owner(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &swap.pool_token_key.pubkey(),
        &payer,
        &swap.authority_pubkey,
    )
    .await
    .unwrap();

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
            assert_eq!(error_index, SwapError::InvalidOutputOwner as u32);
        }
        _ => {
            panic!("Wrong error occurs while trying pool token account owner is swap authority")
        }
    }
}

#[tokio::test]
async fn fn_test_initialize_fail_pool_mint_auth_is_not_swap_authority() {
    let (mut banks_client, payer, recent_blockhash) = helpers::program_test().start().await;

    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    let mut swap = helpers::create_standard_setup(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        None,
        &mint_a,
        &mint_b,
        1000,
        2000,
    )
    .await;

    // pool mint authority is not swap authority

    let new_account = Keypair::new();
    let new_account2 = Keypair::new();

    //recreate the pool mint
    helpers::create_mint(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &new_account,
        &new_account2.pubkey(),
        None,
    )
    .await
    .unwrap();

    swap.pool_mint_key = new_account;

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
            assert_eq!(error_index, SwapError::InvalidOwner as u32);
        }
        _ => {
            panic!("Wrong error occurs while trying pool mint authority is not swap authority")
        }
    }
}

#[tokio::test]
async fn fn_test_initialize_fail_pool_mint_auth_has_freeze_authority() {
    let (mut banks_client, payer, recent_blockhash) = helpers::program_test().start().await;

    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    let mut swap = helpers::create_standard_setup(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        None,
        &mint_a,
        &mint_b,
        1000,
        2000,
    )
    .await;

    // pool mint token has freeze authority

    let new_account = Keypair::new();

    //recreate the pool mint with a freeze auth
    helpers::create_mint(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &new_account,
        &swap.authority_pubkey,
        Some(&new_account.pubkey()),
    )
    .await
    .unwrap();

    swap.pool_mint_key = new_account;

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
            assert_eq!(error_index, SwapError::InvalidFreezeAuthority as u32);
        }
        _ => {
            panic!("Wrong error occurs while trying pool mint token has freeze authority")
        }
    }
}

//at this point I'm realizing - all the above sub-unit tests of init
//still work as unit tests (arguably better).
//I just need to hit the ones that dont:

#[tokio::test]
async fn fn_test_initialize_pass() {
    let (mut banks_client, payer, recent_blockhash) = helpers::program_test().start().await;

    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    let mut swap = helpers::create_standard_setup(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        None,
        &mint_a,
        &mint_b,
        1000,
        2000,
    )
    .await;

    // create valid swap
    swap.initialize_swap(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();

    //validate initialize
    let swap_account = banks_client
        .get_account(swap.swap_pubkey)
        .await
        .unwrap()
        .unwrap();
    let swap_state = SwapVersion::unpack(&swap_account.data).unwrap();
    assert!(swap_state.is_initialized());
    assert_eq!(swap_state.nonce(), swap.nonce);
    assert_eq!(
        swap_state.swap_curve().curve_type,
        swap.swap_curve.curve_type
    );
    assert_eq!(*swap_state.token_a_account(), swap.token_a_key.pubkey());
    assert_eq!(*swap_state.token_b_account(), swap.token_b_key.pubkey());
    assert_eq!(*swap_state.pool_mint(), swap.pool_mint_key.pubkey());
    assert_eq!(*swap_state.token_a_mint(), swap.token_a_mint_key.pubkey());
    assert_eq!(*swap_state.token_b_mint(), swap.token_b_mint_key.pubkey());
    assert_eq!(*swap_state.pool_fee_account(), swap.pool_fee_key.pubkey());
    //assert_eq!(*swap_state.pool_registry_pubkey(), swap.pool_registry_pubkey);
    //assert_eq!(*swap_state.pool_nonce(), swap.pool_nonce);

    let token_a_acct = banks_client
        .get_account(swap.token_a_key.pubkey())
        .await
        .unwrap()
        .unwrap();
    let token_a = spl_token::state::Account::unpack(&token_a_acct.data).unwrap();
    assert_eq!(token_a.amount, 1000);

    let token_b_acct = banks_client
        .get_account(swap.token_b_key.pubkey())
        .await
        .unwrap()
        .unwrap();
    let token_b = spl_token::state::Account::unpack(&token_b_acct.data).unwrap();
    assert_eq!(token_b.amount, 2000);

    let pool_account_acct = banks_client
        .get_account(swap.pool_token_key.pubkey())
        .await
        .unwrap()
        .unwrap();
    let pool_account = spl_token::state::Account::unpack(&pool_account_acct.data).unwrap();
    let pool_mint_acct = banks_client
        .get_account(swap.pool_mint_key.pubkey())
        .await
        .unwrap()
        .unwrap();
    let pool_mint = spl_token::state::Mint::unpack(&pool_mint_acct.data).unwrap();
    assert_eq!(pool_mint.supply, pool_account.amount);
}

#[tokio::test]
async fn fn_test_initialize_twice() {
    let (mut banks_client, payer, recent_blockhash) = helpers::program_test().start().await;

    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    let mut swap = helpers::create_standard_setup(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        None,
        &mint_a,
        &mint_b,
        1000,
        2000,
    )
    .await;

    // create valid swap
    swap.initialize_swap(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();

    //grab a new blockhash from the chain
    let (recent_blockhash, _calc) = banks_client
        .get_new_blockhash(&recent_blockhash)
        .await
        .unwrap();

    //try create again
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
            assert_eq!(error_index, SwapError::AlreadyInUse as u32);
        }
        _ => {
            panic!("Wrong error occurs while creating swap twice")
        }
    }
}

#[tokio::test]
async fn fn_test_pool_reg_removes() {
    let (mut banks_client, payer, recent_blockhash) = helpers::program_test().start().await;

    let mints: Vec<Keypair> = (0..6).map(|_| Keypair::new()).collect();

    let mut swap = helpers::create_standard_setup(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        None,
        &mints.get(0).unwrap(),
        &mints.get(1).unwrap(),
        1000,
        2000,
    )
    .await;
    let fixed_pool_reg = swap.pool_registry_pubkey;

    // create valid swap
    swap.initialize_swap(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();

    //grab a new blockhash from the chain
    let (recent_blockhash, _calc) = banks_client
        .get_new_blockhash(&recent_blockhash)
        .await
        .unwrap();

    let mut swap = helpers::create_standard_setup(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        Some(fixed_pool_reg),
        &mints.get(2).unwrap(),
        &mints.get(3).unwrap(),
        1000,
        2000,
    )
    .await;

    // create valid swap
    swap.initialize_swap(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();
    let mid_key = swap.swap_pubkey;

    //grab a new blockhash from the chain
    let (recent_blockhash, _calc) = banks_client
        .get_new_blockhash(&recent_blockhash)
        .await
        .unwrap();

    let mut swap = helpers::create_standard_setup(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        Some(fixed_pool_reg),
        &mints.get(4).unwrap(),
        &mints.get(5).unwrap(),
        1000,
        2000,
    )
    .await;

    // create valid swap
    swap.initialize_swap(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();
    let last_key = swap.swap_pubkey;

    //validate middle account in registry is middle we created
    let reg_account = banks_client
        .get_account(fixed_pool_reg)
        .await
        .unwrap()
        .unwrap();
    let data = reg_account.data();
    let reg: &PoolRegistry = from_bytes(data);
    let regsize_ref = std::ptr::addr_of!(reg.registry_size);
    let registry_size = unsafe { regsize_ref.read_unaligned() };
    assert_eq!(registry_size, 3);
    assert_eq!(reg.accounts[1], mid_key);
    assert_eq!(reg.accounts[2], last_key);

    //grab a new blockhash from the chain
    let (recent_blockhash, _calc) = banks_client
        .get_new_blockhash(&recent_blockhash)
        .await
        .unwrap();

    //remove index 1 (middle swap pool in reg)
    let mut tx = Transaction::new_with_payer(
        &[instruction::deregister_pool(&id(), &payer.pubkey(), &fixed_pool_reg, 1).unwrap()],
        Some(&payer.pubkey()),
    );
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    //validate middle account in registry is now the last we created
    let reg_account = banks_client
        .get_account(fixed_pool_reg)
        .await
        .unwrap()
        .unwrap();
    let data = reg_account.data();
    let reg: &PoolRegistry = from_bytes(data);
    let regsize_ref = std::ptr::addr_of!(reg.registry_size);
    let registry_size = unsafe { regsize_ref.read_unaligned() };
    assert_eq!(registry_size, 2);
    assert_eq!(reg.accounts[1], last_key);

    //grab a new blockhash from the chain
    let (recent_blockhash, _calc) = banks_client
        .get_new_blockhash(&recent_blockhash)
        .await
        .unwrap();

    //remove index 1 (last swap pool in reg)
    let mut tx = Transaction::new_with_payer(
        &[instruction::deregister_pool(&id(), &payer.pubkey(), &fixed_pool_reg, 1).unwrap()],
        Some(&payer.pubkey()),
    );
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    //validate registry is now size 1, and the 1 is not mid or last
    let reg_account = banks_client
        .get_account(fixed_pool_reg)
        .await
        .unwrap()
        .unwrap();
    let data = reg_account.data();
    let reg: &PoolRegistry = from_bytes(data);
    let regsize_ref = std::ptr::addr_of!(reg.registry_size);
    let registry_size = unsafe { regsize_ref.read_unaligned() };
    assert_eq!(registry_size, 1);
    assert!(reg.accounts[0] != mid_key);
    assert!(reg.accounts[0] != last_key);
}

#[tokio::test]
async fn fn_test_initialize_invalid_flat_swap() {
    let (mut banks_client, payer, recent_blockhash) = helpers::program_test().start().await;

    let pool_registry_key =
        helpers::create_pool_registry(&mut banks_client, &payer, &recent_blockhash, &payer)
            .await
            .unwrap();

    let token_b_price = 0;
    let fees = Fees {
        trade_fee_numerator: 1,
        trade_fee_denominator: 2,
        owner_trade_fee_numerator: 1,
        owner_trade_fee_denominator: 10,
        owner_withdraw_fee_numerator: 1,
        owner_withdraw_fee_denominator: 5,
    };

    let swap_curve = SwapCurve {
        curve_type: CurveType::ConstantPrice,
        calculator: Arc::new(ConstantPriceCurve { token_b_price }),
    };
    let token_a_amount = 1000;
    let token_b_amount = 2000;

    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    let mut swap: helpers::TokenSwapAccounts = helpers::TokenSwapAccounts::new(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        pool_registry_key,
        fees,
        swap_curve,
        &mint_a,
        &mint_b,
        token_a_amount,
        token_b_amount,
    )
    .await;
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
            assert_eq!(error_index, SwapError::InvalidCurve as u32);
        }
        _ => {
            panic!("Wrong error occurs while trying invalid flat swap")
        }
    }
}

#[tokio::test]
async fn fn_test_initialize_valid_flat_swap() {
    let (mut banks_client, payer, recent_blockhash) = helpers::program_test().start().await;

    let pool_registry_key =
        helpers::create_pool_registry(&mut banks_client, &payer, &recent_blockhash, &payer)
            .await
            .unwrap();

    let token_b_price = 10_000;
    let fees = Fees {
        trade_fee_numerator: 1,
        trade_fee_denominator: 2,
        owner_trade_fee_numerator: 1,
        owner_trade_fee_denominator: 10,
        owner_withdraw_fee_numerator: 1,
        owner_withdraw_fee_denominator: 5,
    };

    let swap_curve = SwapCurve {
        curve_type: CurveType::ConstantPrice,
        calculator: Arc::new(ConstantPriceCurve { token_b_price }),
    };
    let token_a_amount = 1000;
    let token_b_amount = 2000;

    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    let mut swap: helpers::TokenSwapAccounts = helpers::TokenSwapAccounts::new(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        pool_registry_key,
        fees,
        swap_curve,
        &mint_a,
        &mint_b,
        token_a_amount,
        token_b_amount,
    )
    .await;
    swap.initialize_swap(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();
}

#[tokio::test]
async fn fn_test_initialize_invalid_offset_swap() {
    let (mut banks_client, payer, recent_blockhash) = helpers::program_test().start().await;

    let pool_registry_key =
        helpers::create_pool_registry(&mut banks_client, &payer, &recent_blockhash, &payer)
            .await
            .unwrap();

    let token_b_offset = 0;
    let fees = Fees {
        trade_fee_numerator: 1,
        trade_fee_denominator: 2,
        owner_trade_fee_numerator: 1,
        owner_trade_fee_denominator: 10,
        owner_withdraw_fee_numerator: 1,
        owner_withdraw_fee_denominator: 5,
    };

    let swap_curve = SwapCurve {
        curve_type: CurveType::Offset,
        calculator: Arc::new(OffsetCurve { token_b_offset }),
    };
    let token_a_amount = 1000;
    let token_b_amount = 2000;

    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    let mut swap: helpers::TokenSwapAccounts = helpers::TokenSwapAccounts::new(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        pool_registry_key,
        fees,
        swap_curve,
        &mint_a,
        &mint_b,
        token_a_amount,
        token_b_amount,
    )
    .await;
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
            assert_eq!(error_index, SwapError::InvalidCurve as u32);
        }
        _ => {
            panic!("Wrong error occurs while trying invalid curve swap")
        }
    }
}

#[tokio::test]
async fn fn_test_initialize_valid_offset_swap() {
    let (mut banks_client, payer, recent_blockhash) = helpers::program_test().start().await;

    let pool_registry_key =
        helpers::create_pool_registry(&mut banks_client, &payer, &recent_blockhash, &payer)
            .await
            .unwrap();

    let token_b_offset = 10;
    let fees = Fees {
        trade_fee_numerator: 1,
        trade_fee_denominator: 2,
        owner_trade_fee_numerator: 1,
        owner_trade_fee_denominator: 10,
        owner_withdraw_fee_numerator: 1,
        owner_withdraw_fee_denominator: 5,
    };

    let swap_curve = SwapCurve {
        curve_type: CurveType::Offset,
        calculator: Arc::new(OffsetCurve { token_b_offset }),
    };
    let token_a_amount = 1000;
    let token_b_amount = 2000;

    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    let mut swap: helpers::TokenSwapAccounts = helpers::TokenSwapAccounts::new(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        pool_registry_key,
        fees,
        swap_curve,
        &mint_a,
        &mint_b,
        token_a_amount,
        token_b_amount,
    )
    .await;
    swap.initialize_swap(&mut banks_client, &payer, &recent_blockhash)
        .await
        .unwrap();
}
