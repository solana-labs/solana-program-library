//! Handler for the load command

use {
    crate::{config::Config, loaders},
    log::info,
    solana_farm_client::client::FarmClient,
    solana_farm_sdk::refdb::StorageType,
    std::fs,
};

pub fn load(
    client: &FarmClient,
    config: &Config,
    target: StorageType,
    filename: &str,
    remove_mode: bool,
) {
    if !remove_mode {
        info!("Loading {} objects from {}...", target, filename);
    } else {
        info!(
            "Removing all {} objects listed in file {}...",
            target, filename
        );
    }

    let data = fs::read_to_string(filename).unwrap();

    match target {
        StorageType::Program => {
            loaders::program::load(client, config, &data, remove_mode);
        }
        StorageType::Vault => {
            loaders::vault::load(client, config, &data, remove_mode);
        }
        StorageType::Farm => {
            loaders::farm::load(client, config, &data, remove_mode);
        }
        StorageType::Pool => {
            loaders::pool::load(client, config, &data, remove_mode);
        }
        StorageType::Token => {
            loaders::token::load(client, config, &data, remove_mode);
        }
        _ => {
            unreachable!();
        }
    }

    info!("Done.")
}
