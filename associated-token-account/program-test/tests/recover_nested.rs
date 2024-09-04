// Mark this test as BPF-only due to current `ProgramTest` limitations when
// CPIing into the system program
#![cfg(feature = "test-sbf")]

mod program_test;

use {
    program_test::{program_test, program_test_2022},
    solana_program::{pubkey::Pubkey, system_instruction},
    solana_program_test::*,
    solana_sdk::{
        instruction::{AccountMeta, InstructionError},
        signature::Signer,
        signer::keypair::Keypair,
        transaction::{Transaction, TransactionError},
    },
    spl_associated_token_account::instruction,
    spl_associated_token_account_client::address::get_associated_token_address_with_program_id,
    spl_token_2022::{
        extension::{ExtensionType, StateWithExtensionsOwned},
        state::{Account, Mint},
    },
};

async fn create_mint(context: &mut ProgramTestContext, program_id: &Pubkey) -> (Pubkey, Keypair) {
    let mint_account = Keypair::new();
    let token_mint_address = mint_account.pubkey();
    let mint_authority = Keypair::new();
    let space = ExtensionType::try_calculate_account_len::<Mint>(&[]).unwrap();
    let rent = context.banks_client.get_rent().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &mint_account.pubkey(),
                rent.minimum_balance(space),
                space as u64,
                program_id,
            ),
            spl_token_2022::instruction::initialize_mint(
                program_id,
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
    program_id: &Pubkey,
) -> Pubkey {
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::create_associated_token_account(
            &context.payer.pubkey(),
            owner,
            mint,
            program_id,
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

    get_associated_token_address_with_program_id(owner, mint, program_id)
}

#[allow(clippy::too_many_arguments)]
async fn try_recover_nested(
    context: &mut ProgramTestContext,
    program_id: &Pubkey,
    nested_mint: Pubkey,
    nested_mint_authority: Keypair,
    nested_associated_token_address: Pubkey,
    destination_token_address: Pubkey,
    wallet: Keypair,
    recover_transaction: Transaction,
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
        &[spl_token_2022::instruction::mint_to(
            program_id,
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
        .process_transaction(recover_transaction)
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

async fn check_same_mint(context: &mut ProgramTestContext, program_id: &Pubkey) {
    let wallet = Keypair::new();
    let (mint, mint_authority) = create_mint(context, program_id).await;

    let owner_associated_token_address =
        create_associated_token_account(context, &wallet.pubkey(), &mint, program_id).await;
    let nested_associated_token_address = create_associated_token_account(
        context,
        &owner_associated_token_address,
        &mint,
        program_id,
    )
    .await;

    context.last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::recover_nested(
            &wallet.pubkey(),
            &mint,
            &mint,
            program_id,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &wallet],
        context.last_blockhash,
    );
    try_recover_nested(
        context,
        program_id,
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
async fn success_same_mint_2022() {
    let dummy_mint = Pubkey::new_unique();
    let pt = program_test_2022(dummy_mint, true);
    let mut context = pt.start_with_context().await;
    check_same_mint(&mut context, &spl_token_2022::id()).await;
}

#[tokio::test]
async fn success_same_mint() {
    let dummy_mint = Pubkey::new_unique();
    let pt = program_test(dummy_mint, true);
    let mut context = pt.start_with_context().await;
    check_same_mint(&mut context, &spl_token::id()).await;
}

async fn check_different_mints(context: &mut ProgramTestContext, program_id: &Pubkey) {
    let wallet = Keypair::new();
    let (owner_mint, _owner_mint_authority) = create_mint(context, program_id).await;
    let (nested_mint, nested_mint_authority) = create_mint(context, program_id).await;

    let owner_associated_token_address =
        create_associated_token_account(context, &wallet.pubkey(), &owner_mint, program_id).await;
    let nested_associated_token_address = create_associated_token_account(
        context,
        &owner_associated_token_address,
        &nested_mint,
        program_id,
    )
    .await;
    let destination_token_address =
        create_associated_token_account(context, &wallet.pubkey(), &nested_mint, program_id).await;

    context.last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::recover_nested(
            &wallet.pubkey(),
            &owner_mint,
            &nested_mint,
            program_id,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &wallet],
        context.last_blockhash,
    );
    try_recover_nested(
        context,
        program_id,
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
async fn success_different_mints() {
    let dummy_mint = Pubkey::new_unique();
    let pt = program_test(dummy_mint, true);
    let mut context = pt.start_with_context().await;
    check_different_mints(&mut context, &spl_token::id()).await;
}

#[tokio::test]
async fn success_different_mints_2022() {
    let dummy_mint = Pubkey::new_unique();
    let pt = program_test_2022(dummy_mint, true);
    let mut context = pt.start_with_context().await;
    check_different_mints(&mut context, &spl_token_2022::id()).await;
}

async fn check_missing_wallet_signature(context: &mut ProgramTestContext, program_id: &Pubkey) {
    let wallet = Keypair::new();
    let (mint, mint_authority) = create_mint(context, program_id).await;

    let owner_associated_token_address =
        create_associated_token_account(context, &wallet.pubkey(), &mint, program_id).await;

    let nested_associated_token_address = create_associated_token_account(
        context,
        &owner_associated_token_address,
        &mint,
        program_id,
    )
    .await;

    let mut recover = instruction::recover_nested(&wallet.pubkey(), &mint, &mint, program_id);
    recover.accounts[5] = AccountMeta::new(wallet.pubkey(), false);
    context.last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[recover],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
    try_recover_nested(
        context,
        program_id,
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
async fn fail_missing_wallet_signature_2022() {
    let dummy_mint = Pubkey::new_unique();
    let pt = program_test_2022(dummy_mint, true);
    let mut context = pt.start_with_context().await;
    check_missing_wallet_signature(&mut context, &spl_token_2022::id()).await;
}

#[tokio::test]
async fn fail_missing_wallet_signature() {
    let dummy_mint = Pubkey::new_unique();
    let pt = program_test(dummy_mint, true);
    let mut context = pt.start_with_context().await;
    check_missing_wallet_signature(&mut context, &spl_token::id()).await;
}

async fn check_wrong_signer(context: &mut ProgramTestContext, program_id: &Pubkey) {
    let wallet = Keypair::new();
    let wrong_wallet = Keypair::new();
    let (mint, mint_authority) = create_mint(context, program_id).await;

    let owner_associated_token_address =
        create_associated_token_account(context, &wallet.pubkey(), &mint, program_id).await;
    let nested_associated_token_address = create_associated_token_account(
        context,
        &owner_associated_token_address,
        &mint,
        program_id,
    )
    .await;

    context.last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::recover_nested(
            &wrong_wallet.pubkey(),
            &mint,
            &mint,
            program_id,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &wrong_wallet],
        context.last_blockhash,
    );
    try_recover_nested(
        context,
        program_id,
        mint,
        mint_authority,
        nested_associated_token_address,
        owner_associated_token_address,
        wrong_wallet,
        transaction,
        Some(InstructionError::IllegalOwner),
    )
    .await;
}

#[tokio::test]
async fn fail_wrong_signer_2022() {
    let dummy_mint = Pubkey::new_unique();
    let pt = program_test_2022(dummy_mint, true);
    let mut context = pt.start_with_context().await;
    check_wrong_signer(&mut context, &spl_token_2022::id()).await;
}

#[tokio::test]
async fn fail_wrong_signer() {
    let dummy_mint = Pubkey::new_unique();
    let pt = program_test(dummy_mint, true);
    let mut context = pt.start_with_context().await;
    check_wrong_signer(&mut context, &spl_token::id()).await;
}

async fn check_not_nested(context: &mut ProgramTestContext, program_id: &Pubkey) {
    let wallet = Keypair::new();
    let wrong_wallet = Pubkey::new_unique();
    let (mint, mint_authority) = create_mint(context, program_id).await;

    let owner_associated_token_address =
        create_associated_token_account(context, &wallet.pubkey(), &mint, program_id).await;
    let nested_associated_token_address =
        create_associated_token_account(context, &wrong_wallet, &mint, program_id).await;

    context.last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::recover_nested(
            &wallet.pubkey(),
            &mint,
            &mint,
            program_id,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &wallet],
        context.last_blockhash,
    );
    try_recover_nested(
        context,
        program_id,
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
async fn fail_not_nested_2022() {
    let dummy_mint = Pubkey::new_unique();
    let pt = program_test_2022(dummy_mint, true);
    let mut context = pt.start_with_context().await;
    check_not_nested(&mut context, &spl_token_2022::id()).await;
}

#[tokio::test]
async fn fail_not_nested() {
    let dummy_mint = Pubkey::new_unique();
    let pt = program_test(dummy_mint, true);
    let mut context = pt.start_with_context().await;
    check_not_nested(&mut context, &spl_token::id()).await;
}

async fn check_wrong_address_derivation_owner(
    context: &mut ProgramTestContext,
    program_id: &Pubkey,
) {
    let wallet = Keypair::new();
    let wrong_wallet = Pubkey::new_unique();
    let (mint, mint_authority) = create_mint(context, program_id).await;

    let owner_associated_token_address =
        create_associated_token_account(context, &wallet.pubkey(), &mint, program_id).await;
    let nested_associated_token_address = create_associated_token_account(
        context,
        &owner_associated_token_address,
        &mint,
        program_id,
    )
    .await;

    let wrong_owner_associated_token_address =
        get_associated_token_address_with_program_id(&mint, &wrong_wallet, program_id);
    let mut recover = instruction::recover_nested(&wallet.pubkey(), &mint, &mint, program_id);
    recover.accounts[3] = AccountMeta::new(wrong_owner_associated_token_address, false);
    context.last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[recover],
        Some(&context.payer.pubkey()),
        &[&context.payer, &wallet],
        context.last_blockhash,
    );
    try_recover_nested(
        context,
        program_id,
        mint,
        mint_authority,
        nested_associated_token_address,
        wrong_owner_associated_token_address,
        wallet,
        transaction,
        Some(InstructionError::InvalidSeeds),
    )
    .await;
}

#[tokio::test]
async fn fail_wrong_address_derivation_owner_2022() {
    let dummy_mint = Pubkey::new_unique();
    let pt = program_test_2022(dummy_mint, true);
    let mut context = pt.start_with_context().await;
    check_wrong_address_derivation_owner(&mut context, &spl_token_2022::id()).await;
}

#[tokio::test]
async fn fail_wrong_address_derivation_owner() {
    let dummy_mint = Pubkey::new_unique();
    let pt = program_test(dummy_mint, true);
    let mut context = pt.start_with_context().await;
    check_wrong_address_derivation_owner(&mut context, &spl_token::id()).await;
}

async fn check_owner_account_does_not_exist(context: &mut ProgramTestContext, program_id: &Pubkey) {
    let wallet = Keypair::new();
    let (mint, mint_authority) = create_mint(context, program_id).await;

    let owner_associated_token_address =
        get_associated_token_address_with_program_id(&wallet.pubkey(), &mint, program_id);
    let nested_associated_token_address = create_associated_token_account(
        context,
        &owner_associated_token_address,
        &mint,
        program_id,
    )
    .await;

    context.last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::recover_nested(
            &wallet.pubkey(),
            &mint,
            &mint,
            program_id,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &wallet],
        context.last_blockhash,
    );
    try_recover_nested(
        context,
        program_id,
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
    let dummy_mint = Pubkey::new_unique();
    let pt = program_test_2022(dummy_mint, true);
    let mut context = pt.start_with_context().await;
    check_owner_account_does_not_exist(&mut context, &spl_token_2022::id()).await;
}

#[tokio::test]
async fn fail_wrong_spl_token_program() {
    let wallet = Keypair::new();
    let dummy_mint = Pubkey::new_unique();
    let pt = program_test_2022(dummy_mint, true);
    let mut context = pt.start_with_context().await;
    let program_id = spl_token_2022::id();
    let wrong_program_id = spl_token::id();
    let (mint, mint_authority) = create_mint(&mut context, &program_id).await;

    let owner_associated_token_address =
        create_associated_token_account(&mut context, &wallet.pubkey(), &mint, &program_id).await;
    let nested_associated_token_address = create_associated_token_account(
        &mut context,
        &owner_associated_token_address,
        &mint,
        &program_id,
    )
    .await;

    context.last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::recover_nested(
            &wallet.pubkey(),
            &mint,
            &mint,
            &wrong_program_id,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &wallet],
        context.last_blockhash,
    );
    try_recover_nested(
        &mut context,
        &program_id,
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
async fn fail_destination_not_wallet_ata() {
    let wallet = Keypair::new();
    let wrong_wallet = Pubkey::new_unique();
    let dummy_mint = Pubkey::new_unique();
    let pt = program_test_2022(dummy_mint, true);
    let program_id = spl_token_2022::id();
    let mut context = pt.start_with_context().await;
    let (mint, mint_authority) = create_mint(&mut context, &program_id).await;

    let owner_associated_token_address =
        create_associated_token_account(&mut context, &wallet.pubkey(), &mint, &program_id).await;
    let nested_associated_token_address = create_associated_token_account(
        &mut context,
        &owner_associated_token_address,
        &mint,
        &program_id,
    )
    .await;
    let wrong_destination_associated_token_account_address =
        create_associated_token_account(&mut context, &wrong_wallet, &mint, &program_id).await;

    let mut recover = instruction::recover_nested(&wallet.pubkey(), &mint, &mint, &program_id);
    recover.accounts[2] =
        AccountMeta::new(wrong_destination_associated_token_account_address, false);

    context.last_blockhash = context
        .banks_client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[recover],
        Some(&context.payer.pubkey()),
        &[&context.payer, &wallet],
        context.last_blockhash,
    );
    try_recover_nested(
        &mut context,
        &program_id,
        mint,
        mint_authority,
        nested_associated_token_address,
        owner_associated_token_address,
        wallet,
        transaction,
        Some(InstructionError::InvalidSeeds),
    )
    .await;
}
