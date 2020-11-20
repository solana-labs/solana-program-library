//! Themis client

use solana_cli_config::{Config, CONFIG_FILE};
use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::read_keypair_file;
use spl_themis_ristretto_client::test_e2e;
use std::path::Path;

fn main() {
    let config_file = CONFIG_FILE.as_ref().unwrap();
    let config = if Path::new(&config_file).exists() {
        Config::load(&config_file).unwrap()
    } else {
        Config::default()
    };

    let client = RpcClient::new(config.json_rpc_url);
    let policies = vec![1u64.into(), 2u64.into()];
    let sender_keypair = read_keypair_file(&config.keypair_path).unwrap();
    test_e2e(
        &client,
        &spl_themis_ristretto::id(),
        sender_keypair,
        policies,
        1_000,
        3u64.into(),
    )
    .unwrap();
}
