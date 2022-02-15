//! Handlers for feature toggling commands

use {
    crate::config::Config,
    log::info,
    solana_farm_client::client::FarmClient,
    solana_farm_sdk::{string::to_pretty_json, token::TokenSelector},
    solana_sdk::pubkey::Pubkey,
    std::str::FromStr,
};

pub fn init(client: &FarmClient, config: &Config, vault_names: &str, step: u64) {
    let vaults = vault_names.split(',').collect::<Vec<_>>();
    for vault in vaults {
        info!("Initializing Vault {}...", vault);
        info!(
            "Signature: {}",
            client
                .init_vault(config.keypair.as_ref(), vault, step)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn shutdown(client: &FarmClient, config: &Config, vault_names: &str) {
    let vaults = vault_names.split(',').collect::<Vec<_>>();
    for vault in vaults {
        info!("Shutting down Vault {}...", vault);
        info!(
            "Signature: {}",
            client
                .shutdown_vault(config.keypair.as_ref(), vault)
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
    receiver: &str,
) {
    let receiver_key = Pubkey::from_str(receiver).unwrap();
    let vaults = vault_names.split(',').collect::<Vec<_>>();
    for vault in vaults {
        info!("Withdrawing fees from the Vault {}...", vault);
        info!(
            "Signature: {}",
            client
                .withdraw_fees_vault(
                    config.keypair.as_ref(),
                    vault,
                    fee_token,
                    amount,
                    &receiver_key
                )
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn crank(client: &FarmClient, config: &Config, vault_names: &str, step: u64) {
    let vaults = vault_names.split(',').collect::<Vec<_>>();
    for vault in vaults {
        info!("Cranking step {} for Vault {}...", step, vault);
        info!(
            "Signature: {}",
            client
                .crank_vault(config.keypair.as_ref(), vault, step)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn crank_all(client: &FarmClient, config: &Config, step: u64) {
    let vaults = client.get_vaults().unwrap();
    for (vault_name, _) in vaults.iter() {
        info!("Cranking step {} for Vault {}...", step, vault_name);
        info!(
            "Signature: {}",
            client
                .crank_vault(config.keypair.as_ref(), vault_name, step)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn set_fee(client: &FarmClient, config: &Config, vault_names: &str, fee_percent: f32) {
    let vaults = vault_names.split(',').collect::<Vec<_>>();
    for vault in vaults {
        info!("Setting fee to {} for Vault {}...", fee_percent, vault);
        info!(
            "Signature: {}",
            client
                .set_fee_vault(config.keypair.as_ref(), vault, fee_percent)
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
    let vaults = vault_names.split(',').collect::<Vec<_>>();
    for vault in vaults {
        info!(
            "Setting external fee to {} for Vault {}...",
            external_fee_percent, vault
        );
        info!(
            "Signature: {}",
            client
                .set_external_fee_vault(config.keypair.as_ref(), vault, external_fee_percent)
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
    let vaults = vault_names.split(',').collect::<Vec<_>>();
    for vault in vaults {
        info!(
            "Setting min crank interval to {} for Vault {}...",
            min_crank_interval, vault
        );
        info!(
            "Signature: {}",
            client
                .set_min_crank_interval_vault(config.keypair.as_ref(), vault, min_crank_interval)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn disable_deposit(client: &FarmClient, config: &Config, vault_names: &str) {
    let vaults = vault_names.split(',').collect::<Vec<_>>();
    for vault in vaults {
        info!("Disabling deposits for Vault {}...", vault);
        info!(
            "Signature: {}",
            client
                .disable_deposit_vault(config.keypair.as_ref(), vault)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn enable_deposit(client: &FarmClient, config: &Config, vault_names: &str) {
    let vaults = vault_names.split(',').collect::<Vec<_>>();
    for vault in vaults {
        info!("Enabling deposits for Vault {}...", vault);
        info!(
            "Signature: {}",
            client
                .enable_deposit_vault(config.keypair.as_ref(), vault)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn disable_withdrawal(client: &FarmClient, config: &Config, vault_names: &str) {
    let vaults = vault_names.split(',').collect::<Vec<_>>();
    for vault in vaults {
        info!("Disabling withdrawals for Vault {}...", vault);
        info!(
            "Signature: {}",
            client
                .disable_withdrawal_vault(config.keypair.as_ref(), vault)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn enable_withdrawal(client: &FarmClient, config: &Config, vault_names: &str) {
    let vaults = vault_names.split(',').collect::<Vec<_>>();
    for vault in vaults {
        info!("Enabling withdrawals for Vault {}...", vault);
        info!(
            "Signature: {}",
            client
                .enable_withdrawal_vault(config.keypair.as_ref(), vault)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn get_info(client: &FarmClient, config: &Config, vault_names: &str) {
    let vaults = vault_names.split(',').collect::<Vec<_>>();
    for vault in vaults {
        info!("Retreiving stats for Vault {}...", vault);

        let info = client.get_vault_info(vault).unwrap();

        if config.no_pretty_print {
            println!("{}", info);
        } else {
            println!("{}", to_pretty_json(&info).unwrap());
        }
    }
    info!("Done.")
}
