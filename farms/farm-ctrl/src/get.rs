//! Handlers for get and get_all command

use {
    crate::config::Config,
    log::info,
    serde::Serialize,
    solana_farm_client::client::FarmClient,
    solana_farm_sdk::{refdb::StorageType, string::to_pretty_json},
    solana_sdk::pubkey::Pubkey,
    std::str::FromStr,
};

pub fn get(client: &FarmClient, config: &Config, target: StorageType, object: &str) {
    info!("Querying {} object {}...", target, object);

    match target {
        StorageType::Program => {
            println!("{}: {}", object, client.get_program_id(object).unwrap());
        }
        StorageType::Vault => {
            print_object(
                config,
                &client.get_vault_ref(&object.to_uppercase()).unwrap(),
                &client.get_vault(&object.to_uppercase()).unwrap(),
            );
        }
        StorageType::Farm => {
            print_object(
                config,
                &client.get_farm_ref(&object.to_uppercase()).unwrap(),
                &client.get_farm(&object.to_uppercase()).unwrap(),
            );
        }
        StorageType::Pool => {
            print_object(
                config,
                &client.get_pool_ref(&object.to_uppercase()).unwrap(),
                &client.get_pool(&object.to_uppercase()).unwrap(),
            );
        }
        StorageType::Token => {
            print_object(
                config,
                &client.get_token_ref(&object.to_uppercase()).unwrap(),
                &client.get_token(&object.to_uppercase()).unwrap(),
            );
        }
        _ => {
            unreachable!();
        }
    }

    info!("Done.")
}

pub fn get_ref(client: &FarmClient, config: &Config, target: StorageType, object: &str) {
    info!("Querying {} object {}...", target, object);

    let pubkey = Pubkey::from_str(object).unwrap();

    match target {
        StorageType::Program => {
            println!("{}: {}", client.get_program_name(&pubkey).unwrap(), object);
        }
        StorageType::Vault => {
            print_object(config, &pubkey, &client.get_vault_by_ref(&pubkey).unwrap());
        }
        StorageType::Farm => {
            print_object(config, &pubkey, &client.get_farm_by_ref(&pubkey).unwrap());
        }
        StorageType::Pool => {
            print_object(config, &pubkey, &client.get_pool_by_ref(&pubkey).unwrap());
        }
        StorageType::Token => {
            print_object(config, &pubkey, &client.get_token_by_ref(&pubkey).unwrap());
        }
        _ => {
            unreachable!();
        }
    }

    info!("Done.")
}

pub fn get_all(client: &FarmClient, config: &Config, target: StorageType) {
    info!("Querying all {} objects...", target);

    match target {
        StorageType::Program => {
            let storage = client.get_program_ids().unwrap();
            for (name, key) in storage.iter() {
                println!("{}: {}", name, key);
            }
        }
        StorageType::Vault => {
            let storage = client.get_vaults().unwrap();
            for (name, key) in storage.iter() {
                print_object(config, &client.get_vault_ref(name).unwrap(), key);
            }
        }
        StorageType::Farm => {
            let storage = client.get_farms().unwrap();
            for (name, key) in storage.iter() {
                print_object(config, &client.get_farm_ref(name).unwrap(), key);
            }
        }
        StorageType::Pool => {
            let storage = client.get_pools().unwrap();
            for (name, key) in storage.iter() {
                print_object(config, &client.get_pool_ref(name).unwrap(), key);
            }
        }
        StorageType::Token => {
            let storage = client.get_tokens().unwrap();
            for (name, key) in storage.iter() {
                print_object(config, &client.get_token_ref(name).unwrap(), key);
            }
        }
        _ => {
            unreachable!();
        }
    }

    info!("Done.")
}

pub fn list_all(client: &FarmClient, _config: &Config, target: StorageType) {
    info!("Querying all {} objects...", target);

    match target {
        StorageType::Program => {
            let storage = client.get_program_ids().unwrap();
            for (name, key) in storage.iter() {
                println!("{}: {}", name, key);
            }
        }
        StorageType::Vault => {
            let storage = client.get_vault_refs().unwrap();
            for (name, key) in storage.iter() {
                println!("{}: {}", name, key);
            }
        }
        StorageType::Farm => {
            let storage = client.get_farm_refs().unwrap();
            for (name, key) in storage.iter() {
                println!("{}: {}", name, key);
            }
        }
        StorageType::Pool => {
            let storage = client.get_pool_refs().unwrap();
            for (name, key) in storage.iter() {
                println!("{}: {}", name, key);
            }
        }
        StorageType::Token => {
            let storage = client.get_token_refs().unwrap();
            for (name, key) in storage.iter() {
                println!("{}: {}", name, key);
            }
        }
        _ => {
            unreachable!();
        }
    }

    info!("Done.")
}

fn print_object<T>(config: &Config, key: &Pubkey, object: &T)
where
    T: ?Sized + Serialize + std::fmt::Display,
{
    if config.no_pretty_print {
        println!("{}: {}", key, object);
    } else {
        println!("{}: {}", key, to_pretty_json(object).unwrap());
    }
}
