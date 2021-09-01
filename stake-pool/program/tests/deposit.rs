#![cfg(feature = "test-bpf")]

mod helpers;

use {
    bincode::deserialize,
    borsh::BorshSerialize,
    helpers::*,
    solana_program::{
        borsh::try_from_slice_unchecked,
        instruction::{AccountMeta, Instruction, InstructionError},
        pubkey::Pubkey,
        sysvar,
    },
    solana_program_test::*,
    solana_sdk::{
        signature::{Keypair, Signer},
        transaction::Transaction,
        transaction::TransactionError,
        transport::TransportError,
    },
    spl_stake_pool::{error, id, instruction, minimum_stake_lamports, stake_program, state},
    spl_token::error as token_error,
};

async fn setup() -> (
    ProgramTestContext,
    StakePoolAccounts,
    ValidatorStakeAccount,
    Keypair,
    Pubkey,
    Pubkey,
    u64,
) {
    let mut context = program_test().start_with_context().await;

    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            1,
        )
        .await
        .unwrap();

    let validator_stake_account = simple_add_validator_to_pool(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts,
    )
    .await;

    let first_normal_slot = context.genesis_config().epoch_schedule.first_normal_slot;
    let slots_per_epoch = context.genesis_config().epoch_schedule.slots_per_epoch;
    let mut slot = first_normal_slot;
    context.warp_to_slot(slot).unwrap();

    let user = Keypair::new();
    // make stake account
    let deposit_stake = Keypair::new();
    let lockup = stake_program::Lockup::default();

    let authorized = stake_program::Authorized {
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

    slot += 2 * slots_per_epoch;
    context.warp_to_slot(slot).unwrap();
    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &[validator_stake_account.vote.pubkey()],
            false,
        )
        .await;

    // make pool token account
    let pool_token_account = Keypair::new();
    create_token_account(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &pool_token_account,
        &stake_pool_accounts.pool_mint.pubkey(),
        &user.pubkey(),
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

#[tokio::test]
async fn success() {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake_account,
        user,
        deposit_stake,
        pool_token_account,
        stake_lamports,
    ) = setup().await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake_program::StakeState>());

    // Save stake pool state before depositing
    let pre_stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let pre_stake_pool =
        try_from_slice_unchecked::<state::StakePool>(&pre_stake_pool.data.as_slice()).unwrap();

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
    assert!(error.is_none());

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
        try_from_slice_unchecked::<state::StakePool>(&post_stake_pool.data.as_slice()).unwrap();
    assert_eq!(
        post_stake_pool.total_stake_lamports,
        pre_stake_pool.total_stake_lamports + stake_lamports
    );
    assert_eq!(
        post_stake_pool.pool_token_supply,
        pre_stake_pool.pool_token_supply + tokens_issued
    );

    // Check minted tokens
    let user_token_balance =
        get_token_balance(&mut context.banks_client, &pool_token_account).await;
    let tokens_issued_user =
        tokens_issued - stake_pool_accounts.calculate_deposit_fee(tokens_issued);
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
        post_validator_stake_item.stake_lamports(),
        pre_validator_stake_item.stake_lamports() + stake_lamports - stake_rent,
    );

    // Check validator stake account actual SOL balance
    let validator_stake_account = get_account(
        &mut context.banks_client,
        &validator_stake_account.stake_account,
    )
    .await;
    let stake_state =
        deserialize::<stake_program::StakeState>(&validator_stake_account.data).unwrap();
    let meta = stake_state.meta().unwrap();
    assert_eq!(
        validator_stake_account.lamports - minimum_stake_lamports(&meta),
        post_validator_stake_item.stake_lamports()
    );
    assert_eq!(post_validator_stake_item.transient_stake_lamports, 0);

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
    ) = setup().await;

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
        &referrer_token_account,
        &stake_pool_accounts.pool_mint.pubkey(),
        &referrer.pubkey(),
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
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake_program::StakeState>());

    // Save stake pool state before depositing
    let pre_stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let pre_stake_pool =
        try_from_slice_unchecked::<state::StakePool>(&pre_stake_pool.data.as_slice()).unwrap();

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
    assert!(error.is_none());

    // Original stake account should be drained
    assert!(context
        .banks_client
        .get_account(deposit_stake)
        .await
        .expect("get_account")
        .is_none());

    let tokens_issued = stake_lamports;
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
        try_from_slice_unchecked::<state::StakePool>(&post_stake_pool.data.as_slice()).unwrap();
    assert_eq!(
        post_stake_pool.total_stake_lamports,
        pre_stake_pool.total_stake_lamports + extra_lamports + stake_lamports
    );
    assert_eq!(
        post_stake_pool.pool_token_supply,
        pre_stake_pool.pool_token_supply + tokens_issued
    );

    // Check minted tokens
    let user_token_balance =
        get_token_balance(&mut context.banks_client, &pool_token_account).await;

    let tokens_issued_user =
        tokens_issued - stake_pool_accounts.calculate_deposit_fee(tokens_issued);
    assert_eq!(user_token_balance, tokens_issued_user);

    let referrer_balance_post =
        get_token_balance(&mut context.banks_client, &referrer_token_account.pubkey()).await;

    let tokens_issued_fees = stake_pool_accounts.calculate_deposit_fee(tokens_issued);
    let tokens_issued_referral_fee = stake_pool_accounts
        .calculate_referral_fee(stake_pool_accounts.calculate_deposit_fee(tokens_issued));
    let tokens_issued_manager_fee = tokens_issued_fees - tokens_issued_referral_fee;

    assert_eq!(
        referrer_balance_post - referrer_balance_pre,
        tokens_issued_referral_fee
    );

    let manager_pool_balance_post = get_token_balance(
        &mut context.banks_client,
        &stake_pool_accounts.pool_fee_account.pubkey(),
    )
    .await;
    assert_eq!(
        manager_pool_balance_post - manager_pool_balance_pre,
        tokens_issued_manager_fee
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
        post_validator_stake_item.stake_lamports(),
        pre_validator_stake_item.stake_lamports() + stake_lamports - stake_rent,
    );

    // Check validator stake account actual SOL balance
    let validator_stake_account = get_account(
        &mut context.banks_client,
        &validator_stake_account.stake_account,
    )
    .await;
    let stake_state =
        deserialize::<stake_program::StakeState>(&validator_stake_account.data).unwrap();
    let meta = stake_state.meta().unwrap();
    assert_eq!(
        validator_stake_account.lamports - minimum_stake_lamports(&meta),
        post_validator_stake_item.stake_lamports()
    );
    assert_eq!(post_validator_stake_item.transient_stake_lamports, 0);

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
        mut context,
        stake_pool_accounts,
        validator_stake_account,
        _user,
        deposit_stake,
        pool_token_account,
        _stake_lamports,
    ) = setup().await;

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
        data: instruction::StakePoolInstruction::DepositStake
            .try_to_vec()
            .unwrap(),
    };

    let mut transaction =
        Transaction::new_with_payer(&[instruction], Some(&context.payer.pubkey()));
    transaction.sign(&[&context.payer], context.last_blockhash);
    let transaction_error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap();

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
        mut context,
        stake_pool_accounts,
        validator_stake_account,
        user,
        deposit_stake,
        pool_token_account,
        _stake_lamports,
    ) = setup().await;

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
        .unwrap();

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
    ) = setup().await;

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
            let program_error = error::StakePoolError::InvalidValidatorStakeList as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to make a deposit with wrong validator stake list account"),
    }
}

#[tokio::test]
async fn fail_with_unknown_validator() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash, 1)
        .await
        .unwrap();

    let validator_stake_account =
        ValidatorStakeAccount::new(&stake_pool_accounts.stake_pool.pubkey(), u64::MAX);
    validator_stake_account
        .create_and_delegate(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &stake_pool_accounts.staker,
        )
        .await;

    let user_pool_account = Keypair::new();
    let user = Keypair::new();
    create_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_pool_account,
        &stake_pool_accounts.pool_mint.pubkey(),
        &user.pubkey(),
    )
    .await
    .unwrap();

    // make stake account
    let user_stake = Keypair::new();
    let lockup = stake_program::Lockup::default();
    let authorized = stake_program::Authorized {
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
    let random_vote_account = Keypair::new();
    create_vote(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &Keypair::new(),
        &random_vote_account,
    )
    .await;
    delegate_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake.pubkey(),
        &user,
        &random_vote_account.pubkey(),
    )
    .await;

    let transaction_error = stake_pool_accounts
        .deposit_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake.pubkey(),
            &user_pool_account.pubkey(),
            &validator_stake_account.stake_account,
            &user,
        )
        .await
        .unwrap()
        .unwrap();

    match transaction_error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            let program_error = error::StakePoolError::ValidatorNotFound as u32;
            assert_eq!(error_index, program_error);
        }
        _ => {
            panic!("Wrong error occurs while try to make a deposit with unknown validator account")
        }
    }
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
    ) = setup().await;

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
            let program_error = error::StakePoolError::InvalidProgramAddress as u32;
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
    ) = setup().await;

    let outside_mint = Keypair::new();
    let outside_withdraw_auth = Keypair::new();
    let outside_manager = Keypair::new();
    let outside_pool_fee_acc = Keypair::new();

    create_mint(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &outside_mint,
        &outside_withdraw_auth.pubkey(),
    )
    .await
    .unwrap();

    create_token_account(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &outside_pool_fee_acc,
        &outside_mint.pubkey(),
        &outside_manager.pubkey(),
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

#[tokio::test]
async fn fail_with_uninitialized_validator_list() {} // TODO

#[tokio::test]
async fn fail_with_out_of_dated_pool_balances() {} // TODO

#[tokio::test]
async fn success_with_stake_deposit_authority() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_deposit_authority = Keypair::new();
    let stake_pool_accounts =
        StakePoolAccounts::new_with_stake_deposit_authority(stake_deposit_authority);
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash, 1)
        .await
        .unwrap();

    let validator_stake_account = simple_add_validator_to_pool(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
    )
    .await;

    let user = Keypair::new();
    let user_stake = Keypair::new();
    let lockup = stake_program::Lockup::default();
    let authorized = stake_program::Authorized {
        staker: user.pubkey(),
        withdrawer: user.pubkey(),
    };
    let _stake_lamports = create_independent_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake,
        &authorized,
        &lockup,
        TEST_STAKE_AMOUNT,
    )
    .await;

    create_vote(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &validator_stake_account.validator,
        &validator_stake_account.vote,
    )
    .await;
    delegate_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake.pubkey(),
        &user,
        &validator_stake_account.vote.pubkey(),
    )
    .await;

    // make pool token account
    let user_pool_account = Keypair::new();
    create_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_pool_account,
        &stake_pool_accounts.pool_mint.pubkey(),
        &user.pubkey(),
    )
    .await
    .unwrap();

    let error = stake_pool_accounts
        .deposit_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake.pubkey(),
            &user_pool_account.pubkey(),
            &validator_stake_account.stake_account,
            &user,
        )
        .await;
    assert!(error.is_none());
}

#[tokio::test]
async fn fail_without_stake_deposit_authority_signature() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_deposit_authority = Keypair::new();
    let mut stake_pool_accounts =
        StakePoolAccounts::new_with_stake_deposit_authority(stake_deposit_authority);
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash, 1)
        .await
        .unwrap();

    let validator_stake_account = simple_add_validator_to_pool(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
    )
    .await;

    let user = Keypair::new();
    let user_stake = Keypair::new();
    let lockup = stake_program::Lockup::default();
    let authorized = stake_program::Authorized {
        staker: user.pubkey(),
        withdrawer: user.pubkey(),
    };
    let _stake_lamports = create_independent_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake,
        &authorized,
        &lockup,
        TEST_STAKE_AMOUNT,
    )
    .await;

    create_vote(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &validator_stake_account.validator,
        &validator_stake_account.vote,
    )
    .await;
    delegate_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake.pubkey(),
        &user,
        &validator_stake_account.vote.pubkey(),
    )
    .await;

    // make pool token account
    let user_pool_account = Keypair::new();
    create_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_pool_account,
        &stake_pool_accounts.pool_mint.pubkey(),
        &user.pubkey(),
    )
    .await
    .unwrap();

    let wrong_depositor = Keypair::new();
    stake_pool_accounts.stake_deposit_authority = wrong_depositor.pubkey();
    stake_pool_accounts.stake_deposit_authority_keypair = Some(wrong_depositor);

    let error = stake_pool_accounts
        .deposit_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake.pubkey(),
            &user_pool_account.pubkey(),
            &validator_stake_account.stake_account,
            &user,
        )
        .await
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            assert_eq!(
                error_index,
                error::StakePoolError::InvalidStakeDepositAuthority as u32
            );
        }
        _ => panic!("Wrong error occurs while try to make a deposit with wrong stake program ID"),
    }
}

#[tokio::test]
async fn success_with_preferred_deposit() {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake,
        user,
        deposit_stake,
        pool_token_account,
        _stake_lamports,
    ) = setup().await;

    stake_pool_accounts
        .set_preferred_validator(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            instruction::PreferredValidatorType::Deposit,
            Some(validator_stake.vote.pubkey()),
        )
        .await;

    let error = stake_pool_accounts
        .deposit_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &deposit_stake,
            &pool_token_account,
            &validator_stake.stake_account,
            &user,
        )
        .await;
    assert!(error.is_none());
}

#[tokio::test]
async fn fail_with_wrong_preferred_deposit() {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake,
        user,
        deposit_stake,
        pool_token_account,
        _stake_lamports,
    ) = setup().await;

    let preferred_validator = simple_add_validator_to_pool(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts,
    )
    .await;

    stake_pool_accounts
        .set_preferred_validator(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            instruction::PreferredValidatorType::Deposit,
            Some(preferred_validator.vote.pubkey()),
        )
        .await;

    let error = stake_pool_accounts
        .deposit_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &deposit_stake,
            &pool_token_account,
            &validator_stake.stake_account,
            &user,
        )
        .await
        .unwrap()
        .unwrap();
    match error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            assert_eq!(
                error_index,
                error::StakePoolError::IncorrectDepositVoteAddress as u32
            );
        }
        _ => panic!("Wrong error occurs while try to make a deposit with wrong stake program ID"),
    }
}

#[tokio::test]
async fn success_with_referral_fee() {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake_account,
        user,
        deposit_stake,
        pool_token_account,
        stake_lamports,
    ) = setup().await;

    let referrer = Keypair::new();
    let referrer_token_account = Keypair::new();
    create_token_account(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &referrer_token_account,
        &stake_pool_accounts.pool_mint.pubkey(),
        &referrer.pubkey(),
    )
    .await
    .unwrap();

    let referrer_balance_pre =
        get_token_balance(&mut context.banks_client, &referrer_token_account.pubkey()).await;

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
            &referrer_token_account.pubkey(),
            &stake_pool_accounts.pool_mint.pubkey(),
            &spl_token::id(),
        ),
        Some(&context.payer.pubkey()),
    );
    transaction.sign(&[&context.payer, &user], context.last_blockhash);
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let referrer_balance_post =
        get_token_balance(&mut context.banks_client, &referrer_token_account.pubkey()).await;
    let referral_fee = stake_pool_accounts
        .calculate_referral_fee(stake_pool_accounts.calculate_deposit_fee(stake_lamports));
    assert!(referral_fee > 0);
    assert_eq!(referrer_balance_pre + referral_fee, referrer_balance_post);
}

#[tokio::test]
async fn fail_with_invalid_referrer() {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake_account,
        user,
        deposit_stake,
        pool_token_account,
        _stake_lamports,
    ) = setup().await;

    let invalid_token_account = Keypair::new();

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
            &invalid_token_account.pubkey(),
            &stake_pool_accounts.pool_mint.pubkey(),
            &spl_token::id(),
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
        .unwrap();

    match transaction_error {
        TransactionError::InstructionError(_, InstructionError::InvalidAccountData) => (),
        _ => panic!(
            "Wrong error occurs while try to make a deposit with an invalid referrer account"
        ),
    }
}
