use solana_client::{client_error::Result as ClientResult, rpc_client::RpcClient};
use solana_core::test_validator::{TestValidator, TestValidatorOptions};
use solana_sdk::{
    bpf_loader,
    commitment_config::CommitmentConfig,
    loader_instruction,
    message::Message,
    native_token::sol_to_lamports,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    system_instruction,
    transaction::Transaction,
};
use spl_themis_ristretto_client::{send_and_confirm_transactions_with_spinner, test_e2e};
use std::{
    fs::{remove_dir_all, File},
    io::Read,
};

const DATA_CHUNK_SIZE: usize = 229; // Keep program chunks under PACKET_DATA_SIZE

fn load_program(name: &str) -> Vec<u8> {
    let mut file = File::open(name).unwrap();

    let mut program = Vec::new();
    file.read_to_end(&mut program).unwrap();
    program
}

fn create_program_account_with_commitment(
    client: &RpcClient,
    loader_id: &Pubkey,
    funder_keypair: &Keypair,
    program_keypair: &Keypair,
    program_len: usize,
    commitment: CommitmentConfig,
) -> ClientResult<Signature> {
    let minimum_balance = 0.01; // TODO: Can we calcualte this from get_fees() and program_len?
    let ix = system_instruction::create_account(
        &funder_keypair.pubkey(),
        &program_keypair.pubkey(),
        sol_to_lamports(minimum_balance),
        program_len as u64,
        loader_id,
    );
    let message = Message::new(&[ix], Some(&funder_keypair.pubkey()));
    let (recent_blockhash, _fee_calculator) = client.get_recent_blockhash()?;
    let transaction = Transaction::new(
        &[funder_keypair, program_keypair],
        message,
        recent_blockhash,
    );
    client.send_and_confirm_transaction_with_spinner_and_commitment(&transaction, commitment)
}

fn write_program_with_commitment(
    client: &RpcClient,
    loader_id: &Pubkey,
    funder_keypair: &Keypair,
    program_keypair: &Keypair,
    program: Vec<u8>,
    commitment: CommitmentConfig,
) -> ClientResult<()> {
    let (recent_blockhash, _fee_calculator, last_valid_slot) = client
        .get_recent_blockhash_with_commitment(CommitmentConfig::default())?
        .value;
    let signer_keys = [funder_keypair, program_keypair];
    let transactions: Vec<_> = program
        .chunks(DATA_CHUNK_SIZE)
        .enumerate()
        .map(|(i, chunk)| {
            let instruction = loader_instruction::write(
                &program_keypair.pubkey(),
                loader_id,
                (i * DATA_CHUNK_SIZE) as u32,
                chunk.to_vec(),
            );
            let message = Message::new(&[instruction], Some(&funder_keypair.pubkey()));
            Transaction::new(&signer_keys, message, recent_blockhash)
        })
        .collect();
    send_and_confirm_transactions_with_spinner(
        client,
        transactions,
        &signer_keys,
        commitment,
        last_valid_slot,
    )
}

fn finalize_program_with_commitment(
    client: &RpcClient,
    loader_id: &Pubkey,
    funder_keypair: &Keypair,
    program_keypair: &Keypair,
    commitment: CommitmentConfig,
) -> ClientResult<Signature> {
    let ix = loader_instruction::finalize(&program_keypair.pubkey(), &loader_id);
    let message = Message::new(&[ix], Some(&funder_keypair.pubkey()));
    let (recent_blockhash, _fee_calculator) = client.get_recent_blockhash()?;
    let transaction = Transaction::new(
        &[funder_keypair, program_keypair],
        message,
        recent_blockhash,
    );
    client.send_and_confirm_transaction_with_spinner_and_commitment(&transaction, commitment)
}

fn deploy_program_with_commitment(
    client: &RpcClient,
    loader_id: &Pubkey,
    funder_keypair: &Keypair,
    program_keypair: &Keypair,
    program: Vec<u8>,
    commitment: CommitmentConfig,
) -> ClientResult<()> {
    create_program_account_with_commitment(
        client,
        loader_id,
        funder_keypair,
        program_keypair,
        program.len(),
        commitment,
    )?;
    write_program_with_commitment(
        client,
        loader_id,
        funder_keypair,
        program_keypair,
        program,
        commitment,
    )?;
    finalize_program_with_commitment(
        client,
        loader_id,
        funder_keypair,
        program_keypair,
        commitment,
    )?;
    Ok(())
}

#[test]
#[ignore]
fn test_validator_e2e() {
    let TestValidator {
        server,
        leader_data,
        alice,
        ledger_path,
        ..
    } = TestValidator::run_with_options(TestValidatorOptions {
        mint_lamports: sol_to_lamports(10.0),
        ..TestValidatorOptions::default()
    });

    let program = load_program("../../target/deploy/spl_themis_ristretto.so");

    let client = RpcClient::new_socket(leader_data.rpc);
    let program_keypair = Keypair::new();
    deploy_program_with_commitment(
        &client,
        &bpf_loader::id(),
        &alice,
        &program_keypair,
        program,
        CommitmentConfig::recent(),
    )
    .unwrap();

    let policies = vec![1u64.into(), 2u64.into()];
    test_e2e(
        &client,
        &program_keypair.pubkey(),
        alice,
        policies,
        10,
        3u64.into(),
    )
    .unwrap();

    // Explicit cleanup, otherwise "pure virtual method called" crash in Docker
    server.close().unwrap();
    remove_dir_all(ledger_path).unwrap();
}
