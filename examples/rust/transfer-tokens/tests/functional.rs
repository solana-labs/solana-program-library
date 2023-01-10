use {
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_pack::Pack,
        pubkey::Pubkey,
        rent::Rent,
    },
    solana_program_test::{processor, tokio, ProgramTest},
    solana_sdk::{account::Account, signature::Signer, transaction::Transaction},
    spl_example_transfer_tokens::processor::process_instruction,
    spl_token::state::{Account as TokenAccount, Mint},
    std::str::FromStr,
};

#[tokio::test]
async fn success() {
    // Setup some pubkeys for the accounts
    let program_id = Pubkey::from_str("TransferTokens11111111111111111111111111111").unwrap();
    let source_pubkey = Pubkey::new_unique();
    let mint_pubkey = Pubkey::new_unique();
    let destination_pubkey = Pubkey::new_unique();
    let destination_owner_pubkey = Pubkey::new_unique();
    let (authority_pubkey, _) = Pubkey::find_program_address(&[b"authority"], &program_id);

    // Add the program to the test framework
    let rent = Rent::default();
    let mut program_test = ProgramTest::new(
        "spl_example_transfer_tokens",
        program_id,
        processor!(process_instruction),
    );
    let amount = 10_000;
    let decimals = 9;

    // Setup the source account, owned by the program-derived address
    let mut data = vec![0; TokenAccount::LEN];
    TokenAccount::pack(
        TokenAccount {
            mint: mint_pubkey,
            owner: authority_pubkey,
            amount,
            state: spl_token::state::AccountState::Initialized,
            ..TokenAccount::default()
        },
        &mut data,
    )
    .unwrap();
    program_test.add_account(
        source_pubkey,
        Account {
            lamports: rent.minimum_balance(TokenAccount::LEN),
            owner: spl_token::id(),
            data,
            ..Account::default()
        },
    );

    // Setup the mint, used in `spl_token::instruction::transfer_checked`
    let mut data = vec![0; Mint::LEN];
    Mint::pack(
        Mint {
            supply: amount,
            decimals,
            is_initialized: true,
            ..Mint::default()
        },
        &mut data,
    )
    .unwrap();
    program_test.add_account(
        mint_pubkey,
        Account {
            lamports: rent.minimum_balance(Mint::LEN),
            owner: spl_token::id(),
            data,
            ..Account::default()
        },
    );

    // Setup the destination account, used to receive tokens from the account
    // owned by the program-derived address
    let mut data = vec![0; TokenAccount::LEN];
    TokenAccount::pack(
        TokenAccount {
            mint: mint_pubkey,
            owner: destination_owner_pubkey,
            amount: 0,
            state: spl_token::state::AccountState::Initialized,
            ..TokenAccount::default()
        },
        &mut data,
    )
    .unwrap();
    program_test.add_account(
        destination_pubkey,
        Account {
            lamports: rent.minimum_balance(TokenAccount::LEN),
            owner: spl_token::id(),
            data,
            ..Account::default()
        },
    );

    // Start the program test
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    // Create an instruction following the account order expected by the program
    let transaction = Transaction::new_signed_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &(),
            vec![
                AccountMeta::new(source_pubkey, false),
                AccountMeta::new_readonly(mint_pubkey, false),
                AccountMeta::new(destination_pubkey, false),
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
        .get_account(destination_pubkey)
        .await
        .unwrap()
        .unwrap();
    let token_account = TokenAccount::unpack(&account.data).unwrap();
    assert_eq!(token_account.amount, amount);
}
