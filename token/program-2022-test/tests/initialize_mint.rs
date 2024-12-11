#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{TestContext, TokenContext},
    solana_program_test::tokio,
    solana_sdk::{
        instruction::InstructionError,
        program_option::COption,
        program_pack::Pack,
        pubkey::Pubkey,
        signature::Signer,
        signer::keypair::Keypair,
        system_instruction,
        transaction::{Transaction, TransactionError},
    },
    spl_token_2022::{
        error::TokenError,
        extension::{
            confidential_transfer, confidential_transfer_fee,
            mint_close_authority::MintCloseAuthority, transfer_fee, BaseStateWithExtensions,
            ExtensionType,
        },
        instruction, native_mint,
        solana_zk_sdk::encryption::pod::elgamal::PodElGamalPubkey,
        state::Mint,
    },
    spl_token_client::token::ExtensionInitializationParams,
    std::convert::TryInto,
};

#[tokio::test]
async fn success_base() {
    let mut context = TestContext::new().await;
    context.init_token_with_mint(vec![]).await.unwrap();
    let TokenContext {
        decimals,
        mint_authority,
        token,
        ..
    } = context.token_context.unwrap();

    let mint = token.get_mint_info().await.unwrap();
    assert_eq!(mint.base.decimals, decimals);
    assert_eq!(
        mint.base.mint_authority,
        COption::Some(mint_authority.pubkey())
    );
    assert_eq!(mint.base.supply, 0);
    assert!(mint.base.is_initialized);
    assert_eq!(mint.base.freeze_authority, COption::None);
}

#[tokio::test]
async fn fail_extension_no_space() {
    let context = TestContext::new().await;
    let ctx = context.context.lock().await;
    let rent = ctx.banks_client.get_rent().await.unwrap();
    let mint_account = Keypair::new();
    let mint_authority_pubkey = Pubkey::new_unique();

    let space = Mint::LEN;
    let instructions = vec![
        system_instruction::create_account(
            &ctx.payer.pubkey(),
            &mint_account.pubkey(),
            rent.minimum_balance(space),
            space as u64,
            &spl_token_2022::id(),
        ),
        instruction::initialize_mint_close_authority(
            &spl_token_2022::id(),
            &mint_account.pubkey(),
            Some(&mint_authority_pubkey),
        )
        .unwrap(),
        instruction::initialize_mint(
            &spl_token_2022::id(),
            &mint_account.pubkey(),
            &mint_authority_pubkey,
            None,
            9,
        )
        .unwrap(),
    ];

    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &mint_account],
        ctx.last_blockhash,
    );
    let err = ctx
        .banks_client
        .process_transaction(tx)
        .await
        .unwrap_err()
        .unwrap();
    assert_eq!(
        err,
        TransactionError::InstructionError(1, InstructionError::InvalidAccountData)
    );
}

#[tokio::test]
async fn fail_extension_after_mint_init() {
    let context = TestContext::new().await;
    let ctx = context.context.lock().await;
    let rent = ctx.banks_client.get_rent().await.unwrap();
    let mint_account = Keypair::new();
    let mint_authority_pubkey = Pubkey::new_unique();

    let space =
        ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::MintCloseAuthority])
            .unwrap();
    let instructions = vec![
        system_instruction::create_account(
            &ctx.payer.pubkey(),
            &mint_account.pubkey(),
            rent.minimum_balance(space),
            space as u64,
            &spl_token_2022::id(),
        ),
        instruction::initialize_mint(
            &spl_token_2022::id(),
            &mint_account.pubkey(),
            &mint_authority_pubkey,
            None,
            9,
        )
        .unwrap(),
        instruction::initialize_mint_close_authority(
            &spl_token_2022::id(),
            &mint_account.pubkey(),
            Some(&mint_authority_pubkey),
        )
        .unwrap(),
    ];

    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &mint_account],
        ctx.last_blockhash,
    );
    let err = ctx
        .banks_client
        .process_transaction(tx)
        .await
        .unwrap_err()
        .unwrap();
    assert_eq!(
        err,
        TransactionError::InstructionError(1, InstructionError::InvalidAccountData)
    );
}

#[tokio::test]
async fn success_extension_and_base() {
    let close_authority = Some(Pubkey::new_unique());
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::MintCloseAuthority {
            close_authority,
        }])
        .await
        .unwrap();
    let TokenContext {
        decimals,
        mint_authority,
        token,
        ..
    } = context.token_context.unwrap();

    let state = token.get_mint_info().await.unwrap();
    assert_eq!(state.base.decimals, decimals);
    assert_eq!(
        state.base.mint_authority,
        COption::Some(mint_authority.pubkey())
    );
    assert_eq!(state.base.supply, 0);
    assert!(state.base.is_initialized);
    assert_eq!(state.base.freeze_authority, COption::None);
    let extension = state.get_extension::<MintCloseAuthority>().unwrap();
    assert_eq!(
        extension.close_authority,
        close_authority.try_into().unwrap(),
    );
}

#[tokio::test]
async fn fail_init_overallocated_mint() {
    let context = TestContext::new().await;
    let ctx = context.context.lock().await;
    let rent = ctx.banks_client.get_rent().await.unwrap();
    let mint_account = Keypair::new();
    let mint_authority_pubkey = Pubkey::new_unique();

    let space =
        ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::MintCloseAuthority])
            .unwrap();
    let instructions = vec![
        system_instruction::create_account(
            &ctx.payer.pubkey(),
            &mint_account.pubkey(),
            rent.minimum_balance(space),
            space as u64,
            &spl_token_2022::id(),
        ),
        instruction::initialize_mint(
            &spl_token_2022::id(),
            &mint_account.pubkey(),
            &mint_authority_pubkey,
            None,
            9,
        )
        .unwrap(),
    ];

    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &mint_account],
        ctx.last_blockhash,
    );
    let err = ctx
        .banks_client
        .process_transaction(tx)
        .await
        .unwrap_err()
        .unwrap();
    assert_eq!(
        err,
        TransactionError::InstructionError(1, InstructionError::InvalidAccountData)
    );
}

#[tokio::test]
async fn fail_account_init_after_mint_extension() {
    let context = TestContext::new().await;
    let ctx = context.context.lock().await;
    let rent = ctx.banks_client.get_rent().await.unwrap();
    let mint_account = Keypair::new();
    let mint_authority_pubkey = Pubkey::new_unique();
    let token_account = Keypair::new();

    let mint_space = ExtensionType::try_calculate_account_len::<Mint>(&[]).unwrap();
    let account_space =
        ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::MintCloseAuthority])
            .unwrap();
    let instructions = vec![
        system_instruction::create_account(
            &ctx.payer.pubkey(),
            &mint_account.pubkey(),
            rent.minimum_balance(mint_space),
            mint_space as u64,
            &spl_token_2022::id(),
        ),
        instruction::initialize_mint(
            &spl_token_2022::id(),
            &mint_account.pubkey(),
            &mint_authority_pubkey,
            None,
            9,
        )
        .unwrap(),
        system_instruction::create_account(
            &ctx.payer.pubkey(),
            &token_account.pubkey(),
            rent.minimum_balance(account_space),
            account_space as u64,
            &spl_token_2022::id(),
        ),
        instruction::initialize_mint_close_authority(
            &spl_token_2022::id(),
            &token_account.pubkey(),
            Some(&mint_authority_pubkey),
        )
        .unwrap(),
        instruction::initialize_account(
            &spl_token_2022::id(),
            &token_account.pubkey(),
            &mint_account.pubkey(),
            &mint_authority_pubkey,
        )
        .unwrap(),
    ];

    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &mint_account, &token_account],
        ctx.last_blockhash,
    );
    let err = ctx
        .banks_client
        .process_transaction(tx)
        .await
        .unwrap_err()
        .unwrap();
    assert_eq!(
        err,
        TransactionError::InstructionError(
            4,
            InstructionError::Custom(TokenError::ExtensionBaseMismatch as u32)
        )
    );
}

#[tokio::test]
async fn fail_account_init_after_mint_init() {
    let context = TestContext::new().await;
    let ctx = context.context.lock().await;
    let rent = ctx.banks_client.get_rent().await.unwrap();
    let mint_account = Keypair::new();
    let mint_authority_pubkey = Pubkey::new_unique();

    let mint_space = ExtensionType::try_calculate_account_len::<Mint>(&[]).unwrap();
    let instructions = vec![
        system_instruction::create_account(
            &ctx.payer.pubkey(),
            &mint_account.pubkey(),
            rent.minimum_balance(mint_space),
            mint_space as u64,
            &spl_token_2022::id(),
        ),
        instruction::initialize_mint(
            &spl_token_2022::id(),
            &mint_account.pubkey(),
            &mint_authority_pubkey,
            None,
            9,
        )
        .unwrap(),
        instruction::initialize_account(
            &spl_token_2022::id(),
            &mint_account.pubkey(),
            &mint_account.pubkey(),
            &mint_authority_pubkey,
        )
        .unwrap(),
    ];

    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &mint_account],
        ctx.last_blockhash,
    );
    let err = ctx
        .banks_client
        .process_transaction(tx)
        .await
        .unwrap_err()
        .unwrap();
    assert_eq!(
        err,
        TransactionError::InstructionError(2, InstructionError::InvalidAccountData)
    );
}

#[tokio::test]
async fn fail_account_init_after_mint_init_with_extension() {
    let context = TestContext::new().await;
    let ctx = context.context.lock().await;
    let rent = ctx.banks_client.get_rent().await.unwrap();
    let mint_account = Keypair::new();
    let mint_authority_pubkey = Pubkey::new_unique();

    let mint_space =
        ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::MintCloseAuthority])
            .unwrap();
    let instructions = vec![
        system_instruction::create_account(
            &ctx.payer.pubkey(),
            &mint_account.pubkey(),
            rent.minimum_balance(mint_space),
            mint_space as u64,
            &spl_token_2022::id(),
        ),
        instruction::initialize_mint_close_authority(
            &spl_token_2022::id(),
            &mint_account.pubkey(),
            Some(&mint_authority_pubkey),
        )
        .unwrap(),
        instruction::initialize_mint(
            &spl_token_2022::id(),
            &mint_account.pubkey(),
            &mint_authority_pubkey,
            None,
            9,
        )
        .unwrap(),
        instruction::initialize_account(
            &spl_token_2022::id(),
            &mint_account.pubkey(),
            &mint_account.pubkey(),
            &mint_authority_pubkey,
        )
        .unwrap(),
    ];

    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &mint_account],
        ctx.last_blockhash,
    );
    let err = ctx
        .banks_client
        .process_transaction(tx)
        .await
        .unwrap_err()
        .unwrap();
    assert_eq!(
        err,
        TransactionError::InstructionError(3, InstructionError::InvalidAccountData)
    );
}

#[tokio::test]
async fn fail_fee_init_after_mint_init() {
    let context = TestContext::new().await;
    let ctx = context.context.lock().await;
    let rent = ctx.banks_client.get_rent().await.unwrap();
    let mint_account = Keypair::new();
    let mint_authority_pubkey = Pubkey::new_unique();

    let space =
        ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::TransferFeeConfig])
            .unwrap();
    let instructions = vec![
        system_instruction::create_account(
            &ctx.payer.pubkey(),
            &mint_account.pubkey(),
            rent.minimum_balance(space),
            space as u64,
            &spl_token_2022::id(),
        ),
        instruction::initialize_mint(
            &spl_token_2022::id(),
            &mint_account.pubkey(),
            &mint_authority_pubkey,
            None,
            9,
        )
        .unwrap(),
        transfer_fee::instruction::initialize_transfer_fee_config(
            &spl_token_2022::id(),
            &mint_account.pubkey(),
            Some(&Pubkey::new_unique()),
            Some(&Pubkey::new_unique()),
            10,
            100,
        )
        .unwrap(),
    ];

    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &mint_account],
        ctx.last_blockhash,
    );
    let err = ctx
        .banks_client
        .process_transaction(tx)
        .await
        .unwrap_err()
        .unwrap();
    assert_eq!(
        err,
        TransactionError::InstructionError(1, InstructionError::InvalidAccountData)
    );
}

#[tokio::test]
async fn create_native_mint() {
    let mut context = TestContext::new().await;
    context.init_token_with_native_mint().await.unwrap();
    let TokenContext { token, .. } = context.token_context.unwrap();

    let mint = token.get_mint_info().await.unwrap();
    assert_eq!(mint.base.decimals, native_mint::DECIMALS);
    assert_eq!(mint.base.mint_authority, COption::None,);
    assert_eq!(mint.base.supply, 0);
    assert!(mint.base.is_initialized);
    assert_eq!(mint.base.freeze_authority, COption::None);
}

#[tokio::test]
async fn fail_invalid_extensions_combination() {
    let context = TestContext::new().await;
    let ctx = context.context.lock().await;
    let rent = ctx.banks_client.get_rent().await.unwrap();
    let mint_account = Keypair::new();
    let mint_authority_pubkey = Pubkey::new_unique();

    let transfer_fee_config_init_instruction =
        transfer_fee::instruction::initialize_transfer_fee_config(
            &spl_token_2022::id(),
            &mint_account.pubkey(),
            Some(&Pubkey::new_unique()),
            Some(&Pubkey::new_unique()),
            10,
            100,
        )
        .unwrap();

    let confidential_transfer_mint_init_instruction =
        confidential_transfer::instruction::initialize_mint(
            &spl_token_2022::id(),
            &mint_account.pubkey(),
            Some(Pubkey::new_unique()),
            true,
            None,
        )
        .unwrap();

    let confidential_transfer_fee_config_init_instruction =
        confidential_transfer_fee::instruction::initialize_confidential_transfer_fee_config(
            &spl_token_2022::id(),
            &mint_account.pubkey(),
            Some(Pubkey::new_unique()),
            &PodElGamalPubkey::default(),
        )
        .unwrap();

    let initialize_mint_instruction = instruction::initialize_mint(
        &spl_token_2022::id(),
        &mint_account.pubkey(),
        &mint_authority_pubkey,
        None,
        9,
    )
    .unwrap();

    // initialize transfer fee and confidential transfers, but no confidential
    // transfer fee
    let mint_space = ExtensionType::try_calculate_account_len::<Mint>(&[
        ExtensionType::TransferFeeConfig,
        ExtensionType::ConfidentialTransferMint,
    ])
    .unwrap();
    let create_account_instruction = system_instruction::create_account(
        &ctx.payer.pubkey(),
        &mint_account.pubkey(),
        rent.minimum_balance(mint_space),
        mint_space as u64,
        &spl_token_2022::id(),
    );

    let instructions = vec![
        create_account_instruction.clone(),
        transfer_fee_config_init_instruction.clone(),
        confidential_transfer_mint_init_instruction.clone(),
        initialize_mint_instruction.clone(),
    ];

    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &mint_account],
        ctx.last_blockhash,
    );
    let err = ctx
        .banks_client
        .process_transaction(tx)
        .await
        .unwrap_err()
        .unwrap();
    assert_eq!(
        err,
        TransactionError::InstructionError(
            3,
            InstructionError::Custom(TokenError::InvalidExtensionCombination as u32)
        )
    );

    // initialize transfer fee and confidential transfer fees, but no confidential
    // transfers
    let mint_space = ExtensionType::try_calculate_account_len::<Mint>(&[
        ExtensionType::TransferFeeConfig,
        ExtensionType::ConfidentialTransferFeeConfig,
    ])
    .unwrap();
    let create_account_instruction = system_instruction::create_account(
        &ctx.payer.pubkey(),
        &mint_account.pubkey(),
        rent.minimum_balance(mint_space),
        mint_space as u64,
        &spl_token_2022::id(),
    );

    let instructions = vec![
        create_account_instruction.clone(),
        transfer_fee_config_init_instruction.clone(),
        confidential_transfer_fee_config_init_instruction.clone(),
        initialize_mint_instruction.clone(),
    ];

    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &mint_account],
        ctx.last_blockhash,
    );
    let err = ctx
        .banks_client
        .process_transaction(tx)
        .await
        .unwrap_err()
        .unwrap();
    assert_eq!(
        err,
        TransactionError::InstructionError(
            3,
            InstructionError::Custom(TokenError::InvalidExtensionCombination as u32)
        )
    );

    // initialize all of transfer fee, confidential transfers, and confidential
    // transfer fees (success case)
    let mint_space = ExtensionType::try_calculate_account_len::<Mint>(&[
        ExtensionType::TransferFeeConfig,
        ExtensionType::ConfidentialTransferMint,
        ExtensionType::ConfidentialTransferFeeConfig,
    ])
    .unwrap();
    let create_account_instruction = system_instruction::create_account(
        &ctx.payer.pubkey(),
        &mint_account.pubkey(),
        rent.minimum_balance(mint_space),
        mint_space as u64,
        &spl_token_2022::id(),
    );

    let instructions = vec![
        create_account_instruction.clone(),
        transfer_fee_config_init_instruction.clone(),
        confidential_transfer_mint_init_instruction.clone(),
        confidential_transfer_fee_config_init_instruction.clone(),
        initialize_mint_instruction.clone(),
    ];

    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &mint_account],
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await.unwrap();
}
