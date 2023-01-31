use anyhow::Result;
use solana_client::{rpc_client::RpcClient, rpc_config::RpcSendTransactionConfig};
use solana_program::pubkey::Pubkey;
use solana_sdk::{commitment_config::CommitmentConfig, signer::Signer, transaction::Transaction};
use spl_universal_idl_program::state::{buffer_seeds, idl_seeds};

use super::common::{read_keypair_file, with_solana_config, CliOverrides};

pub enum IdlAccountType {
    Idl,
    Buffer,
}

pub fn close_account(
    overrides: CliOverrides,
    program_id: Pubkey,
    account_type: IdlAccountType,
    recipient: Pubkey,
    authority_filepath: &str,
) -> Result<()> {
    with_solana_config(&overrides.clone(), |config| {
        let authority_keypair = read_keypair_file(authority_filepath, "authority")?;

        let idl_account: Pubkey = match account_type {
            IdlAccountType::Buffer => buffer_seeds(&program_id).0,
            IdlAccountType::Idl => idl_seeds(&program_id).0,
        };

        let client = RpcClient::new(&config.rpc_url);
        client
            .get_account_with_commitment(&idl_account, CommitmentConfig::confirmed())?
            .value
            .expect(&format!("Account {idl_account} does not exist"));

        let ix = spl_universal_idl_program::instructions::close(
            config.idl_program,
            idl_account,
            recipient,
            authority_keypair.pubkey(),
        )?;

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&authority_keypair.pubkey()),
            &[&authority_keypair],
            client.get_latest_blockhash()?,
        );
        client.send_and_confirm_transaction_with_spinner_and_config(
            &tx,
            CommitmentConfig::processed(),
            RpcSendTransactionConfig {
                skip_preflight: false,
                ..RpcSendTransactionConfig::default()
            },
        )?;

        Ok(())
    })
}
