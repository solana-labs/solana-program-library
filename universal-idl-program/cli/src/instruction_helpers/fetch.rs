use std::io::Read;

use anyhow::{anyhow, Result};
use flate2::bufread::ZlibDecoder;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use solana_client::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use solana_sdk::{account::ReadableAccount, commitment_config::CommitmentConfig};
use spl_universal_idl_program::state::{idl_seeds, Idl, SolanaAccount};

use crate::instruction_helpers::common::with_solana_config;

use super::common::CliOverrides;

fn fetch_idl(client: &RpcClient, program_or_idl: Pubkey) -> Result<Idl> {
    let mut account = client
        .get_account_with_commitment(&program_or_idl, CommitmentConfig::processed())?
        .value
        .map_or(Err(anyhow!("Idl account not found")), Ok)?;

    // We were given a program, so we now fetch it's IDL
    if account.executable {
        let idl_addr = idl_seeds(&program_or_idl).0;

        account = client
            .get_account_with_commitment(&idl_addr, CommitmentConfig::processed())?
            .value
            .map_or(Err(anyhow!("Idl account not found")), Ok)?;
    }

    Ok(Idl::safe_deserialize(&account.data())?)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UiIdlData {
    pub authority: String,
    pub slot: u64,
    /// Parsed IDL is stored here
    /// Parse it by piping to `jq` command line tool
    pub idl: Value,
}

pub fn idl_fetch(overrides: CliOverrides, program_or_idl: Pubkey) -> Result<()> {
    with_solana_config(&overrides, |config| {
        let client = RpcClient::new(&config.rpc_url);
        let idl = fetch_idl(&client, program_or_idl)?;

        // Unzip the raw bytes
        let raw_bytes = idl.data.as_ref();
        let mut z = ZlibDecoder::new(raw_bytes);
        let mut idl_file_bytes = Vec::new();
        z.read_to_end(&mut idl_file_bytes)?;

        let data = String::from_utf8(idl_file_bytes.clone()).unwrap();
        let parsed = serde_json::from_str::<serde_json::Value>(&data).unwrap();

        let obj = UiIdlData {
            authority: idl.authority.to_string(),
            slot: idl.slot,
            idl: parsed,
        };
        println!("{}", &serde_json::to_string_pretty(&obj)?);
        Ok(())
    })
}
