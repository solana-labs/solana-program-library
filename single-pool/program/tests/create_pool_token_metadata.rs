#![allow(clippy::arithmetic_side_effects)]
#![cfg(feature = "test-sbf")]

mod helpers;

use {
    helpers::*,
    solana_program_test::*,
    solana_sdk::{
        instruction::InstructionError, pubkey::Pubkey, signature::Signer,
        system_instruction::SystemError, transaction::Transaction,
    },
    spl_single_pool::{id, instruction},
};

fn assert_metadata(vote_account: &Pubkey, metadata: &Metadata) {
    let vote_address_str = vote_account.to_string();
    let name = format!("SPL Single Pool {}", &vote_address_str[0..15]);
    let symbol = format!("st{}", &vote_address_str[0..7]);

    assert!(metadata.name.starts_with(&name));
    assert!(metadata.symbol.starts_with(&symbol));
}

#[tokio::test]
async fn success() {
    let mut context = program_test(false).start_with_context().await;
    let accounts = SinglePoolAccounts::default();
    accounts.initialize(&mut context).await;

    let metadata = get_metadata_account(&mut context.banks_client, &accounts.mint).await;
    assert_metadata(&accounts.vote_account.pubkey(), &metadata);
}

#[tokio::test]
async fn fail_double_init() {
    let mut context = program_test(false).start_with_context().await;
    let accounts = SinglePoolAccounts::default();
    accounts.initialize(&mut context).await;
    refresh_blockhash(&mut context).await;

    let instruction =
        instruction::create_token_metadata(&id(), &accounts.pool, &context.payer.pubkey());
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    let e = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err();
    check_error::<InstructionError>(e, SystemError::AccountAlreadyInUse.into());
}
