use solana_banks_client::{start_tcp_client, BanksClient, BanksClientExt};
use solana_core::test_validator::{TestValidator, TestValidatorOptions};
use solana_sdk::{
    bpf_loader,
    commitment_config::CommitmentLevel,
    loader_instruction,
    message::Message,
    native_token::sol_to_lamports,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
    transport,
};
use spl_themis_ristretto_client::{process_transactions_with_commitment, test_e2e};
use std::{
    fs::{remove_dir_all, File},
    io::Read,
};
use tokio::runtime::Runtime;

const DATA_CHUNK_SIZE: usize = 229; // Keep program chunks under PACKET_DATA_SIZE

fn load_program(name: &str) -> Vec<u8> {
    let mut file = File::open(name).unwrap();

    let mut program = Vec::new();
    file.read_to_end(&mut program).unwrap();
    program
}

async fn create_program_account_with_commitment(
    client: &mut BanksClient,
    loader_id: &Pubkey,
    funder_keypair: &Keypair,
    program_keypair: &Keypair,
    program_len: usize,
    commitment: CommitmentLevel,
) -> transport::Result<()> {
    let minimum_balance = 0.01; // TODO: Can we calcualte this from get_fees() and program_len?
    let ix = system_instruction::create_account(
        &funder_keypair.pubkey(),
        &program_keypair.pubkey(),
        sol_to_lamports(minimum_balance),
        program_len as u64,
        loader_id,
    );
    let message = Message::new(&[ix], Some(&funder_keypair.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().await?;
    let transaction = Transaction::new(
        &[funder_keypair, program_keypair],
        message,
        recent_blockhash,
    );
    client
        .process_transaction_with_commitment(transaction, commitment)
        .await
}

async fn write_program_with_commitment(
    client: &mut BanksClient,
    loader_id: &Pubkey,
    funder_keypair: &Keypair,
    program_keypair: &Keypair,
    program: Vec<u8>,
    commitment: CommitmentLevel,
) -> transport::Result<()> {
    let recent_blockhash = client.get_recent_blockhash().await?;
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
            Transaction::new(
                &[funder_keypair, program_keypair],
                message,
                recent_blockhash,
            )
        })
        .collect();
    process_transactions_with_commitment(client, transactions, commitment).await
}

async fn finalize_program_with_commitment(
    client: &mut BanksClient,
    loader_id: &Pubkey,
    funder_keypair: &Keypair,
    program_keypair: &Keypair,
    commitment: CommitmentLevel,
) -> transport::Result<()> {
    let ix = loader_instruction::finalize(&program_keypair.pubkey(), &loader_id);
    let message = Message::new(&[ix], Some(&funder_keypair.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().await?;
    let transaction = Transaction::new(
        &[funder_keypair, program_keypair],
        message,
        recent_blockhash,
    );
    client
        .process_transaction_with_commitment(transaction, commitment)
        .await
}

// TODO: Add this to BanksClient
async fn deploy_program_with_commitment(
    client: &mut BanksClient,
    loader_id: &Pubkey,
    funder_keypair: &Keypair,
    program_keypair: &Keypair,
    program: Vec<u8>,
    commitment: CommitmentLevel,
) -> transport::Result<()> {
    create_program_account_with_commitment(
        client,
        loader_id,
        funder_keypair,
        program_keypair,
        program.len(),
        commitment,
    )
    .await?;
    write_program_with_commitment(
        client,
        loader_id,
        funder_keypair,
        program_keypair,
        program,
        commitment,
    )
    .await?;
    finalize_program_with_commitment(
        client,
        loader_id,
        funder_keypair,
        program_keypair,
        commitment,
    )
    .await?;
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

    Runtime::new().unwrap().block_on(async {
        let mut banks_client = start_tcp_client(leader_data.rpc_banks).await.unwrap();
        let program_keypair = Keypair::new();
        deploy_program_with_commitment(
            &mut banks_client,
            &bpf_loader::id(),
            &alice,
            &program_keypair,
            program,
            CommitmentLevel::Recent,
        )
        .await
        .unwrap();

        let policies = vec![1u64.into(), 2u64.into()];
        test_e2e(
            &mut banks_client,
            &program_keypair.pubkey(),
            alice,
            policies,
            10,
            3u64.into(),
        )
        .await
        .unwrap();
    });

    // Explicit cleanup, otherwise "pure virtual method called" crash in Docker
    server.close().unwrap();
    remove_dir_all(ledger_path).unwrap();
}
