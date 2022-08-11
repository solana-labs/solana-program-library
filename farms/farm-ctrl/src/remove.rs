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
        StorageType::Fund => {
            client
                .remove_fund(config.keypair.as_ref(), &object.to_uppercase())
                .unwrap();
        }
        StorageType::Vault => {
            client
                .remove_vault(config.keypair.as_ref(), &object.to_uppercase())
                .unwrap();
        }
        StorageType::Farm => {
            client
                .remove_farm(config.keypair.as_ref(), &object.to_uppercase())
                .unwrap();
        }
        StorageType::Pool => {
            client
                .remove_pool(config.keypair.as_ref(), &object.to_uppercase())
                .unwrap();
        }
        StorageType::Token => {
            client
                .remove_token(config.keypair.as_ref(), &object.to_uppercase())
                .unwrap();
        }
        _ => {
            unreachable!();
        }
    }

    info!("Done.")
}

pub fn remove_ref(client: &FarmClient, config: &Config, target: StorageType, object: &str) {
    info!("Removing {} reference {}...", target, object);
    client
        .remove_reference(config.keypair.as_ref(), target, object)
        .unwrap();

    info!("Done.")
}

pub fn remove_all(client: &FarmClient, config: &Config, target: StorageType) {
    info!("Removing all {} objects...", target);

    match target {
        StorageType::Program => {
            for (name, _) in client.get_program_ids().unwrap() {
                client
                    .remove_program_id(config.keypair.as_ref(), &name)
                    .unwrap();
            }
        }
        StorageType::Fund => {
            for (name, _) in client.get_fund_refs().unwrap() {
                client.remove_fund(config.keypair.as_ref(), &name).unwrap();
            }
        }
        StorageType::Vault => {
            for (name, _) in client.get_vault_refs().unwrap() {
                client.remove_vault(config.keypair.as_ref(), &name).unwrap();
            }
        }
        StorageType::Farm => {
            for (name, _) in client.get_farm_refs().unwrap() {
                client.remove_farm(config.keypair.as_ref(), &name).unwrap();
            }
        }
        StorageType::Pool => {
            for (name, _) in client.get_pool_refs().unwrap() {
                client.remove_pool(config.keypair.as_ref(), &name).unwrap();
            }
        }
        StorageType::Token => {
            for (name, _) in client.get_token_refs().unwrap() {
                client.remove_token(config.keypair.as_ref(), &name).unwrap();
            }
        }
        _ => {
            unreachable!();
        }
    }

    info!("Done.")
}
