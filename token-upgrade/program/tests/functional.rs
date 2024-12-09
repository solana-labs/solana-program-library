// Mark this test as SBF-only due to current `ProgramTest` limitations when
// CPIing into the system program
#![cfg(feature = "test-sbf")]

use {
    solana_program_test::{
        processor,
        tokio::{self, sync::Mutex},
        ProgramTest, ProgramTestContext,
    },
    solana_sdk::{
        instruction::{AccountMeta, InstructionError},
        pubkey::Pubkey,
        signature::Signer,
        signer::keypair::Keypair,
        transaction::{Transaction, TransactionError},
    },
    spl_token_client::{
        client::{
            ProgramBanksClient, ProgramBanksClientProcessTransaction, ProgramClient,
            SendTransaction, SimulateTransaction,
        },
        token::Token,
    },
    spl_token_upgrade::{
        error::TokenUpgradeError, get_token_upgrade_authority_address, instruction::exchange,
    },
    std::sync::Arc,
    test_case::test_case,
};

fn keypair_clone(kp: &Keypair) -> Keypair {
    Keypair::from_bytes(&kp.to_bytes()).expect("failed to copy keypair")
}

async fn setup() -> (
    Arc<Mutex<ProgramTestContext>>,
    Arc<dyn ProgramClient<ProgramBanksClientProcessTransaction>>,
    Arc<Keypair>,
) {
    let mut program_test = ProgramTest::new(
        "spl_token_upgrade",
        spl_token_upgrade::id(),
        processor!(spl_token_upgrade::processor::process),
    );

    program_test.prefer_bpf(false); // simplicity in the build

    program_test.add_program(
        "spl_token_2022",
        spl_token_2022::id(),
        processor!(spl_token_2022::processor::Processor::process),
    );
    program_test.add_program(
        "spl_token",
        spl_token::id(),
        processor!(spl_token::processor::Processor::process),
    );

    let context = program_test.start_with_context().await;
    let payer = Arc::new(keypair_clone(&context.payer));
    let context = Arc::new(Mutex::new(context));

    let client: Arc<dyn ProgramClient<ProgramBanksClientProcessTransaction>> =
        Arc::new(ProgramBanksClient::new_from_context(
            Arc::clone(&context),
            ProgramBanksClientProcessTransaction,
        ));
    (context, client, payer)
}

async fn setup_mint<T: SendTransaction + SimulateTransaction>(
    program_id: &Pubkey,
    mint_authority: &Pubkey,
    decimals: u8,
    payer: Arc<Keypair>,
    client: Arc<dyn ProgramClient<T>>,
) -> Token<T> {
    let mint_account = Keypair::new();
    let token = Token::new(
        client,
        program_id,
        &mint_account.pubkey(),
        Some(decimals),
        payer,
    );
    token
        .create_mint(mint_authority, None, vec![], &[&mint_account])
        .await
        .unwrap();
    token
}

#[test_case(spl_token::id(), spl_token_2022::id() ; "upgrade to token-2022")]
#[test_case(spl_token_2022::id(), spl_token::id() ; "downgrade to token")]
#[test_case(spl_token::id(), spl_token::id() ; "token to token")]
#[test_case(spl_token_2022::id(), spl_token_2022::id() ; "token-2022 to token-2022")]
#[tokio::test]
async fn success(original_program_id: Pubkey, new_program_id: Pubkey) {
    let (context, client, payer) = setup().await;

    let wallet = Keypair::new();
    let mint_authority = Keypair::new();
    let mint_authority_pubkey = mint_authority.pubkey();

    let decimals = 2;
    let original_token = setup_mint(
        &original_program_id,
        &mint_authority_pubkey,
        decimals,
        payer.clone(),
        client.clone(),
    )
    .await;
    let new_token = setup_mint(
        &new_program_id,
        &mint_authority_pubkey,
        decimals,
        payer.clone(),
        client.clone(),
    )
    .await;

    let program_escrow = get_token_upgrade_authority_address(
        original_token.get_address(),
        new_token.get_address(),
        &spl_token_upgrade::id(),
    );

    original_token
        .create_associated_token_account(&wallet.pubkey())
        .await
        .unwrap();
    let original_account = original_token.get_associated_token_address(&wallet.pubkey());
    let token_amount = 1_000_000_000_000;
    original_token
        .mint_to(
            &original_account,
            &mint_authority_pubkey,
            token_amount,
            &[&mint_authority],
        )
        .await
        .unwrap();

    new_token
        .create_associated_token_account(&wallet.pubkey())
        .await
        .unwrap();
    let new_account = new_token.get_associated_token_address(&wallet.pubkey());
    new_token
        .create_associated_token_account(&program_escrow)
        .await
        .unwrap();
    let escrow_account = new_token.get_associated_token_address(&program_escrow);
    new_token
        .mint_to(
            &escrow_account,
            &mint_authority_pubkey,
            token_amount,
            &[&mint_authority],
        )
        .await
        .unwrap();

    {
        let context = context.lock().await;
        let transaction = Transaction::new_signed_with_payer(
            &[exchange(
                &spl_token_upgrade::id(),
                &original_account,
                original_token.get_address(),
                &escrow_account,
                &new_account,
                new_token.get_address(),
                &original_program_id,
                &new_program_id,
                &wallet.pubkey(),
                &[],
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, &wallet],
            context.last_blockhash,
        );
        context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();
    }

    let original_mint = original_token.get_mint_info().await.unwrap();
    assert_eq!(original_mint.base.supply, 0);
    let original_account_info = original_token
        .get_account_info(&original_account)
        .await
        .unwrap();
    assert_eq!(original_account_info.base.amount, 0);
    let new_account_info = new_token.get_account_info(&new_account).await.unwrap();
    assert_eq!(new_account_info.base.amount, token_amount);
    let escrow_info = new_token.get_account_info(&escrow_account).await.unwrap();
    assert_eq!(escrow_info.base.amount, 0);
}

#[test_case(spl_token::id(), spl_token_2022::id() ; "fail upgrade to token-2022")]
#[tokio::test]
async fn fail_incorrect_escrow_derivation(original_program_id: Pubkey, new_program_id: Pubkey) {
    let (context, client, payer) = setup().await;

    let wallet = Keypair::new();
    let mint_authority = Keypair::new();
    let mint_authority_pubkey = mint_authority.pubkey();

    let decimals = 2;
    let original_token = setup_mint(
        &original_program_id,
        &mint_authority_pubkey,
        decimals,
        payer.clone(),
        client.clone(),
    )
    .await;
    let new_token = setup_mint(
        &new_program_id,
        &mint_authority_pubkey,
        decimals,
        payer.clone(),
        client.clone(),
    )
    .await;

    // backwards derivation
    let program_escrow = get_token_upgrade_authority_address(
        new_token.get_address(),
        original_token.get_address(),
        &spl_token_upgrade::id(),
    );

    original_token
        .create_associated_token_account(&wallet.pubkey())
        .await
        .unwrap();
    let original_account = original_token.get_associated_token_address(&wallet.pubkey());
    let token_amount = 1_000_000_000_000;
    original_token
        .mint_to(
            &original_account,
            &mint_authority_pubkey,
            token_amount,
            &[&mint_authority],
        )
        .await
        .unwrap();

    new_token
        .create_associated_token_account(&wallet.pubkey())
        .await
        .unwrap();
    let new_account = new_token.get_associated_token_address(&wallet.pubkey());
    new_token
        .create_associated_token_account(&program_escrow)
        .await
        .unwrap();
    let escrow_account = new_token.get_associated_token_address(&program_escrow);
    new_token
        .mint_to(
            &escrow_account,
            &mint_authority_pubkey,
            token_amount,
            &[&mint_authority],
        )
        .await
        .unwrap();

    let mut instruction = exchange(
        &spl_token_upgrade::id(),
        &original_account,
        original_token.get_address(),
        &escrow_account,
        &new_account,
        new_token.get_address(),
        &original_program_id,
        &new_program_id,
        &wallet.pubkey(),
        &[],
    );
    instruction.accounts[5] = AccountMeta::new_readonly(program_escrow, false);
    let context = context.lock().await;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &[&context.payer, &wallet],
        context.last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(TokenUpgradeError::InvalidOwner as u32)
        )
    );
}

#[test_case(spl_token::id(), spl_token_2022::id() ; "fail upgrade to token-2022")]
#[tokio::test]
async fn fail_decimals_mismatch(original_program_id: Pubkey, new_program_id: Pubkey) {
    let (context, client, payer) = setup().await;

    let wallet = Keypair::new();
    let mint_authority = Keypair::new();
    let mint_authority_pubkey = mint_authority.pubkey();

    // different decimals
    let original_decimals = 2;
    let new_decimals = 3;

    let original_token = setup_mint(
        &original_program_id,
        &mint_authority_pubkey,
        original_decimals,
        payer.clone(),
        client.clone(),
    )
    .await;
    let new_token = setup_mint(
        &new_program_id,
        &mint_authority_pubkey,
        new_decimals,
        payer.clone(),
        client.clone(),
    )
    .await;

    let program_escrow = get_token_upgrade_authority_address(
        original_token.get_address(),
        new_token.get_address(),
        &spl_token_upgrade::id(),
    );

    original_token
        .create_associated_token_account(&wallet.pubkey())
        .await
        .unwrap();
    let original_account = original_token.get_associated_token_address(&wallet.pubkey());
    let token_amount = 1_000_000_000_000;
    original_token
        .mint_to(
            &original_account,
            &mint_authority_pubkey,
            token_amount,
            &[&mint_authority],
        )
        .await
        .unwrap();

    new_token
        .create_associated_token_account(&wallet.pubkey())
        .await
        .unwrap();
    let new_account = new_token.get_associated_token_address(&wallet.pubkey());
    new_token
        .create_associated_token_account(&program_escrow)
        .await
        .unwrap();
    let escrow_account = new_token.get_associated_token_address(&program_escrow);
    new_token
        .mint_to(
            &escrow_account,
            &mint_authority_pubkey,
            token_amount,
            &[&mint_authority],
        )
        .await
        .unwrap();

    let context = context.lock().await;
    let transaction = Transaction::new_signed_with_payer(
        &[exchange(
            &spl_token_upgrade::id(),
            &original_account,
            original_token.get_address(),
            &escrow_account,
            &new_account,
            new_token.get_address(),
            &original_program_id,
            &new_program_id,
            &wallet.pubkey(),
            &[],
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &wallet],
        context.last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(TokenUpgradeError::DecimalsMismatch as u32)
        )
    );
}
