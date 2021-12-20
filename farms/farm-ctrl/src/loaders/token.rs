//! Tokens loader.

use {
    crate::{
        config::Config,
        loaders::{farm::JsonOrcaFarm, pool::JsonOrcaPool, utils::*},
    },
    log::info,
    serde::Deserialize,
    serde_json::Value,
    solana_farm_client::client::FarmClient,
    solana_farm_sdk::{
        git_token::GitToken,
        pack::{as64_deserialize, pubkey_deserialize},
        refdb::StorageType,
        string::{str_to_as64, ArrayString64},
        token::Token,
        token::TokenType,
    },
    solana_sdk::pubkey::Pubkey,
    std::collections::HashMap,
};

#[derive(Deserialize, Debug)]
struct JsonRaydiumLPToken {
    #[serde(deserialize_with = "as64_deserialize")]
    symbol: ArrayString64,
    #[serde(deserialize_with = "as64_deserialize")]
    name: ArrayString64,
    coin: String,
    pc: String,
    #[serde(rename = "mintAddress", deserialize_with = "pubkey_deserialize")]
    mint_address: Pubkey,
    decimals: u8,
}

pub fn load(client: &FarmClient, config: &Config, data: &str, remove_mode: bool) {
    let parsed: Value = serde_json::from_str(data).unwrap();
    let last_index = client
        .get_refdb_last_index(&StorageType::Token.to_string())
        .expect("Token RefDB query error");
    let is_saber = parsed["name"] == "Saber Tokens";

    if parsed["name"] == "Solana Token List" || is_saber {
        load_solana_tokens(client, config, remove_mode, &parsed, last_index);
    } else if parsed["name"] == "Raydium LP Tokens" {
        load_raydium_tokens(client, config, remove_mode, &parsed, last_index);
    } else if parsed["name"] == "Orca Pools" {
        load_orca_pool_tokens(client, config, remove_mode, &parsed, last_index);
    } else if parsed["name"] == "Orca Farms" {
        load_orca_farm_tokens(client, config, remove_mode, &parsed, last_index);
    } else {
        panic!("Unsupported tokens file");
    }
}

fn load_solana_tokens(
    client: &FarmClient,
    config: &Config,
    remove_mode: bool,
    parsed: &Value,
    last_index: u32,
) {
    let mut last_index = last_index;
    let is_saber = parsed["name"] == "Saber Tokens";
    let tokens = parsed["tokens"].as_array().unwrap();
    for val in tokens {
        let git_token: GitToken = serde_json::from_value(val.clone()).unwrap();
        let token_type = if git_token.symbol.to_uppercase() == "SOL" {
            TokenType::WrappedSol
        } else {
            get_token_type_from_tags(&git_token.tags)
        };
        let name = if is_saber && token_type == TokenType::LpToken {
            "LP.SBR.".to_string()
                + &normalize_name(git_token.name.split(' ').collect::<Vec<&str>>()[0], true)
        } else if token_type == TokenType::VtToken {
            git_token.symbol
        } else {
            normalize_name(&git_token.symbol, false)
        };

        if git_token.chain_id != 101 || (token_type == TokenType::LpToken && !is_saber) {
            continue;
        }
        if !remove_mode {
            if config.skip_existing && client.get_token(&name).is_ok() {
                info!("Skipping existing Token \"{}\"...", name);
                continue;
            }
            info!("Writing Token \"{}\" to on-chain RefDB...", name);
        } else {
            info!("Removing Token \"{}\" from on-chain RefDB...", name);
            client.remove_token(config.keypair.as_ref(), &name).unwrap();
            continue;
        }
        let (index, counter) = if let Ok(token) = client.get_token(&name) {
            (token.refdb_index, token.refdb_counter)
        } else {
            last_index += 1;
            (Some(last_index - 1), 0u16)
        };
        let token = Token {
            name: str_to_as64(&name).unwrap(),
            description: str_to_as64(&git_token.name).unwrap(),
            token_type,
            refdb_index: index,
            refdb_counter: counter,
            decimals: git_token.decimals as u8,
            chain_id: git_token.chain_id as u16,
            mint: convert_pubkey(&git_token.address),
        };

        client.add_token(config.keypair.as_ref(), token).unwrap();
    }
}

fn load_raydium_tokens(
    client: &FarmClient,
    config: &Config,
    remove_mode: bool,
    parsed: &Value,
    last_index: u32,
) {
    let mut last_index = last_index;
    let tokens: HashMap<String, JsonRaydiumLPToken> =
        serde_json::from_value(parsed["tokens"].clone()).unwrap();
    for (symbol, token) in tokens.iter() {
        let name = "LP.RDM.".to_string() + &normalize_name(symbol, true);
        if !remove_mode {
            if config.skip_existing && client.get_token(&name).is_ok() {
                info!("Skipping existing Token \"{}\"...", name);
                continue;
            }
            info!("Writing Token \"{}\" to on-chain RefDB...", name);
        } else {
            info!("Removing Token \"{}\" from on-chain RefDB...", name);
            let _ = client.remove_token(config.keypair.as_ref(), &name);
            continue;
        }
        let (index, counter) = if let Ok(token) = client.get_token(&name) {
            (token.refdb_index, token.refdb_counter)
        } else {
            last_index += 1;
            (Some(last_index - 1), 0u16)
        };
        let token = Token {
            name: str_to_as64(&name).unwrap(),
            description: token.name,
            token_type: TokenType::LpToken,
            refdb_index: index,
            refdb_counter: counter,
            decimals: token.decimals,
            chain_id: 101u16,
            mint: token.mint_address,
        };

        client.add_token(config.keypair.as_ref(), token).unwrap();
    }
}

fn load_orca_pool_tokens(
    client: &FarmClient,
    config: &Config,
    remove_mode: bool,
    parsed: &Value,
    last_index: u32,
) {
    let mut last_index = last_index;
    let pools = parsed["pools"].as_array().unwrap();
    for val in pools {
        let json_pool: JsonOrcaPool = serde_json::from_value(val.clone()).unwrap();
        let name = "LP.ORC.".to_string() + &json_pool.name.to_uppercase().replace("_", "-");
        if !remove_mode {
            if config.skip_existing && client.get_token(&name).is_ok() {
                info!("Skipping existing Token \"{}\"...", name);
                continue;
            }
            info!("Writing Token \"{}\" to on-chain RefDB...", name);
        } else {
            info!("Removing Token \"{}\" from on-chain RefDB...", name);
            let _ = client.remove_token(config.keypair.as_ref(), &name);
            continue;
        }
        let (index, counter) = if let Ok(token) = client.get_token(&name) {
            (token.refdb_index, token.refdb_counter)
        } else {
            last_index += 1;
            (Some(last_index - 1), 0u16)
        };
        let token = Token {
            name: str_to_as64(&name).unwrap(),
            description: str_to_as64(format!("Orca {} LP Token", json_pool.name).as_str()).unwrap(),
            token_type: TokenType::LpToken,
            refdb_index: index,
            refdb_counter: counter,
            decimals: json_pool.pool_token_decimals,
            chain_id: 101u16,
            mint: json_pool.pool_token_mint,
        };

        client.add_token(config.keypair.as_ref(), token).unwrap();
    }
}

fn load_orca_farm_tokens(
    client: &FarmClient,
    config: &Config,
    remove_mode: bool,
    parsed: &Value,
    last_index: u32,
) {
    let mut last_index = last_index;
    let farms = parsed["farms"].as_array().unwrap();
    for val in farms {
        let json_farm: JsonOrcaFarm = serde_json::from_value(val.clone()).unwrap();
        let name = "LP.ORC.".to_string() + &json_farm.name.to_uppercase().replace("_", "-");
        if !remove_mode {
            if config.skip_existing && client.get_token(&name).is_ok() {
                info!("Skipping existing Token \"{}\"...", name);
                continue;
            }
            info!("Writing Token \"{}\" to on-chain RefDB...", name);
        } else {
            info!("Removing Token \"{}\" from on-chain RefDB...", name);
            let _ = client.remove_token(config.keypair.as_ref(), &name);
            continue;
        }
        let (index, counter) = if let Ok(token) = client.get_token(&name) {
            (token.refdb_index, token.refdb_counter)
        } else {
            last_index += 1;
            (Some(last_index - 1), 0u16)
        };
        let token = Token {
            name: str_to_as64(&name).unwrap(),
            description: str_to_as64(format!("Orca {} Farm LP Token", json_farm.name).as_str())
                .unwrap(),
            token_type: TokenType::LpToken,
            refdb_index: index,
            refdb_counter: counter,
            decimals: json_farm.base_token_decimals,
            chain_id: 101u16,
            mint: json_farm.farm_token_mint,
        };

        client.add_token(config.keypair.as_ref(), token).unwrap();
    }
}

fn get_token_type_from_tags(tags: &[String]) -> TokenType {
    if tags.contains(&String::from("Solana tokenized")) {
        TokenType::WrappedSol
    } else if tags.contains(&String::from("wrapped-sollet")) {
        TokenType::WrappedSollet
    } else if tags.contains(&String::from("wrapped"))
        || tags.contains(&String::from("wormhole-v1"))
        || tags.contains(&String::from("wormhole-v2"))
    {
        TokenType::WrappedWarmhole
    } else if tags.contains(&String::from("lp-token"))
        || tags.contains(&String::from("saber-stableswap-lp"))
    {
        TokenType::LpToken
    } else if tags.contains(&String::from("vt-token")) {
        TokenType::VtToken
    } else {
        TokenType::SplToken
    }
}
