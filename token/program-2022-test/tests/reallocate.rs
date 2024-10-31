#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{TestContext, TokenContext},
    solana_program_test::tokio,
    solana_sdk::{
        instruction::InstructionError,
        program_option::COption,
        pubkey::Pubkey,
        signature::Signer,
        signer::keypair::Keypair,
        system_instruction,
        transaction::{Transaction, TransactionError},
        transport::TransportError,
    },
    spl_token_2022::{error::TokenError, extension::ExtensionType, state::Account},
    spl_token_client::token::{ExtensionInitializationParams, TokenError as TokenClientError},
    test_case::test_case,
};

#[tokio::test]
async fn reallocate() {
    let mut context = TestContext::new().await;
    context.init_token_with_mint(vec![]).await.unwrap();
    let TokenContext {
        token,
        alice,
        mint_authority,
        ..
    } = context.token_context.unwrap();

    // reallocate fails on wrong account type
    let error = token
        .reallocate(
            token.get_address(),
            &mint_authority.pubkey(),
            &[ExtensionType::ImmutableOwner],
            &[&mint_authority],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(0, InstructionError::InvalidAccountData)
        )))
    );

    // create account just large enough for base
    let alice_account = Keypair::new();
    token
        .create_auxiliary_token_account(&alice_account, &alice.pubkey())
        .await
        .unwrap();
    let alice_account = alice_account.pubkey();

    // reallocate fails on invalid extension type
    let error = token
        .reallocate(
            &alice_account,
            &alice.pubkey(),
            &[ExtensionType::MintCloseAuthority],
            &[&alice],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::InvalidState as u32)
            )
        )))
    );

    // reallocate fails on invalid authority
    let error = token
        .reallocate(
            &alice_account,
            &mint_authority.pubkey(),
            &[ExtensionType::ImmutableOwner],
            &[&mint_authority],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::OwnerMismatch as u32)
            )
        )))
    );

    // reallocate succeeds
    token
        .reallocate(
            &alice_account,
            &alice.pubkey(),
            &[ExtensionType::ImmutableOwner],
            &[&alice],
        )
        .await
        .unwrap();
    let account = token.get_account(alice_account).await.unwrap();
    assert_eq!(
        account.data.len(),
        ExtensionType::try_calculate_account_len::<Account>(&[ExtensionType::ImmutableOwner])
            .unwrap()
    );

    // reallocate succeeds with noop if account is already large enough
    token.get_new_latest_blockhash().await.unwrap();
    token
        .reallocate(
            &alice_account,
            &alice.pubkey(),
            &[ExtensionType::ImmutableOwner],
            &[&alice],
        )
        .await
        .unwrap();
    let account = token.get_account(alice_account).await.unwrap();
    assert_eq!(
        account.data.len(),
        ExtensionType::try_calculate_account_len::<Account>(&[ExtensionType::ImmutableOwner])
            .unwrap()
    );

    // reallocate only reallocates enough for new extension, and dedupes extensions
    token
        .reallocate(
            &alice_account,
            &alice.pubkey(),
            &[
                ExtensionType::ImmutableOwner,
                ExtensionType::ImmutableOwner,
                ExtensionType::TransferFeeAmount,
                ExtensionType::TransferFeeAmount,
            ],
            &[&alice],
        )
        .await
        .unwrap();
    let account = token.get_account(alice_account).await.unwrap();
    assert_eq!(
        account.data.len(),
        ExtensionType::try_calculate_account_len::<Account>(&[
            ExtensionType::ImmutableOwner,
            ExtensionType::TransferFeeAmount
        ])
        .unwrap()
    );
}

#[tokio::test]
async fn reallocate_without_current_extension_knowledge() {
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::TransferFeeConfig {
            transfer_fee_config_authority: COption::Some(Pubkey::new_unique()).into(),
            withdraw_withheld_authority: COption::Some(Pubkey::new_unique()).into(),
            transfer_fee_basis_points: 250,
            maximum_fee: 10_000_000,
        }])
        .await
        .unwrap();
    let TokenContext { token, alice, .. } = context.token_context.unwrap();

    // create account just large enough for TransferFeeAmount extension
    let alice_account = Keypair::new();
    token
        .create_auxiliary_token_account(&alice_account, &alice.pubkey())
        .await
        .unwrap();
    let alice_account = alice_account.pubkey();

    // reallocate resizes account to accommodate new and existing extensions
    token
        .reallocate(
            &alice_account,
            &alice.pubkey(),
            &[ExtensionType::ImmutableOwner],
            &[&alice],
        )
        .await
        .unwrap();
    let account = token.get_account(alice_account).await.unwrap();
    assert_eq!(
        account.data.len(),
        ExtensionType::try_calculate_account_len::<Account>(&[
            ExtensionType::TransferFeeAmount,
            ExtensionType::ImmutableOwner
        ])
        .unwrap()
    );
}

#[test_case(&[ExtensionType::CpiGuard], 1_000_000_000, true ; "transfer more than new rent and sync")]
#[test_case(&[ExtensionType::CpiGuard], 1_000_000_000, false ; "transfer more than new rent")]
#[test_case(&[ExtensionType::CpiGuard], 1, true ; "transfer less than new rent and sync")]
#[test_case(&[ExtensionType::CpiGuard], 1, false ; "transfer less than new rent")]
#[test_case(&[ExtensionType::CpiGuard], 0, false ; "no transfer with extension")]
#[test_case(&[], 1_000_000_000, true ; "transfer lamports and sync without extension")]
#[test_case(&[], 1_000_000_000, false ; "transfer lamports without extension")]
#[test_case(&[], 0, false ; "no transfer without extension")]
#[tokio::test]
async fn reallocate_updates_native_rent_exemption(
    extensions: &[ExtensionType],
    transfer_lamports: u64,
    sync_native: bool,
) {
    let mut context = TestContext::new().await;
    context.init_token_with_native_mint().await.unwrap();
    let TokenContext { token, alice, .. } = context.token_context.unwrap();
    let context = context.context.clone();

    let alice_account = Keypair::new();
    token
        .create_auxiliary_token_account(&alice_account, &alice.pubkey())
        .await
        .unwrap();
    let alice_account = alice_account.pubkey();

    // transfer more lamports
    if transfer_lamports > 0 {
        let context = context.lock().await;
        let instructions = vec![system_instruction::transfer(
            &context.payer.pubkey(),
            &alice_account,
            transfer_lamports,
        )];
        let tx = Transaction::new_signed_with_payer(
            &instructions,
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );
        context.banks_client.process_transaction(tx).await.unwrap();
    }

    // amount in the account should be 0 no matter what
    let account_info = token.get_account_info(&alice_account).await.unwrap();
    assert_eq!(account_info.base.amount, 0);

    if sync_native {
        token.sync_native(&alice_account).await.unwrap();
        let account_info = token.get_account_info(&alice_account).await.unwrap();
        assert_eq!(account_info.base.amount, transfer_lamports);
    }

    let token_account = token.get_account_info(&alice_account).await.unwrap();
    let pre_amount = token_account.base.amount;
    let pre_rent_exempt_reserve = token_account.base.is_native.unwrap();

    // reallocate resizes account to accommodate new extension
    token
        .reallocate(&alice_account, &alice.pubkey(), extensions, &[&alice])
        .await
        .unwrap();

    let account = token.get_account(alice_account).await.unwrap();
    assert_eq!(
        account.data.len(),
        ExtensionType::try_calculate_account_len::<Account>(extensions).unwrap()
    );
    let expected_rent_exempt_reserve = {
        let context = context.lock().await;
        let rent = context.banks_client.get_rent().await.unwrap();
        rent.minimum_balance(account.data.len())
    };
    let token_account = token.get_account_info(&alice_account).await.unwrap();
    let post_amount = token_account.base.amount;
    let post_rent_exempt_reserve = token_account.base.is_native.unwrap();
    // amount of lamports should be totally unchanged
    assert_eq!(pre_amount, post_amount);
    // but rent exempt reserve should change
    assert_eq!(post_rent_exempt_reserve, expected_rent_exempt_reserve);
    if extensions.is_empty() {
        assert_eq!(pre_rent_exempt_reserve, post_rent_exempt_reserve);
    } else {
        assert!(pre_rent_exempt_reserve < post_rent_exempt_reserve);
    }
}
