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
    std::convert::TryInto,
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
        ExtensionType::get_account_len::<Account>(&[ExtensionType::ImmutableOwner])
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
        ExtensionType::get_account_len::<Account>(&[ExtensionType::ImmutableOwner])
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
        ExtensionType::get_account_len::<Account>(&[
            ExtensionType::ImmutableOwner,
            ExtensionType::TransferFeeAmount
        ])
    );
}

#[tokio::test]
async fn reallocate_without_current_extension_knowledge() {
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::TransferFeeConfig {
            transfer_fee_config_authority: COption::Some(Pubkey::new_unique()).try_into().unwrap(),
            withdraw_withheld_authority: COption::Some(Pubkey::new_unique()).try_into().unwrap(),
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
        ExtensionType::get_account_len::<Account>(&[
            ExtensionType::TransferFeeAmount,
            ExtensionType::ImmutableOwner
        ])
    );
}

#[test_case(true, true ; "transfer lamports and sync")]
#[test_case(true, false ; "transfer lamports only")]
#[test_case(false, false ; "do not transfer")]
#[tokio::test]
async fn reallocate_syncs_native(transfer_lamports: bool, sync_native: bool) {
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
    let transfer_amount = 1_000_000_000;
    if transfer_lamports {
        let mut context = context.lock().await;
        let instructions = vec![system_instruction::transfer(
            &context.payer.pubkey(),
            &alice_account,
            transfer_amount,
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
        if transfer_lamports {
            assert_eq!(account_info.base.amount, transfer_amount);
        } else {
            assert_eq!(account_info.base.amount, 0);
        }
    }

    let token_account = token.get_account_info(&alice_account).await.unwrap();
    let pre_amount = token_account.base.amount;
    let pre_rent_exempt_reserve = token_account.base.is_native.unwrap();

    // reallocate resizes account to accommodate new extension
    token
        .reallocate(
            &alice_account,
            &alice.pubkey(),
            &[ExtensionType::CpiGuard],
            &[&alice],
        )
        .await
        .unwrap();

    let account = token.get_account(alice_account).await.unwrap();
    assert_eq!(
        account.data.len(),
        ExtensionType::get_account_len::<Account>(&[ExtensionType::CpiGuard])
    );
    let expected_rent_exempt_reserve = {
        let mut context = context.lock().await;
        let rent = context.banks_client.get_rent().await.unwrap();
        rent.minimum_balance(account.data.len())
    };
    let token_account = token.get_account_info(&alice_account).await.unwrap();
    let post_amount = token_account.base.amount;
    let post_rent_exempt_reserve = token_account.base.is_native.unwrap();
    if transfer_lamports && !sync_native {
        let rent_diff = expected_rent_exempt_reserve
            .checked_sub(pre_rent_exempt_reserve)
            .unwrap();
        // if we didn't sync and transferred lamports, the extra required rent
        // exemption lamports are taken from the unsynced lamports
        assert_eq!(
            post_amount,
            pre_amount
                .checked_add(transfer_amount)
                .and_then(|x| x.checked_sub(rent_diff))
                .unwrap()
        );
    } else {
        // amount of lamports should be totally unchanged otherwise
        assert_eq!(pre_amount, post_amount);
    }
    // but rent exempt reserve should change
    assert_eq!(post_rent_exempt_reserve, expected_rent_exempt_reserve);
    assert!(pre_rent_exempt_reserve < post_rent_exempt_reserve);
}
