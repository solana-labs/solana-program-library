use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, pubkey::Pubkey,
};

use crate::processor::Processor;

entrypoint!(process_instruction);
fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    Processor::process(program_id, accounts, instruction_data)
}

// #[cfg(test)]
// mod test {
//     use {
//         super::*,
//         assert_matches::*,
//         solana_program::instruction::{AccountMeta, Instruction},
//         solana_program_test::*,
//         solana_sdk::{signature::Signer, transaction::Transaction},
//     };

//     #[tokio::test]
//     async fn test_transaction() {
//         let program_id = Pubkey::new_unique();

//         let (mut banks_client, payer, recent_blockhash) = ProgramTest::new(
//             "bpf_program_template",
//             program_id,
//             processor!(process_instruction),
//         )
//         .start()
//         .await;

//         let mut transaction = Transaction::new_with_payer(
//             &[Instruction {
//                 program_id,
//                 accounts: vec![AccountMeta::new(payer.pubkey(), false)],
//                 data: vec![1, 2, 3],
//             }],
//             Some(&payer.pubkey()),
//         );
//         transaction.sign(&[&payer], recent_blockhash);

//         assert_matches!(banks_client.process_transaction(transaction).await, Ok(()));
//     }
// }
