use anyhow::Result;
use solana_client::{rpc_client::RpcClient, rpc_config::RpcSendTransactionConfig};
use solana_program::pubkey::Pubkey;
use solana_sdk::{commitment_config::CommitmentConfig, signer::Signer, transaction::Transaction};
use spl_universal_idl_program::state::{buffer_seeds, idl_seeds};

use super::common::{read_keypair_file, with_solana_config, CliOverrides};

pub fn set_buffer(
    overrides: CliOverrides,
    program_id: Pubkey,
    payer_filepath: &str,
    authority_filepath: &str,
) -> Result<()> {
    with_solana_config(&overrides, |config| {
        let payer_keypair = read_keypair_file(payer_filepath, "payer")?;
        let authority_keypair = read_keypair_file(authority_filepath, "authority")?;

        let idl_account = idl_seeds(&program_id).0;
        let buffer_account = buffer_seeds(&program_id).0;

        let client = RpcClient::new(&config.rpc_url);

        let ix = spl_universal_idl_program::instructions::set_buffer(
            config.idl_program,
            payer_keypair.pubkey(),
            authority_keypair.pubkey(),
            idl_account,
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
