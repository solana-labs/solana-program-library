//! Handlers for feature toggling commands

use {
    crate::config::Config,
    log::info,
    solana_farm_client::client::FarmClient,
    solana_farm_sdk::{string::to_pretty_json, token::TokenSelector},
    solana_sdk::pubkey::Pubkey,
};

pub fn init(client: &FarmClient, config: &Config, vault_names: &str, step: u64) {
    let vaults = get_vaults_list(client, vault_names);
    for vault in vaults {
        info!("Initializing Vault {}...", vault);
        info!(
            "Signature: {}",
            client
                .init_vault(config.keypair.as_ref(), &vault, step)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn set_admins(
    client: &FarmClient,
    config: &Config,
    vault_names: &str,
    admin_signers: &[Pubkey],
    min_signatures: u8,
) {
    let vaults = get_vaults_list(client, vault_names);
    for vault in vaults {
        info!("Initializing Vault {} multisig with new signers...", vault);

        info!(
            "Signature: {}",
            client
                .set_vault_admins(
                    config.keypair.as_ref(),
                    &vault,
                    admin_signers,
                    min_signatures
                )
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn get_admins(client: &FarmClient, config: &Config, vault_names: &str) {
    let vaults = get_vaults_list(client, vault_names);
    for vault in vaults {
        if config.no_pretty_print {
            println!("{}: {}", vault, client.get_vault_admins(&vault).unwrap());
        } else {
            println!(
                "{}: {}",
                vault,
                to_pretty_json(&client.get_vault_admins(&vault).unwrap()).unwrap()
            );
        }
    }
}

pub fn shutdown(client: &FarmClient, config: &Config, vault_names: &str) {
    let vaults = get_vaults_list(client, vault_names);
    for vault in vaults {
        info!("Shutting down Vault {}...", vault);
        info!(
            "Signature: {}",
            client
                .shutdown_vault(config.keypair.as_ref(), &vault)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn withdraw_fees(
    client: &FarmClient,
    config: &Config,
    vault_names: &str,
    fee_token: TokenSelector,
    amount: f64,
    receiver: &Pubkey,
) {
    let vaults = get_vaults_list(client, vault_names);
    for vault in vaults {
        info!("Withdrawing fees from the Vault {}...", vault);
        info!(
            "Signature: {}",
            client
                .withdraw_fees_vault(config.keypair.as_ref(), &vault, fee_token, amount, receiver)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn crank(client: &FarmClient, config: &Config, vault_names: &str, step: u64) {
    let vaults = get_vaults_list(client, vault_names);
    for vault in vaults {
        info!("Cranking step {} for the Vault {}...", step, vault);
        info!(
            "Signature: {}",
            client
                .crank_vault(config.keypair.as_ref(), &vault, step)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn set_fee(client: &FarmClient, config: &Config, vault_names: &str, fee_percent: f32) {
    let vaults = get_vaults_list(client, vault_names);
    for vault in vaults {
        info!("Setting fee to {} for the Vault {}...", fee_percent, vault);
        info!(
            "Signature: {}",
            client
                .set_fee_vault(config.keypair.as_ref(), &vault, fee_percent)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn set_external_fee(
    client: &FarmClient,
    config: &Config,
    vault_names: &str,
    external_fee_percent: f32,
) {
    let vaults = get_vaults_list(client, vault_names);
    for vault in vaults {
        info!(
            "Setting external fee to {} for the Vault {}...",
            external_fee_percent, vault
        );
        info!(
            "Signature: {}",
            client
                .set_external_fee_vault(config.keypair.as_ref(), &vault, external_fee_percent)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn set_min_crank_interval(
    client: &FarmClient,
    config: &Config,
    vault_names: &str,
    min_crank_interval: u32,
) {
    let vaults = get_vaults_list(client, vault_names);
    for vault in vaults {
        info!(
            "Setting min crank interval to {} for the Vault {}...",
            min_crank_interval, vault
        );
        info!(
            "Signature: {}",
            client
                .set_min_crank_interval_vault(config.keypair.as_ref(), &vault, min_crank_interval)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn disable_deposits(client: &FarmClient, config: &Config, vault_names: &str) {
    let vaults = get_vaults_list(client, vault_names);
    for vault in vaults {
        info!("Disabling deposits for the Vault {}...", vault);
        info!(
            "Signature: {}",
            client
                .disable_deposits_vault(config.keypair.as_ref(), &vault)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn enable_deposits(client: &FarmClient, config: &Config, vault_names: &str) {
    let vaults = get_vaults_list(client, vault_names);
    for vault in vaults {
        info!("Enabling deposits for the Vault {}...", vault);
        info!(
            "Signature: {}",
            client
                .enable_deposits_vault(config.keypair.as_ref(), &vault)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn disable_withdrawals(client: &FarmClient, config: &Config, vault_names: &str) {
    let vaults = get_vaults_list(client, vault_names);
    for vault in vaults {
        info!("Disabling withdrawals for the Vault {}...", vault);
        info!(
            "Signature: {}",
            client
                .disable_withdrawals_vault(config.keypair.as_ref(), &vault)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn enable_withdrawals(client: &FarmClient, config: &Config, vault_names: &str) {
    let vaults = get_vaults_list(client, vault_names);
    for vault in vaults {
        info!("Enabling withdrawals for the Vault {}...", vault);
        info!(
            "Signature: {}",
            client
                .enable_withdrawals_vault(config.keypair.as_ref(), &vault)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn get_info(client: &FarmClient, config: &Config, vault_names: &str) {
    let vaults = get_vaults_list(client, vault_names);
    for vault in vaults {
        info!("Retreiving stats for the Vault {}...", vault);

        let info = client.get_vault_info(&vault).unwrap();

        if config.no_pretty_print {
            println!("{}", info);
        } else {
            println!("{}", to_pretty_json(&info).unwrap());
        }
    }
    info!("Done.")
}

fn get_vaults_list(client: &FarmClient, vault_names: &str) -> Vec<String> {
    if vault_names.to_lowercase() == "all" {
        client.get_vaults().unwrap().keys().cloned().collect()
    } else {
        vault_names.split(',').map(|s| s.into()).collect()
    }
}
