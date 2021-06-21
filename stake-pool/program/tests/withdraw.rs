#![cfg(feature = "test-bpf")]

mod helpers;

use {
    bincode::deserialize,
    borsh::BorshSerialize,
    helpers::*,
    solana_program::{
        borsh::try_from_slice_unchecked,
        hash::Hash,
        instruction::{AccountMeta, Instruction, InstructionError},
        pubkey::Pubkey,
        sysvar,
    },
    solana_program_test::*,
    solana_sdk::{
        signature::{Keypair, Signer},
        transaction::{Transaction, TransactionError},
        transport::TransportError,
    },
    spl_stake_pool::{
        error::StakePoolError, id, instruction, minimum_stake_lamports, stake_program, state,
    },
    spl_token::error::TokenError,
};

async fn setup() -> (
    BanksClient,
    Keypair,
    Hash,
    StakePoolAccounts,
    ValidatorStakeAccount,
    DepositStakeAccount,
    Keypair,
    Keypair,
    u64,
) {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
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

    let deposit_info = simple_deposit(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
        &validator_stake_account,
        TEST_STAKE_AMOUNT,
    )
    .await
    .unwrap();

    let tokens_to_burn = deposit_info.pool_tokens / 4;

    // Delegate tokens for burning
    let user_transfer_authority = Keypair::new();
    delegate_tokens(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &deposit_info.pool_account.pubkey(),
        &deposit_info.authority,
        &user_transfer_authority.pubkey(),
        tokens_to_burn,
    )
    .await;

    // Create stake account to withdraw to
    let user_stake_recipient = Keypair::new();
    create_blank_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake_recipient,
    )
    .await;

    (
        banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        validator_stake_account,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_burn,
    )
}

#[tokio::test]
async fn success() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        validator_stake_account,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_burn,
    ) = setup().await;

    // Save stake pool state before withdrawal
    let stake_pool_before =
        get_account(&mut banks_client, &stake_pool_accounts.stake_pool.pubkey()).await;
    let stake_pool_before =
        try_from_slice_unchecked::<state::StakePool>(&stake_pool_before.data.as_slice()).unwrap();

    // Check user recipient stake account balance
    let initial_stake_lamports = get_account(&mut banks_client, &user_stake_recipient.pubkey())
        .await
        .lamports;

    // Save validator stake account record before withdrawal
    let validator_list = get_account(
        &mut banks_client,
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let validator_list =
        try_from_slice_unchecked::<state::ValidatorList>(validator_list.data.as_slice()).unwrap();
    let validator_stake_item_before = validator_list
        .find(&validator_stake_account.vote.pubkey())
        .unwrap();

    // Save user token balance
    let user_token_balance_before =
        get_token_balance(&mut banks_client, &deposit_info.pool_account.pubkey()).await;

    let new_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .withdraw_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &validator_stake_account.stake_account,
            &new_authority,
            tokens_to_burn,
        )
        .await;
    assert!(error.is_none());

    // Check pool stats
    let stake_pool = get_account(&mut banks_client, &stake_pool_accounts.stake_pool.pubkey()).await;
    let stake_pool =
        try_from_slice_unchecked::<state::StakePool>(&stake_pool.data.as_slice()).unwrap();
    assert_eq!(
        stake_pool.total_stake_lamports,
        stake_pool_before.total_stake_lamports - tokens_to_burn
    );
    assert_eq!(
        stake_pool.pool_token_supply,
        stake_pool_before.pool_token_supply - tokens_to_burn
    );

    // Check validator stake list storage
    let validator_list = get_account(
        &mut banks_client,
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let validator_list =
        try_from_slice_unchecked::<state::ValidatorList>(validator_list.data.as_slice()).unwrap();
    let validator_stake_item = validator_list
        .find(&validator_stake_account.vote.pubkey())
        .unwrap();
    assert_eq!(
        validator_stake_item.stake_lamports(),
        validator_stake_item_before.stake_lamports() - tokens_to_burn
    );
    assert_eq!(
        validator_stake_item.active_stake_lamports,
        validator_stake_item.stake_lamports(),
    );

    // Check tokens burned
    let user_token_balance =
        get_token_balance(&mut banks_client, &deposit_info.pool_account.pubkey()).await;
    assert_eq!(
        user_token_balance,
        user_token_balance_before - tokens_to_burn
    );

    // Check validator stake account balance
    let validator_stake_account =
        get_account(&mut banks_client, &validator_stake_account.stake_account).await;
    let stake_state =
        deserialize::<stake_program::StakeState>(&validator_stake_account.data).unwrap();
    let meta = stake_state.meta().unwrap();
    assert_eq!(
        validator_stake_account.lamports - minimum_stake_lamports(&meta),
        validator_stake_item.active_stake_lamports
    );

    // Check user recipient stake account balance
    let user_stake_recipient_account =
        get_account(&mut banks_client, &user_stake_recipient.pubkey()).await;
    assert_eq!(
        user_stake_recipient_account.lamports,
        initial_stake_lamports + tokens_to_burn
    );
}

#[tokio::test]
async fn fail_with_wrong_stake_program() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        validator_stake_account,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_burn,
    ) = setup().await;

    let new_authority = Pubkey::new_unique();
    let wrong_stake_program = Pubkey::new_unique();

    let accounts = vec![
        AccountMeta::new(stake_pool_accounts.stake_pool.pubkey(), false),
        AccountMeta::new(stake_pool_accounts.validator_list.pubkey(), false),
        AccountMeta::new_readonly(stake_pool_accounts.withdraw_authority, false),
        AccountMeta::new(validator_stake_account.stake_account, false),
        AccountMeta::new(user_stake_recipient.pubkey(), false),
        AccountMeta::new_readonly(new_authority, false),
        AccountMeta::new_readonly(user_transfer_authority.pubkey(), true),
        AccountMeta::new(deposit_info.pool_account.pubkey(), false),
        AccountMeta::new(stake_pool_accounts.pool_mint.pubkey(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(wrong_stake_program, false),
    ];
    let instruction = Instruction {
        program_id: id(),
        accounts,
        data: instruction::StakePoolInstruction::Withdraw(tokens_to_burn)
            .try_to_vec()
            .unwrap(),
    };

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer, &user_transfer_authority],
        recent_blockhash,
    );
    let transaction_error = banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(_, error)) => {
            assert_eq!(error, InstructionError::IncorrectProgramId);
        }
        _ => panic!("Wrong error occurs while try to withdraw with wrong stake program ID"),
    }
}

#[tokio::test]
async fn fail_with_wrong_withdraw_authority() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        mut stake_pool_accounts,
        validator_stake_account,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_burn,
    ) = setup().await;

    let new_authority = Pubkey::new_unique();
    stake_pool_accounts.withdraw_authority = Keypair::new().pubkey();

    let transaction_error = stake_pool_accounts
        .withdraw_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &validator_stake_account.stake_account,
            &new_authority,
            tokens_to_burn,
        )
        .await
        .unwrap();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = StakePoolError::InvalidProgramAddress as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to withdraw with wrong withdraw authority"),
    }
}

#[tokio::test]
async fn fail_with_wrong_token_program_id() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        validator_stake_account,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_burn,
    ) = setup().await;

    let new_authority = Pubkey::new_unique();
    let wrong_token_program = Keypair::new();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::withdraw(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.validator_list.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &validator_stake_account.stake_account,
            &user_stake_recipient.pubkey(),
            &new_authority,
            &user_transfer_authority.pubkey(),
            &deposit_info.pool_account.pubkey(),
            &stake_pool_accounts.pool_mint.pubkey(),
            &wrong_token_program.pubkey(),
            tokens_to_burn,
        )],
        Some(&payer.pubkey()),
        &[&payer, &user_transfer_authority],
        recent_blockhash,
    );
    let transaction_error = banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(_, error)) => {
            assert_eq!(error, InstructionError::IncorrectProgramId);
        }
        _ => panic!("Wrong error occurs while try to withdraw with wrong token program ID"),
    }
}

#[tokio::test]
async fn fail_with_wrong_validator_list() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        mut stake_pool_accounts,
        validator_stake_account,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_burn,
    ) = setup().await;

    let new_authority = Pubkey::new_unique();
    stake_pool_accounts.validator_list = Keypair::new();

    let transaction_error = stake_pool_accounts
        .withdraw_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &validator_stake_account.stake_account,
            &new_authority,
            tokens_to_burn,
        )
        .await
        .unwrap();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = StakePoolError::InvalidValidatorStakeList as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!(
            "Wrong error occurs while try to withdraw with wrong validator stake list account"
        ),
    }
}

#[tokio::test]
async fn fail_with_unknown_validator() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        _,
        _,
        user_transfer_authority,
        user_stake_recipient,
        _,
    ) = setup().await;

    let validator_stake_account =
        ValidatorStakeAccount::new(&stake_pool_accounts.stake_pool.pubkey());
    validator_stake_account
        .create_and_delegate(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &stake_pool_accounts.staker,
        )
        .await;

    let user_stake = ValidatorStakeAccount::new(&stake_pool_accounts.stake_pool.pubkey());
    user_stake
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

    let user = Keypair::new();
    // make stake account
    let user_stake = Keypair::new();
    let lockup = stake_program::Lockup::default();
    let authorized = stake_program::Authorized {
        staker: stake_pool_accounts.deposit_authority,
        withdrawer: stake_pool_accounts.deposit_authority,
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

    let user_pool_account = user_pool_account.pubkey();
    let pool_tokens = get_token_balance(&mut banks_client, &user_pool_account).await;

    let tokens_to_burn = pool_tokens / 4;

    // Delegate tokens for burning
    delegate_tokens(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_pool_account,
        &user,
        &user_transfer_authority.pubkey(),
        tokens_to_burn,
    )
    .await;

    let new_authority = Pubkey::new_unique();

    let transaction_error = stake_pool_accounts
        .withdraw_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &user_pool_account,
            &validator_stake_account.stake_account,
            &new_authority,
            tokens_to_burn,
        )
        .await
        .unwrap()
        .unwrap();

    match transaction_error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            let program_error = StakePoolError::ValidatorNotFound as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to do withdraw from unknown validator"),
    }
}

#[tokio::test]
async fn fail_double_withdraw_to_the_same_account() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        validator_stake_account,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_burn,
    ) = setup().await;

    let new_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .withdraw_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &validator_stake_account.stake_account,
            &new_authority,
            tokens_to_burn,
        )
        .await;
    assert!(error.is_none());

    let latest_blockhash = banks_client.get_recent_blockhash().await.unwrap();

    // Delegate tokens for burning
    delegate_tokens(
        &mut banks_client,
        &payer,
        &latest_blockhash,
        &deposit_info.pool_account.pubkey(),
        &deposit_info.authority,
        &user_transfer_authority.pubkey(),
        tokens_to_burn,
    )
    .await;

    let transaction_error = stake_pool_accounts
        .withdraw_stake(
            &mut banks_client,
            &payer,
            &latest_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &validator_stake_account.stake_account,
            &new_authority,
            tokens_to_burn,
        )
        .await
        .unwrap();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(_, error)) => {
            assert_eq!(error, InstructionError::InvalidAccountData);
        }
        _ => panic!("Wrong error occurs while try to do double withdraw"),
    }
}

#[tokio::test]
async fn fail_without_token_approval() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
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

    let deposit_info = simple_deposit(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
        &validator_stake_account,
        TEST_STAKE_AMOUNT,
    )
    .await
    .unwrap();

    let tokens_to_burn = deposit_info.pool_tokens / 4;

    // Create stake account to withdraw to
    let user_stake_recipient = Keypair::new();
    create_blank_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake_recipient,
    )
    .await;

    let new_authority = Pubkey::new_unique();
    let user_transfer_authority = Keypair::new();
    let transaction_error = stake_pool_accounts
        .withdraw_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &validator_stake_account.stake_account,
            &new_authority,
            tokens_to_burn,
        )
        .await
        .unwrap();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = TokenError::OwnerMismatch as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!(
            "Wrong error occurs while try to do withdraw without token delegation for burn before"
        ),
    }
}

#[tokio::test]
async fn fail_with_low_delegation() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
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

    let deposit_info = simple_deposit(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
        &validator_stake_account,
        TEST_STAKE_AMOUNT,
    )
    .await
    .unwrap();

    let tokens_to_burn = deposit_info.pool_tokens / 4;

    let user_transfer_authority = Keypair::new();
    // Delegate tokens for burning
    delegate_tokens(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &deposit_info.pool_account.pubkey(),
        &deposit_info.authority,
        &user_transfer_authority.pubkey(),
        1,
    )
    .await;

    // Create stake account to withdraw to
    let user_stake_recipient = Keypair::new();
    create_blank_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake_recipient,
    )
    .await;

    let new_authority = Pubkey::new_unique();
    let transaction_error = stake_pool_accounts
        .withdraw_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &validator_stake_account.stake_account,
            &new_authority,
            tokens_to_burn,
        )
        .await
        .unwrap();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = TokenError::InsufficientFunds as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!(
            "Wrong error occurs while try to do withdraw with not enough delegated tokens to burn"
        ),
    }
}

#[tokio::test]
async fn fail_overdraw_validator() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        _validator_stake_account,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_burn,
    ) = setup().await;

    let validator_stake_account = simple_add_validator_to_pool(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
    )
    .await;

    let new_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .withdraw_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &validator_stake_account.stake_account,
            &new_authority,
            tokens_to_burn,
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(StakePoolError::StakeLamportsNotEqualToMinimum as u32)
        ),
    );
}

#[tokio::test]
async fn success_with_reserve() {
    let mut context = program_test().start_with_context().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    let initial_reserve_lamports = 1;
    stake_pool_accounts
        .initialize_stake_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            initial_reserve_lamports,
        )
        .await
        .unwrap();

    let validator_stake = simple_add_validator_to_pool(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts,
    )
    .await;

    let deposit_lamports = TEST_STAKE_AMOUNT;
    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake_program::StakeState>());

    let deposit_info = simple_deposit(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts,
        &validator_stake,
        deposit_lamports,
    )
    .await
    .unwrap();

    // decrease some stake
    let error = stake_pool_accounts
        .decrease_validator_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            deposit_lamports - 1,
        )
        .await;
    assert!(error.is_none());

    // warp forward to deactivation
    let first_normal_slot = context.genesis_config().epoch_schedule.first_normal_slot;
    let slots_per_epoch = context.genesis_config().epoch_schedule.slots_per_epoch;
    context
        .warp_to_slot(first_normal_slot + slots_per_epoch)
        .unwrap();

    // update to merge deactivated stake into reserve
    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &[validator_stake.vote.pubkey()],
            false,
        )
        .await;

    // Delegate tokens for burning during withdraw
    let user_transfer_authority = Keypair::new();
    delegate_tokens(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &deposit_info.pool_account.pubkey(),
        &deposit_info.authority,
        &user_transfer_authority.pubkey(),
        deposit_info.pool_tokens,
    )
    .await;

    // Withdraw directly from reserve, fail because some stake left
    let withdraw_destination = Keypair::new();
    let withdraw_destination_authority = Pubkey::new_unique();
    let initial_stake_lamports = create_blank_stake_account(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &withdraw_destination,
    )
    .await;
    let error = stake_pool_accounts
        .withdraw_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &withdraw_destination.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &stake_pool_accounts.reserve_stake.pubkey(),
            &withdraw_destination_authority,
            deposit_info.pool_tokens,
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(StakePoolError::StakeLamportsNotEqualToMinimum as u32)
        )
    );

    // decrease rest of stake
    let error = stake_pool_accounts
        .decrease_validator_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            stake_rent + 1,
        )
        .await;
    assert!(error.is_none());

    // warp forward to deactivation
    context
        .warp_to_slot(first_normal_slot + 2 * slots_per_epoch)
        .unwrap();

    // update to merge deactivated stake into reserve
    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &[validator_stake.vote.pubkey()],
            false,
        )
        .await;

    // now it works
    let error = stake_pool_accounts
        .withdraw_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &withdraw_destination.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &stake_pool_accounts.reserve_stake.pubkey(),
            &withdraw_destination_authority,
            deposit_info.pool_tokens,
        )
        .await;
    assert!(error.is_none());

    // Check tokens burned
    let user_token_balance = get_token_balance(
        &mut context.banks_client,
        &deposit_info.pool_account.pubkey(),
    )
    .await;
    assert_eq!(user_token_balance, 0);

    // Check reserve stake account balance
    let reserve_stake_account = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.reserve_stake.pubkey(),
    )
    .await;
    let stake_state =
        deserialize::<stake_program::StakeState>(&reserve_stake_account.data).unwrap();
    let meta = stake_state.meta().unwrap();
    assert_eq!(
        initial_reserve_lamports + meta.rent_exempt_reserve,
        reserve_stake_account.lamports
    );

    // Check user recipient stake account balance
    let user_stake_recipient_account =
        get_account(&mut context.banks_client, &withdraw_destination.pubkey()).await;
    assert_eq!(
        user_stake_recipient_account.lamports,
        initial_stake_lamports + deposit_info.stake_lamports + stake_rent
    );
}

#[tokio::test]
async fn success_with_preferred_validator() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        validator_stake,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_burn,
    ) = setup().await;

    stake_pool_accounts
        .set_preferred_validator(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            instruction::PreferredValidatorType::Withdraw,
            Some(validator_stake.vote.pubkey()),
        )
        .await;

    let new_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .withdraw_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &validator_stake.stake_account,
            &new_authority,
            tokens_to_burn,
        )
        .await;
    assert!(error.is_none());
}

#[tokio::test]
async fn fail_with_wrong_preferred_withdraw() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        validator_stake,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_burn,
    ) = setup().await;

    let preferred_validator = simple_add_validator_to_pool(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
    )
    .await;

    stake_pool_accounts
        .set_preferred_validator(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            instruction::PreferredValidatorType::Withdraw,
            Some(preferred_validator.vote.pubkey()),
        )
        .await;

    // preferred is empty, this works
    let new_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .withdraw_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &validator_stake.stake_account,
            &new_authority,
            tokens_to_burn,
        )
        .await;
    assert!(error.is_none());

    // deposit into preferred, then fail
    let _preferred_deposit = simple_deposit(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
        &preferred_validator,
        TEST_STAKE_AMOUNT,
    )
    .await
    .unwrap();

    // Create stake account to withdraw to
    let user_stake_recipient = Keypair::new();
    create_blank_stake_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_stake_recipient,
    )
    .await;

    let error = stake_pool_accounts
        .withdraw_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &validator_stake.stake_account,
            &new_authority,
            tokens_to_burn,
        )
        .await
        .unwrap()
        .unwrap();
    match error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            assert_eq!(
                error_index,
                StakePoolError::IncorrectWithdrawVoteAddress as u32
            );
        }
        _ => panic!("Wrong error occurs while try to make a deposit with wrong stake program ID"),
    }
}

#[tokio::test]
async fn success_withdraw_from_transient() {
    let mut context = program_test().start_with_context().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    let initial_reserve_lamports = 1;
    stake_pool_accounts
        .initialize_stake_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            initial_reserve_lamports,
        )
        .await
        .unwrap();

    // add a preferred withdraw validator, keep it empty, to be sure that this works
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
            instruction::PreferredValidatorType::Withdraw,
            Some(preferred_validator.vote.pubkey()),
        )
        .await;

    let validator_stake = simple_add_validator_to_pool(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts,
    )
    .await;

    let deposit_lamports = TEST_STAKE_AMOUNT;
    let rent = context.banks_client.get_rent().await.unwrap();
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake_program::StakeState>());

    let deposit_info = simple_deposit(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts,
        &validator_stake,
        deposit_lamports,
    )
    .await
    .unwrap();

    // Delegate tokens for burning during withdraw
    let user_transfer_authority = Keypair::new();
    delegate_tokens(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &deposit_info.pool_account.pubkey(),
        &deposit_info.authority,
        &user_transfer_authority.pubkey(),
        deposit_info.pool_tokens,
    )
    .await;

    // decrease minimum stake
    let error = stake_pool_accounts
        .decrease_validator_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            stake_rent + 1,
        )
        .await;
    assert!(error.is_none());

    let withdraw_destination = Keypair::new();
    let withdraw_destination_authority = Pubkey::new_unique();
    let _initial_stake_lamports = create_blank_stake_account(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &withdraw_destination,
    )
    .await;

    // fail withdrawing from transient, still a lamport in the validator stake account
    let error = stake_pool_accounts
        .withdraw_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &withdraw_destination.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &validator_stake.transient_stake_account,
            &withdraw_destination_authority,
            deposit_info.pool_tokens / 2,
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(StakePoolError::InvalidStakeAccountAddress as u32)
        )
    );

    // warp forward to deactivation
    let first_normal_slot = context.genesis_config().epoch_schedule.first_normal_slot;
    let slots_per_epoch = context.genesis_config().epoch_schedule.slots_per_epoch;
    context
        .warp_to_slot(first_normal_slot + slots_per_epoch)
        .unwrap();

    // update to merge deactivated stake into reserve
    stake_pool_accounts
        .update_all(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &[
                preferred_validator.vote.pubkey(),
                validator_stake.vote.pubkey(),
            ],
            false,
        )
        .await;

    // decrease rest of stake
    let error = stake_pool_accounts
        .decrease_validator_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &validator_stake.stake_account,
            &validator_stake.transient_stake_account,
            deposit_lamports - 1,
        )
        .await;
    assert!(error.is_none());

    // nothing left in the validator stake account (or any others), so withdrawing
    // from the transient account is ok!
    let error = stake_pool_accounts
        .withdraw_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &withdraw_destination.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &validator_stake.transient_stake_account,
            &withdraw_destination_authority,
            deposit_info.pool_tokens / 4,
        )
        .await;
    assert!(error.is_none());
}
