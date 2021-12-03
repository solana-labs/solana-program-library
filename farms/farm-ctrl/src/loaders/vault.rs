//! Vaults loader.

use {
    crate::config::Config,
    log::info,
    solana_farm_client::client::FarmClient,
    solana_farm_sdk::{refdb::StorageType, vault::Vault},
};

pub fn load(client: &FarmClient, config: &Config, data: &str, remove_mode: bool) {
    let parsed: serde_json::Value = serde_json::from_str(data).unwrap();
    let mut last_index = client
        .get_refdb_last_index(&StorageType::Vault.to_string())
        .expect("Vault RefDB query error");

    if parsed["name"] != "Solana Vaults List" {
        panic!("Unsupported vaults file");
    }
    let vaults = parsed["vaults"].as_array().unwrap();
    for val in vaults {
        let json_vault: Vault = serde_json::from_value(val.clone()).unwrap();
        if !remove_mode {
            if config.skip_existing && client.get_vault(&json_vault.name).is_ok() {
                info!("Skipping existing Vault \"{}\"...", json_vault.name);
                continue;
            }
            info!("Writing Vault \"{}\" to on-chain RefDB...", json_vault.name);
        } else {
            info!(
                "Removing Vault \"{}\" from on-chain RefDB...",
                json_vault.name
            );
            client
                .remove_vault(config.keypair.as_ref(), &json_vault.name.to_string())
                .unwrap();
            continue;
        }
        let (index, counter) = if let Ok(vault) = client.get_vault(&json_vault.name.to_string()) {
            (vault.refdb_index, vault.refdb_counter)
        } else {
            last_index += 1;
            (Some(last_index - 1), 0u16)
        };
        let vault = Vault {
            name: json_vault.name,
            version: json_vault.version as u16,
            vault_type: json_vault.vault_type,
            official: json_vault.official,
            refdb_index: index,
            refdb_counter: counter,
            metadata_bump: json_vault.metadata_bump,
            authority_bump: json_vault.authority_bump,
            vault_token_bump: json_vault.vault_token_bump,
            lock_required: json_vault.lock_required,
            unlock_required: json_vault.unlock_required,
            vault_program_id: json_vault.vault_program_id,
            vault_authority: json_vault.vault_authority,
            vault_token_ref: json_vault.vault_token_ref,
            info_account: json_vault.info_account,
            admin_account: json_vault.admin_account,
            fees_account_a: json_vault.fees_account_a,
            fees_account_b: json_vault.fees_account_b,
            strategy: json_vault.strategy,
        };

        client.add_vault(config.keypair.as_ref(), vault).unwrap();
    }
}
