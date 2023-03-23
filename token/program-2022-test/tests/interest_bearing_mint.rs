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
        extension::{interest_bearing_mint::InterestBearingConfig, BaseStateWithExtensions},
        instruction::{amount_to_ui_amount, ui_amount_to_amount, AuthorityType},
        processor::Processor,
    },
    spl_token_client::token::{ExtensionInitializationParams, TokenError as TokenClientError},
    std::{convert::TryInto, sync::Arc},
};

#[tokio::test]
async fn success_initialize() {
    for (rate, rate_authority) in [(i16::MIN, None), (i16::MAX, Some(Pubkey::new_unique()))] {
        let mut context = TestContext::new().await;
        context
            .init_token_with_mint(vec![ExtensionInitializationParams::InterestBearingConfig {
                rate_authority,
                rate,
            }])
            .await
            .unwrap();
        let TokenContext { token, .. } = context.token_context.unwrap();

        let state = token.get_mint_info().await.unwrap();
        let extension = state.get_extension::<InterestBearingConfig>().unwrap();
        assert_eq!(
            Option::<Pubkey>::from(extension.rate_authority),
            rate_authority,
        );
        assert_eq!(i16::from(extension.current_rate), rate,);
        assert_eq!(i16::from(extension.pre_update_average_rate), rate,);
    }
}

#[tokio::test]
async fn update_rate() {
    let rate_authority = Keypair::new();
    let initial_rate = 500;
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::InterestBearingConfig {
            rate_authority: Some(rate_authority.pubkey()),
            rate: initial_rate,
        }])
        .await
        .unwrap();
    let TokenContext { token, .. } = context.token_context.take().unwrap();

    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<InterestBearingConfig>().unwrap();
    assert_eq!(i16::from(extension.current_rate), initial_rate);
    assert_eq!(i16::from(extension.pre_update_average_rate), initial_rate);
    let initialization_timestamp = i64::from(extension.initialization_timestamp);
    assert_eq!(
        extension.initialization_timestamp,
        extension.last_update_timestamp
    );

    // warp forward, so last update timestamp is advanced during update
    let warp_slot = 1_000;
    let initial_num_warps = 10;
    for i in 1..initial_num_warps {
        context
            .context
            .lock()
            .await
            .warp_to_slot(i * warp_slot)
            .unwrap();
    }

    // correct
    let middle_rate = 1_000;
    token
        .update_interest_rate(&rate_authority.pubkey(), middle_rate, &[&rate_authority])
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<InterestBearingConfig>().unwrap();
    assert_eq!(i16::from(extension.current_rate), middle_rate);
    assert_eq!(i16::from(extension.pre_update_average_rate), initial_rate);
    let last_update_timestamp = i64::from(extension.last_update_timestamp);
    assert!(last_update_timestamp > initialization_timestamp);

    // warp forward
    let final_num_warps = 20;
    for i in initial_num_warps..final_num_warps {
        context
            .context
            .lock()
            .await
            .warp_to_slot(i * warp_slot)
            .unwrap();
    }

    // update again, pre_update_average_rate is between the two previous
    let new_rate = 2_000;
    token
        .update_interest_rate(&rate_authority.pubkey(), new_rate, &[&rate_authority])
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<InterestBearingConfig>().unwrap();
    assert_eq!(i16::from(extension.current_rate), new_rate);
    let pre_update_average_rate = i16::from(extension.pre_update_average_rate);
    assert!(pre_update_average_rate > initial_rate);
    assert!(middle_rate > pre_update_average_rate);
    let final_update_timestamp = i64::from(extension.last_update_timestamp);
    assert!(final_update_timestamp > last_update_timestamp);

    // wrong signer
    let wrong_signer = Keypair::new();
    let err = token
        .update_interest_rate(&wrong_signer.pubkey(), 0, &[&wrong_signer])
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
    let rate_authority = Keypair::new();
    let initial_rate = 500;
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::InterestBearingConfig {
            rate_authority: Some(rate_authority.pubkey()),
            rate: initial_rate,
        }])
        .await
        .unwrap();
    let TokenContext { token, .. } = context.token_context.take().unwrap();

    // success
    let new_rate_authority = Keypair::new();
    token
        .set_authority(
            token.get_address(),
            &rate_authority.pubkey(),
            Some(&new_rate_authority.pubkey()),
            AuthorityType::InterestRate,
            &[&rate_authority],
        )
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<InterestBearingConfig>().unwrap();
    assert_eq!(
        extension.rate_authority,
        Some(new_rate_authority.pubkey()).try_into().unwrap(),
    );
    token
        .update_interest_rate(&new_rate_authority.pubkey(), 10, &[&new_rate_authority])
        .await
        .unwrap();
    let err = token
        .update_interest_rate(&rate_authority.pubkey(), 100, &[&rate_authority])
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
            &new_rate_authority.pubkey(),
            None,
            AuthorityType::InterestRate,
            &[&new_rate_authority],
        )
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<InterestBearingConfig>().unwrap();
    assert_eq!(extension.rate_authority, None.try_into().unwrap(),);

    // now all fail
    let err = token
        .update_interest_rate(&new_rate_authority.pubkey(), 50, &[&new_rate_authority])
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
        .update_interest_rate(&rate_authority.pubkey(), 5, &[&rate_authority])
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
        &ui_amount_to_amount(token_program.key, mint_info.key, "10")?,
        &[mint_info.clone(), token_program.clone()],
    )?;
    let (_, return_data) = get_return_data().unwrap();
    let amount = u64::from_le_bytes(return_data[0..8].try_into().unwrap());
    msg!("amount: {}", amount);
    if amount >= test_amount {
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
    if float_ui_amount <= 10.0 {
        return Err(ProgramError::InvalidInstructionData);
    }
    Ok(())
}

#[tokio::test]
async fn amount_conversions() {
    let rate_authority = Keypair::new();
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
    let initial_rate = i16::MAX;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::InterestBearingConfig {
            rate_authority: Some(rate_authority.pubkey()),
            rate: initial_rate,
        }])
        .await
        .unwrap();
    let TokenContext { token, .. } = context.token_context.take().unwrap();

    // warp forward, so interest is accrued
    let warp_slot: u64 = 1_000;
    let initial_num_warps: u64 = 10;
    for i in 1..initial_num_warps {
        context
            .context
            .lock()
            .await
            .warp_to_slot(i.checked_mul(warp_slot).unwrap())
            .unwrap();
    }

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
