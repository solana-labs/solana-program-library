//Source: https://github.com/mvines/solana-bpf-program-template/blob/af5c59f5ed4b07e3575aede76f05550ab251a22a/src/lib.rs
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    hash::Hash,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar,
};
use std::convert::TryInto;

entrypoint!(process_instruction);
fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let sysvar_slot_history = next_account_info(accounts_iter)?;

    /*
        Decoding the SlotHashes sysvar using `from_account_info` is too expensive.
        For example this statement will exceed the current BPF compute unit budget:

            let slot_hashes = SlotHashes::from_account_info(&sysvar_slot_history).unwrap();

        Instead manually decode the sysvar.
    */

    if *sysvar_slot_history.key != sysvar::slot_hashes::id() {
        msg!("Invalid SlotHashes sysvar");
        return Err(ProgramError::InvalidArgument);
    }

    let data = sysvar_slot_history.try_borrow_data()?;

    let num_slot_hashes = u64::from_le_bytes(data[0..8].try_into().unwrap());
    let mut pos = 8;

    for _i in 0..num_slot_hashes {
        let slot = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
        pos += 8;
        let hash = &data[pos..pos + 32];
        pos += 32;

        if slot == 54943128 {
            msg!("Found slot {}, hash {}", slot, Hash::new(hash));
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use {
        super::*,
        assert_matches::*,
        solana_program::{
            instruction::{AccountMeta, Instruction},
            native_token::sol_to_lamports,
            sysvar,
        },
        solana_program_test::*,
        solana_sdk::{signature::Signer, transaction::Transaction},
    };

    #[tokio::test]
    async fn test_transaction() {
        let program_id = Pubkey::new_unique();

        let mut program_test = ProgramTest::new(
            "bpf_program_template",
            program_id,
            processor!(process_instruction),
        );

        // Replace the SlotHashes sysvar will a fully populated version that was grabbed off Mainnet
        // Beta by running:
        //      solana account SysvarS1otHashes111111111111111111111111111 -o slot_hashes.bin
        program_test.add_account_with_file_data(
            sysvar::slot_hashes::id(),
            sol_to_lamports(1.),
            Pubkey::default(),
            "slot_hashes.bin",
        );

        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

        let mut transaction = Transaction::new_with_payer(
            &[Instruction {
                program_id,
                accounts: vec![AccountMeta::new(sysvar::slot_hashes::id(), false)],
                data: vec![1, 2, 3],
            }],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&payer], recent_blockhash);

        assert_matches!(banks_client.process_transaction(transaction).await, Ok(()));
    }
}