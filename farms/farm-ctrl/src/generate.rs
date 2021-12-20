//! Handlers for generate command

use {
    crate::config::Config,
    log::info,
    serde_json::Value,
    solana_farm_client::client::FarmClient,
    solana_farm_sdk::{
        farm::{FarmRoute, FarmType},
        git_token::GitToken,
        id::main_router_admin,
        program::pda::find_target_pda,
        refdb::StorageType,
        string::{str_to_as64, to_pretty_json},
        vault::{Vault, VaultStrategy, VaultType},
    },
    solana_sdk::pubkey::Pubkey,
    std::collections::HashMap,
    std::str::FromStr,
};

pub fn generate_rdm_stc_vault(
    client: &FarmClient,
    _config: &Config,
    vault_address: &Pubkey,
    vault_name: &str,
    token_name: &str,
) {
    let farm_name = "RDM.".to_string() + vault_name.split('.').collect::<Vec<&str>>()[2];
    let farm = client.get_farm(&farm_name).unwrap();
    let lp_token = client
        .get_token_by_ref(&farm.lp_token_ref.unwrap())
        .unwrap();
    let pool = client.find_pools_with_lp(lp_token.name.as_str()).unwrap()[0];
    let farm_reward_token_b_account = match farm.route {
        FarmRoute::Raydium {
            farm_reward_token_b_account,
            ..
        } => farm_reward_token_b_account,
        _ => None,
    };
    let vault = Vault {
        name: str_to_as64(vault_name).unwrap(),
        version: 1,
        vault_type: VaultType::AmmStake,
        official: true,
        refdb_index: None,
        refdb_counter: 0,
        metadata_bump: find_target_pda(StorageType::Vault, &str_to_as64(vault_name).unwrap()).1,
        authority_bump: Pubkey::find_program_address(
            &[b"vault_authority", vault_name.as_bytes()],
            vault_address,
        )
        .1,
        vault_token_bump: Pubkey::find_program_address(
            &[b"vault_token_mint", vault_name.as_bytes()],
            vault_address,
        )
        .1,
        lock_required: false,
        unlock_required: true,
        vault_program_id: *vault_address,
        vault_authority: Pubkey::find_program_address(
            &[b"vault_authority", vault_name.as_bytes()],
            vault_address,
        )
        .0,
        vault_token_ref: find_target_pda(StorageType::Token, &str_to_as64(token_name).unwrap()).0,
        info_account: Pubkey::find_program_address(
            &[b"info_account", vault_name.as_bytes()],
            vault_address,
        )
        .0,
        admin_account: main_router_admin::id(),
        fees_account_a: Some(
            Pubkey::find_program_address(
                &[b"fees_account_a", vault_name.as_bytes()],
                vault_address,
            )
            .0,
        ),
        fees_account_b: if farm.farm_type == FarmType::DualReward
            || farm_reward_token_b_account.is_some()
        {
            Some(
                Pubkey::find_program_address(
                    &[b"fees_account_b", vault_name.as_bytes()],
                    vault_address,
                )
                .0,
            )
        } else {
            None
        },
        strategy: VaultStrategy::StakeLpCompoundRewards {
            pool_id_ref: client.get_pool_ref(pool.name.as_str()).unwrap(),
            farm_id_ref: client.get_farm_ref(&farm_name).unwrap(),
            lp_token_custody: Pubkey::find_program_address(
                &[b"lp_token_custody", vault_name.as_bytes()],
                vault_address,
            )
            .0,
            token_a_custody: Pubkey::find_program_address(
                &[b"token_a_custody", vault_name.as_bytes()],
                vault_address,
            )
            .0,
            token_b_custody: Some(
                Pubkey::find_program_address(
                    &[b"token_b_custody", vault_name.as_bytes()],
                    vault_address,
                )
                .0,
            ),
            token_a_reward_custody: Pubkey::find_program_address(
                &[b"token_a_reward_custody", vault_name.as_bytes()],
                vault_address,
            )
            .0,
            token_b_reward_custody: if farm.farm_type == FarmType::DualReward
                || farm_reward_token_b_account.is_some()
            {
                Some(
                    Pubkey::find_program_address(
                        &[b"token_b_reward_custody", vault_name.as_bytes()],
                        vault_address,
                    )
                    .0,
                )
            } else {
                None
            },
            vault_stake_info: if farm.version < 4 {
                Pubkey::find_program_address(
                    &[b"vault_stake_info", vault_name.as_bytes()],
                    vault_address,
                )
                .0
            } else {
                Pubkey::find_program_address(
                    &[b"vault_stake_info_v4", vault_name.as_bytes()],
                    vault_address,
                )
                .0
            },
        },
    };
    println!("{}", to_pretty_json(&vault).unwrap());

    let token = GitToken {
        chain_id: 101,
        address: Pubkey::find_program_address(
            &[b"vault_token_mint", vault_name.as_bytes()],
            vault_address,
        )
        .0
        .to_string(),
        symbol: token_name.to_string(),
        name: "Raydium ".to_string()
            + token_name.split('.').collect::<Vec<&str>>()[3]
            + " Stake Compound Vault Token",
        decimals: client
            .get_token_by_ref(&farm.lp_token_ref.unwrap())
            .unwrap()
            .decimals as i32,
        logo_uri: String::default(),
        tags: vec!["vt-token".to_string()],
        extra: HashMap::<String, Value>::default(),
    };
    println!("{}", to_pretty_json(&token).unwrap());
}

pub fn generate_sbr_stc_vault(
    client: &FarmClient,
    _config: &Config,
    vault_address: &Pubkey,
    vault_name: &str,
    token_name: &str,
) {
    let farm_name = "SBR.".to_string() + vault_name.split('.').collect::<Vec<&str>>()[2];
    let farm = client.get_farm(&farm_name).unwrap();
    let lp_token = client
        .get_token_by_ref(&farm.lp_token_ref.unwrap())
        .unwrap();
    let pool = client.find_pools_with_lp(lp_token.name.as_str()).unwrap()[0];
    let (is_token_a_wrapped, is_token_b_wrapped) = client
        .pool_has_saber_wrapped_tokens(pool.name.as_str())
        .unwrap();
    let quarry = match farm.route {
        FarmRoute::Saber { quarry, .. } => quarry,
        _ => unreachable!(),
    };
    let (vault_authority, authority_bump) =
        Pubkey::find_program_address(&[b"vault_authority", vault_name.as_bytes()], vault_address);

    let vault = Vault {
        name: str_to_as64(vault_name).unwrap(),
        version: 1,
        vault_type: VaultType::AmmStake,
        official: true,
        refdb_index: None,
        refdb_counter: 0,
        metadata_bump: find_target_pda(StorageType::Vault, &str_to_as64(vault_name).unwrap()).1,
        authority_bump,
        vault_token_bump: Pubkey::find_program_address(
            &[b"vault_token_mint", vault_name.as_bytes()],
            vault_address,
        )
        .1,
        lock_required: true,
        unlock_required: false,
        vault_program_id: *vault_address,
        vault_authority,
        vault_token_ref: find_target_pda(StorageType::Token, &str_to_as64(token_name).unwrap()).0,
        info_account: Pubkey::find_program_address(
            &[b"info_account", vault_name.as_bytes()],
            vault_address,
        )
        .0,
        admin_account: main_router_admin::id(),
        fees_account_a: Some(
            Pubkey::find_program_address(
                &[b"fees_account_a", vault_name.as_bytes()],
                vault_address,
            )
            .0,
        ),
        fees_account_b: Some(
            Pubkey::find_program_address(
                &[b"fees_account_b", vault_name.as_bytes()],
                vault_address,
            )
            .0,
        ),
        strategy: VaultStrategy::StakeLpCompoundRewards {
            pool_id_ref: client.get_pool_ref(pool.name.as_str()).unwrap(),
            farm_id_ref: client.get_farm_ref(&farm_name).unwrap(),
            lp_token_custody: Pubkey::find_program_address(
                &[b"lp_token_custody", vault_name.as_bytes()],
                vault_address,
            )
            .0,
            token_a_custody: Pubkey::find_program_address(
                &[b"token_a_custody", vault_name.as_bytes()],
                vault_address,
            )
            .0,
            token_b_custody: if is_token_a_wrapped || is_token_b_wrapped {
                Some(
                    Pubkey::find_program_address(
                        &[b"token_b_custody", vault_name.as_bytes()],
                        vault_address,
                    )
                    .0,
                )
            } else {
                None
            },
            token_a_reward_custody: Pubkey::find_program_address(
                &[b"token_a_reward_custody", vault_name.as_bytes()],
                vault_address,
            )
            .0,
            token_b_reward_custody: Some(
                Pubkey::find_program_address(
                    &[b"token_b_reward_custody", vault_name.as_bytes()],
                    vault_address,
                )
                .0,
            ),
            vault_stake_info: Pubkey::find_program_address(
                &[b"Miner", &quarry.to_bytes(), &vault_authority.to_bytes()],
                &quarry_mine::id(),
            )
            .0,
        },
    };
    println!("{},", to_pretty_json(&vault).unwrap());

    let token = GitToken {
        chain_id: 101,
        address: Pubkey::find_program_address(
            &[b"vault_token_mint", vault_name.as_bytes()],
            vault_address,
        )
        .0
        .to_string(),
        symbol: token_name.to_string(),
        name: "Saber ".to_string()
            + token_name.split('.').collect::<Vec<&str>>()[3]
            + " Stake Compound Vault Token",
        decimals: client
            .get_token_by_ref(&farm.lp_token_ref.unwrap())
            .unwrap()
            .decimals as i32,
        logo_uri: String::default(),
        tags: vec!["vt-token".to_string()],
        extra: HashMap::<String, Value>::default(),
    };
    println!("{}", to_pretty_json(&token).unwrap());
}

pub fn generate(
    client: &FarmClient,
    config: &Config,
    target: StorageType,
    object: &str,
    param1: &str,
    param2: &str,
) {
    info!(
        "Generating json boilerplate for {} {} {}...",
        target, object, param1
    );

    match target {
        StorageType::Vault => {
            if param1.starts_with("RDM.") {
                generate_rdm_stc_vault(
                    client,
                    config,
                    &Pubkey::from_str(object).unwrap(),
                    param1,
                    param2,
                );
            } else if param1.starts_with("SBR.") {
                generate_sbr_stc_vault(
                    client,
                    config,
                    &Pubkey::from_str(object).unwrap(),
                    param1,
                    param2,
                );
            } else {
                panic!("Unexpected Vault name: {}", param1);
            }
        }
        _ => {
            panic!("Target is not supported: {}", target);
        }
    }

    info!("Done.")
}
