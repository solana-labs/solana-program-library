#![cfg(feature = "test-sbf")]

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
            confidential_transfer::{ConfidentialTransferAccount, ConfidentialTransferMint},
            confidential_transfer_fee::{
                account_info::WithheldTokensInfo, ConfidentialTransferFeeAmount,
                ConfidentialTransferFeeConfig,
            },
            transfer_fee::TransferFee,
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

struct ConfidentialTokenAccountMeta {
    token_account: Pubkey,
    elgamal_keypair: ElGamalKeypair,
    aes_key: AeKey,
}

impl ConfidentialTokenAccountMeta {
    async fn new<T>(
        token: &Token<T>,
        owner: &Keypair,
        mint_authority: &Keypair,
        amount: u64,
        decimals: u8,
    ) -> Self
    where
        T: SendTransaction + SimulateTransaction,
    {
        let token_account_keypair = Keypair::new();
        let extensions = vec![
            ExtensionType::ConfidentialTransferAccount,
            ExtensionType::ConfidentialTransferFeeAmount,
        ];

        token
            .create_auxiliary_token_account_with_extension_space(
                &token_account_keypair,
                &owner.pubkey(),
                extensions,
            )
            .await
            .unwrap();
        let token_account = token_account_keypair.pubkey();

        let elgamal_keypair =
            ElGamalKeypair::new_from_signer(owner, &token_account.to_bytes()).unwrap();
        let aes_key = AeKey::new_from_signer(owner, &token_account.to_bytes()).unwrap();

        token
            .confidential_transfer_configure_token_account(
                &token_account,
                &owner.pubkey(),
                None,
                None,
                &elgamal_keypair,
                &aes_key,
                &[owner],
            )
            .await
            .unwrap();

        token
            .mint_to(
                &token_account,
                &mint_authority.pubkey(),
                amount,
                &[mint_authority],
            )
            .await
            .unwrap();

        token
            .confidential_transfer_deposit(
                &token_account,
                &owner.pubkey(),
                amount,
                decimals,
                &[owner],
            )
            .await
            .unwrap();

        token
            .confidential_transfer_apply_pending_balance(
                &token_account,
                &owner.pubkey(),
                None,
                elgamal_keypair.secret(),
                &aes_key,
                &[owner],
            )
            .await
            .unwrap();

        Self {
            token_account,
            elgamal_keypair,
            aes_key,
        }
    }

    #[cfg(feature = "zk-ops")]
    async fn check_balances<T>(&self, token: &Token<T>, expected: ConfidentialTokenAccountBalances)
    where
        T: SendTransaction + SimulateTransaction,
    {
        let state = token.get_account_info(&self.token_account).await.unwrap();
        let extension = state
            .get_extension::<ConfidentialTransferAccount>()
            .unwrap();

        assert_eq!(
            extension
                .pending_balance_lo
                .decrypt(self.elgamal_keypair.secret())
                .unwrap(),
            expected.pending_balance_lo,
        );
        assert_eq!(
            extension
                .pending_balance_hi
                .decrypt(self.elgamal_keypair.secret())
                .unwrap(),
            expected.pending_balance_hi,
        );
        assert_eq!(
            extension
                .available_balance
                .decrypt(self.elgamal_keypair.secret())
                .unwrap(),
            expected.available_balance,
        );
        assert_eq!(
            self.aes_key
                .decrypt(&extension.decryptable_available_balance.try_into().unwrap())
                .unwrap(),
            expected.decryptable_available_balance,
        );
    }
}

#[cfg(feature = "zk-ops")]
struct ConfidentialTokenAccountBalances {
    pending_balance_lo: u64,
    pending_balance_hi: u64,
    available_balance: u64,
    decryptable_available_balance: u64,
}

#[cfg(feature = "zk-ops")]
async fn check_withheld_amount_in_mint<T>(
    token: &Token<T>,
    withdraw_withheld_authority_elgamal_keypair: &ElGamalKeypair,
    expected: u64,
) where
    T: SendTransaction + SimulateTransaction,
{
    let state = token.get_mint_info().await.unwrap();
    let extension = state
        .get_extension::<ConfidentialTransferFeeConfig>()
        .unwrap();
    let decrypted_amount = extension
        .withheld_amount
        .decrypt(withdraw_withheld_authority_elgamal_keypair.secret())
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

#[cfg(feature = "zk-ops")]
#[tokio::test]
async fn confidential_transfer_withdraw_withheld_tokens_from_mint() {
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

    let alice_meta =
        ConfidentialTokenAccountMeta::new(&token, &alice, &mint_authority, 100, decimals).await;
    let bob_meta =
        ConfidentialTokenAccountMeta::new(&token, &bob, &mint_authority, 0, decimals).await;

    let transfer_fee_parameters = TransferFee {
        epoch: 0.into(),
        maximum_fee: TEST_MAXIMUM_FEE.into(),
        transfer_fee_basis_points: TEST_FEE_BASIS_POINTS.into(),
    };

    // Test fee is 2.5% so the withheld fees should be 3
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
            transfer_fee_parameters.transfer_fee_basis_points.into(),
            transfer_fee_parameters.maximum_fee.into(),
            &[&alice],
        )
        .await
        .unwrap();

    let new_decryptable_available_balance = alice_meta.aes_key.encrypt(0);
    token
        .confidential_transfer_withdraw_withheld_tokens_from_mint(
            &alice_meta.token_account,
            &withdraw_withheld_authority.pubkey(),
            None,
            None,
            &withdraw_withheld_authority_elgamal_keypair,
            alice_meta.elgamal_keypair.pubkey(),
            &new_decryptable_available_balance.into(),
            &[&withdraw_withheld_authority],
        )
        .await
        .unwrap();

    // withheld fees are not harvested to mint yet
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
        .confidential_transfer_harvest_withheld_tokens_to_mint(&[&bob_meta.token_account])
        .await
        .unwrap();

    let state = token
        .get_account_info(&bob_meta.token_account)
        .await
        .unwrap();
    let extension = state
        .get_extension::<ConfidentialTransferFeeAmount>()
        .unwrap();
    assert_eq!(extension.withheld_amount, pod::ElGamalCiphertext::zeroed());

    // calculate and encrypt fee to attach to the `WithdrawWithheldTokensFromMint`
    // instruction data
    let fee = transfer_fee_parameters.calculate_fee(100).unwrap();
    let new_decryptable_available_balance = alice_meta.aes_key.encrypt(fee);

    check_withheld_amount_in_mint(&token, &withdraw_withheld_authority_elgamal_keypair, fee).await;

    token
        .confidential_transfer_withdraw_withheld_tokens_from_mint(
            &alice_meta.token_account,
            &withdraw_withheld_authority.pubkey(),
            None,
            None,
            &withdraw_withheld_authority_elgamal_keypair,
            alice_meta.elgamal_keypair.pubkey(),
            &new_decryptable_available_balance.into(),
            &[&withdraw_withheld_authority],
        )
        .await
        .unwrap();

    // withheld fees are withdrawn back to alice's account
    alice_meta
        .check_balances(
            &token,
            ConfidentialTokenAccountBalances {
                pending_balance_lo: 0,
                pending_balance_hi: 0,
                available_balance: 3,
                decryptable_available_balance: 3,
            },
        )
        .await;

    check_withheld_amount_in_mint(&token, &withdraw_withheld_authority_elgamal_keypair, 0).await;
}

#[cfg(feature = "zk-ops")]
#[tokio::test]
async fn confidential_transfer_withdraw_withheld_tokens_from_accounts() {
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

    let alice_meta =
        ConfidentialTokenAccountMeta::new(&token, &alice, &mint_authority, 100, decimals).await;
    let bob_meta =
        ConfidentialTokenAccountMeta::new(&token, &bob, &mint_authority, 0, decimals).await;

    let transfer_fee_parameters = TransferFee {
        epoch: 0.into(),
        maximum_fee: TEST_MAXIMUM_FEE.into(),
        transfer_fee_basis_points: TEST_FEE_BASIS_POINTS.into(),
    };

    // Test fee is 2.5% so the withheld fees should be 3
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
            transfer_fee_parameters.transfer_fee_basis_points.into(),
            transfer_fee_parameters.maximum_fee.into(),
            &[&alice],
        )
        .await
        .unwrap();

    let fee = transfer_fee_parameters.calculate_fee(100).unwrap();
    let new_decryptable_available_balance = alice_meta.aes_key.encrypt(fee);
    token
        .confidential_transfer_withdraw_withheld_tokens_from_accounts(
            &alice_meta.token_account,
            &withdraw_withheld_authority.pubkey(),
            None,
            None,
            &withdraw_withheld_authority_elgamal_keypair,
            alice_meta.elgamal_keypair.pubkey(),
            &new_decryptable_available_balance.into(),
            &[&bob_meta.token_account],
            &[&withdraw_withheld_authority],
        )
        .await
        .unwrap();

    alice_meta
        .check_balances(
            &token,
            ConfidentialTokenAccountBalances {
                pending_balance_lo: 0,
                pending_balance_hi: 0,
                available_balance: fee,
                decryptable_available_balance: fee,
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

    let state = token
        .get_account_info(&bob_meta.token_account)
        .await
        .unwrap();
    let extension = state
        .get_extension::<ConfidentialTransferFeeAmount>()
        .unwrap();
    assert_eq!(extension.withheld_amount, pod::ElGamalCiphertext::zeroed());
}

#[cfg(feature = "zk-ops")]
#[tokio::test]
async fn confidential_transfer_withdraw_withheld_tokens_from_mint_with_proof_context() {
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

    let alice_meta =
        ConfidentialTokenAccountMeta::new(&token, &alice, &mint_authority, 100, decimals).await;
    let bob_meta =
        ConfidentialTokenAccountMeta::new(&token, &bob, &mint_authority, 0, decimals).await;

    let transfer_fee_parameters = TransferFee {
        epoch: 0.into(),
        maximum_fee: TEST_MAXIMUM_FEE.into(),
        transfer_fee_basis_points: TEST_FEE_BASIS_POINTS.into(),
    };

    // Test fee is 2.5% so the withheld fees should be 3
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
            transfer_fee_parameters.transfer_fee_basis_points.into(),
            transfer_fee_parameters.maximum_fee.into(),
            &[&alice],
        )
        .await
        .unwrap();

    token
        .confidential_transfer_harvest_withheld_tokens_to_mint(&[&bob_meta.token_account])
        .await
        .unwrap();

    let context_state_account = Keypair::new();

    // create context state
    {
        let context_state_authority = Keypair::new();
        let space = size_of::<ProofContextState<CiphertextCiphertextEqualityProofContext>>();

        let instruction_type = ProofInstruction::VerifyCiphertextCiphertextEquality;

        let context_state_info = ContextStateInfo {
            context_state_account: &context_state_account.pubkey(),
            context_state_authority: &context_state_authority.pubkey(),
        };

        let state = token.get_mint_info().await.unwrap();
        let extension = state
            .get_extension::<ConfidentialTransferFeeConfig>()
            .unwrap();
        let withheld_tokens_info = WithheldTokensInfo::new(&extension.withheld_amount);

        let proof_data = withheld_tokens_info
            .generate_proof_data(
                &withdraw_withheld_authority_elgamal_keypair,
                alice_meta.elgamal_keypair.pubkey(),
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

        let recent_blockhash = ctx.get_new_latest_blockhash().await.unwrap();
        let tx = Transaction::new_signed_with_payer(
            &instructions,
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer, &context_state_account],
            recent_blockhash,
        );
        ctx.banks_client.process_transaction(tx).await.unwrap();
    }

    // calculate and encrypt fee to attach to the `WithdrawWithheldTokensFromMint`
    // instruction data
    let fee = transfer_fee_parameters.calculate_fee(100).unwrap();
    let new_decryptable_available_balance = alice_meta.aes_key.encrypt(fee);
    token
        .confidential_transfer_withdraw_withheld_tokens_from_mint(
            &alice_meta.token_account,
            &withdraw_withheld_authority.pubkey(),
            Some(&context_state_account.pubkey()),
            None,
            &withdraw_withheld_authority_elgamal_keypair,
            alice_meta.elgamal_keypair.pubkey(),
            &new_decryptable_available_balance.into(),
            &[&withdraw_withheld_authority],
        )
        .await
        .unwrap();

    // withheld fees are withdrawn back to alice's account
    alice_meta
        .check_balances(
            &token,
            ConfidentialTokenAccountBalances {
                pending_balance_lo: 0,
                pending_balance_hi: 0,
                available_balance: 3,
                decryptable_available_balance: 3,
            },
        )
        .await;

    check_withheld_amount_in_mint(&token, &withdraw_withheld_authority_elgamal_keypair, 0).await;
}

#[cfg(feature = "zk-ops")]
#[tokio::test]
async fn confidential_transfer_withdraw_withheld_tokens_from_accounts_with_proof_context() {
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

    let alice_meta =
        ConfidentialTokenAccountMeta::new(&token, &alice, &mint_authority, 100, decimals).await;
    let bob_meta =
        ConfidentialTokenAccountMeta::new(&token, &bob, &mint_authority, 0, decimals).await;

    let transfer_fee_parameters = TransferFee {
        epoch: 0.into(),
        maximum_fee: TEST_MAXIMUM_FEE.into(),
        transfer_fee_basis_points: TEST_FEE_BASIS_POINTS.into(),
    };

    // Test fee is 2.5% so the withheld fees should be 3
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
            transfer_fee_parameters.transfer_fee_basis_points.into(),
            transfer_fee_parameters.maximum_fee.into(),
            &[&alice],
        )
        .await
        .unwrap();

    let context_state_account = Keypair::new();

    // create context state
    {
        let context_state_authority = Keypair::new();
        let space = size_of::<ProofContextState<CiphertextCiphertextEqualityProofContext>>();

        let instruction_type = ProofInstruction::VerifyCiphertextCiphertextEquality;

        let context_state_info = ContextStateInfo {
            context_state_account: &context_state_account.pubkey(),
            context_state_authority: &context_state_authority.pubkey(),
        };

        let state = token
            .get_account_info(&bob_meta.token_account)
            .await
            .unwrap();
        let withheld_amount = state
            .get_extension::<ConfidentialTransferFeeAmount>()
            .unwrap()
            .withheld_amount;
        let withheld_tokens_info = WithheldTokensInfo::new(&withheld_amount);

        let proof_data = withheld_tokens_info
            .generate_proof_data(
                &withdraw_withheld_authority_elgamal_keypair,
                alice_meta.elgamal_keypair.pubkey(),
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

        let recent_blockhash = ctx.get_new_latest_blockhash().await.unwrap();
        let tx = Transaction::new_signed_with_payer(
            &instructions,
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer, &context_state_account],
            recent_blockhash,
        );
        ctx.banks_client.process_transaction(tx).await.unwrap();
    }

    let fee = transfer_fee_parameters.calculate_fee(100).unwrap();
    let new_decryptable_available_balance = alice_meta.aes_key.encrypt(fee);
    token
        .confidential_transfer_withdraw_withheld_tokens_from_accounts(
            &alice_meta.token_account,
            &withdraw_withheld_authority.pubkey(),
            Some(&context_state_account.pubkey()),
            None,
            &withdraw_withheld_authority_elgamal_keypair,
            alice_meta.elgamal_keypair.pubkey(),
            &new_decryptable_available_balance.into(),
            &[&bob_meta.token_account],
            &[&withdraw_withheld_authority],
        )
        .await
        .unwrap();

    alice_meta
        .check_balances(
            &token,
            ConfidentialTokenAccountBalances {
                pending_balance_lo: 0,
                pending_balance_hi: 0,
                available_balance: fee,
                decryptable_available_balance: fee,
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

    let state = token
        .get_account_info(&bob_meta.token_account)
        .await
        .unwrap();
    let extension = state
        .get_extension::<ConfidentialTransferFeeAmount>()
        .unwrap();
    assert_eq!(extension.withheld_amount, pod::ElGamalCiphertext::zeroed());
}

#[cfg(feature = "zk-ops")]
#[tokio::test]
async fn confidential_transfer_harvest_withheld_tokens_to_mint() {
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

    let alice_meta =
        ConfidentialTokenAccountMeta::new(&token, &alice, &mint_authority, 100, decimals).await;
    let bob_meta =
        ConfidentialTokenAccountMeta::new(&token, &bob, &mint_authority, 0, decimals).await;

    let transfer_fee_parameters = TransferFee {
        epoch: 0.into(),
        maximum_fee: TEST_MAXIMUM_FEE.into(),
        transfer_fee_basis_points: TEST_FEE_BASIS_POINTS.into(),
    };

    // there are no withheld fees in bob's account yet, but try harvesting
    token
        .confidential_transfer_harvest_withheld_tokens_to_mint(&[&bob_meta.token_account])
        .await
        .unwrap();

    // Test fee is 2.5% so the withheld fees should be 3
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
            transfer_fee_parameters.transfer_fee_basis_points.into(),
            transfer_fee_parameters.maximum_fee.into(),
            &[&alice],
        )
        .await
        .unwrap();

    // disable harvest withheld tokens to mint
    token
        .confidential_transfer_disable_harvest_to_mint(
            &confidential_transfer_fee_authority.pubkey(),
            &[&confidential_transfer_fee_authority],
        )
        .await
        .unwrap();

    let err = token
        .confidential_transfer_harvest_withheld_tokens_to_mint(&[&bob_meta.token_account])
        .await
        .unwrap_err();

    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::HarvestToMintDisabled as u32),
            )
        )))
    );

    // enable harvest withheld tokens to mint
    token
        .confidential_transfer_enable_harvest_to_mint(
            &confidential_transfer_fee_authority.pubkey(),
            &[&confidential_transfer_fee_authority],
        )
        .await
        .unwrap();

    // Refresh the blockhash since we're doing the same thing twice in a row
    token.get_new_latest_blockhash().await.unwrap();
    token
        .confidential_transfer_harvest_withheld_tokens_to_mint(&[&bob_meta.token_account])
        .await
        .unwrap();

    let state = token
        .get_account_info(&bob_meta.token_account)
        .await
        .unwrap();
    let extension = state
        .get_extension::<ConfidentialTransferFeeAmount>()
        .unwrap();
    assert_eq!(extension.withheld_amount, pod::ElGamalCiphertext::zeroed());

    // calculate and encrypt fee to attach to the `WithdrawWithheldTokensFromMint`
    // instruction data
    let fee = transfer_fee_parameters.calculate_fee(100).unwrap();

    check_withheld_amount_in_mint(&token, &withdraw_withheld_authority_elgamal_keypair, fee).await;
}
