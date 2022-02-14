// Mark this test as BPF-only due to current `ProgramTest` limitations when CPIing into the system program
#![cfg(feature = "test-bpf")]

use {
    solana_program::pubkey::Pubkey,
    solana_program_test::*,
    solana_sdk::{program_pack::Pack, signature::Signer, transaction::Transaction},
    spl_associated_token_account::{
        get_associated_token_address, id, instruction::create_associated_token_account,
        processor::process_instruction,
    },
    spl_token::state::Account,
};

#[allow(deprecated)]
use spl_associated_token_account::create_associated_token_account as deprecated_create_associated_token_account;

fn program_test(token_mint_address: Pubkey, use_latest_spl_token: bool) -> ProgramTest {
    let mut pc = ProgramTest::new(
        "spl_associated_token_account",
        id(),
        processor!(process_instruction),
    );

    if use_latest_spl_token {
        // TODO: Remove after Token >3.2.0 is available by default in program-test
        pc.add_program(
            "spl_token",
            spl_token::id(),
            processor!(spl_token::processor::Processor::process),
        );
    }

    // Add a token mint account
    //
    // The account data was generated by running:
    //      $ solana account EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
    //                       --output-file tests/fixtures/token-mint-data.bin
    //
    pc.add_account_with_file_data(
        token_mint_address,
        1461600,
        spl_token::id(),
        "token-mint-data.bin",
    );

    // Dial down the BPF compute budget to detect if the program gets bloated in the future
    pc.set_compute_max_units(50_000);

    pc
}

#[tokio::test]
async fn success_create() {
    let wallet_address = Pubkey::new_unique();
    let token_mint_address = Pubkey::new_unique();
    let associated_token_address =
        get_associated_token_address(&wallet_address, &token_mint_address);

    let (mut banks_client, payer, recent_blockhash) =
        program_test(token_mint_address, true).start().await;
    let rent = banks_client.get_rent().await.unwrap();
    let expected_token_account_len = Account::LEN;
    let expected_token_account_balance = rent.minimum_balance(expected_token_account_len);

    // Associated account does not exist
    assert_eq!(
        banks_client
            .get_account(associated_token_address)
            .await
            .expect("get_account"),
        None,
    );

    let transaction = Transaction::new_signed_with_payer(
        &[create_associated_token_account(
            &payer.pubkey(),
            &wallet_address,
            &token_mint_address,
            &spl_token::id(),
        )],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    // Associated account now exists
    let associated_account = banks_client
        .get_account(associated_token_address)
        .await
        .expect("get_account")
        .expect("associated_account not none");
    assert_eq!(associated_account.data.len(), expected_token_account_len);
    assert_eq!(associated_account.owner, spl_token::id());
    assert_eq!(associated_account.lamports, expected_token_account_balance);
}

#[tokio::test]
async fn success_using_deprecated_instruction_creator() {
    let wallet_address = Pubkey::new_unique();
    let token_mint_address = Pubkey::new_unique();
    let associated_token_address =
        get_associated_token_address(&wallet_address, &token_mint_address);

    let (mut banks_client, payer, recent_blockhash) =
        program_test(token_mint_address, true).start().await;
    let rent = banks_client.get_rent().await.unwrap();
    let expected_token_account_len = Account::LEN;
    let expected_token_account_balance = rent.minimum_balance(expected_token_account_len);

    // Associated account does not exist
    assert_eq!(
        banks_client
            .get_account(associated_token_address)
            .await
            .expect("get_account"),
        None,
    );

    // Use legacy instruction creator
    #[allow(deprecated)]
    let create_associated_token_account_ix = deprecated_create_associated_token_account(
        &payer.pubkey(),
        &wallet_address,
        &token_mint_address,
    );

    let transaction = Transaction::new_signed_with_payer(
        &[create_associated_token_account_ix],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    // Associated account now exists
    let associated_account = banks_client
        .get_account(associated_token_address)
        .await
        .expect("get_account")
        .expect("associated_account not none");
    assert_eq!(associated_account.data.len(), expected_token_account_len);
    assert_eq!(associated_account.owner, spl_token::id());
    assert_eq!(associated_account.lamports, expected_token_account_balance);
}
