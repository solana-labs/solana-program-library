//! Command-line interface for the Farm Client

mod config;
mod printer;

use {
    clap::ArgMatches,
    log::error,
    num_enum::TryFromPrimitive,
    solana_farm_client::client::FarmClient,
    solana_farm_sdk::{fund::FundVaultType, id::DAO_CUSTODY_NAME, token::TokenSelector},
    solana_sdk::{bpf_loader_upgradeable, instruction::Instruction, pubkey::Pubkey},
    spl_associated_token_account::create_associated_token_account,
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
    let wallet = config.keypair.pubkey();

    // parse commands
    match matches.subcommand() {
        ("get", Some(subcommand_matches)) => {
            let target = config::get_target(subcommand_matches);
            let objects = config::get_vec_str_val_raw(subcommand_matches, "object_name");
            for object in objects {
                printer::print(&client, &config, &target, &object.to_string());
            }
        }
        ("get-ref", Some(subcommand_matches)) => {
            let target = config::get_target(subcommand_matches);
            let objects = config::get_vec_str_val_raw(subcommand_matches, "object_name");
            for object in objects {
                printer::print_with_ref(&client, &config, &target, &object.to_string());
            }
        }
        ("get-all", Some(subcommand_matches)) => {
            let target = config::get_target(subcommand_matches);
            printer::print_all(&client, &config, &target);
        }
        ("list-all", Some(subcommand_matches)) => {
            let target = config::get_target(subcommand_matches);
            printer::list_all(&client, &config, &target);
        }
        ("protocols", Some(_subcommand_matches)) => {
            let protocols = client.get_protocols().unwrap();
            for protocol in protocols {
                println!("{}", protocol);
            }
        }
        ("pool-price", Some(subcommand_matches)) => {
            let pools = config::get_vec_str_val(subcommand_matches, "pool_name");
            for pool in pools {
                println!("{} price: {}", pool, client.get_pool_price(&pool).unwrap());
            }
        }
        ("oracle-price", Some(subcommand_matches)) => {
            let symbol = config::get_str_val(subcommand_matches, "symbol");
            let max_price_age_sec =
                config::get_integer_val(subcommand_matches, "max_price_age_sec");
            let max_price_error = config::get_floating_val(subcommand_matches, "max_price_error");
            println!(
                "{} price: {}",
                symbol,
                client
                    .get_oracle_price(&symbol, max_price_age_sec, max_price_error)
                    .unwrap()
            );
        }
        ("transfer", Some(subcommand_matches)) => {
            let destination = config::get_pubkey_val(subcommand_matches, "wallet");
            let amount = config::get_floating_val(subcommand_matches, "amount");
            println!(
                "Done: {}",
                client
                    .transfer(config.keypair.as_ref(), &destination, amount)
                    .unwrap()
            );
        }
        ("token-transfer", Some(subcommand_matches)) => {
            let token_name = config::get_str_val(subcommand_matches, "token_name");
            let destination = config::get_pubkey_val(subcommand_matches, "wallet");
            let amount = config::get_floating_val(subcommand_matches, "amount");
            println!(
                "Done: {}",
                client
                    .token_transfer(config.keypair.as_ref(), &token_name, &destination, amount)
                    .unwrap()
            );
        }
        ("wrap-sol", Some(subcommand_matches)) => {
            let amount = config::get_floating_val(subcommand_matches, "amount");
            println!(
                "Done: {}",
                client.wrap_sol(config.keypair.as_ref(), amount).unwrap()
            );
        }
        ("unwrap-sol", Some(_subcommand_matches)) => {
            println!(
                "Done: {}",
                client.unwrap_sol(config.keypair.as_ref()).unwrap()
            );
        }
        ("sync-token-balance", Some(subcommand_matches)) => {
            let token_name = config::get_str_val(subcommand_matches, "token_name");
            println!(
                "Done: {}",
                client
                    .sync_token_balance(config.keypair.as_ref(), &token_name)
                    .unwrap()
            );
        }
        ("token-address", Some(subcommand_matches)) => {
            let tokens = config::get_vec_str_val(subcommand_matches, "token_name");
            for token in tokens {
                println!(
                    "{} address: {}",
                    token,
                    client
                        .get_associated_token_address(&wallet, &token)
                        .unwrap()
                );
            }
        }
        ("token-data", Some(subcommand_matches)) => {
            let tokens = config::get_vec_str_val(subcommand_matches, "token_name");
            for token in tokens {
                println!(
                    "{} data:\n{:#?}",
                    token,
                    client.get_token_account_data(&wallet, &token).unwrap()
                );
            }
        }
        ("token-create", Some(subcommand_matches)) => {
            let tokens = config::get_vec_str_val(subcommand_matches, "token_name");
            for token in tokens {
                println!(
                    "{} created: {}",
                    token,
                    client
                        .get_or_create_token_account(config.keypair.as_ref(), &token)
                        .unwrap()
                );
            }
        }
        ("token-close", Some(subcommand_matches)) => {
            let tokens = config::get_vec_str_val(subcommand_matches, "token_name");
            for token in tokens {
                println!(
                    "{} closed: {}",
                    token,
                    client
                        .close_token_account(config.keypair.as_ref(), &token)
                        .unwrap()
                );
            }
        }
        ("token-supply", Some(subcommand_matches)) => {
            let tokens = config::get_vec_str_val(subcommand_matches, "token_name");
            for token in tokens {
                println!(
                    "{} supply: {}",
                    token,
                    client.get_token_supply(&token).unwrap()
                );
            }
        }
        ("balance", Some(_)) => {
            println!(
                "SOL balance: {}",
                client.get_account_balance(&wallet).unwrap()
            );
        }
        ("token-balance", Some(subcommand_matches)) => {
            let tokens = config::get_vec_str_val(subcommand_matches, "token_name");
            for token in tokens {
                if let Ok(balance) = client.get_token_account_balance(&wallet, &token) {
                    println!("{} balance: {}", token, balance);
                } else {
                    println!("{} balance: no account", token);
                }
            }
        }
        ("stake-balance", Some(subcommand_matches)) => {
            let farms = config::get_vec_str_val(subcommand_matches, "farm_name");
            for farm in farms {
                if let Ok(balance) = client.get_user_stake_balance(&wallet, &farm) {
                    println!("{} balance: {}", farm, balance);
                } else {
                    println!("{} balance: no account", farm);
                }
            }
        }
        ("wallet-balances", Some(_subcommand_matches)) => {
            println!(
                "SOL balance: {}",
                client.get_account_balance(&wallet).unwrap()
            );
            let tokens = client.get_wallet_tokens(&wallet).unwrap();
            for token in tokens {
                if let Ok(balance) = client.get_token_account_balance(&wallet, &token) {
                    println!("{} balance: {}", token, balance);
                } else {
                    println!("{} balance: no account", token);
                }
            }
        }
        ("vault-info", Some(subcommand_matches)) => {
            let object = config::get_str_val(subcommand_matches, "vault_name");
            let vault = client.get_vault(&object).unwrap();
            let vault_info = client.get_vault_info(&object).unwrap();
            printer::print_object(&config, &vault.info_account, &vault_info);
        }
        ("vault-user-info", Some(subcommand_matches)) => {
            let object = config::get_str_val(subcommand_matches, "vault_name");
            let account = client
                .get_vault_user_info_account(&wallet, &object)
                .unwrap();
            let user_info = client.get_vault_user_info(&wallet, &object).unwrap();
            printer::print_object(&config, &account, &user_info);
        }
        ("fund-info", Some(subcommand_matches)) => {
            let object = config::get_str_val(subcommand_matches, "fund_name");
            let fund = client.get_fund(&object).unwrap();
            let fund_info = client.get_fund_info(&object).unwrap();
            printer::print_object(&config, &fund.info_account, &fund_info);
        }
        ("fund-user-info", Some(subcommand_matches)) => {
            let object = config::get_str_val(subcommand_matches, "fund_name");
            let account = client.get_fund_user_info_account(&wallet, &object).unwrap();
            let user_info = client.get_fund_user_info(&wallet, &object).unwrap();
            printer::print_object(&config, &account, &user_info);
        }
        ("fund-user-requests", Some(subcommand_matches)) => {
            let object = config::get_str_val(subcommand_matches, "fund_name");
            let token = config::get_str_val(subcommand_matches, "token_name");
            let account = client
                .get_fund_user_requests_account(&wallet, &object, &token)
                .unwrap();
            let user_requests = client
                .get_fund_user_requests(&wallet, &object, &token)
                .unwrap();
            printer::print_object(&config, &account, &user_requests);
        }
        ("fund-assets", Some(subcommand_matches)) => {
            let object = config::get_str_val(subcommand_matches, "fund_name");
            let asset_type = config::get_str_val_raw(subcommand_matches, "asset_type")
                .parse()
                .unwrap();
            let account = client.get_fund_assets_account(&object, asset_type).unwrap();
            let fund_assets = client.get_fund_assets(&object, asset_type).unwrap();
            printer::print_object(&config, &account, &fund_assets);
        }
        ("fund-custody", Some(subcommand_matches)) => {
            let object = config::get_str_val(subcommand_matches, "fund_name");
            let token = config::get_str_val(subcommand_matches, "token_name");
            let custody_type = config::get_str_val_raw(subcommand_matches, "custody_type")
                .parse()
                .unwrap();
            let account = client
                .get_fund_custody_account(&object, &token, custody_type)
                .unwrap();
            let fund_custody = client
                .get_fund_custody_with_balance(&object, &token, custody_type)
                .unwrap();
            printer::print_object(&config, &account, &fund_custody);
        }
        ("fund-custodies", Some(subcommand_matches)) => {
            let object = config::get_str_val(subcommand_matches, "fund_name");
            let custodies = client.get_fund_custodies_with_balance(&object).unwrap();
            for custody in &custodies {
                printer::print_object(&config, &custody.token_name, &custody);
            }
        }
        ("fund-vault", Some(subcommand_matches)) => {
            let object = config::get_str_val(subcommand_matches, "fund_name");
            let vault = config::get_str_val(subcommand_matches, "vault_name");
            let vault_type = config::get_str_val_raw(subcommand_matches, "vault_type")
                .parse()
                .unwrap();
            let account = client
                .get_fund_vault_account(&object, &vault, vault_type)
                .unwrap();
            let fund_vault = client.get_fund_vault(&object, &vault, vault_type).unwrap();
            printer::print_object(&config, &account, &fund_vault);
        }
        ("fund-vaults", Some(subcommand_matches)) => {
            let object = config::get_str_val(subcommand_matches, "fund_name");
            let vaults = client.get_fund_vaults(&object).unwrap();
            for vault in &vaults {
                let vault_name = match vault.vault_type {
                    FundVaultType::Vault => client.get_vault_by_ref(&vault.vault_ref).unwrap().name,
                    FundVaultType::Farm => client.get_farm_by_ref(&vault.vault_ref).unwrap().name,
                    FundVaultType::Pool => client.get_pool_by_ref(&vault.vault_ref).unwrap().name,
                };
                printer::print_object(&config, &vault_name, &vault);
            }
        }
        ("find-funds", Some(subcommand_matches)) => {
            let vault_name_pattern = config::get_str_val(subcommand_matches, "vault_name_pattern");
            match client.find_funds(&vault_name_pattern) {
                Ok(funds) => {
                    for fund in funds {
                        println!("{}", fund.name);
                    }
                }
                Err(e) => {
                    println!("{}", e);
                }
            }
        }
        ("find-pools", Some(subcommand_matches)) => {
            let protocol = config::get_str_val(subcommand_matches, "protocol");
            let token1 = config::get_str_val(subcommand_matches, "token_name");
            let token2 = config::get_str_val(subcommand_matches, "token_name2");
            match client.find_pools(
                protocol.parse().expect("Failed to parse protocol argument"),
                &token1,
                &token2,
            ) {
                Ok(pools) => {
                    for pool in pools {
                        println!("{}", pool.name);
                    }
                }
                Err(e) => {
                    println!("{}", e);
                }
            }
        }
        ("find-pools-with-lp", Some(subcommand_matches)) => {
            let lp_token = config::get_str_val(subcommand_matches, "token_name");
            match client.find_pools_with_lp(&lp_token) {
                Ok(pools) => {
                    for pool in pools {
                        println!("{}", pool.name);
                    }
                }
                Err(e) => {
                    println!("{}", e);
                }
            }
        }
        ("find-farms-with-lp", Some(subcommand_matches)) => {
            let lp_token = config::get_str_val(subcommand_matches, "token_name");
            match client.find_farms_with_lp(&lp_token) {
                Ok(farms) => {
                    for farm in farms {
                        println!("{}", farm.name);
                    }
                }
                Err(e) => {
                    println!("{}", e);
                }
            }
        }
        ("find-vaults", Some(subcommand_matches)) => {
            let token1 = config::get_str_val(subcommand_matches, "token_name");
            let token2 = config::get_str_val(subcommand_matches, "token_name2");
            match client.find_vaults(&token1, &token2) {
                Ok(vaults) => {
                    for vault in vaults {
                        println!("{}", vault.name);
                    }
                }
                Err(e) => {
                    println!("{}", e);
                }
            }
        }
        ("find-vaults-with-lp", Some(subcommand_matches)) => {
            let vt_token = config::get_str_val(subcommand_matches, "token_name");
            match client.find_vaults_with_vt(&vt_token) {
                Ok(vaults) => {
                    for vault in vaults {
                        println!("{}", vault.name);
                    }
                }
                Err(e) => {
                    println!("{}", e);
                }
            }
        }
        ("swap", Some(subcommand_matches)) => {
            let protocol = config::get_str_val(subcommand_matches, "protocol");
            let token_from = config::get_str_val(subcommand_matches, "token_name");
            let token_to = config::get_str_val(subcommand_matches, "token_name2");
            let amount_in = config::get_floating_val(subcommand_matches, "amount");
            let min_amount_out = config::get_floating_val(subcommand_matches, "amount2");
            println!(
                "Done: {}",
                client
                    .swap(
                        config.keypair.as_ref(),
                        protocol.parse().expect("Failed to parse protocol argument"),
                        &token_from,
                        &token_to,
                        amount_in,
                        min_amount_out
                    )
                    .unwrap()
            );
        }
        ("deposit-pool", Some(subcommand_matches)) => {
            let pool_name = config::get_str_val(subcommand_matches, "pool_name");
            let token_a_amount = config::get_floating_val(subcommand_matches, "amount");
            let token_b_amount = config::get_floating_val(subcommand_matches, "amount2");
            println!(
                "Done: {}",
                client
                    .add_liquidity_pool(
                        config.keypair.as_ref(),
                        &pool_name,
                        token_a_amount,
                        token_b_amount
                    )
                    .unwrap()
            );
        }
        ("withdraw-pool", Some(subcommand_matches)) => {
            let pool_name = config::get_str_val(subcommand_matches, "pool_name");
            let amount = config::get_floating_val(subcommand_matches, "amount");
            println!(
                "Done: {}",
                client
                    .remove_liquidity_pool(config.keypair.as_ref(), &pool_name, amount)
                    .unwrap()
            );
        }
        ("stake", Some(subcommand_matches)) => {
            let farm_name = config::get_str_val(subcommand_matches, "farm_name");
            let amount = config::get_floating_val(subcommand_matches, "amount");
            println!(
                "Done: {}",
                client
                    .stake(config.keypair.as_ref(), &farm_name, amount)
                    .unwrap()
            );
        }
        ("harvest", Some(subcommand_matches)) => {
            let farm_name = config::get_str_val(subcommand_matches, "farm_name");
            println!(
                "Done: {}",
                client.harvest(config.keypair.as_ref(), &farm_name).unwrap()
            );
        }
        ("unstake", Some(subcommand_matches)) => {
            let farm_name = config::get_str_val(subcommand_matches, "farm_name");
            let amount = config::get_floating_val(subcommand_matches, "amount");
            println!(
                "Done: {}",
                client
                    .unstake(config.keypair.as_ref(), &farm_name, amount)
                    .unwrap()
            );
        }
        ("deposit-vault", Some(subcommand_matches)) => {
            let vault_name = config::get_str_val(subcommand_matches, "vault_name");
            let token_a_amount = config::get_floating_val(subcommand_matches, "amount");
            let token_b_amount = config::get_floating_val(subcommand_matches, "amount2");
            println!(
                "Done: {}",
                client
                    .add_liquidity_vault(
                        config.keypair.as_ref(),
                        &vault_name,
                        token_a_amount,
                        token_b_amount
                    )
                    .unwrap()
            );
        }
        ("deposit-vault-locked", Some(subcommand_matches)) => {
            let vault_name = config::get_str_val(subcommand_matches, "vault_name");
            let amount = config::get_floating_val(subcommand_matches, "amount");
            println!(
                "Done: {}",
                client
                    .add_locked_liquidity_vault(config.keypair.as_ref(), &vault_name, amount)
                    .unwrap()
            );
        }
        ("withdraw-vault", Some(subcommand_matches)) => {
            let vault_name = config::get_str_val(subcommand_matches, "vault_name");
            let amount = config::get_floating_val(subcommand_matches, "amount");
            println!(
                "Done: {}",
                client
                    .remove_liquidity_vault(config.keypair.as_ref(), &vault_name, amount)
                    .unwrap()
            );
        }
        ("withdraw-vault-unlocked", Some(subcommand_matches)) => {
            let vault_name = config::get_str_val(subcommand_matches, "vault_name");
            let amount = config::get_floating_val(subcommand_matches, "amount");
            println!(
                "Done: {}",
                client
                    .remove_unlocked_liquidity_vault(config.keypair.as_ref(), &vault_name, amount)
                    .unwrap()
            );
        }
        ("crank-vault", Some(subcommand_matches)) => {
            let vault_name = config::get_str_val(subcommand_matches, "vault_name");
            let step = config::get_integer_val(subcommand_matches, "step");
            println!(
                "Done: {}",
                client
                    .crank_vault(config.keypair.as_ref(), &vault_name, step)
                    .unwrap()
            );
        }
        ("crank-vaults", Some(subcommand_matches)) => {
            let step = config::get_integer_val(subcommand_matches, "step");
            println!(
                "Done: {} vaults cranked",
                client.crank_vaults(config.keypair.as_ref(), step).unwrap()
            );
        }
        ("request-deposit-fund", Some(subcommand_matches)) => {
            let fund_name = config::get_str_val(subcommand_matches, "fund_name");
            let token = config::get_str_val(subcommand_matches, "token_name");
            let amount = config::get_floating_val(subcommand_matches, "amount");
            println!(
                "Done: {}",
                client
                    .request_deposit_fund(config.keypair.as_ref(), &fund_name, &token, amount)
                    .unwrap()
            );
        }
        ("cancel-deposit-fund", Some(subcommand_matches)) => {
            let fund_name = config::get_str_val(subcommand_matches, "fund_name");
            let token = config::get_str_val(subcommand_matches, "token_name");
            println!(
                "Done: {}",
                client
                    .cancel_deposit_fund(config.keypair.as_ref(), &fund_name, &token)
                    .unwrap()
            );
        }
        ("request-withdrawal-fund", Some(subcommand_matches)) => {
            let fund_name = config::get_str_val(subcommand_matches, "fund_name");
            let token = config::get_str_val(subcommand_matches, "token_name");
            let amount = config::get_floating_val(subcommand_matches, "amount");
            println!(
                "Done: {}",
                client
                    .request_withdrawal_fund(config.keypair.as_ref(), &fund_name, &token, amount)
                    .unwrap()
            );
        }
        ("cancel-withdrawal-fund", Some(subcommand_matches)) => {
            let fund_name = config::get_str_val(subcommand_matches, "fund_name");
            let token = config::get_str_val(subcommand_matches, "token_name");
            println!(
                "Done: {}",
                client
                    .cancel_withdrawal_fund(config.keypair.as_ref(), &fund_name, &token)
                    .unwrap()
            );
        }
        ("start-liquidation-fund", Some(subcommand_matches)) => {
            let fund_name = config::get_str_val(subcommand_matches, "fund_name");
            println!(
                "Done: {}",
                client
                    .start_liquidation_fund(config.keypair.as_ref(), &fund_name)
                    .unwrap()
            );
        }
        ("update-fund-assets-with-custody", Some(subcommand_matches)) => {
            let fund_name = config::get_str_val(subcommand_matches, "fund_name");
            let custody_id = config::get_integer_val(subcommand_matches, "custody_id");
            println!(
                "Done: {}",
                client
                    .update_fund_assets_with_custody(
                        config.keypair.as_ref(),
                        &fund_name,
                        custody_id as u32
                    )
                    .unwrap()
            );
        }
        ("update-fund-assets-with-custodies", Some(subcommand_matches)) => {
            let fund_name = config::get_str_val(subcommand_matches, "fund_name");
            println!(
                "Done: {} custodies processed",
                client
                    .update_fund_assets_with_custodies(config.keypair.as_ref(), &fund_name)
                    .unwrap()
            );
        }
        ("update-fund-assets-with-vault", Some(subcommand_matches)) => {
            let fund_name = config::get_str_val(subcommand_matches, "fund_name");
            let vault_id = config::get_integer_val(subcommand_matches, "vault_id");
            println!(
                "Done: {}",
                client
                    .update_fund_assets_with_vault(
                        config.keypair.as_ref(),
                        &fund_name,
                        vault_id as u32
                    )
                    .unwrap()
            );
        }
        ("update-fund-assets-with-vaults", Some(subcommand_matches)) => {
            let fund_name = config::get_str_val(subcommand_matches, "fund_name");
            println!(
                "Done: {} vaults processed",
                client
                    .update_fund_assets_with_vaults(config.keypair.as_ref(), &fund_name)
                    .unwrap()
            );
        }
        ("governance", Some(subcommand_matches)) => match subcommand_matches.subcommand() {
            ("get-config", Some(subcommand_matches)) => {
                let governance_name =
                    config::get_str_val_raw(subcommand_matches, "governance_name");
                let governance_config = client.governance_get_config(&governance_name).unwrap();
                println!("{:#?}", governance_config);
            }
            ("get-address", Some(subcommand_matches)) => {
                let governance_name =
                    config::get_str_val_raw(subcommand_matches, "governance_name");
                let governance_address = client.governance_get_address(&governance_name).unwrap();
                println!("{}: {}", governance_name, governance_address);
            }
            ("get-instruction", Some(subcommand_matches)) => {
                let (governance_name, proposal_index, instruction_index) =
                    get_instruction_args(subcommand_matches);
                let stored_instruction = client
                    .governance_get_instruction(&governance_name, proposal_index, instruction_index)
                    .unwrap();
                println!("{:#?}", stored_instruction);
            }
            ("custody-new", Some(subcommand_matches)) => {
                let token_name = config::get_str_val_raw(subcommand_matches, "token_name");
                let custody_authority = client.governance_get_address(DAO_CUSTODY_NAME).unwrap();

                // create wsol account for custody authority
                if !client.has_active_token_account(&custody_authority, &token_name) {
                    let token = client.get_token(&token_name).unwrap();
                    let inst =
                        create_associated_token_account(&wallet, &custody_authority, &token.mint);
                    println!(
                        "Done: {}",
                        client
                            .sign_and_send_instructions(&[config.keypair.as_ref()], &[inst])
                            .unwrap()
                    );
                }
                println!(
                    "{} account: {}",
                    token_name,
                    client
                        .get_associated_token_address(&custody_authority, &token_name)
                        .unwrap()
                );
            }
            ("tokens-deposit", Some(subcommand_matches)) => {
                let amount = config::get_floating_val(subcommand_matches, "amount");
                println!(
                    "Done: {}",
                    client
                        .governance_tokens_deposit(config.keypair.as_ref(), amount)
                        .unwrap()
                );
            }
            ("tokens-withdraw", Some(_subcommand_matches)) => {
                println!(
                    "Done: {}",
                    client
                        .governance_tokens_withdraw(config.keypair.as_ref())
                        .unwrap()
                );
            }
            ("proposal-new", Some(subcommand_matches)) => {
                let governance_name =
                    config::get_str_val_raw(subcommand_matches, "governance_name");
                let proposal_name = config::get_str_val_raw(subcommand_matches, "proposal_name");
                let proposal_link = config::get_str_val_raw(subcommand_matches, "proposal_link");
                let proposal_index = config::get_integer_val(subcommand_matches, "proposal_index");
                println!(
                    "Done: {}",
                    client
                        .governance_proposal_new(
                            config.keypair.as_ref(),
                            &governance_name,
                            &proposal_name,
                            &proposal_link,
                            proposal_index as u32
                        )
                        .unwrap()
                );
            }
            ("proposal-cancel", Some(subcommand_matches)) => {
                let governance_name =
                    config::get_str_val_raw(subcommand_matches, "governance_name");
                let proposal_index = config::get_integer_val(subcommand_matches, "proposal_index");
                println!(
                    "Done: {}",
                    client
                        .governance_proposal_cancel(
                            config.keypair.as_ref(),
                            &governance_name,
                            proposal_index as u32
                        )
                        .unwrap()
                );
            }
            ("proposal-state", Some(subcommand_matches)) => {
                let governance_name =
                    config::get_str_val_raw(subcommand_matches, "governance_name");
                let proposal_index = config::get_integer_val(subcommand_matches, "proposal_index");
                let governance_state = client
                    .governance_get_proposal_state(&governance_name, proposal_index as u32)
                    .unwrap();
                println!("{:#?}", governance_state);
            }
            ("signatory-add", Some(subcommand_matches)) => {
                let governance_name =
                    config::get_str_val_raw(subcommand_matches, "governance_name");
                let proposal_index = config::get_integer_val(subcommand_matches, "proposal_index");
                let signatory =
                    Pubkey::from_str(&config::get_str_val_raw(subcommand_matches, "signatory"))
                        .unwrap();
                println!(
                    "Done: {}",
                    client
                        .governance_signatory_add(
                            config.keypair.as_ref(),
                            &governance_name,
                            proposal_index as u32,
                            &signatory
                        )
                        .unwrap()
                );
            }
            ("signatory-remove", Some(subcommand_matches)) => {
                let governance_name =
                    config::get_str_val_raw(subcommand_matches, "governance_name");
                let proposal_index = config::get_integer_val(subcommand_matches, "proposal_index");
                let signatory =
                    Pubkey::from_str(&config::get_str_val_raw(subcommand_matches, "signatory"))
                        .unwrap();
                println!(
                    "Done: {}",
                    client
                        .governance_signatory_remove(
                            config.keypair.as_ref(),
                            &governance_name,
                            proposal_index as u32,
                            &signatory
                        )
                        .unwrap()
                );
            }
            ("sign-off", Some(subcommand_matches)) => {
                let governance_name =
                    config::get_str_val_raw(subcommand_matches, "governance_name");
                let proposal_index = config::get_integer_val(subcommand_matches, "proposal_index");
                println!(
                    "Done: {}",
                    client
                        .governance_sign_off(
                            config.keypair.as_ref(),
                            &governance_name,
                            proposal_index as u32
                        )
                        .unwrap()
                );
            }
            ("vote-cast", Some(subcommand_matches)) => {
                let governance_name =
                    config::get_str_val_raw(subcommand_matches, "governance_name");
                let proposal_index = config::get_integer_val(subcommand_matches, "proposal_index");
                let vote = config::get_integer_val(subcommand_matches, "vote");
                println!(
                    "Done: {}",
                    client
                        .governance_vote_cast(
                            config.keypair.as_ref(),
                            &governance_name,
                            proposal_index as u32,
                            vote as u8
                        )
                        .unwrap()
                );
            }
            ("vote-relinquish", Some(subcommand_matches)) => {
                let governance_name =
                    config::get_str_val_raw(subcommand_matches, "governance_name");
                let proposal_index = config::get_integer_val(subcommand_matches, "proposal_index");
                println!(
                    "Done: {}",
                    client
                        .governance_vote_relinquish(
                            config.keypair.as_ref(),
                            &governance_name,
                            proposal_index as u32
                        )
                        .unwrap()
                );
            }
            ("vote-finalize", Some(subcommand_matches)) => {
                let governance_name =
                    config::get_str_val_raw(subcommand_matches, "governance_name");
                let proposal_index = config::get_integer_val(subcommand_matches, "proposal_index");
                println!(
                    "Done: {}",
                    client
                        .governance_vote_finalize(
                            config.keypair.as_ref(),
                            &governance_name,
                            proposal_index as u32
                        )
                        .unwrap()
                );
            }
            ("instruction-execute", Some(subcommand_matches)) => {
                let (governance_name, proposal_index, instruction_index) =
                    get_instruction_args(subcommand_matches);
                println!(
                    "Done: {}",
                    client
                        .governance_instruction_execute(
                            config.keypair.as_ref(),
                            &governance_name,
                            proposal_index,
                            instruction_index,
                        )
                        .unwrap()
                );
            }
            ("instruction-flag-error", Some(subcommand_matches)) => {
                let (governance_name, proposal_index, instruction_index) =
                    get_instruction_args(subcommand_matches);
                println!(
                    "Done: {}",
                    client
                        .governance_instruction_flag_error(
                            config.keypair.as_ref(),
                            &governance_name,
                            proposal_index,
                            instruction_index,
                        )
                        .unwrap()
                );
            }
            ("instruction-remove", Some(subcommand_matches)) => {
                let (governance_name, proposal_index, instruction_index) =
                    get_instruction_args(subcommand_matches);
                println!(
                    "Done: {}",
                    client
                        .governance_instruction_remove(
                            config.keypair.as_ref(),
                            &governance_name,
                            proposal_index,
                            instruction_index,
                        )
                        .unwrap()
                );
            }
            ("instruction-insert", Some(subcommand_matches)) => {
                let (governance_name, proposal_index, instruction_index) =
                    get_instruction_args(subcommand_matches);
                let instruction_str =
                    config::get_str_val_raw(subcommand_matches, "base64_instruction");
                let data = base64::decode(&instruction_str).unwrap();
                let instruction: Instruction = bincode::deserialize(data.as_slice()).unwrap();
                println!(
                    "Done: {}",
                    client
                        .governance_instruction_insert(
                            config.keypair.as_ref(),
                            &governance_name,
                            proposal_index,
                            instruction_index,
                            &instruction
                        )
                        .unwrap()
                );
            }
            ("instruction-verify", Some(subcommand_matches)) => {
                let instruction_str =
                    config::get_str_val_raw(subcommand_matches, "base64_instruction");
                let data = base64::decode(&instruction_str).unwrap();
                let instruction: Instruction = bincode::deserialize(data.as_slice()).unwrap();

                verify_instruction(&client, subcommand_matches, &instruction);
            }
            ("instruction-insert-token-transfer", Some(subcommand_matches)) => {
                let (governance_name, proposal_index, instruction_index) =
                    get_instruction_args(subcommand_matches);
                let token_name = config::get_str_val(subcommand_matches, "token_name");
                let destination = config::get_pubkey_val(subcommand_matches, "wallet");
                let amount = config::get_floating_val(subcommand_matches, "amount");
                let custody_authority = client.governance_get_address(DAO_CUSTODY_NAME).unwrap();
                let instruction = client
                    .new_instruction_token_transfer(
                        &custody_authority,
                        &token_name,
                        &destination,
                        amount,
                    )
                    .unwrap();

                println!(
                    "Done: {}",
                    client
                        .governance_instruction_insert(
                            config.keypair.as_ref(),
                            &governance_name,
                            proposal_index,
                            instruction_index,
                            &instruction
                        )
                        .unwrap()
                );
            }
            ("instruction-verify-token-transfer", Some(subcommand_matches)) => {
                let token_name = config::get_str_val(subcommand_matches, "token_name");
                let destination = config::get_pubkey_val(subcommand_matches, "wallet");
                let amount = config::get_floating_val(subcommand_matches, "amount");
                let custody_authority = client.governance_get_address(DAO_CUSTODY_NAME).unwrap();
                let instruction = client
                    .new_instruction_token_transfer(
                        &custody_authority,
                        &token_name,
                        &destination,
                        amount,
                    )
                    .unwrap();

                verify_instruction(&client, subcommand_matches, &instruction);
            }
            ("instruction-insert-swap", Some(subcommand_matches)) => {
                let (governance_name, proposal_index, instruction_index) =
                    get_instruction_args(subcommand_matches);
                let protocol = config::get_str_val(subcommand_matches, "protocol");
                let token_from = config::get_str_val(subcommand_matches, "token_name");
                let token_to = config::get_str_val(subcommand_matches, "token_name2");
                let amount_in = config::get_floating_val(subcommand_matches, "amount");
                let min_amount_out = config::get_floating_val(subcommand_matches, "amount2");
                let custody_authority = client.governance_get_address(DAO_CUSTODY_NAME).unwrap();
                let instruction = client
                    .new_instruction_swap(
                        &custody_authority,
                        protocol.parse().expect("Failed to parse protocol argument"),
                        &token_from,
                        &token_to,
                        amount_in,
                        min_amount_out,
                    )
                    .unwrap();

                println!(
                    "Done: {}",
                    client
                        .governance_instruction_insert(
                            config.keypair.as_ref(),
                            &governance_name,
                            proposal_index,
                            instruction_index,
                            &instruction
                        )
                        .unwrap()
                );
            }
            ("instruction-verify-swap", Some(subcommand_matches)) => {
                let protocol = config::get_str_val(subcommand_matches, "protocol");
                let token_from = config::get_str_val(subcommand_matches, "token_name");
                let token_to = config::get_str_val(subcommand_matches, "token_name2");
                let amount_in = config::get_floating_val(subcommand_matches, "amount");
                let min_amount_out = config::get_floating_val(subcommand_matches, "amount2");
                let custody_authority = client.governance_get_address(DAO_CUSTODY_NAME).unwrap();
                let instruction = client
                    .new_instruction_swap(
                        &custody_authority,
                        protocol.parse().expect("Failed to parse protocol argument"),
                        &token_from,
                        &token_to,
                        amount_in,
                        min_amount_out,
                    )
                    .unwrap();

                verify_instruction(&client, subcommand_matches, &instruction);
            }
            ("instruction-insert-deposit-pool", Some(subcommand_matches)) => {
                let (governance_name, proposal_index, instruction_index) =
                    get_instruction_args(subcommand_matches);
                let pool_name = config::get_str_val(subcommand_matches, "pool_name");
                let token_a_amount = config::get_floating_val(subcommand_matches, "amount");
                let token_b_amount = config::get_floating_val(subcommand_matches, "amount2");
                let custody_authority = client.governance_get_address(DAO_CUSTODY_NAME).unwrap();
                let instruction = client
                    .new_instruction_add_liquidity_pool(
                        &custody_authority,
                        &pool_name,
                        token_a_amount,
                        token_b_amount,
                    )
                    .unwrap();

                println!(
                    "Done: {}",
                    client
                        .governance_instruction_insert(
                            config.keypair.as_ref(),
                            &governance_name,
                            proposal_index,
                            instruction_index,
                            &instruction
                        )
                        .unwrap()
                );
            }
            ("instruction-verify-deposit-pool", Some(subcommand_matches)) => {
                let pool_name = config::get_str_val(subcommand_matches, "pool_name");
                let token_a_amount = config::get_floating_val(subcommand_matches, "amount");
                let token_b_amount = config::get_floating_val(subcommand_matches, "amount2");
                let custody_authority = client.governance_get_address(DAO_CUSTODY_NAME).unwrap();
                let instruction = client
                    .new_instruction_add_liquidity_pool(
                        &custody_authority,
                        &pool_name,
                        token_a_amount,
                        token_b_amount,
                    )
                    .unwrap();

                verify_instruction(&client, subcommand_matches, &instruction);
            }
            ("instruction-insert-withdraw-pool", Some(subcommand_matches)) => {
                let (governance_name, proposal_index, instruction_index) =
                    get_instruction_args(subcommand_matches);
                let pool_name = config::get_str_val(subcommand_matches, "pool_name");
                let amount = config::get_floating_val(subcommand_matches, "amount");
                let custody_authority = client.governance_get_address(DAO_CUSTODY_NAME).unwrap();
                let instruction = client
                    .new_instruction_remove_liquidity_pool(&custody_authority, &pool_name, amount)
                    .unwrap();

                println!(
                    "Done: {}",
                    client
                        .governance_instruction_insert(
                            config.keypair.as_ref(),
                            &governance_name,
                            proposal_index,
                            instruction_index,
                            &instruction
                        )
                        .unwrap()
                );
            }
            ("instruction-verify-withdraw-pool", Some(subcommand_matches)) => {
                let pool_name = config::get_str_val(subcommand_matches, "pool_name");
                let amount = config::get_floating_val(subcommand_matches, "amount");
                let custody_authority = client.governance_get_address(DAO_CUSTODY_NAME).unwrap();
                let instruction = client
                    .new_instruction_remove_liquidity_pool(&custody_authority, &pool_name, amount)
                    .unwrap();

                verify_instruction(&client, subcommand_matches, &instruction);
            }
            ("instruction-insert-stake", Some(subcommand_matches)) => {
                let (governance_name, proposal_index, instruction_index) =
                    get_instruction_args(subcommand_matches);
                let farm_name = config::get_str_val(subcommand_matches, "farm_name");
                let amount = config::get_floating_val(subcommand_matches, "amount");
                let custody_authority = client.governance_get_address(DAO_CUSTODY_NAME).unwrap();
                let instruction = client
                    .new_instruction_stake(&custody_authority, &farm_name, amount)
                    .unwrap();

                println!(
                    "Done: {}",
                    client
                        .governance_instruction_insert(
                            config.keypair.as_ref(),
                            &governance_name,
                            proposal_index,
                            instruction_index,
                            &instruction
                        )
                        .unwrap()
                );
            }
            ("instruction-verify-stake", Some(subcommand_matches)) => {
                let farm_name = config::get_str_val(subcommand_matches, "farm_name");
                let amount = config::get_floating_val(subcommand_matches, "amount");
                let custody_authority = client.governance_get_address(DAO_CUSTODY_NAME).unwrap();
                let instruction = client
                    .new_instruction_stake(&custody_authority, &farm_name, amount)
                    .unwrap();

                verify_instruction(&client, subcommand_matches, &instruction);
            }
            ("instruction-insert-harvest", Some(subcommand_matches)) => {
                let (governance_name, proposal_index, instruction_index) =
                    get_instruction_args(subcommand_matches);
                let farm_name = config::get_str_val(subcommand_matches, "farm_name");
                let custody_authority = client.governance_get_address(DAO_CUSTODY_NAME).unwrap();
                let instruction = client
                    .new_instruction_harvest(&custody_authority, &farm_name)
                    .unwrap();

                println!(
                    "Done: {}",
                    client
                        .governance_instruction_insert(
                            config.keypair.as_ref(),
                            &governance_name,
                            proposal_index,
                            instruction_index,
                            &instruction
                        )
                        .unwrap()
                );
            }
            ("instruction-verify-harvest", Some(subcommand_matches)) => {
                let farm_name = config::get_str_val(subcommand_matches, "farm_name");
                let custody_authority = client.governance_get_address(DAO_CUSTODY_NAME).unwrap();
                let instruction = client
                    .new_instruction_harvest(&custody_authority, &farm_name)
                    .unwrap();

                verify_instruction(&client, subcommand_matches, &instruction);
            }
            ("instruction-insert-unstake", Some(subcommand_matches)) => {
                let (governance_name, proposal_index, instruction_index) =
                    get_instruction_args(subcommand_matches);
                let farm_name = config::get_str_val(subcommand_matches, "farm_name");
                let amount = config::get_floating_val(subcommand_matches, "amount");
                let custody_authority = client.governance_get_address(DAO_CUSTODY_NAME).unwrap();
                let instruction = client
                    .new_instruction_unstake(&custody_authority, &farm_name, amount)
                    .unwrap();

                println!(
                    "Done: {}",
                    client
                        .governance_instruction_insert(
                            config.keypair.as_ref(),
                            &governance_name,
                            proposal_index,
                            instruction_index,
                            &instruction
                        )
                        .unwrap()
                );
            }
            ("instruction-verify-unstake", Some(subcommand_matches)) => {
                let farm_name = config::get_str_val(subcommand_matches, "farm_name");
                let amount = config::get_floating_val(subcommand_matches, "amount");
                let custody_authority = client.governance_get_address(DAO_CUSTODY_NAME).unwrap();
                let instruction = client
                    .new_instruction_unstake(&custody_authority, &farm_name, amount)
                    .unwrap();

                verify_instruction(&client, subcommand_matches, &instruction);
            }
            ("instruction-insert-deposit-vault", Some(subcommand_matches)) => {
                let (governance_name, proposal_index, instruction_index) =
                    get_instruction_args(subcommand_matches);
                let vault_name = config::get_str_val(subcommand_matches, "vault_name");
                let token_a_amount = config::get_floating_val(subcommand_matches, "amount");
                let token_b_amount = config::get_floating_val(subcommand_matches, "amount2");
                let custody_authority = client.governance_get_address(DAO_CUSTODY_NAME).unwrap();
                let instruction = client
                    .new_instruction_add_liquidity_vault(
                        &custody_authority,
                        &vault_name,
                        token_a_amount,
                        token_b_amount,
                    )
                    .unwrap();

                println!(
                    "Done: {}",
                    client
                        .governance_instruction_insert(
                            config.keypair.as_ref(),
                            &governance_name,
                            proposal_index,
                            instruction_index,
                            &instruction
                        )
                        .unwrap()
                );
            }
            ("instruction-verify-deposit-vault", Some(subcommand_matches)) => {
                let vault_name = config::get_str_val(subcommand_matches, "vault_name");
                let token_a_amount = config::get_floating_val(subcommand_matches, "amount");
                let token_b_amount = config::get_floating_val(subcommand_matches, "amount2");
                let custody_authority = client.governance_get_address(DAO_CUSTODY_NAME).unwrap();
                let instruction = client
                    .new_instruction_add_liquidity_vault(
                        &custody_authority,
                        &vault_name,
                        token_a_amount,
                        token_b_amount,
                    )
                    .unwrap();

                verify_instruction(&client, subcommand_matches, &instruction);
            }
            ("instruction-insert-withdraw-vault", Some(subcommand_matches)) => {
                let (governance_name, proposal_index, instruction_index) =
                    get_instruction_args(subcommand_matches);
                let vault_name = config::get_str_val(subcommand_matches, "vault_name");
                let amount = config::get_floating_val(subcommand_matches, "amount");
                let custody_authority = client.governance_get_address(DAO_CUSTODY_NAME).unwrap();
                let instruction = client
                    .new_instruction_remove_liquidity_vault(&custody_authority, &vault_name, amount)
                    .unwrap();

                println!(
                    "Done: {}",
                    client
                        .governance_instruction_insert(
                            config.keypair.as_ref(),
                            &governance_name,
                            proposal_index,
                            instruction_index,
                            &instruction
                        )
                        .unwrap()
                );
            }
            ("instruction-verify-withdraw-vault", Some(subcommand_matches)) => {
                let vault_name = config::get_str_val(subcommand_matches, "vault_name");
                let amount = config::get_floating_val(subcommand_matches, "amount");
                let custody_authority = client.governance_get_address(DAO_CUSTODY_NAME).unwrap();
                let instruction = client
                    .new_instruction_remove_liquidity_vault(&custody_authority, &vault_name, amount)
                    .unwrap();

                verify_instruction(&client, subcommand_matches, &instruction);
            }
            ("instruction-insert-withdraw-fees-vault", Some(subcommand_matches)) => {
                let (governance_name, proposal_index, instruction_index) =
                    get_instruction_args(subcommand_matches);
                let vault_name = config::get_str_val(subcommand_matches, "vault_name");
                let fee_token = TokenSelector::try_from_primitive(config::get_integer_val(
                    subcommand_matches,
                    "fee_token",
                ) as u8)
                .unwrap();
                let amount = config::get_floating_val(subcommand_matches, "amount");
                let receiver = config::get_pubkey_val(subcommand_matches, "receiver");
                let custody_authority = client.governance_get_address(DAO_CUSTODY_NAME).unwrap();
                let instruction = client
                    .new_instruction_withdraw_fees_vault(
                        &custody_authority,
                        &vault_name,
                        fee_token,
                        amount,
                        &receiver,
                    )
                    .unwrap();

                println!(
                    "Done: {}",
                    client
                        .governance_instruction_insert(
                            config.keypair.as_ref(),
                            &governance_name,
                            proposal_index,
                            instruction_index,
                            &instruction
                        )
                        .unwrap()
                );
            }
            ("instruction-verify-withdraw-fees-vault", Some(subcommand_matches)) => {
                let vault_name = config::get_str_val(subcommand_matches, "vault_name");
                let fee_token = TokenSelector::try_from_primitive(config::get_integer_val(
                    subcommand_matches,
                    "fee_token",
                ) as u8)
                .unwrap();
                let amount = config::get_floating_val(subcommand_matches, "amount");
                let receiver = config::get_pubkey_val(subcommand_matches, "receiver");
                let custody_authority = client.governance_get_address(DAO_CUSTODY_NAME).unwrap();
                let instruction = client
                    .new_instruction_withdraw_fees_vault(
                        &custody_authority,
                        &vault_name,
                        fee_token,
                        amount,
                        &receiver,
                    )
                    .unwrap();

                verify_instruction(&client, subcommand_matches, &instruction);
            }
            ("instruction-insert-program-upgrade", Some(subcommand_matches)) => {
                let (governance_name, proposal_index, instruction_index) =
                    get_instruction_args(subcommand_matches);
                let program_address = client.get_program_id(&governance_name).unwrap();
                let buffer_address = config::get_pubkey_val(subcommand_matches, "buffer_address");
                let program_authority = client.governance_get_address(&governance_name).unwrap();
                let instruction = bpf_loader_upgradeable::upgrade(
                    &program_address,
                    &buffer_address,
                    &program_authority,
                    &config.keypair.pubkey(),
                );

                println!(
                    "Done: {}",
                    client
                        .governance_instruction_insert(
                            config.keypair.as_ref(),
                            &governance_name,
                            proposal_index,
                            instruction_index,
                            &instruction
                        )
                        .unwrap()
                );
            }
            ("instruction-verify-program-upgrade", Some(subcommand_matches)) => {
                let (governance_name, _proposal_index, _instruction_index) =
                    get_instruction_args(subcommand_matches);
                let program_address = client.get_program_id(&governance_name).unwrap();
                let buffer_address = config::get_pubkey_val(subcommand_matches, "buffer_address");
                let program_authority = client.governance_get_address(&governance_name).unwrap();
                let instruction = bpf_loader_upgradeable::upgrade(
                    &program_address,
                    &buffer_address,
                    &program_authority,
                    &config.keypair.pubkey(),
                );

                verify_instruction(&client, subcommand_matches, &instruction);
            }
            _ => unreachable!(),
        },
        _ => error!("Unrecognized command. Use --help to list known commands."),
    };
}

fn get_instruction_args(matches: &ArgMatches) -> (String, u32, u16) {
    let governance_name = config::get_str_val_raw(matches, "governance_name");
    let proposal_index = config::get_integer_val(matches, "proposal_index") as u32;
    let instruction_index = config::get_integer_val(matches, "instruction_index") as u16;
    (governance_name, proposal_index, instruction_index)
}

fn verify_instruction(client: &FarmClient, matches: &ArgMatches, instruction: &Instruction) {
    let (governance_name, proposal_index, instruction_index) = get_instruction_args(matches);
    let stored_instruction = client
        .governance_get_instruction(&governance_name, proposal_index, instruction_index)
        .unwrap();

    if instruction == &stored_instruction {
        println!("Instructions match.");
    } else {
        println!("Instructions are different.");
        println!("Expected: {:?}", instruction);
        println!("Stored: {:?}", stored_instruction);
    }
}
