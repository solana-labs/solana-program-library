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
    solana_sdk::{
        instruction::InstructionError, pubkey::Pubkey, signature::Signer,
        system_instruction::SystemError, transaction::Transaction,
    },
    spl_single_validator_pool::{id, instruction},
};

fn assert_metadata(pool_account: &Pubkey, metadata: &Metadata) {
    let pool_address_str = pool_account.to_string();
    let name = format!("SPL Single Pool {}", &pool_address_str[0..15]);
    let symbol = format!("st{}", &pool_address_str[0..7]);
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
    assert_metadata(&accounts.pool, &metadata);
}

#[tokio::test]
async fn fail_double_init() {
    let mut context = program_test().start_with_context().await;
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
