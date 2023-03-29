#![allow(clippy::integer_arithmetic)]
#![cfg(feature = "test-sbf")]

mod helpers;

use {
    helpers::*,
    solana_program_test::*,
    solana_sdk::{
        instruction::Instruction,
        message::Message,
        program_error::ProgramError,
        signature::{Keypair, Signer},
        stake, system_program,
        transaction::Transaction,
    },
    spl_single_validator_pool::{error::SinglePoolError, id, instruction},
    test_case::test_case,
};

#[derive(Clone, Debug, PartialEq, Eq)]
enum TestMode {
    Initialize,
    Deposit,
    Withdraw,
}

async fn build_instructions(
    context: &mut ProgramTestContext,
    accounts: &SinglePoolAccounts,
    test_mode: TestMode,
) -> (Vec<Instruction>, usize) {
    let initialize_instructions = if test_mode == TestMode::Initialize {
        let first_normal_slot = context.genesis_config().epoch_schedule.first_normal_slot;
        context.warp_to_slot(first_normal_slot).unwrap();

        create_vote(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &accounts.validator,
            &accounts.vote_account,
        )
        .await;

        transfer(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &accounts.alice.pubkey(),
            USER_STARTING_LAMPORTS,
        )
        .await;

        let rent = context.banks_client.get_rent().await.unwrap();
        let minimum_delegation = get_minimum_delegation(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
        )
        .await;

        instruction::initialize(
            &id(),
            &accounts.vote_account.pubkey(),
            &accounts.alice.pubkey(),
            &rent,
            minimum_delegation,
        )
    } else {
        accounts
            .initialize_for_deposit(context, TEST_STAKE_AMOUNT, None)
            .await;
        advance_epoch(context).await;

        vec![]
    };

    let deposit_instructions = instruction::deposit(
        &id(),
        &accounts.vote_account.pubkey(),
        &accounts.alice_stake.pubkey(),
        &accounts.alice_token,
        &accounts.alice.pubkey(),
        &accounts.alice.pubkey(),
    );

    let withdraw_instructions = if test_mode == TestMode::Withdraw {
        let message = Message::new(&deposit_instructions, Some(&accounts.alice.pubkey()));
        let transaction = Transaction::new(&[&accounts.alice], message, context.last_blockhash);

        context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        create_blank_stake_account(
            &mut context.banks_client,
            &accounts.alice,
            &context.last_blockhash,
            &accounts.alice_stake,
        )
        .await;

        instruction::withdraw(
            &id(),
            &accounts.vote_account.pubkey(),
            &accounts.alice_stake.pubkey(),
            &accounts.alice.pubkey(),
            &accounts.alice_token,
            &accounts.alice.pubkey(),
            get_token_balance(&mut context.banks_client, &accounts.alice_token).await,
        )
    } else {
        vec![]
    };

    let (instructions, i) = match test_mode {
        TestMode::Initialize => (initialize_instructions, 2),
        TestMode::Deposit => (deposit_instructions, 2),
        TestMode::Withdraw => (withdraw_instructions, 1),
    };

    // guard against instructions moving with code changes
    assert_eq!(instructions[i].program_id, id());

    (instructions, i)
}

#[test_case(TestMode::Initialize; "initialize")]
#[test_case(TestMode::Deposit; "deposit")]
#[test_case(TestMode::Withdraw; "withdraw")]
#[tokio::test]
async fn fail_account_checks(test_mode: TestMode) {
    let mut context = program_test().start_with_context().await;
    let accounts = SinglePoolAccounts::default();
    let (instructions, i) = build_instructions(&mut context, &accounts, test_mode).await;

    for j in 0..instructions[i].accounts.len() {
        let mut instructions = instructions.clone();
        let instruction_account = &mut instructions[i].accounts[j];

        // wallet address can be arbitrary
        if instruction_account.pubkey == accounts.alice.pubkey() {
            continue;
        }

        let prev_pubkey = instruction_account.pubkey;
        instruction_account.pubkey = Keypair::new().pubkey();

        let message = Message::new(&instructions, Some(&accounts.alice.pubkey()));
        let transaction = Transaction::new(&[&accounts.alice], message, context.last_blockhash);

        // random addresses should error always otherwise
        let e = context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err();

        // these ones we can also make sure we hit the explicit check, before we use it
        if prev_pubkey == accounts.stake_account {
            check_error(e, SinglePoolError::InvalidPoolStakeAccount)
        } else if prev_pubkey == accounts.authority {
            check_error(e, SinglePoolError::InvalidPoolAuthority)
        } else if prev_pubkey == accounts.mint {
            check_error(e, SinglePoolError::InvalidPoolMint)
        } else if [system_program::id(), spl_token::id(), stake::program::id()]
            .contains(&prev_pubkey)
        {
            check_error(e, ProgramError::IncorrectProgramId)
        }

        // TODO explicitly check clock/rent/stake history/stake config? we just let the stake program do it
    }
}
