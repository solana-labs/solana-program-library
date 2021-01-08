use serde::{Deserialize, Serialize};
use solana_program::bpf_loader_upgradeable;
use solana_program_test::*;
use solana_sdk::{
    account::Account,
    bpf_loader_upgradeable::UpgradeableLoaderState,
    pubkey::Pubkey,
    signature::{read_keypair_file, Signer},
};
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
    pub fn insert_upgradeable_program(&mut self, program_id: Pubkey, filename: &str) {
        let program_file = find_file(filename).expect(&format!("couldn't find {}", filename));
        let program_data = read_file(program_file);
        let programdata_address = Pubkey::new_unique();
        let upgrade_authority_keypair =
            read_keypair_file("tests/fixtures/lending_market.json").unwrap();
        let programdata_data_offset = UpgradeableLoaderState::programdata_data_offset().unwrap();
        let programdata_space = program_data.len() + programdata_data_offset;
        let programdata_state = UpgradeableLoaderState::ProgramData {
            slot: 0,
            upgrade_authority_address: Some(upgrade_authority_keypair.pubkey()),
        };
        let mut programdata_account = Account::new_data_with_space(
            u32::MAX as u64,
            &programdata_state,
            programdata_space,
            &bpf_loader_upgradeable::id(),
        )
        .unwrap();
        programdata_account.data
            [programdata_data_offset..programdata_data_offset + program_data.len()]
            .copy_from_slice(&program_data[..]);
        self.0.insert(
            programdata_address.to_string(),
            Base64Account {
                owner: programdata_account.owner.to_string(),
                balance: programdata_account.lamports,
                executable: false,
                data: base64::encode(&programdata_account.data),
            },
        );

        let program_state = UpgradeableLoaderState::Program {
            programdata_address,
        };
        let program_account = Account::new_data(
            u32::MAX as u64,
            &program_state,
            &bpf_loader_upgradeable::id(),
        )
        .unwrap();
        self.0.insert(
            program_id.to_string(),
            Base64Account {
                owner: program_account.owner.to_string(),
                balance: program_account.lamports,
                executable: true,
                data: base64::encode(&program_account.data),
            },
        );
    }

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
