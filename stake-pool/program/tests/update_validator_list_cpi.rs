#![cfg(feature = "test-bpf")]

mod helpers;

use {
    solana_program::{
        borsh::{get_instance_packed_len, get_packed_len, try_from_slice_unchecked},
        hash::Hash,
        program_pack::Pack,
        program::invoke,
        pubkey::Pubkey,
        system_instruction, system_program,
        instruction::{Instruction, InstructionError, AccountMeta},
        account_info::AccountInfo,
        entrypoint::ProgramResult,
    },
    solana_program_test::*,
    solana_sdk::{
        account::Account,
        signature::{Keypair, Signer},
        transaction::{Transaction, TransactionError},
        transport::TransportError,
        msg,
    },
    solana_vote_program::{
        self, vote_instruction,
        vote_state::{VoteInit, VoteState},
    },
    spl_stake_pool::{
        error::StakePoolError,
        find_stake_program_address, find_transient_stake_program_address, id, instruction,
        processor, stake_program, state,
    },
    std::str::FromStr,
    helpers::{StakePoolAccounts,  ValidatorStakeAccount},
};

pub fn program_test_with_cpi() -> ProgramTest {
    let mut program_test = ProgramTest::new(
        "spl_stake_pool",
        id(),
        processor!(processor::Processor::process),
    );
    program_test.prefer_bpf(false);
    program_test.add_program(
        "proxy_invoker",
        Pubkey::from_str("proxy11111111111111111111111111111111111111").unwrap(),
        processor!(cpi_proxy_invocation_processor),
    );
    program_test
}

// this instruction processes the internal
pub(crate) fn cpi_proxy_invocation_processor(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let ix: Instruction = bincode::deserialize(instruction_data).unwrap();
    let ret = invoke(
        &ix,
        &accounts[..],
    )?;
    Ok(())
}

#[tokio::test]
async fn fail_with_cpi_update_validator_list() {
    let proxy_id = Pubkey::from_str("proxy11111111111111111111111111111111111111").unwrap();
    let mut context = program_test_with_cpi().start_with_context().await;
    let first_normal_slot = context.genesis_config().epoch_schedule.first_normal_slot;
    let mut slot = first_normal_slot;
    context.warp_to_slot(slot).unwrap();

    let reserve_stake_amount = 1_000_000 as u64;
    let stake_pool_accounts = StakePoolAccounts::new();
    stake_pool_accounts
        .initialize_stake_pool(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            reserve_stake_amount + 1,
        )
        .await
        .unwrap();

    let stake_account = ValidatorStakeAccount::new(&stake_pool_accounts.stake_pool.pubkey());
    stake_account
        .create_and_delegate(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &stake_pool_accounts.staker,
        )
        .await;

    let ix = instruction::update_validator_list_balance(
        &spl_stake_pool::id(),
        &stake_pool_accounts.stake_pool.pubkey(),
        &stake_pool_accounts.withdraw_authority,
        &stake_pool_accounts.validator_list.pubkey(),
        &stake_pool_accounts.reserve_stake.pubkey(),
        &[stake_account.stake_account],
        0,
        false,
    );

    let mut accounts = vec![AccountMeta::new_readonly(spl_stake_pool::id(), false)];
    accounts.append(&mut ix.accounts.to_vec());
    let ix_serialized = bincode::serialize(&ix).unwrap();
    let ix_deserialized: Instruction = bincode::deserialize(&ix_serialized[..]).unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[Instruction {
            program_id: proxy_id,
            accounts,
            data: ix_serialized,
        }],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .err()
        .unwrap()
        .unwrap();

    match error {
        TransactionError::InstructionError(_, InstructionError::Custom(error_index)) => {
            let program_error = StakePoolError::InvalidCallingContext as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!(
            "Wrong error occurs while try to update validator list with wrong calling context"
        ),
    }
}
