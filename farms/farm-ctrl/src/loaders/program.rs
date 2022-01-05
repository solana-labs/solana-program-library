//! Program IDs loader.

use {
    crate::config::Config,
    log::info,
    serde::Deserialize,
    solana_farm_client::client::FarmClient,
    solana_farm_sdk::{id::ProgramIDType, pack::pubkey_deserialize},
    solana_sdk::pubkey::Pubkey,
};

#[derive(Deserialize, Debug)]
struct JsonProgram {
    name: String,
    description: String,
    program_type: ProgramIDType,
    #[serde(deserialize_with = "pubkey_deserialize")]
    address: Pubkey,
}

#[derive(Deserialize, Debug)]
struct JsonPrograms {
    name: String,
    timestamp: String,
    programs: Vec<JsonProgram>,
}

pub fn load(client: &FarmClient, config: &Config, data: &str, remove_mode: bool) {
    let parsed: JsonPrograms = serde_json::from_str(data).unwrap();

    for program in parsed.programs.iter() {
        if remove_mode {
            info!(
                "Removing Program \"{}\" from on-chain RefDB...",
                program.name
            );
            client
                .remove_program_id(config.keypair.as_ref(), &program.name, None)
                .unwrap();
        } else {
            if config.skip_existing && client.get_program_id(&program.name).is_ok() {
                info!("Skipping existing Program \"{}\"...", program.name);
                continue;
            }
            info!("Writing Program \"{}\" to on-chain RefDB...", program.name);
            client
                .add_program_id(
                    config.keypair.as_ref(),
                    &program.name,
                    &program.address,
                    program.program_type,
                    None,
                )
                .unwrap();
        }
    }
}
