//! Handlers for Fund management commands

use {
    crate::config::Config,
    log::info,
    solana_farm_client::client::FarmClient,
    solana_farm_sdk::{
        fund::{Fund, FundAssetsTrackingConfig, FundCustodyType, FundSchedule, FundVaultType},
        string::to_pretty_json,
        Protocol,
    },
    solana_sdk::{clock::UnixTimestamp, pubkey::Pubkey},
};

pub fn init(client: &FarmClient, config: &Config, fund_names: &str, step: u64) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!("Initializing Fund {}...", fund);
        info!(
            "Signature: {}",
            client
                .init_fund(config.keypair.as_ref(), fund, step)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn set_admins(
    client: &FarmClient,
    config: &Config,
    fund_names: &str,
    admin_signers: &[Pubkey],
    min_signatures: u8,
) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!("Initializing Fund {} multisig with new signers...", fund);

        info!(
            "Signature: {}",
            client
                .set_fund_admins(config.keypair.as_ref(), fund, admin_signers, min_signatures)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn get_admins(client: &FarmClient, config: &Config, fund_names: &str) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        if config.no_pretty_print {
            println!("{}: {}", fund, client.get_fund_admins(fund).unwrap());
        } else {
            println!(
                "{}: {}",
                fund,
                to_pretty_json(&client.get_fund_admins(fund).unwrap()).unwrap()
            );
        }
    }
}

pub fn set_fund_manager(client: &FarmClient, config: &Config, fund_names: &str, manager: &Pubkey) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!("Setting manager for the Fund {}...", fund);
        let fund_meta = Fund {
            fund_manager: *manager,
            ..client.get_fund(fund).unwrap()
        };
        info!(
            "Signature: {}",
            client.add_fund(config.keypair.as_ref(), fund_meta).unwrap()
        );
    }
    info!("Done.")
}

pub fn add_custody(
    client: &FarmClient,
    config: &Config,
    fund_names: &str,
    token_name: &str,
    custody_type: FundCustodyType,
) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!("Adding {} custody to the Fund {}...", custody_type, fund);
        info!(
            "Signature: {}",
            client
                .add_fund_custody(config.keypair.as_ref(), fund, token_name, custody_type)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn remove_custody(
    client: &FarmClient,
    config: &Config,
    fund_names: &str,
    token_name: &str,
    custody_type: FundCustodyType,
) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!(
            "Removing {} custody from the Fund {}...",
            custody_type, fund
        );
        info!(
            "Signature: {}",
            client
                .remove_fund_custody(config.keypair.as_ref(), fund, token_name, custody_type)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn add_vault(
    client: &FarmClient,
    config: &Config,
    fund_names: &str,
    vault_name: &str,
    vault_type: FundVaultType,
) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!("Adding Vault {} to the Fund {}...", vault_name, fund);
        info!(
            "Signature: {}",
            client
                .add_fund_vault(config.keypair.as_ref(), fund, vault_name, vault_type)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn remove_vault(
    client: &FarmClient,
    config: &Config,
    fund_names: &str,
    vault_name: &str,
    vault_type: FundVaultType,
) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!("Removing Vault {} from the Fund {}...", vault_name, fund);
        info!(
            "Signature: {}",
            client
                .remove_fund_vault(config.keypair.as_ref(), fund, vault_name, vault_type)
                .unwrap()
        );
    }
    info!("Done.")
}

#[allow(clippy::too_many_arguments)]
pub fn set_assets_tracking_config(
    client: &FarmClient,
    config: &Config,
    fund_names: &str,
    assets_limit_usd: f64,
    max_update_age_sec: u64,
    max_price_error: f64,
    max_price_age_sec: u64,
    issue_virtual_tokens: bool,
) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!("Setting assets tracking config for the Fund {}...", fund);
        info!(
            "Signature: {}",
            client
                .set_fund_assets_tracking_config(
                    config.keypair.as_ref(),
                    fund,
                    &FundAssetsTrackingConfig {
                        assets_limit_usd,
                        max_update_age_sec,
                        max_price_error,
                        max_price_age_sec,
                        issue_virtual_tokens
                    }
                )
                .unwrap()
        );
    }
    info!("Done.")
}

#[allow(clippy::too_many_arguments)]
pub fn set_deposit_schedule(
    client: &FarmClient,
    config: &Config,
    fund_names: &str,
    start_time: UnixTimestamp,
    end_time: UnixTimestamp,
    approval_required: bool,
    min_amount_usd: f64,
    max_amount_usd: f64,
    fee: f64,
) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!("Setting deposit schedule for the Fund {}...", fund);
        info!(
            "Signature: {}",
            client
                .set_fund_deposit_schedule(
                    config.keypair.as_ref(),
                    fund,
                    &FundSchedule {
                        start_time,
                        end_time,
                        approval_required,
                        min_amount_usd,
                        max_amount_usd,
                        fee
                    }
                )
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn disable_deposits(client: &FarmClient, config: &Config, fund_names: &str) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!("Disabling deposits for the Fund {}...", fund);
        info!(
            "Signature: {}",
            client
                .disable_deposits_fund(config.keypair.as_ref(), fund)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn approve_deposit(
    client: &FarmClient,
    config: &Config,
    fund_names: &str,
    user_address: &Pubkey,
    token_name: &str,
    ui_amount: f64,
) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!(
            "Approving deposit from {} to the Fund {}...",
            user_address, fund
        );
        info!(
            "Signature: {}",
            client
                .approve_deposit_fund(
                    config.keypair.as_ref(),
                    fund,
                    user_address,
                    token_name,
                    ui_amount
                )
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn deny_deposit(
    client: &FarmClient,
    config: &Config,
    fund_names: &str,
    user_address: &Pubkey,
    token_name: &str,
    deny_reason: &str,
) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!(
            "Denying deposit from {} to the Fund {}...",
            user_address, fund
        );
        info!(
            "Signature: {}",
            client
                .deny_deposit_fund(
                    config.keypair.as_ref(),
                    fund,
                    user_address,
                    token_name,
                    deny_reason
                )
                .unwrap()
        );
    }
    info!("Done.")
}

#[allow(clippy::too_many_arguments)]
pub fn set_withdrawal_schedule(
    client: &FarmClient,
    config: &Config,
    fund_names: &str,
    start_time: UnixTimestamp,
    end_time: UnixTimestamp,
    approval_required: bool,
    min_amount_usd: f64,
    max_amount_usd: f64,
    fee: f64,
) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!("Setting withdrawal schedule for the Fund {}...", fund);
        info!(
            "Signature: {}",
            client
                .set_fund_withdrawal_schedule(
                    config.keypair.as_ref(),
                    fund,
                    &FundSchedule {
                        start_time,
                        end_time,
                        approval_required,
                        min_amount_usd,
                        max_amount_usd,
                        fee
                    }
                )
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn disable_withdrawals(client: &FarmClient, config: &Config, fund_names: &str) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!("Disabling withdrawals for the Fund {}...", fund);
        info!(
            "Signature: {}",
            client
                .disable_withdrawals_fund(config.keypair.as_ref(), fund)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn approve_withdrawal(
    client: &FarmClient,
    config: &Config,
    fund_names: &str,
    user_address: &Pubkey,
    token_name: &str,
    ui_amount: f64,
) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!(
            "Approving withdrawal from {} to the Fund {}...",
            user_address, fund
        );
        info!(
            "Signature: {}",
            client
                .approve_withdrawal_fund(
                    config.keypair.as_ref(),
                    fund,
                    user_address,
                    token_name,
                    ui_amount
                )
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn deny_withdrawal(
    client: &FarmClient,
    config: &Config,
    fund_names: &str,
    user_address: &Pubkey,
    token_name: &str,
    deny_reason: &str,
) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!(
            "Denying withdrawal from {} to the Fund {}...",
            user_address, fund
        );
        info!(
            "Signature: {}",
            client
                .deny_withdrawal_fund(
                    config.keypair.as_ref(),
                    fund,
                    user_address,
                    token_name,
                    deny_reason
                )
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn lock_assets(
    client: &FarmClient,
    config: &Config,
    fund_names: &str,
    token_name: &str,
    ui_amount: f64,
) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!("Moving {} to the Fund {}...", token_name, fund);
        info!(
            "Signature: {}",
            client
                .lock_assets_fund(config.keypair.as_ref(), fund, token_name, ui_amount)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn unlock_assets(
    client: &FarmClient,
    config: &Config,
    fund_names: &str,
    token_name: &str,
    ui_amount: f64,
) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!("Moving {} out of the Fund {}...", token_name, fund);
        info!(
            "Signature: {}",
            client
                .unlock_assets_fund(config.keypair.as_ref(), fund, token_name, ui_amount)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn withdraw_fees(
    client: &FarmClient,
    config: &Config,
    fund_names: &str,
    token_name: &str,
    custody_type: FundCustodyType,
    ui_amount: f64,
    receiver: &Pubkey,
) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!(
            "Withdrawing fees from {} {} custody of the Fund {} to {}...",
            token_name, custody_type, fund, receiver
        );
        info!(
            "Signature: {}",
            client
                .withdraw_fees_fund(
                    config.keypair.as_ref(),
                    fund,
                    token_name,
                    custody_type,
                    ui_amount,
                    receiver
                )
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn update_assets_with_custody(
    client: &FarmClient,
    config: &Config,
    fund_names: &str,
    custody_id: u32,
) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!("Updating assets with custody for the Fund {}...", fund);
        info!(
            "Signature: {}",
            client
                .update_fund_assets_with_custody(config.keypair.as_ref(), fund, custody_id)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn update_assets_with_custodies(client: &FarmClient, config: &Config, fund_names: &str) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!("Updating assets with custodies for the Fund {}...", fund);
        info!(
            "Updated: {} custodies processed",
            client
                .update_fund_assets_with_custodies(config.keypair.as_ref(), fund)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn update_assets_with_vault(
    client: &FarmClient,
    config: &Config,
    fund_names: &str,
    vault_id: u32,
) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!("Updating assets with Vault for the Fund {}...", fund);
        info!(
            "Signature: {}",
            client
                .update_fund_assets_with_vault(config.keypair.as_ref(), fund, vault_id)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn update_assets_with_vaults(client: &FarmClient, config: &Config, fund_names: &str) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!("Updating assets with Vaults for the Fund {}...", fund);
        info!(
            "Updated: {} Vaults processed",
            client
                .update_fund_assets_with_vaults(config.keypair.as_ref(), fund)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn stop_liquidation(client: &FarmClient, config: &Config, fund_names: &str) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!("Stopping liquidation of the Fund {}...", fund);
        info!(
            "Signature: {}",
            client
                .stop_liquidation_fund(config.keypair.as_ref(), fund)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn add_liquidity_pool(
    client: &FarmClient,
    config: &Config,
    fund_names: &str,
    pool_name: &str,
    max_token_a_ui_amount: f64,
    max_token_b_ui_amount: f64,
) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!(
            "Adding liquidity to the Pool {} in the Fund {}...",
            pool_name, fund
        );
        info!(
            "Signature: {}",
            client
                .fund_add_liquidity_pool(
                    config.keypair.as_ref(),
                    fund,
                    pool_name,
                    max_token_a_ui_amount,
                    max_token_b_ui_amount
                )
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn remove_liquidity_pool(
    client: &FarmClient,
    config: &Config,
    fund_names: &str,
    pool_name: &str,
    ui_amount: f64,
) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!(
            "Removing liquidity from the Pool {} in the Fund {}...",
            pool_name, fund
        );
        info!(
            "Signature: {}",
            client
                .fund_remove_liquidity_pool(config.keypair.as_ref(), fund, pool_name, ui_amount)
                .unwrap()
        );
    }
    info!("Done.")
}

#[allow(clippy::too_many_arguments)]
pub fn swap(
    client: &FarmClient,
    config: &Config,
    fund_names: &str,
    protocol: Protocol,
    from_token: &str,
    to_token: &str,
    ui_amount_in: f64,
    min_ui_amount_out: f64,
) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!(
            "Swapping {} to {} in the Fund {}...",
            from_token, to_token, fund
        );
        info!(
            "Signature: {}",
            client
                .fund_swap(
                    config.keypair.as_ref(),
                    fund,
                    protocol,
                    from_token,
                    to_token,
                    ui_amount_in,
                    min_ui_amount_out
                )
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn stake(
    client: &FarmClient,
    config: &Config,
    fund_names: &str,
    farm_name: &str,
    ui_amount: f64,
) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!(
            "Staking tokens to the Farm {} in the Fund {}...",
            farm_name, fund
        );
        info!(
            "Signature: {}",
            client
                .fund_stake(config.keypair.as_ref(), fund, farm_name, ui_amount)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn unstake(
    client: &FarmClient,
    config: &Config,
    fund_names: &str,
    farm_name: &str,
    ui_amount: f64,
) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!(
            "Unstaking tokens from the Farm {} in the Fund {}...",
            farm_name, fund
        );
        info!(
            "Signature: {}",
            client
                .fund_unstake(config.keypair.as_ref(), fund, farm_name, ui_amount)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn harvest(client: &FarmClient, config: &Config, fund_names: &str, farm_name: &str) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!(
            "Harvesting rewards from the Farm {} in the Fund {}...",
            farm_name, fund
        );
        info!(
            "Signature: {}",
            client
                .fund_harvest(config.keypair.as_ref(), fund, farm_name)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn add_liquidity_vault(
    client: &FarmClient,
    config: &Config,
    fund_names: &str,
    vault_name: &str,
    max_token_a_ui_amount: f64,
    max_token_b_ui_amount: f64,
) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!(
            "Adding liquidity to the Vault {} in the Fund {}...",
            vault_name, fund
        );
        info!(
            "Signature: {}",
            client
                .fund_add_liquidity_vault(
                    config.keypair.as_ref(),
                    fund,
                    vault_name,
                    max_token_a_ui_amount,
                    max_token_b_ui_amount,
                )
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn add_locked_liquidity_vault(
    client: &FarmClient,
    config: &Config,
    fund_names: &str,
    vault_name: &str,
    ui_amount: f64,
) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!(
            "Adding locked liquidity to the Vault {} in the Fund {}...",
            vault_name, fund
        );
        info!(
            "Signature: {}",
            client
                .fund_add_locked_liquidity_vault(
                    config.keypair.as_ref(),
                    fund,
                    vault_name,
                    ui_amount,
                )
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn remove_liquidity_vault(
    client: &FarmClient,
    config: &Config,
    fund_names: &str,
    vault_name: &str,
    ui_amount: f64,
) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!(
            "Removing liquidity from the Vault {} in the Fund {}...",
            vault_name, fund
        );
        info!(
            "Signature: {}",
            client
                .fund_remove_liquidity_vault(config.keypair.as_ref(), fund, vault_name, ui_amount,)
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn remove_unlocked_liquidity_vault(
    client: &FarmClient,
    config: &Config,
    fund_names: &str,
    vault_name: &str,
    ui_amount: f64,
) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!(
            "Removing unlocked liquidity from the Vault {} in the Fund {}...",
            vault_name, fund
        );
        info!(
            "Signature: {}",
            client
                .fund_remove_unlocked_liquidity_vault(
                    config.keypair.as_ref(),
                    fund,
                    vault_name,
                    ui_amount,
                )
                .unwrap()
        );
    }
    info!("Done.")
}

pub fn get_info(client: &FarmClient, config: &Config, fund_names: &str) {
    let funds = fund_names.split(',').collect::<Vec<_>>();
    for fund in funds {
        info!("Retreiving stats for Fund {}...", fund);

        let info = client.get_fund_info(fund).unwrap();

        if config.no_pretty_print {
            println!("{}", info);
        } else {
            println!("{}", to_pretty_json(&info).unwrap());
        }
    }
    info!("Done.")
}
