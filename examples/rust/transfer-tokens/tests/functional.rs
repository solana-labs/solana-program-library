use {
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_pack::Pack,
        pubkey::Pubkey,
        rent::Rent,
        system_instruction,
    },
    solana_program_test::{processor, tokio, ProgramTest},
    solana_sdk::{signature::Signer, signer::keypair::Keypair, transaction::Transaction},
    spl_example_transfer_tokens::processor::process_instruction,
    spl_token::state::{Account, Mint},
    std::str::FromStr,
};

#[tokio::test]
async fn success() {
    // Setup some pubkeys for the accounts
    let program_id = Pubkey::from_str("TransferTokens11111111111111111111111111111").unwrap();
    let source = Keypair::new();
    let mint = Keypair::new();
    let destination = Keypair::new();
    let (authority_pubkey, _) = Pubkey::find_program_address(&[b"authority"], &program_id);

    // Add the program to the test framework
    let program_test = ProgramTest::new(
        "spl_example_transfer_tokens",
        program_id,
        processor!(process_instruction),
    );
    let amount = 10_000;
    let decimals = 9;
    let rent = Rent::default();

    // Start the program test
    let (banks_client, payer, recent_blockhash) = program_test.start().await;

    // Setup the mint, used in `spl_token::instruction::transfer_checked`
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &mint.pubkey(),
                rent.minimum_balance(Mint::LEN),
                Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &mint.pubkey(),
                &payer.pubkey(),
                None,
                decimals,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
        &[&payer, &mint],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    // Setup the source account, owned by the program-derived address
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &source.pubkey(),
                rent.minimum_balance(Account::LEN),
                Account::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_account(
                &spl_token::id(),
                &source.pubkey(),
                &mint.pubkey(),
                &authority_pubkey,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
        &[&payer, &source],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    // Setup the destination account, used to receive tokens from the account
    // owned by the program-derived address
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &destination.pubkey(),
                rent.minimum_balance(Account::LEN),
                Account::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_account(
                &spl_token::id(),
                &destination.pubkey(),
                &mint.pubkey(),
                &payer.pubkey(),
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
        &[&payer, &destination],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    // Mint some tokens to the PDA account
    let transaction = Transaction::new_signed_with_payer(
        &[spl_token::instruction::mint_to(
            &spl_token::id(),
            &mint.pubkey(),
            &source.pubkey(),
            &payer.pubkey(),
            &[],
            amount,
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    // Create an instruction following the account order expected by the program
    let transaction = Transaction::new_signed_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &(),
            vec![
                AccountMeta::new(source.pubkey(), false),
                AccountMeta::new_readonly(mint.pubkey(), false),
                AccountMeta::new(destination.pubkey(), false),
                AccountMeta::new_readonly(authority_pubkey, false),
                AccountMeta::new_readonly(spl_token::id(), false),
            ],
        )],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    // See that the transaction processes successfully
    banks_client.process_transaction(transaction).await.unwrap();

    // Check that the destination account now has `amount` tokens
    let account = banks_client
        .get_account(destination.pubkey())
        .await
        .unwrap()
        .unwrap();
    let token_account = Account::unpack(&account.data).unwrap();
    assert_eq!(token_account.amount, amount);
}
