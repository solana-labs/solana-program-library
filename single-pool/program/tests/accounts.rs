#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::items_after_test_module)]
#![cfg(feature = "test-sbf")]

mod helpers;

use {
    helpers::*,
    solana_program_test::*,
    solana_sdk::{
        instruction::Instruction, program_error::ProgramError, pubkey::Pubkey, signature::Signer,
        stake, system_program, transaction::Transaction,
    },
    spl_single_pool::{
        error::SinglePoolError,
        id,
        instruction::{self, SinglePoolInstruction},
    },
    test_case::test_case,
};

#[derive(Clone, Debug, PartialEq, Eq)]
enum TestMode {
    Initialize,
    Deposit,
    Withdraw,
}

// build a full transaction for initialize, deposit, and withdraw
// this is used to test knocking out individual accounts, for the sake of
// confirming the pubkeys are checked
async fn build_instructions(
    context: &mut ProgramTestContext,
    accounts: &SinglePoolAccounts,
    test_mode: TestMode,
) -> (Vec<Instruction>, usize) {
    let initialize_instructions = if test_mode == TestMode::Initialize {
        let slot = context.genesis_config().epoch_schedule.first_normal_slot + 1;
        context.warp_to_slot(slot).unwrap();

        create_vote(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &accounts.validator,
            &accounts.voter.pubkey(),
            &accounts.withdrawer.pubkey(),
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
        let minimum_delegation = get_pool_minimum_delegation(
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
        &accounts.pool,
        &accounts.alice_stake.pubkey(),
        &accounts.alice_token,
        &accounts.alice.pubkey(),
        &accounts.alice.pubkey(),
    );

    let withdraw_instructions = if test_mode == TestMode::Withdraw {
        let transaction = Transaction::new_signed_with_payer(
            &deposit_instructions,
            Some(&accounts.alice.pubkey()),
            &[&accounts.alice],
            context.last_blockhash,
        );

        context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        create_blank_stake_account(
            &mut context.banks_client,
            &context.payer,
            &accounts.alice,
            &context.last_blockhash,
            &accounts.alice_stake,
        )
        .await;

        instruction::withdraw(
            &id(),
            &accounts.pool,
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
        TestMode::Initialize => (initialize_instructions, 3),
        TestMode::Deposit => (deposit_instructions, 2),
        TestMode::Withdraw => (withdraw_instructions, 1),
    };

    // guard against instructions moving with code changes
    assert_eq!(instructions[i].program_id, id());

    (instructions, i)
}

// test that account addresses are checked properly
#[test_case(TestMode::Initialize; "initialize")]
#[test_case(TestMode::Deposit; "deposit")]
#[test_case(TestMode::Withdraw; "withdraw")]
#[tokio::test]
async fn fail_account_checks(test_mode: TestMode) {
    let mut context = program_test(false).start_with_context().await;
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
        instruction_account.pubkey = Pubkey::new_unique();

        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&accounts.alice.pubkey()),
            &[&accounts.alice],
            context.last_blockhash,
        );

        // random addresses should error always otherwise
        let e = context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err();

        // these ones we can also make sure we hit the explicit check, before we use it
        if prev_pubkey == accounts.pool {
            check_error(e, SinglePoolError::InvalidPoolAccount)
        } else if prev_pubkey == accounts.stake_account {
            check_error(e, SinglePoolError::InvalidPoolStakeAccount)
        } else if prev_pubkey == accounts.stake_authority {
            check_error(e, SinglePoolError::InvalidPoolStakeAuthority)
        } else if prev_pubkey == accounts.mint_authority {
            check_error(e, SinglePoolError::InvalidPoolMintAuthority)
        } else if prev_pubkey == accounts.mpl_authority {
            check_error(e, SinglePoolError::InvalidPoolMplAuthority)
        } else if prev_pubkey == accounts.mint {
            check_error(e, SinglePoolError::InvalidPoolMint)
        } else if [system_program::id(), spl_token::id(), stake::program::id()]
            .contains(&prev_pubkey)
        {
            check_error(e, ProgramError::IncorrectProgramId)
        }
    }
}

// make an individual instruction for all program instructions
// the match is just so this will error if new instructions are added
// if you are reading this because of that error, add the case to the
// `consistent_account_order` test!!!
fn make_basic_instruction(
    accounts: &SinglePoolAccounts,
    instruction_type: SinglePoolInstruction,
) -> Instruction {
    match instruction_type {
        SinglePoolInstruction::InitializePool => {
            instruction::initialize_pool(&id(), &accounts.vote_account.pubkey())
        }
        SinglePoolInstruction::ReactivatePoolStake => {
            instruction::reactivate_pool_stake(&id(), &accounts.vote_account.pubkey())
        }
        SinglePoolInstruction::DepositStake => instruction::deposit_stake(
            &id(),
            &accounts.pool,
            &Pubkey::default(),
            &Pubkey::default(),
            &Pubkey::default(),
        ),
        SinglePoolInstruction::WithdrawStake { .. } => instruction::withdraw_stake(
            &id(),
            &accounts.pool,
            &Pubkey::default(),
            &Pubkey::default(),
            &Pubkey::default(),
            0,
        ),
        SinglePoolInstruction::CreateTokenMetadata => {
            instruction::create_token_metadata(&id(), &accounts.pool, &Pubkey::default())
        }
        SinglePoolInstruction::UpdateTokenMetadata { .. } => instruction::update_token_metadata(
            &id(),
            &accounts.vote_account.pubkey(),
            &accounts.withdrawer.pubkey(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
        ),
    }
}

// advanced technology
fn is_sorted<T>(data: &[T]) -> bool
where
    T: Ord,
{
    data.windows(2).all(|w| w[0] <= w[1])
}

// check that major accounts always show up in the same order, to spare
// developer confusion
#[test]
fn consistent_account_order() {
    let accounts = SinglePoolAccounts::default();

    let ordering = vec![
        accounts.vote_account.pubkey(),
        accounts.pool,
        accounts.stake_account,
        accounts.mint,
        accounts.stake_authority,
        accounts.mint_authority,
        accounts.mpl_authority,
    ];

    let instructions = vec![
        make_basic_instruction(&accounts, SinglePoolInstruction::InitializePool),
        make_basic_instruction(&accounts, SinglePoolInstruction::ReactivatePoolStake),
        make_basic_instruction(&accounts, SinglePoolInstruction::DepositStake),
        make_basic_instruction(
            &accounts,
            SinglePoolInstruction::WithdrawStake {
                user_stake_authority: Pubkey::default(),
                token_amount: 0,
            },
        ),
        make_basic_instruction(&accounts, SinglePoolInstruction::CreateTokenMetadata),
        make_basic_instruction(
            &accounts,
            SinglePoolInstruction::UpdateTokenMetadata {
                name: "".to_string(),
                symbol: "".to_string(),
                uri: "".to_string(),
            },
        ),
    ];

    for instruction in instructions {
        let mut indexes = vec![];

        for target in &ordering {
            if let Some(i) = instruction
                .accounts
                .iter()
                .position(|meta| meta.pubkey == *target)
            {
                indexes.push(i);
            }
        }

        assert!(is_sorted(&indexes));
    }
}
