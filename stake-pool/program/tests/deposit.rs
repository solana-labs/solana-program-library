#![allow(clippy::arithmetic_side_effects)]
#![cfg(feature = "test-sbf")]

mod helpers;

use {
    helpers::*,
    solana_program::{
        borsh1::try_from_slice_unchecked,
        instruction::{AccountMeta, Instruction, InstructionError},
        pubkey::Pubkey,
        stake, sysvar,
    },
    solana_program_test::*,
    solana_sdk::{
        signature::{Keypair, Signer},
        transaction::{Transaction, TransactionError},
        transport::TransportError,
    },
    spl_stake_pool::{error::StakePoolError, id, instruction, state, MINIMUM_RESERVE_LAMPORTS},
    spl_token::error as token_error,
    test_case::test_case,
};

async fn setup(
    token_program_id: Pubkey,
) -> (
    ProgramTestContext,
    StakePoolAccounts,
    ValidatorStakeAccount,
    Keypair,
    Pubkey,
    Pubkey,
    u64,
) {
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

    let validator_stake_account = simple_add_validator_to_pool(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts,
        None,
    )
    .await;

    let user = Keypair::new();
    // make stake account
    let deposit_stake = Keypair::new();
    let lockup = stake::state::Lockup::default();

    let authorized = stake::state::Authorized {
        staker: user.pubkey(),
        withdrawer: user.pubkey(),
    };

    let stake_lamports = create_independent_stake_account(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &deposit_stake,
        &authorized,
        &lockup,
        TEST_STAKE_AMOUNT,
    )
    .await;

    delegate_stake_account(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &deposit_stake.pubkey(),
        &user,
        &validator_stake_account.vote.pubkey(),
    )
    .await;

    let first_normal_slot = context.genesis_config().epoch_schedule.first_normal_slot;
    context.warp_to_slot(first_normal_slot + 1).unwrap();
    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            false,
        )
        .await;

    // make pool token account
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
        validator_stake_account,
        user,
        deposit_stake.pubkey(),
        pool_token_account.pubkey(),
        stake_lamports,
    )
}

#[test_case(spl_token::id(); "token")]
#[test_case(spl_token_2022::id(); "token-2022")]
#[tokio::test]
async fn success(token_program_id: Pubkey) {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake_account,
        user,
        deposit_stake,
        pool_token_account,
        stake_lamports,
    ) = setup(token_program_id).await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeStateV2>());

    // Save stake pool state before depositing
    let pre_stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let pre_stake_pool =
        try_from_slice_unchecked::<state::StakePool>(pre_stake_pool.data.as_slice()).unwrap();

    // Save validator stake account record before depositing
    let validator_list = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let validator_list =
        try_from_slice_unchecked::<state::ValidatorList>(validator_list.data.as_slice()).unwrap();
    let pre_validator_stake_item = validator_list
        .find(&validator_stake_account.vote.pubkey())
        .unwrap();

    // Save reserve state before depositing
    let pre_reserve_lamports = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
    )
    .await
    .lamports;

    let error = stake_pool_accounts
        .deposit_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &deposit_stake,
            &pool_token_account,
            &validator_stake_account.stake_account,
            &user,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    // Original stake account should be drained
    assert!(context
        .banks_client
        .get_account(deposit_stake)
        .await
        .expect("get_account")
        .is_none());

    let tokens_issued = stake_lamports; // For now tokens are 1:1 to stake

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
        pre_stake_pool.total_lamports + stake_lamports
    );
    assert_eq!(
        post_stake_pool.pool_token_supply,
        pre_stake_pool.pool_token_supply + tokens_issued
    );

    // Check minted tokens
    let user_token_balance =
        get_token_balance(&mut context.banks_client, &pool_token_account).await;
    let tokens_issued_user = tokens_issued
        - post_stake_pool
            .calc_pool_tokens_sol_deposit_fee(stake_rent)
            .unwrap()
        - post_stake_pool
            .calc_pool_tokens_stake_deposit_fee(stake_lamports - stake_rent)
            .unwrap();
    assert_eq!(user_token_balance, tokens_issued_user);

    // Check balances in validator stake account list storage
    let validator_list = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let validator_list =
        try_from_slice_unchecked::<state::ValidatorList>(validator_list.data.as_slice()).unwrap();
    let post_validator_stake_item = validator_list
        .find(&validator_stake_account.vote.pubkey())
        .unwrap();
    assert_eq!(
        post_validator_stake_item.stake_lamports().unwrap(),
        pre_validator_stake_item.stake_lamports().unwrap() + stake_lamports - stake_rent,
    );

    // Check validator stake account actual SOL balance
    let validator_stake_account = get_account(
        &mut context.banks_client,
        &validator_stake_account.stake_account,
    )
    .await;
    assert_eq!(
        validator_stake_account.lamports,
        post_validator_stake_item.stake_lamports().unwrap()
    );
    assert_eq!(
        u64::from(post_validator_stake_item.transient_stake_lamports),
        0
    );

    // Check reserve
    let post_reserve_lamports = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
    )
    .await
    .lamports;
    assert_eq!(post_reserve_lamports, pre_reserve_lamports + stake_rent);
}

#[tokio::test]
async fn success_with_extra_stake_lamports() {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake_account,
        user,
        deposit_stake,
        pool_token_account,
        stake_lamports,
    ) = setup(spl_token::id()).await;

    let extra_lamports = TEST_STAKE_AMOUNT * 3 + 1;

    transfer(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &deposit_stake,
        extra_lamports,
    )
    .await;

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

    let manager_pool_balance_pre = get_token_balance(
        &mut context.banks_client,
        &stake_pool_accounts.pool_fee_account.pubkey(),
    )
    .await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeStateV2>());

    // Save stake pool state before depositing
    let pre_stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let pre_stake_pool =
        try_from_slice_unchecked::<state::StakePool>(pre_stake_pool.data.as_slice()).unwrap();

    // Save validator stake account record before depositing
    let validator_list = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let validator_list =
        try_from_slice_unchecked::<state::ValidatorList>(validator_list.data.as_slice()).unwrap();
    let pre_validator_stake_item = validator_list
        .find(&validator_stake_account.vote.pubkey())
        .unwrap();

    // Save reserve state before depositing
    let pre_reserve_lamports = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
    )
    .await
    .lamports;

    let error = stake_pool_accounts
        .deposit_stake_with_referral(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &deposit_stake,
            &pool_token_account,
            &validator_stake_account.stake_account,
            &user,
            &referrer_token_account.pubkey(),
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    // Original stake account should be drained
    assert!(context
        .banks_client
        .get_account(deposit_stake)
        .await
        .expect("get_account")
        .is_none());

    let tokens_issued = stake_lamports + extra_lamports;
    // For now tokens are 1:1 to stake

    // Stake pool should add its balance to the pool balance

    // The extra lamports will not get recorded in total stake lamports unless
    // update_stake_pool_balance is called
    let post_stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;

    let post_stake_pool =
        try_from_slice_unchecked::<state::StakePool>(post_stake_pool.data.as_slice()).unwrap();
    assert_eq!(
        post_stake_pool.total_lamports,
        pre_stake_pool.total_lamports + extra_lamports + stake_lamports
    );
    assert_eq!(
        post_stake_pool.pool_token_supply,
        pre_stake_pool.pool_token_supply + tokens_issued
    );

    // Check minted tokens
    let user_token_balance =
        get_token_balance(&mut context.banks_client, &pool_token_account).await;

    let fee_tokens = post_stake_pool
        .calc_pool_tokens_sol_deposit_fee(extra_lamports + stake_rent)
        .unwrap()
        + post_stake_pool
            .calc_pool_tokens_stake_deposit_fee(stake_lamports - stake_rent)
            .unwrap();
    let tokens_issued_user = tokens_issued - fee_tokens;
    assert_eq!(user_token_balance, tokens_issued_user);

    let referrer_balance_post =
        get_token_balance(&mut context.banks_client, &referrer_token_account.pubkey()).await;

    let referral_fee = stake_pool_accounts.calculate_referral_fee(fee_tokens);
    let manager_fee = fee_tokens - referral_fee;

    assert_eq!(referrer_balance_post - referrer_balance_pre, referral_fee);

    let manager_pool_balance_post = get_token_balance(
        &mut context.banks_client,
        &stake_pool_accounts.pool_fee_account.pubkey(),
    )
    .await;
    assert_eq!(
        manager_pool_balance_post - manager_pool_balance_pre,
        manager_fee
    );

    // Check balances in validator stake account list storage
    let validator_list = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let validator_list =
        try_from_slice_unchecked::<state::ValidatorList>(validator_list.data.as_slice()).unwrap();
    let post_validator_stake_item = validator_list
        .find(&validator_stake_account.vote.pubkey())
        .unwrap();
    assert_eq!(
        post_validator_stake_item.stake_lamports().unwrap(),
        pre_validator_stake_item.stake_lamports().unwrap() + stake_lamports - stake_rent,
    );

    // Check validator stake account actual SOL balance
    let validator_stake_account = get_account(
        &mut context.banks_client,
        &validator_stake_account.stake_account,
    )
    .await;
    assert_eq!(
        validator_stake_account.lamports,
        post_validator_stake_item.stake_lamports().unwrap()
    );
    assert_eq!(
        u64::from(post_validator_stake_item.transient_stake_lamports),
        0
    );

    // Check reserve
    let post_reserve_lamports = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
    )
    .await
    .lamports;
    assert_eq!(
        post_reserve_lamports,
        pre_reserve_lamports + stake_rent + extra_lamports
    );
}

#[tokio::test]
async fn fail_with_wrong_stake_program_id() {
    let (
        context,
        stake_pool_accounts,
        validator_stake_account,
        _user,
        deposit_stake,
        pool_token_account,
        _stake_lamports,
    ) = setup(spl_token::id()).await;

    let wrong_stake_program = Pubkey::new_unique();

    let accounts = vec![
        AccountMeta::new(stake_pool_accounts.stake_pool.pubkey(), false),
        AccountMeta::new(stake_pool_accounts.validator_list.pubkey(), false),
        AccountMeta::new_readonly(stake_pool_accounts.stake_deposit_authority, false),
        AccountMeta::new_readonly(stake_pool_accounts.withdraw_authority, false),
        AccountMeta::new(deposit_stake, false),
        AccountMeta::new(validator_stake_account.stake_account, false),
        AccountMeta::new(stake_pool_accounts.reserve_stake.pubkey(), false),
        AccountMeta::new(pool_token_account, false),
        AccountMeta::new(stake_pool_accounts.pool_fee_account.pubkey(), false),
        AccountMeta::new(stake_pool_accounts.pool_fee_account.pubkey(), false),
        AccountMeta::new(stake_pool_accounts.pool_mint.pubkey(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(sysvar::stake_history::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(wrong_stake_program, false),
    ];
    let instruction = Instruction {
        program_id: id(),
        accounts,
        data: borsh::to_vec(&instruction::StakePoolInstruction::DepositStake).unwrap(),
    };

    let mut transaction =
        Transaction::new_with_payer(&[instruction], Some(&context.payer.pubkey()));
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
        _ => panic!("Wrong error occurs while try to make a deposit with wrong stake program ID"),
    }
}

#[tokio::test]
async fn fail_with_wrong_token_program_id() {
    let (
        context,
        stake_pool_accounts,
        validator_stake_account,
        user,
        deposit_stake,
        pool_token_account,
        _stake_lamports,
    ) = setup(spl_token::id()).await;

    let wrong_token_program = Keypair::new();

    let mut transaction = Transaction::new_with_payer(
        &instruction::deposit_stake(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.validator_list.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &deposit_stake,
            &user.pubkey(),
            &validator_stake_account.stake_account,
            &stake_pool_accounts.reserve_stake.pubkey(),
            &pool_token_account,
            &stake_pool_accounts.pool_fee_account.pubkey(),
            &stake_pool_accounts.pool_fee_account.pubkey(),
            &stake_pool_accounts.pool_mint.pubkey(),
            &wrong_token_program.pubkey(),
        ),
        Some(&context.payer.pubkey()),
    );
    transaction.sign(&[&context.payer, &user], context.last_blockhash);
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
async fn fail_with_wrong_validator_list_account() {
    let (
        mut context,
        mut stake_pool_accounts,
        validator_stake_account,
        user,
        deposit_stake,
        pool_token_account,
        _stake_lamports,
    ) = setup(spl_token::id()).await;

    let wrong_validator_list = Keypair::new();
    stake_pool_accounts.validator_list = wrong_validator_list;

    let transaction_error = stake_pool_accounts
        .deposit_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &deposit_stake,
            &pool_token_account,
            &validator_stake_account.stake_account,
            &user,
        )
        .await
        .unwrap()
        .unwrap();

    match transaction_error {
        TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        ) => {
            let program_error = StakePoolError::InvalidValidatorStakeList as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to make a deposit with wrong validator stake list account"),
    }
}

#[tokio::test]
async fn fail_with_unknown_validator() {
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

    let unknown_stake = create_unknown_validator_stake(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts.stake_pool.pubkey(),
        0,
    )
    .await;

    let user = Keypair::new();
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

    // make stake account
    let user_stake = Keypair::new();
    let lockup = stake::state::Lockup::default();
    let authorized = stake::state::Authorized {
        staker: user.pubkey(),
        withdrawer: user.pubkey(),
    };
    create_independent_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake,
        &authorized,
        &lockup,
        TEST_STAKE_AMOUNT,
    )
    .await;
    delegate_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake.pubkey(),
        &user,
        &unknown_stake.vote.pubkey(),
    )
    .await;

    let error = stake_pool_accounts
        .deposit_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake.pubkey(),
            &user_pool_account.pubkey(),
            &unknown_stake.stake_account,
            &user,
        )
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        error,
        TransactionError::InstructionError(
            2,
            InstructionError::Custom(StakePoolError::ValidatorNotFound as u32)
        )
    );
}

#[tokio::test]
async fn fail_with_wrong_withdraw_authority() {
    let (
        mut context,
        mut stake_pool_accounts,
        validator_stake_account,
        user,
        deposit_stake,
        pool_token_account,
        _stake_lamports,
    ) = setup(spl_token::id()).await;

    stake_pool_accounts.withdraw_authority = Pubkey::new_unique();

    let transaction_error = stake_pool_accounts
        .deposit_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &deposit_stake,
            &pool_token_account,
            &validator_stake_account.stake_account,
            &user,
        )
        .await
        .unwrap()
        .unwrap();

    match transaction_error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            let program_error = StakePoolError::InvalidProgramAddress as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to make a deposit with wrong withdraw authority"),
    }
}

#[tokio::test]
async fn fail_with_wrong_mint_for_receiver_acc() {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake_account,
        user,
        deposit_stake,
        _pool_token_account,
        _stake_lamports,
    ) = setup(spl_token::id()).await;

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
        .deposit_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &deposit_stake,
            &outside_pool_fee_acc.pubkey(),
            &validator_stake_account.stake_account,
            &user,
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

#[test_case(spl_token::id(); "token")]
#[test_case(spl_token_2022::id(); "token-2022")]
#[tokio::test]
async fn success_with_slippage(token_program_id: Pubkey) {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake_account,
        user,
        deposit_stake,
        pool_token_account,
        stake_lamports,
    ) = setup(token_program_id).await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeStateV2>());

    // Save stake pool state before depositing
    let pre_stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let pre_stake_pool =
        try_from_slice_unchecked::<state::StakePool>(pre_stake_pool.data.as_slice()).unwrap();

    let tokens_issued = stake_lamports; // For now tokens are 1:1 to stake
    let tokens_issued_user = tokens_issued
        - pre_stake_pool
            .calc_pool_tokens_sol_deposit_fee(stake_rent)
            .unwrap()
        - pre_stake_pool
            .calc_pool_tokens_stake_deposit_fee(stake_lamports - stake_rent)
            .unwrap();

    let error = stake_pool_accounts
        .deposit_stake_with_slippage(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &deposit_stake,
            &pool_token_account,
            &validator_stake_account.stake_account,
            &user,
            tokens_issued_user + 1,
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            2,
            InstructionError::Custom(StakePoolError::ExceededSlippage as u32)
        )
    );

    let error = stake_pool_accounts
        .deposit_stake_with_slippage(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &deposit_stake,
            &pool_token_account,
            &validator_stake_account.stake_account,
            &user,
            tokens_issued_user,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    // Original stake account should be drained
    assert!(context
        .banks_client
        .get_account(deposit_stake)
        .await
        .expect("get_account")
        .is_none());

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
        pre_stake_pool.total_lamports + stake_lamports
    );
    assert_eq!(
        post_stake_pool.pool_token_supply,
        pre_stake_pool.pool_token_supply + tokens_issued
    );

    // Check minted tokens
    let user_token_balance =
        get_token_balance(&mut context.banks_client, &pool_token_account).await;
    assert_eq!(user_token_balance, tokens_issued_user);
}
