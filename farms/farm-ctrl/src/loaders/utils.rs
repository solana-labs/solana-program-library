//! Common helpers.

use {
    serde_json::Value, solana_farm_client::client::FarmClient,
    solana_farm_sdk::git_token::GitToken, solana_sdk::pubkey::Pubkey, std::str::FromStr,
};

pub fn convert_raydium_program_id(client: &FarmClient, program_id: &str) -> Pubkey {
    match program_id {
        "LIQUIDITY_POOL_PROGRAM_ID_V2" => client.get_program_id(&"RaydiumV2".to_string()).unwrap(),
        "LIQUIDITY_POOL_PROGRAM_ID_V3" => client.get_program_id(&"RaydiumV3".to_string()).unwrap(),
        "LIQUIDITY_POOL_PROGRAM_ID_V4" => client.get_program_id(&"RaydiumV4".to_string()).unwrap(),
        "STAKE_PROGRAM_ID" => client.get_program_id(&"RaydiumStake".to_string()).unwrap(),
        "STAKE_PROGRAM_ID_V4" => client
            .get_program_id(&"RaydiumStakeV4".to_string())
            .unwrap(),
        "STAKE_PROGRAM_ID_V5" => client
            .get_program_id(&"RaydiumStakeV5".to_string())
            .unwrap(),
        _ => convert_pubkey(program_id),
    }
}

pub fn convert_serum_program_id(client: &FarmClient, program_id: &str) -> Pubkey {
    match program_id {
        "SERUM_PROGRAM_ID_V2" => client.get_program_id(&"SerumV2".to_string()).unwrap(),
        "SERUM_PROGRAM_ID_V3" => client.get_program_id(&"SerumV3".to_string()).unwrap(),
        _ => convert_pubkey(program_id),
    }
}

pub fn convert_pubkey(pubkey_as_string: &str) -> Pubkey {
    Pubkey::from_str(pubkey_as_string).unwrap_or_else(|_| {
        panic!(
            "Failed to convert the string to pubkey {}",
            pubkey_as_string
        )
    })
}

#[allow(dead_code)]
pub fn convert_optional_pubkey(pubkey_as_string: &str) -> Option<Pubkey> {
    if pubkey_as_string.is_empty() {
        None
    } else {
        Some(Pubkey::from_str(pubkey_as_string).unwrap_or_else(|_| {
            panic!(
                "Failed to convert the string to pubkey {}",
                pubkey_as_string
            )
        }))
    }
}

pub fn json_to_pubkey(input: &Value) -> Pubkey {
    if let Ok(pubkey) = Pubkey::from_str(input.as_str().unwrap()) {
        return pubkey;
    }
    panic!("Failed to convert the input to a pubkey: {}", input);
}

pub fn normalize_name(name: &str, allow_dashes: bool) -> String {
    if allow_dashes {
        name.to_uppercase()
            .replace(" ", "_")
            .replace("/", "_")
            .replace(".", "_")
    } else {
        name.to_uppercase()
            .replace(" ", "_")
            .replace("/", "_")
            .replace(".", "_")
            .replace("-", "_")
    }
}

pub fn get_saber_lp_token_name(lp_token: &str) -> String {
    "LP.SBR.".to_string() + &normalize_name(lp_token.split(' ').collect::<Vec<&str>>()[0], true)
}

pub fn extract_saber_wrapped_token_name(name: &str) -> String {
    if name.len() > 3
        && (&name[..1] == "s" || &name[..1] == "S")
        && vec!["_8", "_9", "10"].contains(&&name[name.len() - 2..])
    {
        name.split('_').collect::<Vec<&str>>()[0][1..].to_string()
    } else {
        panic!("Unexpected Saber wrapped token name {}", name);
    }
}

pub fn is_saber_wrapped(token: &GitToken) -> bool {
    token.symbol.len() > 3 && token.tags.contains(&String::from("saber-decimal-wrapped"))
}

pub fn get_saber_pool_name(token1: &GitToken, token2: &GitToken) -> String {
    let token1_symbol_norm = normalize_name(&token1.symbol, false);
    let token2_symbol_norm = normalize_name(&token2.symbol, false);
    let token1_name = if is_saber_wrapped(token1) {
        extract_saber_wrapped_token_name(&token1_symbol_norm)
    } else {
        token1_symbol_norm
    };
    let token2_name = if is_saber_wrapped(token2) {
        extract_saber_wrapped_token_name(&token2_symbol_norm)
    } else {
        token2_symbol_norm
    };
    format!("SBR.{}-{}-V1", token1_name, token2_name)
}

pub fn get_token_ref_with_mint(client: &FarmClient, token_mint: &Pubkey) -> Pubkey {
    client
        .get_token_ref(
            client
                .get_token_with_mint(token_mint)
                .unwrap()
                .name
                .as_str(),
        )
        .unwrap()
}
