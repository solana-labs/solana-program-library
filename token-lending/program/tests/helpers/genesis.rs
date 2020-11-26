use serde::{Deserialize, Serialize};
use solana_program_test::*;
use solana_sdk::{account::Account, pubkey::Pubkey};
use std::{collections::HashMap, fs::File, io::Write, path::Path};

/// An account where the data is encoded as a Base64 string.
#[derive(Serialize, Deserialize, Debug)]
pub struct Base64Account {
    pub balance: u64,
    pub owner: String,
    pub data: String,
    pub executable: bool,
}

#[derive(Default)]
pub struct GenesisAccounts(HashMap<String, Base64Account>);

impl GenesisAccounts {
    pub async fn fetch_and_insert(&mut self, banks_client: &mut BanksClient, pubkey: Pubkey) {
        let account: Account = banks_client.get_account(pubkey).await.unwrap().unwrap();
        self.0.insert(
            pubkey.to_string(),
            Base64Account {
                owner: account.owner.to_string(),
                balance: u32::MAX as u64,
                executable: false,
                data: base64::encode(&account.data),
            },
        );
    }

    pub fn write_yaml(&self) {
        let serialized = serde_yaml::to_string(&self.0).unwrap();
        let path = Path::new("../../target/deploy/lending_accounts.yml");
        let mut file = File::create(path).unwrap();
        file.write_all(&serialized.into_bytes()).unwrap();
    }
}
