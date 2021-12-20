//! Handlers for print_pda and print_size commands

use {
    crate::config::Config,
    log::info,
    solana_farm_client::client::FarmClient,
    solana_farm_sdk::{
        farm::Farm, pool::Pool, program::pda, refdb::ReferenceType, refdb::StorageType,
        token::Token, vault::Vault,
    },
};

pub fn print_pda(_client: &FarmClient, _config: &Config, target: StorageType) {
    info!(
        "{} RefDB address: {}",
        target,
        pda::find_refdb_pda(&target.to_string()).0
    );
}

pub fn print_pda_all(client: &FarmClient, config: &Config) {
    print_pda(client, config, StorageType::Program);
    print_pda(client, config, StorageType::Token);
    print_pda(client, config, StorageType::Pool);
    print_pda(client, config, StorageType::Farm);
    print_pda(client, config, StorageType::Vault);
}

pub fn print_size(client: &FarmClient, _config: &Config, target: StorageType) {
    let refdb_size = StorageType::get_storage_size_for_max_records(target, ReferenceType::Pubkey);
    let target_size = match target {
        StorageType::Program => 0,
        StorageType::Token => Token::LEN,
        StorageType::Pool => Pool::MAX_LEN,
        StorageType::Farm => Farm::MAX_LEN,
        StorageType::Vault => Vault::MAX_LEN,
        _ => 0,
    };
    let target_max_recs = StorageType::get_default_max_records(target, ReferenceType::Pubkey);
    let refdb_cost = client
        .rpc_client
        .get_minimum_balance_for_rent_exemption(refdb_size)
        .unwrap();
    let target_cost = client
        .rpc_client
        .get_minimum_balance_for_rent_exemption(target_size)
        .unwrap();

    info!("{} recs / size / cost:", target.to_string());
    info!(
        "RefDB: {} / {} / {}",
        target_max_recs,
        refdb_size,
        lam_to_sol(refdb_cost)
    );
    info!(
        "Target: {} / {} / {}",
        1,
        target_size,
        lam_to_sol(target_cost)
    );
    info!(
        "Target Max: {} / {} / {}\n",
        target_max_recs,
        target_size * target_max_recs,
        lam_to_sol(target_cost * (target_max_recs as u64))
    );
}

pub fn print_size_all(client: &FarmClient, config: &Config) {
    print_size(client, config, StorageType::Program);
    print_size(client, config, StorageType::Token);
    print_size(client, config, StorageType::Pool);
    print_size(client, config, StorageType::Farm);
    print_size(client, config, StorageType::Vault);
}

fn lam_to_sol(amount: u64) -> f64 {
    (amount as f64) / 10f64.powi(9)
}
