//! Common functions for tests

use {
    solana_farm_client::client::FarmClient,
    solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::keypair::read_keypair_file},
};

#[derive(Copy, Clone)]
pub struct Swap<'a> {
    pub protocol: &'a str,
    pub from_token: &'a str,
    pub to_token: &'a str,
    pub amount: f64,
}

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
pub fn get_token_or_native_balance(client: &FarmClient, wallet: &Pubkey, token_name: &str) -> f64 {
    if token_name != "SOL" {
        if let Ok(balance) = client.get_token_account_balance(wallet, token_name) {
            balance
        } else {
            0.0
        }
    } else if let Ok(balance) = client.get_account_balance(wallet) {
        balance
    } else {
        0.0
    }
}

#[allow(dead_code)]
pub fn get_balance(
    client: &FarmClient,
    wallet: &Pubkey,
    token_name: &str,
    description: &str,
) -> f64 {
    let token_balance = get_token_or_native_balance(client, wallet, token_name);
    println!(
        "  {} balance. {}: {}",
        description, token_name, token_balance
    );
    token_balance
}

#[allow(dead_code)]
pub fn get_balances(
    client: &FarmClient,
    wallet: &Pubkey,
    token_a: &str,
    token_b: &str,
    description: &str,
) -> (f64, f64) {
    let token_a_balance = get_token_or_native_balance(client, wallet, token_a);
    let token_b_balance = get_token_or_native_balance(client, wallet, token_b);
    println!(
        "  {} balances. {}: {}, {}: {}",
        description, token_a, token_a_balance, token_b, token_b_balance
    );
    (token_a_balance, token_b_balance)
}

#[allow(dead_code)]
pub fn get_vault_stake_balance(client: &FarmClient, vault_name: &str) -> f64 {
    let stake_balance = client.get_vault_stake_balance(vault_name).unwrap();
    println!("  Stake balance. {}", stake_balance);
    stake_balance
}
