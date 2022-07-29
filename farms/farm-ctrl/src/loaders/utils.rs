//! Common helpers.

use {
    serde_json::Value, solana_farm_client::client::FarmClient, solana_farm_sdk::token::GitToken,
    solana_sdk::pubkey::Pubkey, std::str::FromStr,
};

pub fn convert_raydium_program_id(client: &FarmClient, program_id: &str) -> Pubkey {
    match program_id {
        "LIQUIDITY_POOL_PROGRAM_ID_V2" => client.get_program_id("RaydiumV2").unwrap(),
        "LIQUIDITY_POOL_PROGRAM_ID_V3" => client.get_program_id("RaydiumV3").unwrap(),
        "LIQUIDITY_POOL_PROGRAM_ID_V4" => client.get_program_id("RaydiumV4").unwrap(),
        "STAKE_PROGRAM_ID" => client.get_program_id("RaydiumStake").unwrap(),
        "STAKE_PROGRAM_ID_V4" => client.get_program_id("RaydiumStakeV4").unwrap(),
        "STAKE_PROGRAM_ID_V5" => client.get_program_id("RaydiumStakeV5").unwrap(),
        _ => convert_pubkey(program_id),
    }
}

pub fn convert_serum_program_id(client: &FarmClient, program_id: &str) -> Pubkey {
    match program_id {
        "SERUM_PROGRAM_ID_V2" => client.get_program_id("SerumV2").unwrap(),
        "SERUM_PROGRAM_ID_V3" => client.get_program_id("SerumV3").unwrap(),
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
            .replace(' ', "_")
            .replace('/', "_")
            .replace('.', "_")
    } else {
        name.to_uppercase()
            .replace(' ', "_")
            .replace('/', "_")
            .replace('.', "_")
            .replace('-', "_")
    }
}

pub fn is_saber_wrapped(token: &GitToken) -> bool {
    token.symbol.len() > 3 && token.tags.contains(&String::from("saber-dec-wrapped"))
}

pub fn get_saber_token_name(client: &FarmClient, token: &GitToken) -> String {
    if is_saber_wrapped(token) {
        client
            .get_token_with_mint(&convert_pubkey(
                token.extra["extensions"]["assetContract"].as_str().unwrap(),
            ))
            .unwrap()
            .name
            .to_string()
    } else {
        client
            .get_token_with_mint(&token.address)
            .unwrap()
            .name
            .to_string()
    }
}

pub fn get_saber_pool_name(client: &FarmClient, token1: &GitToken, token2: &GitToken) -> String {
    let token1_name = get_saber_token_name(client, token1);
    let token2_name = get_saber_token_name(client, token2);
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
