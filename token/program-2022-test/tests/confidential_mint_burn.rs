//#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{ConfidentialTokenAccountMeta, TestContext, TokenContext},
    solana_program_test::tokio,
    solana_sdk::{pubkey::Pubkey, signature::Signer, signer::keypair::Keypair, signers::Signers},
    spl_token_2022::{
        extension::{
            confidential_mint_burn::{account_info::SupplyAccountInfo, ConfidentialMintBurn},
            confidential_transfer::{
                account_info::TransferAccountInfo, ConfidentialTransferAccount,
            },
            BaseStateWithExtensions,
        },
        solana_zk_sdk::encryption::{
            auth_encryption::AeKey, elgamal::*, pod::elgamal::PodElGamalPubkey,
        },
    },
    spl_token_client::{
        client::ProgramBanksClientProcessTransaction,
        token::{ExtensionInitializationParams, ProofAccount, ProofAccountWithCiphertext, Token},
    },
    spl_token_confidential_transfer_proof_generation::{
        burn::burn_split_proof_data, mint::mint_split_proof_data,
    },
    std::convert::TryInto,
};

const MINT_AMOUNT: u64 = 42;
const BURN_AMOUNT: u64 = 12;

#[tokio::test]
async fn test_confidential_mint() {
    let authority = Keypair::new();
    let auditor_elgamal_keypair = ElGamalKeypair::new_rand();
    let auditor_elgamal_pubkey = (*auditor_elgamal_keypair.pubkey()).into();
    let supply_aes_key = AeKey::new_rand();
    let mint_account = Keypair::new();

    let mut context = TestContext::new().await;
    context
        .init_token_with_mint_keypair_and_freeze_authority_and_mint_authority(
            mint_account,
            vec![
                ExtensionInitializationParams::ConfidentialTransferMint {
                    authority: Some(authority.pubkey()),
                    auto_approve_new_accounts: true,
                    auditor_elgamal_pubkey: Some(auditor_elgamal_pubkey),
                },
                ExtensionInitializationParams::ConfidentialMintBurnMint {
                    confidential_supply_pubkey: auditor_elgamal_pubkey,
                    decryptable_supply: supply_aes_key.encrypt(0).into(),
                },
            ],
            None,
            // hacky but we have to clone somehow
            Keypair::from_bytes(&authority.to_bytes()).unwrap(),
        )
        .await
        .unwrap();

    let TokenContext { token, alice, .. } = context.token_context.unwrap();
    let alice_meta = ConfidentialTokenAccountMeta::new(&token, &alice, None, false, false).await;

    mint_tokens(
        &token,
        &alice_meta.token_account,
        &authority.pubkey(),
        MINT_AMOUNT,
        &auditor_elgamal_keypair,
        &supply_aes_key,
        &[&authority],
    )
    .await;

    assert_eq!(
        token
            .confidential_balance(
                &alice_meta.token_account,
                &alice_meta.elgamal_keypair,
                &alice_meta.aes_key
            )
            .await
            .unwrap()
            .1,
        MINT_AMOUNT
    );

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

    assert_eq!(
        token
            .confidential_balance(
                &alice_meta.token_account,
                &alice_meta.elgamal_keypair,
                &alice_meta.aes_key
            )
            .await
            .unwrap()
            .0,
        MINT_AMOUNT
    );

    assert_eq!(
        token
            .confidential_supply(&auditor_elgamal_keypair, &supply_aes_key)
            .await
            .unwrap(),
        MINT_AMOUNT
    );
}

#[tokio::test]
async fn test_confidential_burn() {
    let authority = Keypair::new();
    let auditor_elgamal_keypair = ElGamalKeypair::new_rand();
    let auditor_elgamal_pubkey = (*auditor_elgamal_keypair.pubkey()).into();
    let supply_aes_key = AeKey::new_rand();
    let mint_account = Keypair::new();

    let mut context = TestContext::new().await;
    context
        .init_token_with_mint_keypair_and_freeze_authority_and_mint_authority(
            mint_account,
            vec![
                ExtensionInitializationParams::ConfidentialTransferMint {
                    authority: Some(authority.pubkey()),
                    auto_approve_new_accounts: true,
                    auditor_elgamal_pubkey: Some(auditor_elgamal_pubkey),
                },
                ExtensionInitializationParams::ConfidentialMintBurnMint {
                    confidential_supply_pubkey: auditor_elgamal_pubkey,
                    decryptable_supply: supply_aes_key.encrypt(0).into(),
                },
            ],
            None,
            Keypair::from_bytes(&authority.to_bytes()).unwrap(),
        )
        .await
        .unwrap();

    let TokenContext { token, alice, .. } = context.token_context.unwrap();
    let alice_meta = ConfidentialTokenAccountMeta::new(&token, &alice, None, false, false).await;

    mint_tokens(
        &token,
        &alice_meta.token_account,
        &authority.pubkey(),
        MINT_AMOUNT,
        &auditor_elgamal_keypair,
        &supply_aes_key,
        &[&authority],
    )
    .await;

    assert_eq!(
        token
            .confidential_supply(&auditor_elgamal_keypair, &supply_aes_key)
            .await
            .unwrap(),
        MINT_AMOUNT
    );

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

    let context_state_authority = Keypair::new();
    let auditor_elgamal_pubkey = token.auditor_elgamal_pubkey().await.unwrap();
    let supply_elgamal_pubkey = token.supply_elgamal_pubkey().await.unwrap();

    let equality_proof_context_state_account = Keypair::new();
    let equality_proof_context_pubkey = equality_proof_context_state_account.pubkey();
    let ciphertext_validity_proof_context_state_account = Keypair::new();
    let ciphertext_validity_proof_context_pubkey =
        ciphertext_validity_proof_context_state_account.pubkey();
    let range_proof_context_state_account = Keypair::new();
    let range_proof_context_pubkey = range_proof_context_state_account.pubkey();

    let state = token
        .get_account_info(&alice_meta.token_account)
        .await
        .unwrap();
    let extension = state
        .get_extension::<ConfidentialTransferAccount>()
        .unwrap();
    let transfer_account_info = TransferAccountInfo::new(extension);

    let proof_data = burn_split_proof_data(
        &transfer_account_info.available_balance.try_into().unwrap(),
        &transfer_account_info
            .decryptable_available_balance
            .try_into()
            .unwrap(),
        BURN_AMOUNT,
        &alice_meta.elgamal_keypair,
        &alice_meta.aes_key,
        &auditor_elgamal_pubkey.unwrap_or_default(),
        &supply_elgamal_pubkey.unwrap_or_default(),
    )
    .unwrap();

    let range_proof_signer = &[&range_proof_context_state_account];
    let equality_proof_signer = &[&equality_proof_context_state_account];
    let ciphertext_validity_proof_signer = &[&ciphertext_validity_proof_context_state_account];
    let context_state_auth_pubkey = context_state_authority.pubkey();
    // setup proofs
    token
        .confidential_transfer_create_context_state_account(
            &equality_proof_context_pubkey,
            &context_state_auth_pubkey,
            &proof_data.equality_proof_data,
            true,
            equality_proof_signer,
        )
        .await
        .unwrap();
    token
        .confidential_transfer_create_context_state_account(
            &ciphertext_validity_proof_context_pubkey,
            &context_state_auth_pubkey,
            &proof_data
                .ciphertext_validity_proof_data_with_ciphertext
                .proof_data,
            false,
            ciphertext_validity_proof_signer,
        )
        .await
        .unwrap();
    token
        .confidential_transfer_create_context_state_account(
            &range_proof_context_pubkey,
            &context_state_auth_pubkey,
            &proof_data.range_proof_data,
            true,
            range_proof_signer,
        )
        .await
        .unwrap();

    let equality_proof_location = ProofAccount::ContextAccount(equality_proof_context_pubkey);
    let ciphertext_validity_proof_location =
        ProofAccount::ContextAccount(ciphertext_validity_proof_context_pubkey);
    let ciphertext_validity_proof_location = ProofAccountWithCiphertext {
        proof_account: ciphertext_validity_proof_location,
        ciphertext_lo: proof_data
            .ciphertext_validity_proof_data_with_ciphertext
            .ciphertext_lo,
        ciphertext_hi: proof_data
            .ciphertext_validity_proof_data_with_ciphertext
            .ciphertext_hi,
    };
    let range_proof_location = ProofAccount::ContextAccount(range_proof_context_pubkey);

    // do the burn
    token
        .confidential_burn(
            &alice_meta.token_account,
            &alice.pubkey(),
            Some(&equality_proof_location),
            Some(&ciphertext_validity_proof_location),
            Some(&range_proof_location),
            BURN_AMOUNT,
            supply_elgamal_pubkey,
            &alice_meta.aes_key,
            &[&alice],
        )
        .await
        .unwrap();

    // close context state accounts
    let context_state_authority_pubkey = context_state_authority.pubkey();
    let close_context_state_signers = &[context_state_authority];
    token
        .confidential_transfer_close_context_state_account(
            &equality_proof_context_pubkey,
            &context_state_authority_pubkey,
            &context_state_authority_pubkey,
            close_context_state_signers,
        )
        .await
        .unwrap();
    token
        .confidential_transfer_close_context_state_account(
            &ciphertext_validity_proof_context_pubkey,
            &context_state_authority_pubkey,
            &context_state_authority_pubkey,
            close_context_state_signers,
        )
        .await
        .unwrap();
    token
        .confidential_transfer_close_context_state_account(
            &range_proof_context_pubkey,
            &context_state_authority_pubkey,
            &context_state_authority_pubkey,
            close_context_state_signers,
        )
        .await
        .unwrap();

    assert_eq!(
        token
            .confidential_balance(
                &alice_meta.token_account,
                &alice_meta.elgamal_keypair,
                &alice_meta.aes_key
            )
            .await
            .unwrap()
            .0,
        MINT_AMOUNT - BURN_AMOUNT,
    );

    assert_eq!(
        token
            .confidential_supply(&auditor_elgamal_keypair, &supply_aes_key)
            .await
            .unwrap(),
        MINT_AMOUNT - BURN_AMOUNT,
    );
}

#[tokio::test]
async fn test_rotate_supply_elgamal() {
    let authority = Keypair::new();
    let auditor_elgamal_keypair = ElGamalKeypair::new_rand();
    let auditor_elgamal_pubkey = (*auditor_elgamal_keypair.pubkey()).into();
    let supply_aes_key = AeKey::new_rand();
    let mint_account = Keypair::new();

    let mut context = TestContext::new().await;
    context
        .init_token_with_mint_keypair_and_freeze_authority_and_mint_authority(
            mint_account,
            vec![
                ExtensionInitializationParams::ConfidentialTransferMint {
                    authority: Some(authority.pubkey()),
                    auto_approve_new_accounts: true,
                    auditor_elgamal_pubkey: Some(auditor_elgamal_pubkey),
                },
                ExtensionInitializationParams::ConfidentialMintBurnMint {
                    confidential_supply_pubkey: auditor_elgamal_pubkey,
                    decryptable_supply: supply_aes_key.encrypt(0).into(),
                },
            ],
            None,
            Keypair::from_bytes(&authority.to_bytes()).unwrap(),
        )
        .await
        .unwrap();

    let TokenContext { token, alice, .. } = context.token_context.unwrap();
    let alice_meta = ConfidentialTokenAccountMeta::new(&token, &alice, None, false, false).await;

    mint_tokens(
        &token,
        &alice_meta.token_account,
        &authority.pubkey(),
        MINT_AMOUNT,
        &auditor_elgamal_keypair,
        &supply_aes_key,
        &[&authority],
    )
    .await;

    assert_eq!(
        token
            .confidential_supply(&auditor_elgamal_keypair, &supply_aes_key)
            .await
            .unwrap(),
        MINT_AMOUNT
    );

    let new_supply_elgamal_keypair = ElGamalKeypair::new_rand();

    let mint = token.get_mint_info().await.unwrap();
    let mint_burn_extension = mint.get_extension::<ConfidentialMintBurn>().unwrap();
    let supply_account_info = SupplyAccountInfo::new(mint_burn_extension);
    let proof_data = supply_account_info
        .generate_rotate_supply_elgamal_pubkey_proof(
            &supply_aes_key,
            &auditor_elgamal_keypair,
            &new_supply_elgamal_keypair,
        )
        .unwrap();

    token
        .rotate_supply_elgamal(
            &authority.pubkey(),
            &new_supply_elgamal_keypair,
            &[authority],
            proof_data,
        )
        .await
        .unwrap();

    assert_eq!(
        token
            .confidential_supply(&new_supply_elgamal_keypair, &supply_aes_key)
            .await
            .unwrap(),
        MINT_AMOUNT
    );

    let mint = token.get_mint_info().await.unwrap();
    let mint_burn_extension = mint.get_extension::<ConfidentialMintBurn>().unwrap();

    assert_eq!(
        mint_burn_extension.supply_elgamal_pubkey,
        Into::<PodElGamalPubkey>::into(*new_supply_elgamal_keypair.pubkey(),),
    );
}

async fn mint_tokens(
    token: &Token<ProgramBanksClientProcessTransaction>,
    token_account: &Pubkey,
    authority: &Pubkey,
    mint_amount: u64,
    supply_elgamal_keypair: &ElGamalKeypair,
    supply_aes_key: &AeKey,
    bulk_signers: &impl Signers,
) {
    let context_state_auth = Keypair::new();
    let equality_proof_context_state_account = Keypair::new();
    let equality_proof_context_pubkey = equality_proof_context_state_account.pubkey();
    let ciphertext_validity_proof_context_state_account = Keypair::new();
    let ciphertext_validity_proof_context_pubkey =
        ciphertext_validity_proof_context_state_account.pubkey();
    let range_proof_context_state_account = Keypair::new();
    let range_proof_context_pubkey = range_proof_context_state_account.pubkey();

    let mint_to_elgamal_pubkey = token.account_elgamal_pubkey(token_account).await.unwrap();
    let auditor_elgamal_pubkey = token.auditor_elgamal_pubkey().await.unwrap();
    let supply_elgamal_pubkey = token.supply_elgamal_pubkey().await.unwrap();

    let mint = token.get_mint_info().await.unwrap();
    let mint_burn_extension = mint.get_extension::<ConfidentialMintBurn>().unwrap();
    let supply_account_info = SupplyAccountInfo::new(mint_burn_extension);

    let proof_data = mint_split_proof_data(
        &mint_burn_extension.confidential_supply.try_into().unwrap(),
        mint_amount,
        supply_account_info
            .decrypt_current_supply(supply_aes_key, supply_elgamal_keypair)
            .unwrap(),
        supply_elgamal_keypair,
        supply_aes_key,
        &mint_to_elgamal_pubkey,
        &auditor_elgamal_pubkey.unwrap_or_default(),
    )
    .unwrap();

    let equality_proof_signer = &[&equality_proof_context_state_account];
    let ciphertext_validity_proof_signer = &[&ciphertext_validity_proof_context_state_account];
    let range_proof_signer = &[&range_proof_context_state_account];

    token
        .confidential_transfer_create_context_state_account(
            &equality_proof_context_pubkey,
            &context_state_auth.pubkey(),
            &proof_data.equality_proof_data,
            false,
            equality_proof_signer,
        )
        .await
        .unwrap();
    token
        .confidential_transfer_create_context_state_account(
            &ciphertext_validity_proof_context_pubkey,
            &context_state_auth.pubkey(),
            &proof_data
                .ciphertext_validity_proof_data_with_ciphertext
                .proof_data,
            false,
            ciphertext_validity_proof_signer,
        )
        .await
        .unwrap();
    token
        .confidential_transfer_create_context_state_account(
            &range_proof_context_pubkey,
            &context_state_auth.pubkey(),
            &proof_data.range_proof_data,
            false,
            range_proof_signer,
        )
        .await
        .unwrap();

    let equality_proof_location = ProofAccount::ContextAccount(equality_proof_context_pubkey);
    let ciphertext_validity_proof_location =
        ProofAccount::ContextAccount(ciphertext_validity_proof_context_pubkey);
    let ciphertext_validity_proof_location = ProofAccountWithCiphertext {
        proof_account: ciphertext_validity_proof_location,
        ciphertext_lo: proof_data
            .ciphertext_validity_proof_data_with_ciphertext
            .ciphertext_lo,
        ciphertext_hi: proof_data
            .ciphertext_validity_proof_data_with_ciphertext
            .ciphertext_hi,
    };
    let range_proof_location = ProofAccount::ContextAccount(range_proof_context_pubkey);

    println!(
        "TOKEN: {}, ata: {token_account}, auth: {authority}",
        token.get_address()
    );
    token
        .confidential_mint(
            token_account,
            authority,
            supply_elgamal_pubkey,
            Some(&equality_proof_location),
            Some(&ciphertext_validity_proof_location),
            Some(&range_proof_location),
            proof_data.new_decryptable_supply,
            bulk_signers,
        )
        .await
        .unwrap();

    let close_context_auth = context_state_auth.pubkey();
    let close_context_state_signers = &[context_state_auth];
    token
        .confidential_transfer_close_context_state_account(
            &range_proof_context_pubkey,
            &close_context_auth,
            &close_context_auth,
            close_context_state_signers,
        )
        .await
        .unwrap();
    token
        .confidential_transfer_close_context_state_account(
            &ciphertext_validity_proof_context_pubkey,
            &close_context_auth,
            &close_context_auth,
            close_context_state_signers,
        )
        .await
        .unwrap();
}
