#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{
        ConfidentialTokenAccountBalances, ConfidentialTokenAccountMeta, TestContext, TokenContext,
    },
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
                self,
                account_info::TransferAccountInfo,
                instruction::{
                    CloseSplitContextStateAccounts, TransferSplitContextStateAccounts,
                    TransferWithFeeSplitContextStateAccounts,
                },
                ConfidentialTransferAccount, MAXIMUM_DEPOSIT_TRANSFER_AMOUNT,
            },
            BaseStateWithExtensions, ExtensionType,
        },
        solana_zk_token_sdk::{
            encryption::{auth_encryption::*, elgamal::*},
            zk_token_elgamal::pod::{self, Zeroable},
            zk_token_proof_instruction::*,
            zk_token_proof_program,
            zk_token_proof_state::ProofContextState,
        },
    },
    spl_token_client::{
        proof_generation::transfer_with_fee_split_proof_data,
        token::{ComputeUnitLimit, ExtensionInitializationParams, TokenError as TokenClientError},
    },
    std::{convert::TryInto, mem::size_of},
};

#[cfg(feature = "zk-ops")]
const TEST_MAXIMUM_FEE: u64 = 100;
#[cfg(feature = "zk-ops")]
const TEST_FEE_BASIS_POINTS: u16 = 250;

#[tokio::test]
async fn confidential_transfer_configure_token_account() {
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
    let alice_meta = ConfidentialTokenAccountMeta::new(&token, &alice, None, false, false).await;
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
    let err = token
        .confidential_transfer_configure_token_account(
            &alice_meta.token_account,
            &alice.pubkey(),
            None,
            None,
            &alice_meta.elgamal_keypair,
            &alice_meta.aes_key,
            &[&alice],
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

#[tokio::test]
async fn confidential_transfer_empty_account() {
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
    assert_eq!(
        extension.pending_balance_lo,
        pod::ElGamalCiphertext::zeroed()
    );
    assert_eq!(
        extension.pending_balance_hi,
        pod::ElGamalCiphertext::zeroed()
    );
    assert_eq!(
        extension.available_balance,
        pod::ElGamalCiphertext::zeroed()
    );

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

#[cfg(feature = "zk-ops")]
#[tokio::test]
async fn confidential_transfer_withdraw() {
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
    token
        .confidential_transfer_withdraw(
            &alice_meta.token_account,
            &alice.pubkey(),
            None,
            0,
            decimals,
            None,
            &alice_meta.elgamal_keypair,
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

    // withdraw entire balance
    token
        .confidential_transfer_withdraw(
            &alice_meta.token_account,
            &alice.pubkey(),
            None,
            42,
            decimals,
            None,
            &alice_meta.elgamal_keypair,
            &alice_meta.aes_key,
            &[&alice],
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

    // attempt to withdraw without enough funds
    let err = token
        .confidential_transfer_withdraw(
            &alice_meta.token_account,
            &alice.pubkey(),
            None,
            1,
            decimals,
            None,
            &alice_meta.elgamal_keypair,
            &alice_meta.aes_key,
            &[&alice],
        )
        .await
        .unwrap_err();

    assert_eq!(err, TokenClientError::ProofGeneration);

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
}

#[cfg(feature = "zk-ops")]
#[tokio::test]
async fn confidential_transfer_transfer() {
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
    token
        .confidential_transfer_transfer(
            &alice_meta.token_account,
            &alice_meta.token_account,
            &alice.pubkey(),
            None,
            0,
            None,
            &alice_meta.elgamal_keypair,
            &alice_meta.aes_key,
            alice_meta.elgamal_keypair.pubkey(),
            Some(auditor_elgamal_keypair.pubkey()),
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

    // Self-transfer of N tokens
    token
        .confidential_transfer_transfer(
            &alice_meta.token_account,
            &alice_meta.token_account,
            &alice.pubkey(),
            None,
            42,
            None,
            &alice_meta.elgamal_keypair,
            &alice_meta.aes_key,
            alice_meta.elgamal_keypair.pubkey(),
            Some(auditor_elgamal_keypair.pubkey()),
            &[&alice],
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

    token
        .confidential_transfer_transfer(
            &alice_meta.token_account,
            &bob_meta.token_account,
            &alice.pubkey(),
            None,
            42,
            None,
            &alice_meta.elgamal_keypair,
            &alice_meta.aes_key,
            bob_meta.elgamal_keypair.pubkey(),
            Some(auditor_elgamal_keypair.pubkey()),
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

    token
        .confidential_transfer_transfer(
            &bob_meta.token_account,
            &bob_meta.token_account,
            &bob.pubkey(),
            None,
            0,
            None,
            &bob_meta.elgamal_keypair,
            &bob_meta.aes_key,
            bob_meta.elgamal_keypair.pubkey(),
            Some(auditor_elgamal_keypair.pubkey()),
            &[&bob],
        )
        .await
        .unwrap();

    let err = token
        .confidential_transfer_transfer(
            &bob_meta.token_account,
            &bob_meta.token_account,
            &bob.pubkey(),
            None,
            0,
            None,
            &bob_meta.elgamal_keypair,
            &bob_meta.aes_key,
            bob_meta.elgamal_keypair.pubkey(),
            Some(auditor_elgamal_keypair.pubkey()),
            &[&bob],
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

#[cfg(feature = "zk-ops")]
#[tokio::test]
async fn confidential_transfer_transfer_with_fee() {
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
    token
        .confidential_transfer_transfer_with_fee(
            &alice_meta.token_account,
            &alice_meta.token_account,
            &alice.pubkey(),
            None,
            0,
            None,
            &alice_meta.elgamal_keypair,
            &alice_meta.aes_key,
            alice_meta.elgamal_keypair.pubkey(),
            Some(auditor_elgamal_keypair.pubkey()),
            withdraw_withheld_authority_elgamal_keypair.pubkey(),
            TEST_FEE_BASIS_POINTS,
            TEST_MAXIMUM_FEE,
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

    // Self-transfers does not incur a fee
    token
        .confidential_transfer_transfer_with_fee(
            &alice_meta.token_account,
            &alice_meta.token_account,
            &alice.pubkey(),
            None,
            100,
            None,
            &alice_meta.elgamal_keypair,
            &alice_meta.aes_key,
            alice_meta.elgamal_keypair.pubkey(),
            Some(auditor_elgamal_keypair.pubkey()),
            withdraw_withheld_authority_elgamal_keypair.pubkey(),
            TEST_FEE_BASIS_POINTS,
            TEST_MAXIMUM_FEE,
            &[&alice],
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

    token
        .confidential_transfer_transfer_with_fee(
            &alice_meta.token_account,
            &bob_meta.token_account,
            &alice.pubkey(),
            None,
            100,
            None,
            &alice_meta.elgamal_keypair,
            &alice_meta.aes_key,
            bob_meta.elgamal_keypair.pubkey(),
            Some(auditor_elgamal_keypair.pubkey()),
            withdraw_withheld_authority_elgamal_keypair.pubkey(),
            TEST_FEE_BASIS_POINTS,
            TEST_MAXIMUM_FEE,
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
    let err = token
        .confidential_transfer_transfer(
            &alice_meta.token_account,
            &bob_meta.token_account,
            &alice.pubkey(),
            None,
            42,
            None,
            &alice_meta.elgamal_keypair,
            &alice_meta.aes_key,
            bob_meta.elgamal_keypair.pubkey(),
            Some(auditor_elgamal_keypair.pubkey()),
            &[&alice],
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
    token
        .with_memo("🦖", vec![alice.pubkey()])
        .confidential_transfer_transfer(
            &alice_meta.token_account,
            &bob_meta.token_account,
            &alice.pubkey(),
            None,
            42,
            None,
            &alice_meta.elgamal_keypair,
            &alice_meta.aes_key,
            bob_meta.elgamal_keypair.pubkey(),
            Some(auditor_elgamal_keypair.pubkey()),
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

    let err = token
        .confidential_transfer_transfer_with_fee(
            &alice_meta.token_account,
            &bob_meta.token_account,
            &alice.pubkey(),
            None,
            100,
            None,
            &alice_meta.elgamal_keypair,
            &alice_meta.aes_key,
            bob_meta.elgamal_keypair.pubkey(),
            Some(auditor_elgamal_keypair.pubkey()),
            withdraw_withheld_authority_elgamal_keypair.pubkey(),
            TEST_FEE_BASIS_POINTS,
            TEST_MAXIMUM_FEE,
            &[&alice],
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

    token
        .with_memo("🦖", vec![alice.pubkey()])
        .confidential_transfer_transfer_with_fee(
            &alice_meta.token_account,
            &bob_meta.token_account,
            &alice.pubkey(),
            None,
            100,
            None,
            &alice_meta.elgamal_keypair,
            &alice_meta.aes_key,
            bob_meta.elgamal_keypair.pubkey(),
            Some(auditor_elgamal_keypair.pubkey()),
            withdraw_withheld_authority_elgamal_keypair.pubkey(),
            TEST_FEE_BASIS_POINTS,
            TEST_MAXIMUM_FEE,
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
async fn confidential_transfer_configure_token_account_with_proof_context() {
    let authority = Keypair::new();
    let auto_approve_new_accounts = false;

    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![
            ExtensionInitializationParams::ConfidentialTransferMint {
                authority: Some(authority.pubkey()),
                auto_approve_new_accounts,
                auditor_elgamal_pubkey: None,
            },
        ])
        .await
        .unwrap();

    let TokenContext {
        token, alice, bob, ..
    } = context.token_context.unwrap();

    let token_account_keypair = Keypair::new();
    token
        .create_auxiliary_token_account_with_extension_space(
            &token_account_keypair,
            &alice.pubkey(),
            vec![ExtensionType::ConfidentialTransferAccount],
        )
        .await
        .unwrap();
    let token_account = token_account_keypair.pubkey();

    let elgamal_keypair =
        ElGamalKeypair::new_from_signer(&alice, &token_account.to_bytes()).unwrap();
    let aes_key = AeKey::new_from_signer(&alice, &token_account.to_bytes()).unwrap();

    let context_state_account = Keypair::new();

    // create context state
    {
        let context_state_authority = Keypair::new();
        let space = size_of::<ProofContextState<PubkeyValidityProofContext>>();

        let instruction_type = ProofInstruction::VerifyPubkeyValidity;

        let context_state_info = ContextStateInfo {
            context_state_account: &context_state_account.pubkey(),
            context_state_authority: &context_state_authority.pubkey(),
        };

        let proof_data =
            confidential_transfer::instruction::PubkeyValidityData::new(&elgamal_keypair).unwrap();

        let mut ctx = context.context.lock().await;
        let rent = ctx.banks_client.get_rent().await.unwrap();

        let instructions = vec![
            system_instruction::create_account(
                &ctx.payer.pubkey(),
                &context_state_account.pubkey(),
                rent.minimum_balance(space),
                space as u64,
                &zk_token_proof_program::id(),
            ),
            instruction_type.encode_verify_proof(Some(context_state_info), &proof_data),
        ];

        let tx = Transaction::new_signed_with_payer(
            &instructions,
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer, &context_state_account],
            ctx.last_blockhash,
        );
        ctx.banks_client.process_transaction(tx).await.unwrap();
    }

    token
        .confidential_transfer_configure_token_account(
            &token_account,
            &alice.pubkey(),
            Some(&context_state_account.pubkey()),
            None,
            &elgamal_keypair,
            &aes_key,
            &[&alice],
        )
        .await
        .unwrap();

    let elgamal_pubkey = (*elgamal_keypair.pubkey()).into();

    let state = token.get_account_info(&token_account).await.unwrap();
    let extension = state
        .get_extension::<ConfidentialTransferAccount>()
        .unwrap();
    assert!(!bool::from(&extension.approved));
    assert!(bool::from(&extension.allow_confidential_credits));
    assert_eq!(extension.elgamal_pubkey, elgamal_pubkey);
    assert_eq!(
        aes_key
            .decrypt(&(extension.decryptable_available_balance.try_into().unwrap()))
            .unwrap(),
        0
    );

    // attempt to create an account with a wrong proof type context state
    let token_account_keypair = Keypair::new();
    token
        .create_auxiliary_token_account_with_extension_space(
            &token_account_keypair,
            &bob.pubkey(),
            vec![ExtensionType::ConfidentialTransferAccount],
        )
        .await
        .unwrap();
    let token_account = token_account_keypair.pubkey();

    let elgamal_keypair = ElGamalKeypair::new_from_signer(&bob, &token_account.to_bytes()).unwrap();
    let aes_key = AeKey::new_from_signer(&bob, &token_account.to_bytes()).unwrap();

    let context_state_account = Keypair::new();

    {
        let context_state_authority = Keypair::new();
        let space = size_of::<ProofContextState<ZeroBalanceProofContext>>();

        let instruction_type = ProofInstruction::VerifyZeroBalance;
        let context_state_info = ContextStateInfo {
            context_state_account: &context_state_account.pubkey(),
            context_state_authority: &context_state_authority.pubkey(),
        };

        let ciphertext = elgamal_keypair.pubkey().encrypt(0_u64);
        let proof_data = confidential_transfer::instruction::ZeroBalanceProofData::new(
            &elgamal_keypair,
            &ciphertext,
        )
        .unwrap();

        let mut ctx = context.context.lock().await;
        let rent = ctx.banks_client.get_rent().await.unwrap();

        let instructions = vec![
            system_instruction::create_account(
                &ctx.payer.pubkey(),
                &context_state_account.pubkey(),
                rent.minimum_balance(space),
                space as u64,
                &zk_token_proof_program::id(),
            ),
            instruction_type.encode_verify_proof(Some(context_state_info), &proof_data),
        ];

        let tx = Transaction::new_signed_with_payer(
            &instructions,
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer, &context_state_account],
            ctx.last_blockhash,
        );
        ctx.banks_client.process_transaction(tx).await.unwrap();
    }

    let err = token
        .confidential_transfer_configure_token_account(
            &token_account,
            &bob.pubkey(),
            Some(&context_state_account.pubkey()),
            None,
            &elgamal_keypair,
            &aes_key,
            &[&bob],
        )
        .await
        .unwrap_err();

    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(0, InstructionError::InvalidArgument,)
        )))
    );
}

#[tokio::test]
async fn confidential_transfer_empty_account_with_proof_context() {
    let authority = Keypair::new();
    let auto_approve_new_accounts = false;

    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![
            ExtensionInitializationParams::ConfidentialTransferMint {
                authority: Some(authority.pubkey()),
                auto_approve_new_accounts,
                auditor_elgamal_pubkey: None,
            },
        ])
        .await
        .unwrap();

    let TokenContext {
        token, alice, bob, ..
    } = context.token_context.unwrap();

    let alice_meta = ConfidentialTokenAccountMeta::new(&token, &alice, None, false, false).await;
    let context_state_account = Keypair::new();

    // create context state
    {
        let context_state_authority = Keypair::new();
        let space = size_of::<ProofContextState<ZeroBalanceProofContext>>();

        let instruction_type = ProofInstruction::VerifyZeroBalance;

        let context_state_info = ContextStateInfo {
            context_state_account: &context_state_account.pubkey(),
            context_state_authority: &context_state_authority.pubkey(),
        };

        let proof_data = confidential_transfer::instruction::ZeroBalanceProofData::new(
            &alice_meta.elgamal_keypair,
            &ElGamalCiphertext::default(),
        )
        .unwrap();

        let mut ctx = context.context.lock().await;
        let rent = ctx.banks_client.get_rent().await.unwrap();

        let instructions = vec![
            system_instruction::create_account(
                &ctx.payer.pubkey(),
                &context_state_account.pubkey(),
                rent.minimum_balance(space),
                space as u64,
                &zk_token_proof_program::id(),
            ),
            instruction_type.encode_verify_proof(Some(context_state_info), &proof_data),
        ];

        let tx = Transaction::new_signed_with_payer(
            &instructions,
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer, &context_state_account],
            ctx.last_blockhash,
        );
        ctx.banks_client.process_transaction(tx).await.unwrap();
    }

    token
        .confidential_transfer_empty_account(
            &alice_meta.token_account,
            &alice.pubkey(),
            Some(&context_state_account.pubkey()),
            None,
            &alice_meta.elgamal_keypair,
            &[&alice],
        )
        .await
        .unwrap();

    // attempt to create an account with a wrong proof type context state
    let bob_meta = ConfidentialTokenAccountMeta::new(&token, &bob, None, false, false).await;
    let context_state_account = Keypair::new();

    {
        let context_state_authority = Keypair::new();
        let space = size_of::<ProofContextState<PubkeyValidityProofContext>>();

        let instruction_type = ProofInstruction::VerifyPubkeyValidity;

        let context_state_info = ContextStateInfo {
            context_state_account: &context_state_account.pubkey(),
            context_state_authority: &context_state_authority.pubkey(),
        };

        let proof_data =
            confidential_transfer::instruction::PubkeyValidityData::new(&bob_meta.elgamal_keypair)
                .unwrap();

        let mut ctx = context.context.lock().await;
        let rent = ctx.banks_client.get_rent().await.unwrap();

        let instructions = vec![
            system_instruction::create_account(
                &ctx.payer.pubkey(),
                &context_state_account.pubkey(),
                rent.minimum_balance(space),
                space as u64,
                &zk_token_proof_program::id(),
            ),
            instruction_type.encode_verify_proof(Some(context_state_info), &proof_data),
        ];

        let tx = Transaction::new_signed_with_payer(
            &instructions,
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer, &context_state_account],
            ctx.last_blockhash,
        );
        ctx.banks_client.process_transaction(tx).await.unwrap();
    }

    let err = token
        .confidential_transfer_empty_account(
            &bob_meta.token_account,
            &bob.pubkey(),
            Some(&context_state_account.pubkey()),
            None,
            &bob_meta.elgamal_keypair,
            &[&bob],
        )
        .await
        .unwrap_err();

    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(0, InstructionError::InvalidArgument,)
        )))
    );
}

#[tokio::test]
async fn confidential_transfer_withdraw_with_proof_context() {
    let authority = Keypair::new();
    let auto_approve_new_accounts = true;

    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![
            ExtensionInitializationParams::ConfidentialTransferMint {
                authority: Some(authority.pubkey()),
                auto_approve_new_accounts,
                auditor_elgamal_pubkey: None,
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

    let context_state_account = Keypair::new();

    // create context state
    {
        let context_state_authority = Keypair::new();
        let space = size_of::<ProofContextState<WithdrawProofContext>>();

        let instruction_type = ProofInstruction::VerifyWithdraw;

        let context_state_info = ContextStateInfo {
            context_state_account: &context_state_account.pubkey(),
            context_state_authority: &context_state_authority.pubkey(),
        };

        let state = token
            .get_account_info(&alice_meta.token_account)
            .await
            .unwrap();
        let extension = state
            .get_extension::<ConfidentialTransferAccount>()
            .unwrap();
        let current_ciphertext = extension.available_balance.try_into().unwrap();

        let proof_data = confidential_transfer::instruction::WithdrawData::new(
            0,
            &alice_meta.elgamal_keypair,
            42,
            &current_ciphertext,
        )
        .unwrap();

        let mut ctx = context.context.lock().await;
        let rent = ctx.banks_client.get_rent().await.unwrap();

        let instructions = vec![
            system_instruction::create_account(
                &ctx.payer.pubkey(),
                &context_state_account.pubkey(),
                rent.minimum_balance(space),
                space as u64,
                &zk_token_proof_program::id(),
            ),
            instruction_type.encode_verify_proof(Some(context_state_info), &proof_data),
        ];

        let tx = Transaction::new_signed_with_payer(
            &instructions,
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer, &context_state_account],
            ctx.last_blockhash,
        );
        ctx.banks_client.process_transaction(tx).await.unwrap();
    }

    token
        .confidential_transfer_withdraw(
            &alice_meta.token_account,
            &alice.pubkey(),
            Some(&context_state_account.pubkey()),
            0,
            decimals,
            None,
            &alice_meta.elgamal_keypair,
            &alice_meta.aes_key,
            &[&alice],
        )
        .await
        .unwrap();

    // attempt to create an account with a wrong proof type context state
    let bob_meta = ConfidentialTokenAccountMeta::new(&token, &bob, None, false, false).await;
    let context_state_account = Keypair::new();

    {
        let context_state_authority = Keypair::new();
        let space = size_of::<ProofContextState<PubkeyValidityProofContext>>();

        let instruction_type = ProofInstruction::VerifyPubkeyValidity;

        let context_state_info = ContextStateInfo {
            context_state_account: &context_state_account.pubkey(),
            context_state_authority: &context_state_authority.pubkey(),
        };

        let proof_data =
            confidential_transfer::instruction::PubkeyValidityData::new(&bob_meta.elgamal_keypair)
                .unwrap();

        let mut ctx = context.context.lock().await;
        let rent = ctx.banks_client.get_rent().await.unwrap();

        let instructions = vec![
            system_instruction::create_account(
                &ctx.payer.pubkey(),
                &context_state_account.pubkey(),
                rent.minimum_balance(space),
                space as u64,
                &zk_token_proof_program::id(),
            ),
            instruction_type.encode_verify_proof(Some(context_state_info), &proof_data),
        ];

        let tx = Transaction::new_signed_with_payer(
            &instructions,
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer, &context_state_account],
            ctx.last_blockhash,
        );
        ctx.banks_client.process_transaction(tx).await.unwrap();
    }

    let err = token
        .confidential_transfer_withdraw(
            &bob_meta.token_account,
            &bob.pubkey(),
            Some(&context_state_account.pubkey()),
            0,
            decimals,
            None,
            &bob_meta.elgamal_keypair,
            &bob_meta.aes_key,
            &[&bob],
        )
        .await
        .unwrap_err();

    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(0, InstructionError::InvalidArgument,)
        )))
    );
}

#[tokio::test]
async fn confidential_transfer_transfer_with_proof_context() {
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

    let bob_meta = ConfidentialTokenAccountMeta::new_with_tokens(
        &token,
        &bob,
        None,
        false,
        false,
        &mint_authority,
        0,
        decimals,
    )
    .await;

    let context_state_account = Keypair::new();

    // create context state
    {
        let context_state_authority = Keypair::new();
        let space = size_of::<ProofContextState<TransferProofContext>>();

        let instruction_type = ProofInstruction::VerifyTransfer;

        let context_state_info = ContextStateInfo {
            context_state_account: &context_state_account.pubkey(),
            context_state_authority: &context_state_authority.pubkey(),
        };

        let state = token
            .get_account_info(&alice_meta.token_account)
            .await
            .unwrap();
        let extension = state
            .get_extension::<ConfidentialTransferAccount>()
            .unwrap();
        let current_available_balance = extension.available_balance.try_into().unwrap();

        let proof_data = confidential_transfer::instruction::TransferData::new(
            42,
            (42, &current_available_balance),
            &alice_meta.elgamal_keypair,
            (
                bob_meta.elgamal_keypair.pubkey(),
                auditor_elgamal_keypair.pubkey(),
            ),
        )
        .unwrap();

        let mut ctx = context.context.lock().await;
        let rent = ctx.banks_client.get_rent().await.unwrap();

        let instructions = vec![
            system_instruction::create_account(
                &ctx.payer.pubkey(),
                &context_state_account.pubkey(),
                rent.minimum_balance(space),
                space as u64,
                &zk_token_proof_program::id(),
            ),
            instruction_type.encode_verify_proof(Some(context_state_info), &proof_data),
        ];

        let tx = Transaction::new_signed_with_payer(
            &instructions,
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer, &context_state_account],
            ctx.last_blockhash,
        );
        ctx.banks_client.process_transaction(tx).await.unwrap();
    }

    token
        .confidential_transfer_transfer(
            &alice_meta.token_account,
            &bob_meta.token_account,
            &alice.pubkey(),
            Some(&context_state_account.pubkey()),
            42,
            None,
            &alice_meta.elgamal_keypair,
            &alice_meta.aes_key,
            bob_meta.elgamal_keypair.pubkey(),
            Some(auditor_elgamal_keypair.pubkey()),
            &[&alice],
        )
        .await
        .unwrap();

    // attempt to create an account with a wrong proof type context state
    let context_state_account = Keypair::new();

    {
        let context_state_authority = Keypair::new();
        let space = size_of::<ProofContextState<WithdrawProofContext>>();

        let instruction_type = ProofInstruction::VerifyWithdraw;

        let context_state_info = ContextStateInfo {
            context_state_account: &context_state_account.pubkey(),
            context_state_authority: &context_state_authority.pubkey(),
        };

        let state = token
            .get_account_info(&alice_meta.token_account)
            .await
            .unwrap();
        let extension = state
            .get_extension::<ConfidentialTransferAccount>()
            .unwrap();
        let current_ciphertext = extension.available_balance.try_into().unwrap();

        let proof_data = confidential_transfer::instruction::WithdrawData::new(
            0,
            &alice_meta.elgamal_keypair,
            0,
            &current_ciphertext,
        )
        .unwrap();

        let mut ctx = context.context.lock().await;
        let rent = ctx.banks_client.get_rent().await.unwrap();

        let instructions = vec![
            system_instruction::create_account(
                &ctx.payer.pubkey(),
                &context_state_account.pubkey(),
                rent.minimum_balance(space),
                space as u64,
                &zk_token_proof_program::id(),
            ),
            instruction_type.encode_verify_proof(Some(context_state_info), &proof_data),
        ];

        let tx = Transaction::new_signed_with_payer(
            &instructions,
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer, &context_state_account],
            ctx.last_blockhash,
        );
        ctx.banks_client.process_transaction(tx).await.unwrap();
    }

    let err = token
        .confidential_transfer_transfer(
            &alice_meta.token_account,
            &bob_meta.token_account,
            &alice.pubkey(),
            Some(&context_state_account.pubkey()),
            0,
            None,
            &alice_meta.elgamal_keypair,
            &alice_meta.aes_key,
            bob_meta.elgamal_keypair.pubkey(),
            Some(auditor_elgamal_keypair.pubkey()),
            &[&alice],
        )
        .await
        .unwrap_err();

    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(0, InstructionError::InvalidArgument,)
        )))
    )
}

#[tokio::test]
async fn confidential_transfer_transfer_with_split_proof_context() {
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

    let bob_meta = ConfidentialTokenAccountMeta::new_with_tokens(
        &token,
        &bob,
        None,
        false,
        false,
        &mint_authority,
        0,
        decimals,
    )
    .await;

    let state = token
        .get_account_info(&alice_meta.token_account)
        .await
        .unwrap();
    let extension = state
        .get_extension::<ConfidentialTransferAccount>()
        .unwrap();
    let transfer_account_info = TransferAccountInfo::new(extension);

    let (
        equality_proof_data,
        ciphertext_validity_proof_data,
        range_proof_data,
        source_decrypt_handles,
    ) = transfer_account_info
        .generate_split_transfer_proof_data(
            42,
            &alice_meta.elgamal_keypair,
            &alice_meta.aes_key,
            bob_meta.elgamal_keypair.pubkey(),
            Some(auditor_elgamal_keypair.pubkey()),
        )
        .unwrap();

    let context_state_authority = Keypair::new();
    let equality_proof_context_state_account = Keypair::new();
    let ciphertext_validity_proof_context_state_account = Keypair::new();
    let range_proof_context_state_account = Keypair::new();

    let transfer_context_state_accounts = TransferSplitContextStateAccounts {
        equality_proof: &equality_proof_context_state_account.pubkey(),
        ciphertext_validity_proof: &ciphertext_validity_proof_context_state_account.pubkey(),
        range_proof: &range_proof_context_state_account.pubkey(),
        authority: &context_state_authority.pubkey(),
        no_op_on_uninitialized_split_context_state: false,
        close_split_context_state_accounts: None,
    };

    // create context state accounts
    token
        .create_equality_and_ciphertext_validity_proof_context_states_for_transfer(
            transfer_context_state_accounts,
            &equality_proof_data,
            &ciphertext_validity_proof_data,
            &[
                &equality_proof_context_state_account,
                &ciphertext_validity_proof_context_state_account,
            ],
        )
        .await
        .unwrap();

    token
        .create_range_proof_context_state_for_transfer(
            transfer_context_state_accounts,
            &range_proof_data,
            &range_proof_context_state_account,
        )
        .await
        .unwrap();

    // create token22 confidential transfer instruction
    token
        .confidential_transfer_transfer_with_split_proofs(
            &alice_meta.token_account,
            &bob_meta.token_account,
            &alice.pubkey(),
            transfer_context_state_accounts,
            42,
            None,
            &alice_meta.aes_key,
            &source_decrypt_handles,
            &[&alice],
        )
        .await
        .unwrap();

    // close context state accounts
    token
        .confidential_transfer_close_context_state(
            &equality_proof_context_state_account.pubkey(),
            &alice_meta.token_account,
            &context_state_authority.pubkey(),
            &[&context_state_authority],
        )
        .await
        .unwrap();

    token
        .confidential_transfer_close_context_state(
            &ciphertext_validity_proof_context_state_account.pubkey(),
            &alice_meta.token_account,
            &context_state_authority.pubkey(),
            &[&context_state_authority],
        )
        .await
        .unwrap();

    token
        .confidential_transfer_close_context_state(
            &range_proof_context_state_account.pubkey(),
            &alice_meta.token_account,
            &context_state_authority.pubkey(),
            &[&context_state_authority],
        )
        .await
        .unwrap();

    // check balances
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

#[tokio::test]
async fn confidential_transfer_transfer_with_split_proof_contexts_in_parallel() {
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

    let bob_meta = ConfidentialTokenAccountMeta::new_with_tokens(
        &token,
        &bob,
        None,
        false,
        false,
        &mint_authority,
        0,
        decimals,
    )
    .await;

    let context_state_authority = Keypair::new();
    let equality_proof_context_state_account = Keypair::new();
    let ciphertext_validity_proof_context_state_account = Keypair::new();
    let range_proof_context_state_account = Keypair::new();

    let lamport_destination = Pubkey::new_unique();

    let close_split_context_state_accounts = CloseSplitContextStateAccounts {
        lamport_destination: &lamport_destination,
        zk_token_proof_program: &zk_token_proof_program::id(),
    };

    let transfer_context_state_accounts = TransferSplitContextStateAccounts {
        equality_proof: &equality_proof_context_state_account.pubkey(),
        ciphertext_validity_proof: &ciphertext_validity_proof_context_state_account.pubkey(),
        range_proof: &range_proof_context_state_account.pubkey(),
        authority: &context_state_authority.pubkey(),
        no_op_on_uninitialized_split_context_state: true,
        close_split_context_state_accounts: Some(close_split_context_state_accounts),
    };

    let equality_and_ciphertext_proof_signers = vec![
        &alice,
        &equality_proof_context_state_account,
        &ciphertext_validity_proof_context_state_account,
        &context_state_authority,
    ];
    let range_proof_signers = vec![
        &alice,
        &range_proof_context_state_account,
        &context_state_authority,
    ];
    // With split proofs in parallel, one of the transactions does more work
    // than the other, which isn't caught during the simulation to discover the
    // compute unit limit.
    let token = token.with_compute_unit_limit(ComputeUnitLimit::Static(500_000));
    token
        .confidential_transfer_transfer_with_split_proofs_in_parallel(
            &alice_meta.token_account,
            &bob_meta.token_account,
            &alice.pubkey(),
            transfer_context_state_accounts,
            42,
            None,
            &alice_meta.elgamal_keypair,
            &alice_meta.aes_key,
            bob_meta.elgamal_keypair.pubkey(),
            Some(auditor_elgamal_keypair.pubkey()),
            &equality_and_ciphertext_proof_signers,
            &range_proof_signers,
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

    let error = token
        .get_account(equality_proof_context_state_account.pubkey())
        .await
        .unwrap_err();
    assert_eq!(error, TokenClientError::AccountNotFound);

    let error = token
        .get_account(ciphertext_validity_proof_context_state_account.pubkey())
        .await
        .unwrap_err();
    assert_eq!(error, TokenClientError::AccountNotFound);

    let error = token
        .get_account(range_proof_context_state_account.pubkey())
        .await
        .unwrap_err();
    assert_eq!(error, TokenClientError::AccountNotFound);

    let lamport_destination = token.get_account(lamport_destination).await.unwrap();
    assert!(lamport_destination.lamports > 0);
}

#[tokio::test]
async fn confidential_transfer_transfer_with_fee_and_split_proof_context() {
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

    let state = token
        .get_account_info(&alice_meta.token_account)
        .await
        .unwrap();
    let extension = state
        .get_extension::<ConfidentialTransferAccount>()
        .unwrap();
    let transfer_account_info = TransferAccountInfo::new(extension);

    let current_source_available_balance =
        transfer_account_info.available_balance.try_into().unwrap();
    let current_decryptable_available_balance = transfer_account_info
        .decryptable_available_balance
        .try_into()
        .unwrap();

    let fee_parameters = FeeParameters {
        fee_rate_basis_points: TEST_FEE_BASIS_POINTS,
        maximum_fee: TEST_MAXIMUM_FEE,
    };

    let (
        equality_proof_data,
        transfer_amount_ciphertext_validity_proof_data,
        fee_sigma_proof_data,
        fee_ciphertext_validity_proof_data,
        range_proof_data,
        source_decrypt_handles,
    ) = transfer_with_fee_split_proof_data(
        &current_source_available_balance,
        &current_decryptable_available_balance,
        100,
        &alice_meta.elgamal_keypair,
        &alice_meta.aes_key,
        bob_meta.elgamal_keypair.pubkey(),
        Some(auditor_elgamal_keypair.pubkey()),
        withdraw_withheld_authority_elgamal_keypair.pubkey(),
        &fee_parameters,
    )
    .unwrap();

    let context_state_authority = Keypair::new();
    let equality_proof_context_state_account = Keypair::new();
    let transfer_amount_ciphertext_validity_proof_context_state_account = Keypair::new();
    let fee_sigma_proof_context_state_account = Keypair::new();
    let fee_ciphertext_validity_proof_context_state_account = Keypair::new();
    let range_proof_context_state_account = Keypair::new();

    let transfer_context_state_accounts = TransferWithFeeSplitContextStateAccounts {
        equality_proof: &equality_proof_context_state_account.pubkey(),
        transfer_amount_ciphertext_validity_proof:
            &transfer_amount_ciphertext_validity_proof_context_state_account.pubkey(),
        fee_sigma_proof: &fee_sigma_proof_context_state_account.pubkey(),
        fee_ciphertext_validity_proof: &fee_ciphertext_validity_proof_context_state_account
            .pubkey(),
        range_proof: &range_proof_context_state_account.pubkey(),
        authority: &context_state_authority.pubkey(),
        no_op_on_uninitialized_split_context_state: false,
        close_split_context_state_accounts: None,
    };

    // create context state accounts
    token
        .create_equality_and_ciphertext_validity_proof_context_states_for_transfer_with_fee(
            transfer_context_state_accounts,
            &equality_proof_data,
            &transfer_amount_ciphertext_validity_proof_data,
            &[
                &equality_proof_context_state_account,
                &transfer_amount_ciphertext_validity_proof_context_state_account,
            ],
        )
        .await
        .unwrap();

    token
        .create_fee_sigma_and_ciphertext_validity_proof_context_states_for_transfer_with_fee(
            transfer_context_state_accounts,
            &fee_sigma_proof_data,
            &fee_ciphertext_validity_proof_data,
            &[
                &fee_sigma_proof_context_state_account,
                &fee_ciphertext_validity_proof_context_state_account,
            ],
        )
        .await
        .unwrap();

    token
        .create_range_proof_context_state_for_transfer_with_fee(
            transfer_context_state_accounts,
            &range_proof_data,
            &[&range_proof_context_state_account],
        )
        .await
        .unwrap();

    // create token22 confidential transfer instruction
    token
        .confidential_transfer_transfer_with_fee_and_split_proofs(
            &alice_meta.token_account,
            &bob_meta.token_account,
            &alice.pubkey(),
            transfer_context_state_accounts,
            100,
            None,
            &alice_meta.aes_key,
            &source_decrypt_handles,
            &[&alice],
        )
        .await
        .unwrap();

    // close context state accounts
    token
        .confidential_transfer_close_context_state(
            &equality_proof_context_state_account.pubkey(),
            &alice_meta.token_account,
            &context_state_authority.pubkey(),
            &[&context_state_authority],
        )
        .await
        .unwrap();

    token
        .confidential_transfer_close_context_state(
            &transfer_amount_ciphertext_validity_proof_context_state_account.pubkey(),
            &alice_meta.token_account,
            &context_state_authority.pubkey(),
            &[&context_state_authority],
        )
        .await
        .unwrap();

    token
        .confidential_transfer_close_context_state(
            &fee_sigma_proof_context_state_account.pubkey(),
            &alice_meta.token_account,
            &context_state_authority.pubkey(),
            &[&context_state_authority],
        )
        .await
        .unwrap();

    token
        .confidential_transfer_close_context_state(
            &fee_ciphertext_validity_proof_context_state_account.pubkey(),
            &alice_meta.token_account,
            &context_state_authority.pubkey(),
            &[&context_state_authority],
        )
        .await
        .unwrap();

    token
        .confidential_transfer_close_context_state(
            &range_proof_context_state_account.pubkey(),
            &alice_meta.token_account,
            &context_state_authority.pubkey(),
            &[&context_state_authority],
        )
        .await
        .unwrap();

    // check balances
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
async fn confidential_transfer_transfer_with_fee_and_split_proof_context_in_parallel() {
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

    let context_state_authority = Keypair::new();
    let equality_proof_context_state_account = Keypair::new();
    let transfer_amount_ciphertext_validity_proof_context_state_account = Keypair::new();
    let fee_sigma_proof_context_state_account = Keypair::new();
    let fee_ciphertext_validity_proof_context_state_account = Keypair::new();
    let range_proof_context_state_account = Keypair::new();

    let lamport_destination = Pubkey::new_unique();

    let close_split_context_state_accounts = CloseSplitContextStateAccounts {
        lamport_destination: &lamport_destination,
        zk_token_proof_program: &zk_token_proof_program::id(),
    };

    let transfer_context_state_accounts = TransferWithFeeSplitContextStateAccounts {
        equality_proof: &equality_proof_context_state_account.pubkey(),
        transfer_amount_ciphertext_validity_proof:
            &transfer_amount_ciphertext_validity_proof_context_state_account.pubkey(),
        fee_sigma_proof: &fee_sigma_proof_context_state_account.pubkey(),
        fee_ciphertext_validity_proof: &fee_ciphertext_validity_proof_context_state_account
            .pubkey(),
        range_proof: &range_proof_context_state_account.pubkey(),
        authority: &context_state_authority.pubkey(),
        no_op_on_uninitialized_split_context_state: true,
        close_split_context_state_accounts: Some(close_split_context_state_accounts),
    };

    let equality_and_ciphertext_proof_signers = vec![
        &alice,
        &equality_proof_context_state_account,
        &transfer_amount_ciphertext_validity_proof_context_state_account,
        &context_state_authority,
    ];
    let fee_sigma_proof_signers = vec![
        &alice,
        &fee_sigma_proof_context_state_account,
        &fee_ciphertext_validity_proof_context_state_account,
        &context_state_authority,
    ];
    let range_proof_signers = vec![
        &alice,
        &range_proof_context_state_account,
        &context_state_authority,
    ];
    // With split proofs in parallel, one of the transactions does more work
    // than the other, which isn't caught during the simulation to discover the
    // compute unit limit.
    let token = token.with_compute_unit_limit(ComputeUnitLimit::Static(500_000));
    token
        .confidential_transfer_transfer_with_fee_and_split_proofs_in_parallel(
            &alice_meta.token_account,
            &bob_meta.token_account,
            &alice.pubkey(),
            transfer_context_state_accounts,
            100,
            None,
            &alice_meta.elgamal_keypair,
            &alice_meta.aes_key,
            bob_meta.elgamal_keypair.pubkey(),
            Some(auditor_elgamal_keypair.pubkey()),
            withdraw_withheld_authority_elgamal_keypair.pubkey(),
            TEST_FEE_BASIS_POINTS,
            TEST_MAXIMUM_FEE,
            &equality_and_ciphertext_proof_signers,
            &fee_sigma_proof_signers,
            &range_proof_signers,
        )
        .await
        .unwrap();

    // check balances
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

    let error = token
        .get_account(equality_proof_context_state_account.pubkey())
        .await
        .unwrap_err();
    assert_eq!(error, TokenClientError::AccountNotFound);

    let error = token
        .get_account(transfer_amount_ciphertext_validity_proof_context_state_account.pubkey())
        .await
        .unwrap_err();
    assert_eq!(error, TokenClientError::AccountNotFound);

    let error = token
        .get_account(fee_sigma_proof_context_state_account.pubkey())
        .await
        .unwrap_err();
    assert_eq!(error, TokenClientError::AccountNotFound);

    let error = token
        .get_account(fee_ciphertext_validity_proof_context_state_account.pubkey())
        .await
        .unwrap_err();
    assert_eq!(error, TokenClientError::AccountNotFound);

    let error = token
        .get_account(range_proof_context_state_account.pubkey())
        .await
        .unwrap_err();
    assert_eq!(error, TokenClientError::AccountNotFound);

    let lamport_destination = token.get_account(lamport_destination).await.unwrap();
    assert!(lamport_destination.lamports > 0);
}
