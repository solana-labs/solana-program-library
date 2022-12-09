#![cfg(feature = "test-sbf")]
use {
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_pack::Pack,
        pubkey::Pubkey,
    },
    solana_program_test::{processor, tokio, ProgramTest},
    solana_sdk::{
        account::Account, program_option::COption, signature::Signer, signer::keypair::Keypair,
        system_program, transaction::Transaction,
    },
    spl_token_2022::state::Mint,
    spl_token_manager_registry::{
        find_manager_registration_address, processor::process_instruction,
    },
};

const LAMPORTS: u64 = 2_000_000;

#[tokio::test]
async fn write_program_id() {
    let program_id = spl_token_manager_registry::id();
    let mint_pubkey = Pubkey::new_unique();
    let mint_authority = Keypair::new();
    let manager_program_pubkey = Pubkey::new_unique();
    let manager_registration_pubkey = find_manager_registration_address(&program_id, &mint_pubkey);
    let mut program_test = ProgramTest::new(
        "spl_token_manager_registry",
        program_id,
        processor!(process_instruction),
    );
    let mut data = vec![0u8; Mint::LEN];
    let mint = Mint {
        mint_authority: COption::Some(mint_authority.pubkey()),
        is_initialized: true,
        ..Mint::default()
    };
    Mint::pack(mint, &mut data).unwrap();
    program_test.add_account(
        mint_pubkey,
        Account {
            lamports: LAMPORTS,
            owner: spl_token_2022::id(),
            data,
            ..Account::default()
        },
    );
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bytes(
            program_id,
            &[],
            vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(mint_pubkey, false),
                AccountMeta::new_readonly(mint_authority.pubkey(), true),
                AccountMeta::new(manager_registration_pubkey, false),
                AccountMeta::new_readonly(manager_program_pubkey, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &mint_authority], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    let registration_account = banks_client
        .get_account(manager_registration_pubkey)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(&registration_account.data, manager_program_pubkey.as_ref());
}
