//! Themis client

#[cfg(test)]
mod tests {
    use bn::Fr;
    use solana_banks_client::start_tcp_client;
    use solana_cli_config::{Config, CONFIG_FILE};
    use solana_sdk::signature::read_keypair_file;
    use spl_themis_client::test_e2e;
    use std::{path::Path, process};
    use tokio::runtime::Runtime;
    use url::Url;

    #[test]
    fn test_tcp_e2e_2ads() {
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
            dbg!(&host_port);
            let mut banks_client = start_tcp_client(host_port).await.unwrap();
            let policies = vec![Fr::new(1u64.into()).unwrap(), Fr::new(2u64.into()).unwrap()];
            dbg!(&policies);
            let sender_keypair =
                read_keypair_file("/Users/gregfitzgerald/.config/solana/id.json").unwrap();
            test_e2e(
                &mut banks_client,
                sender_keypair,
                policies,
                Fr::new(3u64.into()).unwrap(),
            )
            .await
            .unwrap();
        });
    }
}
