//! Solana Farms control interface.

mod config;
mod fund;
mod generate;
mod get;
mod governance;
mod load;
mod loaders;
mod print;
mod refdb;
mod remove;
mod vault;

use {
    log::error, solana_farm_client::client::FarmClient, solana_sdk::pubkey::Pubkey,
    std::str::FromStr,
};

fn main() {
    let matches = config::get_clap_app(solana_version::version!()).get_matches();

    // set log verbosity level
    let log_level = "solana=".to_string() + matches.value_of("log_level").unwrap();
    solana_logger::setup_with_default(log_level.as_str());

    // load config params
    let config = config::Config::new(&matches);
    let client = FarmClient::new_with_commitment(&config.farm_client_url, config.commitment);

    // parse commands
    match matches.subcommand() {
        ("init", Some(subcommand_matches)) => {
            refdb::init(&client, &config, config::get_target(subcommand_matches));
        }
        ("init-all", Some(_subcommand_matches)) => {
            refdb::init_all(&client, &config);
        }
        ("set-admins", Some(subcommand_matches)) => {
            refdb::set_admins(
                &client,
                &config,
                config::get_pubkey_multi_val(subcommand_matches, "admin_signers").as_slice(),
                config::get_integer_val(subcommand_matches, "min_signatures") as u8,
            );
        }
        ("get-admins", Some(_subcommand_matches)) => {
            refdb::get_admins(&client, &config);
        }
        ("drop", Some(subcommand_matches)) => {
            refdb::drop(&client, &config, config::get_target(subcommand_matches));
        }
        ("drop-all", Some(_subcommand_matches)) => {
            refdb::drop_all(&client, &config);
        }
        ("load", Some(subcommand_matches)) => {
            load::load(
                &client,
                &config,
                config::get_target(subcommand_matches),
                &config::get_str_val_raw(subcommand_matches, "file_name"),
                false,
            );
        }
        ("load-all", Some(subcommand_matches)) => {
            load::load(
                &client,
                &config,
                config::get_target(subcommand_matches),
                &config::get_str_val_raw(subcommand_matches, "file_name"),
                false,
            );
        }
        ("remove", Some(subcommand_matches)) => {
            remove::remove(
                &client,
                &config,
                config::get_target(subcommand_matches),
                &config::get_str_val_raw(subcommand_matches, "object_name"),
            );
        }
        ("remove-ref", Some(subcommand_matches)) => {
            remove::remove_ref(
                &client,
                &config,
                config::get_target(subcommand_matches),
                &config::get_str_val_raw(subcommand_matches, "object_name"),
            );
        }
        ("remove-all", Some(subcommand_matches)) => {
            remove::remove_all(&client, &config, config::get_target(subcommand_matches));
        }
        ("remove-all-with-file", Some(subcommand_matches)) => {
            load::load(
                &client,
                &config,
                config::get_target(subcommand_matches),
                &config::get_str_val_raw(subcommand_matches, "file_name"),
                true,
            );
        }
        ("get", Some(subcommand_matches)) => {
            get::get(
                &client,
                &config,
                config::get_target(subcommand_matches),
                &config::get_str_val_raw(subcommand_matches, "object_name"),
            );
        }
        ("get-ref", Some(subcommand_matches)) => {
            get::get_ref(
                &client,
                &config,
                config::get_target(subcommand_matches),
                &config::get_str_val_raw(subcommand_matches, "object_name"),
            );
        }
        ("get-all", Some(subcommand_matches)) => {
            get::get_all(&client, &config, config::get_target(subcommand_matches));
        }
        ("list-all", Some(subcommand_matches)) => {
            get::list_all(&client, &config, config::get_target(subcommand_matches));
        }
        ("program-get-admins", Some(subcommand_matches)) => {
            refdb::get_program_admins(
                &client,
                &config,
                &config::get_pubkey_val(subcommand_matches, "program_id"),
            );
        }
        ("program-set-admins", Some(subcommand_matches)) => {
            refdb::set_program_admins(
                &client,
                &config,
                &config::get_pubkey_val(subcommand_matches, "program_id"),
                config::get_pubkey_multi_val(subcommand_matches, "admin_signers").as_slice(),
                config::get_integer_val(subcommand_matches, "min_signatures") as u8,
            );
        }
        ("program-set-single-authority", Some(subcommand_matches)) => {
            refdb::set_program_single_authority(
                &client,
                &config,
                &config::get_pubkey_val(subcommand_matches, "program_id"),
                &config::get_pubkey_val(subcommand_matches, "upgrade_authority"),
            );
        }
        ("program-upgrade", Some(subcommand_matches)) => {
            refdb::upgrade_program(
                &client,
                &config,
                &config::get_pubkey_val(subcommand_matches, "program_id"),
                &config::get_pubkey_val(subcommand_matches, "buffer_address"),
            );
        }
        ("vault-init", Some(subcommand_matches)) => {
            vault::init(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "vault_name"),
                config::get_integer_val(subcommand_matches, "step"),
            );
        }
        ("vault-set-admins", Some(subcommand_matches)) => {
            vault::set_admins(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "vault_name"),
                config::get_pubkey_multi_val(subcommand_matches, "admin_signers").as_slice(),
                config::get_integer_val(subcommand_matches, "min_signatures") as u8,
            );
        }
        ("vault-get-admins", Some(subcommand_matches)) => {
            vault::get_admins(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "vault_name"),
            );
        }
        ("vault-shutdown", Some(subcommand_matches)) => {
            vault::shutdown(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "vault_name"),
            );
        }
        ("vault-withdraw-fees", Some(subcommand_matches)) => {
            vault::withdraw_fees(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "vault_name"),
                config::get_str_val_raw(subcommand_matches, "fee_token")
                    .parse()
                    .unwrap(),
                config::get_floating_val(subcommand_matches, "amount"),
                &config::get_pubkey_val(subcommand_matches, "receiver"),
            );
        }
        ("vault-crank", Some(subcommand_matches)) => {
            vault::crank(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "vault_name"),
                config::get_integer_val(subcommand_matches, "step"),
            );
        }
        ("vault-set-fee", Some(subcommand_matches)) => {
            vault::set_fee(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "vault_name"),
                config::get_floating_val(subcommand_matches, "fee_percent") as f32,
            );
        }
        ("vault-set-external-fee", Some(subcommand_matches)) => {
            vault::set_external_fee(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "vault_name"),
                config::get_floating_val(subcommand_matches, "external_fee_percent") as f32,
            );
        }
        ("vault-set-min-crank-interval", Some(subcommand_matches)) => {
            vault::set_min_crank_interval(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "vault_name"),
                config::get_integer_val(subcommand_matches, "min_crank_interval") as u32,
            );
        }
        ("vault-disable-deposits", Some(subcommand_matches)) => {
            vault::disable_deposits(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "vault_name"),
            );
        }
        ("vault-enable-deposits", Some(subcommand_matches)) => {
            vault::enable_deposits(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "vault_name"),
            );
        }
        ("vault-disable-withdrawals", Some(subcommand_matches)) => {
            vault::disable_withdrawals(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "vault_name"),
            );
        }
        ("vault-enable-withdrawals", Some(subcommand_matches)) => {
            vault::enable_withdrawals(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "vault_name"),
            );
        }
        ("vault-get-info", Some(subcommand_matches)) => {
            vault::get_info(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "vault_name"),
            );
        }
        ("fund-init", Some(subcommand_matches)) => {
            fund::init(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
                config::get_integer_val(subcommand_matches, "step"),
            );
        }
        ("fund-set-admins", Some(subcommand_matches)) => {
            fund::set_admins(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
                config::get_pubkey_multi_val(subcommand_matches, "admin_signers").as_slice(),
                config::get_integer_val(subcommand_matches, "min_signatures") as u8,
            );
        }
        ("fund-get-admins", Some(subcommand_matches)) => {
            fund::get_admins(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
            );
        }
        ("fund-set-manager", Some(subcommand_matches)) => {
            fund::set_fund_manager(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
                &config::get_pubkey_val(subcommand_matches, "manager"),
            );
        }
        ("fund-add-custody", Some(subcommand_matches)) => {
            fund::add_custody(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
                &config::get_str_val(subcommand_matches, "token_name"),
                config::get_str_val_raw(subcommand_matches, "custody_type")
                    .parse()
                    .unwrap(),
            );
        }
        ("fund-remove-custody", Some(subcommand_matches)) => {
            fund::remove_custody(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
                &config::get_str_val(subcommand_matches, "token_name"),
                config::get_str_val_raw(subcommand_matches, "custody_type")
                    .parse()
                    .unwrap(),
            );
        }
        ("fund-add-vault", Some(subcommand_matches)) => {
            fund::add_vault(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
                &config::get_str_val(subcommand_matches, "vault_name"),
                config::get_str_val_raw(subcommand_matches, "vault_type")
                    .parse()
                    .unwrap(),
            );
        }
        ("fund-remove-vault", Some(subcommand_matches)) => {
            fund::remove_vault(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
                &config::get_str_val(subcommand_matches, "vault_name"),
                config::get_str_val_raw(subcommand_matches, "vault_type")
                    .parse()
                    .unwrap(),
            );
        }
        ("fund-set-assets-tracking-config", Some(subcommand_matches)) => {
            fund::set_assets_tracking_config(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
                config::get_floating_val(subcommand_matches, "assets_limit_usd"),
                config::get_integer_val(subcommand_matches, "max_update_age_sec"),
                config::get_floating_val(subcommand_matches, "max_price_error"),
                config::get_integer_val(subcommand_matches, "max_price_age_sec"),
                config::get_boolean_val(subcommand_matches, "issue_virtual_tokens"),
            );
        }
        ("fund-set-deposit-schedule", Some(subcommand_matches)) => {
            fund::set_deposit_schedule(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
                config::get_integer_val(subcommand_matches, "start_time") as i64,
                config::get_integer_val(subcommand_matches, "end_time") as i64,
                config::get_str_val_raw(subcommand_matches, "approval_required")
                    .parse()
                    .unwrap(),
                config::get_floating_val(subcommand_matches, "min_amount_usd"),
                config::get_floating_val(subcommand_matches, "max_amount_usd"),
                config::get_floating_val(subcommand_matches, "fee"),
            );
        }
        ("fund-disable-deposits", Some(subcommand_matches)) => {
            fund::disable_deposits(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
            );
        }
        ("fund-approve-deposit", Some(subcommand_matches)) => {
            fund::approve_deposit(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
                &config::get_pubkey_val(subcommand_matches, "user_address"),
                &config::get_str_val(subcommand_matches, "token_name"),
                config::get_floating_val(subcommand_matches, "amount"),
            );
        }
        ("fund-deny-deposit", Some(subcommand_matches)) => {
            fund::deny_deposit(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
                &config::get_pubkey_val(subcommand_matches, "user_address"),
                &config::get_str_val(subcommand_matches, "token_name"),
                &config::get_str_val_raw(subcommand_matches, "deny_reason"),
            );
        }
        ("fund-set-withdrawal-schedule", Some(subcommand_matches)) => {
            fund::set_withdrawal_schedule(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
                config::get_integer_val(subcommand_matches, "start_time") as i64,
                config::get_integer_val(subcommand_matches, "end_time") as i64,
                config::get_str_val_raw(subcommand_matches, "approval_required")
                    .parse()
                    .unwrap(),
                config::get_floating_val(subcommand_matches, "min_amount_usd"),
                config::get_floating_val(subcommand_matches, "max_amount_usd"),
                config::get_floating_val(subcommand_matches, "fee"),
            );
        }
        ("fund-disable-withdrawals", Some(subcommand_matches)) => {
            fund::disable_withdrawals(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
            );
        }
        ("fund-approve-withdrawal", Some(subcommand_matches)) => {
            fund::approve_withdrawal(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
                &config::get_pubkey_val(subcommand_matches, "user_address"),
                &config::get_str_val(subcommand_matches, "token_name"),
                config::get_floating_val(subcommand_matches, "amount"),
            );
        }
        ("fund-deny-withdrawal", Some(subcommand_matches)) => {
            fund::deny_withdrawal(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
                &config::get_pubkey_val(subcommand_matches, "user_address"),
                &config::get_str_val(subcommand_matches, "token_name"),
                &config::get_str_val_raw(subcommand_matches, "deny_reason"),
            );
        }
        ("fund-lock-assets", Some(subcommand_matches)) => {
            fund::lock_assets(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
                &config::get_str_val(subcommand_matches, "token_name"),
                config::get_floating_val(subcommand_matches, "amount"),
            );
        }
        ("fund-unlock-assets", Some(subcommand_matches)) => {
            fund::unlock_assets(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
                &config::get_str_val(subcommand_matches, "token_name"),
                config::get_floating_val(subcommand_matches, "amount"),
            );
        }
        ("fund-withdraw-fees", Some(subcommand_matches)) => {
            fund::withdraw_fees(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
                &config::get_str_val(subcommand_matches, "token_name"),
                config::get_str_val_raw(subcommand_matches, "custody_type")
                    .parse()
                    .unwrap(),
                config::get_floating_val(subcommand_matches, "amount"),
                &config::get_pubkey_val(subcommand_matches, "receiver"),
            );
        }
        ("fund-update-assets-with-custody", Some(subcommand_matches)) => {
            fund::update_assets_with_custody(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
                config::get_integer_val(subcommand_matches, "custody_id") as u32,
            );
        }
        ("fund-update-assets-with-custodies", Some(subcommand_matches)) => {
            fund::update_assets_with_custodies(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
            );
        }
        ("fund-update-assets-with-vault", Some(subcommand_matches)) => {
            fund::update_assets_with_vault(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
                config::get_integer_val(subcommand_matches, "vault_id") as u32,
            );
        }
        ("fund-update-assets-with-vaults", Some(subcommand_matches)) => {
            fund::update_assets_with_vaults(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
            );
        }
        ("fund-stop-liquidation", Some(subcommand_matches)) => {
            fund::stop_liquidation(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
            );
        }
        ("fund-get-info", Some(subcommand_matches)) => {
            fund::get_info(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
            );
        }
        ("fund-deposit-pool", Some(subcommand_matches)) => {
            fund::add_liquidity_pool(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
                &config::get_str_val(subcommand_matches, "pool_name"),
                config::get_floating_val(subcommand_matches, "max_token_a_ui_amount"),
                config::get_floating_val(subcommand_matches, "max_token_b_ui_amount"),
            );
        }
        ("fund-withdraw-pool", Some(subcommand_matches)) => {
            fund::remove_liquidity_pool(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
                &config::get_str_val(subcommand_matches, "pool_name"),
                config::get_floating_val(subcommand_matches, "amount"),
            );
        }
        ("fund-swap", Some(subcommand_matches)) => {
            fund::swap(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
                config::get_str_val(subcommand_matches, "protocol")
                    .parse()
                    .expect("Failed to parse protocol argument"),
                &config::get_str_val(subcommand_matches, "from_token"),
                &config::get_str_val(subcommand_matches, "to_token"),
                config::get_floating_val(subcommand_matches, "amount_in"),
                config::get_floating_val(subcommand_matches, "min_amount_out"),
            );
        }
        ("fund-stake", Some(subcommand_matches)) => {
            fund::stake(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
                &config::get_str_val(subcommand_matches, "farm_name"),
                config::get_floating_val(subcommand_matches, "amount"),
            );
        }
        ("fund-unstake", Some(subcommand_matches)) => {
            fund::unstake(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
                &config::get_str_val(subcommand_matches, "farm_name"),
                config::get_floating_val(subcommand_matches, "amount"),
            );
        }
        ("fund-harvest", Some(subcommand_matches)) => {
            fund::harvest(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
                &config::get_str_val(subcommand_matches, "farm_name"),
            );
        }
        ("fund-deposit-vault", Some(subcommand_matches)) => {
            fund::add_liquidity_vault(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
                &config::get_str_val(subcommand_matches, "vault_name"),
                config::get_floating_val(subcommand_matches, "max_token_a_amount"),
                config::get_floating_val(subcommand_matches, "max_token_b_amount"),
            );
        }
        ("fund-deposit-vault-locked", Some(subcommand_matches)) => {
            fund::add_locked_liquidity_vault(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
                &config::get_str_val(subcommand_matches, "vault_name"),
                config::get_floating_val(subcommand_matches, "amount"),
            );
        }
        ("fund-withdraw-vault", Some(subcommand_matches)) => {
            fund::remove_liquidity_vault(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
                &config::get_str_val(subcommand_matches, "vault_name"),
                config::get_floating_val(subcommand_matches, "amount"),
            );
        }
        ("fund-withdraw-vault-unlocked", Some(subcommand_matches)) => {
            fund::remove_unlocked_liquidity_vault(
                &client,
                &config,
                &config::get_str_val(subcommand_matches, "fund_name"),
                &config::get_str_val(subcommand_matches, "vault_name"),
                config::get_floating_val(subcommand_matches, "amount"),
            );
        }
        ("print-pda", Some(subcommand_matches)) => {
            print::print_pda(&client, &config, config::get_target(subcommand_matches));
        }
        ("print-pda-all", Some(_subcommand_matches)) => {
            print::print_pda_all(&client, &config);
        }
        ("print-size", Some(subcommand_matches)) => {
            print::print_size(&client, &config, config::get_target(subcommand_matches));
        }
        ("print-size-all", Some(_subcommand_matches)) => {
            print::print_size_all(&client, &config);
        }
        ("generate", Some(subcommand_matches)) => {
            generate::generate(
                &client,
                &config,
                config::get_target(subcommand_matches),
                &config::get_str_val_raw(subcommand_matches, "object_name"),
                &config::get_str_val_raw(subcommand_matches, "param1"),
                &config::get_str_val_raw(subcommand_matches, "param2"),
            );
        }
        ("governance", Some(subcommand_matches)) => match subcommand_matches.subcommand() {
            ("init", Some(subcommand_matches)) => {
                let address_str = subcommand_matches
                    .value_of("governance-program-address")
                    .unwrap();
                let dao_address = Pubkey::from_str(address_str).unwrap();
                governance::init(
                    &client,
                    &config,
                    &dao_address,
                    config::get_floating_val(subcommand_matches, "mint-ui-amount"),
                );
            }
            _ => unreachable!(),
        },
        _ => error!("Unrecognized command. Use --help to list known commands."),
    };
}
