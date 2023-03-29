#![allow(clippy::integer_arithmetic)]
#![cfg(feature = "test-sbf")]

mod helpers;

use {
    helpers::*,
    mpl_token_metadata::{
        state::Metadata,
        state::{MAX_NAME_LENGTH, MAX_SYMBOL_LENGTH},
        utils::puffed_out_string,
    },
    solana_program_test::*,
    solana_sdk::{message::Message, pubkey::Pubkey, signature::Signer, transaction::Transaction},
    spl_single_validator_pool::{id, instruction},
};

fn assert_metadata(vote_account: &Pubkey, metadata: &Metadata) {
    let vote_address_str = vote_account.to_string();
    let name = format!("SPL Single Pool {}", &vote_address_str[0..15]);
    let symbol = format!("st{}", &vote_address_str[0..7]);
    let puffy_name = puffed_out_string(&name, MAX_NAME_LENGTH);
    let puffy_symbol = puffed_out_string(&symbol, MAX_SYMBOL_LENGTH);

    assert_eq!(metadata.data.name, puffy_name);
    assert_eq!(metadata.data.symbol, puffy_symbol);
}

#[tokio::test]
async fn success() {
    let mut context = program_test().start_with_context().await;
    let accounts = SinglePoolAccounts::default();
    accounts.initialize(&mut context).await;

    let metadata = get_metadata_account(&mut context.banks_client, &accounts.mint).await;
    assert_metadata(&accounts.vote_account.pubkey(), &metadata);
}

#[tokio::test]
async fn fail_double_init() {
    let mut context = program_test().start_with_context().await;
    let accounts = SinglePoolAccounts::default();
    accounts.initialize(&mut context).await;
    refresh_blockhash(&mut context).await;

    let instruction = instruction::create_token_metadata(
        &id(),
        &accounts.vote_account.pubkey(),
        &context.payer.pubkey(),
    );
    let message = Message::new(&[instruction], Some(&context.payer.pubkey()));
    let transaction = Transaction::new(&[&context.payer], message, context.last_blockhash);

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err();
}
