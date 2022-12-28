#![cfg(feature = "test-sbf")]

use {
    solana_program_test::{processor, tokio, ProgramTest, ProgramTestContext},
    solana_sdk::{
        instruction::InstructionError,
        program_pack::Pack,
        pubkey::Pubkey,
        signature::Signer,
        signer::keypair::Keypair,
        system_instruction,
        transaction::{Transaction, TransactionError},
    },
    spl_token::{
        instruction,
        processor::Processor,
        state::{Account, Mint},
    },
};

async fn setup_mint_and_account(
    context: &mut ProgramTestContext,
    mint: &Keypair,
    token_account: &Keypair,
    owner: &Pubkey,
    token_program_id: &Pubkey,
) {
    let rent = context.banks_client.get_rent().await.unwrap();
    let mint_authority_pubkey = Pubkey::new_unique();

    let space = Mint::LEN;
    let tx = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &mint.pubkey(),
                rent.minimum_balance(space),
                space as u64,
                token_program_id,
            ),
            instruction::initialize_mint(
                token_program_id,
                &mint.pubkey(),
                &mint_authority_pubkey,
                None,
                9,
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, mint],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();
    let space = Account::LEN;
    let tx = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &token_account.pubkey(),
                rent.minimum_balance(space),
                space as u64,
                token_program_id,
            ),
            instruction::initialize_account(
                token_program_id,
                &token_account.pubkey(),
                &mint.pubkey(),
                owner,
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, token_account],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();
}

#[tokio::test]
async fn success_init_after_close_account() {
    let program_test =
        ProgramTest::new("spl_token", spl_token::id(), processor!(Processor::process));
    let mut context = program_test.start_with_context().await;
    let mint = Keypair::new();
    let token_account = Keypair::new();
    let owner = Keypair::new();
    let token_program_id = spl_token::id();
    setup_mint_and_account(
        &mut context,
        &mint,
        &token_account,
        &owner.pubkey(),
        &token_program_id,
    )
    .await;

    let destination = Pubkey::new_unique();
    let tx = Transaction::new_signed_with_payer(
        &[
            instruction::close_account(
                &token_program_id,
                &token_account.pubkey(),
                &destination,
                &owner.pubkey(),
                &[],
            )
            .unwrap(),
            system_instruction::create_account(
                &context.payer.pubkey(),
                &token_account.pubkey(),
                1_000_000_000,
                Account::LEN as u64,
                &token_program_id,
            ),
            instruction::initialize_account(
                &token_program_id,
                &token_account.pubkey(),
                &mint.pubkey(),
                &owner.pubkey(),
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &owner, &token_account],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();
    let destination = context
        .banks_client
        .get_account(destination)
        .await
        .unwrap()
        .unwrap();
    assert!(destination.lamports > 0);
}

#[tokio::test]
async fn fail_init_after_close_account() {
    let program_test =
        ProgramTest::new("spl_token", spl_token::id(), processor!(Processor::process));
    let mut context = program_test.start_with_context().await;
    let mint = Keypair::new();
    let token_account = Keypair::new();
    let owner = Keypair::new();
    let token_program_id = spl_token::id();
    setup_mint_and_account(
        &mut context,
        &mint,
        &token_account,
        &owner.pubkey(),
        &token_program_id,
    )
    .await;

    let destination = Pubkey::new_unique();
    let tx = Transaction::new_signed_with_payer(
        &[
            instruction::close_account(
                &token_program_id,
                &token_account.pubkey(),
                &destination,
                &owner.pubkey(),
                &[],
            )
            .unwrap(),
            system_instruction::transfer(
                &context.payer.pubkey(),
                &token_account.pubkey(),
                1_000_000_000,
            ),
            instruction::initialize_account(
                &token_program_id,
                &token_account.pubkey(),
                &mint.pubkey(),
                &owner.pubkey(),
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &owner],
        context.last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(tx)
        .await
        .unwrap_err()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(2, InstructionError::InvalidAccountData)
    );
    assert!(context
        .banks_client
        .get_account(destination)
        .await
        .unwrap()
        .is_none());
}
