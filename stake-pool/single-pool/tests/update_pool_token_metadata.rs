#![allow(clippy::integer_arithmetic)]
#![cfg(feature = "test-sbf")]
mod helpers;

use {
    helpers::*,
    mpl_token_metadata::{
        state::{MAX_NAME_LENGTH, MAX_SYMBOL_LENGTH, MAX_URI_LENGTH},
        utils::puffed_out_string,
    },
    solana_program_test::*,
    solana_sdk::{signature::Signer, transaction::Transaction},
    spl_single_validator_pool::{id, instruction},
};

#[tokio::test]
async fn success_update_pool_token_metadata() {
    let mut context = program_test().start_with_context().await;
    let accounts = SinglePoolAccounts::default();
    accounts.initialize(&mut context).await;

    let updated_name = "updated_name";
    let updated_symbol = "USYM";
    let updated_uri = "updated_uri";

    let puffed_name = puffed_out_string(updated_name, MAX_NAME_LENGTH);
    let puffed_symbol = puffed_out_string(updated_symbol, MAX_SYMBOL_LENGTH);
    let puffed_uri = puffed_out_string(updated_uri, MAX_URI_LENGTH);

    let instruction = instruction::update_token_metadata(
        &id(),
        &accounts.vote_account.pubkey(),
        &accounts.withdrawer.pubkey(),
        updated_name.to_string(),
        updated_symbol.to_string(),
        updated_uri.to_string(),
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &[&context.payer, &accounts.withdrawer],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let metadata = get_metadata_account(&mut context.banks_client, &accounts.mint).await;

    assert_eq!(metadata.data.name, puffed_name);
    assert_eq!(metadata.data.symbol, puffed_symbol);
    assert_eq!(metadata.data.uri, puffed_uri);
}

// TODO test bad withdrawer, test bad vote account (edit the ixn by hand to have the correct authority)
// doing these in a different pr pending a program change, to avoid merge conflicts
