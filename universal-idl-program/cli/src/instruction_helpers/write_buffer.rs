use std::fs;

use super::{
    buffer_extend, buffer_init,
    common::{with_solana_config, CliOverrides},
    serialize_idl,
};
use anyhow::Result;
use solana_client::rpc_client::RpcClient;
use solana_program::{pubkey::Pubkey, system_program};
use solana_sdk::commitment_config::CommitmentConfig;
use spl_universal_idl_program::state::buffer_seeds;

pub fn write_buffer(
    overrides: CliOverrides,
    program_id: Pubkey,
    payer_filepath: &str,
    authority_filepath: &str,
    idl_filepath: &str,
) -> Result<()> {
    with_solana_config(&overrides.clone(), |config| {
        let bytes = fs::read(idl_filepath)?;
        let data = serialize_idl(bytes)?;

        let buffer = buffer_seeds(&program_id).0;

        // Check if buffer exists
        let client = RpcClient::new(&config.rpc_url);
        let buffer_account = client
            .get_account_with_commitment(&buffer, CommitmentConfig::processed())?
            .value;

        // Check if buffer account has been created yet || overwrite it if it exists
        if buffer_account.is_none() || buffer_account.unwrap().owner == system_program::id() {
            buffer_init(
                overrides.clone(),
                config.idl_program,
                buffer,
                payer_filepath,
                authority_filepath,
            )?;
        }

        buffer_extend(
            overrides.clone(),
            config.idl_program,
            buffer,
            payer_filepath,
            authority_filepath,
            data,
        )?;

        Ok(())
    })
}
