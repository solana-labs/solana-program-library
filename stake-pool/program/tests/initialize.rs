#![cfg(feature = "test-bpf")]

mod helpers;

use {
    borsh::BorshSerialize,
    helpers::*,
    solana_program::{
        borsh::{get_instance_packed_len, get_packed_len, try_from_slice_unchecked},
        hash::Hash,
        instruction::{AccountMeta, Instruction},
        program_pack::Pack,
        pubkey::Pubkey,
        system_instruction, sysvar,
    },
    solana_program_test::*,
    solana_sdk::{
        instruction::InstructionError, signature::Keypair, signature::Signer,
        transaction::Transaction, transaction::TransactionError, transport::TransportError,
    },
    spl_stake_pool::{error, id, instruction, stake_program, state},
};

async fn create_required_accounts(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    stake_pool_accounts: &StakePoolAccounts,
) {
    create_mint(
        banks_client,
        payer,
        recent_blockhash,
        &stake_pool_accounts.pool_mint,
        &stake_pool_accounts.withdraw_authority,
    )
    .await
    .unwrap();

    create_token_account(
        banks_client,
        payer,
        recent_blockhash,
        &stake_pool_accounts.pool_fee_account,
        &stake_pool_accounts.pool_mint.pubkey(),
        &stake_pool_accounts.manager.pubkey(),
    )
    .await
    .unwrap();

    create_independent_stake_account(
        banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts.reserve_stake,
        &stake_program::Authorized {
            staker: stake_pool_accounts.withdraw_authority,
            withdrawer: stake_pool_accounts.withdraw_authority,
        },
        &stake_program::Lockup::default(),
        1,
    )
    .await;
}

#[tokio::test]
async fn success() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash, 1)
        .await
        .unwrap();

    // Stake pool now exists
    let stake_pool = get_account(&mut banks_client, &stake_pool_accounts.stake_pool.pubkey()).await;
    assert_eq!(stake_pool.data.len(), get_packed_len::<state::StakePool>());
    assert_eq!(stake_pool.owner, id());

    // Validator stake list storage initialized
    let validator_list = get_account(
        &mut banks_client,
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let validator_list =
        try_from_slice_unchecked::<state::ValidatorList>(validator_list.data.as_slice()).unwrap();
    assert!(validator_list.header.is_valid());
}

#[tokio::test]
async fn fail_double_initialize() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash, 1)
        .await
        .unwrap();

    let latest_blockhash = banks_client.get_recent_blockhash().await.unwrap();

    let mut second_stake_pool_accounts = StakePoolAccounts::new();
    second_stake_pool_accounts.stake_pool = stake_pool_accounts.stake_pool;

    let transaction_error = second_stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &latest_blockhash, 1)
        .await
        .err()
        .unwrap();
    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = error::StakePoolError::AlreadyInUse as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to initialize already initialized stake pool"),
    }
}

#[tokio::test]
async fn fail_with_already_initialized_validator_list() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash, 1)
        .await
        .unwrap();

    let latest_blockhash = banks_client.get_recent_blockhash().await.unwrap();

    let mut second_stake_pool_accounts = StakePoolAccounts::new();
    second_stake_pool_accounts.validator_list = stake_pool_accounts.validator_list;

    let transaction_error = second_stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &latest_blockhash, 1)
        .await
        .err()
        .unwrap();
    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = error::StakePoolError::AlreadyInUse as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to initialize stake pool with already initialized stake list storage"),
    }
}

#[tokio::test]
async fn fail_with_high_fee() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let mut stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts.fee = state::Fee {
        numerator: 100001,
        denominator: 100000,
    };

    let transaction_error = stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash, 1)
        .await
        .err()
        .unwrap();
    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = error::StakePoolError::FeeTooHigh as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to initialize stake pool with high fee"),
    }
}

#[tokio::test]
async fn fail_with_high_withdrawal_fee() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let mut stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts.withdrawal_fee = state::Fee {
        numerator: 100_001,
        denominator: 100_000,
    };

    let transaction_error = stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash, 1)
        .await
        .err()
        .unwrap();
    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = error::StakePoolError::FeeTooHigh as u32;
            assert_eq!(error_index, program_error);
        }
        _ => {
            panic!("Wrong error occurs while try to initialize stake pool with high withdrawal fee")
        }
    }
}

#[tokio::test]
async fn fail_with_wrong_max_validators() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();

    create_required_accounts(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
    )
    .await;

    let rent = banks_client.get_rent().await.unwrap();
    let rent_stake_pool = rent.minimum_balance(get_packed_len::<state::StakePool>());
    let validator_list_size = get_instance_packed_len(&state::ValidatorList::new(
        stake_pool_accounts.max_validators - 1,
    ))
    .unwrap();
    let rent_validator_list = rent.minimum_balance(validator_list_size);

    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &stake_pool_accounts.stake_pool.pubkey(),
                rent_stake_pool,
                get_packed_len::<state::StakePool>() as u64,
                &id(),
            ),
            system_instruction::create_account(
                &payer.pubkey(),
                &stake_pool_accounts.validator_list.pubkey(),
                rent_validator_list,
                validator_list_size as u64,
                &id(),
            ),
            instruction::initialize(
                &id(),
                &stake_pool_accounts.stake_pool.pubkey(),
                &stake_pool_accounts.manager.pubkey(),
                &stake_pool_accounts.staker.pubkey(),
                &stake_pool_accounts.validator_list.pubkey(),
                &stake_pool_accounts.reserve_stake.pubkey(),
                &stake_pool_accounts.pool_mint.pubkey(),
                &stake_pool_accounts.pool_fee_account.pubkey(),
                &spl_token::id(),
                None,
                stake_pool_accounts.fee,
                stake_pool_accounts.withdrawal_fee,
                stake_pool_accounts.deposit_fee,
                stake_pool_accounts.referral_fee,
                stake_pool_accounts.max_validators,
            ),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(
        &[
            &payer,
            &stake_pool_accounts.stake_pool,
            &stake_pool_accounts.validator_list,
            &stake_pool_accounts.manager,
        ],
        recent_blockhash,
    );
    let transaction_error = banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = error::StakePoolError::UnexpectedValidatorListAccountSize as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to initialize stake pool with high fee"),
    }
}

#[tokio::test]
async fn fail_with_wrong_mint_authority() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    let wrong_mint = Keypair::new();

    create_required_accounts(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
    )
    .await;

    // create wrong mint
    create_mint(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &wrong_mint,
        &stake_pool_accounts.withdraw_authority,
    )
    .await
    .unwrap();

    let transaction_error = create_stake_pool(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts.stake_pool,
        &stake_pool_accounts.validator_list,
        &stake_pool_accounts.reserve_stake.pubkey(),
        &wrong_mint.pubkey(),
        &stake_pool_accounts.pool_fee_account.pubkey(),
        &stake_pool_accounts.manager,
        &stake_pool_accounts.staker.pubkey(),
        &None,
        &stake_pool_accounts.fee,
        &stake_pool_accounts.withdrawal_fee,
        &stake_pool_accounts.deposit_fee,
        stake_pool_accounts.referral_fee,
        &stake_pool_accounts.sol_deposit_fee,
        stake_pool_accounts.sol_referral_fee,
        stake_pool_accounts.max_validators,
    )
    .await
    .err()
    .unwrap();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = error::StakePoolError::WrongAccountMint as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to initialize stake pool with wrong mint authority of pool fee account"),
    }
}

#[tokio::test]
async fn fail_with_freeze_authority() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();

    create_required_accounts(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
    )
    .await;

    // create mint with freeze authority
    let wrong_mint = Keypair::new();
    let rent = banks_client.get_rent().await.unwrap();
    let mint_rent = rent.minimum_balance(spl_token::state::Mint::LEN);

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &wrong_mint.pubkey(),
                mint_rent,
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &wrong_mint.pubkey(),
                &stake_pool_accounts.withdraw_authority,
                Some(&stake_pool_accounts.withdraw_authority),
                0,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
        &[&payer, &wrong_mint],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    let pool_fee_account = Keypair::new();
    create_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &pool_fee_account,
        &wrong_mint.pubkey(),
        &stake_pool_accounts.manager.pubkey(),
    )
    .await
    .unwrap();

    let error = create_stake_pool(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts.stake_pool,
        &stake_pool_accounts.validator_list,
        &stake_pool_accounts.reserve_stake.pubkey(),
        &wrong_mint.pubkey(),
        &pool_fee_account.pubkey(),
        &stake_pool_accounts.manager,
        &stake_pool_accounts.staker.pubkey(),
        &None,
        &stake_pool_accounts.fee,
        &stake_pool_accounts.withdrawal_fee,
        &stake_pool_accounts.deposit_fee,
        stake_pool_accounts.referral_fee,
        &stake_pool_accounts.sol_deposit_fee,
        stake_pool_accounts.sol_referral_fee,
        stake_pool_accounts.max_validators,
    )
    .await
    .err()
    .unwrap()
    .unwrap();

    assert_eq!(
        error,
        TransactionError::InstructionError(
            2,
            InstructionError::Custom(error::StakePoolError::InvalidMintFreezeAuthority as u32),
        )
    );
}

#[tokio::test]
async fn fail_with_wrong_token_program_id() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();

    let wrong_token_program = Keypair::new();

    create_mint(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts.pool_mint,
        &stake_pool_accounts.withdraw_authority,
    )
    .await
    .unwrap();

    let rent = banks_client.get_rent().await.unwrap();

    let account_rent = rent.minimum_balance(spl_token::state::Account::LEN);
    let mut transaction = Transaction::new_with_payer(
        &[system_instruction::create_account(
            &payer.pubkey(),
            &stake_pool_accounts.pool_fee_account.pubkey(),
            account_rent,
            spl_token::state::Account::LEN as u64,
            &wrong_token_program.pubkey(),
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(
        &[&payer, &stake_pool_accounts.pool_fee_account],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    let rent_stake_pool = rent.minimum_balance(get_packed_len::<state::StakePool>());
    let validator_list_size = get_instance_packed_len(&state::ValidatorList::new(
        stake_pool_accounts.max_validators,
    ))
    .unwrap();
    let rent_validator_list = rent.minimum_balance(validator_list_size);

    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &stake_pool_accounts.stake_pool.pubkey(),
                rent_stake_pool,
                get_packed_len::<state::StakePool>() as u64,
                &id(),
            ),
            system_instruction::create_account(
                &payer.pubkey(),
                &stake_pool_accounts.validator_list.pubkey(),
                rent_validator_list,
                validator_list_size as u64,
                &id(),
            ),
            instruction::initialize(
                &id(),
                &stake_pool_accounts.stake_pool.pubkey(),
                &stake_pool_accounts.manager.pubkey(),
                &stake_pool_accounts.staker.pubkey(),
                &stake_pool_accounts.validator_list.pubkey(),
                &stake_pool_accounts.reserve_stake.pubkey(),
                &stake_pool_accounts.pool_mint.pubkey(),
                &stake_pool_accounts.pool_fee_account.pubkey(),
                &wrong_token_program.pubkey(),
                None,
                stake_pool_accounts.fee,
                stake_pool_accounts.withdrawal_fee,
                stake_pool_accounts.deposit_fee,
                stake_pool_accounts.referral_fee,
                stake_pool_accounts.max_validators,
            ),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(
        &[
            &payer,
            &stake_pool_accounts.stake_pool,
            &stake_pool_accounts.validator_list,
            &stake_pool_accounts.manager,
        ],
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
        _ => panic!(
            "Wrong error occurs while try to initialize stake pool with wrong token program ID"
        ),
    }
}

#[tokio::test]
async fn fail_with_wrong_fee_account() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();

    create_mint(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts.pool_mint,
        &stake_pool_accounts.withdraw_authority,
    )
    .await
    .unwrap();
    let rent = banks_client.get_rent().await.unwrap();
    let account_rent = rent.minimum_balance(spl_token::state::Account::LEN);

    let mut transaction = Transaction::new_with_payer(
        &[system_instruction::create_account(
            &payer.pubkey(),
            &stake_pool_accounts.pool_fee_account.pubkey(),
            account_rent,
            spl_token::state::Account::LEN as u64,
            &Keypair::new().pubkey(),
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(
        &[&payer, &stake_pool_accounts.pool_fee_account],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    let transaction_error = create_stake_pool(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts.stake_pool,
        &stake_pool_accounts.validator_list,
        &stake_pool_accounts.reserve_stake.pubkey(),
        &stake_pool_accounts.pool_mint.pubkey(),
        &stake_pool_accounts.pool_fee_account.pubkey(),
        &stake_pool_accounts.manager,
        &stake_pool_accounts.staker.pubkey(),
        &None,
        &stake_pool_accounts.fee,
        &stake_pool_accounts.withdrawal_fee,
        &stake_pool_accounts.deposit_fee,
        stake_pool_accounts.referral_fee,
        &stake_pool_accounts.sol_deposit_fee,
        stake_pool_accounts.sol_referral_fee,
        stake_pool_accounts.max_validators,
    )
    .await
    .err()
    .unwrap()
    .unwrap();

    assert_eq!(
        transaction_error,
        TransactionError::InstructionError(2, InstructionError::IncorrectProgramId)
    );
}

#[tokio::test]
async fn fail_with_wrong_withdraw_authority() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let mut stake_pool_accounts = StakePoolAccounts::new();

    stake_pool_accounts.withdraw_authority = Keypair::new().pubkey();

    let transaction_error = stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash, 1)
        .await
        .err()
        .unwrap();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = error::StakePoolError::WrongMintingAuthority as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!(
            "Wrong error occurs while try to initialize stake pool with wrong withdraw authority"
        ),
    }
}

#[tokio::test]
async fn fail_with_not_rent_exempt_pool() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();

    create_required_accounts(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
    )
    .await;

    let rent = banks_client.get_rent().await.unwrap();
    let validator_list_size = get_instance_packed_len(&state::ValidatorList::new(
        stake_pool_accounts.max_validators,
    ))
    .unwrap();
    let rent_validator_list = rent.minimum_balance(validator_list_size);

    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &stake_pool_accounts.stake_pool.pubkey(),
                1,
                get_packed_len::<state::StakePool>() as u64,
                &id(),
            ),
            system_instruction::create_account(
                &payer.pubkey(),
                &stake_pool_accounts.validator_list.pubkey(),
                rent_validator_list,
                validator_list_size as u64,
                &id(),
            ),
            instruction::initialize(
                &id(),
                &stake_pool_accounts.stake_pool.pubkey(),
                &stake_pool_accounts.manager.pubkey(),
                &stake_pool_accounts.staker.pubkey(),
                &stake_pool_accounts.validator_list.pubkey(),
                &stake_pool_accounts.reserve_stake.pubkey(),
                &stake_pool_accounts.pool_mint.pubkey(),
                &stake_pool_accounts.pool_fee_account.pubkey(),
                &spl_token::id(),
                None,
                stake_pool_accounts.fee,
                stake_pool_accounts.withdrawal_fee,
                stake_pool_accounts.deposit_fee,
                stake_pool_accounts.referral_fee,
                stake_pool_accounts.max_validators,
            ),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(
        &[
            &payer,
            &stake_pool_accounts.stake_pool,
            &stake_pool_accounts.validator_list,
            &stake_pool_accounts.manager,
        ],
        recent_blockhash,
    );
    let result = banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err()
        .unwrap();
    assert!(
        result == TransactionError::InstructionError(2, InstructionError::InvalidError,)
            || result
                == TransactionError::InstructionError(2, InstructionError::AccountNotRentExempt,)
    );
}

#[tokio::test]
async fn fail_with_not_rent_exempt_validator_list() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();

    create_required_accounts(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
    )
    .await;

    let rent = banks_client.get_rent().await.unwrap();
    let rent_stake_pool = rent.minimum_balance(get_packed_len::<state::StakePool>());
    let validator_list_size = get_instance_packed_len(&state::ValidatorList::new(
        stake_pool_accounts.max_validators,
    ))
    .unwrap();

    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &stake_pool_accounts.stake_pool.pubkey(),
                rent_stake_pool,
                get_packed_len::<state::StakePool>() as u64,
                &id(),
            ),
            system_instruction::create_account(
                &payer.pubkey(),
                &stake_pool_accounts.validator_list.pubkey(),
                1,
                validator_list_size as u64,
                &id(),
            ),
            instruction::initialize(
                &id(),
                &stake_pool_accounts.stake_pool.pubkey(),
                &stake_pool_accounts.manager.pubkey(),
                &stake_pool_accounts.staker.pubkey(),
                &stake_pool_accounts.validator_list.pubkey(),
                &stake_pool_accounts.reserve_stake.pubkey(),
                &stake_pool_accounts.pool_mint.pubkey(),
                &stake_pool_accounts.pool_fee_account.pubkey(),
                &spl_token::id(),
                None,
                stake_pool_accounts.fee,
                stake_pool_accounts.withdrawal_fee,
                stake_pool_accounts.deposit_fee,
                stake_pool_accounts.referral_fee,
                stake_pool_accounts.max_validators,
            ),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(
        &[
            &payer,
            &stake_pool_accounts.stake_pool,
            &stake_pool_accounts.validator_list,
            &stake_pool_accounts.manager,
        ],
        recent_blockhash,
    );

    let result = banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err()
        .unwrap();

    assert!(
        result == TransactionError::InstructionError(2, InstructionError::InvalidError,)
            || result
                == TransactionError::InstructionError(2, InstructionError::AccountNotRentExempt,)
    );
}

#[tokio::test]
async fn fail_without_manager_signature() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();

    create_required_accounts(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
    )
    .await;

    let rent = banks_client.get_rent().await.unwrap();
    let rent_stake_pool = rent.minimum_balance(get_packed_len::<state::StakePool>());
    let validator_list_size = get_instance_packed_len(&state::ValidatorList::new(
        stake_pool_accounts.max_validators,
    ))
    .unwrap();
    let rent_validator_list = rent.minimum_balance(validator_list_size);

    let init_data = instruction::StakePoolInstruction::Initialize {
        fee: stake_pool_accounts.fee,
        withdrawal_fee: stake_pool_accounts.withdrawal_fee,
        deposit_fee: stake_pool_accounts.deposit_fee,
        referral_fee: stake_pool_accounts.referral_fee,
        max_validators: stake_pool_accounts.max_validators,
    };
    let data = init_data.try_to_vec().unwrap();
    let accounts = vec![
        AccountMeta::new(stake_pool_accounts.stake_pool.pubkey(), true),
        AccountMeta::new_readonly(stake_pool_accounts.manager.pubkey(), false),
        AccountMeta::new_readonly(stake_pool_accounts.staker.pubkey(), false),
        AccountMeta::new(stake_pool_accounts.validator_list.pubkey(), false),
        AccountMeta::new_readonly(stake_pool_accounts.reserve_stake.pubkey(), false),
        AccountMeta::new_readonly(stake_pool_accounts.pool_mint.pubkey(), false),
        AccountMeta::new_readonly(stake_pool_accounts.pool_fee_account.pubkey(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let stake_pool_init_instruction = Instruction {
        program_id: id(),
        accounts,
        data,
    };

    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &stake_pool_accounts.stake_pool.pubkey(),
                rent_stake_pool,
                get_packed_len::<state::StakePool>() as u64,
                &id(),
            ),
            system_instruction::create_account(
                &payer.pubkey(),
                &stake_pool_accounts.validator_list.pubkey(),
                rent_validator_list,
                validator_list_size as u64,
                &id(),
            ),
            stake_pool_init_instruction,
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(
        &[
            &payer,
            &stake_pool_accounts.stake_pool,
            &stake_pool_accounts.validator_list,
        ],
        recent_blockhash,
    );
    let transaction_error = banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = error::StakePoolError::SignatureMissing as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!(
            "Wrong error occurs while try to initialize stake pool without manager's signature"
        ),
    }
}

#[tokio::test]
async fn fail_with_pre_minted_pool_tokens() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    let mint_authority = Keypair::new();

    create_mint(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts.pool_mint,
        &mint_authority.pubkey(),
    )
    .await
    .unwrap();

    create_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts.pool_fee_account,
        &stake_pool_accounts.pool_mint.pubkey(),
        &stake_pool_accounts.manager.pubkey(),
    )
    .await
    .unwrap();

    mint_tokens(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts.pool_mint.pubkey(),
        &stake_pool_accounts.pool_fee_account.pubkey(),
        &mint_authority,
        1,
    )
    .await
    .unwrap();

    let transaction_error = create_stake_pool(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts.stake_pool,
        &stake_pool_accounts.validator_list,
        &stake_pool_accounts.reserve_stake.pubkey(),
        &stake_pool_accounts.pool_mint.pubkey(),
        &stake_pool_accounts.pool_fee_account.pubkey(),
        &stake_pool_accounts.manager,
        &stake_pool_accounts.staker.pubkey(),
        &None,
        &stake_pool_accounts.fee,
        &stake_pool_accounts.withdrawal_fee,
        &stake_pool_accounts.deposit_fee,
        stake_pool_accounts.referral_fee,
        &stake_pool_accounts.sol_deposit_fee,
        stake_pool_accounts.sol_referral_fee,
        stake_pool_accounts.max_validators,
    )
    .await
    .err()
    .unwrap();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = error::StakePoolError::NonZeroPoolTokenSupply as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to initialize stake pool with wrong mint authority of pool fee account"),
    }
}

#[tokio::test]
async fn fail_with_bad_reserve() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_pool_accounts = StakePoolAccounts::new();
    let wrong_authority = Pubkey::new_unique();

    create_required_accounts(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &stake_pool_accounts,
    )
    .await;

    {
        let bad_stake = Keypair::new();
        create_independent_stake_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &bad_stake,
            &stake_program::Authorized {
                staker: wrong_authority,
                withdrawer: stake_pool_accounts.withdraw_authority,
            },
            &stake_program::Lockup::default(),
            1,
        )
        .await;

        let error = create_stake_pool(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &stake_pool_accounts.stake_pool,
            &stake_pool_accounts.validator_list,
            &bad_stake.pubkey(),
            &stake_pool_accounts.pool_mint.pubkey(),
            &stake_pool_accounts.pool_fee_account.pubkey(),
            &stake_pool_accounts.manager,
            &stake_pool_accounts.staker.pubkey(),
            &None,
            &stake_pool_accounts.fee,
            &stake_pool_accounts.withdrawal_fee,
            &stake_pool_accounts.deposit_fee,
            stake_pool_accounts.referral_fee,
            &stake_pool_accounts.sol_deposit_fee,
            stake_pool_accounts.sol_referral_fee,
            stake_pool_accounts.max_validators,
        )
        .await
        .err()
        .unwrap()
        .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                2,
                InstructionError::Custom(error::StakePoolError::WrongStakeState as u32),
            )
        );
    }

    {
        let bad_stake = Keypair::new();
        create_independent_stake_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &bad_stake,
            &stake_program::Authorized {
                staker: stake_pool_accounts.withdraw_authority,
                withdrawer: wrong_authority,
            },
            &stake_program::Lockup::default(),
            1,
        )
        .await;

        let error = create_stake_pool(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &stake_pool_accounts.stake_pool,
            &stake_pool_accounts.validator_list,
            &bad_stake.pubkey(),
            &stake_pool_accounts.pool_mint.pubkey(),
            &stake_pool_accounts.pool_fee_account.pubkey(),
            &stake_pool_accounts.manager,
            &stake_pool_accounts.staker.pubkey(),
            &None,
            &stake_pool_accounts.fee,
            &stake_pool_accounts.withdrawal_fee,
            &stake_pool_accounts.deposit_fee,
            stake_pool_accounts.referral_fee,
            &stake_pool_accounts.sol_deposit_fee,
            stake_pool_accounts.sol_referral_fee,
            stake_pool_accounts.max_validators,
        )
        .await
        .err()
        .unwrap()
        .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                2,
                InstructionError::Custom(error::StakePoolError::WrongStakeState as u32),
            )
        );
    }

    {
        let bad_stake = Keypair::new();
        create_independent_stake_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &bad_stake,
            &stake_program::Authorized {
                staker: stake_pool_accounts.withdraw_authority,
                withdrawer: stake_pool_accounts.withdraw_authority,
            },
            &stake_program::Lockup {
                custodian: wrong_authority,
                ..stake_program::Lockup::default()
            },
            1,
        )
        .await;

        let error = create_stake_pool(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &stake_pool_accounts.stake_pool,
            &stake_pool_accounts.validator_list,
            &bad_stake.pubkey(),
            &stake_pool_accounts.pool_mint.pubkey(),
            &stake_pool_accounts.pool_fee_account.pubkey(),
            &stake_pool_accounts.manager,
            &stake_pool_accounts.staker.pubkey(),
            &None,
            &stake_pool_accounts.fee,
            &stake_pool_accounts.withdrawal_fee,
            &stake_pool_accounts.deposit_fee,
            stake_pool_accounts.referral_fee,
            &stake_pool_accounts.sol_deposit_fee,
            stake_pool_accounts.sol_referral_fee,
            stake_pool_accounts.max_validators,
        )
        .await
        .err()
        .unwrap()
        .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                2,
                InstructionError::Custom(error::StakePoolError::WrongStakeState as u32),
            )
        );
    }

    {
        let bad_stake = Keypair::new();
        let rent = banks_client.get_rent().await.unwrap();
        let lamports = rent.minimum_balance(std::mem::size_of::<stake_program::StakeState>());

        let transaction = Transaction::new_signed_with_payer(
            &[system_instruction::create_account(
                &payer.pubkey(),
                &bad_stake.pubkey(),
                lamports,
                std::mem::size_of::<stake_program::StakeState>() as u64,
                &stake_program::id(),
            )],
            Some(&payer.pubkey()),
            &[&payer, &bad_stake],
            recent_blockhash,
        );
        banks_client.process_transaction(transaction).await.unwrap();

        let error = create_stake_pool(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &stake_pool_accounts.stake_pool,
            &stake_pool_accounts.validator_list,
            &bad_stake.pubkey(),
            &stake_pool_accounts.pool_mint.pubkey(),
            &stake_pool_accounts.pool_fee_account.pubkey(),
            &stake_pool_accounts.manager,
            &stake_pool_accounts.staker.pubkey(),
            &None,
            &stake_pool_accounts.fee,
            &stake_pool_accounts.withdrawal_fee,
            &stake_pool_accounts.deposit_fee,
            stake_pool_accounts.referral_fee,
            &stake_pool_accounts.sol_deposit_fee,
            stake_pool_accounts.sol_referral_fee,
            stake_pool_accounts.max_validators,
        )
        .await
        .err()
        .unwrap()
        .unwrap();

        assert_eq!(
            error,
            TransactionError::InstructionError(
                2,
                InstructionError::Custom(error::StakePoolError::WrongStakeState as u32),
            )
        );
    }
}

#[tokio::test]
async fn success_with_required_stake_deposit_authority() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let stake_deposit_authority = Keypair::new();
    let stake_pool_accounts =
        StakePoolAccounts::new_with_stake_deposit_authority(stake_deposit_authority);
    stake_pool_accounts
        .initialize_stake_pool(&mut banks_client, &payer, &recent_blockhash, 1)
        .await
        .unwrap();

    // Stake pool now exists
    let stake_pool_account =
        get_account(&mut banks_client, &stake_pool_accounts.stake_pool.pubkey()).await;
    let stake_pool =
        try_from_slice_unchecked::<state::StakePool>(stake_pool_account.data.as_slice()).unwrap();
    assert_eq!(
        stake_pool.stake_deposit_authority,
        stake_pool_accounts.stake_deposit_authority
    );
}
