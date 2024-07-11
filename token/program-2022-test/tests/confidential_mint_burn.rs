#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{ConfidentialTokenAccountMeta, TestContext, TokenContext},
    solana_program_test::tokio,
    solana_sdk::{pubkey::Pubkey, signature::Signer, signer::keypair::Keypair, signers::Signers},
    spl_token_2022::{
        extension::{
            confidential_mint_burn::{
                proof_generation::{generate_burn_proofs, generate_mint_proofs},
                ConfidentialMintBurn,
            },
            confidential_transfer::{
                account_info::TransferAccountInfo, instruction::TransferSplitContextStateAccounts,
                ConfidentialTransferAccount,
            },
            BaseStateWithExtensions,
        },
        proof::ProofLocation,
        solana_zk_token_sdk::{
            encryption::elgamal::*, zk_token_elgamal::pod::ElGamalPubkey as PodElGamalPubkey,
        },
    },
    spl_token_client::{
        client::ProgramBanksClientProcessTransaction,
        token::{ExtensionInitializationParams, Token},
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

    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![
            ExtensionInitializationParams::ConfidentialTransferMint {
                authority: Some(authority.pubkey()),
                auto_approve_new_accounts: true,
                auditor_elgamal_pubkey: Some(auditor_elgamal_pubkey),
            },
            ExtensionInitializationParams::ConfidentialMintBurnMint {
                authority: authority.pubkey(),
                confidential_supply_pubkey: Some(auditor_elgamal_pubkey),
            },
        ])
        .await
        .unwrap();

    let TokenContext { token, alice, .. } = context.token_context.unwrap();
    let alice_meta = ConfidentialTokenAccountMeta::new(&token, &alice, None, false, false).await;
    //let alice_elgamal_pubkey = (*alice_meta.elgamal_keypair.pubkey()).into();

    mint_tokens(
        &token,
        &alice_meta.token_account,
        &authority.pubkey(),
        MINT_AMOUNT,
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
            .confidential_supply(&auditor_elgamal_keypair,)
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

    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![
            ExtensionInitializationParams::ConfidentialTransferMint {
                authority: Some(authority.pubkey()),
                auto_approve_new_accounts: true,
                auditor_elgamal_pubkey: Some(auditor_elgamal_pubkey),
            },
            ExtensionInitializationParams::ConfidentialMintBurnMint {
                authority: authority.pubkey(),
                confidential_supply_pubkey: Some(auditor_elgamal_pubkey),
            },
        ])
        .await
        .unwrap();

    let TokenContext { token, alice, .. } = context.token_context.unwrap();
    let alice_meta = ConfidentialTokenAccountMeta::new(&token, &alice, None, false, false).await;
    //let alice_elgamal_pubkey = (*alice_meta.elgamal_keypair.pubkey()).into();

    mint_tokens(
        &token,
        &alice_meta.token_account,
        &authority.pubkey(),
        MINT_AMOUNT,
        &[&authority],
    )
    .await;

    assert_eq!(
        token
            .confidential_supply(&auditor_elgamal_keypair,)
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
    let equality_proof_pubkey = equality_proof_context_state_account.pubkey();
    let ciphertext_validity_proof_context_state_account = Keypair::new();
    let ciphertext_validity_proof_pubkey = ciphertext_validity_proof_context_state_account.pubkey();
    let range_proof_context_state_account = Keypair::new();
    let range_proof_pubkey = range_proof_context_state_account.pubkey();

    let context_state_accounts = TransferSplitContextStateAccounts {
        equality_proof: &equality_proof_pubkey,
        ciphertext_validity_proof: &ciphertext_validity_proof_pubkey,
        range_proof: &range_proof_pubkey,
        authority: &context_state_authority.pubkey(),
        no_op_on_uninitialized_split_context_state: false,
        close_split_context_state_accounts: None,
    };

    let state = token
        .get_account_info(&alice_meta.token_account)
        .await
        .unwrap();
    let extension = state
        .get_extension::<ConfidentialTransferAccount>()
        .unwrap();
    let transfer_account_info = TransferAccountInfo::new(extension);

    let (equality_proof_data, ciphertext_validity_proof_data, range_proof_data, pedersen_openings) =
        generate_burn_proofs(
            &transfer_account_info.available_balance.try_into().unwrap(),
            &transfer_account_info
                .decryptable_available_balance
                .try_into()
                .unwrap(),
            BURN_AMOUNT,
            &alice_meta.elgamal_keypair,
            &alice_meta.aes_key,
            &auditor_elgamal_pubkey,
            &supply_elgamal_pubkey,
        )
        .unwrap();

    // setup proofs
    token
        .create_range_proof_context_state_for_transfer(
            context_state_accounts,
            &range_proof_data,
            &range_proof_context_state_account,
        )
        .await
        .unwrap();
    token
        .create_equality_proof_context_state_for_transfer(
            context_state_accounts,
            &equality_proof_data,
            &equality_proof_context_state_account,
        )
        .await
        .unwrap();
    token
        .create_batched_grouped_3_handles_ciphertext_validity_proof_context_state(
            context_state_accounts.ciphertext_validity_proof,
            context_state_accounts.authority,
            &ciphertext_validity_proof_data,
            &ciphertext_validity_proof_context_state_account,
        )
        .await
        .unwrap();

    // do the burn
    token
        .confidential_burn(
            &alice_meta.token_account,
            &alice.pubkey(),
            context_state_accounts,
            BURN_AMOUNT,
            auditor_elgamal_pubkey,
            supply_elgamal_pubkey,
            &alice_meta.aes_key,
            &[&alice],
            &pedersen_openings,
        )
        .await
        .unwrap();

    // close context state accounts
    let context_state_authority_pubkey = context_state_authority.pubkey();
    let close_context_state_signers = &[context_state_authority];
    token
        .confidential_transfer_close_context_state(
            &equality_proof_pubkey,
            &context_state_authority_pubkey,
            &context_state_authority_pubkey,
            close_context_state_signers,
        )
        .await
        .unwrap();
    token
        .confidential_transfer_close_context_state(
            &ciphertext_validity_proof_pubkey,
            &context_state_authority_pubkey,
            &context_state_authority_pubkey,
            close_context_state_signers,
        )
        .await
        .unwrap();
    token
        .confidential_transfer_close_context_state(
            &range_proof_pubkey,
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
            .confidential_supply(&auditor_elgamal_keypair,)
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

    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![
            ExtensionInitializationParams::ConfidentialTransferMint {
                authority: Some(authority.pubkey()),
                auto_approve_new_accounts: true,
                auditor_elgamal_pubkey: Some(auditor_elgamal_pubkey),
            },
            ExtensionInitializationParams::ConfidentialMintBurnMint {
                authority: authority.pubkey(),
                confidential_supply_pubkey: Some(auditor_elgamal_pubkey),
            },
        ])
        .await
        .unwrap();

    let TokenContext { token, alice, .. } = context.token_context.unwrap();
    let alice_meta = ConfidentialTokenAccountMeta::new(&token, &alice, None, false, false).await;
    //let alice_elgamal_pubkey = (*alice_meta.elgamal_keypair.pubkey()).into();

    mint_tokens(
        &token,
        &alice_meta.token_account,
        &authority.pubkey(),
        MINT_AMOUNT,
        &[&authority],
    )
    .await;

    assert_eq!(
        token
            .confidential_supply(&auditor_elgamal_keypair,)
            .await
            .unwrap(),
        MINT_AMOUNT
    );

    let new_supply_elgamal_keypair = ElGamalKeypair::new_rand();
    token
        .rotate_supply_elgamal(
            &authority.pubkey(),
            &auditor_elgamal_keypair,
            &new_supply_elgamal_keypair,
            &[authority],
        )
        .await
        .unwrap();

    assert_eq!(
        token
            .confidential_supply(&new_supply_elgamal_keypair)
            .await
            .unwrap(),
        MINT_AMOUNT
    );

    let mint = token.get_mint_info().await.unwrap();
    let conf_mint_burn_ext = mint.get_extension::<ConfidentialMintBurn>().unwrap();

    assert_eq!(
        conf_mint_burn_ext.supply_elgamal_pubkey,
        Some(Into::<PodElGamalPubkey>::into(
            *new_supply_elgamal_keypair.pubkey(),
        ))
        .try_into()
        .unwrap(),
    );
}

async fn mint_tokens(
    token: &Token<ProgramBanksClientProcessTransaction>,
    token_account: &Pubkey,
    authority: &Pubkey,
    mint_amount: u64,
    bulk_signers: &impl Signers,
) {
    let context_state_authority = Keypair::new();
    let range_proof_context_state_account = Keypair::new();
    let range_proof_context_pubkey = range_proof_context_state_account.pubkey();
    let ciphertext_validity_proof_context_state_account = Keypair::new();
    let ciphertext_validity_proof_context_pubkey =
        ciphertext_validity_proof_context_state_account.pubkey();

    let mint_to_elgamal_pubkey = token.account_elgamal_pubkey(token_account).await.unwrap();
    let auditor_elgamal_pubkey = token.auditor_elgamal_pubkey().await.unwrap();
    let supply_elgamal_pubkey = token.supply_elgamal_pubkey().await.unwrap();

    let (range_proof, ciphertext_validity_proof, pedersen_openings) = generate_mint_proofs(
        mint_amount,
        &mint_to_elgamal_pubkey,
        &auditor_elgamal_pubkey,
        &supply_elgamal_pubkey,
    )
    .unwrap();

    token
        .create_batched_u64_range_proof_context_state(
            &range_proof_context_pubkey,
            &context_state_authority.pubkey(),
            &range_proof,
            &range_proof_context_state_account,
        )
        .await
        .unwrap();
    token
        .create_batched_grouped_3_handles_ciphertext_validity_proof_context_state(
            &ciphertext_validity_proof_context_pubkey,
            &context_state_authority.pubkey(),
            &ciphertext_validity_proof,
            &ciphertext_validity_proof_context_state_account,
        )
        .await
        .unwrap();

    let range_proof_location = ProofLocation::ContextStateAccount(&range_proof_context_pubkey);
    let ciphertext_validity_proof_location =
        ProofLocation::ContextStateAccount(&ciphertext_validity_proof_context_pubkey);

    println!("MINT: account is {token_account} with owner {authority}");
    token
        .confidential_mint(
            token_account,
            authority,
            mint_amount,
            auditor_elgamal_pubkey,
            supply_elgamal_pubkey,
            range_proof_location,
            ciphertext_validity_proof_location,
            &pedersen_openings,
            bulk_signers,
        )
        .await
        .map_err(|e| println!("{}", e))
        .unwrap();

    let close_context_auth = context_state_authority.pubkey();
    let close_context_state_signers = &[context_state_authority];
    token
        .confidential_transfer_close_context_state(
            &range_proof_context_pubkey,
            &close_context_auth,
            &close_context_auth,
            close_context_state_signers,
        )
        .await
        .unwrap();
    token
        .confidential_transfer_close_context_state(
            &ciphertext_validity_proof_context_pubkey,
            &close_context_auth,
            &close_context_auth,
            close_context_state_signers,
        )
        .await
        .unwrap();
}
