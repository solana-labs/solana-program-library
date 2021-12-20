//! Handlers for refdb_init and refdb_drop commands

use {
    crate::config::Config,
    log::info,
    solana_farm_client::client::FarmClient,
    solana_farm_sdk::{refdb::ReferenceType, refdb::StorageType},
};

pub fn init(client: &FarmClient, config: &Config, target: StorageType) {
    if client.is_refdb_initialized(&target.to_string()).unwrap() {
        info!("Already initialized RefDB found for {} objects", target);
        return;
    }
    info!("Initializing RefDB for {} objects", target);

    client
        .initialize_refdb(
            config.keypair.as_ref(),
            &target.to_string(),
            ReferenceType::Pubkey,
            StorageType::get_default_max_records(target, ReferenceType::Pubkey),
            true,
        )
        .unwrap();

    info!("Done.")
}

pub fn init_all(client: &FarmClient, config: &Config) {
    init(client, config, StorageType::Program);
    init(client, config, StorageType::Token);
    init(client, config, StorageType::Pool);
    init(client, config, StorageType::Farm);
    init(client, config, StorageType::Vault);
}

pub fn drop(client: &FarmClient, config: &Config, target: StorageType) {
    if !client.is_refdb_initialized(&target.to_string()).unwrap() {
        info!("No initialized RefDB found for {} objects", target);
        return;
    }
    info!("Removing RefDB for {} objects", target);

    client
        .drop_refdb(config.keypair.as_ref(), &target.to_string(), true)
        .unwrap();

    info!("Done.")
}

pub fn drop_all(client: &FarmClient, config: &Config) {
    drop(client, config, StorageType::Vault);
    drop(client, config, StorageType::Farm);
    drop(client, config, StorageType::Pool);
    drop(client, config, StorageType::Token);
    drop(client, config, StorageType::Program);
}
