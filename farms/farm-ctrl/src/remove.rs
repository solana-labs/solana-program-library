//! Handlers for remove and remove_all commands

use {
    crate::config::Config, log::info, solana_farm_client::client::FarmClient,
    solana_farm_sdk::refdb::StorageType,
};

pub fn remove(client: &FarmClient, config: &Config, target: StorageType, object: &str) {
    info!("Removing {} object {}...", target, object);

    match target {
        StorageType::Program => {
            client
                .remove_program_id(config.keypair.as_ref(), object)
                .unwrap();
        }
        StorageType::Vault => {
            client
                .remove_vault(config.keypair.as_ref(), object)
                .unwrap();
        }
        StorageType::Farm => {
            client.remove_farm(config.keypair.as_ref(), object).unwrap();
        }
        StorageType::Pool => {
            client.remove_pool(config.keypair.as_ref(), object).unwrap();
        }
        StorageType::Token => {
            client
                .remove_token(config.keypair.as_ref(), object)
                .unwrap();
        }
        _ => {
            unreachable!();
        }
    }

    info!("Done.")
}

pub fn remove_all(client: &FarmClient, config: &Config, target: StorageType) {
    info!("Removing all {} objects...", target);

    match target {
        StorageType::Program => {
            let storage = client.get_program_ids().unwrap();
            for (name, _) in storage.iter() {
                client
                    .remove_program_id(config.keypair.as_ref(), name)
                    .unwrap();
            }
        }
        StorageType::Vault => {
            let storage = client.get_vaults().unwrap();
            for (name, _) in storage.iter() {
                client.remove_vault(config.keypair.as_ref(), name).unwrap();
            }
        }
        StorageType::Farm => {
            let storage = client.get_farms().unwrap();
            for (name, _) in storage.iter() {
                client.remove_farm(config.keypair.as_ref(), name).unwrap();
            }
        }
        StorageType::Pool => {
            let storage = client.get_pools().unwrap();
            for (name, _) in storage.iter() {
                client.remove_pool(config.keypair.as_ref(), name).unwrap();
            }
        }
        StorageType::Token => {
            let storage = client.get_tokens().unwrap();
            for (name, _) in storage.iter() {
                client.remove_token(config.keypair.as_ref(), name).unwrap();
            }
        }
        _ => {
            unreachable!();
        }
    }

    info!("Done.")
}
