//! Command-line interface for the Farm Client

mod config;
mod printer;

use {
    log::error,
    solana_farm_client::client::FarmClient,
    solana_sdk::{instruction::Instruction, pubkey::Pubkey},
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
            let objects = config::get_vec_str_val(subcommand_matches, "object_name");
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
        ("pool-price", Some(subcommand_matches)) => {
            let pools = config::get_vec_str_val(subcommand_matches, "pool_name");
            for pool in pools {
                println!("{} price: {}", pool, client.get_pool_price(&pool).unwrap());
            }
        }
        ("transfer", Some(subcommand_matches)) => {
            let destination = config::get_pubkey_val(subcommand_matches, "wallet");
            let amount = config::get_amount_val(subcommand_matches, "amount");
            client
                .transfer(config.keypair.as_ref(), &destination, amount)
                .unwrap();
        }
        ("token-transfer", Some(subcommand_matches)) => {
            let token_name = config::get_str_val(subcommand_matches, "token_name");
            let destination = config::get_pubkey_val(subcommand_matches, "wallet");
            let amount = config::get_amount_val(subcommand_matches, "amount");
            client
                .token_transfer(config.keypair.as_ref(), &token_name, &destination, amount)
                .unwrap();
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
        ("token-create", Some(subcommand_matches)) => {
            let tokens = config::get_vec_str_val(subcommand_matches, "token_name");
            for token in tokens {
                println!(
                    "{} address: {}",
                    token,
                    client
                        .get_or_create_token_account(config.keypair.as_ref(), &token)
                        .unwrap()
                );
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
        ("find-pools", Some(subcommand_matches)) => {
            let protocol = config::get_str_val(subcommand_matches, "protocol");
            let token1 = config::get_str_val(subcommand_matches, "token_name");
            let token2 = config::get_str_val(subcommand_matches, "token_name2");
            match client.find_pools(&protocol, &token1, &token2) {
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
        ("swap", Some(subcommand_matches)) => {
            let protocol = config::get_str_val(subcommand_matches, "protocol");
            let token_from = config::get_str_val(subcommand_matches, "token_name");
            let token_to = config::get_str_val(subcommand_matches, "token_name2");
            let amount_in = config::get_amount_val(subcommand_matches, "amount");
            let min_amount_out = config::get_amount_val(subcommand_matches, "amount2");
            println!(
                "Done: {}",
                client
                    .swap(
                        config.keypair.as_ref(),
                        &protocol,
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
            let token_a_amount = config::get_amount_val(subcommand_matches, "amount");
            let token_b_amount = config::get_amount_val(subcommand_matches, "amount2");
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
            let amount = config::get_amount_val(subcommand_matches, "amount");
            println!(
                "Done: {}",
                client
                    .remove_liquidity_pool(config.keypair.as_ref(), &pool_name, amount)
                    .unwrap()
            );
        }
        ("stake", Some(subcommand_matches)) => {
            let farm_name = config::get_str_val(subcommand_matches, "farm_name");
            let amount = config::get_amount_val(subcommand_matches, "amount");
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
            let amount = config::get_amount_val(subcommand_matches, "amount");
            println!(
                "Done: {}",
                client
                    .unstake(config.keypair.as_ref(), &farm_name, amount)
                    .unwrap()
            );
        }
        ("deposit-vault", Some(subcommand_matches)) => {
            let vault_name = config::get_str_val(subcommand_matches, "vault_name");
            let token_a_amount = config::get_amount_val(subcommand_matches, "amount");
            let token_b_amount = config::get_amount_val(subcommand_matches, "amount2");
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
            let amount = config::get_amount_val(subcommand_matches, "amount");
            println!(
                "Done: {}",
                client
                    .add_locked_liquidity_vault(config.keypair.as_ref(), &vault_name, amount)
                    .unwrap()
            );
        }
        ("withdraw-vault", Some(subcommand_matches)) => {
            let vault_name = config::get_str_val(subcommand_matches, "vault_name");
            let amount = config::get_amount_val(subcommand_matches, "amount");
            println!(
                "Done: {}",
                client
                    .remove_liquidity_vault(config.keypair.as_ref(), &vault_name, amount)
                    .unwrap()
            );
        }
        ("withdraw-vault-unlocked", Some(subcommand_matches)) => {
            let vault_name = config::get_str_val(subcommand_matches, "vault_name");
            let amount = config::get_amount_val(subcommand_matches, "amount");
            println!(
                "Done: {}",
                client
                    .remove_unlocked_liquidity_vault(config.keypair.as_ref(), &vault_name, amount)
                    .unwrap()
            );
        }
        ("governance", Some(subcommand_matches)) => match subcommand_matches.subcommand() {
            ("tokens-deposit", Some(subcommand_matches)) => {
                let amount = config::get_amount_val(subcommand_matches, "amount");
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
                let governed_account_name =
                    config::get_str_val_raw(subcommand_matches, "governed_account_name");
                let proposal_name = config::get_str_val_raw(subcommand_matches, "proposal_name");
                let proposal_link = config::get_str_val_raw(subcommand_matches, "proposal_link");
                let proposal_index = config::get_integer_val(subcommand_matches, "proposal_index");
                println!(
                    "Done: {}",
                    client
                        .governance_proposal_new(
                            config.keypair.as_ref(),
                            &governed_account_name,
                            &proposal_name,
                            &proposal_link,
                            proposal_index as u32
                        )
                        .unwrap()
                );
            }
            ("proposal-cancel", Some(subcommand_matches)) => {
                let governed_account_name =
                    config::get_str_val_raw(subcommand_matches, "governed_account_name");
                let proposal_index = config::get_integer_val(subcommand_matches, "proposal_index");
                println!(
                    "Done: {}",
                    client
                        .governance_proposal_cancel(
                            config.keypair.as_ref(),
                            &governed_account_name,
                            proposal_index as u32
                        )
                        .unwrap()
                );
            }
            ("signatory-add", Some(subcommand_matches)) => {
                let governed_account_name =
                    config::get_str_val_raw(subcommand_matches, "governed_account_name");
                let proposal_index = config::get_integer_val(subcommand_matches, "proposal_index");
                let signatory =
                    Pubkey::from_str(&config::get_str_val_raw(subcommand_matches, "signatory"))
                        .unwrap();
                println!(
                    "Done: {}",
                    client
                        .governance_signatory_add(
                            config.keypair.as_ref(),
                            &governed_account_name,
                            proposal_index as u32,
                            &signatory
                        )
                        .unwrap()
                );
            }
            ("signatory-remove", Some(subcommand_matches)) => {
                let governed_account_name =
                    config::get_str_val_raw(subcommand_matches, "governed_account_name");
                let proposal_index = config::get_integer_val(subcommand_matches, "proposal_index");
                let signatory =
                    Pubkey::from_str(&config::get_str_val_raw(subcommand_matches, "signatory"))
                        .unwrap();
                println!(
                    "Done: {}",
                    client
                        .governance_signatory_remove(
                            config.keypair.as_ref(),
                            &governed_account_name,
                            proposal_index as u32,
                            &signatory
                        )
                        .unwrap()
                );
            }
            ("sign-off", Some(subcommand_matches)) => {
                let governed_account_name =
                    config::get_str_val_raw(subcommand_matches, "governed_account_name");
                let proposal_index = config::get_integer_val(subcommand_matches, "proposal_index");
                println!(
                    "Done: {}",
                    client
                        .governance_sign_off(
                            config.keypair.as_ref(),
                            &governed_account_name,
                            proposal_index as u32
                        )
                        .unwrap()
                );
            }
            ("vote-cast", Some(subcommand_matches)) => {
                let governed_account_name =
                    config::get_str_val_raw(subcommand_matches, "governed_account_name");
                let proposal_index = config::get_integer_val(subcommand_matches, "proposal_index");
                let vote = config::get_integer_val(subcommand_matches, "vote");
                println!(
                    "Done: {}",
                    client
                        .governance_vote_cast(
                            config.keypair.as_ref(),
                            &governed_account_name,
                            proposal_index as u32,
                            vote as u8
                        )
                        .unwrap()
                );
            }
            ("vote-relinquish", Some(subcommand_matches)) => {
                let governed_account_name =
                    config::get_str_val_raw(subcommand_matches, "governed_account_name");
                let proposal_index = config::get_integer_val(subcommand_matches, "proposal_index");
                println!(
                    "Done: {}",
                    client
                        .governance_vote_relinquish(
                            config.keypair.as_ref(),
                            &governed_account_name,
                            proposal_index as u32
                        )
                        .unwrap()
                );
            }
            ("vote-finalize", Some(subcommand_matches)) => {
                let governed_account_name =
                    config::get_str_val_raw(subcommand_matches, "governed_account_name");
                let proposal_index = config::get_integer_val(subcommand_matches, "proposal_index");
                println!(
                    "Done: {}",
                    client
                        .governance_vote_finalize(
                            config.keypair.as_ref(),
                            &governed_account_name,
                            proposal_index as u32
                        )
                        .unwrap()
                );
            }
            ("instruction-insert", Some(subcommand_matches)) => {
                let governed_account_name =
                    config::get_str_val_raw(subcommand_matches, "governed_account_name");
                let proposal_index = config::get_integer_val(subcommand_matches, "proposal_index");
                let instruction_index =
                    config::get_integer_val(subcommand_matches, "instruction_index");
                let instruction_str =
                    config::get_str_val_raw(subcommand_matches, "base64_instruction");
                let data = base64::decode(&instruction_str).unwrap();
                let instruction: Instruction = bincode::deserialize(data.as_slice()).unwrap();
                println!(
                    "Done: {}",
                    client
                        .governance_instruction_insert(
                            config.keypair.as_ref(),
                            &governed_account_name,
                            proposal_index as u32,
                            instruction_index as u16,
                            &instruction
                        )
                        .unwrap()
                );
            }
            ("instruction-remove", Some(subcommand_matches)) => {
                let governed_account_name =
                    config::get_str_val_raw(subcommand_matches, "governed_account_name");
                let proposal_index = config::get_integer_val(subcommand_matches, "proposal_index");
                let instruction_index =
                    config::get_integer_val(subcommand_matches, "instruction_index");
                println!(
                    "Done: {}",
                    client
                        .governance_instruction_remove(
                            config.keypair.as_ref(),
                            &governed_account_name,
                            proposal_index as u32,
                            instruction_index as u16,
                        )
                        .unwrap()
                );
            }
            ("instruction-execute", Some(subcommand_matches)) => {
                let governed_account_name =
                    config::get_str_val_raw(subcommand_matches, "governed_account_name");
                let proposal_index = config::get_integer_val(subcommand_matches, "proposal_index");
                let instruction_index =
                    config::get_integer_val(subcommand_matches, "instruction_index");
                println!(
                    "Done: {}",
                    client
                        .governance_instruction_execute(
                            config.keypair.as_ref(),
                            &governed_account_name,
                            proposal_index as u32,
                            instruction_index as u16,
                        )
                        .unwrap()
                );
            }
            ("instruction-flag-error", Some(subcommand_matches)) => {
                let governed_account_name =
                    config::get_str_val_raw(subcommand_matches, "governed_account_name");
                let proposal_index = config::get_integer_val(subcommand_matches, "proposal_index");
                let instruction_index =
                    config::get_integer_val(subcommand_matches, "instruction_index");
                println!(
                    "Done: {}",
                    client
                        .governance_instruction_flag_error(
                            config.keypair.as_ref(),
                            &governed_account_name,
                            proposal_index as u32,
                            instruction_index as u16,
                        )
                        .unwrap()
                );
            }
            _ => unreachable!(),
        },
        _ => error!("Unrecognized command. Use --help to list known commands."),
    };
}
