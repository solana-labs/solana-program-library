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
        stake, sysvar,
    },
    solana_program_test::*,
    solana_sdk::{
        signature::{Keypair, Signer},
        transaction::{Transaction, TransactionError},
        transport::TransportError,
    },
    spl_stake_pool::{error::StakePoolError, id, instruction, minimum_stake_lamports, state},
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

    let deposit_info = simple_deposit_stake(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
        &validator_stake_account,
        TEST_STAKE_AMOUNT,
    )
    .await
    .unwrap();

    let tokens_to_withdraw = deposit_info.pool_tokens / 4;

    // Delegate tokens for withdrawing
    let user_transfer_authority = Keypair::new();
    delegate_tokens(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &deposit_info.pool_account.pubkey(),
        &deposit_info.authority,
        &user_transfer_authority.pubkey(),
        tokens_to_withdraw,
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
        tokens_to_withdraw,
    )
}

#[tokio::test]
async fn success() {
    _success(SuccessTestType::Success).await;
}

#[tokio::test]
async fn success_with_closed_manager_fee_account() {
    _success(SuccessTestType::UninitializedManagerFee).await;
}

enum SuccessTestType {
    Success,
    UninitializedManagerFee,
}

async fn _success(test_type: SuccessTestType) {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        validator_stake_account,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_withdraw,
    ) = setup().await;

    // Save stake pool state before withdrawal
    let stake_pool_before =
        get_account(&mut banks_client, &stake_pool_accounts.stake_pool.pubkey()).await;
    let stake_pool_before =
        try_from_slice_unchecked::<state::StakePool>(stake_pool_before.data.as_slice()).unwrap();

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

    // Save pool fee token balance
    let pool_fee_balance_before = get_token_balance(
        &mut banks_client,
        &stake_pool_accounts.pool_fee_account.pubkey(),
    )
    .await;

    let destination_keypair = Keypair::new();
    create_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &destination_keypair,
        &stake_pool_accounts.pool_mint.pubkey(),
        &Keypair::new().pubkey(),
    )
    .await
    .unwrap();

    if let SuccessTestType::UninitializedManagerFee = test_type {
        transfer_spl_tokens(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &stake_pool_accounts.pool_fee_account.pubkey(),
            &destination_keypair.pubkey(),
            &stake_pool_accounts.manager,
            pool_fee_balance_before,
        )
        .await;
        // Check that the account cannot be frozen due to lack of
        // freeze authority.
        let transaction_error = freeze_token_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &stake_pool_accounts.pool_fee_account.pubkey(),
            &stake_pool_accounts.pool_mint.pubkey(),
            &stake_pool_accounts.manager,
        )
        .await
        .unwrap_err();

        match transaction_error {
            TransportError::TransactionError(TransactionError::InstructionError(_, error)) => {
                assert_eq!(error, InstructionError::Custom(0x10));
            }
            _ => panic!("Wrong error occurs while try to withdraw with wrong stake program ID"),
        }
        close_token_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &stake_pool_accounts.pool_fee_account.pubkey(),
            &destination_keypair.pubkey(),
            &stake_pool_accounts.manager,
        )
        .await
        .unwrap();
    }

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
            tokens_to_withdraw,
        )
        .await;
    assert!(error.is_none());

    // Check pool stats
    let stake_pool = get_account(&mut banks_client, &stake_pool_accounts.stake_pool.pubkey()).await;
    let stake_pool =
        try_from_slice_unchecked::<state::StakePool>(stake_pool.data.as_slice()).unwrap();
    // first and only deposit, lamports:pool 1:1
    let tokens_withdrawal_fee = match test_type {
        SuccessTestType::Success => {
            stake_pool_accounts.calculate_withdrawal_fee(tokens_to_withdraw)
        }
        _ => 0,
    };
    let tokens_burnt = tokens_to_withdraw - tokens_withdrawal_fee;
    assert_eq!(
        stake_pool.total_lamports,
        stake_pool_before.total_lamports - tokens_burnt
    );
    assert_eq!(
        stake_pool.pool_token_supply,
        stake_pool_before.pool_token_supply - tokens_burnt
    );

    if let SuccessTestType::Success = test_type {
        // Check manager received withdrawal fee
        let pool_fee_balance = get_token_balance(
            &mut banks_client,
            &stake_pool_accounts.pool_fee_account.pubkey(),
        )
        .await;
        assert_eq!(
            pool_fee_balance,
            pool_fee_balance_before + tokens_withdrawal_fee,
        );
    }

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
        validator_stake_item_before.stake_lamports() - tokens_burnt
    );
    assert_eq!(
        validator_stake_item.active_stake_lamports,
        validator_stake_item.stake_lamports(),
    );

    // Check tokens used
    let user_token_balance =
        get_token_balance(&mut banks_client, &deposit_info.pool_account.pubkey()).await;
    assert_eq!(
        user_token_balance,
        user_token_balance_before - tokens_to_withdraw
    );

    // Check validator stake account balance
    let validator_stake_account =
        get_account(&mut banks_client, &validator_stake_account.stake_account).await;
    let stake_state =
        deserialize::<stake::state::StakeState>(&validator_stake_account.data).unwrap();
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
        initial_stake_lamports + tokens_burnt
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
        AccountMeta::new(stake_pool_accounts.pool_fee_account.pubkey(), false),
        AccountMeta::new(stake_pool_accounts.pool_mint.pubkey(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(wrong_stake_program, false),
    ];
    let instruction = Instruction {
        program_id: id(),
        accounts,
        data: instruction::StakePoolInstruction::WithdrawStake(tokens_to_burn)
            .try_to_vec()
            .unwrap(),
    };

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer, &user_transfer_authority],
        recent_blockhash,
    );
    #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
    let transaction_error = banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .into();

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
        &[instruction::withdraw_stake(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.validator_list.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &validator_stake_account.stake_account,
            &user_stake_recipient.pubkey(),
            &new_authority,
            &user_transfer_authority.pubkey(),
            &deposit_info.pool_account.pubkey(),
            &stake_pool_accounts.pool_fee_account.pubkey(),
            &stake_pool_accounts.pool_mint.pubkey(),
            &wrong_token_program.pubkey(),
            tokens_to_burn,
        )],
        Some(&payer.pubkey()),
        &[&payer, &user_transfer_authority],
        recent_blockhash,
    );
    #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
    let transaction_error = banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .into();

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
        validator_stake,
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
            &validator_stake.stake_account,
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
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_withdraw,
    ) = setup().await;

    let unknown_stake = create_unknown_validator_stake(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts.stake_pool.pubkey(),
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
            &unknown_stake.stake_account,
            &new_authority,
            tokens_to_withdraw,
        )
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(StakePoolError::ValidatorNotFound as u32)
        )
    );
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

    let latest_blockhash = banks_client.get_latest_blockhash().await.unwrap();

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

    let deposit_info = simple_deposit_stake(
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

    let deposit_info = simple_deposit_stake(
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
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeState>());

    let deposit_info = simple_deposit_stake(
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
            validator_stake.transient_stake_seed,
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

    // Delegate tokens for using for withdrawal
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
            validator_stake.transient_stake_seed,
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

    // first and only deposit, lamports:pool 1:1
    let stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let stake_pool =
        try_from_slice_unchecked::<state::StakePool>(stake_pool.data.as_slice()).unwrap();
    // the entire deposit is actually stake since it isn't activated, so only
    // the stake deposit fee is charged
    let deposit_fee = stake_pool
        .calc_pool_tokens_stake_deposit_fee(stake_rent + deposit_info.stake_lamports)
        .unwrap();
    assert_eq!(
        deposit_info.stake_lamports + stake_rent - deposit_fee,
        deposit_info.pool_tokens,
        "stake {} rent {} deposit fee {} pool tokens {}",
        deposit_info.stake_lamports,
        stake_rent,
        deposit_fee,
        deposit_info.pool_tokens
    );

    let withdrawal_fee = stake_pool_accounts.calculate_withdrawal_fee(deposit_info.pool_tokens);

    // Check tokens used
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
    let stake_state = deserialize::<stake::state::StakeState>(&reserve_stake_account.data).unwrap();
    let meta = stake_state.meta().unwrap();
    assert_eq!(
        initial_reserve_lamports + meta.rent_exempt_reserve + withdrawal_fee + deposit_fee,
        reserve_stake_account.lamports
    );

    // Check user recipient stake account balance
    let user_stake_recipient_account =
        get_account(&mut context.banks_client, &withdraw_destination.pubkey()).await;
    assert_eq!(
        user_stake_recipient_account.lamports,
        initial_stake_lamports + deposit_info.stake_lamports + stake_rent
            - withdrawal_fee
            - deposit_fee
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
    let _preferred_deposit = simple_deposit_stake(
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
    let stake_rent = rent.minimum_balance(std::mem::size_of::<stake::state::StakeState>());

    let deposit_info = simple_deposit_stake(
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
            validator_stake.transient_stake_seed,
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
            validator_stake.transient_stake_seed,
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

#[tokio::test]
async fn success_withdraw_all_fee_tokens() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        validator_stake_account,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_withdraw,
    ) = setup().await;

    // move tokens to fee account
    transfer_spl_tokens(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &deposit_info.pool_account.pubkey(),
        &stake_pool_accounts.pool_fee_account.pubkey(),
        &user_transfer_authority,
        tokens_to_withdraw,
    )
    .await;

    let fee_tokens = get_token_balance(
        &mut banks_client,
        &stake_pool_accounts.pool_fee_account.pubkey(),
    )
    .await;

    let user_transfer_authority = Keypair::new();
    delegate_tokens(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts.pool_fee_account.pubkey(),
        &stake_pool_accounts.manager,
        &user_transfer_authority.pubkey(),
        fee_tokens,
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
            &stake_pool_accounts.pool_fee_account.pubkey(),
            &validator_stake_account.stake_account,
            &new_authority,
            fee_tokens,
        )
        .await;
    assert!(error.is_none());

    // Check balance is 0
    let fee_tokens = get_token_balance(
        &mut banks_client,
        &stake_pool_accounts.pool_fee_account.pubkey(),
    )
    .await;
    assert_eq!(fee_tokens, 0);
}

#[tokio::test]
async fn success_empty_out_stake_with_fee() {
    let (
        mut banks_client,
        payer,
        recent_blockhash,
        stake_pool_accounts,
        _,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_withdraw,
    ) = setup().await;

    // add another validator and deposit into it
    let other_validator_stake_account = simple_add_validator_to_pool(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
    )
    .await;

    let other_deposit_info = simple_deposit_stake(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
        &other_validator_stake_account,
        TEST_STAKE_AMOUNT,
    )
    .await
    .unwrap();

    // move tokens to new account
    transfer_spl_tokens(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &deposit_info.pool_account.pubkey(),
        &other_deposit_info.pool_account.pubkey(),
        &user_transfer_authority,
        tokens_to_withdraw,
    )
    .await;

    let user_tokens =
        get_token_balance(&mut banks_client, &other_deposit_info.pool_account.pubkey()).await;

    let user_transfer_authority = Keypair::new();
    delegate_tokens(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &other_deposit_info.pool_account.pubkey(),
        &other_deposit_info.authority,
        &user_transfer_authority.pubkey(),
        user_tokens,
    )
    .await;

    // calculate exactly how much to withdraw, given the fee, to get the account
    // down to 0, using an inverse fee calculation
    let validator_stake_account = get_account(
        &mut banks_client,
        &other_validator_stake_account.stake_account,
    )
    .await;
    let stake_state =
        deserialize::<stake::state::StakeState>(&validator_stake_account.data).unwrap();
    let meta = stake_state.meta().unwrap();
    let lamports_to_withdraw = validator_stake_account.lamports - minimum_stake_lamports(&meta);
    let stake_pool_account =
        get_account(&mut banks_client, &stake_pool_accounts.stake_pool.pubkey()).await;
    let stake_pool =
        try_from_slice_unchecked::<state::StakePool>(stake_pool_account.data.as_slice()).unwrap();
    let fee = stake_pool.stake_withdrawal_fee;
    let inverse_fee = state::Fee {
        numerator: fee.denominator - fee.numerator,
        denominator: fee.denominator,
    };
    let pool_tokens_to_withdraw =
        lamports_to_withdraw * inverse_fee.denominator / inverse_fee.numerator;

    let new_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .withdraw_stake(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &other_deposit_info.pool_account.pubkey(),
            &other_validator_stake_account.stake_account,
            &new_authority,
            pool_tokens_to_withdraw,
        )
        .await;
    assert!(error.is_none());

    // Check balance of validator stake account is MINIMUM + rent-exemption
    let validator_stake_account = get_account(
        &mut banks_client,
        &other_validator_stake_account.stake_account,
    )
    .await;
    let stake_state =
        deserialize::<stake::state::StakeState>(&validator_stake_account.data).unwrap();
    let meta = stake_state.meta().unwrap();
    assert_eq!(
        validator_stake_account.lamports,
        minimum_stake_lamports(&meta)
    );
}
