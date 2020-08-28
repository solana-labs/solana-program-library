//! budget program
use crate::{
    expr::Witness,
    instruction::{BudgetError, BudgetInstruction},
    state::BudgetState,
};
use chrono::prelude::{DateTime, Utc};
use log::*;
use solana_sdk::{
    account_info::{next_account_info, AccountInfo},
    hash::hash,
    program_error::ProgramError,
    //program_utils::limited_deserialize,
    pubkey::Pubkey,
};

/// Process a Witness Signature. Any payment plans waiting on this signature
/// will progress one step.
fn apply_signature(
    budget_state: &mut BudgetState,
    witness_account_info: &AccountInfo,
    contract_account_info: &AccountInfo,
    to_account_info: Result<&AccountInfo, ProgramError>,
) -> Result<(), ProgramError> {
    let mut final_payment = None;
    if let Some(ref mut expr) = budget_state.pending_budget {
        let key = witness_account_info.signer_key().unwrap();
        expr.apply_witness(&Witness::Signature, key);
        final_payment = expr.final_payment();
    }

    if let Some(payment) = final_payment {
        if let Some(key) = witness_account_info.signer_key() {
            if &payment.to == key {
                budget_state.pending_budget = None;
                **contract_account_info.lamports.borrow_mut() -= payment.lamports;
                **witness_account_info.lamports.borrow_mut() += payment.lamports;
                return Ok(());
            }
        }
        let to_account_info = to_account_info?;
        if &payment.to != to_account_info.unsigned_key() {
            trace!("destination missing");
            return Err(BudgetError::DestinationMissing.into());
        }
        budget_state.pending_budget = None;
        **contract_account_info.lamports.borrow_mut() -= payment.lamports;
        **to_account_info.lamports.borrow_mut() += payment.lamports;
    }
    Ok(())
}

/// Process a Witness Timestamp. Any payment plans waiting on this timestamp
/// will progress one step.
fn apply_timestamp(
    budget_state: &mut BudgetState,
    witness_account_info: &AccountInfo,
    contract_account_info: &AccountInfo,
    to_account_info: Result<&AccountInfo, ProgramError>,
    dt: DateTime<Utc>,
) -> Result<(), ProgramError> {
    // Check to see if any timelocked transactions can be completed.
    let mut final_payment = None;

    if let Some(ref mut expr) = budget_state.pending_budget {
        let key = witness_account_info.signer_key().unwrap();
        expr.apply_witness(&Witness::Timestamp(dt), key);
        final_payment = expr.final_payment();
    }

    if let Some(payment) = final_payment {
        let to_account_info = to_account_info?;
        if &payment.to != to_account_info.unsigned_key() {
            trace!("destination missing");
            return Err(BudgetError::DestinationMissing.into());
        }
        budget_state.pending_budget = None;
        **contract_account_info.lamports.borrow_mut() -= payment.lamports;
        **to_account_info.lamports.borrow_mut() += payment.lamports;
    }
    Ok(())
}

/// Process an AccountData Witness and any payment waiting on it.
fn apply_account_data(
    budget_state: &mut BudgetState,
    witness_account_info: &AccountInfo,
    contract_account_info: &AccountInfo,
    to_account_info: Result<&AccountInfo, ProgramError>,
) -> Result<(), ProgramError> {
    // Check to see if any timelocked transactions can be completed.
    let mut final_payment = None;

    if let Some(ref mut expr) = budget_state.pending_budget {
        let key = witness_account_info.unsigned_key();
        let program_id = witness_account_info.owner;
        let actual_hash = hash(&witness_account_info.data.borrow());
        expr.apply_witness(&Witness::AccountData(actual_hash, *program_id), key);
        final_payment = expr.final_payment();
    }

    if let Some(payment) = final_payment {
        let to_account_info = to_account_info?;
        if &payment.to != to_account_info.unsigned_key() {
            trace!("destination missing");
            return Err(BudgetError::DestinationMissing.into());
        }
        budget_state.pending_budget = None;
        **contract_account_info.lamports.borrow_mut() -= payment.lamports;
        **to_account_info.lamports.borrow_mut() += payment.lamports;
    }
    Ok(())
}

pub fn process_instruction<'a>(
    _program_id: &Pubkey,
    account_infos: &'a [AccountInfo<'a>],
    data: &[u8],
) -> Result<(), ProgramError> {
    let account_infos_iter = &mut account_infos.iter();
    //let instruction = limited_deserialize(data)?;
    let instruction = bincode::deserialize(data).map_err(|_| ProgramError::InvalidInstructionData)?;

    trace!("process_instruction: {:?}", instruction);

    match instruction {
        BudgetInstruction::InitializeAccount(expr) => {
            let contract_account_info = next_account_info(account_infos_iter)?;

            if let Some(payment) = expr.final_payment() {
                let to_account_info = contract_account_info;
                let contract_account_info = next_account_info(account_infos_iter)?;
                **contract_account_info.lamports.borrow_mut() = 0;
                **to_account_info.lamports.borrow_mut() += payment.lamports;
                return Ok(());
            }
            let existing = BudgetState::deserialize(&contract_account_info.data.borrow()).ok();
            if Some(true) == existing.map(|x| x.initialized) {
                trace!("contract already exists");
                return Err(ProgramError::AccountAlreadyInitialized);
            }
            let mut budget_state = BudgetState::default();
            budget_state.pending_budget = Some(*expr);
            budget_state.initialized = true;
            budget_state.serialize(&mut contract_account_info.data.borrow_mut())
        }
        BudgetInstruction::ApplyTimestamp(dt) => {
            let witness_account_info = next_account_info(account_infos_iter)?;
            let contract_account_info = next_account_info(account_infos_iter)?;
            let mut budget_state = BudgetState::deserialize(&contract_account_info.data.borrow())?;
            if !budget_state.is_pending() {
                return Ok(()); // Nothing to do here.
            }
            if !budget_state.initialized {
                trace!("contract is uninitialized");
                return Err(ProgramError::UninitializedAccount);
            }
            if witness_account_info.signer_key().is_none() {
                return Err(ProgramError::MissingRequiredSignature);
            }
            trace!("apply timestamp");
            apply_timestamp(
                &mut budget_state,
                witness_account_info,
                contract_account_info,
                next_account_info(account_infos_iter),
                dt,
            )?;
            trace!("apply timestamp committed");
            budget_state.serialize(&mut contract_account_info.data.borrow_mut())
        }
        BudgetInstruction::ApplySignature => {
            let witness_account_info = next_account_info(account_infos_iter)?;
            let contract_account_info = next_account_info(account_infos_iter)?;
            let mut budget_state = BudgetState::deserialize(&contract_account_info.data.borrow())?;
            if !budget_state.is_pending() {
                return Ok(()); // Nothing to do here.
            }
            if !budget_state.initialized {
                trace!("contract is uninitialized");
                return Err(ProgramError::UninitializedAccount);
            }
            if witness_account_info.signer_key().is_none() {
                return Err(ProgramError::MissingRequiredSignature);
            }
            trace!("apply signature");
            apply_signature(
                &mut budget_state,
                witness_account_info,
                contract_account_info,
                next_account_info(account_infos_iter),
            )?;
            trace!("apply signature committed");
            budget_state.serialize(&mut contract_account_info.data.borrow_mut())
        }
        BudgetInstruction::ApplyAccountData => {
            let witness_account_info = next_account_info(account_infos_iter)?;
            let contract_account_info = next_account_info(account_infos_iter)?;
            let mut budget_state = BudgetState::deserialize(&contract_account_info.data.borrow())?;
            if !budget_state.is_pending() {
                return Ok(()); // Nothing to do here.
            }
            if !budget_state.initialized {
                trace!("contract is uninitialized");
                return Err(ProgramError::UninitializedAccount);
            }
            apply_account_data(
                &mut budget_state,
                witness_account_info,
                contract_account_info,
                next_account_info(account_infos_iter),
            )?;
            trace!("apply account data committed");
            budget_state.serialize(&mut contract_account_info.data.borrow_mut())
        }
    }
}

#[cfg(test)]
mod tests {
    //    use super::*;
    //    use crate::{id, instruction};
    //    use solana_runtime::{bank::Bank, bank_client::BankClient};
    //    use solana_sdk::{
    //        account::Account,
    //        client::SyncClient,
    //        genesis_config::create_genesis_config,
    //        hash::hash,
    //        instruction::InstructionError,
    //        message::Message,
    //        signature::{Keypair, Signer},
    //        transaction::TransactionError,
    //    };
    //
    //    fn create_bank(lamports: u64) -> (Bank, Keypair) {
    //        let (genesis_config, mint_keypair) = create_genesis_config(lamports);
    //        let mut bank = Bank::new(&genesis_config);
    //        bank.add_builtin_program("budget_program", id(), process_instruction);
    //        (bank, mint_keypair)
    //    }
    //
    //    #[test]
    //    fn test_initialize_no_panic() {
    //        let (bank, alice_keypair) = create_bank(1);
    //        let bank_client = BankClient::new(bank);
    //
    //        let alice_pubkey = alice_keypair.pubkey();
    //        let budget_keypair = Keypair::new();
    //        let budget_pubkey = budget_keypair.pubkey();
    //        let bob_pubkey = Pubkey::new_rand();
    //
    //        let mut instructions = instruction::payment(&alice_pubkey, &bob_pubkey, &budget_pubkey, 1);
    //        instructions[1].accounts = vec![]; // <!-- Attack! Prevent accounts from being passed into processor.
    //
    //        let message = Message::new(&instructions, Some(&alice_pubkey));
    //        assert_eq!(
    //            bank_client
    //                .send_and_confirm_message(&[&alice_keypair, &budget_keypair], message)
    //                .unwrap_err()
    //                .unwrap(),
    //            TransactionError::InstructionError(1, InstructionError::NotEnoughAccountKeys)
    //        );
    //    }
    //
    //    #[test]
    //    fn test_budget_payment() {
    //        let (bank, alice_keypair) = create_bank(10_000);
    //        let bank_client = BankClient::new(bank);
    //        let alice_pubkey = alice_keypair.pubkey();
    //        let bob_pubkey = Pubkey::new_rand();
    //        let budget_keypair = Keypair::new();
    //        let budget_pubkey = budget_keypair.pubkey();
    //        let instructions = instruction::payment(&alice_pubkey, &bob_pubkey, &budget_pubkey, 100);
    //        let message = Message::new(&instructions, Some(&alice_pubkey));
    //        bank_client
    //            .send_and_confirm_message(&[&alice_keypair, &budget_keypair], message)
    //            .unwrap();
    //        assert_eq!(bank_client.get_balance(&bob_pubkey).unwrap(), 100);
    //    }
    //
    //    #[test]
    //    fn test_unsigned_witness_key() {
    //        let (bank, alice_keypair) = create_bank(10_000);
    //        let bank_client = BankClient::new(bank);
    //        let alice_pubkey = alice_keypair.pubkey();
    //
    //        // Initialize BudgetState
    //        let budget_keypair = Keypair::new();
    //        let budget_pubkey = budget_keypair.pubkey();
    //        let bob_pubkey = Pubkey::new_rand();
    //        let witness = Pubkey::new_rand();
    //        let instructions = instruction::when_signed(
    //            &alice_pubkey,
    //            &bob_pubkey,
    //            &budget_pubkey,
    //            &witness,
    //            None,
    //            1,
    //        );
    //        let message = Message::new(&instructions, Some(&alice_pubkey));
    //        bank_client
    //            .send_and_confirm_message(&[&alice_keypair, &budget_keypair], message)
    //            .unwrap();
    //
    //        // Attack! Part 1: Sign a witness transaction with a random key.
    //        let mallory_keypair = Keypair::new();
    //        let mallory_pubkey = mallory_keypair.pubkey();
    //        bank_client
    //            .transfer_and_confirm(1, &alice_keypair, &mallory_pubkey)
    //            .unwrap();
    //        let instruction =
    //            instruction::apply_signature(&mallory_pubkey, &budget_pubkey, &bob_pubkey);
    //        let mut message = Message::new(&[instruction], Some(&mallory_pubkey));
    //
    //        // Attack! Part 2: Point the instruction to the expected, but unsigned, key.
    //        message.account_keys.insert(3, alice_pubkey);
    //        message.instructions[0].accounts[0] = 3;
    //        message.instructions[0].program_id_index = 4;
    //
    //        // Ensure the transaction fails because of the unsigned key.
    //        assert_eq!(
    //            bank_client
    //                .send_and_confirm_message(&[&mallory_keypair], message)
    //                .unwrap_err()
    //                .unwrap(),
    //            TransactionError::InstructionError(0, InstructionError::MissingRequiredSignature)
    //        );
    //    }
    //
    //    #[test]
    //    fn test_unsigned_timestamp() {
    //        let (bank, alice_keypair) = create_bank(10_000);
    //        let bank_client = BankClient::new(bank);
    //        let alice_pubkey = alice_keypair.pubkey();
    //
    //        // Initialize BudgetState
    //        let budget_keypair = Keypair::new();
    //        let budget_pubkey = budget_keypair.pubkey();
    //        let bob_pubkey = Pubkey::new_rand();
    //        let dt = Utc::now();
    //        let instructions = instruction::on_date(
    //            &alice_pubkey,
    //            &bob_pubkey,
    //            &budget_pubkey,
    //            dt,
    //            &alice_pubkey,
    //            None,
    //            1,
    //        );
    //        let message = Message::new(&instructions, Some(&alice_pubkey));
    //        bank_client
    //            .send_and_confirm_message(&[&alice_keypair, &budget_keypair], message)
    //            .unwrap();
    //
    //        // Attack! Part 1: Sign a timestamp transaction with a random key.
    //        let mallory_keypair = Keypair::new();
    //        let mallory_pubkey = mallory_keypair.pubkey();
    //        bank_client
    //            .transfer_and_confirm(1, &alice_keypair, &mallory_pubkey)
    //            .unwrap();
    //        let instruction =
    //            instruction::apply_timestamp(&mallory_pubkey, &budget_pubkey, &bob_pubkey, dt);
    //        let mut message = Message::new(&[instruction], Some(&mallory_pubkey));
    //
    //        // Attack! Part 2: Point the instruction to the expected, but unsigned, key.
    //        message.account_keys.insert(3, alice_pubkey);
    //        message.instructions[0].accounts[0] = 3;
    //        message.instructions[0].program_id_index = 4;
    //
    //        // Ensure the transaction fails because of the unsigned key.
    //        assert_eq!(
    //            bank_client
    //                .send_and_confirm_message(&[&mallory_keypair], message)
    //                .unwrap_err()
    //                .unwrap(),
    //            TransactionError::InstructionError(0, InstructionError::MissingRequiredSignature)
    //        );
    //    }
    //
    //    #[test]
    //    fn test_pay_on_date() {
    //        let (bank, alice_keypair) = create_bank(2);
    //        let bank_client = BankClient::new(bank);
    //        let alice_pubkey = alice_keypair.pubkey();
    //        let budget_keypair = Keypair::new();
    //        let budget_pubkey = budget_keypair.pubkey();
    //        let bob_pubkey = Pubkey::new_rand();
    //        let mallory_pubkey = Pubkey::new_rand();
    //        let dt = Utc::now();
    //
    //        let instructions = instruction::on_date(
    //            &alice_pubkey,
    //            &bob_pubkey,
    //            &budget_pubkey,
    //            dt,
    //            &alice_pubkey,
    //            None,
    //            1,
    //        );
    //        let message = Message::new(&instructions, Some(&alice_pubkey));
    //        bank_client
    //            .send_and_confirm_message(&[&alice_keypair, &budget_keypair], message)
    //            .unwrap();
    //        assert_eq!(bank_client.get_balance(&alice_pubkey).unwrap(), 1);
    //        assert_eq!(bank_client.get_balance(&budget_pubkey).unwrap(), 1);
    //
    //        let contract_account = bank_client
    //            .get_account_data(&budget_pubkey)
    //            .unwrap()
    //            .unwrap();
    //        let budget_state = BudgetState::deserialize(&contract_account).unwrap();
    //        assert!(budget_state.is_pending());
    //
    //        // Attack! Try to payout to mallory_pubkey
    //        let instruction =
    //            instruction::apply_timestamp(&alice_pubkey, &budget_pubkey, &mallory_pubkey, dt);
    //        assert_eq!(
    //            bank_client
    //                .send_and_confirm_instruction(&alice_keypair, instruction)
    //                .unwrap_err()
    //                .unwrap(),
    //            TransactionError::InstructionError(
    //                0,
    //                InstructionError::Custom(BudgetError::DestinationMissing as u32)
    //            )
    //        );
    //        assert_eq!(bank_client.get_balance(&alice_pubkey).unwrap(), 1);
    //        assert_eq!(bank_client.get_balance(&budget_pubkey).unwrap(), 1);
    //        assert_eq!(bank_client.get_balance(&bob_pubkey).unwrap(), 0);
    //
    //        let contract_account = bank_client
    //            .get_account_data(&budget_pubkey)
    //            .unwrap()
    //            .unwrap();
    //        let budget_state = BudgetState::deserialize(&contract_account).unwrap();
    //        assert!(budget_state.is_pending());
    //
    //        // Now, acknowledge the time in the condition occurred and
    //        // that pubkey's funds are now available.
    //        let instruction =
    //            instruction::apply_timestamp(&alice_pubkey, &budget_pubkey, &bob_pubkey, dt);
    //        bank_client
    //            .send_and_confirm_instruction(&alice_keypair, instruction)
    //            .unwrap();
    //        assert_eq!(bank_client.get_balance(&alice_pubkey).unwrap(), 1);
    //        assert_eq!(bank_client.get_balance(&budget_pubkey).unwrap(), 0);
    //        assert_eq!(bank_client.get_balance(&bob_pubkey).unwrap(), 1);
    //        assert_eq!(bank_client.get_account_data(&budget_pubkey).unwrap(), None);
    //    }
    //
    //    #[test]
    //    fn test_cancel_payment() {
    //        let (bank, alice_keypair) = create_bank(3);
    //        let bank_client = BankClient::new(bank);
    //        let alice_pubkey = alice_keypair.pubkey();
    //        let budget_keypair = Keypair::new();
    //        let budget_pubkey = budget_keypair.pubkey();
    //        let bob_pubkey = Pubkey::new_rand();
    //        let dt = Utc::now();
    //
    //        let instructions = instruction::on_date(
    //            &alice_pubkey,
    //            &bob_pubkey,
    //            &budget_pubkey,
    //            dt,
    //            &alice_pubkey,
    //            Some(alice_pubkey),
    //            1,
    //        );
    //        let message = Message::new(&instructions, Some(&alice_pubkey));
    //        bank_client
    //            .send_and_confirm_message(&[&alice_keypair, &budget_keypair], message)
    //            .unwrap();
    //        assert_eq!(bank_client.get_balance(&alice_pubkey).unwrap(), 2);
    //        assert_eq!(bank_client.get_balance(&budget_pubkey).unwrap(), 1);
    //
    //        let contract_account = bank_client
    //            .get_account_data(&budget_pubkey)
    //            .unwrap()
    //            .unwrap();
    //        let budget_state = BudgetState::deserialize(&contract_account).unwrap();
    //        assert!(budget_state.is_pending());
    //
    //        // Attack! try to put the lamports into the wrong account with cancel
    //        let mallory_keypair = Keypair::new();
    //        let mallory_pubkey = mallory_keypair.pubkey();
    //        bank_client
    //            .transfer_and_confirm(1, &alice_keypair, &mallory_pubkey)
    //            .unwrap();
    //        assert_eq!(bank_client.get_balance(&alice_pubkey).unwrap(), 1);
    //
    //        let instruction =
    //            instruction::apply_signature(&mallory_pubkey, &budget_pubkey, &bob_pubkey);
    //        bank_client
    //            .send_and_confirm_instruction(&mallory_keypair, instruction)
    //            .unwrap();
    //        // nothing should be changed because apply witness didn't finalize a payment
    //        assert_eq!(bank_client.get_balance(&alice_pubkey).unwrap(), 1);
    //        assert_eq!(bank_client.get_balance(&budget_pubkey).unwrap(), 1);
    //        assert_eq!(bank_client.get_account_data(&bob_pubkey).unwrap(), None);
    //
    //        // Now, cancel the transaction. mint gets her funds back
    //        let instruction =
    //            instruction::apply_signature(&alice_pubkey, &budget_pubkey, &alice_pubkey);
    //        bank_client
    //            .send_and_confirm_instruction(&alice_keypair, instruction)
    //            .unwrap();
    //        assert_eq!(bank_client.get_balance(&alice_pubkey).unwrap(), 2);
    //        assert_eq!(bank_client.get_account_data(&budget_pubkey).unwrap(), None);
    //        assert_eq!(bank_client.get_account_data(&bob_pubkey).unwrap(), None);
    //    }
    //
    //    #[test]
    //    fn test_pay_when_account_data() {
    //        let (bank, alice_keypair) = create_bank(42);
    //        let game_pubkey = Pubkey::new_rand();
    //        let game_account = Account {
    //            lamports: 1,
    //            data: vec![1, 2, 3],
    //            ..Account::default()
    //        };
    //        bank.store_account(&game_pubkey, &game_account);
    //        assert_eq!(bank.get_account(&game_pubkey).unwrap().data, vec![1, 2, 3]);
    //
    //        let bank_client = BankClient::new(bank);
    //
    //        let alice_pubkey = alice_keypair.pubkey();
    //        let game_hash = hash(&[1, 2, 3]);
    //        let budget_keypair = Keypair::new();
    //        let budget_pubkey = budget_keypair.pubkey();
    //        let bob_keypair = Keypair::new();
    //        let bob_pubkey = bob_keypair.pubkey();
    //
    //        // Give Bob some lamports so he can sign the witness transaction.
    //        bank_client
    //            .transfer_and_confirm(1, &alice_keypair, &bob_pubkey)
    //            .unwrap();
    //
    //        let instructions = instruction::when_account_data(
    //            &alice_pubkey,
    //            &bob_pubkey,
    //            &budget_pubkey,
    //            &game_pubkey,
    //            &game_account.owner,
    //            game_hash,
    //            41,
    //        );
    //        let message = Message::new(&instructions, Some(&alice_pubkey));
    //        bank_client
    //            .send_and_confirm_message(&[&alice_keypair, &budget_keypair], message)
    //            .unwrap();
    //        assert_eq!(bank_client.get_balance(&alice_pubkey).unwrap(), 0);
    //        assert_eq!(bank_client.get_balance(&budget_pubkey).unwrap(), 41);
    //
    //        let contract_account = bank_client
    //            .get_account_data(&budget_pubkey)
    //            .unwrap()
    //            .unwrap();
    //        let budget_state = BudgetState::deserialize(&contract_account).unwrap();
    //        assert!(budget_state.is_pending());
    //
    //        // Acknowledge the condition occurred and that Bob's funds are now available.
    //        let instruction =
    //            instruction::apply_account_data(&game_pubkey, &budget_pubkey, &bob_pubkey);
    //
    //        // Anyone can sign the message, but presumably it's Bob, since he's the
    //        // one claiming the payout.
    //        let message = Message::new(&[instruction], Some(&bob_pubkey));
    //        bank_client
    //            .send_and_confirm_message(&[&bob_keypair], message)
    //            .unwrap();
    //
    //        assert_eq!(bank_client.get_balance(&alice_pubkey).unwrap(), 0);
    //        assert_eq!(bank_client.get_balance(&budget_pubkey).unwrap(), 0);
    //        assert_eq!(bank_client.get_balance(&bob_pubkey).unwrap(), 42);
    //        assert_eq!(bank_client.get_account_data(&budget_pubkey).unwrap(), None);
    //    }
}
