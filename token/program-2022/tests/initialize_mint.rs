#![cfg(feature = "test-bpf")]

mod program_test;
use {
    program_test::TestContext,
    solana_program_test::{processor, tokio, ProgramTest},
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
        extension::{mint_close_authority::MintCloseAuthority, transfer_fee, ExtensionType},
        id, instruction,
        processor::Processor,
        state::Mint,
    },
    spl_token_client::token::ExtensionInitializationParams,
    std::convert::TryInto,
};

#[tokio::test]
async fn success_base() {
    let TestContext {
        decimals,
        mint_authority,
        token,
        ..
    } = TestContext::new(vec![]).await.unwrap();

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
    let program_test = ProgramTest::new("spl_token_2022", id(), processor!(Processor::process));
    let mut ctx = program_test.start_with_context().await;
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
    #[allow(clippy::useless_conversion)]
    let err: TransactionError = ctx
        .banks_client
        .process_transaction(tx)
        .await
        .unwrap_err()
        .unwrap()
        .into();
    assert_eq!(
        err,
        TransactionError::InstructionError(1, InstructionError::InvalidAccountData)
    );
}

#[tokio::test]
async fn fail_extension_after_mint_init() {
    let program_test = ProgramTest::new("spl_token_2022", id(), processor!(Processor::process));
    let mut ctx = program_test.start_with_context().await;
    let rent = ctx.banks_client.get_rent().await.unwrap();
    let mint_account = Keypair::new();
    let mint_authority_pubkey = Pubkey::new_unique();

    let space = ExtensionType::get_account_len::<Mint>(&[ExtensionType::MintCloseAuthority]);
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
    #[allow(clippy::useless_conversion)]
    let err: TransactionError = ctx
        .banks_client
        .process_transaction(tx)
        .await
        .unwrap_err()
        .unwrap()
        .into();
    assert_eq!(
        err,
        TransactionError::InstructionError(1, InstructionError::InvalidAccountData)
    );
}

#[tokio::test]
async fn success_extension_and_base() {
    let close_authority = Some(Pubkey::new_unique());
    let TestContext {
        decimals,
        mint_authority,
        token,
        ..
    } = TestContext::new(vec![ExtensionInitializationParams::MintCloseAuthority {
        close_authority: close_authority.clone(),
    }])
    .await
    .unwrap();

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
    let program_test = ProgramTest::new("spl_token_2022", id(), processor!(Processor::process));
    let mut ctx = program_test.start_with_context().await;
    let rent = ctx.banks_client.get_rent().await.unwrap();
    let mint_account = Keypair::new();
    let mint_authority_pubkey = Pubkey::new_unique();

    let space = ExtensionType::get_account_len::<Mint>(&[ExtensionType::MintCloseAuthority]);
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
    #[allow(clippy::useless_conversion)]
    let err: TransactionError = ctx
        .banks_client
        .process_transaction(tx)
        .await
        .unwrap_err()
        .unwrap()
        .into();
    assert_eq!(
        err,
        TransactionError::InstructionError(1, InstructionError::InvalidAccountData)
    );
}

#[tokio::test]
async fn fail_account_init_after_mint_extension() {
    let program_test = ProgramTest::new("spl_token_2022", id(), processor!(Processor::process));
    let mut ctx = program_test.start_with_context().await;
    let rent = ctx.banks_client.get_rent().await.unwrap();
    let mint_account = Keypair::new();
    let mint_authority_pubkey = Pubkey::new_unique();
    let token_account = Keypair::new();

    let mint_space = ExtensionType::get_account_len::<Mint>(&[]);
    let account_space =
        ExtensionType::get_account_len::<Mint>(&[ExtensionType::MintCloseAuthority]);
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
    #[allow(clippy::useless_conversion)]
    let err: TransactionError = ctx
        .banks_client
        .process_transaction(tx)
        .await
        .unwrap_err()
        .unwrap()
        .into();
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
    let program_test = ProgramTest::new("spl_token_2022", id(), processor!(Processor::process));
    let mut ctx = program_test.start_with_context().await;
    let rent = ctx.banks_client.get_rent().await.unwrap();
    let mint_account = Keypair::new();
    let mint_authority_pubkey = Pubkey::new_unique();

    let mint_space = ExtensionType::get_account_len::<Mint>(&[]);
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
    #[allow(clippy::useless_conversion)]
    let err: TransactionError = ctx
        .banks_client
        .process_transaction(tx)
        .await
        .unwrap_err()
        .unwrap()
        .into();
    assert_eq!(
        err,
        TransactionError::InstructionError(2, InstructionError::InvalidAccountData)
    );
}

#[tokio::test]
async fn fail_account_init_after_mint_init_with_extension() {
    let program_test = ProgramTest::new("spl_token_2022", id(), processor!(Processor::process));
    let mut ctx = program_test.start_with_context().await;
    let rent = ctx.banks_client.get_rent().await.unwrap();
    let mint_account = Keypair::new();
    let mint_authority_pubkey = Pubkey::new_unique();

    let mint_space = ExtensionType::get_account_len::<Mint>(&[ExtensionType::MintCloseAuthority]);
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
    #[allow(clippy::useless_conversion)]
    let err: TransactionError = ctx
        .banks_client
        .process_transaction(tx)
        .await
        .unwrap_err()
        .unwrap()
        .into();
    assert_eq!(
        err,
        TransactionError::InstructionError(3, InstructionError::InvalidAccountData)
    );
}

#[tokio::test]
async fn fail_fee_init_after_mint_init() {
    let program_test = ProgramTest::new("spl_token_2022", id(), processor!(Processor::process));
    let mut ctx = program_test.start_with_context().await;
    let rent = ctx.banks_client.get_rent().await.unwrap();
    let mint_account = Keypair::new();
    let mint_authority_pubkey = Pubkey::new_unique();

    let space = ExtensionType::get_account_len::<Mint>(&[ExtensionType::TransferFeeConfig]);
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
            &mint_account.pubkey(),
            Some(&Pubkey::new_unique()),
            Some(&Pubkey::new_unique()),
            10,
            100,
        ),
    ];

    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &mint_account],
        ctx.last_blockhash,
    );
    #[allow(clippy::useless_conversion)]
    let err: TransactionError = ctx
        .banks_client
        .process_transaction(tx)
        .await
        .unwrap_err()
        .unwrap()
        .into();
    assert_eq!(
        err,
        TransactionError::InstructionError(1, InstructionError::InvalidAccountData)
    );
}
