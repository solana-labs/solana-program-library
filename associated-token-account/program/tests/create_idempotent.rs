#![cfg(feature = "test-bpf")]

mod program_test;

use {
    program_test::program_test,
    solana_program::{instruction::*, pubkey::Pubkey},
    solana_program_test::*,
    solana_sdk::{
        account::Account as SolanaAccount,
        program_option::COption,
        program_pack::Pack,
        signature::Signer,
        signer::keypair::Keypair,
        system_instruction::create_account,
        transaction::{Transaction, TransactionError},
    },
    spl_associated_token_account::{
        error::AssociatedTokenAccountError,
        get_associated_token_address_with_program_id,
        instruction::{
            create_associated_token_account, create_associated_token_account_idempotent,
        },
    },
    spl_token_2022::{
        extension::ExtensionType,
        instruction::initialize_account,
        state::{Account, AccountState},
    },
};

#[tokio::test]
async fn success_account_exists() {
    let wallet_address = Pubkey::new_unique();
    let token_mint_address = Pubkey::new_unique();
    let associated_token_address = get_associated_token_address_with_program_id(
        &wallet_address,
        &token_mint_address,
        &spl_token_2022::id(),
    );

    let (mut banks_client, payer, recent_blockhash) =
        program_test(token_mint_address, true).start().await;
    let rent = banks_client.get_rent().await.unwrap();
    let expected_token_account_len =
        ExtensionType::get_account_len::<Account>(&[ExtensionType::ImmutableOwner]);
    let expected_token_account_balance = rent.minimum_balance(expected_token_account_len);

    let instruction = create_associated_token_account_idempotent(
        &payer.pubkey(),
        &wallet_address,
        &token_mint_address,
        &spl_token_2022::id(),
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
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
    assert_eq!(associated_account.owner, spl_token_2022::id());
    assert_eq!(associated_account.lamports, expected_token_account_balance);

    // Unchecked instruction fails
    let instruction = create_associated_token_account(
        &payer.pubkey(),
        &wallet_address,
        &token_mint_address,
        &spl_token_2022::id(),
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    assert_eq!(
        banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(0, InstructionError::IllegalOwner)
    );

    // Get a new blockhash, succeed with create if non existent
    let recent_blockhash = banks_client
        .get_new_latest_blockhash(&recent_blockhash)
        .await
        .unwrap();

    let instruction = create_associated_token_account_idempotent(
        &payer.pubkey(),
        &wallet_address,
        &token_mint_address,
        &spl_token_2022::id(),
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    // Associated account is unchanged
    let associated_account = banks_client
        .get_account(associated_token_address)
        .await
        .expect("get_account")
        .expect("associated_account not none");
    assert_eq!(associated_account.data.len(), expected_token_account_len);
    assert_eq!(associated_account.owner, spl_token_2022::id());
    assert_eq!(associated_account.lamports, expected_token_account_balance);
}

#[tokio::test]
async fn fail_account_exists_with_wrong_owner() {
    let wallet_address = Pubkey::new_unique();
    let token_mint_address = Pubkey::new_unique();
    let associated_token_address = get_associated_token_address_with_program_id(
        &wallet_address,
        &token_mint_address,
        &spl_token_2022::id(),
    );

    let wrong_owner = Pubkey::new_unique();
    let mut associated_token_account =
        SolanaAccount::new(1_000_000_000, Account::LEN, &spl_token_2022::id());
    let token_account = Account {
        mint: token_mint_address,
        owner: wrong_owner,
        amount: 0,
        delegate: COption::None,
        state: AccountState::Initialized,
        is_native: COption::None,
        delegated_amount: 0,
        close_authority: COption::None,
    };
    Account::pack(token_account, &mut associated_token_account.data).unwrap();
    let mut pt = program_test(token_mint_address, true);
    pt.add_account(associated_token_address, associated_token_account);
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    // fail creating token account if non existent
    let instruction = create_associated_token_account_idempotent(
        &payer.pubkey(),
        &wallet_address,
        &token_mint_address,
        &spl_token_2022::id(),
    );
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    assert_eq!(
        banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AssociatedTokenAccountError::InvalidOwner as u32)
        )
    );
}

#[tokio::test]
async fn fail_non_ata() {
    let token_mint_address = Pubkey::new_unique();
    let (mut banks_client, payer, recent_blockhash) =
        program_test(token_mint_address, true).start().await;

    let rent = banks_client.get_rent().await.unwrap();
    let token_account_len =
        ExtensionType::get_account_len::<Account>(&[ExtensionType::ImmutableOwner]);
    let token_account_balance = rent.minimum_balance(token_account_len);

    let wallet_address = Pubkey::new_unique();
    let account = Keypair::new();
    let transaction = Transaction::new_signed_with_payer(
        &[
            create_account(
                &payer.pubkey(),
                &account.pubkey(),
                token_account_balance,
                token_account_len as u64,
                &spl_token_2022::id(),
            ),
            initialize_account(
                &spl_token_2022::id(),
                &account.pubkey(),
                &token_mint_address,
                &wallet_address,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
        &[&payer, &account],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    let mut instruction = create_associated_token_account_idempotent(
        &payer.pubkey(),
        &wallet_address,
        &token_mint_address,
        &spl_token_2022::id(),
    );
    instruction.accounts[1] = AccountMeta::new(account.pubkey(), false); // <-- Invalid associated_account_address

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    assert_eq!(
        banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(0, InstructionError::InvalidSeeds)
    );
}
