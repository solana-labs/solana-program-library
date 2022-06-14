//! Funds loader.

use {
    crate::config::Config,
    log::info,
    solana_farm_client::client::FarmClient,
    solana_farm_sdk::{fund::Fund, refdb::StorageType},
};

pub fn load(client: &FarmClient, config: &Config, data: &str, remove_mode: bool) {
    let parsed: serde_json::Value = serde_json::from_str(data).unwrap();
    let mut last_index = client
        .get_refdb_last_index(&StorageType::Fund.to_string())
        .expect("Fund RefDB query error");

    if parsed["name"] != "Solana Funds List" {
        panic!("Unsupported funds file");
    }
    let funds = parsed["funds"].as_array().unwrap();
    for val in funds {
        let json_fund: Fund = serde_json::from_value(val.clone()).unwrap();
        if !remove_mode {
            if config.skip_existing && client.get_fund(&json_fund.name).is_ok() {
                info!("Skipping existing Fund \"{}\"...", json_fund.name);
                continue;
            }
            info!("Writing Fund \"{}\" to on-chain RefDB...", json_fund.name);
        } else {
            info!(
                "Removing Fund \"{}\" from on-chain RefDB...",
                json_fund.name
            );
            client
                .remove_fund(config.keypair.as_ref(), &json_fund.name)
                .unwrap();
            continue;
        }
        let (index, counter) = if let Ok(fund) = client.get_fund(&json_fund.name) {
            (fund.refdb_index, fund.refdb_counter)
        } else {
            last_index += 1;
            (Some(last_index - 1), 0u16)
        };
        let fund = Fund {
            name: json_fund.name,
            version: json_fund.version as u16,
            fund_type: json_fund.fund_type,
            official: json_fund.official,
            refdb_index: index,
            refdb_counter: counter,
            metadata_bump: json_fund.metadata_bump,
            authority_bump: json_fund.authority_bump,
            fund_token_bump: json_fund.fund_token_bump,
            multisig_bump: json_fund.multisig_bump,
            fund_program_id: json_fund.fund_program_id,
            fund_authority: json_fund.fund_authority,
            fund_manager: json_fund.fund_manager,
            fund_token_ref: json_fund.fund_token_ref,
            info_account: json_fund.info_account,
            multisig_account: json_fund.multisig_account,
            vaults_assets_info: json_fund.vaults_assets_info,
            custodies_assets_info: json_fund.custodies_assets_info,
            description_account: json_fund.description_account,
        };

        client.add_fund(config.keypair.as_ref(), fund).unwrap();
    }
}
