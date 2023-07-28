#![cfg(all(feature = "test-sbf"))]
#![cfg(twoxtx)]

mod program_test;
use {
    program_test::{TestContext, TokenContext},
    solana_program_test::tokio,
    solana_sdk::{
        instruction::InstructionError,
        pubkey::Pubkey,
        signature::Signer,
        signer::keypair::Keypair,
        system_instruction,
        transaction::{Transaction, TransactionError},
        transport::TransportError,
    },
    spl_token_2022::{
        error::TokenError,
        extension::{
            confidential_transfer::{
                self, ConfidentialTransferAccount, ConfidentialTransferMint,
                MAXIMUM_DEPOSIT_TRANSFER_AMOUNT,
            },
            BaseStateWithExtensions, ExtensionType,
        },
        instruction,
        solana_zk_token_sdk::{
            encryption::{auth_encryption::*, elgamal::*},
            zk_token_elgamal::pod::{self, Zeroable},
            zk_token_proof_instruction::*,
            zk_token_proof_program,
            zk_token_proof_state::ProofContextState,
        },
    },
    spl_token_client::{
        client::{SendTransaction, SimulateTransaction},
        token::{ExtensionInitializationParams, Token, TokenError as TokenClientError},
    },
    std::{convert::TryInto, mem::size_of},
};

#[cfg(feature = "zk-ops")]
const TEST_MAXIMUM_FEE: u64 = 100;
#[cfg(feature = "zk-ops")]
const TEST_FEE_BASIS_POINTS: u16 = 250;

#[cfg(all(feature = "zk-ops", feature = "proof-program"))]
async fn check_withheld_amount_in_mint<T>(
    token: &Token<T>,
    withdraw_withheld_authority_elgamal_keypair: &ElGamalKeypair,
    expected: u64,
) where
    T: SendTransaction + SimulateTransaction,
{
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<ConfidentialTransferMint>().unwrap();
    let decrypted_amount = extension
        .withheld_amount
        .decrypt(&withdraw_withheld_authority_elgamal_keypair.secret)
        .unwrap();
    assert_eq!(decrypted_amount, expected);
}

#[cfg(feature = "zk-ops")]
#[tokio::test]
async fn confidential_transfer_fee_config() {
    let transfer_fee_authority = Keypair::new();
    let withdraw_withheld_authority = Keypair::new();

    let confidential_transfer_authority = Keypair::new();
    let auto_approve_new_accounts = true;
    let auditor_elgamal_keypair = ElGamalKeypair::new_rand();
    let auditor_elgamal_pubkey = (*auditor_elgamal_keypair.pubkey()).into();

    let confidential_transfer_fee_authority = Keypair::new();
    let withdraw_withheld_authority_elgamal_keypair = ElGamalKeypair::new_rand();
    let withdraw_withheld_authority_elgamal_pubkey =
        (*withdraw_withheld_authority_elgamal_keypair.pubkey()).into();

    let mut context = TestContext::new().await;

    // Try invalid combinations of extensions
    let err = context
        .init_token_with_mint(vec![
            ExtensionInitializationParams::TransferFeeConfig {
                transfer_fee_config_authority: Some(transfer_fee_authority.pubkey()),
                withdraw_withheld_authority: Some(withdraw_withheld_authority.pubkey()),
                transfer_fee_basis_points: TEST_FEE_BASIS_POINTS,
                maximum_fee: TEST_MAXIMUM_FEE,
            },
            ExtensionInitializationParams::ConfidentialTransferMint {
                authority: Some(confidential_transfer_authority.pubkey()),
                auto_approve_new_accounts,
                auditor_elgamal_pubkey: Some(auditor_elgamal_pubkey),
            },
        ])
        .await
        .unwrap_err();

    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                3,
                InstructionError::Custom(TokenError::InvalidExtensionCombination as u32),
            )
        )))
    );

    let err = context
        .init_token_with_mint(vec![
            ExtensionInitializationParams::ConfidentialTransferMint {
                authority: Some(confidential_transfer_authority.pubkey()),
                auto_approve_new_accounts,
                auditor_elgamal_pubkey: Some(auditor_elgamal_pubkey),
            },
            ExtensionInitializationParams::ConfidentialTransferFeeConfig {
                authority: Some(confidential_transfer_fee_authority.pubkey()),
                withdraw_withheld_authority_elgamal_pubkey,
            },
        ])
        .await
        .unwrap_err();

    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                3,
                InstructionError::Custom(TokenError::InvalidExtensionCombination as u32),
            )
        )))
    );

    let err = context
        .init_token_with_mint(vec![
            ExtensionInitializationParams::ConfidentialTransferFeeConfig {
                authority: Some(confidential_transfer_fee_authority.pubkey()),
                withdraw_withheld_authority_elgamal_pubkey,
            },
            ExtensionInitializationParams::ConfidentialTransferFeeConfig {
                authority: Some(confidential_transfer_fee_authority.pubkey()),
                withdraw_withheld_authority_elgamal_pubkey,
            },
        ])
        .await
        .unwrap_err();

    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                3,
                InstructionError::Custom(TokenError::InvalidExtensionCombination as u32),
            )
        )))
    );

    let err = context
        .init_token_with_mint(vec![
            ExtensionInitializationParams::ConfidentialTransferFeeConfig {
                authority: Some(confidential_transfer_fee_authority.pubkey()),
                withdraw_withheld_authority_elgamal_pubkey,
            },
        ])
        .await
        .unwrap_err();

    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                2,
                InstructionError::Custom(TokenError::InvalidExtensionCombination as u32),
            )
        )))
    );

    context
        .init_token_with_mint(vec![
            ExtensionInitializationParams::TransferFeeConfig {
                transfer_fee_config_authority: Some(transfer_fee_authority.pubkey()),
                withdraw_withheld_authority: Some(withdraw_withheld_authority.pubkey()),
                transfer_fee_basis_points: TEST_FEE_BASIS_POINTS,
                maximum_fee: TEST_MAXIMUM_FEE,
            },
            ExtensionInitializationParams::ConfidentialTransferMint {
                authority: Some(confidential_transfer_authority.pubkey()),
                auto_approve_new_accounts,
                auditor_elgamal_pubkey: Some(auditor_elgamal_pubkey),
            },
            ExtensionInitializationParams::ConfidentialTransferFeeConfig {
                authority: Some(confidential_transfer_fee_authority.pubkey()),
                withdraw_withheld_authority_elgamal_pubkey,
            },
        ])
        .await
        .unwrap();
}

#[tokio::test]
async fn confidential_transfer_initialize_and_update_mint() {
    let authority = Keypair::new();
    let auto_approve_new_accounts = true;
    let auditor_elgamal_keypair = ElGamalKeypair::new_rand();
    let auditor_elgamal_pubkey = (*auditor_elgamal_keypair.pubkey()).into();

    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![
            ExtensionInitializationParams::ConfidentialTransferMint {
                authority: Some(authority.pubkey()),
                auto_approve_new_accounts,
                auditor_elgamal_pubkey: Some(auditor_elgamal_pubkey),
            },
        ])
        .await
        .unwrap();

    let TokenContext { token, .. } = context.token_context.unwrap();

    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<ConfidentialTransferMint>().unwrap();

    assert_eq!(
        extension.authority,
        Some(authority.pubkey()).try_into().unwrap()
    );
    assert_eq!(
        extension.auto_approve_new_accounts,
        auto_approve_new_accounts.into()
    );
    assert_eq!(
        extension.auditor_elgamal_pubkey,
        Some(auditor_elgamal_pubkey).try_into().unwrap()
    );

    // Change the authority
    let new_authority = Keypair::new();
    let wrong_keypair = Keypair::new();

    let err = token
        .set_authority(
            token.get_address(),
            &wrong_keypair.pubkey(),
            Some(&new_authority.pubkey()),
            instruction::AuthorityType::ConfidentialTransferMint,
            &[&wrong_keypair],
        )
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

    token
        .set_authority(
            token.get_address(),
            &authority.pubkey(),
            Some(&new_authority.pubkey()),
            instruction::AuthorityType::ConfidentialTransferMint,
            &[&authority],
        )
        .await
        .unwrap();

    // New authority can change mint parameters while the old cannot
    let new_auto_approve_new_accounts = false;
    let new_auditor_elgamal_pubkey = None;

    let err = token
        .confidential_transfer_update_mint(
            &authority.pubkey(),
            new_auto_approve_new_accounts,
            new_auditor_elgamal_pubkey,
            &[&authority],
        )
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

    token
        .confidential_transfer_update_mint(
            &new_authority.pubkey(),
            new_auto_approve_new_accounts,
            new_auditor_elgamal_pubkey,
            &[&new_authority],
        )
        .await
        .unwrap();

    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<ConfidentialTransferMint>().unwrap();
    assert_eq!(
        extension.authority,
        Some(new_authority.pubkey()).try_into().unwrap()
    );
    assert_eq!(
        extension.auto_approve_new_accounts,
        new_auto_approve_new_accounts.try_into().unwrap(),
    );
    assert_eq!(
        extension.auditor_elgamal_pubkey,
        new_auditor_elgamal_pubkey.try_into().unwrap(),
    );

    // Set new authority to None
    token
        .set_authority(
            token.get_address(),
            &new_authority.pubkey(),
            None,
            instruction::AuthorityType::ConfidentialTransferMint,
            &[&new_authority],
        )
        .await
        .unwrap();

    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<ConfidentialTransferMint>().unwrap();
    assert_eq!(extension.authority, None.try_into().unwrap());
}

#[cfg(all(feature = "zk-ops", feature = "proof-program"))]
#[tokio::test]
async fn ct_withdraw_withheld_tokens_from_mint() {
    let ConfidentialTransferMintWithKeypairs {
        ct_mint,
        ct_mint_transfer_auditor_elgamal_keypair,
        ct_mint_withdraw_withheld_authority_elgamal_keypair,
        ..
    } = ConfidentialTransferMintWithKeypairs::new();

    let ct_mint_withdraw_withheld_authority = Keypair::new();

    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![
            ExtensionInitializationParams::TransferFeeConfig {
                transfer_fee_config_authority: Some(Pubkey::new_unique()),
                withdraw_withheld_authority: Some(ct_mint_withdraw_withheld_authority.pubkey()),
                transfer_fee_basis_points: TEST_FEE_BASIS_POINTS,
                maximum_fee: TEST_MAXIMUM_FEE,
            },
            ExtensionInitializationParams::ConfidentialTransferMint {
                authority: ct_mint.authority.into(),
                auto_approve_new_accounts: ct_mint.auto_approve_new_accounts.try_into().unwrap(),
                auditor_elgamal_pubkey: ct_mint.auditor_elgamal_pubkey.into(),
                withdraw_withheld_authority_elgamal_pubkey: ct_mint
                    .withdraw_withheld_authority_elgamal_pubkey
                    .into(),
            },
        ])
        .await
        .unwrap();

    let TokenContext {
        token,
        alice,
        bob,
        mint_authority,
        decimals,
        ..
    } = context.token_context.unwrap();

    let epoch_info = test_epoch_info();

    let alice_meta = ConfidentialTokenAccountMeta::with_tokens(
        &token,
        &alice,
        None,
        false,
        false,
        &mint_authority,
        100,
        decimals,
    )
    .await;
    let bob_meta = ConfidentialTokenAccountMeta::new(&token, &bob, None, false, false).await;

    token
        .confidential_transfer_withdraw_withheld_tokens_from_mint_with_key(
            &ct_mint_withdraw_withheld_authority,
            &alice_meta.token_account,
            &alice_meta.elgamal_keypair.public,
            0_u64,
            &ct_mint.withheld_amount.try_into().unwrap(),
            &ct_mint_withdraw_withheld_authority_elgamal_keypair,
        )
        .await
        .unwrap();

    alice_meta
        .check_balances(
            &token,
            ConfidentialTokenAccountBalances {
                pending_balance_lo: 0,
                pending_balance_hi: 0,
                available_balance: 100,
                decryptable_available_balance: 100,
            },
        )
        .await;

    check_withheld_amount_in_mint(
        &token,
        &ct_mint_withdraw_withheld_authority_elgamal_keypair,
        0,
    )
    .await;

    let state = token
        .get_account_info(&alice_meta.token_account)
        .await
        .unwrap();
    let extension = state
        .get_extension::<ConfidentialTransferAccount>()
        .unwrap();

    // Test fee is 2.5% so the withheld fees should be 3
    token
        .confidential_transfer_transfer_with_fee(
            &alice_meta.token_account,
            &bob_meta.token_account,
            &alice,
            None,
            100,
            100,
            &extension.available_balance.try_into().unwrap(),
            &bob_meta.elgamal_keypair.public,
            Some(ct_mint_transfer_auditor_elgamal_keypair.public),
            &ct_mint_withdraw_withheld_authority_elgamal_keypair.public,
            &epoch_info,
        )
        .await
        .unwrap();

    let state = token
        .get_account_info(&bob_meta.token_account)
        .await
        .unwrap();
    let extension = state
        .get_extension::<ConfidentialTransferAccount>()
        .unwrap();

    assert_eq!(
        extension
            .withheld_amount
            .decrypt(&ct_mint_withdraw_withheld_authority_elgamal_keypair.secret),
        Some(3),
    );

    token
        .confidential_transfer_harvest_withheld_tokens_to_mint(&[&bob_meta.token_account])
        .await
        .unwrap();

    check_withheld_amount_in_mint(
        &token,
        &ct_mint_withdraw_withheld_authority_elgamal_keypair,
        3,
    )
    .await;

    let state = token.get_mint_info().await.unwrap();
    let ct_mint = state.get_extension::<ConfidentialTransferMint>().unwrap();

    token
        .confidential_transfer_withdraw_withheld_tokens_from_mint_with_key(
            &ct_mint_withdraw_withheld_authority,
            &alice_meta.token_account,
            &alice_meta.elgamal_keypair.public,
            3_u64,
            &ct_mint.withheld_amount.try_into().unwrap(),
            &ct_mint_withdraw_withheld_authority_elgamal_keypair,
        )
        .await
        .unwrap();

    alice_meta
        .check_balances(
            &token,
            ConfidentialTokenAccountBalances {
                pending_balance_lo: 3,
                pending_balance_hi: 0,
                available_balance: 0,
                decryptable_available_balance: 0,
            },
        )
        .await;
}

#[cfg(all(feature = "zk-ops", feature = "proof-program"))]
#[tokio::test]
async fn ct_withdraw_withheld_tokens_from_accounts() {
    let ConfidentialTransferMintWithKeypairs {
        ct_mint,
        ct_mint_transfer_auditor_elgamal_keypair,
        ct_mint_withdraw_withheld_authority_elgamal_keypair,
        ..
    } = ConfidentialTransferMintWithKeypairs::new();

    let ct_mint_withdraw_withheld_authority = Keypair::new();

    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![
            ExtensionInitializationParams::TransferFeeConfig {
                transfer_fee_config_authority: Some(Pubkey::new_unique()),
                withdraw_withheld_authority: Some(ct_mint_withdraw_withheld_authority.pubkey()),
                transfer_fee_basis_points: TEST_FEE_BASIS_POINTS,
                maximum_fee: TEST_MAXIMUM_FEE,
            },
            ExtensionInitializationParams::ConfidentialTransferMint {
                authority: ct_mint.authority.into(),
                auto_approve_new_accounts: ct_mint.auto_approve_new_accounts.try_into().unwrap(),
                auditor_elgamal_pubkey: ct_mint.auditor_elgamal_pubkey.into(),
                withdraw_withheld_authority_elgamal_pubkey: ct_mint
                    .withdraw_withheld_authority_elgamal_pubkey
                    .into(),
            },
        ])
        .await
        .unwrap();

    let TokenContext {
        token,
        alice,
        bob,
        mint_authority,
        decimals,
        ..
    } = context.token_context.unwrap();

    let epoch_info = test_epoch_info();

    let alice_meta = ConfidentialTokenAccountMeta::with_tokens(
        &token,
        &alice,
        None,
        false,
        false,
        &mint_authority,
        100,
        decimals,
    )
    .await;
    let bob_meta = ConfidentialTokenAccountMeta::new(&token, &bob, None, false, false).await;

    let state = token
        .get_account_info(&alice_meta.token_account)
        .await
        .unwrap();
    let extension = state
        .get_extension::<ConfidentialTransferAccount>()
        .unwrap();

    // Test fee is 2.5% so the withheld fees should be 3
    token
        .confidential_transfer_transfer_with_fee(
            &alice_meta.token_account,
            &bob_meta.token_account,
            &alice,
            None,
            100,
            100,
            &extension.available_balance.try_into().unwrap(),
            &bob_meta.elgamal_keypair.public,
            Some(ct_mint_transfer_auditor_elgamal_keypair.public),
            &ct_mint_withdraw_withheld_authority_elgamal_keypair.public,
            &epoch_info,
        )
        .await
        .unwrap();

    let state = token
        .get_account_info(&bob_meta.token_account)
        .await
        .unwrap();
    let extension = state
        .get_extension::<ConfidentialTransferAccount>()
        .unwrap();

    assert_eq!(
        extension
            .withheld_amount
            .decrypt(&ct_mint_withdraw_withheld_authority_elgamal_keypair.secret),
        Some(3),
    );

    token
        .confidential_transfer_withdraw_withheld_tokens_from_accounts_with_key(
            &ct_mint_withdraw_withheld_authority,
            &alice_meta.token_account,
            &alice_meta.elgamal_keypair.public,
            3_u64,
            &extension.withheld_amount.try_into().unwrap(),
            &ct_mint_withdraw_withheld_authority_elgamal_keypair,
            &[&bob_meta.token_account],
        )
        .await
        .unwrap();

    bob_meta
        .check_balances(
            &token,
            ConfidentialTokenAccountBalances {
                pending_balance_lo: 97,
                pending_balance_hi: 0,
                available_balance: 0,
                decryptable_available_balance: 0,
            },
        )
        .await;

    alice_meta
        .check_balances(
            &token,
            ConfidentialTokenAccountBalances {
                pending_balance_lo: 3,
                pending_balance_hi: 0,
                available_balance: 0,
                decryptable_available_balance: 0,
            },
        )
        .await;
}
