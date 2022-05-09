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
        id::zero,
        pack::{as64_deserialize, pubkey_deserialize},
        refdb::StorageType,
        string::{str_to_as64, ArrayString64},
        token::{GitToken, OracleType, Token, TokenType},
    },
    solana_sdk::pubkey::Pubkey,
    std::collections::HashMap,
    std::str::FromStr,
};

#[allow(dead_code)]
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
        } else if token_type == TokenType::VtToken || token_type == TokenType::FundToken {
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
        let (oracle_type, oracle_account) = get_oracle_price_account(config, &name);
        let token = Token {
            name: str_to_as64(&name).unwrap(),
            description: str_to_as64(&git_token.name).unwrap(),
            token_type,
            refdb_index: index,
            refdb_counter: counter,
            decimals: git_token.decimals as u8,
            chain_id: git_token.chain_id as u16,
            mint: convert_pubkey(&git_token.address),
            oracle_type,
            oracle_account,
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
        let (oracle_type, oracle_account) = get_oracle_price_account(config, &name);
        let token = Token {
            name: str_to_as64(&name).unwrap(),
            description: token.name,
            token_type: TokenType::LpToken,
            refdb_index: index,
            refdb_counter: counter,
            decimals: token.decimals,
            chain_id: 101u16,
            mint: token.mint_address,
            oracle_type,
            oracle_account,
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
        let name = "LP.ORC.".to_string() + &json_pool.name.to_uppercase().replace('_', "-");
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
        let (oracle_type, oracle_account) = get_oracle_price_account(config, &name);
        let token = Token {
            name: str_to_as64(&name).unwrap(),
            description: str_to_as64(format!("Orca {} LP Token", json_pool.name).as_str()).unwrap(),
            token_type: TokenType::LpToken,
            refdb_index: index,
            refdb_counter: counter,
            decimals: json_pool.pool_token_decimals,
            chain_id: 101u16,
            mint: json_pool.pool_token_mint,
            oracle_type,
            oracle_account,
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
        let name = "LP.ORC.".to_string() + &json_farm.name.to_uppercase().replace('_', "-");
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
        let (oracle_type, oracle_account) = get_oracle_price_account(config, &name);
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
            oracle_type,
            oracle_account,
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
    } else if tags.contains(&String::from("fund-token")) {
        TokenType::FundToken
    } else {
        TokenType::SplToken
    }
}

fn get_oracle_price_account(config: &Config, symbol: &str) -> (OracleType, Pubkey) {
    let acc = if config.farm_client_url.contains("devnet") {
        match symbol {
            "SOL" => "J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix",
            "WSOL" => "J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix",
            "MSOL" => "9a6RNx3tCu1TSs6TBSfV2XRXEPEZXQ6WB7jRojZRvyeZ",
            "USDC" => "5SSkXsEKQepHHAewytPVwdej4epN1nxgLVM84L4KXgy7",
            "USDT" => "38xoQ4oeJCBrcVvca2cGk7iV1dAfrmTR1kmhSCJQ8Jto",
            "RAY" => "EhgAdTrgxi4ZoVZLQx1n93vULucPpiFi2BQtz9RJr1y6",
            "SRM" => "992moaMQKs32GKZ9dxi8keyM2bUmbrwBZpK4p2K6X5Vs",
            "COIN" => "J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix",
            "PC" => "J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix",
            _ => return (OracleType::Unsupported, zero::id()),
        }
    } else {
        match symbol {
            "BCH" => "5ALDzwcRJfSyGdGyhP3kP628aqBNHZzLuVww7o9kdspe",
            "LTC" => "8RMnV1eD55iqUFJLMguPkYBkq8DCtx81XcmAja93LvRR",
            "BTC" => "GVXRSBjFk6e6J3NbVPXohDJetcTjaeeuykUpbQF8UoMU",
            "BNB" => "4CkQJBxhU8EZ2UjhigbtdaPbpTe6mqf811fipYBFbSYN",
            "DOGE" => "FsSM3s38PX9K7Dn6eGzuE29S2Dsk1Sss1baytTQdCaQj",
            "USDT" => "3vxLXJqLqF3JG5TCbYycbKWRBbCJQLxQmBGCkyqEEefL",
            "SOL" => "H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG",
            "WSOL" => "H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG",
            "USDC" => "Gnt27xtC473ZT2Mw5u8wZ68Z3gULkSTb5DuxJy7eJotD",
            "ETH" => "JBu1AL4obBcCMqKBBxhpWCNUt136ijcuMZLFvTP7iWdB",
            "SRM" => "3NBReDRTLKMQEKiLD5tGcx4kXbTf88b7f2xLS9UuGjym",
            "LUNA" => "5bmWuR1dgP4avtGYMNKLuxumZTVKGgoN2BCMXWDNL9nY",
            "FTT" => "8JPJJkmDScpcNmBRKGZuPuG2GYAveQgP3t5gFuMymwvF",
            "MER" => "G4AQpTYKH1Fmg38VpFQbv6uKYQMpRhJzNPALhp7hqdrs",
            "SABER" => "8Td9VML1nHxQK6M8VVyzsHo32D7VBk72jSpa9U861z2A",
            "RAY" => "AnLf8tVYCM816gmBjiy8n53eXKKEDydT5piYjjQDPgTB",
            "HXRO" => "B47CC1ULLw1jKTSsr1N1198zrUHp3LPduzepJyzgLn2g",
            "COPE" => "9xYBiDWYsh2fHzpsz3aaCnNHCKWBNtfEDLtU6kS4aFD9",
            "MIR" => "m24crrKFG5jw5ySpvb1k83PRFKVUgzTRm4uvK2WYZtX",
            "SNY" => "BkN8hYgRjhyH5WNBQfDV73ivvdqNKfonCMhiYVJ1D9n9",
            "MNGO" => "79wm3jjcPr6RaNQ4DGvP5KxG1mNd3gEBsg6FsNVFezK4",
            "ADA" => "3pyn4svBbxJ9Wnn3RVeafyLWfzie6yC5eTig2S62v9SC",
            "DOT" => "EcV1X1gY2yb4KXxjVQtTHTbioum2gvmPnFk4zYAt7zne",
            "ATOM" => "CrCpTerNqtZvqLcKqz1k13oVeXV9WkMD2zA9hBKXrsbN",
            "MSOL" => "E4v1BBgoso9s64TQvmyownAVJbhbEPGyzA3qn4n46qj9",
            "UST" => "H8DvrfSaRfUyP1Ytse1exGf7VSinLWtmKNNaBhA4as9P",
            "ALGO" => "HqFyq1wh1xKvL7KDqqT7NJeSPdAqsDqnmBisUC2XdXAX",
            "AVAX" => "Ax9ujW5B9oqcv59N8m6f1BpTBq2rGeGaBcpKjC5UYsXU",
            "ORCA" => "4ivThkX8uRxBpHsdWSqyXYihzKF3zpRGAUCqyuagnLoV",
            "MATIC" => "7KVswB9vkCgeM3SHP7aGDijvdRAHK8P5wi9JXViCrtYh",
            "SLND" => "HkGEau5xY1e8REXUFbwvWWvyJGywkgiAZZFpryyraWqJ",
            "STSOL" => "Bt1hEbY62aMriY1SyQqbeZbm8VmSbQVGBFzSzMuVNWzN",
            "PORT" => "jrMH4afMEodMqirQ7P89q5bGNJxD8uceELcsZaVBDeh",
            "FIDA" => "ETp9eKXVv1dWwHSpsXRUuXHmw24PwRkttCGVgpZEY9zF",
            _ => return (OracleType::Unsupported, zero::id()),
        }
    };

    (OracleType::Pyth, Pubkey::from_str(acc).unwrap())
}
