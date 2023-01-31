use anyhow::Result;
use solana_client::{rpc_client::RpcClient, rpc_config::RpcSendTransactionConfig};
use solana_program::pubkey::Pubkey;
use solana_sdk::{commitment_config::CommitmentConfig, signer::Signer, transaction::Transaction};
use spl_universal_idl_program::state::idl_seeds;

use super::common::{read_keypair_file, with_solana_config, CliOverrides};

pub fn set_authority(
    overrides: CliOverrides,
    program_id: Pubkey,
    new_authority: Pubkey,
    authority_filepath: &str,
) -> Result<()> {
    with_solana_config(&overrides.clone(), |config| {
        let client = RpcClient::new(&config.rpc_url);
        let authority_keypair = read_keypair_file(authority_filepath, "authority")?;
        let idl_account = idl_seeds(&program_id).0;
        let ix = spl_universal_idl_program::instructions::set_authority(
            config.idl_program,
            idl_account,
            program_id,
            authority_keypair.pubkey(),
            new_authority,
        )?;

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&authority_keypair.pubkey()),
            &[&authority_keypair],
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
