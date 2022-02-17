//! Handlers for get and get_all command

use {
    crate::config::Config,
    log::{error, info},
    serde::Serialize,
    solana_farm_client::client::FarmClient,
    solana_farm_sdk::string::to_pretty_json,
    solana_sdk::pubkey::Pubkey,
    std::str::FromStr,
};

pub fn print(client: &FarmClient, config: &Config, target: &str, object: &str) {
    match target {
        "program" => {
            println!("{}: {}", object, client.get_program_id(object).unwrap());
        }
        "vault" => {
            print_object(
                config,
                &client.get_vault_ref(&object.to_uppercase()).unwrap(),
                &client.get_vault(&object.to_uppercase()).unwrap(),
            );
        }
        "farm" => {
            print_object(
                config,
                &client.get_farm_ref(&object.to_uppercase()).unwrap(),
                &client.get_farm(&object.to_uppercase()).unwrap(),
            );
        }
        "pool" => {
            print_object(
                config,
                &client.get_pool_ref(&object.to_uppercase()).unwrap(),
                &client.get_pool(&object.to_uppercase()).unwrap(),
            );
        }
        "token" => {
            print_object(
                config,
                &client.get_token_ref(&object.to_uppercase()).unwrap(),
                &client.get_token(&object.to_uppercase()).unwrap(),
            );
        }
        _ => {
            error!("Unrecognized target. Must be one of: token, pool, farm, vault, or program.");
        }
    }
}

pub fn print_with_ref(client: &FarmClient, config: &Config, target: &str, object: &str) {
    let ref_key = Pubkey::from_str(object).unwrap();
    match target {
        "program" => {
            println!("{}: {}", client.get_program_name(&ref_key).unwrap(), object);
        }
        "vault" => {
            print_object(
                config,
                &ref_key,
                &client.get_vault_by_ref(&ref_key).unwrap(),
            );
        }
        "farm" => {
            print_object(config, &ref_key, &client.get_farm_by_ref(&ref_key).unwrap());
        }
        "pool" => {
            print_object(config, &ref_key, &client.get_pool_by_ref(&ref_key).unwrap());
        }
        "token" => {
            print_object(
                config,
                &ref_key,
                &client.get_token_by_ref(&ref_key).unwrap(),
            );
        }
        _ => {
            error!("Unrecognized target. Must be one of: token, pool, farm, vault, or program.");
        }
    }
}

pub fn print_all(client: &FarmClient, config: &Config, target: &str) {
    info!("Loading {} objects...", target);

    match target {
        "program" => {
            let storage = client.get_program_ids().unwrap();
            for (name, key) in storage.iter() {
                println!("{}: {}", name, key);
            }
        }
        "vault" => {
            let storage = client.get_vaults().unwrap();
            for (name, key) in storage.iter() {
                print_object(config, &client.get_vault_ref(name).unwrap(), key);
            }
        }
        "farm" => {
            let storage = client.get_farms().unwrap();
            for (name, key) in storage.iter() {
                print_object(config, &client.get_farm_ref(name).unwrap(), key);
            }
        }
        "pool" => {
            let storage = client.get_pools().unwrap();
            for (name, key) in storage.iter() {
                print_object(config, &client.get_pool_ref(name).unwrap(), key);
            }
        }
        "token" => {
            let storage = client.get_tokens().unwrap();
            for (name, key) in storage.iter() {
                print_object(config, &client.get_token_ref(name).unwrap(), key);
            }
        }
        _ => {
            error!("Unrecognized target. Must be one of: token, pool, farm, vault, or program.");
        }
    }

    info!("Done.")
}

pub fn list_all(client: &FarmClient, _config: &Config, target: &str) {
    info!("Loading {} objects...", target);

    match target {
        "program" => {
            let storage = client.get_program_ids().unwrap();
            for (name, key) in storage.iter() {
                println!("{}: {}", name, key);
            }
        }
        "vault" => {
            let storage = client.get_vault_refs().unwrap();
            for (name, key) in storage.iter() {
                println!("{}: {}", name, key);
            }
        }
        "farm" => {
            let storage = client.get_farm_refs().unwrap();
            for (name, key) in storage.iter() {
                println!("{}: {}", name, key);
            }
        }
        "pool" => {
            let storage = client.get_pool_refs().unwrap();
            for (name, key) in storage.iter() {
                println!("{}: {}", name, key);
            }
        }
        "token" => {
            let storage = client.get_token_refs().unwrap();
            for (name, key) in storage.iter() {
                println!("{}: {}", name, key);
            }
        }
        _ => {
            error!("Unrecognized target. Must be one of: token, pool, farm, vault, or program.");
        }
    }

    info!("Done.")
}

pub fn print_object<T>(config: &Config, key: &Pubkey, object: &T)
where
    T: ?Sized + Serialize + std::fmt::Display,
{
    if config.no_pretty_print {
        println!("{}: {}", key, object);
    } else {
        println!("{}: {}", key, to_pretty_json(object).unwrap());
    }
}
