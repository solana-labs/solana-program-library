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

impl From<Account> for Base64Account {
    fn from(account: Account) -> Self {
        Self {
            owner: account.owner.to_string(),
            balance: account.lamports,
            executable: account.executable,
            data: base64::encode(&account.data),
        }
    }
}

#[derive(Default)]
pub struct GenesisAccounts(HashMap<String, Base64Account>);

impl GenesisAccounts {
    pub fn insert_upgradeable_program(&mut self, program_id: Pubkey, filename: &str) {
        let program_file =
            find_file(filename).unwrap_or_else(|| panic!("couldn't find {}", filename));
        let program_data = read_file(program_file);
        let upgrade_authority_keypair =
            read_keypair_file("tests/fixtures/lending_market_owner.json").unwrap();

        let programdata_address =
            Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader_upgradeable::id()).0;
        let programdata_data_offset = UpgradeableLoaderState::programdata_data_offset().unwrap();
        let programdata_space = 2 * program_data.len() + programdata_data_offset;
        let mut programdata_account = Account::new_data_with_space(
            u32::MAX as u64,
            &UpgradeableLoaderState::ProgramData {
                slot: 0,
                upgrade_authority_address: Some(upgrade_authority_keypair.pubkey()),
            },
            programdata_space,
            &bpf_loader_upgradeable::id(),
        )
        .unwrap();

        programdata_account.data
            [programdata_data_offset..programdata_data_offset + program_data.len()]
            .copy_from_slice(&program_data[..]);

        self.0
            .insert(programdata_address.to_string(), programdata_account.into());

        let mut program_account = Account::new_data(
            u32::MAX as u64,
            &UpgradeableLoaderState::Program {
                programdata_address,
            },
            &bpf_loader_upgradeable::id(),
        )
        .unwrap();
        program_account.executable = true;

        self.0
            .insert(program_id.to_string(), program_account.into());
    }

    pub async fn fetch_and_insert(&mut self, banks_client: &mut BanksClient, pubkey: Pubkey) {
        let mut account: Account = banks_client.get_account(pubkey).await.unwrap().unwrap();
        account.lamports = u32::MAX as u64;
        self.0.insert(pubkey.to_string(), account.into());
    }

    pub fn write_yaml(&self) {
        let serialized = serde_yaml::to_string(&self.0).unwrap();
        let path = Path::new("../../target/deploy/lending_accounts.yml");
        let mut file = File::create(path).unwrap();
        file.write_all(&serialized.into_bytes()).unwrap();
    }
}
