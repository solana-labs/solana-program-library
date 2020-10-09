//! Themis client

use solana_banks_client::start_tcp_client;
use solana_cli_config::{Config, CONFIG_FILE};
use solana_sdk::signature::read_keypair_file;
use spl_themis_ristretto_client::test_e2e;
use std::path::Path;
use tokio::runtime::Runtime;
use url::Url;

fn main() {
    let config_file = CONFIG_FILE.as_ref().unwrap();
    let config = if Path::new(&config_file).exists() {
        Config::load(&config_file).unwrap()
    } else {
        Config::default()
    };
    let rpc_banks_url = Config::compute_rpc_banks_url(&config.json_rpc_url);
    let url = Url::parse(&rpc_banks_url).unwrap();
    let host_port = (url.host_str().unwrap(), url.port().unwrap());

    Runtime::new().unwrap().block_on(async {
        let mut banks_client = start_tcp_client(host_port).await.unwrap();
        let policies = vec![1u64.into(), 2u64.into()];
        let sender_keypair = read_keypair_file(&config.keypair_path).unwrap();
        test_e2e(
            &mut banks_client,
            &spl_themis_ristretto::id(),
            sender_keypair,
            policies,
            1_000,
            3u64.into(),
        )
        .await
        .unwrap();
    });
}
