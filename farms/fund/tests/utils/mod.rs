//! Common functions for tests

use {
    solana_farm_client::client::FarmClient,
    solana_sdk::{
        clock::UnixTimestamp, pubkey::Pubkey, signature::Keypair,
        signer::keypair::read_keypair_file,
    },
};

#[allow(dead_code)]
pub fn get_endpoint_and_keypair() -> (String, Keypair) {
    let cli_config = if let Some(ref config_file) = *solana_cli_config::CONFIG_FILE {
        solana_cli_config::Config::load(config_file).unwrap()
    } else {
        solana_cli_config::Config::default()
    };

    (
        cli_config.json_rpc_url.to_string(),
        read_keypair_file(&cli_config.keypair_path).unwrap_or_else(|_| {
            panic!("Filed to read keypair from \"{}\"", cli_config.keypair_path)
        }),
    )
}

#[allow(dead_code)]
pub fn get_time() -> UnixTimestamp {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as UnixTimestamp
}

#[allow(dead_code)]
pub fn get_token_balance(client: &FarmClient, token_account: &Pubkey) -> u64 {
    if let Ok(balance) = client.rpc_client.get_token_account_balance(token_account) {
        balance.amount.parse::<u64>().unwrap()
    } else {
        0
    }
}

#[allow(dead_code)]
pub fn get_token_ui_balance(client: &FarmClient, token_account: &Pubkey) -> f64 {
    if let Ok(balance) = client.rpc_client.get_token_account_balance(token_account) {
        if let Some(amount) = balance.ui_amount {
            return amount;
        }
    }
    0.0
}
