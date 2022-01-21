//! Configuration management

use {
    serde_derive::{Deserialize, Serialize},
    std::{
        fs::{create_dir_all, File},
        io::{self, Write},
        path::Path,
    },
};

lazy_static! {
    pub static ref CONFIG_FILE: Option<String> = {
        dirs_next::home_dir().map(|mut path| {
            path.extend(&[".config", "solana", "farm", "rpc_config.yml"]);
            path.to_str().unwrap().to_string()
        })
    };
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Config {
    pub json_rpc_url: String,
    pub websocket_url: String,
    pub max_threads: u32,
    pub token_list_url: String,
    pub farm_client_url: String,
}

impl Default for Config {
    fn default() -> Self {
        let json_rpc_url = "http://127.0.0.1:9000".to_string();
        let websocket_url = "wss://127.0.0.1:9001".to_string();
        let token_list_url = "https://raw.githubusercontent.com/solana-labs/token-list/main/src/tokens/solana.tokenlist.json".to_string();
        let farm_client_url = "http://127.0.0.1:8899".to_string();
        let max_threads = 4;

        Self {
            json_rpc_url,
            websocket_url,
            max_threads,
            token_list_url,
            farm_client_url,
        }
    }
}

impl Config {
    pub fn load(&mut self, config_file: &str) -> Result<(), io::Error> {
        let file = File::open(config_file)?;
        *self = serde_yaml::from_reader(file)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("{:?}", err)))?;
        Ok(())
    }

    pub fn save(&self, config_file: &str) -> Result<(), io::Error> {
        let serialized = serde_yaml::to_string(self)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("{:?}", err)))?;

        if let Some(outdir) = Path::new(config_file).parent() {
            create_dir_all(outdir)?;
        }
        let mut file = File::create(config_file)?;
        file.write_all(&serialized.into_bytes())?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_default() {
        let config: Config = Default::default();
        assert_eq!(config.json_rpc_url, "http://127.0.0.1:9000");
        assert_eq!(config.websocket_url, "wss://127.0.0.1:9001");
        assert_eq!(config.farm_client_url, "http://127.0.0.1:8899");
        assert_eq!(config.max_threads, 4);
    }

    #[test]
    fn test_load_save() {
        let config = Config {
            json_rpc_url: "http://test:9000".to_string(),
            websocket_url: "wss://test:9001".to_string(),
            max_threads: 99,
            token_list_url: "none".to_string(),
            farm_client_url: "http://test_farm:8899".to_string(),
        };
        let _ = config.save("_test_config.yml");

        let mut config2: Config = Default::default();
        let _ = config2.load("_test_config.yml");

        assert_eq!(config.json_rpc_url, config2.json_rpc_url);
        assert_eq!(config.websocket_url, config2.websocket_url);
        assert_eq!(config.max_threads, config2.max_threads);
        assert_eq!(config.farm_client_url, config2.farm_client_url);
    }
}
