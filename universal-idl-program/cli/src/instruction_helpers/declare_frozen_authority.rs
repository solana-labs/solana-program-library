use anyhow::{anyhow, Result};
use solana_client::{rpc_client::RpcClient, rpc_config::RpcSendTransactionConfig};
use solana_program::pubkey::Pubkey;
use solana_sdk::{commitment_config::CommitmentConfig, signer::Signer, transaction::Transaction};

use super::common::{with_solana_config, CliOverrides};

pub fn declare_frozen_authority(
    overrides: CliOverrides,
    program_id: Pubkey,
    new_program_authority: Pubkey,
    payer: String,
) -> Result<()> {
    with_solana_config(&overrides, |config| {
        let client = RpcClient::new(&config.rpc_url);

        let payer_keypair = solana_sdk::signature::read_keypair_file(payer)
            .map_err(|_| anyhow!("Unable to read payer keypair file"))?;

        let ix = spl_universal_idl_program::instructions::declare_frozen_authority(
            config.idl_program,
            program_id,
            payer_keypair.pubkey(),
            payer_keypair.pubkey(),
            new_program_authority,
        )?;

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&payer_keypair.pubkey()),
            &[&payer_keypair],
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
