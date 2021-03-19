// Mark this test as BPF-only due to current `ProgramTest` limitations when CPIing into the system program
#![cfg(feature = "test-bpf")]

use {
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        rent::Rent,
        system_program,
    },
    solana_program_test::*,
    solana_sdk::{account::Account, signature::Signer, transaction::Transaction},
    spl_example_cross_program_invocation::processor::{process_instruction, SIZE},
    std::str::FromStr,
};

#[tokio::test]
async fn test_cross_program_invocation() {
    let program_id = Pubkey::from_str(&"invoker111111111111111111111111111111111111").unwrap();
    let (allocated_pubkey, bump_seed) =
        Pubkey::find_program_address(&[b"You pass butter"], &program_id);
    let mut program_test = ProgramTest::new(
        "spl_example_cross_program_invocation",
        program_id,
        processor!(process_instruction),
    );
    program_test.add_account(
        allocated_pubkey,
        Account {
            lamports: Rent::default().minimum_balance(SIZE),
            ..Account::default()
        },
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &[bump_seed],
            vec![
                AccountMeta::new(system_program::id(), false),
                AccountMeta::new(allocated_pubkey, false),
            ],
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Associated account now exists
    let allocated_account = banks_client
        .get_account(allocated_pubkey)
        .await
        .expect("get_account")
        .expect("associated_account not none");
    assert_eq!(allocated_account.data.len(), SIZE);
}
