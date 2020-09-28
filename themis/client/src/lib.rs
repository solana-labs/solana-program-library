//! Themis client

#[cfg(test)]
mod tests {
    use bn::{G1, Group, Fr};
    use elgamal_bn::ciphertext::Ciphertext;
    use solana_banks_client::{start_client, BanksClient, BanksClientExt};
    use solana_banks_server::banks_server::start_local_server;
    use solana_runtime::{bank::Bank, bank_forks::BankForks};
    use solana_sdk::{
        account::{Account, KeyedAccount},
        account_info::AccountInfo,
        commitment_config::CommitmentLevel,
        genesis_config::create_genesis_config,
        instruction::InstructionError,
        message::Message,
        native_token::sol_to_lamports,
        program_error::ProgramError,
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        transaction::Transaction,
    };
    use spl_themis::{
        instruction,
        processor::process_instruction,
        state::{generate_keys, recover_scalar, User},
    };
    use std::{
        collections::HashMap,
        io,
        sync::{Arc, RwLock},
        {cell::RefCell, rc::Rc},
    };
    use tokio::runtime::Runtime;

    fn to_instruction_error(error: ProgramError) -> InstructionError {
        match error {
            ProgramError::Custom(err) => InstructionError::Custom(err),
            ProgramError::InvalidArgument => InstructionError::InvalidArgument,
            ProgramError::InvalidInstructionData => InstructionError::InvalidInstructionData,
            ProgramError::InvalidAccountData => InstructionError::InvalidAccountData,
            ProgramError::AccountDataTooSmall => InstructionError::AccountDataTooSmall,
            ProgramError::InsufficientFunds => InstructionError::InsufficientFunds,
            ProgramError::IncorrectProgramId => InstructionError::IncorrectProgramId,
            ProgramError::MissingRequiredSignature => InstructionError::MissingRequiredSignature,
            ProgramError::AccountAlreadyInitialized => InstructionError::AccountAlreadyInitialized,
            ProgramError::UninitializedAccount => InstructionError::UninitializedAccount,
            ProgramError::NotEnoughAccountKeys => InstructionError::NotEnoughAccountKeys,
            ProgramError::AccountBorrowFailed => InstructionError::AccountBorrowFailed,
            ProgramError::MaxSeedLengthExceeded => InstructionError::MaxSeedLengthExceeded,
            ProgramError::InvalidSeeds => InstructionError::InvalidSeeds,
        }
    }

    // Same as process_instruction, but but can be used as a builtin program. Handy for unit-testing.
    pub fn process_instruction_native(
        program_id: &Pubkey,
        keyed_accounts: &[KeyedAccount],
        input: &[u8],
    ) -> Result<(), InstructionError> {
        // Copy all the accounts into a HashMap to ensure there are no duplicates
        let mut accounts: HashMap<Pubkey, Account> = keyed_accounts
            .iter()
            .map(|ka| (*ka.unsigned_key(), ka.account.borrow().clone()))
            .collect();

        // Create shared references to each account's lamports/data/owner
        let account_refs: HashMap<_, _> = accounts
            .iter_mut()
            .map(|(key, account)| {
                (
                    *key,
                    (
                        Rc::new(RefCell::new(&mut account.lamports)),
                        Rc::new(RefCell::new(&mut account.data[..])),
                        &account.owner,
                    ),
                )
            })
            .collect();

        // Create AccountInfos
        let account_infos: Vec<AccountInfo> = keyed_accounts
            .iter()
            .map(|keyed_account| {
                let key = keyed_account.unsigned_key();
                let (lamports, data, owner) = &account_refs[key];
                AccountInfo {
                    key,
                    is_signer: keyed_account.signer_key().is_some(),
                    is_writable: keyed_account.is_writable(),
                    lamports: lamports.clone(),
                    data: data.clone(),
                    owner,
                    executable: keyed_account.executable().unwrap(),
                    rent_epoch: keyed_account.rent_epoch().unwrap(),
                }
            })
            .collect();

        // Execute the BPF entrypoint
        process_instruction(program_id, &account_infos, input).map_err(to_instruction_error)?;

        // Commit changes to the KeyedAccounts
        for keyed_account in keyed_accounts {
            let mut account = keyed_account.account.borrow_mut();
            let key = keyed_account.unsigned_key();
            let (lamports, data, _owner) = &account_refs[key];
            account.lamports = **lamports.borrow();
            account.data = data.borrow().to_vec();
        }

        Ok(())
    }

    async fn test_e2e(
        client: &mut BanksClient,
        sender_keypair: Keypair,
        policies: Vec<Fr>,
        expected_scalar_aggregate: Fr,
    ) -> io::Result<()> {
        let (sk, pk) = generate_keys();
        let interactions: Vec<_> = policies
            .iter()
            .map(|_| pk.encrypt(&G1::one()).points)
            .collect();

        let sender_pubkey = sender_keypair.pubkey();
        let policies_keypair = Keypair::new();
        let policies_pubkey = policies_keypair.pubkey();
        let user_keypair = Keypair::new();
        let user_pubkey = user_keypair.pubkey();

        let mut ixs = instruction::create_policies_account(
            &sender_pubkey,
            &policies_pubkey,
            sol_to_lamports(1.0),
            policies,
        );
        ixs.append(&mut instruction::create_user_account(
            &sender_pubkey,
            &user_pubkey,
            sol_to_lamports(1.0),
        ));
        let msg = Message::new(&ixs, Some(&sender_keypair.pubkey()));
        let recent_blockhash = client.get_recent_blockhash().await?;
        let tx = Transaction::new(
            &[&sender_keypair, &policies_keypair, &user_keypair],
            msg,
            recent_blockhash,
        );
        let tx_size = bincode::serialize(&tx).unwrap().len();
        assert!(tx_size <= 1200, "transaction over 1200 bytes: {} bytes", tx_size);
        client
            .process_transaction_with_commitment(tx, CommitmentLevel::Recent)
            .await
            .unwrap();

        let ix = instruction::calculate_aggregate(&user_pubkey, &policies_pubkey, interactions, pk);
        let msg = Message::new(&[ix], Some(&sender_keypair.pubkey()));
        let recent_blockhash = client.get_recent_blockhash().await?;
        let tx = Transaction::new(&[&sender_keypair, &user_keypair], msg, recent_blockhash);
        let tx_size = bincode::serialize(&tx).unwrap().len();
        assert!(tx_size <= 1200, "transaction over 1200 bytes: {} bytes", tx_size);
        client
            .process_transaction_with_commitment(tx, CommitmentLevel::Recent)
            .await
            .unwrap();

        let user_account = client.get_account(user_pubkey).await.unwrap().unwrap();
        let user = User::deserialize(&user_account.data).unwrap();
        let ciphertext = Ciphertext {
            points: user.fetch_encrypted_aggregate(),
            pk,
        };

        let decrypted_aggregate = sk.decrypt(&ciphertext);
        let scalar_aggregate = recover_scalar(decrypted_aggregate, 16);
        assert_eq!(scalar_aggregate, expected_scalar_aggregate);

        let ((announcement_g, announcement_ctx), response) =
            sk.prove_correct_decryption_no_Merlin(&ciphertext, &decrypted_aggregate).unwrap();

        let ix = instruction::submit_proof_decryption(
            &user_pubkey,
            decrypted_aggregate,
            announcement_g,
            announcement_ctx,
            response,
        );
        let msg = Message::new(&[ix], Some(&sender_keypair.pubkey()));
        let recent_blockhash = client.get_recent_blockhash().await?;
        let tx = Transaction::new(&[&sender_keypair, &user_keypair], msg, recent_blockhash);
        let tx_size = bincode::serialize(&tx).unwrap().len();
        assert!(tx_size <= 1200, "transaction over 1200 bytes: {} bytes", tx_size);
        client
            .process_transaction_with_commitment(tx, CommitmentLevel::Recent)
            .await
            .unwrap();

        let user_account = client.get_account(user_pubkey).await.unwrap().unwrap();
        let user = User::deserialize(&user_account.data).unwrap();
        assert!(user.fetch_proof_verification());
        Ok(())
    }

    #[test]
    fn test_local_e2e_2ads() {
        let (genesis_config, sender_keypair) = create_genesis_config(sol_to_lamports(9_000_000.0));
        let mut bank = Bank::new(&genesis_config);
        bank.add_builtin_program("Themis", spl_themis::id(), process_instruction_native);
        let bank_forks = Arc::new(RwLock::new(BankForks::new(bank)));
        Runtime::new().unwrap().block_on(async {
            let transport = start_local_server(&bank_forks).await;
            let mut banks_client = start_client(transport).await.unwrap();
            let policies = vec![Fr::new(1u64.into()).unwrap(), Fr::new(2u64.into()).unwrap()];
            test_e2e(&mut banks_client, sender_keypair, policies, Fr::new(3u64.into()).unwrap())
                .await
                .unwrap();
        });
    }
}
