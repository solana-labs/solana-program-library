use std::str::FromStr;
use {
    log::info,
    solana_farm_client::{client::FarmClient, error::FarmClientError},
    solana_farm_sdk::{
        farm::{FarmRoute, FarmType},
        fund::{Fund, FundType},
        pool::PoolRoute,
        refdb,
        refdb::{find_target_pda, StorageType},
        string::str_to_as64,
        token::{OracleType, Token, TokenType},
        vault::{Vault, VaultStrategy, VaultType},
    },
    solana_sdk::{pubkey::Pubkey, signature::Keypair},
};

#[allow(dead_code)]
pub fn init_fund(
    client: &FarmClient,
    admin_keypair: &Keypair,
    manager_address: &Pubkey,
    fund_name: Option<&str>,
    fund_token_name: Option<&str>,
) -> Result<String, FarmClientError> {
    let rand_name = "FUND_".to_string() + &rand::random::<u32>().to_string();
    let fund_name: &str = if let Some(name) = fund_name {
        name
    } else {
        &rand_name
    };

    let fund_token_name = if let Some(name) = fund_token_name {
        name
    } else {
        fund_name
    };

    client
        .add_program_id(
            admin_keypair,
            "FarmFund",
            &Pubkey::from_str("EmpaFV97uaRPXwWcq8iaHMMTCx7oWkrMSsTcRFJKXHmy").unwrap(),
            solana_farm_sdk::ProgramIDType::Fund,
            None,
        )
        .unwrap();

    let fund_address = client.get_program_id("FarmFund")?;

    if client.get_token(fund_token_name).is_err() {
        let last_index = client.get_refdb_last_index(&StorageType::Token.to_string())?;
        let token = Token {
            name: str_to_as64(fund_token_name)?,
            description: str_to_as64(&(fund_name.to_string() + " Token"))?,
            token_type: TokenType::FundToken,
            refdb_index: Some(last_index),
            refdb_counter: 0u16,
            decimals: 6,
            chain_id: 101,
            mint: Pubkey::find_program_address(
                &[b"fund_token_mint", fund_name.as_bytes()],
                &fund_address,
            )
            .0,
            oracle_type: OracleType::Unsupported,
            oracle_account: None,
            description_account: refdb::find_description_pda(StorageType::Token, fund_token_name).0,
        };

        info!("Recording token {}", fund_token_name);
        client.add_token(admin_keypair, token)?;
    }

    if client.get_fund(fund_name).is_err() {
        let last_index = client.get_refdb_last_index(&StorageType::Fund.to_string())?;
        let fund = Fund {
            name: str_to_as64(fund_name).unwrap(),
            version: 1,
            fund_type: FundType::General,
            official: true,
            refdb_index: Some(last_index),
            refdb_counter: 0u16,
            metadata_bump: find_target_pda(StorageType::Fund, &str_to_as64(fund_name).unwrap()).1,
            authority_bump: Pubkey::find_program_address(
                &[b"fund_authority", fund_name.as_bytes()],
                &fund_address,
            )
            .1,
            fund_token_bump: Pubkey::find_program_address(
                &[b"fund_token_mint", fund_name.as_bytes()],
                &fund_address,
            )
            .1,
            multisig_bump: Pubkey::find_program_address(
                &[b"multisig", fund_name.as_bytes()],
                &fund_address,
            )
            .1,
            fund_program_id: fund_address,
            fund_authority: Pubkey::find_program_address(
                &[b"fund_authority", fund_name.as_bytes()],
                &fund_address,
            )
            .0,
            fund_manager: *manager_address,
            fund_token_ref: find_target_pda(
                StorageType::Token,
                &str_to_as64(fund_token_name).unwrap(),
            )
            .0,
            info_account: Pubkey::find_program_address(
                &[b"info_account", fund_name.as_bytes()],
                &fund_address,
            )
            .0,
            multisig_account: Pubkey::find_program_address(
                &[b"multisig", fund_name.as_bytes()],
                &fund_address,
            )
            .0,
            vaults_assets_info: Pubkey::find_program_address(
                &[b"vaults_assets_info", fund_name.as_bytes()],
                &fund_address,
            )
            .0,
            custodies_assets_info: Pubkey::find_program_address(
                &[b"custodies_assets_info", fund_name.as_bytes()],
                &fund_address,
            )
            .0,
            description_account: Pubkey::find_program_address(
                &[b"description_account", fund_name.as_bytes()],
                &fund_address,
            )
            .0,
        };

        info!("Recording Fund {}", fund_name);
        client.add_fund(admin_keypair, fund)?;

        info!("Initializing Fund {}", fund_name);
        client.init_fund(admin_keypair, fund_name, 0)?;
    }

    Ok(fund_name.to_string())
}

#[allow(dead_code)]
pub fn init_vault(
    client: &FarmClient,
    admin_keypair: &Keypair,
    vault_name: &str,
    vault_token_name: &str,
) -> Result<(), FarmClientError> {
    let vault_address = client.get_program_id("STCVaultRaydium")?;

    if client.get_token(vault_token_name).is_err() {
        let last_index = client.get_refdb_last_index(&StorageType::Token.to_string())?;
        let token = Token {
            name: str_to_as64(vault_token_name)?,
            description: str_to_as64(&(vault_name.to_string() + " Token"))?,
            token_type: TokenType::VtToken,
            refdb_index: Some(last_index),
            refdb_counter: 0u16,
            decimals: 6,
            chain_id: 101,
            mint: Pubkey::find_program_address(
                &[b"vault_token_mint", vault_name.as_bytes()],
                &vault_address,
            )
            .0,
            oracle_type: OracleType::Unsupported,
            oracle_account: None,
            description_account: refdb::find_description_pda(StorageType::Token, vault_token_name)
                .0,
        };

        info!("Recording token {}", vault_token_name);
        client.add_token(admin_keypair, token)?;
    }

    if client.get_vault(vault_name).is_err() {
        let farm_name = "RDM.".to_string() + vault_name.split('.').collect::<Vec<&str>>()[2];
        let farm = client.get_farm(&farm_name).unwrap();
        let lp_token = client
            .get_token_by_ref(&farm.lp_token_ref.unwrap())
            .unwrap();
        let pool = client.find_pools_with_lp(lp_token.name.as_str()).unwrap()[0];
        let farm_second_reward_token_account = match farm.route {
            FarmRoute::Raydium {
                farm_second_reward_token_account,
                ..
            } => farm_second_reward_token_account,
            _ => None,
        };
        let last_index = client.get_refdb_last_index(&StorageType::Vault.to_string())?;
        let vault = Vault {
            name: str_to_as64(vault_name).unwrap(),
            version: 1,
            vault_type: VaultType::AmmStake,
            official: true,
            refdb_index: Some(last_index),
            refdb_counter: 0u16,
            metadata_bump: find_target_pda(StorageType::Vault, &str_to_as64(vault_name).unwrap()).1,
            authority_bump: Pubkey::find_program_address(
                &[b"vault_authority", vault_name.as_bytes()],
                &vault_address,
            )
            .1,
            vault_token_bump: Pubkey::find_program_address(
                &[b"vault_token_mint", vault_name.as_bytes()],
                &vault_address,
            )
            .1,
            lock_required: true,
            unlock_required: true,
            vault_program_id: vault_address,
            vault_authority: Pubkey::find_program_address(
                &[b"vault_authority", vault_name.as_bytes()],
                &vault_address,
            )
            .0,
            vault_token_ref: find_target_pda(
                StorageType::Token,
                &str_to_as64(vault_token_name).unwrap(),
            )
            .0,
            info_account: Pubkey::find_program_address(
                &[b"info_account", vault_name.as_bytes()],
                &vault_address,
            )
            .0,
            multisig_account: Pubkey::find_program_address(
                &[b"multisig", vault_name.as_bytes()],
                &vault_address,
            )
            .0,
            fees_account_a: Some(
                Pubkey::find_program_address(
                    &[b"fees_account_a", vault_name.as_bytes()],
                    &vault_address,
                )
                .0,
            ),
            fees_account_b: if farm.farm_type == FarmType::DualReward
                || farm_second_reward_token_account.is_some()
            {
                Some(
                    Pubkey::find_program_address(
                        &[b"fees_account_b", vault_name.as_bytes()],
                        &vault_address,
                    )
                    .0,
                )
            } else {
                None
            },
            strategy: VaultStrategy::StakeLpCompoundRewards {
                pool_router_id: pool.router_program_id,
                pool_id: match pool.route {
                    PoolRoute::Raydium { amm_id, .. } => amm_id,
                    PoolRoute::Saber { swap_account, .. } => swap_account,
                    PoolRoute::Orca { amm_id, .. } => amm_id,
                },
                pool_ref: client.get_pool_ref(&pool.name).unwrap(),
                farm_router_id: farm.router_program_id,
                farm_id: match farm.route {
                    FarmRoute::Raydium { farm_id, .. } => farm_id,
                    FarmRoute::Saber { quarry, .. } => quarry,
                    FarmRoute::Orca { farm_id, .. } => farm_id,
                },
                farm_ref: client.get_farm_ref(&farm.name).unwrap(),
                lp_token_custody: Pubkey::find_program_address(
                    &[b"lp_token_custody", vault_name.as_bytes()],
                    &vault_address,
                )
                .0,
                token_a_custody: Pubkey::find_program_address(
                    &[b"token_a_custody", vault_name.as_bytes()],
                    &vault_address,
                )
                .0,
                token_b_custody: Some(
                    Pubkey::find_program_address(
                        &[b"token_b_custody", vault_name.as_bytes()],
                        &vault_address,
                    )
                    .0,
                ),
                token_a_reward_custody: Pubkey::find_program_address(
                    &[b"token_a_reward_custody", vault_name.as_bytes()],
                    &vault_address,
                )
                .0,
                token_b_reward_custody: if farm.farm_type == FarmType::DualReward
                    || farm_second_reward_token_account.is_some()
                {
                    Some(
                        Pubkey::find_program_address(
                            &[b"token_b_reward_custody", vault_name.as_bytes()],
                            &vault_address,
                        )
                        .0,
                    )
                } else {
                    None
                },
                vault_stake_info: if farm.version < 4 {
                    Pubkey::find_program_address(
                        &[b"vault_stake_info", vault_name.as_bytes()],
                        &vault_address,
                    )
                    .0
                } else {
                    Pubkey::find_program_address(
                        &[b"vault_stake_info_v4", vault_name.as_bytes()],
                        &vault_address,
                    )
                    .0
                },
                vault_stake_custody: None,
                reward_exchange_pool_id: None,
                reward_exchange_pool_ref: None,
            },
        };

        info!("Recording Vault {}", vault_name);
        client.add_vault(admin_keypair, vault)?;

        info!("Initializing Vault {}", vault_name);
        client.init_vault(admin_keypair, vault_name, 1)?;
        client.init_vault(admin_keypair, vault_name, 2)?;
        client.enable_deposits_vault(admin_keypair, vault_name)?;
        client.enable_withdrawals_vault(admin_keypair, vault_name)?;
    }

    Ok(())
}
