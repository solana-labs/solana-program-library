#![cfg(feature = "test-bpf")]

mod helpers;

use {
    bincode,
    borsh::BorshSerialize,
    helpers::*,
    solana_program::{
        borsh::try_from_slice_unchecked, program_option::COption, program_pack::Pack,
        pubkey::Pubkey,
    },
    solana_program_test::*,
    solana_sdk::{
        account::{Account, WritableAccount},
        clock::{Clock, Epoch},
        signature::{Keypair, Signer},
        transaction::Transaction,
    },
    solana_vote_program::{
        self,
        vote_state::{VoteInit, VoteState, VoteStateVersions},
    },
    spl_stake_pool::{
        find_stake_program_address, find_transient_stake_program_address,
        find_withdraw_authority_program_address, id,
        instruction::{self, PreferredValidatorType},
        stake_program,
        state::{AccountType, Fee, StakePool, StakeStatus, ValidatorList, ValidatorStakeInfo},
        MAX_VALIDATORS_TO_UPDATE, MINIMUM_ACTIVE_STAKE,
    },
    spl_token::state::{Account as SplAccount, AccountState as SplAccountState, Mint},
};

const HUGE_POOL_SIZE: u32 = 3_950;
const ACCOUNT_RENT_EXEMPTION: u64 = 1_000_000_000; // go with something big to be safe
const STAKE_AMOUNT: u64 = 200_000_000_000;
const STAKE_ACCOUNT_RENT_EXEMPTION: u64 = 2_282_880;

async fn setup(
    max_validators: u32,
    num_validators: u32,
    stake_amount: u64,
) -> (
    ProgramTestContext,
    StakePoolAccounts,
    Vec<Pubkey>,
    Pubkey,
    Keypair,
    Pubkey,
    Pubkey,
) {
    let mut program_test = program_test();
    let mut vote_account_pubkeys = vec![];
    let mut stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts.max_validators = max_validators;

    let stake_pool_pubkey = stake_pool_accounts.stake_pool.pubkey();
    let (_, stake_withdraw_bump_seed) =
        find_withdraw_authority_program_address(&id(), &stake_pool_pubkey);

    let mut stake_pool = StakePool {
        account_type: AccountType::StakePool,
        manager: stake_pool_accounts.manager.pubkey(),
        staker: stake_pool_accounts.staker.pubkey(),
        stake_deposit_authority: stake_pool_accounts.stake_deposit_authority,
        stake_withdraw_bump_seed,
        validator_list: stake_pool_accounts.validator_list.pubkey(),
        reserve_stake: stake_pool_accounts.reserve_stake.pubkey(),
        pool_mint: stake_pool_accounts.pool_mint.pubkey(),
        manager_fee_account: stake_pool_accounts.pool_fee_account.pubkey(),
        token_program_id: spl_token::id(),
        total_stake_lamports: 0,
        pool_token_supply: 0,
        last_update_epoch: 0,
        lockup: stake_program::Lockup::default(),
        fee: stake_pool_accounts.fee,
        next_epoch_fee: None,
        preferred_deposit_validator_vote_address: None,
        preferred_withdraw_validator_vote_address: None,
        stake_deposit_fee: Fee::default(),
        sol_deposit_fee: Fee::default(),
        withdrawal_fee: Fee::default(),
        next_withdrawal_fee: None,
        stake_referral_fee: 0,
        sol_referral_fee: 0,
        sol_deposit_authority: None,
    };

    let mut validator_list = ValidatorList::new(max_validators);
    validator_list.validators = vec![];

    let authorized_voter = Pubkey::new_unique();
    let authorized_withdrawer = Pubkey::new_unique();
    let commission = 1;

    let meta = stake_program::Meta {
        rent_exempt_reserve: STAKE_ACCOUNT_RENT_EXEMPTION,
        authorized: stake_program::Authorized {
            staker: stake_pool_accounts.withdraw_authority,
            withdrawer: stake_pool_accounts.withdraw_authority,
        },
        lockup: stake_program::Lockup::default(),
    };

    for _ in 0..max_validators {
        // create vote account
        let vote_pubkey = Pubkey::new_unique();
        vote_account_pubkeys.push(vote_pubkey);
        let node_pubkey = Pubkey::new_unique();
        let vote_state = VoteStateVersions::new_current(VoteState::new(
            &VoteInit {
                node_pubkey,
                authorized_voter,
                authorized_withdrawer,
                commission,
            },
            &Clock::default(),
        ));
        let vote_account = Account::create(
            ACCOUNT_RENT_EXEMPTION,
            bincode::serialize::<VoteStateVersions>(&vote_state).unwrap(),
            solana_vote_program::id(),
            false,
            Epoch::default(),
        );
        program_test.add_account(vote_pubkey, vote_account);
    }

    for i in 0..num_validators as usize {
        let vote_account_address = vote_account_pubkeys[i];

        // create validator stake account
        let stake = stake_program::Stake {
            delegation: stake_program::Delegation {
                voter_pubkey: vote_account_address,
                stake: stake_amount,
                activation_epoch: 0,
                deactivation_epoch: u64::MAX,
                warmup_cooldown_rate: 0.25, // default
            },
            credits_observed: 0,
        };

        let stake_account = Account::create(
            stake_amount + STAKE_ACCOUNT_RENT_EXEMPTION,
            bincode::serialize::<stake_program::StakeState>(&stake_program::StakeState::Stake(
                meta, stake,
            ))
            .unwrap(),
            stake_program::id(),
            false,
            Epoch::default(),
        );

        let (stake_address, _) =
            find_stake_program_address(&id(), &vote_account_address, &stake_pool_pubkey);
        program_test.add_account(stake_address, stake_account);
        let active_stake_lamports = stake_amount - MINIMUM_ACTIVE_STAKE;
        // add to validator list
        validator_list.validators.push(ValidatorStakeInfo {
            status: StakeStatus::Active,
            vote_account_address,
            active_stake_lamports,
            transient_stake_lamports: 0,
            last_update_epoch: 0,
            transient_seed_suffix_start: 0,
            transient_seed_suffix_end: 0,
        });

        stake_pool.total_stake_lamports += active_stake_lamports;
        stake_pool.pool_token_supply += active_stake_lamports;
    }

    let mut validator_list_bytes = validator_list.try_to_vec().unwrap();

    // add extra room if needed
    for _ in num_validators..max_validators {
        validator_list_bytes.append(&mut ValidatorStakeInfo::default().try_to_vec().unwrap());
    }

    let reserve_stake_account = Account::create(
        stake_amount + STAKE_ACCOUNT_RENT_EXEMPTION,
        bincode::serialize::<stake_program::StakeState>(&stake_program::StakeState::Initialized(
            meta,
        ))
        .unwrap(),
        stake_program::id(),
        false,
        Epoch::default(),
    );
    program_test.add_account(
        stake_pool_accounts.reserve_stake.pubkey(),
        reserve_stake_account,
    );

    let mut stake_pool_bytes = stake_pool.try_to_vec().unwrap();
    // more room for optionals
    stake_pool_bytes.extend_from_slice(&Pubkey::default().to_bytes());
    stake_pool_bytes.extend_from_slice(&Pubkey::default().to_bytes());
    let stake_pool_account = Account::create(
        ACCOUNT_RENT_EXEMPTION,
        stake_pool_bytes,
        id(),
        false,
        Epoch::default(),
    );
    program_test.add_account(stake_pool_pubkey, stake_pool_account);

    let validator_list_account = Account::create(
        ACCOUNT_RENT_EXEMPTION,
        validator_list_bytes,
        id(),
        false,
        Epoch::default(),
    );
    program_test.add_account(
        stake_pool_accounts.validator_list.pubkey(),
        validator_list_account,
    );

    let mut mint_vec = vec![0u8; Mint::LEN];
    let mint = Mint {
        mint_authority: COption::Some(stake_pool_accounts.withdraw_authority),
        supply: stake_pool.pool_token_supply,
        decimals: 9,
        is_initialized: true,
        freeze_authority: COption::None,
    };
    Pack::pack(mint, &mut mint_vec).unwrap();
    let stake_pool_mint = Account::create(
        ACCOUNT_RENT_EXEMPTION,
        mint_vec,
        spl_token::id(),
        false,
        Epoch::default(),
    );
    program_test.add_account(stake_pool_accounts.pool_mint.pubkey(), stake_pool_mint);

    let mut fee_account_vec = vec![0u8; SplAccount::LEN];
    let fee_account_data = SplAccount {
        mint: stake_pool_accounts.pool_mint.pubkey(),
        owner: stake_pool_accounts.manager.pubkey(),
        amount: 0,
        delegate: COption::None,
        state: SplAccountState::Initialized,
        is_native: COption::None,
        delegated_amount: 0,
        close_authority: COption::None,
    };
    Pack::pack(fee_account_data, &mut fee_account_vec).unwrap();
    let fee_account = Account::create(
        ACCOUNT_RENT_EXEMPTION,
        fee_account_vec,
        spl_token::id(),
        false,
        Epoch::default(),
    );
    program_test.add_account(stake_pool_accounts.pool_fee_account.pubkey(), fee_account);

    let mut context = program_test.start_with_context().await;

    let vote_pubkey = vote_account_pubkeys[HUGE_POOL_SIZE as usize - 1];
    // make stake account
    let user = Keypair::new();
    let deposit_stake = Keypair::new();
    let lockup = stake_program::Lockup::default();

    let authorized = stake_program::Authorized {
        staker: user.pubkey(),
        withdrawer: user.pubkey(),
    };

    let _stake_lamports = create_independent_stake_account(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &deposit_stake,
        &authorized,
        &lockup,
        stake_amount,
    )
    .await;

    delegate_stake_account(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &deposit_stake.pubkey(),
        &user,
        &vote_pubkey,
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
        vote_account_pubkeys,
        vote_pubkey,
        user,
        deposit_stake.pubkey(),
        pool_token_account.pubkey(),
    )
}

#[tokio::test]
async fn update() {
    let (mut context, stake_pool_accounts, vote_account_pubkeys, _, _, _, _) =
        setup(HUGE_POOL_SIZE, HUGE_POOL_SIZE, STAKE_AMOUNT).await;

    let validator_list = stake_pool_accounts
        .get_validator_list(&mut context.banks_client)
        .await;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::update_validator_list_balance(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &stake_pool_accounts.validator_list.pubkey(),
            &stake_pool_accounts.reserve_stake.pubkey(),
            &validator_list,
            &vote_account_pubkeys[0..MAX_VALIDATORS_TO_UPDATE],
            0,
            /* no_merge = */ false,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err();
    assert!(error.is_none());

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::update_stake_pool_balance(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.withdraw_authority,
            &stake_pool_accounts.validator_list.pubkey(),
            &stake_pool_accounts.reserve_stake.pubkey(),
            &stake_pool_accounts.pool_fee_account.pubkey(),
            &stake_pool_accounts.pool_mint.pubkey(),
            &spl_token::id(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err();
    assert!(error.is_none());

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::cleanup_removed_validator_entries(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.validator_list.pubkey(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err();
    assert!(error.is_none());
}

#[tokio::test]
async fn remove_validator_from_pool() {
    let (mut context, stake_pool_accounts, vote_account_pubkeys, _, _, _, _) =
        setup(HUGE_POOL_SIZE, HUGE_POOL_SIZE, MINIMUM_ACTIVE_STAKE).await;

    let first_vote = vote_account_pubkeys[0];
    let (stake_address, _) =
        find_stake_program_address(&id(), &first_vote, &stake_pool_accounts.stake_pool.pubkey());
    let transient_stake_seed = u64::MAX;
    let (transient_stake_address, _) = find_transient_stake_program_address(
        &id(),
        &first_vote,
        &stake_pool_accounts.stake_pool.pubkey(),
        transient_stake_seed,
    );

    let new_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .remove_validator_from_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &new_authority,
            &stake_address,
            &transient_stake_address,
        )
        .await;
    assert!(error.is_none());

    let middle_index = HUGE_POOL_SIZE as usize / 2;
    let middle_vote = vote_account_pubkeys[middle_index];
    let (stake_address, _) = find_stake_program_address(
        &id(),
        &middle_vote,
        &stake_pool_accounts.stake_pool.pubkey(),
    );
    let (transient_stake_address, _) = find_transient_stake_program_address(
        &id(),
        &middle_vote,
        &stake_pool_accounts.stake_pool.pubkey(),
        transient_stake_seed,
    );

    let new_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .remove_validator_from_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &new_authority,
            &stake_address,
            &transient_stake_address,
        )
        .await;
    assert!(error.is_none());

    let last_index = HUGE_POOL_SIZE as usize - 1;
    let last_vote = vote_account_pubkeys[last_index];
    let (stake_address, _) =
        find_stake_program_address(&id(), &last_vote, &stake_pool_accounts.stake_pool.pubkey());
    let (transient_stake_address, _) = find_transient_stake_program_address(
        &id(),
        &last_vote,
        &stake_pool_accounts.stake_pool.pubkey(),
        transient_stake_seed,
    );

    let new_authority = Pubkey::new_unique();
    let error = stake_pool_accounts
        .remove_validator_from_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &new_authority,
            &stake_address,
            &transient_stake_address,
        )
        .await;
    assert!(error.is_none());

    let validator_list = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let validator_list =
        try_from_slice_unchecked::<ValidatorList>(validator_list.data.as_slice()).unwrap();
    let first_element = &validator_list.validators[0];
    assert_eq!(first_element.status, StakeStatus::ReadyForRemoval);
    assert_eq!(first_element.active_stake_lamports, 0);
    assert_eq!(first_element.transient_stake_lamports, 0);

    let middle_element = &validator_list.validators[middle_index];
    assert_eq!(middle_element.status, StakeStatus::ReadyForRemoval);
    assert_eq!(middle_element.active_stake_lamports, 0);
    assert_eq!(middle_element.transient_stake_lamports, 0);

    let last_element = &validator_list.validators[last_index];
    assert_eq!(last_element.status, StakeStatus::ReadyForRemoval);
    assert_eq!(last_element.active_stake_lamports, 0);
    assert_eq!(last_element.transient_stake_lamports, 0);

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::cleanup_removed_validator_entries(
            &id(),
            &stake_pool_accounts.stake_pool.pubkey(),
            &stake_pool_accounts.validator_list.pubkey(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err();
    assert!(error.is_none());

    let validator_list = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let validator_list =
        try_from_slice_unchecked::<ValidatorList>(validator_list.data.as_slice()).unwrap();
    assert_eq!(validator_list.validators.len() as u32, HUGE_POOL_SIZE - 3);
    // assert they're gone
    assert!(!validator_list
        .validators
        .iter()
        .any(|x| x.vote_account_address == first_vote));
    assert!(!validator_list
        .validators
        .iter()
        .any(|x| x.vote_account_address == middle_vote));
    assert!(!validator_list
        .validators
        .iter()
        .any(|x| x.vote_account_address == last_vote));

    // but that we didn't remove too many
    assert!(validator_list
        .validators
        .iter()
        .any(|x| x.vote_account_address == vote_account_pubkeys[1]));
    assert!(validator_list
        .validators
        .iter()
        .any(|x| x.vote_account_address == vote_account_pubkeys[middle_index - 1]));
    assert!(validator_list
        .validators
        .iter()
        .any(|x| x.vote_account_address == vote_account_pubkeys[middle_index + 1]));
    assert!(validator_list
        .validators
        .iter()
        .any(|x| x.vote_account_address == vote_account_pubkeys[last_index - 1]));
}

#[tokio::test]
async fn add_validator_to_pool() {
    let (mut context, stake_pool_accounts, _, test_vote_address, _, _, _) =
        setup(HUGE_POOL_SIZE, HUGE_POOL_SIZE - 1, STAKE_AMOUNT).await;

    let last_index = HUGE_POOL_SIZE as usize - 1;
    let stake_pool_pubkey = stake_pool_accounts.stake_pool.pubkey();
    let (stake_address, _) =
        find_stake_program_address(&id(), &test_vote_address, &stake_pool_pubkey);

    create_validator_stake_account(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_pool_pubkey,
        &stake_pool_accounts.staker,
        &stake_address,
        &test_vote_address,
    )
    .await;

    let error = stake_pool_accounts
        .add_validator_to_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &stake_address,
        )
        .await;
    assert!(error.is_none());

    let validator_list = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let validator_list =
        try_from_slice_unchecked::<ValidatorList>(validator_list.data.as_slice()).unwrap();
    assert_eq!(validator_list.validators.len(), last_index + 1);
    let last_element = validator_list.validators[last_index];
    assert_eq!(last_element.status, StakeStatus::Active);
    assert_eq!(last_element.active_stake_lamports, 0);
    assert_eq!(last_element.transient_stake_lamports, 0);
    assert_eq!(last_element.vote_account_address, test_vote_address);

    let transient_stake_seed = u64::MAX;
    let (transient_stake_address, _) = find_transient_stake_program_address(
        &id(),
        &test_vote_address,
        &stake_pool_pubkey,
        transient_stake_seed,
    );
    let increase_amount = MINIMUM_ACTIVE_STAKE;
    let error = stake_pool_accounts
        .increase_validator_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &transient_stake_address,
            &test_vote_address,
            increase_amount,
            transient_stake_seed,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);

    let validator_list = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.validator_list.pubkey(),
    )
    .await;
    let validator_list =
        try_from_slice_unchecked::<ValidatorList>(validator_list.data.as_slice()).unwrap();
    let last_element = validator_list.validators[last_index];
    assert_eq!(last_element.status, StakeStatus::Active);
    assert_eq!(last_element.active_stake_lamports, 0);
    assert_eq!(
        last_element.transient_stake_lamports,
        increase_amount + STAKE_ACCOUNT_RENT_EXEMPTION
    );
    assert_eq!(last_element.vote_account_address, test_vote_address);
}

#[tokio::test]
async fn set_preferred() {
    let (mut context, stake_pool_accounts, _, vote_account_address, _, _, _) =
        setup(HUGE_POOL_SIZE, HUGE_POOL_SIZE, STAKE_AMOUNT).await;

    let error = stake_pool_accounts
        .set_preferred_validator(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            PreferredValidatorType::Deposit,
            Some(vote_account_address),
        )
        .await;
    assert!(error.is_none());
    let error = stake_pool_accounts
        .set_preferred_validator(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            PreferredValidatorType::Withdraw,
            Some(vote_account_address),
        )
        .await;
    assert!(error.is_none());

    let stake_pool = get_account(
        &mut context.banks_client,
        &stake_pool_accounts.stake_pool.pubkey(),
    )
    .await;
    let stake_pool = try_from_slice_unchecked::<StakePool>(&stake_pool.data.as_slice()).unwrap();

    assert_eq!(
        stake_pool.preferred_deposit_validator_vote_address,
        Some(vote_account_address)
    );
    assert_eq!(
        stake_pool.preferred_withdraw_validator_vote_address,
        Some(vote_account_address)
    );
}

#[tokio::test]
async fn deposit_stake() {
    let (mut context, stake_pool_accounts, _, vote_pubkey, user, stake_pubkey, pool_account_pubkey) =
        setup(HUGE_POOL_SIZE, HUGE_POOL_SIZE, STAKE_AMOUNT).await;

    let (stake_address, _) = find_stake_program_address(
        &id(),
        &vote_pubkey,
        &stake_pool_accounts.stake_pool.pubkey(),
    );

    let error = stake_pool_accounts
        .deposit_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &stake_pubkey,
            &pool_account_pubkey,
            &stake_address,
            &user,
        )
        .await;
    assert!(error.is_none());
}

#[tokio::test]
async fn withdraw() {
    let (mut context, stake_pool_accounts, _, vote_pubkey, user, stake_pubkey, pool_account_pubkey) =
        setup(HUGE_POOL_SIZE, HUGE_POOL_SIZE, STAKE_AMOUNT).await;

    let (stake_address, _) = find_stake_program_address(
        &id(),
        &vote_pubkey,
        &stake_pool_accounts.stake_pool.pubkey(),
    );

    let error = stake_pool_accounts
        .deposit_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &stake_pubkey,
            &pool_account_pubkey,
            &stake_address,
            &user,
        )
        .await;
    assert!(error.is_none());

    // Create stake account to withdraw to
    let user_stake_recipient = Keypair::new();
    create_blank_stake_account(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &user_stake_recipient,
    )
    .await;

    let error = stake_pool_accounts
        .withdraw_stake(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &user_stake_recipient.pubkey(),
            &user,
            &pool_account_pubkey,
            &stake_address,
            &user.pubkey(),
            STAKE_AMOUNT,
        )
        .await;
    assert!(error.is_none(), "{:?}", error);
}
