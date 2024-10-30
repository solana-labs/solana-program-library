#![allow(clippy::arithmetic_side_effects)]
#![cfg(feature = "test-sbf")]

mod helpers;

use {
    helpers::*,
    solana_program::{
        borsh1::try_from_slice_unchecked, instruction::InstructionError, pubkey::Pubkey,
    },
    solana_program_test::*,
    solana_sdk::{
        signature::{Keypair, Signer},
        transaction::{Transaction, TransactionError},
        transport::TransportError,
    },
    spl_stake_pool::{
        error, id,
        instruction::{self, FundingType},
        state, MINIMUM_RESERVE_LAMPORTS,
    },
    spl_token::error as token_error,
    test_case::test_case,
};

async fn setup(
    token_program_id: Pubkey,
) -> (ProgramTestContext, StakePoolAccounts, Keypair, Pubkey) {
    let mut context = program_test().start_with_context().await;

    let stake_pool_accounts = StakePoolAccounts::new_with_token_program(token_program_id);
    stake_pool_accounts
        .initialize_stake_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            MINIMUM_RESERVE_LAMPORTS,
        )
        .await
        .unwrap();

    let user = Keypair::new();

    // make pool token account for user
    let pool_token_account = Keypair::new();
    create_token_account(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts.token_program_id,
        &pool_token_account,
        &stake_pool_accounts.pool_mint.pubkey(),
        &user,
        &[],
    )
    .await
    .unwrap();

    (
        context,
        stake_pool_accounts,
        user,
        pool_token_account.pubkey(),
    )
}

#[test_case(spl_token::id(); "token")]
#[test_case(spl_token_2022::id(); "token-2022")]
#[tokio::test]
async fn success(token_program_id: Pubkey) {
    let (mut context, stake_pool_accounts, _user, pool_token_account) =
        setup(token_program_id).await;

    // Save stake pool state before depositing
    let pre_stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let pre_stake_pool =
        try_from_slice_unchecked::<state::StakePool>(pre_stake_pool.data.as_slice()).unwrap();

    // Save reserve state before depositing
    let pre_reserve_lamports = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
    )
    .await
    .lamports;

    let error = stake_pool_accounts
        .deposit_sol(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &pool_token_account,
            TEST_STAKE_AMOUNT,
            None,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    let tokens_issued = TEST_STAKE_AMOUNT; // For now tokens are 1:1 to stake

    // Stake pool should add its balance to the pool balance
    let post_stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let post_stake_pool =
        try_from_slice_unchecked::<state::StakePool>(post_stake_pool.data.as_slice()).unwrap();
    assert_eq!(
        post_stake_pool.total_lamports,
        pre_stake_pool.total_lamports + TEST_STAKE_AMOUNT
    );
    assert_eq!(
        post_stake_pool.pool_token_supply,
        pre_stake_pool.pool_token_supply + tokens_issued
    );

    // Check minted tokens
    let user_token_balance =
        get_token_balance(&mut context.banks_client, &pool_token_account).await;
    let tokens_issued_user =
        tokens_issued - stake_pool_accounts.calculate_sol_deposit_fee(tokens_issued);
    assert_eq!(user_token_balance, tokens_issued_user);

    // Check reserve
    let post_reserve_lamports = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
    )
    .await
    .lamports;
    assert_eq!(
        post_reserve_lamports,
        pre_reserve_lamports + TEST_STAKE_AMOUNT
    );
}

#[tokio::test]
async fn fail_with_wrong_token_program_id() {
    let (context, stake_pool_accounts, _user, pool_token_account) = setup(spl_token::id()).await;

    let wrong_token_program = Keypair::new();

    let mut transaction = Transaction::new_with_payer(
        &[instruction::deposit_sol(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &stake_pool_accounts.reserve_stake.pubkey(),
            &context.payer.pubkey(),
            &pool_token_account,
            &stake_pool_accounts.pool_fee_account.pubkey(),
            &stake_pool_accounts.pool_fee_account.pubkey(),
            &stake_pool_accounts.pool_mint.pubkey(),
            &wrong_token_program.pubkey(),
            TEST_STAKE_AMOUNT,
        )],
        Some(&context.payer.pubkey()),
    );
    transaction.sign(&[&context.payer], context.last_blockhash);
    let transaction_error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .into();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(_, error)) => {
            assert_eq!(error, InstructionError::IncorrectProgramId);
        }
        _ => panic!("Wrong error occurs while try to make a deposit with wrong token program ID"),
    }
}

#[tokio::test]
async fn fail_with_wrong_withdraw_authority() {
    let (mut context, mut stake_pool_accounts, _user, pool_token_account) =
        setup(spl_token::id()).await;

    stake_pool_accounts.withdraw_authority = Pubkey::new_unique();

    let transaction_error = stake_pool_accounts
        .deposit_sol(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &pool_token_account,
            TEST_STAKE_AMOUNT,
            None,
        )
        .await
        .unwrap()
        .unwrap();

    match transaction_error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            let program_error = error::StakePoolError::InvalidProgramAddress as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to make a deposit with wrong withdraw authority"),
    }
}

#[tokio::test]
async fn fail_with_wrong_mint_for_receiver_acc() {
    let (mut context, stake_pool_accounts, _user, _pool_token_account) =
        setup(spl_token::id()).await;

    let outside_mint = Keypair::new();
    let outside_withdraw_auth = Keypair::new();
    let outside_manager = Keypair::new();
    let outside_pool_fee_acc = Keypair::new();

    create_mint(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts.token_program_id,
        &outside_mint,
        &outside_withdraw_auth.pubkey(),
        0,
        &[],
    )
    .await
    .unwrap();

    create_token_account(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts.token_program_id,
        &outside_pool_fee_acc,
        &outside_mint.pubkey(),
        &outside_manager,
        &[],
    )
    .await
    .unwrap();

    let transaction_error = stake_pool_accounts
        .deposit_sol(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &outside_pool_fee_acc.pubkey(),
            TEST_STAKE_AMOUNT,
            None,
        )
        .await
        .unwrap()
        .unwrap();

    match transaction_error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            let program_error = token_error::TokenError::MintMismatch as u32;
            assert_eq!(error_index, program_error);
        }
        _ => {
            panic!("Wrong error occurs while try to deposit with wrong mint from receiver account")
        }
    }
}

#[tokio::test]
async fn success_with_sol_deposit_authority() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::default();
    stake_pool_accounts
        .initialize_stake_pool(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            MINIMUM_RESERVE_LAMPORTS,
        )
        .await
        .unwrap();

    let user = Keypair::new();

    // make pool token account
    let user_pool_account = Keypair::new();
    create_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts.token_program_id,
        &user_pool_account,
        &stake_pool_accounts.pool_mint.pubkey(),
        &user,
        &[],
    )
    .await
    .unwrap();

    let error = stake_pool_accounts
        .deposit_sol(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_pool_account.pubkey(),
            TEST_STAKE_AMOUNT,
            None,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    let sol_deposit_authority = Keypair::new();

    let mut transaction = Transaction::new_with_payer(
        &[instruction::set_funding_authority(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            Some(&sol_deposit_authority.pubkey()),
            FundingType::SolDeposit,
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &stake_pool_accounts.manager], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    let error = stake_pool_accounts
        .deposit_sol(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_pool_account.pubkey(),
            TEST_STAKE_AMOUNT,
            Some(&sol_deposit_authority),
        )
        .await;
    assert!(error.is_none(), "{:?}", error);
}

#[tokio::test]
async fn fail_without_sol_deposit_authority_signature() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let sol_deposit_authority = Keypair::new();
    let stake_pool_accounts = StakePoolAccounts::default();
    stake_pool_accounts
        .initialize_stake_pool(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            MINIMUM_RESERVE_LAMPORTS,
        )
        .await
        .unwrap();

    let user = Keypair::new();

    // make pool token account
    let user_pool_account = Keypair::new();
    create_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts.token_program_id,
        &user_pool_account,
        &stake_pool_accounts.pool_mint.pubkey(),
        &user,
        &[],
    )
    .await
    .unwrap();

    let mut transaction = Transaction::new_with_payer(
        &[instruction::set_funding_authority(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.manager.pubkey(),
            Some(&sol_deposit_authority.pubkey()),
            FundingType::SolDeposit,
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &stake_pool_accounts.manager], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    let wrong_depositor = Keypair::new();

    let error = stake_pool_accounts
        .deposit_sol(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_pool_account.pubkey(),
            TEST_STAKE_AMOUNT,
            Some(&wrong_depositor),
        )
        .await
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            assert_eq!(
                error_index,
                error::StakePoolError::InvalidSolDepositAuthority as u32
            );
        }
        _ => panic!("Wrong error occurs while trying to make a deposit without SOL deposit authority signature"),
    }
}

#[tokio::test]
async fn success_with_referral_fee() {
    let (mut context, stake_pool_accounts, _user, pool_token_account) =
        setup(spl_token::id()).await;

    let referrer = Keypair::new();
    let referrer_token_account = Keypair::new();
    create_token_account(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts.token_program_id,
        &referrer_token_account,
        &stake_pool_accounts.pool_mint.pubkey(),
        &referrer,
        &[],
    )
    .await
    .unwrap();

    let referrer_balance_pre =
        get_token_balance(&mut context.banks_client, &referrer_token_account.pubkey()).await;

    let mut transaction = Transaction::new_with_payer(
        &[instruction::deposit_sol(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &stake_pool_accounts.reserve_stake.pubkey(),
            &context.payer.pubkey(),
            &pool_token_account,
            &stake_pool_accounts.pool_fee_account.pubkey(),
            &referrer_token_account.pubkey(),
            &stake_pool_accounts.pool_mint.pubkey(),
            &spl_token::id(),
            TEST_STAKE_AMOUNT,
        )],
        Some(&context.payer.pubkey()),
    );
    transaction.sign(&[&context.payer], context.last_blockhash);
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let referrer_balance_post =
        get_token_balance(&mut context.banks_client, &referrer_token_account.pubkey()).await;
    let referral_fee = stake_pool_accounts.calculate_sol_referral_fee(
        stake_pool_accounts.calculate_sol_deposit_fee(TEST_STAKE_AMOUNT),
    );
    assert!(referral_fee > 0);
    assert_eq!(referrer_balance_pre + referral_fee, referrer_balance_post);
}

#[tokio::test]
async fn fail_with_invalid_referrer() {
    let (context, stake_pool_accounts, _user, pool_token_account) = setup(spl_token::id()).await;

    let invalid_token_account = Keypair::new();

    let mut transaction = Transaction::new_with_payer(
        &[instruction::deposit_sol(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &stake_pool_accounts.reserve_stake.pubkey(),
            &context.payer.pubkey(),
            &pool_token_account,
            &stake_pool_accounts.pool_fee_account.pubkey(),
            &invalid_token_account.pubkey(),
            &stake_pool_accounts.pool_mint.pubkey(),
            &spl_token::id(),
            TEST_STAKE_AMOUNT,
        )],
        Some(&context.payer.pubkey()),
    );
    transaction.sign(&[&context.payer], context.last_blockhash);
    let transaction_error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .unwrap();

    match transaction_error {
        TransactionError::InstructionError(_, InstructionError::InvalidAccountData) => (),
        _ => panic!(
            "Wrong error occurs while try to make a deposit with an invalid referrer account"
        ),
    }
}

#[test_case(spl_token::id(); "token")]
#[test_case(spl_token_2022::id(); "token-2022")]
#[tokio::test]
async fn success_with_slippage(token_program_id: Pubkey) {
    let (mut context, stake_pool_accounts, _user, pool_token_account) =
        setup(token_program_id).await;

    // Save stake pool state before depositing
    let pre_stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let pre_stake_pool =
        try_from_slice_unchecked::<state::StakePool>(pre_stake_pool.data.as_slice()).unwrap();

    // Save reserve state before depositing
    let pre_reserve_lamports = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
    )
    .await
    .lamports;

    let new_pool_tokens = pre_stake_pool
        .calc_pool_tokens_for_deposit(TEST_STAKE_AMOUNT)
        .unwrap();
    let pool_tokens_sol_deposit_fee = pre_stake_pool
        .calc_pool_tokens_sol_deposit_fee(new_pool_tokens)
        .unwrap();
    let tokens_issued = new_pool_tokens - pool_tokens_sol_deposit_fee;

    // Fail with 1 more token in slippage
    let error = stake_pool_accounts
        .deposit_sol_with_slippage(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &pool_token_account,
            TEST_STAKE_AMOUNT,
            tokens_issued + 1,
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(error::StakePoolError::ExceededSlippage as u32)
        )
    );

    // Succeed with exact return amount
    let error = stake_pool_accounts
        .deposit_sol_with_slippage(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &pool_token_account,
            TEST_STAKE_AMOUNT,
            tokens_issued,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    // Stake pool should add its balance to the pool balance
    let post_stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let post_stake_pool =
        try_from_slice_unchecked::<state::StakePool>(post_stake_pool.data.as_slice()).unwrap();
    assert_eq!(
        post_stake_pool.total_lamports,
        pre_stake_pool.total_lamports + TEST_STAKE_AMOUNT
    );
    assert_eq!(
        post_stake_pool.pool_token_supply,
        pre_stake_pool.pool_token_supply + new_pool_tokens
    );

    // Check minted tokens
    let user_token_balance =
        get_token_balance(&mut context.banks_client, &pool_token_account).await;
    assert_eq!(user_token_balance, tokens_issued);

    // Check reserve
    let post_reserve_lamports = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
    )
    .await
    .lamports;
    assert_eq!(
        post_reserve_lamports,
        pre_reserve_lamports + TEST_STAKE_AMOUNT
    );
}
