use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::Write;
use std::str::FromStr;

use anyhow::{anyhow, Result};
use clap::Parser;
use solana_cli_config::{Config, CONFIG_FILE};
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;

#[derive(Debug, Default, Clone, PartialEq, Eq, Parser)]
pub struct CliOverrides {
    #[clap(long = "url")]
    pub rpc_url: Option<String>,
    #[clap(long = "idl-program")]
    pub idl_program: Option<Pubkey>,
}

pub struct CliConfig {
    pub rpc_url: String,
    pub idl_program: Pubkey,
}

fn get_rpc_url(overrides: &CliOverrides) -> String {
    if overrides.rpc_url.is_some() {
        overrides.rpc_url.clone().unwrap().to_string()
    } else {
        let cfg = Config::load(&CONFIG_FILE.as_ref().unwrap())
            .expect("Solana CLI must be setup to use this tool. Run `solana config set`.");
        cfg.json_rpc_url.to_string()
    }
}

fn default_idl_program() -> Pubkey {
    Pubkey::from_str("uipLuk57b21BUNutsX2kVxCyTXZeBmfyd4dswaRjWaL").unwrap()
}

pub fn with_solana_config<R>(overrides: &CliOverrides, f: impl FnOnce(CliConfig) -> R) -> R {
    let cfg = CliConfig {
        rpc_url: get_rpc_url(&overrides).clone(),
        idl_program: overrides
            .idl_program
            .unwrap_or(default_idl_program())
            .clone(),
    };
    f(cfg)
}

pub fn read_keypair_file(filepath: &str, name: &str) -> Result<Keypair> {
    solana_sdk::signature::read_keypair_file(filepath)
        .map_err(|_| anyhow!("Unable to read {} keypair file", name))
}

pub fn serialize_idl(bytes: Vec<u8>) -> Result<Vec<u8>> {
    let mut encoded = ZlibEncoder::new(Vec::new(), Compression::default());
    encoded.write_all(&bytes)?;
    encoded.finish().map_err(Into::into)
}
