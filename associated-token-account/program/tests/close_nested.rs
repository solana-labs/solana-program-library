// Mark this test as BPF-only due to current `ProgramTest` limitations when CPIing into the system program
#![cfg(feature = "test-bpf")]

mod program_test;

use {
    program_test::program_test,
    solana_program::{pubkey::Pubkey, system_instruction},
    solana_program_test::*,
    solana_sdk::{
        instruction::{AccountMeta, InstructionError},
        signature::Signer,
        signer::keypair::Keypair,
        transaction::{Transaction, TransactionError},
    },
    spl_associated_token_account::{get_associated_token_address_with_program_id, instruction},
    spl_token::{
        extension::{ExtensionType, StateWithExtensionsOwned},
        state::{Account, Mint},
    },
};

async fn create_mint(context: &mut ProgramTestContext) -> (Pubkey, Keypair) {
    let mint_account = Keypair::new();
    let token_mint_address = mint_account.pubkey();
    let mint_authority = Keypair::new();
    let space = ExtensionType::get_account_len::<Mint>(&[]);
    let rent = context.banks_client.get_rent().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &mint_account.pubkey(),
                rent.minimum_balance(space),
                space as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &token_mint_address,
                &mint_authority.pubkey(),
                Some(&mint_authority.pubkey()),
                0,
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &mint_account],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
    (token_mint_address, mint_authority)
}

async fn create_associated_token_account(
    context: &mut ProgramTestContext,
    owner: &Pubkey,
    mint: &Pubkey,
) -> Pubkey {
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::create_associated_token_account(
            &context.payer.pubkey(),
            owner,
            mint,
            &spl_token::id(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    get_associated_token_address_with_program_id(owner, mint, &spl_token::id())
}

#[allow(clippy::too_many_arguments)]
async fn try_close_nested(
    context: &mut ProgramTestContext,
    nested_mint: Pubkey,
    nested_mint_authority: Keypair,
    nested_associated_token_address: Pubkey,
    destination_token_address: Pubkey,
    wallet: Keypair,
    close_transaction: Transaction,
    expected_error: Option<InstructionError>,
) {
    let nested_account = context
        .banks_client
        .get_account(nested_associated_token_address)
        .await
        .unwrap()
        .unwrap();
    let lamports = nested_account.lamports;

    // mint to nested account
    let amount = 100;
    let transaction = Transaction::new_signed_with_payer(
        &[spl_token::instruction::mint_to(
            &spl_token::id(),
            &nested_mint,
            &nested_associated_token_address,
            &nested_mint_authority.pubkey(),
            &[],
            amount,
        )
        .unwrap()],
        Some(&context.payer.pubkey()),
        &[&context.payer, &nested_mint_authority],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // transfer / close nested account
    let result = context
        .banks_client
        .process_transaction(close_transaction)
        .await;

    if let Some(expected_error) = expected_error {
        let error = result.unwrap_err().unwrap();
        assert_eq!(error, TransactionError::InstructionError(0, expected_error));
    } else {
        result.unwrap();
        // nested account is gone
        assert!(context
            .banks_client
            .get_account(nested_associated_token_address)
            .await
            .unwrap()
            .is_none());
        let destination_account = context
            .banks_client
            .get_account(destination_token_address)
            .await
            .unwrap()
            .unwrap();
        let destination_state =
            StateWithExtensionsOwned::<Account>::unpack(destination_account.data).unwrap();
        assert_eq!(destination_state.base.amount, amount);
        let wallet_account = context
            .banks_client
            .get_account(wallet.pubkey())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(wallet_account.lamports, lamports);
    }
}

#[tokio::test]
async fn success_same_mint() {
    let wallet = Keypair::new();
    let dummy_mint = Pubkey::new_unique();
    let pt = program_test(dummy_mint, true);
    let mut context = pt.start_with_context().await;
    let (mint, mint_authority) = create_mint(&mut context).await;

    let owner_associated_token_address =
        create_associated_token_account(&mut context, &wallet.pubkey(), &mint).await;
    let nested_associated_token_address =
        create_associated_token_account(&mut context, &owner_associated_token_address, &mint).await;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::close_nested(
            &wallet.pubkey(),
            &mint,
            &mint,
            &owner_associated_token_address,
            &wallet.pubkey(),
            &spl_token::id(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &wallet],
        context.last_blockhash,
    );
    try_close_nested(
        &mut context,
        mint,
        mint_authority,
        nested_associated_token_address,
        owner_associated_token_address,
        wallet,
        transaction,
        None,
    )
    .await;
}

#[tokio::test]
async fn success_different_mints() {
    let wallet = Keypair::new();
    let dummy_mint = Pubkey::new_unique();
    let pt = program_test(dummy_mint, true);
    let mut context = pt.start_with_context().await;
    let (owner_mint, _owner_mint_authority) = create_mint(&mut context).await;
    let (nested_mint, nested_mint_authority) = create_mint(&mut context).await;

    let owner_associated_token_address =
        create_associated_token_account(&mut context, &wallet.pubkey(), &owner_mint).await;
    let nested_associated_token_address = create_associated_token_account(
        &mut context,
        &owner_associated_token_address,
        &nested_mint,
    )
    .await;
    let destination_token_address =
        create_associated_token_account(&mut context, &wallet.pubkey(), &nested_mint).await;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::close_nested(
            &wallet.pubkey(),
            &owner_mint,
            &nested_mint,
            &destination_token_address,
            &wallet.pubkey(),
            &spl_token::id(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &wallet],
        context.last_blockhash,
    );
    try_close_nested(
        &mut context,
        nested_mint,
        nested_mint_authority,
        nested_associated_token_address,
        destination_token_address,
        wallet,
        transaction,
        None,
    )
    .await;
}

#[tokio::test]
async fn fail_missing_wallet_signature() {
    let wallet = Keypair::new();
    let dummy_mint = Pubkey::new_unique();
    let pt = program_test(dummy_mint, true);
    let mut context = pt.start_with_context().await;
    let (mint, mint_authority) = create_mint(&mut context).await;

    let owner_associated_token_address =
        create_associated_token_account(&mut context, &wallet.pubkey(), &mint).await;

    let nested_associated_token_address =
        create_associated_token_account(&mut context, &owner_associated_token_address, &mint).await;

    let mut close = instruction::close_nested(
        &wallet.pubkey(),
        &mint,
        &mint,
        &owner_associated_token_address,
        &wallet.pubkey(),
        &spl_token::id(),
    );
    close.accounts[6] = AccountMeta::new(wallet.pubkey(), false);
    let transaction = Transaction::new_signed_with_payer(
        &[close],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
    try_close_nested(
        &mut context,
        mint,
        mint_authority,
        nested_associated_token_address,
        owner_associated_token_address,
        wallet,
        transaction,
        Some(InstructionError::MissingRequiredSignature),
    )
    .await;
}

#[tokio::test]
async fn fail_wrong_signer() {
    let wallet = Keypair::new();
    let wrong_wallet = Keypair::new();
    let dummy_mint = Pubkey::new_unique();
    let pt = program_test(dummy_mint, true);
    let mut context = pt.start_with_context().await;
    let (mint, mint_authority) = create_mint(&mut context).await;

    let owner_associated_token_address =
        create_associated_token_account(&mut context, &wrong_wallet.pubkey(), &mint).await;
    let nested_associated_token_address =
        create_associated_token_account(&mut context, &owner_associated_token_address, &mint).await;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::close_nested(
            &wallet.pubkey(),
            &mint,
            &mint,
            &owner_associated_token_address,
            &wallet.pubkey(),
            &spl_token::id(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &wallet],
        context.last_blockhash,
    );
    try_close_nested(
        &mut context,
        mint,
        mint_authority,
        nested_associated_token_address,
        owner_associated_token_address,
        wallet,
        transaction,
        Some(InstructionError::IllegalOwner),
    )
    .await;
}

#[tokio::test]
async fn fail_not_nested() {
    let wallet = Keypair::new();
    let wrong_wallet = Keypair::new();
    let dummy_mint = Pubkey::new_unique();
    let pt = program_test(dummy_mint, true);
    let mut context = pt.start_with_context().await;
    let (mint, mint_authority) = create_mint(&mut context).await;

    let owner_associated_token_address =
        create_associated_token_account(&mut context, &wallet.pubkey(), &mint).await;
    let nested_associated_token_address =
        create_associated_token_account(&mut context, &wrong_wallet.pubkey(), &mint).await;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::close_nested(
            &wallet.pubkey(),
            &mint,
            &mint,
            &owner_associated_token_address,
            &wallet.pubkey(),
            &spl_token::id(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &wallet],
        context.last_blockhash,
    );
    try_close_nested(
        &mut context,
        mint,
        mint_authority,
        nested_associated_token_address,
        owner_associated_token_address,
        wallet,
        transaction,
        Some(InstructionError::IllegalOwner),
    )
    .await;
}

#[tokio::test]
async fn fail_wrong_address_derivation_owner() {
    let wallet = Keypair::new();
    let wrong_wallet = Keypair::new();
    let dummy_mint = Pubkey::new_unique();
    let pt = program_test(dummy_mint, true);
    let mut context = pt.start_with_context().await;
    let (mint, mint_authority) = create_mint(&mut context).await;

    let _ = create_associated_token_account(&mut context, &wallet.pubkey(), &mint).await;
    let owner_associated_token_address =
        get_associated_token_address_with_program_id(&mint, &wallet.pubkey(), &spl_token::id());
    let nested_associated_token_address =
        create_associated_token_account(&mut context, &wrong_wallet.pubkey(), &mint).await;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::close_nested(
            &wallet.pubkey(),
            &mint,
            &mint,
            &owner_associated_token_address,
            &wallet.pubkey(),
            &spl_token::id(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &wallet],
        context.last_blockhash,
    );
    try_close_nested(
        &mut context,
        mint,
        mint_authority,
        nested_associated_token_address,
        owner_associated_token_address,
        wallet,
        transaction,
        Some(InstructionError::IllegalOwner),
    )
    .await;
}

#[tokio::test]
async fn fail_owner_account_does_not_exist() {
    let wallet = Keypair::new();
    let dummy_mint = Pubkey::new_unique();
    let pt = program_test(dummy_mint, true);
    let mut context = pt.start_with_context().await;
    let (mint, mint_authority) = create_mint(&mut context).await;

    let owner_associated_token_address =
        get_associated_token_address_with_program_id(&wallet.pubkey(), &mint, &spl_token::id());
    let nested_associated_token_address =
        create_associated_token_account(&mut context, &owner_associated_token_address, &mint).await;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::close_nested(
            &wallet.pubkey(),
            &mint,
            &mint,
            &owner_associated_token_address,
            &wallet.pubkey(),
            &spl_token::id(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &wallet],
        context.last_blockhash,
    );
    try_close_nested(
        &mut context,
        mint,
        mint_authority,
        nested_associated_token_address,
        owner_associated_token_address,
        wallet,
        transaction,
        Some(InstructionError::IllegalOwner),
    )
    .await;
}

#[tokio::test]
async fn fail_wrong_spl_token_program() {
    let wallet = Keypair::new();
    let dummy_mint = Pubkey::new_unique();
    let pt = program_test(dummy_mint, true);
    let mut context = pt.start_with_context().await;
    let (mint, mint_authority) = create_mint(&mut context).await;

    let owner_associated_token_address =
        create_associated_token_account(&mut context, &wallet.pubkey(), &mint).await;
    let nested_associated_token_address =
        create_associated_token_account(&mut context, &owner_associated_token_address, &mint).await;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::close_nested(
            &wallet.pubkey(),
            &mint,
            &mint,
            &owner_associated_token_address,
            &wallet.pubkey(),
            &Pubkey::new_unique(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &wallet],
        context.last_blockhash,
    );
    try_close_nested(
        &mut context,
        mint,
        mint_authority,
        nested_associated_token_address,
        owner_associated_token_address,
        wallet,
        transaction,
        Some(InstructionError::IllegalOwner),
    )
    .await;
}
