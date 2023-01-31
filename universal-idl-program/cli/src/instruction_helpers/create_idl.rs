use super::{
    common::{read_keypair_file, with_solana_config, CliOverrides},
    upgrade,
};
use anyhow::{anyhow, Result};
use solana_client::{rpc_client::RpcClient, rpc_config::RpcSendTransactionConfig};
use solana_program::{
    bpf_loader_upgradeable::{self, UpgradeableLoaderState},
    pubkey::Pubkey,
};
use solana_sdk::{commitment_config::CommitmentConfig, signer::Signer, transaction::Transaction};
use spl_universal_idl_program::state::{frozen_authority_seeds, idl_seeds};

/// Derive the program authority from the program id
/// It may be the upgradeable program authority or it may be a
/// declared frozen authority
fn derive_program_authority(client: &RpcClient, program_id: Pubkey) -> Result<Pubkey> {
    let program_account_info = client.get_account(&program_id)?;

    if program_account_info.owner == bpf_loader_upgradeable::id() {
        let program: UpgradeableLoaderState = bincode::deserialize(&program_account_info.data)?;
        if let UpgradeableLoaderState::Program {
            programdata_address,
        } = program
        {
            Ok(programdata_address)
        } else {
            Err(anyhow!(
                "Program could not be parsed as an upgradeable program"
            ))
        }
    } else {
        Ok(frozen_authority_seeds(&program_id).0)
    }
}

pub fn create_idl(
    overrides: CliOverrides,
    program_id: Pubkey,
    payer_filepath: &str,
    program_authority_filepath: &str,
    idl_filepath: String,
) -> Result<()> {
    with_solana_config(&overrides.clone(), |cli_config| {
        let client = RpcClient::new(&cli_config.rpc_url);

        let payer_keypair = read_keypair_file(payer_filepath, "payer")?;
        let program_authority_keypair =
            read_keypair_file(program_authority_filepath, "program authority")?;

        let authority_verifying_account = derive_program_authority(&client, program_id)?;

        // Init the IDL account with authority
        let idl_account = idl_seeds(&program_id).0;
        let ix = spl_universal_idl_program::instructions::create_idl(
            cli_config.idl_program,
            payer_keypair.pubkey(),
            program_authority_keypair.pubkey(),
            idl_account,
            program_id,
            authority_verifying_account,
        )?;

        let latest_hash = client.get_latest_blockhash()?;
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&payer_keypair.pubkey()),
            &[&payer_keypair, &program_authority_keypair],
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

        // Set the IDL
        upgrade(
            overrides,
            program_id,
            payer_filepath,
            program_authority_filepath,
            &idl_filepath,
        )?;
        Ok(())
    })
}
