//! Handlers for refdb_init and refdb_drop commands

use {
    crate::config::Config,
    log::info,
    solana_farm_client::client::FarmClient,
    solana_farm_sdk::{refdb::ReferenceType, refdb::StorageType, string::to_pretty_json},
    solana_sdk::pubkey::Pubkey,
};

pub fn init(client: &FarmClient, config: &Config, target: StorageType) {
    if client.is_refdb_initialized(&target.to_string()).unwrap() {
        info!("Already initialized RefDB found for {} objects...", target);
        return;
    }
    info!("Initializing RefDB for {} objects", target);

    info!(
        "Signature: {}",
        client
            .initialize_refdb(
                config.keypair.as_ref(),
                &target.to_string(),
                ReferenceType::Pubkey,
                StorageType::get_default_max_records(target, ReferenceType::Pubkey),
                true,
            )
            .unwrap()
    );

    info!("Done.")
}

pub fn init_all(client: &FarmClient, config: &Config) {
    init(client, config, StorageType::Program);
    init(client, config, StorageType::Token);
    init(client, config, StorageType::Pool);
    init(client, config, StorageType::Farm);
    init(client, config, StorageType::Vault);
    init(client, config, StorageType::Fund);
}

pub fn set_admins(
    client: &FarmClient,
    config: &Config,
    admin_signers: &[Pubkey],
    min_signatures: u8,
) {
    info!("Initializing Main Router multisig with new signers...");

    info!(
        "Signature: {}",
        client
            .set_admins(config.keypair.as_ref(), admin_signers, min_signatures)
            .unwrap()
    );

    info!("Done.")
}

pub fn get_admins(client: &FarmClient, config: &Config) {
    if config.no_pretty_print {
        println!("Main Router: {}", client.get_admins().unwrap());
    } else {
        println!(
            "Main Router: {}",
            to_pretty_json(&client.get_admins().unwrap()).unwrap()
        );
    }
}

pub fn set_program_admins(
    client: &FarmClient,
    config: &Config,
    program_id: &Pubkey,
    admin_signers: &[Pubkey],
    min_signatures: u8,
) {
    info!(
        "Setting new admin signers for the program {}...",
        program_id
    );

    info!(
        "Signature: {}",
        client
            .set_program_admins(
                config.keypair.as_ref(),
                program_id,
                admin_signers,
                min_signatures
            )
            .unwrap()
    );

    info!("Done.")
}

pub fn get_program_admins(client: &FarmClient, config: &Config, program_id: &Pubkey) {
    if config.no_pretty_print {
        println!(
            "{}: {}",
            client.get_program_multisig_account(program_id).unwrap(),
            client.get_program_admins(program_id).unwrap()
        );
    } else {
        println!(
            "{}: {}",
            client.get_program_multisig_account(program_id).unwrap(),
            to_pretty_json(&client.get_program_admins(program_id).unwrap()).unwrap()
        );
    }
}

pub fn set_program_single_authority(
    client: &FarmClient,
    config: &Config,
    program_id: &Pubkey,
    upgrade_authority: &Pubkey,
) {
    info!(
        "Setting single upgrade authority for the program {}...",
        program_id
    );

    info!(
        "Signature: {}",
        client
            .set_program_single_authority(config.keypair.as_ref(), program_id, upgrade_authority)
            .unwrap()
    );

    info!("Done.")
}

pub fn upgrade_program(
    client: &FarmClient,
    config: &Config,
    program_id: &Pubkey,
    buffer_address: &Pubkey,
) {
    info!("Upgrading program {}...", program_id);

    info!(
        "Signature: {}",
        client
            .upgrade_program(config.keypair.as_ref(), program_id, buffer_address)
            .unwrap()
    );

    info!("Done.")
}

pub fn drop(client: &FarmClient, config: &Config, target: StorageType) {
    if !client.is_refdb_initialized(&target.to_string()).unwrap() {
        info!("No initialized RefDB found for {} objects...", target);
        return;
    }
    info!("Removing RefDB for {} objects", target);

    info!(
        "Signature: {}",
        client
            .drop_refdb(config.keypair.as_ref(), &target.to_string(), true)
            .unwrap()
    );

    info!("Done.")
}

pub fn drop_all(client: &FarmClient, config: &Config) {
    drop(client, config, StorageType::Fund);
    drop(client, config, StorageType::Vault);
    drop(client, config, StorageType::Farm);
    drop(client, config, StorageType::Pool);
    drop(client, config, StorageType::Token);
    drop(client, config, StorageType::Program);
}
