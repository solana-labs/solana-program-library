// Mark this test as BPF-only due to current `ProgramTest` limitations when CPIing into the system program
#![cfg(feature = "test-bpf")]

use futures::{Future, FutureExt};
use solana_program::{
    feature::{self, Feature},
    program_pack::Pack,
    pubkey::Pubkey,
    system_program,
};
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_feature_proposal::{instruction::*, state::*, *};
use std::io;

fn program_test() -> ProgramTest {
    let mut pc = ProgramTest::new(
        "spl_feature_proposal",
        id(),
        processor!(processor::process_instruction),
    );

    // Add SPL Token program
    pc.add_program(
        "spl_token",
        spl_token::id(),
        processor!(spl_token::processor::Processor::process),
    );

    pc
}

fn get_account<T: Pack>(
    banks_client: &mut BanksClient,
    address: Pubkey,
) -> impl Future<Output = std::io::Result<T>> + '_ {
    banks_client.get_account(address).map(|result| {
        let account =
            result?.ok_or_else(|| io::Error::new(io::ErrorKind::Other, "account not found"))?;

        T::unpack_from_slice(&account.data)
            .ok()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to deserialize account"))
    })
}

#[tokio::test]
async fn test_basic() {
    let feature_proposal = Keypair::new();

    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;

    let feature_id_address = get_feature_id_address(&feature_proposal.pubkey());

    // Create a new feature proposal
    let mut transaction = Transaction::new_with_payer(
        &[propose(
            &payer.pubkey(),
            &feature_proposal.pubkey(),
            42,
            AcceptanceCriteria {
                tokens_required: 42,
                deadline: None,
            },
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &feature_proposal], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Confirm feature id account is now funded and allocated, but not assigned
    let feature_id_acccount = banks_client
        .get_account(feature_id_address)
        .await
        .expect("success")
        .expect("some account");
    assert_eq!(feature_id_acccount.owner, system_program::id());
    assert_eq!(feature_id_acccount.data.len(), Feature::size_of());

    // Tally #1: Does nothing because the acceptance criteria has not been met
    let mut transaction =
        Transaction::new_with_payer(&[tally(&feature_proposal.pubkey())], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Confirm feature id account is not yet assigned
    let feature_id_acccount = banks_client
        .get_account(feature_id_address)
        .await
        .expect("success")
        .expect("some account");
    assert_eq!(feature_id_acccount.owner, system_program::id());

    let feature_proposal_acccount = banks_client
        .get_account(feature_proposal.pubkey())
        .await
        .expect("success")
        .expect("some account");
    let feature_proposal_acccount =
        spl_feature_proposal::state::FeatureProposal::unpack_from_slice(
            &feature_proposal_acccount.data,
        )
        .expect("unpack success");
    assert!(matches!(
        feature_proposal_acccount,
        FeatureProposal::Pending(AcceptanceCriteria {
            tokens_required: 42,
            deadline: None,
        })
    ));
    assert!(matches!(
        get_account::<FeatureProposal>(&mut banks_client, feature_proposal.pubkey()).await,
        Ok(FeatureProposal::Pending(_))
    ));

    // Transfer tokens to the acceptance account
    let delivery_token_address = get_delivery_token_address(&feature_proposal.pubkey());
    let acceptance_token_address = get_acceptance_token_address(&feature_proposal.pubkey());

    let mut transaction = Transaction::new_with_payer(
        &[spl_token::instruction::transfer(
            &spl_token::id(),
            &delivery_token_address,
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
        .get_new_blockhash(&recent_blockhash)
        .await
        .unwrap()
        .0;

    // Tally #2: the acceptance criteria is now met
    let mut transaction =
        Transaction::new_with_payer(&[tally(&feature_proposal.pubkey())], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Confirm feature id account is now assigned
    let feature_id_acccount = banks_client
        .get_account(feature_id_address)
        .await
        .expect("success")
        .expect("some account");
    assert_eq!(feature_id_acccount.owner, feature::id());

    // Confirm feature proposal account state
    assert!(matches!(
        get_account::<FeatureProposal>(&mut banks_client, feature_proposal.pubkey()).await,
        Ok(FeatureProposal::Accepted {
            tokens_upon_acceptance: 42
        })
    ));
}

// TODO: more tests....
