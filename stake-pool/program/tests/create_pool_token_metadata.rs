#![cfg(feature = "test-bpf")]
mod helpers;

use borsh::BorshDeserialize;
use helpers::*;
use mpl_token_metadata::state::{
    Metadata, MAX_NAME_LENGTH, MAX_SYMBOL_LENGTH, MAX_URI_LENGTH, PREFIX,
};
use mpl_token_metadata::utils::puffed_out_string;
use solana_program::borsh::try_from_slice_unchecked;
use solana_program::instruction::InstructionError;
use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::{Transaction, TransactionError};
use spl_stake_pool::instruction;
use spl_stake_pool::MINIMUM_RESERVE_LAMPORTS;

async fn setup() -> (ProgramTestContext, StakePoolAccounts) {
    let mut context = program_test_with_metadata_program()
        .start_with_context()
        .await;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            MINIMUM_RESERVE_LAMPORTS,
        )
        .await
        .unwrap();

    (context, stake_pool_accounts)
}

#[tokio::test]
async fn success_create_pool_token_metadata() {
    let (mut context, stake_pool_accounts) = setup().await;

    let name = "test_name";
    let symbol = "SYM";
    let uri = "test_uri";

    let puffed_name = puffed_out_string(&name, MAX_NAME_LENGTH);
    let puffed_symbol = puffed_out_string(&symbol, MAX_SYMBOL_LENGTH);
    let puffed_uri = puffed_out_string(&uri, MAX_URI_LENGTH);

    let ix = instruction::create_token_metadata(
        &spl_stake_pool::id(),
        &stake_pool_accounts.stake_pool.pubkey(),
        &stake_pool_accounts.manager.pubkey(),
        &stake_pool_accounts.pool_mint.pubkey(),
        &context.payer.pubkey(),
        name.to_string(),
        symbol.to_string(),
        uri.to_string(),
    );

    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &stake_pool_accounts.manager],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let metadata = get_metadata_account(
        &mut context.banks_client,
        &stake_pool_accounts.pool_mint.pubkey(),
    )
    .await;

    assert_eq!(metadata.data.name.to_string(), puffed_name);
    assert_eq!(metadata.data.symbol.to_string(), puffed_symbol);
    assert_eq!(metadata.data.uri.to_string(), puffed_uri);
}

#[tokio::test]
async fn fail_manager_did_not_sign() {
    let (mut context, stake_pool_accounts) = setup().await;

    let name = "test_name";
    let symbol = "SYM";
    let uri = "test_uri";

    let ix = instruction::create_token_metadata(
        &spl_stake_pool::id(),
        &stake_pool_accounts.stake_pool.pubkey(),
        &stake_pool_accounts.manager.pubkey(),
        &stake_pool_accounts.pool_mint.pubkey(),
        &context.payer.pubkey(),
        name.to_string(),
        symbol.to_string(),
        uri.to_string(),
    );

    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            let program_error = crate::error::SignatureMissing as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while signing with the wrong manager"),
    }
}
