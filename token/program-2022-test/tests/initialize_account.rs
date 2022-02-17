#![cfg(feature = "test-bpf")]

mod program_test;
use {
    program_test::TestContext,
    solana_program_test::tokio,
    solana_sdk::{
        instruction::InstructionError,
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
            transfer_fee::{self, TransferFeeAmount},
            ExtensionType, StateWithExtensions,
        },
        instruction,
        state::{Account, Mint},
    },
};

#[tokio::test]
async fn no_extensions() {
    let context = TestContext::new().await;
    let mut ctx = context.context.lock().await;
    let rent = ctx.banks_client.get_rent().await.unwrap();
    let mint_account = Keypair::new();
    let mint_authority_pubkey = Pubkey::new_unique();

    let space = ExtensionType::get_account_len::<Mint>(&[]);
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
    ctx.banks_client.process_transaction(tx).await.unwrap();

    let account = Keypair::new();
    let account_owner_pubkey = Pubkey::new_unique();
    let space = ExtensionType::get_account_len::<Account>(&[]);
    let instructions = vec![
        system_instruction::create_account(
            &ctx.payer.pubkey(),
            &account.pubkey(),
            rent.minimum_balance(space),
            space as u64,
            &spl_token_2022::id(),
        ),
        instruction::initialize_account3(
            &spl_token_2022::id(),
            &account.pubkey(),
            &mint_account.pubkey(),
            &account_owner_pubkey,
        )
        .unwrap(),
    ];
    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &account],
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await.unwrap();
    let account_info = ctx
        .banks_client
        .get_account(account.pubkey())
        .await
        .expect("get_account")
        .expect("account not none");
    assert_eq!(account_info.data.len(), spl_token_2022::state::Account::LEN);
    assert_eq!(account_info.owner, spl_token_2022::id());
    assert_eq!(account_info.lamports, rent.minimum_balance(space));
}

#[tokio::test]
async fn fail_on_invalid_mint() {
    let context = TestContext::new().await;
    let mut ctx = context.context.lock().await;
    let rent = ctx.banks_client.get_rent().await.unwrap();
    let mint_account = Keypair::new();

    let space = ExtensionType::get_account_len::<Mint>(&[]);
    let instructions = vec![system_instruction::create_account(
        &ctx.payer.pubkey(),
        &mint_account.pubkey(),
        rent.minimum_balance(space),
        space as u64,
        &spl_token_2022::id(),
    )];
    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &mint_account],
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await.unwrap();

    let account = Keypair::new();
    let account_owner_pubkey = Pubkey::new_unique();
    let space = ExtensionType::get_account_len::<Account>(&[]);
    let instructions = vec![
        system_instruction::create_account(
            &ctx.payer.pubkey(),
            &account.pubkey(),
            rent.minimum_balance(space),
            space as u64,
            &spl_token_2022::id(),
        ),
        instruction::initialize_account3(
            &spl_token_2022::id(),
            &account.pubkey(),
            &mint_account.pubkey(),
            &account_owner_pubkey,
        )
        .unwrap(),
    ];
    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &account],
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
            1,
            InstructionError::Custom(TokenError::InvalidMint as u32)
        )
    );
}

#[tokio::test]
async fn single_extension() {
    let context = TestContext::new().await;
    let mut ctx = context.context.lock().await;
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
        transfer_fee::instruction::initialize_transfer_fee_config(
            &spl_token_2022::id(),
            &mint_account.pubkey(),
            None,
            None,
            10,
            4242,
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
    ctx.banks_client.process_transaction(tx).await.unwrap();

    let account = Keypair::new();
    let account_owner_pubkey = Pubkey::new_unique();
    let space = ExtensionType::get_account_len::<Account>(&[ExtensionType::TransferFeeAmount]);
    let instructions = vec![
        system_instruction::create_account(
            &ctx.payer.pubkey(),
            &account.pubkey(),
            rent.minimum_balance(space),
            space as u64,
            &spl_token_2022::id(),
        ),
        instruction::initialize_account3(
            &spl_token_2022::id(),
            &account.pubkey(),
            &mint_account.pubkey(),
            &account_owner_pubkey,
        )
        .unwrap(),
    ];
    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &account],
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await.unwrap();
    let account_info = ctx
        .banks_client
        .get_account(account.pubkey())
        .await
        .expect("get_account")
        .expect("account not none");
    assert_eq!(
        account_info.data.len(),
        ExtensionType::get_account_len::<Account>(&[ExtensionType::TransferFeeAmount]),
    );
    assert_eq!(account_info.owner, spl_token_2022::id());
    assert_eq!(account_info.lamports, rent.minimum_balance(space));
    let state = StateWithExtensions::<Account>::unpack(&account_info.data).unwrap();
    assert_eq!(state.base.mint, mint_account.pubkey());
    assert_eq!(
        &state.get_extension_types().unwrap(),
        &[ExtensionType::TransferFeeAmount]
    );
    let unpacked_extension = state.get_extension::<TransferFeeAmount>().unwrap();
    assert_eq!(
        *unpacked_extension,
        TransferFeeAmount {
            withheld_amount: 0.into()
        }
    );
}

// TODO: add test for multiple Account extensions when memo extension is present
