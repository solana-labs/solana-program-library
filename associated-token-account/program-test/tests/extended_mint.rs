// Mark this test as BPF-only due to current `ProgramTest` limitations when
// CPIing into the system program
#![cfg(feature = "test-sbf")]

mod program_test;

use {
    program_test::program_test_2022,
    solana_program::{instruction::*, pubkey::Pubkey, system_instruction},
    solana_program_test::*,
    solana_sdk::{
        signature::Signer,
        signer::keypair::Keypair,
        transaction::{Transaction, TransactionError},
    },
    spl_associated_token_account::instruction::create_associated_token_account,
    spl_associated_token_account_client::address::get_associated_token_address_with_program_id,
    spl_token_2022::{
        error::TokenError,
        extension::{
            transfer_fee, BaseStateWithExtensions, ExtensionType, StateWithExtensionsOwned,
        },
        state::{Account, Mint},
    },
};

#[tokio::test]
async fn test_associated_token_account_with_transfer_fees() {
    let wallet_sender = Keypair::new();
    let wallet_address_sender = wallet_sender.pubkey();
    let wallet_address_receiver = Pubkey::new_unique();
    let (mut banks_client, payer, recent_blockhash) =
        program_test_2022(Pubkey::new_unique(), true).start().await;
    let rent = banks_client.get_rent().await.unwrap();

    // create extended mint
    // ... in the future, a mint can be pre-loaded in program_test.rs like the
    // regular mint
    let mint_account = Keypair::new();
    let token_mint_address = mint_account.pubkey();
    let mint_authority = Keypair::new();
    let space =
        ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::TransferFeeConfig])
            .unwrap();
    let maximum_fee = 100;
    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &mint_account.pubkey(),
                rent.minimum_balance(space),
                space as u64,
                &spl_token_2022::id(),
            ),
            transfer_fee::instruction::initialize_transfer_fee_config(
                &spl_token_2022::id(),
                &token_mint_address,
                Some(&mint_authority.pubkey()),
                Some(&mint_authority.pubkey()),
                1_000,
                maximum_fee,
            )
            .unwrap(),
            spl_token_2022::instruction::initialize_mint(
                &spl_token_2022::id(),
                &token_mint_address,
                &mint_authority.pubkey(),
                Some(&mint_authority.pubkey()),
                0,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &mint_account], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // create extended ATAs
    let mut transaction = Transaction::new_with_payer(
        &[create_associated_token_account(
            &payer.pubkey(),
            &wallet_address_sender,
            &token_mint_address,
            &spl_token_2022::id(),
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    let recent_blockhash = banks_client
        .get_new_latest_blockhash(&recent_blockhash)
        .await
        .unwrap();

    let mut transaction = Transaction::new_with_payer(
        &[create_associated_token_account(
            &payer.pubkey(),
            &wallet_address_receiver,
            &token_mint_address,
            &spl_token_2022::id(),
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    let associated_token_address_sender = get_associated_token_address_with_program_id(
        &wallet_address_sender,
        &token_mint_address,
        &spl_token_2022::id(),
    );
    let associated_token_address_receiver = get_associated_token_address_with_program_id(
        &wallet_address_receiver,
        &token_mint_address,
        &spl_token_2022::id(),
    );

    // mint tokens
    let sender_amount = 50 * maximum_fee;
    let mut transaction = Transaction::new_with_payer(
        &[spl_token_2022::instruction::mint_to(
            &spl_token_2022::id(),
            &token_mint_address,
            &associated_token_address_sender,
            &mint_authority.pubkey(),
            &[],
            sender_amount,
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &mint_authority], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // not enough tokens
    let mut transaction = Transaction::new_with_payer(
        &[transfer_fee::instruction::transfer_checked_with_fee(
            &spl_token_2022::id(),
            &associated_token_address_sender,
            &token_mint_address,
            &associated_token_address_receiver,
            &wallet_address_sender,
            &[],
            10_001,
            0,
            maximum_fee,
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &wallet_sender], recent_blockhash);
    let err = banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err()
        .unwrap();
    assert_eq!(
        err,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(TokenError::InsufficientFunds as u32)
        )
    );

    let recent_blockhash = banks_client
        .get_new_latest_blockhash(&recent_blockhash)
        .await
        .unwrap();

    // success
    let transfer_amount = 500;
    let fee = 50;
    let mut transaction = Transaction::new_with_payer(
        &[transfer_fee::instruction::transfer_checked_with_fee(
            &spl_token_2022::id(),
            &associated_token_address_sender,
            &token_mint_address,
            &associated_token_address_receiver,
            &wallet_address_sender,
            &[],
            transfer_amount,
            0,
            fee,
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &wallet_sender], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    let sender_account = banks_client
        .get_account(associated_token_address_sender)
        .await
        .unwrap()
        .unwrap();
    let sender_state = StateWithExtensionsOwned::<Account>::unpack(sender_account.data).unwrap();
    assert_eq!(sender_state.base.amount, sender_amount - transfer_amount);
    let extension = sender_state
        .get_extension::<transfer_fee::TransferFeeAmount>()
        .unwrap();
    assert_eq!(extension.withheld_amount, 0.into());

    let receiver_account = banks_client
        .get_account(associated_token_address_receiver)
        .await
        .unwrap()
        .unwrap();
    let receiver_state =
        StateWithExtensionsOwned::<Account>::unpack(receiver_account.data).unwrap();
    assert_eq!(receiver_state.base.amount, transfer_amount - fee);
    let extension = receiver_state
        .get_extension::<transfer_fee::TransferFeeAmount>()
        .unwrap();
    assert_eq!(extension.withheld_amount, fee.into());
}
