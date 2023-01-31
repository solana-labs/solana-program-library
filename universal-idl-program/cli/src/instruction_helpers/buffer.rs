use anyhow::Result;
use solana_client::{rpc_client::RpcClient, rpc_config::RpcSendTransactionConfig};
use solana_program::pubkey::Pubkey;
use solana_sdk::{commitment_config::CommitmentConfig, signer::Signer, transaction::Transaction};
use spl_universal_idl_program::state::idl_seeds;

use super::common::{read_keypair_file, with_solana_config, CliOverrides};

pub fn buffer_init(
    overrides: CliOverrides,
    program_id: Pubkey,
    buffer_account: Pubkey,
    payer_filepath: &str,
    authority_filepath: &str,
) -> Result<()> {
    with_solana_config(&overrides, |config| {
        let client = RpcClient::new(&config.rpc_url);

        let payer_keypair = read_keypair_file(payer_filepath, "payer")?;
        let authority_keypair = read_keypair_file(authority_filepath, "authority")?;

        let idl_account = idl_seeds(&program_id).0;
        let ix = spl_universal_idl_program::instructions::create_buffer(
            config.idl_program,
            payer_keypair.pubkey(),
            idl_account,
            authority_keypair.pubkey(),
            buffer_account,
            program_id,
        )?;
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&payer_keypair.pubkey()),
            &[&payer_keypair, &authority_keypair],
            client.get_latest_blockhash()?,
        );
        client.send_and_confirm_transaction_with_spinner_and_config(
            &tx,
            CommitmentConfig::confirmed(),
            RpcSendTransactionConfig {
                skip_preflight: true,
                ..RpcSendTransactionConfig::default()
            },
        )?;

        Ok(())
    })
}

pub fn buffer_extend(
    overrides: CliOverrides,
    program_id: Pubkey,
    buffer_account: Pubkey,
    payer_filepath: &str,
    authority_filepath: &str,
    idl_data: Vec<u8>,
) -> Result<()> {
    with_solana_config(&overrides, |config| {
        let client = RpcClient::new(&config.rpc_url);
        let payer_keypair = read_keypair_file(payer_filepath, "payer")?;
        let authority_keypair = read_keypair_file(authority_filepath, "authority")?;

        const MAX_WRITE_SIZE: usize = 900;

        let mut offset = 0;
        while offset < idl_data.len() {
            let start = offset;
            let end = std::cmp::min(offset + MAX_WRITE_SIZE, idl_data.len());

            println!("program id is: {} ", program_id);
            let ix = spl_universal_idl_program::instructions::extend(
                config.idl_program,
                buffer_account,
                payer_keypair.pubkey(),
                authority_keypair.pubkey(),
                program_id,
                idl_data[start..end].to_vec(),
            )?;

            let latest_hash = client.get_latest_blockhash()?;
            let tx = Transaction::new_signed_with_payer(
                &[ix],
                Some(&payer_keypair.pubkey()),
                &[&payer_keypair, &authority_keypair],
                latest_hash,
            );
            client.send_and_confirm_transaction_with_spinner_and_config(
                &tx,
                CommitmentConfig::confirmed(),
                RpcSendTransactionConfig {
                    skip_preflight: true,
                    ..RpcSendTransactionConfig::default()
                },
            )?;
            offset += MAX_WRITE_SIZE;
        }

        Ok(())
    })
}
