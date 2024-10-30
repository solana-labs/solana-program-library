#![allow(clippy::arithmetic_side_effects)]
#![cfg(feature = "test-sbf")]

mod helpers;

use {
    helpers::*,
    solana_program::{
        borsh1::try_from_slice_unchecked,
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
    spl_stake_pool::{error::StakePoolError, id, instruction, state},
    spl_token::error::TokenError,
    test_case::test_case,
};

#[test_case(spl_token::id(); "token")]
#[test_case(spl_token_2022::id(); "token-2022")]
#[tokio::test]
async fn success(token_program_id: Pubkey) {
    _success(token_program_id, SuccessTestType::Success).await;
}

#[tokio::test]
async fn success_with_closed_manager_fee_account() {
    _success(spl_token::id(), SuccessTestType::UninitializedManagerFee).await;
}

enum SuccessTestType {
    Success,
    UninitializedManagerFee,
}

async fn _success(token_program_id: Pubkey, test_type: SuccessTestType) {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake_account,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_withdraw,
    ) = setup_for_withdraw(token_program_id, 0).await;

    // Save stake pool state before withdrawal
    let stake_pool_before = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let stake_pool_before =
        try_from_slice_unchecked::<state::StakePool>(stake_pool_before.data.as_slice()).unwrap();

    // Check user recipient stake account balance
    let initial_stake_lamports =
        get_account(&mut context.banks_client, &user_stake_recipient.pubkey())
            .await
            .lamports;

    // Save validator stake account record before withdrawal
    let validator_list = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let validator_list =
        try_from_slice_unchecked::<state::ValidatorList>(validator_list.data.as_slice()).unwrap();
    let validator_stake_item_before = validator_list
        .find(&validator_stake_account.vote.pubkey())
        .unwrap();

    // Save user token balance
    let user_token_balance_before = get_token_balance(
        &mut context.banks_client,
        &deposit_info.pool_account.pubkey(),
    )
    .await;

    // Save pool fee token balance
    let pool_fee_balance_before = get_token_balance(
        &mut context.banks_client,
        &stake_pool_accounts.pool_fee_account.pubkey(),
    )
    .await;

    let destination_keypair = Keypair::new();
    create_token_account(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts.token_program_id,
        &destination_keypair,
        &stake_pool_accounts.pool_mint.pubkey(),
        &Keypair::new(),
        &[],
    )
    .await
    .unwrap();

    if let SuccessTestType::UninitializedManagerFee = test_type {
        transfer_spl_tokens(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &stake_pool_accounts.token_program_id,
            &stake_pool_accounts.pool_fee_account.pubkey(),
            &stake_pool_accounts.pool_mint.pubkey(),
            &destination_keypair.pubkey(),
            &stake_pool_accounts.manager,
            pool_fee_balance_before,
            stake_pool_accounts.pool_decimals,
        )
        .await;
        // Check that the account cannot be frozen due to lack of
        // freeze authority.
        let transaction_error = freeze_token_account(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &stake_pool_accounts.token_program_id,
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
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &stake_pool_accounts.token_program_id,
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
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &validator_stake_account.stake_account,
            &new_authority,
            tokens_to_withdraw,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    // Check pool stats
    let stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
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
            &mut context.banks_client,
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
        &mut context.banks_client,
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let validator_list =
        try_from_slice_unchecked::<state::ValidatorList>(validator_list.data.as_slice()).unwrap();
    let validator_stake_item = validator_list
        .find(&validator_stake_account.vote.pubkey())
        .unwrap();
    assert_eq!(
        validator_stake_item.stake_lamports().unwrap(),
        validator_stake_item_before.stake_lamports().unwrap() - tokens_burnt
    );
    assert_eq!(
        u64::from(validator_stake_item.active_stake_lamports),
        validator_stake_item.stake_lamports().unwrap(),
    );

    // Check tokens used
    let user_token_balance = get_token_balance(
        &mut context.banks_client,
        &deposit_info.pool_account.pubkey(),
    )
    .await;
    assert_eq!(
        user_token_balance,
        user_token_balance_before - tokens_to_withdraw
    );

    // Check validator stake account balance
    let validator_stake_account = get_account(
        &mut context.banks_client,
        &validator_stake_account.stake_account,
    )
    .await;
    assert_eq!(
        validator_stake_account.lamports,
        u64::from(validator_stake_item.active_stake_lamports)
    );

    // Check user recipient stake account balance
    let user_stake_recipient_account =
        get_account(&mut context.banks_client, &user_stake_recipient.pubkey()).await;
    assert_eq!(
        user_stake_recipient_account.lamports,
        initial_stake_lamports + tokens_burnt
    );
}

#[tokio::test]
async fn fail_with_wrong_stake_program() {
    let (
        context,
        stake_pool_accounts,
        validator_stake_account,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_burn,
    ) = setup_for_withdraw(spl_token::id(), 0).await;

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
        data: borsh::to_vec(&instruction::StakePoolInstruction::WithdrawStake(
            tokens_to_burn,
        ))
        .unwrap(),
    };

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &[&context.payer, &user_transfer_authority],
        context.last_blockhash,
    );
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
        _ => panic!("Wrong error occurs while try to withdraw with wrong stake program ID"),
    }
}

#[tokio::test]
async fn fail_with_wrong_withdraw_authority() {
    let (
        mut context,
        mut stake_pool_accounts,
        validator_stake_account,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_burn,
    ) = setup_for_withdraw(spl_token::id(), 0).await;

    let new_authority = Pubkey::new_unique();
    stake_pool_accounts.withdraw_authority = Keypair::new().pubkey();

    let transaction_error = stake_pool_accounts
        .withdraw_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
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
        context,
        stake_pool_accounts,
        validator_stake_account,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_burn,
    ) = setup_for_withdraw(spl_token::id(), 0).await;

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
        Some(&context.payer.pubkey()),
        &[&context.payer, &user_transfer_authority],
        context.last_blockhash,
    );
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
        _ => panic!("Wrong error occurs while try to withdraw with wrong token program ID"),
    }
}

#[tokio::test]
async fn fail_with_wrong_validator_list() {
    let (
        mut context,
        mut stake_pool_accounts,
        validator_stake,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_burn,
    ) = setup_for_withdraw(spl_token::id(), 0).await;

    let new_authority = Pubkey::new_unique();
    stake_pool_accounts.validator_list = Keypair::new();

    let transaction_error = stake_pool_accounts
        .withdraw_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
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
        mut context,
        stake_pool_accounts,
        _,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_withdraw,
    ) = setup_for_withdraw(spl_token::id(), 0).await;

    let unknown_stake = create_unknown_validator_stake(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts.stake_pool.pubkey(),
        0,
    )
    .await;

    let new_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .withdraw_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
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
        mut context,
        stake_pool_accounts,
        validator_stake_account,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_burn,
    ) = setup_for_withdraw(spl_token::id(), 0).await;

    let new_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .withdraw_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &validator_stake_account.stake_account,
            &new_authority,
            tokens_to_burn / 2,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    let latest_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    // Delegate tokens for burning
    delegate_tokens(
        &mut context.banks_client,
        &context.payer,
        &latest_blockhash,
        &stake_pool_accounts.token_program_id,
        &deposit_info.pool_account.pubkey(),
        &deposit_info.authority,
        &user_transfer_authority.pubkey(),
        tokens_to_burn / 2,
    )
    .await;

    let transaction_error = stake_pool_accounts
        .withdraw_stake(
            &mut context.banks_client,
            &context.payer,
            &latest_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &validator_stake_account.stake_account,
            &new_authority,
            tokens_to_burn / 2,
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
    let (
        mut context,
        stake_pool_accounts,
        validator_stake_account,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_burn,
    ) = setup_for_withdraw(spl_token::id(), 0).await;

    revoke_tokens(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_accounts.token_program_id,
        &deposit_info.pool_account.pubkey(),
        &deposit_info.authority,
    )
    .await;

    let new_authority = Pubkey::new_unique();
    let transaction_error = stake_pool_accounts
        .withdraw_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
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
async fn fail_with_not_enough_tokens() {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake_account,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_burn,
    ) = setup_for_withdraw(spl_token::id(), 0).await;

    let last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .unwrap();

    // Empty validator stake account
    let empty_stake_account = simple_add_validator_to_pool(
        &mut context.banks_client,
        &context.payer,
        &last_blockhash,
        &stake_pool_accounts,
        None,
    )
    .await;

    let new_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .withdraw_stake(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &empty_stake_account.stake_account,
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

    // revoked delegation
    revoke_tokens(
        &mut context.banks_client,
        &context.payer,
        &last_blockhash,
        &stake_pool_accounts.token_program_id,
        &deposit_info.pool_account.pubkey(),
        &deposit_info.authority,
    )
    .await;

    let last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&last_blockhash)
        .await
        .unwrap();

    // generate a new authority each time to make each transaction unique
    let new_authority = Pubkey::new_unique();
    let transaction_error = stake_pool_accounts
        .withdraw_stake(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
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
        transaction_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(TokenError::OwnerMismatch as u32),
        )
    );

    // Delegate few tokens for burning
    delegate_tokens(
        &mut context.banks_client,
        &context.payer,
        &last_blockhash,
        &stake_pool_accounts.token_program_id,
        &deposit_info.pool_account.pubkey(),
        &deposit_info.authority,
        &user_transfer_authority.pubkey(),
        1,
    )
    .await;

    let last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&last_blockhash)
        .await
        .unwrap();

    // generate a new authority each time to make each transaction unique
    let new_authority = Pubkey::new_unique();
    let transaction_error = stake_pool_accounts
        .withdraw_stake(
            &mut context.banks_client,
            &context.payer,
            &last_blockhash,
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
        transaction_error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(TokenError::InsufficientFunds as u32),
        )
    );
}

#[test_case(spl_token::id(); "token")]
#[test_case(spl_token_2022::id(); "token-2022")]
#[tokio::test]
async fn success_with_slippage(token_program_id: Pubkey) {
    let (
        mut context,
        stake_pool_accounts,
        validator_stake_account,
        deposit_info,
        user_transfer_authority,
        user_stake_recipient,
        tokens_to_withdraw,
    ) = setup_for_withdraw(token_program_id, 0).await;

    // Save user token balance
    let user_token_balance_before = get_token_balance(
        &mut context.banks_client,
        &deposit_info.pool_account.pubkey(),
    )
    .await;

    // first and only deposit, lamports:pool 1:1
    let tokens_withdrawal_fee = stake_pool_accounts.calculate_withdrawal_fee(tokens_to_withdraw);
    let received_lamports = tokens_to_withdraw - tokens_withdrawal_fee;

    let new_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .withdraw_stake_with_slippage(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &validator_stake_account.stake_account,
            &new_authority,
            tokens_to_withdraw,
            received_lamports + 1,
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(StakePoolError::ExceededSlippage as u32)
        )
    );

    let error = stake_pool_accounts
        .withdraw_stake_with_slippage(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &user_stake_recipient.pubkey(),
            &user_transfer_authority,
            &deposit_info.pool_account.pubkey(),
            &validator_stake_account.stake_account,
            &new_authority,
            tokens_to_withdraw,
            received_lamports,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    // Check tokens used
    let user_token_balance = get_token_balance(
        &mut context.banks_client,
        &deposit_info.pool_account.pubkey(),
    )
    .await;
    assert_eq!(
        user_token_balance,
        user_token_balance_before - tokens_to_withdraw
    );
}
