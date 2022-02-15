// Mark this test as BPF-only due to current `ProgramTest` limitations when CPIing into the system program
#![cfg(feature = "test-bpf")]

use {
    solana_program::{
        feature::{self, Feature},
        program_option::COption,
        system_program,
    },
    solana_program_test::*,
    solana_sdk::{
        signature::{Keypair, Signer},
        transaction::Transaction,
    },
    spl_feature_proposal::{instruction::*, state::*, *},
};

fn program_test() -> ProgramTest {
    ProgramTest::new(
        "spl_feature_proposal",
        id(),
        processor!(processor::process_instruction),
    )
}

#[tokio::test]
async fn test_basic() {
    let feature_proposal = Keypair::new();

    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;

    let feature_id_address = get_feature_id_address(&feature_proposal.pubkey());
    let mint_address = get_mint_address(&feature_proposal.pubkey());
    let distributor_token_address = get_distributor_token_address(&feature_proposal.pubkey());
    let acceptance_token_address = get_acceptance_token_address(&feature_proposal.pubkey());

    // Create a new feature proposal
    let mut transaction = Transaction::new_with_payer(
        &[propose(
            &payer.pubkey(),
            &feature_proposal.pubkey(),
            42,
            AcceptanceCriteria {
                tokens_required: 42,
                deadline: i64::MAX,
            },
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &feature_proposal], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Confirm feature id account is now funded and allocated, but not assigned
    let feature_id_account = banks_client
        .get_account(feature_id_address)
        .await
        .expect("success")
        .expect("some account");
    assert_eq!(feature_id_account.owner, system_program::id());
    assert_eq!(feature_id_account.data.len(), Feature::size_of());

    // Confirm mint account state
    let mint = banks_client
        .get_packed_account_data::<spl_token::state::Mint>(mint_address)
        .await
        .unwrap();
    assert_eq!(mint.supply, 42);
    assert_eq!(mint.decimals, 9);
    assert!(mint.freeze_authority.is_none());
    assert_eq!(mint.mint_authority, COption::Some(mint_address));

    // Confirm distributor token account state
    let distributor_token = banks_client
        .get_packed_account_data::<spl_token::state::Account>(distributor_token_address)
        .await
        .unwrap();
    assert_eq!(distributor_token.amount, 42);
    assert_eq!(distributor_token.mint, mint_address);
    assert_eq!(distributor_token.owner, feature_proposal.pubkey());
    assert!(distributor_token.close_authority.is_none());

    // Confirm acceptance token account state
    let acceptance_token = banks_client
        .get_packed_account_data::<spl_token::state::Account>(acceptance_token_address)
        .await
        .unwrap();
    assert_eq!(acceptance_token.amount, 0);
    assert_eq!(acceptance_token.mint, mint_address);
    assert_eq!(acceptance_token.owner, id());
    assert_eq!(
        acceptance_token.close_authority,
        COption::Some(feature_proposal.pubkey())
    );

    // Tally #1: Does nothing because the acceptance criteria has not been met
    let mut transaction =
        Transaction::new_with_payer(&[tally(&feature_proposal.pubkey())], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Confirm feature id account is not yet assigned
    let feature_id_account = banks_client
        .get_account(feature_id_address)
        .await
        .expect("success")
        .expect("some account");
    assert_eq!(feature_id_account.owner, system_program::id());

    assert!(matches!(
        banks_client
            .get_packed_account_data::<FeatureProposal>(feature_proposal.pubkey())
            .await,
        Ok(FeatureProposal::Pending(_))
    ));

    // Transfer tokens to the acceptance account
    let mut transaction = Transaction::new_with_payer(
        &[spl_token::instruction::transfer(
            &spl_token::id(),
            &distributor_token_address,
            &acceptance_token_address,
            &feature_proposal.pubkey(),
            &[],
            42,
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &feature_proposal], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Fetch a new blockhash to avoid the second Tally transaction having the same signature as the
    // first Tally transaction
    let recent_blockhash = banks_client
        .get_new_latest_blockhash(&recent_blockhash)
        .await
        .unwrap();

    // Tally #2: the acceptance criteria is now met
    let mut transaction =
        Transaction::new_with_payer(&[tally(&feature_proposal.pubkey())], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Confirm feature id account is now assigned
    let feature_id_account = banks_client
        .get_account(feature_id_address)
        .await
        .expect("success")
        .expect("some account");
    assert_eq!(feature_id_account.owner, feature::id());

    // Confirm feature proposal account state
    assert!(matches!(
        banks_client
            .get_packed_account_data::<FeatureProposal>(feature_proposal.pubkey())
            .await,
        Ok(FeatureProposal::Accepted {
            tokens_upon_acceptance: 42
        })
    ));
}

#[tokio::test]
async fn test_expired() {
    let feature_proposal = Keypair::new();

    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;

    // Create a new feature proposal
    let mut transaction = Transaction::new_with_payer(
        &[propose(
            &payer.pubkey(),
            &feature_proposal.pubkey(),
            42,
            AcceptanceCriteria {
                tokens_required: 42,
                deadline: 0, // <=== Already expired
            },
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &feature_proposal], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    assert!(matches!(
        banks_client
            .get_packed_account_data::<FeatureProposal>(feature_proposal.pubkey())
            .await,
        Ok(FeatureProposal::Pending(_))
    ));

    // Tally will cause the proposal to expire
    let mut transaction =
        Transaction::new_with_payer(&[tally(&feature_proposal.pubkey())], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    assert!(matches!(
        banks_client
            .get_packed_account_data::<FeatureProposal>(feature_proposal.pubkey())
            .await,
        Ok(FeatureProposal::Expired)
    ));
}
