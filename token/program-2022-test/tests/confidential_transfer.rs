#![cfg(feature = "test-sbf")]

mod program_test;
use {
    bytemuck::Zeroable,
    program_test::{
        ConfidentialTokenAccountBalances, ConfidentialTokenAccountMeta, ConfidentialTransferOption,
        TestContext, TokenContext,
    },
    solana_program_test::tokio,
    solana_sdk::{
        instruction::InstructionError,
        pubkey::Pubkey,
        signature::Signer,
        signer::{keypair::Keypair, signers::Signers},
        system_instruction,
        transaction::{Transaction, TransactionError},
        transport::TransportError,
    },
    spl_elgamal_registry::state::ELGAMAL_REGISTRY_ACCOUNT_LEN,
    spl_record::state::RecordData,
    spl_token_2022::{
        error::TokenError,
        extension::{
            confidential_transfer::{
                self,
                account_info::{EmptyAccountAccountInfo, TransferAccountInfo, WithdrawAccountInfo},
                ConfidentialTransferAccount, MAXIMUM_DEPOSIT_TRANSFER_AMOUNT,
            },
            BaseStateWithExtensions, ExtensionType,
        },
        solana_zk_sdk::{
            encryption::{auth_encryption::*, elgamal::*, pod::elgamal::PodElGamalCiphertext},
            zk_elgamal_proof_program::proof_data::*,
        },
    },
    spl_token_client::{
        client::ProgramBanksClientProcessTransaction,
        token::{
            ExtensionInitializationParams, ProofAccount, ProofAccountWithCiphertext, Token,
            TokenError as TokenClientError, TokenResult,
        },
    },
    spl_token_confidential_transfer_proof_extraction::instruction::{ProofData, ProofLocation},
    spl_token_confidential_transfer_proof_generation::{
        transfer::TransferProofData, transfer_with_fee::TransferWithFeeProofData,
        withdraw::WithdrawProofData,
    },
    std::convert::TryInto,
};

#[cfg(feature = "zk-ops")]
const TEST_MAXIMUM_FEE: u64 = 100;
#[cfg(feature = "zk-ops")]
const TEST_FEE_BASIS_POINTS: u16 = 250;

async fn configure_account_with_option<S: Signers>(
    token: &Token<ProgramBanksClientProcessTransaction>,
    account: &Pubkey,
    authority: &Pubkey,
    elgamal_keypair: &ElGamalKeypair,
    aes_key: &AeKey,
    signing_keypairs: &S,
    option: ConfidentialTransferOption,
) -> TokenResult<()> {
    match option {
        ConfidentialTransferOption::InstructionData => {
            token
                .confidential_transfer_configure_token_account(
                    account,
                    authority,
                    None,
                    None,
                    elgamal_keypair,
                    aes_key,
                    signing_keypairs,
                )
                .await
        }
        ConfidentialTransferOption::RecordAccount => {
            let pubkey_validity_proof_data = PubkeyValidityProofData::new(elgamal_keypair).unwrap();

            let pubkey_validity_proof_record_account = Keypair::new();
            let record_account_authority = Keypair::new();

            token
                .confidential_transfer_create_record_account(
                    &pubkey_validity_proof_record_account.pubkey(),
                    &record_account_authority.pubkey(),
                    &pubkey_validity_proof_data,
                    &pubkey_validity_proof_record_account,
                    &record_account_authority,
                )
                .await
                .unwrap();

            let pubkey_validity_proof_account = ProofAccount::RecordAccount(
                pubkey_validity_proof_record_account.pubkey(),
                RecordData::WRITABLE_START_INDEX as u32,
            );

            let result = token
                .confidential_transfer_configure_token_account(
                    account,
                    authority,
                    Some(&pubkey_validity_proof_account),
                    None,
                    elgamal_keypair,
                    aes_key,
                    signing_keypairs,
                )
                .await;

            token
                .confidential_transfer_close_record_account(
                    &pubkey_validity_proof_record_account.pubkey(),
                    account,
                    &record_account_authority.pubkey(),
                    &[&record_account_authority],
                )
                .await
                .unwrap();

            result
        }
        ConfidentialTransferOption::ContextStateAccount => {
            let pubkey_validity_proof_data = PubkeyValidityProofData::new(elgamal_keypair).unwrap();

            let pubkey_validity_proof_context_account = Keypair::new();
            let context_account_authority = Keypair::new();

            token
                .confidential_transfer_create_context_state_account(
                    &pubkey_validity_proof_context_account.pubkey(),
                    &context_account_authority.pubkey(),
                    &pubkey_validity_proof_data,
                    false,
                    &[&pubkey_validity_proof_context_account],
                )
                .await
                .unwrap();

            let pubkey_validity_proof_account =
                ProofAccount::ContextAccount(pubkey_validity_proof_context_account.pubkey());

            let result = token
                .confidential_transfer_configure_token_account(
                    account,
                    authority,
                    Some(&pubkey_validity_proof_account),
                    None,
                    elgamal_keypair,
                    aes_key,
                    signing_keypairs,
                )
                .await;

            token
                .confidential_transfer_close_context_state_account(
                    &pubkey_validity_proof_context_account.pubkey(),
                    account,
                    &context_account_authority.pubkey(),
                    &[&context_account_authority],
                )
                .await
                .unwrap();

            result
        }
    }
}

#[tokio::test]
async fn confidential_transfer_configure_token_account() {
    confidential_transfer_configure_token_account_with_option(
        ConfidentialTransferOption::InstructionData,
    )
    .await;
    confidential_transfer_configure_token_account_with_option(
        ConfidentialTransferOption::RecordAccount,
    )
    .await;
    confidential_transfer_configure_token_account_with_option(
        ConfidentialTransferOption::ContextStateAccount,
    )
    .await;
}

async fn confidential_transfer_configure_token_account_with_option(
    option: ConfidentialTransferOption,
) {
    let authority = Keypair::new();
    let auto_approve_new_accounts = false;
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

    let TokenContext { token, alice, .. } = context.token_context.unwrap();
    let alice_account_keypair = Keypair::new();
    token
        .create_auxiliary_token_account_with_extension_space(
            &alice_account_keypair,
            &alice.pubkey(),
            vec![ExtensionType::ConfidentialTransferAccount],
        )
        .await
        .unwrap();
    let elgamal_keypair =
        ElGamalKeypair::new_from_signer(&alice, &alice_account_keypair.pubkey().to_bytes())
            .unwrap();
    let aes_key =
        AeKey::new_from_signer(&alice, &alice_account_keypair.pubkey().to_bytes()).unwrap();

    let alice_meta = ConfidentialTokenAccountMeta {
        token_account: alice_account_keypair.pubkey(),
        elgamal_keypair,
        aes_key,
    };

    configure_account_with_option(
        &token,
        &alice_meta.token_account,
        &alice.pubkey(),
        &alice_meta.elgamal_keypair,
        &alice_meta.aes_key,
        &[&alice],
        option,
    )
    .await
    .unwrap();

    let alice_elgamal_pubkey = (*alice_meta.elgamal_keypair.pubkey()).into();

    let state = token
        .get_account_info(&alice_meta.token_account)
        .await
        .unwrap();
    let extension = state
        .get_extension::<ConfidentialTransferAccount>()
        .unwrap();
    assert!(!bool::from(&extension.approved));
    assert!(bool::from(&extension.allow_confidential_credits));
    assert_eq!(extension.elgamal_pubkey, alice_elgamal_pubkey);
    assert_eq!(
        alice_meta
            .aes_key
            .decrypt(&(extension.decryptable_available_balance.try_into().unwrap()))
            .unwrap(),
        0
    );

    token
        .confidential_transfer_approve_account(
            &alice_meta.token_account,
            &authority.pubkey(),
            &[&authority],
        )
        .await
        .unwrap();

    let state = token
        .get_account_info(&alice_meta.token_account)
        .await
        .unwrap();
    let extension = state
        .get_extension::<ConfidentialTransferAccount>()
        .unwrap();
    assert!(bool::from(&extension.approved));

    // Configuring an already initialized account should produce an error
    let err = configure_account_with_option(
        &token,
        &alice_meta.token_account,
        &alice.pubkey(),
        &alice_meta.elgamal_keypair,
        &alice_meta.aes_key,
        &[&alice],
        option,
    )
    .await
    .unwrap_err();

    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::ExtensionAlreadyInitialized as u32),
            )
        )))
    );
}

#[tokio::test]
async fn confidential_transfer_fail_approving_account_on_wrong_mint() {
    let authority = Keypair::new();
    let auto_approve_new_accounts = false;
    let auditor_elgamal_keypair = ElGamalKeypair::new_rand();
    let auditor_elgamal_pubkey = (*auditor_elgamal_keypair.pubkey()).into();

    let mut context_a = TestContext::new().await;
    context_a
        .init_token_with_mint(vec![
            ExtensionInitializationParams::ConfidentialTransferMint {
                authority: Some(authority.pubkey()),
                auto_approve_new_accounts,
                auditor_elgamal_pubkey: Some(auditor_elgamal_pubkey),
            },
        ])
        .await
        .unwrap();

    let token_a_context = context_a.token_context.unwrap();

    let mut context_b = TestContext {
        context: context_a.context.clone(),
        token_context: None,
    };
    context_b
        .init_token_with_mint(vec![
            ExtensionInitializationParams::ConfidentialTransferMint {
                authority: Some(authority.pubkey()),
                auto_approve_new_accounts,
                auditor_elgamal_pubkey: Some(auditor_elgamal_pubkey),
            },
        ])
        .await
        .unwrap();
    let TokenContext { token, alice, .. } = context_b.token_context.unwrap();
    let alice_meta = ConfidentialTokenAccountMeta::new(&token, &alice, None, false, false).await;

    let err = token_a_context
        .token
        .confidential_transfer_approve_account(
            &alice_meta.token_account,
            &authority.pubkey(),
            &[&authority],
        )
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::MintMismatch as u32)
            )
        )))
    );
}

#[tokio::test]
async fn confidential_transfer_enable_disable_confidential_credits() {
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

    let TokenContext {
        token,
        alice,
        mint_authority,
        decimals,
        ..
    } = context.token_context.unwrap();
    let alice_meta = ConfidentialTokenAccountMeta::new(&token, &alice, None, false, false).await;

    token
        .confidential_transfer_disable_confidential_credits(
            &alice_meta.token_account,
            &alice.pubkey(),
            &[&alice],
        )
        .await
        .unwrap();
    let state = token
        .get_account_info(&alice_meta.token_account)
        .await
        .unwrap();
    let extension = state
        .get_extension::<ConfidentialTransferAccount>()
        .unwrap();
    assert!(!bool::from(&extension.allow_confidential_credits));

    token
        .mint_to(
            &alice_meta.token_account,
            &mint_authority.pubkey(),
            10,
            &[&mint_authority],
        )
        .await
        .unwrap();

    let err = token
        .confidential_transfer_deposit(
            &alice_meta.token_account,
            &alice.pubkey(),
            10,
            decimals,
            &[&alice],
        )
        .await
        .unwrap_err();

    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    TokenError::ConfidentialTransferDepositsAndTransfersDisabled as u32
                )
            )
        )))
    );

    token
        .confidential_transfer_enable_confidential_credits(
            &alice_meta.token_account,
            &alice.pubkey(),
            &[&alice],
        )
        .await
        .unwrap();
    let state = token
        .get_account_info(&alice_meta.token_account)
        .await
        .unwrap();
    let extension = state
        .get_extension::<ConfidentialTransferAccount>()
        .unwrap();
    assert!(bool::from(&extension.allow_confidential_credits));

    // Refresh the blockhash since we're doing the same thing twice in a row
    token.get_new_latest_blockhash().await.unwrap();
    token
        .confidential_transfer_deposit(
            &alice_meta.token_account,
            &alice.pubkey(),
            10,
            decimals,
            &[&alice],
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn confidential_transfer_enable_disable_non_confidential_credits() {
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

    let TokenContext {
        token,
        alice,
        bob,
        mint_authority,
        ..
    } = context.token_context.unwrap();
    let alice_meta = ConfidentialTokenAccountMeta::new(&token, &alice, None, false, false).await;
    let bob_meta = ConfidentialTokenAccountMeta::new(&token, &bob, None, false, false).await;

    token
        .mint_to(
            &alice_meta.token_account,
            &mint_authority.pubkey(),
            10,
            &[&mint_authority],
        )
        .await
        .unwrap();

    token
        .confidential_transfer_disable_non_confidential_credits(
            &bob_meta.token_account,
            &bob.pubkey(),
            &[&bob],
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
    assert!(!bool::from(&extension.allow_non_confidential_credits));

    let err = token
        .transfer(
            &alice_meta.token_account,
            &bob_meta.token_account,
            &alice.pubkey(),
            10,
            &[&alice],
        )
        .await
        .unwrap_err();

    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::NonConfidentialTransfersDisabled as u32)
            )
        )))
    );

    token
        .confidential_transfer_enable_non_confidential_credits(
            &bob_meta.token_account,
            &bob.pubkey(),
            &[&bob],
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
    assert!(bool::from(&extension.allow_non_confidential_credits));

    // transfer a different number to change the signature
    token
        .transfer(
            &alice_meta.token_account,
            &bob_meta.token_account,
            &alice.pubkey(),
            9,
            &[&alice],
        )
        .await
        .unwrap();
}

#[cfg(feature = "zk-ops")]
async fn empty_account_with_option<S: Signers>(
    token: &Token<ProgramBanksClientProcessTransaction>,
    account: &Pubkey,
    authority: &Pubkey,
    elgamal_keypair: &ElGamalKeypair,
    signing_keypairs: &S,
    option: ConfidentialTransferOption,
) -> TokenResult<()> {
    match option {
        ConfidentialTransferOption::InstructionData => {
            token
                .confidential_transfer_empty_account(
                    account,
                    authority,
                    None,
                    None,
                    elgamal_keypair,
                    signing_keypairs,
                )
                .await
        }
        ConfidentialTransferOption::RecordAccount => {
            let state = token.get_account_info(account).await.unwrap();
            let extension = state
                .get_extension::<ConfidentialTransferAccount>()
                .unwrap();
            let account_info = EmptyAccountAccountInfo::new(extension);

            let zero_ciphertext_proof_data =
                account_info.generate_proof_data(elgamal_keypair).unwrap();

            let zero_ciphertext_proof_record_account = Keypair::new();
            let record_account_authority = Keypair::new();

            token
                .confidential_transfer_create_record_account(
                    &zero_ciphertext_proof_record_account.pubkey(),
                    &record_account_authority.pubkey(),
                    &zero_ciphertext_proof_data,
                    &zero_ciphertext_proof_record_account,
                    &record_account_authority,
                )
                .await
                .unwrap();

            let zero_ciphertext_account = ProofAccount::RecordAccount(
                zero_ciphertext_proof_record_account.pubkey(),
                RecordData::WRITABLE_START_INDEX as u32,
            );

            let result = token
                .confidential_transfer_empty_account(
                    account,
                    authority,
                    Some(&zero_ciphertext_account),
                    None,
                    elgamal_keypair,
                    signing_keypairs,
                )
                .await;

            token
                .confidential_transfer_close_record_account(
                    &zero_ciphertext_proof_record_account.pubkey(),
                    account,
                    &record_account_authority.pubkey(),
                    &[&record_account_authority],
                )
                .await
                .unwrap();

            result
        }
        ConfidentialTransferOption::ContextStateAccount => {
            let state = token.get_account_info(account).await.unwrap();
            let extension = state
                .get_extension::<ConfidentialTransferAccount>()
                .unwrap();
            let account_info = EmptyAccountAccountInfo::new(extension);

            let zero_ciphertext_proof_data =
                account_info.generate_proof_data(elgamal_keypair).unwrap();

            let zero_ciphertext_proof_context_account = Keypair::new();
            let context_account_authority = Keypair::new();

            token
                .confidential_transfer_create_context_state_account(
                    &zero_ciphertext_proof_context_account.pubkey(),
                    &context_account_authority.pubkey(),
                    &zero_ciphertext_proof_data,
                    false,
                    &[&zero_ciphertext_proof_context_account],
                )
                .await
                .unwrap();

            let zero_ciphertext_account =
                ProofAccount::ContextAccount(zero_ciphertext_proof_context_account.pubkey());

            let result = token
                .confidential_transfer_empty_account(
                    account,
                    authority,
                    Some(&zero_ciphertext_account),
                    None,
                    elgamal_keypair,
                    signing_keypairs,
                )
                .await;

            token
                .confidential_transfer_close_context_state_account(
                    &zero_ciphertext_proof_context_account.pubkey(),
                    account,
                    &context_account_authority.pubkey(),
                    &[&context_account_authority],
                )
                .await
                .unwrap();

            result
        }
    }
}

#[cfg(feature = "zk-ops")]
#[tokio::test]
async fn confidential_transfer_empty_account() {
    confidential_transfer_empty_account_with_option(ConfidentialTransferOption::InstructionData)
        .await;
    confidential_transfer_empty_account_with_option(ConfidentialTransferOption::RecordAccount)
        .await;
    confidential_transfer_empty_account_with_option(
        ConfidentialTransferOption::ContextStateAccount,
    )
    .await;
}

#[cfg(feature = "zk-ops")]
async fn confidential_transfer_empty_account_with_option(option: ConfidentialTransferOption) {
    let authority = Keypair::new();
    let auto_approve_new_accounts = true;
    let auditor_elgamal_keypair = ElGamalKeypair::new_rand();
    let auditor_elgamal_pubkey = (*auditor_elgamal_keypair.pubkey()).into();

    let mut context = TestContext::new().await;

    // newly created confidential transfer account should hold no balance and
    // therefore, immediately closable
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

    let TokenContext { token, alice, .. } = context.token_context.unwrap();
    let alice_meta = ConfidentialTokenAccountMeta::new(&token, &alice, None, false, false).await;

    empty_account_with_option(
        &token,
        &alice_meta.token_account,
        &alice.pubkey(),
        &alice_meta.elgamal_keypair,
        &[&alice],
        option,
    )
    .await
    .unwrap();
}

#[cfg(feature = "zk-ops")]
#[tokio::test]
async fn confidential_transfer_deposit() {
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

    let TokenContext {
        token,
        alice,
        mint_authority,
        decimals,
        ..
    } = context.token_context.unwrap();
    let alice_meta = ConfidentialTokenAccountMeta::new(&token, &alice, Some(2), false, false).await;

    token
        .mint_to(
            &alice_meta.token_account,
            &mint_authority.pubkey(),
            65537,
            &[&mint_authority],
        )
        .await
        .unwrap();

    let state = token
        .get_account_info(&alice_meta.token_account)
        .await
        .unwrap();
    assert_eq!(state.base.amount, 65537);
    let extension = state
        .get_extension::<ConfidentialTransferAccount>()
        .unwrap();
    assert_eq!(extension.pending_balance_credit_counter, 0.into());
    assert_eq!(extension.expected_pending_balance_credit_counter, 0.into());
    assert_eq!(extension.actual_pending_balance_credit_counter, 0.into());
    assert_eq!(extension.pending_balance_lo, PodElGamalCiphertext::zeroed());
    assert_eq!(extension.pending_balance_hi, PodElGamalCiphertext::zeroed());
    assert_eq!(extension.available_balance, PodElGamalCiphertext::zeroed());

    token
        .confidential_transfer_deposit(
            &alice_meta.token_account,
            &alice.pubkey(),
            65537,
            decimals,
            &[&alice],
        )
        .await
        .unwrap();

    let state = token
        .get_account_info(&alice_meta.token_account)
        .await
        .unwrap();
    assert_eq!(state.base.amount, 0);
    let extension = state
        .get_extension::<ConfidentialTransferAccount>()
        .unwrap();
    assert_eq!(extension.pending_balance_credit_counter, 1.into());
    assert_eq!(extension.expected_pending_balance_credit_counter, 0.into());
    assert_eq!(extension.actual_pending_balance_credit_counter, 0.into());

    alice_meta
        .check_balances(
            &token,
            ConfidentialTokenAccountBalances {
                pending_balance_lo: 1,
                pending_balance_hi: 1,
                available_balance: 0,
                decryptable_available_balance: 0,
            },
        )
        .await;

    // deposit zero amount
    token
        .confidential_transfer_deposit(
            &alice_meta.token_account,
            &alice.pubkey(),
            0,
            decimals,
            &[&alice],
        )
        .await
        .unwrap();

    token
        .confidential_transfer_apply_pending_balance(
            &alice_meta.token_account,
            &alice.pubkey(),
            None,
            alice_meta.elgamal_keypair.secret(),
            &alice_meta.aes_key,
            &[&alice],
        )
        .await
        .unwrap();

    // try to deposit over maximum allowed value
    let illegal_amount = MAXIMUM_DEPOSIT_TRANSFER_AMOUNT.checked_add(1).unwrap();

    token
        .mint_to(
            &alice_meta.token_account,
            &mint_authority.pubkey(),
            illegal_amount,
            &[&mint_authority],
        )
        .await
        .unwrap();

    let err = token
        .confidential_transfer_deposit(
            &alice_meta.token_account,
            &alice.pubkey(),
            illegal_amount,
            decimals,
            &[&alice],
        )
        .await
        .unwrap_err();

    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::MaximumDepositAmountExceeded as u32),
            )
        )))
    );

    // deposit maximum allowed value
    token
        .confidential_transfer_deposit(
            &alice_meta.token_account,
            &alice.pubkey(),
            MAXIMUM_DEPOSIT_TRANSFER_AMOUNT,
            decimals,
            &[&alice],
        )
        .await
        .unwrap();

    // maximum pending balance credits exceeded
    token
        .confidential_transfer_deposit(
            &alice_meta.token_account,
            &alice.pubkey(),
            0,
            decimals,
            &[&alice],
        )
        .await
        .unwrap();

    let err = token
        .confidential_transfer_deposit(
            &alice_meta.token_account,
            &alice.pubkey(),
            1,
            decimals,
            &[&alice],
        )
        .await
        .unwrap_err();

    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    TokenError::MaximumPendingBalanceCreditCounterExceeded as u32
                ),
            )
        )))
    );

    let state = token
        .get_account_info(&alice_meta.token_account)
        .await
        .unwrap();
    assert_eq!(state.base.amount, 1);
    let extension = state
        .get_extension::<ConfidentialTransferAccount>()
        .unwrap();
    assert_eq!(extension.pending_balance_credit_counter, 2.into());
    assert_eq!(extension.expected_pending_balance_credit_counter, 2.into());
    assert_eq!(extension.actual_pending_balance_credit_counter, 2.into());
}

#[allow(clippy::too_many_arguments)]
#[cfg(feature = "zk-ops")]
async fn withdraw_with_option<S: Signers>(
    token: &Token<ProgramBanksClientProcessTransaction>,
    source_account: &Pubkey,
    source_authority: &Pubkey,
    withdraw_amount: u64,
    decimals: u8,
    source_elgamal_keypair: &ElGamalKeypair,
    source_aes_key: &AeKey,
    signing_keypairs: &S,
    option: ConfidentialTransferOption,
) -> TokenResult<()> {
    match option {
        ConfidentialTransferOption::InstructionData => {
            token
                .confidential_transfer_withdraw(
                    source_account,
                    source_authority,
                    None,
                    None,
                    withdraw_amount,
                    decimals,
                    None,
                    source_elgamal_keypair,
                    source_aes_key,
                    signing_keypairs,
                )
                .await
        }
        ConfidentialTransferOption::RecordAccount => {
            let state = token.get_account_info(source_account).await.unwrap();
            let extension = state
                .get_extension::<ConfidentialTransferAccount>()
                .unwrap();
            let withdraw_account_info = WithdrawAccountInfo::new(extension);

            let WithdrawProofData {
                equality_proof_data,
                range_proof_data,
            } = withdraw_account_info
                .generate_proof_data(withdraw_amount, source_elgamal_keypair, source_aes_key)
                .unwrap();

            let equality_proof_record_account = Keypair::new();
            let range_proof_record_account = Keypair::new();
            let record_account_authority = Keypair::new();

            token
                .confidential_transfer_create_record_account(
                    &equality_proof_record_account.pubkey(),
                    &record_account_authority.pubkey(),
                    &equality_proof_data,
                    &equality_proof_record_account,
                    &record_account_authority,
                )
                .await
                .unwrap();

            let equality_proof_account = ProofAccount::RecordAccount(
                equality_proof_record_account.pubkey(),
                RecordData::WRITABLE_START_INDEX as u32,
            );

            token
                .confidential_transfer_create_record_account(
                    &range_proof_record_account.pubkey(),
                    &record_account_authority.pubkey(),
                    &range_proof_data,
                    &range_proof_record_account,
                    &record_account_authority,
                )
                .await
                .unwrap();

            let range_proof_account = ProofAccount::RecordAccount(
                range_proof_record_account.pubkey(),
                RecordData::WRITABLE_START_INDEX as u32,
            );

            let result = token
                .confidential_transfer_withdraw(
                    source_account,
                    source_authority,
                    Some(&equality_proof_account),
                    Some(&range_proof_account),
                    withdraw_amount,
                    decimals,
                    None,
                    source_elgamal_keypair,
                    source_aes_key,
                    signing_keypairs,
                )
                .await;

            token
                .confidential_transfer_close_record_account(
                    &equality_proof_record_account.pubkey(),
                    source_account,
                    &record_account_authority.pubkey(),
                    &[&record_account_authority],
                )
                .await
                .unwrap();

            token
                .confidential_transfer_close_record_account(
                    &range_proof_record_account.pubkey(),
                    source_account,
                    &record_account_authority.pubkey(),
                    &[&record_account_authority],
                )
                .await
                .unwrap();

            result
        }
        ConfidentialTransferOption::ContextStateAccount => {
            let state = token.get_account_info(source_account).await.unwrap();
            let extension = state
                .get_extension::<ConfidentialTransferAccount>()
                .unwrap();
            let withdraw_account_info = WithdrawAccountInfo::new(extension);

            let WithdrawProofData {
                equality_proof_data,
                range_proof_data,
            } = withdraw_account_info
                .generate_proof_data(withdraw_amount, source_elgamal_keypair, source_aes_key)
                .unwrap();

            let equality_proof_context_account = Keypair::new();
            let range_proof_context_account = Keypair::new();
            let context_account_authority = Keypair::new();

            token
                .confidential_transfer_create_context_state_account(
                    &equality_proof_context_account.pubkey(),
                    &context_account_authority.pubkey(),
                    &equality_proof_data,
                    false,
                    &[&equality_proof_context_account],
                )
                .await
                .unwrap();

            let equality_proof_account =
                ProofAccount::ContextAccount(equality_proof_context_account.pubkey());

            token
                .confidential_transfer_create_context_state_account(
                    &range_proof_context_account.pubkey(),
                    &context_account_authority.pubkey(),
                    &range_proof_data,
                    false,
                    &[&range_proof_context_account],
                )
                .await
                .unwrap();

            let range_proof_account =
                ProofAccount::ContextAccount(range_proof_context_account.pubkey());

            let result = token
                .confidential_transfer_withdraw(
                    source_account,
                    source_authority,
                    Some(&equality_proof_account),
                    Some(&range_proof_account),
                    withdraw_amount,
                    decimals,
                    None,
                    source_elgamal_keypair,
                    source_aes_key,
                    signing_keypairs,
                )
                .await;

            token
                .confidential_transfer_close_context_state_account(
                    &equality_proof_context_account.pubkey(),
                    source_account,
                    &context_account_authority.pubkey(),
                    &[&context_account_authority],
                )
                .await
                .unwrap();

            token
                .confidential_transfer_close_context_state_account(
                    &range_proof_context_account.pubkey(),
                    source_account,
                    &context_account_authority.pubkey(),
                    &[&context_account_authority],
                )
                .await
                .unwrap();

            result
        }
    }
}

#[cfg(feature = "zk-ops")]
#[tokio::test]
async fn confidential_transfer_withdraw() {
    confidential_transfer_withdraw_with_option(ConfidentialTransferOption::InstructionData).await;
    confidential_transfer_withdraw_with_option(ConfidentialTransferOption::RecordAccount).await;
    confidential_transfer_withdraw_with_option(ConfidentialTransferOption::ContextStateAccount)
        .await;
}

#[cfg(feature = "zk-ops")]
async fn confidential_transfer_withdraw_with_option(option: ConfidentialTransferOption) {
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

    let TokenContext {
        token,
        alice,
        mint_authority,
        decimals,
        ..
    } = context.token_context.unwrap();
    let alice_meta = ConfidentialTokenAccountMeta::new_with_tokens(
        &token,
        &alice,
        None,
        false,
        false,
        &mint_authority,
        42,
        decimals,
    )
    .await;

    let state = token
        .get_account_info(&alice_meta.token_account)
        .await
        .unwrap();
    assert_eq!(state.base.amount, 0);
    alice_meta
        .check_balances(
            &token,
            ConfidentialTokenAccountBalances {
                pending_balance_lo: 0,
                pending_balance_hi: 0,
                available_balance: 42,
                decryptable_available_balance: 42,
            },
        )
        .await;

    // withdraw zero amount
    withdraw_with_option(
        &token,
        &alice_meta.token_account,
        &alice.pubkey(),
        0,
        decimals,
        &alice_meta.elgamal_keypair,
        &alice_meta.aes_key,
        &[&alice],
        option,
    )
    .await
    .unwrap();

    alice_meta
        .check_balances(
            &token,
            ConfidentialTokenAccountBalances {
                pending_balance_lo: 0,
                pending_balance_hi: 0,
                available_balance: 42,
                decryptable_available_balance: 42,
            },
        )
        .await;

    // withdraw entire balance
    withdraw_with_option(
        &token,
        &alice_meta.token_account,
        &alice.pubkey(),
        42,
        decimals,
        &alice_meta.elgamal_keypair,
        &alice_meta.aes_key,
        &[&alice],
        option,
    )
    .await
    .unwrap();

    let state = token
        .get_account_info(&alice_meta.token_account)
        .await
        .unwrap();
    assert_eq!(state.base.amount, 42);
    alice_meta
        .check_balances(
            &token,
            ConfidentialTokenAccountBalances {
                pending_balance_lo: 0,
                pending_balance_hi: 0,
                available_balance: 0,
                decryptable_available_balance: 0,
            },
        )
        .await;
}

#[allow(clippy::too_many_arguments)]
#[cfg(feature = "zk-ops")]
async fn confidential_transfer_with_option<S: Signers>(
    token: &Token<ProgramBanksClientProcessTransaction>,
    source_account: &Pubkey,
    destination_account: &Pubkey,
    source_authority: &Pubkey,
    transfer_amount: u64,
    source_elgamal_keypair: &ElGamalKeypair,
    source_aes_key: &AeKey,
    destination_elgamal_pubkey: &ElGamalPubkey,
    auditor_elgamal_pubkey: Option<&ElGamalPubkey>,
    memo: Option<(&str, Vec<Pubkey>)>,
    signing_keypairs: &S,
    option: ConfidentialTransferOption,
) -> TokenResult<()> {
    match option {
        ConfidentialTransferOption::InstructionData => {
            let transfer_token = if let Some((memo, signing_pubkey)) = memo {
                token.with_memo(memo, signing_pubkey)
            } else {
                token
            };

            transfer_token
                .confidential_transfer_transfer(
                    source_account,
                    destination_account,
                    source_authority,
                    None,
                    None,
                    None,
                    transfer_amount,
                    None,
                    source_elgamal_keypair,
                    source_aes_key,
                    destination_elgamal_pubkey,
                    auditor_elgamal_pubkey,
                    signing_keypairs,
                )
                .await
        }
        ConfidentialTransferOption::RecordAccount => {
            let state = token.get_account_info(source_account).await.unwrap();
            let extension = state
                .get_extension::<ConfidentialTransferAccount>()
                .unwrap();
            let transfer_account_info = TransferAccountInfo::new(extension);

            let TransferProofData {
                equality_proof_data,
                ciphertext_validity_proof_data_with_ciphertext,
                range_proof_data,
            } = transfer_account_info
                .generate_split_transfer_proof_data(
                    transfer_amount,
                    source_elgamal_keypair,
                    source_aes_key,
                    destination_elgamal_pubkey,
                    auditor_elgamal_pubkey,
                )
                .unwrap();

            let transfer_amount_auditor_ciphertext_lo =
                ciphertext_validity_proof_data_with_ciphertext.ciphertext_lo;
            let transfer_amount_auditor_ciphertext_hi =
                ciphertext_validity_proof_data_with_ciphertext.ciphertext_hi;

            let equality_proof_record_account = Keypair::new();
            let ciphertext_validity_proof_record_account = Keypair::new();
            let range_proof_record_account = Keypair::new();
            let record_account_authority = Keypair::new();

            token
                .confidential_transfer_create_record_account(
                    &equality_proof_record_account.pubkey(),
                    &record_account_authority.pubkey(),
                    &equality_proof_data,
                    &equality_proof_record_account,
                    &record_account_authority,
                )
                .await
                .unwrap();

            let equality_proof_account = ProofAccount::RecordAccount(
                equality_proof_record_account.pubkey(),
                RecordData::WRITABLE_START_INDEX as u32,
            );

            token
                .confidential_transfer_create_record_account(
                    &ciphertext_validity_proof_record_account.pubkey(),
                    &record_account_authority.pubkey(),
                    &ciphertext_validity_proof_data_with_ciphertext.proof_data,
                    &ciphertext_validity_proof_record_account,
                    &record_account_authority,
                )
                .await
                .unwrap();

            let ciphertext_validity_proof_account = ProofAccount::RecordAccount(
                ciphertext_validity_proof_record_account.pubkey(),
                RecordData::WRITABLE_START_INDEX as u32,
            );

            token
                .confidential_transfer_create_record_account(
                    &range_proof_record_account.pubkey(),
                    &record_account_authority.pubkey(),
                    &range_proof_data,
                    &range_proof_record_account,
                    &record_account_authority,
                )
                .await
                .unwrap();

            let range_proof_account = ProofAccount::RecordAccount(
                range_proof_record_account.pubkey(),
                RecordData::WRITABLE_START_INDEX as u32,
            );

            let transfer_token = if let Some((memo, signing_pubkey)) = memo {
                token.with_memo(memo, signing_pubkey)
            } else {
                token
            };

            let ciphertext_validity_proof_account_with_ciphertext = ProofAccountWithCiphertext {
                proof_account: ciphertext_validity_proof_account,
                ciphertext_lo: transfer_amount_auditor_ciphertext_lo,
                ciphertext_hi: transfer_amount_auditor_ciphertext_hi,
            };

            let result = transfer_token
                .confidential_transfer_transfer(
                    source_account,
                    destination_account,
                    source_authority,
                    Some(&equality_proof_account),
                    Some(&ciphertext_validity_proof_account_with_ciphertext),
                    Some(&range_proof_account),
                    transfer_amount,
                    None,
                    source_elgamal_keypair,
                    source_aes_key,
                    destination_elgamal_pubkey,
                    auditor_elgamal_pubkey,
                    signing_keypairs,
                )
                .await;

            token
                .confidential_transfer_close_record_account(
                    &equality_proof_record_account.pubkey(),
                    source_account,
                    &record_account_authority.pubkey(),
                    &[&record_account_authority],
                )
                .await
                .unwrap();

            token
                .confidential_transfer_close_record_account(
                    &ciphertext_validity_proof_record_account.pubkey(),
                    source_account,
                    &record_account_authority.pubkey(),
                    &[&record_account_authority],
                )
                .await
                .unwrap();

            token
                .confidential_transfer_close_record_account(
                    &range_proof_record_account.pubkey(),
                    source_account,
                    &record_account_authority.pubkey(),
                    &[&record_account_authority],
                )
                .await
                .unwrap();

            result
        }
        ConfidentialTransferOption::ContextStateAccount => {
            let state = token.get_account_info(source_account).await.unwrap();
            let extension = state
                .get_extension::<ConfidentialTransferAccount>()
                .unwrap();
            let transfer_account_info = TransferAccountInfo::new(extension);

            let TransferProofData {
                equality_proof_data,
                ciphertext_validity_proof_data_with_ciphertext,
                range_proof_data,
            } = transfer_account_info
                .generate_split_transfer_proof_data(
                    transfer_amount,
                    source_elgamal_keypair,
                    source_aes_key,
                    destination_elgamal_pubkey,
                    auditor_elgamal_pubkey,
                )
                .unwrap();

            let transfer_amount_auditor_ciphertext_lo =
                ciphertext_validity_proof_data_with_ciphertext.ciphertext_lo;
            let transfer_amount_auditor_ciphertext_hi =
                ciphertext_validity_proof_data_with_ciphertext.ciphertext_hi;

            let equality_proof_context_account = Keypair::new();
            let ciphertext_validity_proof_context_account = Keypair::new();
            let range_proof_context_account = Keypair::new();
            let context_account_authority = Keypair::new();

            token
                .confidential_transfer_create_context_state_account(
                    &equality_proof_context_account.pubkey(),
                    &context_account_authority.pubkey(),
                    &equality_proof_data,
                    false,
                    &[&equality_proof_context_account],
                )
                .await
                .unwrap();

            let equality_proof_context_proof_account =
                ProofAccount::ContextAccount(equality_proof_context_account.pubkey());

            token
                .confidential_transfer_create_context_state_account(
                    &ciphertext_validity_proof_context_account.pubkey(),
                    &context_account_authority.pubkey(),
                    &ciphertext_validity_proof_data_with_ciphertext.proof_data,
                    false,
                    &[&ciphertext_validity_proof_context_account],
                )
                .await
                .unwrap();

            let ciphertext_validity_proof_context_proof_account =
                ProofAccount::ContextAccount(ciphertext_validity_proof_context_account.pubkey());

            token
                .confidential_transfer_create_context_state_account(
                    &range_proof_context_account.pubkey(),
                    &context_account_authority.pubkey(),
                    &range_proof_data,
                    false,
                    &[&range_proof_context_account],
                )
                .await
                .unwrap();

            let range_proof_context_proof_account =
                ProofAccount::ContextAccount(range_proof_context_account.pubkey());

            let transfer_token = if let Some((memo, signing_pubkey)) = memo {
                token.with_memo(memo, signing_pubkey)
            } else {
                token
            };

            let ciphertext_validity_proof_account_with_ciphertext = ProofAccountWithCiphertext {
                proof_account: ciphertext_validity_proof_context_proof_account,
                ciphertext_lo: transfer_amount_auditor_ciphertext_lo,
                ciphertext_hi: transfer_amount_auditor_ciphertext_hi,
            };

            let result = transfer_token
                .confidential_transfer_transfer(
                    source_account,
                    destination_account,
                    source_authority,
                    Some(&equality_proof_context_proof_account),
                    Some(&ciphertext_validity_proof_account_with_ciphertext),
                    Some(&range_proof_context_proof_account),
                    transfer_amount,
                    None,
                    source_elgamal_keypair,
                    source_aes_key,
                    destination_elgamal_pubkey,
                    auditor_elgamal_pubkey,
                    signing_keypairs,
                )
                .await;

            token
                .confidential_transfer_close_context_state_account(
                    &equality_proof_context_account.pubkey(),
                    source_account,
                    &context_account_authority.pubkey(),
                    &[&context_account_authority],
                )
                .await
                .unwrap();

            token
                .confidential_transfer_close_context_state_account(
                    &ciphertext_validity_proof_context_account.pubkey(),
                    source_account,
                    &context_account_authority.pubkey(),
                    &[&context_account_authority],
                )
                .await
                .unwrap();

            token
                .confidential_transfer_close_context_state_account(
                    &range_proof_context_account.pubkey(),
                    source_account,
                    &context_account_authority.pubkey(),
                    &[&context_account_authority],
                )
                .await
                .unwrap();

            result
        }
    }
}

#[cfg(feature = "zk-ops")]
#[tokio::test]
async fn confidential_transfer_transfer() {
    confidential_transfer_transfer_with_option(ConfidentialTransferOption::InstructionData).await;
    confidential_transfer_transfer_with_option(ConfidentialTransferOption::RecordAccount).await;
    confidential_transfer_transfer_with_option(ConfidentialTransferOption::ContextStateAccount)
        .await;
}

#[cfg(feature = "zk-ops")]
#[tokio::test]
async fn pause_confidential_deposit() {
    let authority = Keypair::new();
    let pausable_authority = Keypair::new();
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
            ExtensionInitializationParams::PausableConfig {
                authority: pausable_authority.pubkey(),
            },
        ])
        .await
        .unwrap();
    let TokenContext {
        token,
        alice,
        mint_authority,
        decimals,
        ..
    } = context.token_context.unwrap();

    let alice_meta = ConfidentialTokenAccountMeta::new(&token, &alice, None, false, false).await;

    token
        .mint_to(
            &alice_meta.token_account,
            &mint_authority.pubkey(),
            42,
            &[mint_authority],
        )
        .await
        .unwrap();

    token
        .pause(&pausable_authority.pubkey(), &[&pausable_authority])
        .await
        .unwrap();

    let error = token
        .confidential_transfer_deposit(
            &alice_meta.token_account,
            &alice.pubkey(),
            42,
            decimals,
            &[alice],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::MintPaused as u32)
            )
        )))
    );
}

#[cfg(feature = "zk-ops")]
#[tokio::test]
async fn pause_confidential_withdraw() {
    let authority = Keypair::new();
    let pausable_authority = Keypair::new();
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
            ExtensionInitializationParams::PausableConfig {
                authority: pausable_authority.pubkey(),
            },
        ])
        .await
        .unwrap();
    let TokenContext {
        token,
        alice,
        mint_authority,
        decimals,
        ..
    } = context.token_context.unwrap();

    let alice_meta = ConfidentialTokenAccountMeta::new(&token, &alice, None, false, false).await;

    token
        .mint_to(
            &alice_meta.token_account,
            &mint_authority.pubkey(),
            42,
            &[mint_authority],
        )
        .await
        .unwrap();

    token
        .confidential_transfer_deposit(
            &alice_meta.token_account,
            &alice.pubkey(),
            42,
            decimals,
            &[&alice],
        )
        .await
        .unwrap();

    token
        .confidential_transfer_apply_pending_balance(
            &alice_meta.token_account,
            &alice.pubkey(),
            None,
            alice_meta.elgamal_keypair.secret(),
            &alice_meta.aes_key,
            &[&alice],
        )
        .await
        .unwrap();

    token
        .pause(&pausable_authority.pubkey(), &[&pausable_authority])
        .await
        .unwrap();

    let error = withdraw_with_option(
        &token,
        &alice_meta.token_account,
        &alice.pubkey(),
        42,
        decimals,
        &alice_meta.elgamal_keypair,
        &alice_meta.aes_key,
        &[&alice],
        ConfidentialTransferOption::InstructionData,
    )
    .await
    .unwrap_err();

    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::MintPaused as u32)
            )
        )))
    );
}

#[cfg(feature = "zk-ops")]
#[tokio::test]
async fn pause_confidential_transfer() {
    let authority = Keypair::new();
    let pausable_authority = Keypair::new();
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
            ExtensionInitializationParams::PausableConfig {
                authority: pausable_authority.pubkey(),
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

    let alice_meta = ConfidentialTokenAccountMeta::new_with_tokens(
        &token,
        &alice,
        None,
        false,
        false,
        &mint_authority,
        42,
        decimals,
    )
    .await;

    let bob_meta = ConfidentialTokenAccountMeta::new(&token, &bob, Some(2), false, false).await;

    // pause it
    token
        .pause(&pausable_authority.pubkey(), &[&pausable_authority])
        .await
        .unwrap();
    let error = confidential_transfer_with_option(
        &token,
        &alice_meta.token_account,
        &bob_meta.token_account,
        &alice.pubkey(),
        10,
        &alice_meta.elgamal_keypair,
        &alice_meta.aes_key,
        bob_meta.elgamal_keypair.pubkey(),
        Some(auditor_elgamal_keypair.pubkey()),
        None,
        &[&alice],
        ConfidentialTransferOption::InstructionData,
    )
    .await
    .unwrap_err();

    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::MintPaused as u32)
            )
        )))
    );
}

#[cfg(feature = "zk-ops")]
async fn confidential_transfer_transfer_with_option(option: ConfidentialTransferOption) {
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

    let TokenContext {
        token,
        alice,
        bob,
        mint_authority,
        decimals,
        ..
    } = context.token_context.unwrap();

    let alice_meta = ConfidentialTokenAccountMeta::new_with_tokens(
        &token,
        &alice,
        None,
        false,
        false,
        &mint_authority,
        42,
        decimals,
    )
    .await;

    let bob_meta = ConfidentialTokenAccountMeta::new(&token, &bob, Some(2), false, false).await;

    // Self-transfer of 0 tokens
    confidential_transfer_with_option(
        &token,
        &alice_meta.token_account,
        &alice_meta.token_account,
        &alice.pubkey(),
        0,
        &alice_meta.elgamal_keypair,
        &alice_meta.aes_key,
        alice_meta.elgamal_keypair.pubkey(),
        Some(auditor_elgamal_keypair.pubkey()),
        None,
        &[&alice],
        option,
    )
    .await
    .unwrap();

    alice_meta
        .check_balances(
            &token,
            ConfidentialTokenAccountBalances {
                pending_balance_lo: 0,
                pending_balance_hi: 0,
                available_balance: 42,
                decryptable_available_balance: 42,
            },
        )
        .await;

    // Self-transfer of N tokens
    confidential_transfer_with_option(
        &token,
        &alice_meta.token_account,
        &alice_meta.token_account,
        &alice.pubkey(),
        42,
        &alice_meta.elgamal_keypair,
        &alice_meta.aes_key,
        alice_meta.elgamal_keypair.pubkey(),
        Some(auditor_elgamal_keypair.pubkey()),
        None,
        &[&alice],
        option,
    )
    .await
    .unwrap();

    alice_meta
        .check_balances(
            &token,
            ConfidentialTokenAccountBalances {
                pending_balance_lo: 42,
                pending_balance_hi: 0,
                available_balance: 0,
                decryptable_available_balance: 0,
            },
        )
        .await;

    token
        .confidential_transfer_apply_pending_balance(
            &alice_meta.token_account,
            &alice.pubkey(),
            None,
            alice_meta.elgamal_keypair.secret(),
            &alice_meta.aes_key,
            &[&alice],
        )
        .await
        .unwrap();

    alice_meta
        .check_balances(
            &token,
            ConfidentialTokenAccountBalances {
                pending_balance_lo: 0,
                pending_balance_hi: 0,
                available_balance: 42,
                decryptable_available_balance: 42,
            },
        )
        .await;

    confidential_transfer_with_option(
        &token,
        &alice_meta.token_account,
        &bob_meta.token_account,
        &alice.pubkey(),
        42,
        &alice_meta.elgamal_keypair,
        &alice_meta.aes_key,
        bob_meta.elgamal_keypair.pubkey(),
        Some(auditor_elgamal_keypair.pubkey()),
        None,
        &[&alice],
        option,
    )
    .await
    .unwrap();

    alice_meta
        .check_balances(
            &token,
            ConfidentialTokenAccountBalances {
                pending_balance_lo: 0,
                pending_balance_hi: 0,
                available_balance: 0,
                decryptable_available_balance: 0,
            },
        )
        .await;

    bob_meta
        .check_balances(
            &token,
            ConfidentialTokenAccountBalances {
                pending_balance_lo: 42,
                pending_balance_hi: 0,
                available_balance: 0,
                decryptable_available_balance: 0,
            },
        )
        .await;

    confidential_transfer_with_option(
        &token,
        &bob_meta.token_account,
        &bob_meta.token_account,
        &bob.pubkey(),
        0,
        &bob_meta.elgamal_keypair,
        &bob_meta.aes_key,
        bob_meta.elgamal_keypair.pubkey(),
        Some(auditor_elgamal_keypair.pubkey()),
        None,
        &[&bob],
        option,
    )
    .await
    .unwrap();

    let err = confidential_transfer_with_option(
        &token,
        &bob_meta.token_account,
        &bob_meta.token_account,
        &bob.pubkey(),
        0,
        &bob_meta.elgamal_keypair,
        &bob_meta.aes_key,
        bob_meta.elgamal_keypair.pubkey(),
        Some(auditor_elgamal_keypair.pubkey()),
        None,
        &[&bob],
        option,
    )
    .await
    .unwrap_err();

    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    TokenError::MaximumPendingBalanceCreditCounterExceeded as u32
                ),
            )
        )))
    );

    token
        .confidential_transfer_apply_pending_balance(
            &bob_meta.token_account,
            &bob.pubkey(),
            None,
            bob_meta.elgamal_keypair.secret(),
            &bob_meta.aes_key,
            &[&bob],
        )
        .await
        .unwrap();

    bob_meta
        .check_balances(
            &token,
            ConfidentialTokenAccountBalances {
                pending_balance_lo: 0,
                pending_balance_hi: 0,
                available_balance: 42,
                decryptable_available_balance: 42,
            },
        )
        .await;
}

#[allow(clippy::too_many_arguments)]
#[cfg(feature = "zk-ops")]
async fn confidential_transfer_with_fee_with_option<S: Signers>(
    token: &Token<ProgramBanksClientProcessTransaction>,
    source_account: &Pubkey,
    destination_account: &Pubkey,
    source_authority: &Pubkey,
    transfer_amount: u64,
    source_elgamal_keypair: &ElGamalKeypair,
    source_aes_key: &AeKey,
    destination_elgamal_pubkey: &ElGamalPubkey,
    auditor_elgamal_pubkey: Option<&ElGamalPubkey>,
    withdraw_withheld_authority_elgamal_pubkey: &ElGamalPubkey,
    fee_rate_basis_points: u16,
    maximum_fee: u64,
    memo: Option<(&str, Vec<Pubkey>)>,
    signing_keypairs: &S,
    option: ConfidentialTransferOption,
) -> TokenResult<()> {
    match option {
        ConfidentialTransferOption::InstructionData => {
            let transfer_token = if let Some((memo, signing_pubkey)) = memo {
                token.with_memo(memo, signing_pubkey)
            } else {
                token
            };

            transfer_token
                .confidential_transfer_transfer_with_fee(
                    source_account,
                    destination_account,
                    source_authority,
                    None,
                    None,
                    None,
                    None,
                    None,
                    transfer_amount,
                    None,
                    source_elgamal_keypair,
                    source_aes_key,
                    destination_elgamal_pubkey,
                    auditor_elgamal_pubkey,
                    withdraw_withheld_authority_elgamal_pubkey,
                    fee_rate_basis_points,
                    maximum_fee,
                    signing_keypairs,
                )
                .await
        }
        ConfidentialTransferOption::RecordAccount => {
            let state = token.get_account_info(source_account).await.unwrap();
            let extension = state
                .get_extension::<ConfidentialTransferAccount>()
                .unwrap();
            let transfer_account_info = TransferAccountInfo::new(extension);

            let TransferWithFeeProofData {
                equality_proof_data,
                transfer_amount_ciphertext_validity_proof_data_with_ciphertext,
                percentage_with_cap_proof_data,
                fee_ciphertext_validity_proof_data,
                range_proof_data,
            } = transfer_account_info
                .generate_split_transfer_with_fee_proof_data(
                    transfer_amount,
                    source_elgamal_keypair,
                    source_aes_key,
                    destination_elgamal_pubkey,
                    auditor_elgamal_pubkey,
                    withdraw_withheld_authority_elgamal_pubkey,
                    fee_rate_basis_points,
                    maximum_fee,
                )
                .unwrap();

            let transfer_amount_auditor_ciphertext_lo =
                transfer_amount_ciphertext_validity_proof_data_with_ciphertext.ciphertext_lo;
            let transfer_amount_auditor_ciphertext_hi =
                transfer_amount_ciphertext_validity_proof_data_with_ciphertext.ciphertext_hi;

            let equality_proof_record_account = Keypair::new();
            let transfer_amount_ciphertext_validity_proof_record_account = Keypair::new();
            let fee_sigma_proof_record_account = Keypair::new();
            let fee_ciphertext_validity_proof_record_account = Keypair::new();
            let range_proof_record_account = Keypair::new();
            let record_account_authority = Keypair::new();

            token
                .confidential_transfer_create_record_account(
                    &equality_proof_record_account.pubkey(),
                    &record_account_authority.pubkey(),
                    &equality_proof_data,
                    &equality_proof_record_account,
                    &record_account_authority,
                )
                .await
                .unwrap();

            let equality_proof_account = ProofAccount::RecordAccount(
                equality_proof_record_account.pubkey(),
                RecordData::WRITABLE_START_INDEX as u32,
            );

            token
                .confidential_transfer_create_record_account(
                    &transfer_amount_ciphertext_validity_proof_record_account.pubkey(),
                    &record_account_authority.pubkey(),
                    &transfer_amount_ciphertext_validity_proof_data_with_ciphertext.proof_data,
                    &transfer_amount_ciphertext_validity_proof_record_account,
                    &record_account_authority,
                )
                .await
                .unwrap();

            let transfer_amount_ciphertext_validity_proof_account = ProofAccount::RecordAccount(
                transfer_amount_ciphertext_validity_proof_record_account.pubkey(),
                RecordData::WRITABLE_START_INDEX as u32,
            );

            token
                .confidential_transfer_create_record_account(
                    &fee_sigma_proof_record_account.pubkey(),
                    &record_account_authority.pubkey(),
                    &percentage_with_cap_proof_data,
                    &fee_sigma_proof_record_account,
                    &record_account_authority,
                )
                .await
                .unwrap();

            let fee_sigma_proof_account = ProofAccount::RecordAccount(
                fee_sigma_proof_record_account.pubkey(),
                RecordData::WRITABLE_START_INDEX as u32,
            );

            token
                .confidential_transfer_create_record_account(
                    &fee_ciphertext_validity_proof_record_account.pubkey(),
                    &record_account_authority.pubkey(),
                    &fee_ciphertext_validity_proof_data,
                    &fee_ciphertext_validity_proof_record_account,
                    &record_account_authority,
                )
                .await
                .unwrap();

            let fee_ciphertext_validity_proof_account = ProofAccount::RecordAccount(
                fee_ciphertext_validity_proof_record_account.pubkey(),
                RecordData::WRITABLE_START_INDEX as u32,
            );

            token
                .confidential_transfer_create_record_account(
                    &range_proof_record_account.pubkey(),
                    &record_account_authority.pubkey(),
                    &range_proof_data,
                    &range_proof_record_account,
                    &record_account_authority,
                )
                .await
                .unwrap();

            let range_proof_account = ProofAccount::RecordAccount(
                range_proof_record_account.pubkey(),
                RecordData::WRITABLE_START_INDEX as u32,
            );

            let transfer_token = if let Some((memo, signing_pubkey)) = memo {
                token.with_memo(memo, signing_pubkey)
            } else {
                token
            };

            let transfer_amount_ciphertext_validity_proof_account_with_ciphertext =
                ProofAccountWithCiphertext {
                    proof_account: transfer_amount_ciphertext_validity_proof_account,
                    ciphertext_lo: transfer_amount_auditor_ciphertext_lo,
                    ciphertext_hi: transfer_amount_auditor_ciphertext_hi,
                };

            let result = transfer_token
                .confidential_transfer_transfer_with_fee(
                    source_account,
                    destination_account,
                    source_authority,
                    Some(&equality_proof_account),
                    Some(&transfer_amount_ciphertext_validity_proof_account_with_ciphertext),
                    Some(&fee_sigma_proof_account),
                    Some(&fee_ciphertext_validity_proof_account),
                    Some(&range_proof_account),
                    transfer_amount,
                    None,
                    source_elgamal_keypair,
                    source_aes_key,
                    destination_elgamal_pubkey,
                    auditor_elgamal_pubkey,
                    withdraw_withheld_authority_elgamal_pubkey,
                    fee_rate_basis_points,
                    maximum_fee,
                    signing_keypairs,
                )
                .await;

            token
                .confidential_transfer_close_record_account(
                    &equality_proof_record_account.pubkey(),
                    source_account,
                    &record_account_authority.pubkey(),
                    &[&record_account_authority],
                )
                .await
                .unwrap();

            token
                .confidential_transfer_close_record_account(
                    &transfer_amount_ciphertext_validity_proof_record_account.pubkey(),
                    source_account,
                    &record_account_authority.pubkey(),
                    &[&record_account_authority],
                )
                .await
                .unwrap();

            token
                .confidential_transfer_close_record_account(
                    &fee_sigma_proof_record_account.pubkey(),
                    source_account,
                    &record_account_authority.pubkey(),
                    &[&record_account_authority],
                )
                .await
                .unwrap();

            token
                .confidential_transfer_close_record_account(
                    &fee_ciphertext_validity_proof_record_account.pubkey(),
                    source_account,
                    &record_account_authority.pubkey(),
                    &[&record_account_authority],
                )
                .await
                .unwrap();

            token
                .confidential_transfer_close_record_account(
                    &range_proof_record_account.pubkey(),
                    source_account,
                    &record_account_authority.pubkey(),
                    &[&record_account_authority],
                )
                .await
                .unwrap();

            result
        }
        ConfidentialTransferOption::ContextStateAccount => {
            let state = token.get_account_info(source_account).await.unwrap();
            let extension = state
                .get_extension::<ConfidentialTransferAccount>()
                .unwrap();
            let transfer_account_info = TransferAccountInfo::new(extension);

            let TransferWithFeeProofData {
                equality_proof_data,
                transfer_amount_ciphertext_validity_proof_data_with_ciphertext,
                percentage_with_cap_proof_data,
                fee_ciphertext_validity_proof_data,
                range_proof_data,
            } = transfer_account_info
                .generate_split_transfer_with_fee_proof_data(
                    transfer_amount,
                    source_elgamal_keypair,
                    source_aes_key,
                    destination_elgamal_pubkey,
                    auditor_elgamal_pubkey,
                    withdraw_withheld_authority_elgamal_pubkey,
                    fee_rate_basis_points,
                    maximum_fee,
                )
                .unwrap();

            let transfer_amount_auditor_ciphertext_lo =
                transfer_amount_ciphertext_validity_proof_data_with_ciphertext.ciphertext_lo;
            let transfer_amount_auditor_ciphertext_hi =
                transfer_amount_ciphertext_validity_proof_data_with_ciphertext.ciphertext_hi;

            let equality_proof_context_account = Keypair::new();
            let transfer_amount_ciphertext_validity_proof_context_account = Keypair::new();
            let percentage_with_cap_proof_context_account = Keypair::new();
            let fee_ciphertext_validity_proof_context_account = Keypair::new();
            let range_proof_context_account = Keypair::new();
            let context_account_authority = Keypair::new();

            token
                .confidential_transfer_create_context_state_account(
                    &equality_proof_context_account.pubkey(),
                    &context_account_authority.pubkey(),
                    &equality_proof_data,
                    false,
                    &[&equality_proof_context_account],
                )
                .await
                .unwrap();

            let equality_proof_context_proof_account =
                ProofAccount::ContextAccount(equality_proof_context_account.pubkey());

            token
                .confidential_transfer_create_context_state_account(
                    &transfer_amount_ciphertext_validity_proof_context_account.pubkey(),
                    &context_account_authority.pubkey(),
                    &transfer_amount_ciphertext_validity_proof_data_with_ciphertext.proof_data,
                    false,
                    &[&transfer_amount_ciphertext_validity_proof_context_account],
                )
                .await
                .unwrap();

            let transfer_amount_ciphertext_validity_proof_context_proof_account =
                ProofAccount::ContextAccount(
                    transfer_amount_ciphertext_validity_proof_context_account.pubkey(),
                );

            token
                .confidential_transfer_create_context_state_account(
                    &percentage_with_cap_proof_context_account.pubkey(),
                    &context_account_authority.pubkey(),
                    &percentage_with_cap_proof_data,
                    false,
                    &[&percentage_with_cap_proof_context_account],
                )
                .await
                .unwrap();

            let fee_sigma_proof_context_proof_account =
                ProofAccount::ContextAccount(percentage_with_cap_proof_context_account.pubkey());

            token
                .confidential_transfer_create_context_state_account(
                    &fee_ciphertext_validity_proof_context_account.pubkey(),
                    &context_account_authority.pubkey(),
                    &fee_ciphertext_validity_proof_data,
                    false,
                    &[&fee_ciphertext_validity_proof_context_account],
                )
                .await
                .unwrap();

            let fee_ciphertext_validity_proof_context_proof_account = ProofAccount::ContextAccount(
                fee_ciphertext_validity_proof_context_account.pubkey(),
            );

            token
                .confidential_transfer_create_context_state_account(
                    &range_proof_context_account.pubkey(),
                    &context_account_authority.pubkey(),
                    &range_proof_data,
                    false,
                    &[&range_proof_context_account],
                )
                .await
                .unwrap();

            let range_proof_context_proof_account =
                ProofAccount::ContextAccount(range_proof_context_account.pubkey());

            let transfer_token = if let Some((memo, signing_pubkey)) = memo {
                token.with_memo(memo, signing_pubkey)
            } else {
                token
            };

            let transfer_amount_ciphertext_validity_proof_account_with_ciphertext =
                ProofAccountWithCiphertext {
                    proof_account: transfer_amount_ciphertext_validity_proof_context_proof_account,
                    ciphertext_lo: transfer_amount_auditor_ciphertext_lo,
                    ciphertext_hi: transfer_amount_auditor_ciphertext_hi,
                };

            let result = transfer_token
                .confidential_transfer_transfer_with_fee(
                    source_account,
                    destination_account,
                    source_authority,
                    Some(&equality_proof_context_proof_account),
                    Some(&transfer_amount_ciphertext_validity_proof_account_with_ciphertext),
                    Some(&fee_sigma_proof_context_proof_account),
                    Some(&fee_ciphertext_validity_proof_context_proof_account),
                    Some(&range_proof_context_proof_account),
                    transfer_amount,
                    None,
                    source_elgamal_keypair,
                    source_aes_key,
                    destination_elgamal_pubkey,
                    auditor_elgamal_pubkey,
                    withdraw_withheld_authority_elgamal_pubkey,
                    fee_rate_basis_points,
                    maximum_fee,
                    signing_keypairs,
                )
                .await;

            token
                .confidential_transfer_close_context_state_account(
                    &equality_proof_context_account.pubkey(),
                    source_account,
                    &context_account_authority.pubkey(),
                    &[&context_account_authority],
                )
                .await
                .unwrap();

            token
                .confidential_transfer_close_context_state_account(
                    &transfer_amount_ciphertext_validity_proof_context_account.pubkey(),
                    source_account,
                    &context_account_authority.pubkey(),
                    &[&context_account_authority],
                )
                .await
                .unwrap();

            token
                .confidential_transfer_close_context_state_account(
                    &percentage_with_cap_proof_context_account.pubkey(),
                    source_account,
                    &context_account_authority.pubkey(),
                    &[&context_account_authority],
                )
                .await
                .unwrap();

            token
                .confidential_transfer_close_context_state_account(
                    &fee_ciphertext_validity_proof_context_account.pubkey(),
                    source_account,
                    &context_account_authority.pubkey(),
                    &[&context_account_authority],
                )
                .await
                .unwrap();

            token
                .confidential_transfer_close_context_state_account(
                    &range_proof_context_account.pubkey(),
                    source_account,
                    &context_account_authority.pubkey(),
                    &[&context_account_authority],
                )
                .await
                .unwrap();

            result
        }
    }
}

#[cfg(feature = "zk-ops")]
#[tokio::test]
async fn confidential_transfer_transfer_with_fee() {
    confidential_transfer_transfer_with_fee_with_option(
        ConfidentialTransferOption::InstructionData,
    )
    .await;
    confidential_transfer_transfer_with_fee_with_option(ConfidentialTransferOption::RecordAccount)
        .await;
    confidential_transfer_transfer_with_fee_with_option(
        ConfidentialTransferOption::ContextStateAccount,
    )
    .await;
}

#[cfg(feature = "zk-ops")]
#[tokio::test]
async fn pause_confidential_transfer_with_fee() {
    let transfer_fee_authority = Keypair::new();
    let withdraw_withheld_authority = Keypair::new();

    let pausable_authority = Keypair::new();
    let confidential_transfer_authority = Keypair::new();
    let auto_approve_new_accounts = true;
    let auditor_elgamal_keypair = ElGamalKeypair::new_rand();
    let auditor_elgamal_pubkey = (*auditor_elgamal_keypair.pubkey()).into();

    let confidential_transfer_fee_authority = Keypair::new();
    let withdraw_withheld_authority_elgamal_keypair = ElGamalKeypair::new_rand();
    let withdraw_withheld_authority_elgamal_pubkey =
        (*withdraw_withheld_authority_elgamal_keypair.pubkey()).into();

    let mut context = TestContext::new().await;
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
            ExtensionInitializationParams::PausableConfig {
                authority: pausable_authority.pubkey(),
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

    let alice_meta = ConfidentialTokenAccountMeta::new_with_tokens(
        &token,
        &alice,
        None,
        false,
        true,
        &mint_authority,
        100,
        decimals,
    )
    .await;

    let bob_meta = ConfidentialTokenAccountMeta::new(&token, &bob, None, false, true).await;

    token
        .pause(&pausable_authority.pubkey(), &[&pausable_authority])
        .await
        .unwrap();

    let error = confidential_transfer_with_fee_with_option(
        &token,
        &alice_meta.token_account,
        &bob_meta.token_account,
        &alice.pubkey(),
        10,
        &alice_meta.elgamal_keypair,
        &alice_meta.aes_key,
        bob_meta.elgamal_keypair.pubkey(),
        Some(auditor_elgamal_keypair.pubkey()),
        withdraw_withheld_authority_elgamal_keypair.pubkey(),
        TEST_FEE_BASIS_POINTS,
        TEST_MAXIMUM_FEE,
        None,
        &[&alice],
        ConfidentialTransferOption::InstructionData,
    )
    .await
    .unwrap_err();

    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::MintPaused as u32)
            )
        )))
    );
}

#[cfg(feature = "zk-ops")]
async fn confidential_transfer_transfer_with_fee_with_option(option: ConfidentialTransferOption) {
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

    let TokenContext {
        token,
        alice,
        bob,
        mint_authority,
        decimals,
        ..
    } = context.token_context.unwrap();

    let alice_meta = ConfidentialTokenAccountMeta::new_with_tokens(
        &token,
        &alice,
        None,
        false,
        true,
        &mint_authority,
        100,
        decimals,
    )
    .await;

    let bob_meta = ConfidentialTokenAccountMeta::new(&token, &bob, None, false, true).await;

    // Self-transfer of 0 tokens
    confidential_transfer_with_fee_with_option(
        &token,
        &alice_meta.token_account,
        &alice_meta.token_account,
        &alice.pubkey(),
        0,
        &alice_meta.elgamal_keypair,
        &alice_meta.aes_key,
        alice_meta.elgamal_keypair.pubkey(),
        Some(auditor_elgamal_keypair.pubkey()),
        withdraw_withheld_authority_elgamal_keypair.pubkey(),
        TEST_FEE_BASIS_POINTS,
        TEST_MAXIMUM_FEE,
        None,
        &[&alice],
        option,
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

    // Self-transfers does not incur a fee
    confidential_transfer_with_fee_with_option(
        &token,
        &alice_meta.token_account,
        &alice_meta.token_account,
        &alice.pubkey(),
        100,
        &alice_meta.elgamal_keypair,
        &alice_meta.aes_key,
        alice_meta.elgamal_keypair.pubkey(),
        Some(auditor_elgamal_keypair.pubkey()),
        withdraw_withheld_authority_elgamal_keypair.pubkey(),
        TEST_FEE_BASIS_POINTS,
        TEST_MAXIMUM_FEE,
        None,
        &[&alice],
        option,
    )
    .await
    .unwrap();

    alice_meta
        .check_balances(
            &token,
            ConfidentialTokenAccountBalances {
                pending_balance_lo: 100,
                pending_balance_hi: 0,
                available_balance: 0,
                decryptable_available_balance: 0,
            },
        )
        .await;

    token
        .confidential_transfer_apply_pending_balance(
            &alice_meta.token_account,
            &alice.pubkey(),
            None,
            alice_meta.elgamal_keypair.secret(),
            &alice_meta.aes_key,
            &[&alice],
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

    confidential_transfer_with_fee_with_option(
        &token,
        &alice_meta.token_account,
        &bob_meta.token_account,
        &alice.pubkey(),
        100,
        &alice_meta.elgamal_keypair,
        &alice_meta.aes_key,
        bob_meta.elgamal_keypair.pubkey(),
        Some(auditor_elgamal_keypair.pubkey()),
        withdraw_withheld_authority_elgamal_keypair.pubkey(),
        TEST_FEE_BASIS_POINTS,
        TEST_MAXIMUM_FEE,
        None,
        &[&alice],
        option,
    )
    .await
    .unwrap();

    alice_meta
        .check_balances(
            &token,
            ConfidentialTokenAccountBalances {
                pending_balance_lo: 0,
                pending_balance_hi: 0,
                available_balance: 0,
                decryptable_available_balance: 0,
            },
        )
        .await;

    token
        .confidential_transfer_empty_account(
            &alice_meta.token_account,
            &alice.pubkey(),
            None,
            None,
            &alice_meta.elgamal_keypair,
            &[&alice],
        )
        .await
        .unwrap();

    let err = token
        .confidential_transfer_empty_account(
            &bob_meta.token_account,
            &bob.pubkey(),
            None,
            None,
            &bob_meta.elgamal_keypair,
            &[&bob],
        )
        .await
        .unwrap_err();

    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::ConfidentialTransferAccountHasBalance as u32)
            )
        )))
    );

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

    token
        .confidential_transfer_apply_pending_balance(
            &bob_meta.token_account,
            &bob.pubkey(),
            None,
            bob_meta.elgamal_keypair.secret(),
            &bob_meta.aes_key,
            &[&bob],
        )
        .await
        .unwrap();

    bob_meta
        .check_balances(
            &token,
            ConfidentialTokenAccountBalances {
                pending_balance_lo: 0,
                pending_balance_hi: 0,
                available_balance: 97,
                decryptable_available_balance: 97,
            },
        )
        .await;
}

#[cfg(feature = "zk-ops")]
#[tokio::test]
async fn confidential_transfer_transfer_memo() {
    confidential_transfer_transfer_memo_with_option(ConfidentialTransferOption::InstructionData)
        .await;
    confidential_transfer_transfer_memo_with_option(ConfidentialTransferOption::RecordAccount)
        .await;
    confidential_transfer_transfer_memo_with_option(
        ConfidentialTransferOption::ContextStateAccount,
    )
    .await;
}

#[cfg(feature = "zk-ops")]
async fn confidential_transfer_transfer_memo_with_option(option: ConfidentialTransferOption) {
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

    let TokenContext {
        token,
        alice,
        bob,
        mint_authority,
        decimals,
        ..
    } = context.token_context.unwrap();

    let alice_meta = ConfidentialTokenAccountMeta::new_with_tokens(
        &token,
        &alice,
        None,
        false,
        false,
        &mint_authority,
        42,
        decimals,
    )
    .await;

    let bob_meta = ConfidentialTokenAccountMeta::new(&token, &bob, None, true, false).await;

    // transfer without memo
    let err = confidential_transfer_with_option(
        &token,
        &alice_meta.token_account,
        &bob_meta.token_account,
        &alice.pubkey(),
        42,
        &alice_meta.elgamal_keypair,
        &alice_meta.aes_key,
        bob_meta.elgamal_keypair.pubkey(),
        Some(auditor_elgamal_keypair.pubkey()),
        None,
        &[&alice],
        option,
    )
    .await
    .unwrap_err();

    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::NoMemo as u32)
            )
        )))
    );

    // transfer with memo
    confidential_transfer_with_option(
        &token,
        &alice_meta.token_account,
        &bob_meta.token_account,
        &alice.pubkey(),
        42,
        &alice_meta.elgamal_keypair,
        &alice_meta.aes_key,
        bob_meta.elgamal_keypair.pubkey(),
        Some(auditor_elgamal_keypair.pubkey()),
        Some(("🦖", vec![alice.pubkey()])),
        &[&alice],
        option,
    )
    .await
    .unwrap();

    alice_meta
        .check_balances(
            &token,
            ConfidentialTokenAccountBalances {
                pending_balance_lo: 0,
                pending_balance_hi: 0,
                available_balance: 0,
                decryptable_available_balance: 0,
            },
        )
        .await;

    bob_meta
        .check_balances(
            &token,
            ConfidentialTokenAccountBalances {
                pending_balance_lo: 42,
                pending_balance_hi: 0,
                available_balance: 0,
                decryptable_available_balance: 0,
            },
        )
        .await;
}

#[cfg(feature = "zk-ops")]
#[tokio::test]
async fn confidential_transfer_transfer_with_fee_and_memo() {
    confidential_transfer_transfer_with_fee_and_memo_option(
        ConfidentialTransferOption::InstructionData,
    )
    .await;
    confidential_transfer_transfer_with_fee_and_memo_option(
        ConfidentialTransferOption::RecordAccount,
    )
    .await;
    confidential_transfer_transfer_with_fee_and_memo_option(
        ConfidentialTransferOption::ContextStateAccount,
    )
    .await;
}

#[cfg(feature = "zk-ops")]
async fn confidential_transfer_transfer_with_fee_and_memo_option(
    option: ConfidentialTransferOption,
) {
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

    let TokenContext {
        token,
        alice,
        bob,
        mint_authority,
        decimals,
        ..
    } = context.token_context.unwrap();

    let alice_meta = ConfidentialTokenAccountMeta::new_with_tokens(
        &token,
        &alice,
        None,
        false,
        true,
        &mint_authority,
        100,
        decimals,
    )
    .await;

    let bob_meta = ConfidentialTokenAccountMeta::new(&token, &bob, None, true, true).await;

    let err = confidential_transfer_with_fee_with_option(
        &token,
        &alice_meta.token_account,
        &bob_meta.token_account,
        &alice.pubkey(),
        100,
        &alice_meta.elgamal_keypair,
        &alice_meta.aes_key,
        bob_meta.elgamal_keypair.pubkey(),
        Some(auditor_elgamal_keypair.pubkey()),
        withdraw_withheld_authority_elgamal_keypair.pubkey(),
        TEST_FEE_BASIS_POINTS,
        TEST_MAXIMUM_FEE,
        None,
        &[&alice],
        option,
    )
    .await
    .unwrap_err();

    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::NoMemo as u32)
            )
        )))
    );

    confidential_transfer_with_fee_with_option(
        &token,
        &alice_meta.token_account,
        &bob_meta.token_account,
        &alice.pubkey(),
        100,
        &alice_meta.elgamal_keypair,
        &alice_meta.aes_key,
        bob_meta.elgamal_keypair.pubkey(),
        Some(auditor_elgamal_keypair.pubkey()),
        withdraw_withheld_authority_elgamal_keypair.pubkey(),
        TEST_FEE_BASIS_POINTS,
        TEST_MAXIMUM_FEE,
        Some(("🦖", vec![alice.pubkey()])),
        &[&alice],
        option,
    )
    .await
    .unwrap();

    alice_meta
        .check_balances(
            &token,
            ConfidentialTokenAccountBalances {
                pending_balance_lo: 0,
                pending_balance_hi: 0,
                available_balance: 0,
                decryptable_available_balance: 0,
            },
        )
        .await;

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
}

#[tokio::test]
async fn confidential_transfer_configure_token_account_with_registry() {
    let authority = Keypair::new();
    let auto_approve_new_accounts = false;
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

    let TokenContext { token, alice, .. } = context.token_context.unwrap();
    let alice_account_keypair = Keypair::new();
    token
        .create_auxiliary_token_account_with_extension_space(
            &alice_account_keypair,
            &alice.pubkey(),
            vec![ExtensionType::ConfidentialTransferAccount],
        )
        .await
        .unwrap();
    let elgamal_keypair = ElGamalKeypair::new_rand();

    // create ElGamal registry
    let ctx = context.context.lock().await;
    let proof_data =
        confidential_transfer::instruction::PubkeyValidityProofData::new(&elgamal_keypair).unwrap();
    let proof_location = ProofLocation::InstructionOffset(
        1.try_into().unwrap(),
        ProofData::InstructionData(&proof_data),
    );

    let elgamal_registry_address = spl_elgamal_registry::get_elgamal_registry_address(
        &alice.pubkey(),
        &spl_elgamal_registry::id(),
    );

    let rent = ctx.banks_client.get_rent().await.unwrap();
    let space = ELGAMAL_REGISTRY_ACCOUNT_LEN;
    let system_instruction = system_instruction::transfer(
        &ctx.payer.pubkey(),
        &elgamal_registry_address,
        rent.minimum_balance(space),
    );
    let create_registry_instructions =
        spl_elgamal_registry::instruction::create_registry(&alice.pubkey(), proof_location)
            .unwrap();

    let instructions = [&[system_instruction], &create_registry_instructions[..]].concat();
    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &alice],
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await.unwrap();

    // update ElGamal registry
    let new_elgamal_keypair =
        ElGamalKeypair::new_from_signer(&alice, &alice_account_keypair.pubkey().to_bytes())
            .unwrap();
    let proof_data =
        confidential_transfer::instruction::PubkeyValidityProofData::new(&new_elgamal_keypair)
            .unwrap();
    let proof_location = ProofLocation::InstructionOffset(
        1.try_into().unwrap(),
        ProofData::InstructionData(&proof_data),
    );

    let payer_pubkey = ctx.payer.pubkey();
    let instructions =
        spl_elgamal_registry::instruction::update_registry(&alice.pubkey(), proof_location)
            .unwrap();
    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &alice],
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await.unwrap();
    drop(ctx);

    // configure account using ElGamal registry
    let alice_account_keypair = Keypair::new();
    let alice_token_account = alice_account_keypair.pubkey();
    token
        .create_auxiliary_token_account_with_extension_space(
            &alice_account_keypair,
            &alice.pubkey(),
            vec![], // do not allocate space for confidential transfers
        )
        .await
        .unwrap();

    token
        .confidential_transfer_configure_token_account_with_registry(
            &alice_account_keypair.pubkey(),
            &elgamal_registry_address,
            Some(&payer_pubkey), // test account allocation
        )
        .await
        .unwrap();

    let state = token.get_account_info(&alice_token_account).await.unwrap();
    let extension = state
        .get_extension::<ConfidentialTransferAccount>()
        .unwrap();
    assert!(!bool::from(&extension.approved));
    assert!(bool::from(&extension.allow_confidential_credits));
    assert_eq!(
        extension.elgamal_pubkey,
        (*new_elgamal_keypair.pubkey()).into()
    );
}
