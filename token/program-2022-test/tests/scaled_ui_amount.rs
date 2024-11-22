#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{keypair_clone, TestContext, TokenContext},
    solana_program_test::{
        processor,
        tokio::{self, sync::Mutex},
        ProgramTest,
    },
    solana_sdk::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        instruction::{AccountMeta, Instruction, InstructionError},
        msg,
        program::{get_return_data, invoke},
        program_error::ProgramError,
        pubkey::Pubkey,
        signature::Signer,
        signer::keypair::Keypair,
        transaction::{Transaction, TransactionError},
        transport::TransportError,
    },
    spl_token_2022::{
        error::TokenError,
        extension::{scaled_ui_amount::ScaledUiAmountConfig, BaseStateWithExtensions},
        instruction::{amount_to_ui_amount, ui_amount_to_amount, AuthorityType},
        processor::Processor,
    },
    spl_token_client::token::{ExtensionInitializationParams, TokenError as TokenClientError},
    std::{convert::TryInto, sync::Arc},
};

#[tokio::test]
async fn success_initialize() {
    for (multiplier, authority) in [
        (f64::MIN_POSITIVE, None),
        (f64::MAX, Some(Pubkey::new_unique())),
    ] {
        let mut context = TestContext::new().await;
        context
            .init_token_with_mint(vec![ExtensionInitializationParams::ScaledUiAmountConfig {
                authority,
                multiplier,
            }])
            .await
            .unwrap();
        let TokenContext { token, .. } = context.token_context.unwrap();

        let state = token.get_mint_info().await.unwrap();
        let extension = state.get_extension::<ScaledUiAmountConfig>().unwrap();
        assert_eq!(Option::<Pubkey>::from(extension.authority), authority,);
        assert_eq!(f64::from(extension.multiplier), multiplier);
        assert_eq!(f64::from(extension.new_multiplier), multiplier);
        assert_eq!(i64::from(extension.new_multiplier_effective_timestamp), 0);
    }
}

#[tokio::test]
async fn fail_initialize_with_interest_bearing() {
    let authority = None;
    let mut context = TestContext::new().await;
    let err = context
        .init_token_with_mint(vec![
            ExtensionInitializationParams::ScaledUiAmountConfig {
                authority,
                multiplier: 1.0,
            },
            ExtensionInitializationParams::InterestBearingConfig {
                rate_authority: None,
                rate: 0,
            },
        ])
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                3,
                InstructionError::Custom(TokenError::InvalidExtensionCombination as u32)
            )
        )))
    );
}

#[tokio::test]
async fn fail_initialize_with_bad_multiplier() {
    let mut context = TestContext::new().await;
    let err = context
        .init_token_with_mint(vec![ExtensionInitializationParams::ScaledUiAmountConfig {
            authority: None,
            multiplier: 0.0,
        }])
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                1,
                InstructionError::Custom(TokenError::InvalidScale as u32)
            )
        )))
    );
}

#[tokio::test]
async fn update_multiplier() {
    let authority = Keypair::new();
    let initial_multiplier = 5.0;
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::ScaledUiAmountConfig {
            authority: Some(authority.pubkey()),
            multiplier: initial_multiplier,
        }])
        .await
        .unwrap();
    let TokenContext { token, .. } = context.token_context.take().unwrap();

    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<ScaledUiAmountConfig>().unwrap();
    assert_eq!(f64::from(extension.multiplier), initial_multiplier);
    assert_eq!(f64::from(extension.new_multiplier), initial_multiplier);

    // correct
    let new_multiplier = 10.0;
    token
        .update_multiplier(&authority.pubkey(), new_multiplier, 0, &[&authority])
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<ScaledUiAmountConfig>().unwrap();
    assert_eq!(f64::from(extension.multiplier), new_multiplier);
    assert_eq!(f64::from(extension.new_multiplier), new_multiplier);
    assert_eq!(i64::from(extension.new_multiplier_effective_timestamp), 0);

    // fail, bad number
    let err = token
        .update_multiplier(&authority.pubkey(), f64::INFINITY, 0, &[&authority])
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::InvalidScale as u32)
            )
        )))
    );

    // correct in the future
    let newest_multiplier = 100.0;
    token
        .update_multiplier(
            &authority.pubkey(),
            newest_multiplier,
            i64::MAX,
            &[&authority],
        )
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<ScaledUiAmountConfig>().unwrap();
    assert_eq!(f64::from(extension.multiplier), new_multiplier);
    assert_eq!(f64::from(extension.new_multiplier), newest_multiplier);
    assert_eq!(
        i64::from(extension.new_multiplier_effective_timestamp),
        i64::MAX
    );

    // wrong signer
    let wrong_signer = Keypair::new();
    let err = token
        .update_multiplier(&wrong_signer.pubkey(), 1.0, 0, &[&wrong_signer])
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::OwnerMismatch as u32)
            )
        )))
    );
}

#[tokio::test]
async fn set_authority() {
    let authority = Keypair::new();
    let initial_multiplier = 500.0;
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::ScaledUiAmountConfig {
            authority: Some(authority.pubkey()),
            multiplier: initial_multiplier,
        }])
        .await
        .unwrap();
    let TokenContext { token, .. } = context.token_context.take().unwrap();

    // success
    let new_authority = Keypair::new();
    token
        .set_authority(
            token.get_address(),
            &authority.pubkey(),
            Some(&new_authority.pubkey()),
            AuthorityType::ScaledUiAmount,
            &[&authority],
        )
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<ScaledUiAmountConfig>().unwrap();
    assert_eq!(
        extension.authority,
        Some(new_authority.pubkey()).try_into().unwrap(),
    );
    token
        .update_multiplier(&new_authority.pubkey(), 10.0, 0, &[&new_authority])
        .await
        .unwrap();
    let err = token
        .update_multiplier(&authority.pubkey(), 100.0, 0, &[&authority])
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::OwnerMismatch as u32)
            )
        )))
    );

    // set to none
    token
        .set_authority(
            token.get_address(),
            &new_authority.pubkey(),
            None,
            AuthorityType::ScaledUiAmount,
            &[&new_authority],
        )
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<ScaledUiAmountConfig>().unwrap();
    assert_eq!(extension.authority, None.try_into().unwrap(),);

    // now all fail
    let err = token
        .update_multiplier(&new_authority.pubkey(), 50.0, 0, &[&new_authority])
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::NoAuthorityExists as u32)
            )
        )))
    );
    let err = token
        .update_multiplier(&authority.pubkey(), 5.5, 0, &[&authority])
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::NoAuthorityExists as u32)
            )
        )))
    );
}

// test program to CPI into token to get ui amounts
fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    _input: &[u8],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_info = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    // 10 tokens, with 9 decimal places
    let test_amount = 10_000_000_000;
    // "10" as an amount should be smaller than test_amount due to interest
    invoke(
        &ui_amount_to_amount(token_program.key, mint_info.key, "50")?,
        &[mint_info.clone(), token_program.clone()],
    )?;
    let (_, return_data) = get_return_data().unwrap();
    let amount = u64::from_le_bytes(return_data[0..8].try_into().unwrap());
    msg!("amount: {}", amount);
    if amount != test_amount {
        return Err(ProgramError::InvalidInstructionData);
    }

    // test_amount as a UI amount should be larger due to interest
    invoke(
        &amount_to_ui_amount(token_program.key, mint_info.key, test_amount)?,
        &[mint_info.clone(), token_program.clone()],
    )?;
    let (_, return_data) = get_return_data().unwrap();
    let ui_amount = String::from_utf8(return_data).unwrap();
    msg!("ui amount: {}", ui_amount);
    let float_ui_amount = ui_amount.parse::<f64>().unwrap();
    if float_ui_amount != 50.0 {
        return Err(ProgramError::InvalidInstructionData);
    }
    Ok(())
}

#[tokio::test]
async fn amount_conversions() {
    let authority = Keypair::new();
    let mut program_test = ProgramTest::default();
    program_test.prefer_bpf(false);
    program_test.add_program(
        "spl_token_2022",
        spl_token_2022::id(),
        processor!(Processor::process),
    );
    let program_id = Pubkey::new_unique();
    program_test.add_program(
        "ui_amount_to_amount",
        program_id,
        processor!(process_instruction),
    );

    let context = program_test.start_with_context().await;
    let payer = keypair_clone(&context.payer);
    let last_blockhash = context.last_blockhash;
    let context = Arc::new(Mutex::new(context));
    let mut context = TestContext {
        context,
        token_context: None,
    };
    let initial_multiplier = 5.0;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::ScaledUiAmountConfig {
            authority: Some(authority.pubkey()),
            multiplier: initial_multiplier,
        }])
        .await
        .unwrap();
    let TokenContext { token, .. } = context.token_context.take().unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new_readonly(*token.get_address(), false),
                AccountMeta::new_readonly(spl_token_2022::id(), false),
            ],
            data: vec![],
        }],
        Some(&payer.pubkey()),
        &[&payer],
        last_blockhash,
    );
    context
        .context
        .lock()
        .await
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}
