//! Solana Farms control interface.

mod config;
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
    log::error, num_enum::TryFromPrimitive, solana_farm_client::client::FarmClient,
    solana_farm_sdk::token::TokenSelector, solana_sdk::pubkey::Pubkey, std::str::FromStr,
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
                &config::get_filename(subcommand_matches),
                false,
            );
        }
        ("load-all", Some(subcommand_matches)) => {
            load::load(
                &client,
                &config,
                config::get_target(subcommand_matches),
                &config::get_filename(subcommand_matches),
                false,
            );
        }
        ("remove", Some(subcommand_matches)) => {
            remove::remove(
                &client,
                &config,
                config::get_target(subcommand_matches),
                &config::get_objectname(subcommand_matches),
            );
        }
        ("remove-ref", Some(subcommand_matches)) => {
            remove::remove_ref(
                &client,
                &config,
                config::get_target(subcommand_matches),
                &config::get_objectname(subcommand_matches),
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
                &config::get_filename(subcommand_matches),
                true,
            );
        }
        ("get", Some(subcommand_matches)) => {
            get::get(
                &client,
                &config,
                config::get_target(subcommand_matches),
                &config::get_objectname(subcommand_matches),
            );
        }
        ("get-ref", Some(subcommand_matches)) => {
            get::get_ref(
                &client,
                &config,
                config::get_target(subcommand_matches),
                &config::get_objectname(subcommand_matches),
            );
        }
        ("get-all", Some(subcommand_matches)) => {
            get::get_all(&client, &config, config::get_target(subcommand_matches));
        }
        ("list-all", Some(subcommand_matches)) => {
            get::list_all(&client, &config, config::get_target(subcommand_matches));
        }
        ("vault-init", Some(subcommand_matches)) => {
            vault::init(
                &client,
                &config,
                &config::get_vaultname(subcommand_matches),
                config::get_step(subcommand_matches),
            );
        }
        ("vault-shutdown", Some(subcommand_matches)) => {
            vault::shutdown(&client, &config, &config::get_vaultname(subcommand_matches));
        }
        ("vault-withdraw-fees", Some(subcommand_matches)) => {
            vault::withdraw_fees(
                &client,
                &config,
                &config::get_vaultname(subcommand_matches),
                TokenSelector::try_from_primitive(
                    subcommand_matches
                        .value_of("fee_token")
                        .unwrap()
                        .parse::<u8>()
                        .unwrap(),
                )
                .unwrap(),
                subcommand_matches
                    .value_of("amount")
                    .unwrap()
                    .parse::<f64>()
                    .unwrap(),
                &subcommand_matches
                    .value_of("receiver")
                    .unwrap()
                    .parse::<String>()
                    .unwrap(),
            );
        }
        ("vault-crank", Some(subcommand_matches)) => {
            vault::crank(
                &client,
                &config,
                &config::get_vaultname(subcommand_matches),
                config::get_step(subcommand_matches),
            );
        }
        ("vault-crank-all", Some(subcommand_matches)) => {
            vault::crank_all(&client, &config, config::get_step(subcommand_matches));
        }
        ("vault-set-fee", Some(subcommand_matches)) => {
            vault::set_fee(
                &client,
                &config,
                &config::get_vaultname(subcommand_matches),
                config::get_vaultparam(subcommand_matches) as f32,
            );
        }
        ("vault-set-external-fee", Some(subcommand_matches)) => {
            vault::set_external_fee(
                &client,
                &config,
                &config::get_vaultname(subcommand_matches),
                config::get_vaultparam(subcommand_matches) as f32,
            );
        }
        ("vault-set-min-crank-interval", Some(subcommand_matches)) => {
            vault::set_min_crank_interval(
                &client,
                &config,
                &config::get_vaultname(subcommand_matches),
                config::get_vaultparam(subcommand_matches) as u32,
            );
        }
        ("vault-disable-deposit", Some(subcommand_matches)) => {
            vault::disable_deposit(&client, &config, &config::get_vaultname(subcommand_matches));
        }
        ("vault-enable-deposit", Some(subcommand_matches)) => {
            vault::enable_deposit(&client, &config, &config::get_vaultname(subcommand_matches));
        }
        ("vault-disable-withdrawal", Some(subcommand_matches)) => {
            vault::disable_withdrawal(&client, &config, &config::get_vaultname(subcommand_matches));
        }
        ("vault-enable-withdrawal", Some(subcommand_matches)) => {
            vault::enable_withdrawal(&client, &config, &config::get_vaultname(subcommand_matches));
        }
        ("vault-get-info", Some(subcommand_matches)) => {
            vault::get_info(&client, &config, &config::get_vaultname(subcommand_matches));
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
                &config::get_objectname(subcommand_matches),
                &subcommand_matches
                    .value_of("param1")
                    .unwrap()
                    .parse::<String>()
                    .unwrap(),
                &subcommand_matches
                    .value_of("param2")
                    .unwrap()
                    .parse::<String>()
                    .unwrap(),
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
                    subcommand_matches
                        .value_of("mint-ui-amount")
                        .unwrap()
                        .parse()
                        .unwrap(),
                );
            }
            _ => unreachable!(),
        },
        _ => error!("Unrecognized command. Use --help to list known commands."),
    };
}
