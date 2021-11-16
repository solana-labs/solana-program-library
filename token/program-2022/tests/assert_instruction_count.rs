#![cfg(feature = "test-bpf")]

use {
    solana_program_test::{processor, tokio, ProgramTest},
    solana_sdk::{
        program_pack::Pack,
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        system_instruction,
        transaction::Transaction,
    },
    spl_token_2022::{
        id, instruction,
        processor::Processor,
        state::{Account, Mint},
    },
};

const TRANSFER_AMOUNT: u64 = 1_000_000_000_000_000;

#[tokio::test]
async fn initialize_mint() {
    let mut pt = ProgramTest::new("spl_token", id(), processor!(Processor::process));
    pt.set_bpf_compute_max_units(2_500); // last known 2252
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let owner_key = Pubkey::new_unique();
    let mint = Keypair::new();
    let decimals = 9;

    let rent = banks_client.get_rent().await.unwrap();
    let mint_rent = rent.minimum_balance(Mint::LEN);
    let transaction = Transaction::new_signed_with_payer(
        &[system_instruction::create_account(
            &payer.pubkey(),
            &mint.pubkey(),
            mint_rent,
            Mint::LEN as u64,
            &id(),
        )],
        Some(&payer.pubkey()),
        &[&payer, &mint],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[
            instruction::initialize_mint(&id(), &mint.pubkey(), &owner_key, None, decimals)
                .unwrap(),
        ],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();
}

#[tokio::test]
async fn initialize_account() {
    let mut pt = ProgramTest::new("spl_token", id(), processor!(Processor::process));
    pt.set_bpf_compute_max_units(4_000); // last known 3284
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let owner = Keypair::new();
    let mint = Keypair::new();
    let account = Keypair::new();
    let decimals = 9;

    let rent = banks_client.get_rent().await.unwrap();
    let mint_rent = rent.minimum_balance(Mint::LEN);
    let account_rent = rent.minimum_balance(Account::LEN);
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &mint.pubkey(),
                mint_rent,
                Mint::LEN as u64,
                &id(),
            ),
            system_instruction::create_account(
                &payer.pubkey(),
                &account.pubkey(),
                account_rent,
                Account::LEN as u64,
                &id(),
            ),
        ],
        Some(&payer.pubkey()),
        &[&payer, &mint, &account],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[
            instruction::initialize_mint(&id(), &mint.pubkey(), &owner.pubkey(), None, decimals)
                .unwrap(),
        ],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::initialize_account(
            &id(),
            &account.pubkey(),
            &mint.pubkey(),
            &owner.pubkey(),
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();
}

#[tokio::test]
async fn mint_to() {
    let mut pt = ProgramTest::new("spl_token", id(), processor!(Processor::process));
    pt.set_bpf_compute_max_units(4_000); // last known 2668
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let owner = Keypair::new();
    let mint = Keypair::new();
    let account = Keypair::new();
    let decimals = 9;

    let rent = banks_client.get_rent().await.unwrap();
    let mint_rent = rent.minimum_balance(Mint::LEN);
    let account_rent = rent.minimum_balance(Account::LEN);
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &mint.pubkey(),
                mint_rent,
                Mint::LEN as u64,
                &id(),
            ),
            system_instruction::create_account(
                &payer.pubkey(),
                &account.pubkey(),
                account_rent,
                Account::LEN as u64,
                &id(),
            ),
        ],
        Some(&payer.pubkey()),
        &[&payer, &mint, &account],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[
            instruction::initialize_mint(&id(), &mint.pubkey(), &owner.pubkey(), None, decimals)
                .unwrap(),
        ],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::initialize_account(
            &id(),
            &account.pubkey(),
            &mint.pubkey(),
            &owner.pubkey(),
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::mint_to(
            &id(),
            &mint.pubkey(),
            &account.pubkey(),
            &owner.pubkey(),
            &[],
            TRANSFER_AMOUNT,
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[&payer, &owner],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();
}

#[tokio::test]
async fn transfer() {
    let mut pt = ProgramTest::new("spl_token", id(), processor!(Processor::process));
    pt.set_bpf_compute_max_units(4_000); // last known 2972
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let owner = Keypair::new();
    let mint = Keypair::new();
    let source = Keypair::new();
    let destination = Keypair::new();
    let decimals = 9;

    let rent = banks_client.get_rent().await.unwrap();
    let mint_rent = rent.minimum_balance(Mint::LEN);
    let account_rent = rent.minimum_balance(Account::LEN);
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &mint.pubkey(),
                mint_rent,
                Mint::LEN as u64,
                &id(),
            ),
            system_instruction::create_account(
                &payer.pubkey(),
                &source.pubkey(),
                account_rent,
                Account::LEN as u64,
                &id(),
            ),
            system_instruction::create_account(
                &payer.pubkey(),
                &destination.pubkey(),
                account_rent,
                Account::LEN as u64,
                &id(),
            ),
        ],
        Some(&payer.pubkey()),
        &[&payer, &mint, &source, &destination],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[
            instruction::initialize_mint(&id(), &mint.pubkey(), &owner.pubkey(), None, decimals)
                .unwrap(),
        ],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::initialize_account(
            &id(),
            &source.pubkey(),
            &mint.pubkey(),
            &owner.pubkey(),
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::initialize_account(
            &id(),
            &destination.pubkey(),
            &mint.pubkey(),
            &owner.pubkey(),
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::mint_to(
            &id(),
            &mint.pubkey(),
            &source.pubkey(),
            &owner.pubkey(),
            &[],
            TRANSFER_AMOUNT,
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[&payer, &owner],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::transfer(
            &id(),
            &source.pubkey(),
            &destination.pubkey(),
            &owner.pubkey(),
            &[],
            TRANSFER_AMOUNT,
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[&payer, &owner],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();
}

#[tokio::test]
async fn burn() {
    let mut pt = ProgramTest::new("spl_token", id(), processor!(Processor::process));
    pt.set_bpf_compute_max_units(4_000); // last known 2655
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let owner = Keypair::new();
    let mint = Keypair::new();
    let account = Keypair::new();
    let decimals = 9;

    let rent = banks_client.get_rent().await.unwrap();
    let mint_rent = rent.minimum_balance(Mint::LEN);
    let account_rent = rent.minimum_balance(Account::LEN);
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &mint.pubkey(),
                mint_rent,
                Mint::LEN as u64,
                &id(),
            ),
            system_instruction::create_account(
                &payer.pubkey(),
                &account.pubkey(),
                account_rent,
                Account::LEN as u64,
                &id(),
            ),
        ],
        Some(&payer.pubkey()),
        &[&payer, &mint, &account],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[
            instruction::initialize_mint(&id(), &mint.pubkey(), &owner.pubkey(), None, decimals)
                .unwrap(),
        ],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::initialize_account(
            &id(),
            &account.pubkey(),
            &mint.pubkey(),
            &owner.pubkey(),
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::mint_to(
            &id(),
            &mint.pubkey(),
            &account.pubkey(),
            &owner.pubkey(),
            &[],
            TRANSFER_AMOUNT,
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[&payer, &owner],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::burn(
            &id(),
            &account.pubkey(),
            &mint.pubkey(),
            &owner.pubkey(),
            &[],
            TRANSFER_AMOUNT,
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[&payer, &owner],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();
}
