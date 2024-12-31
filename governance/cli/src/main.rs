#![allow(clippy::arithmetic_side_effects)]
use {
    clap::{
        crate_description, crate_name, crate_version, value_t_or_exit, App, AppSettings, Arg,
        SubCommand,
    },
    solana_clap_utils::{
        input_parsers::{keypair_of, pubkey_of},
        input_validators::{is_keypair, is_url, is_valid_percentage, is_valid_pubkey},
    },
    solana_client::rpc_client::RpcClient,
    solana_sdk::{
        clock::UnixTimestamp,
        commitment_config::CommitmentConfig,
        program_pack::Pack,
        pubkey::Pubkey,
        signature::{read_keypair_file, Keypair, Signer},
        transaction::Transaction,
    },
    spl_governance::state::{
        governance::get_governance_address, native_treasury::get_native_treasury_address,
    },
    std::{
        collections::HashMap,
        fs::File,
        io::Write,
        time::{Duration, SystemTime, UNIX_EPOCH},
    },
};

struct Config {
    keypair: Keypair,
    json_rpc_url: String,
    verbose: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app_matches = App::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg({
            let arg = Arg::with_name("config_file")
                .short("C")
                .long("config")
                .value_name("PATH")
                .takes_value(true)
                .global(true)
                .help("Configuration file to use");
            if let Some(ref config_file) = *solana_cli_config::CONFIG_FILE {
                arg.default_value(config_file)
            } else {
                arg
            }
        })
        .arg(
            Arg::with_name("keypair")
                .long("keypair")
                .value_name("KEYPAIR")
                .validator(is_keypair)
                .takes_value(true)
                .global(true)
                .help("Filepath or URL to a keypair [default: client keypair]"),
        )
        .arg(
            Arg::with_name("verbose")
                .long("verbose")
                .short("v")
                .takes_value(false)
                .global(true)
                .help("Show additional information"),
        )
        .arg(
            Arg::with_name("json_rpc_url")
                .long("url")
                .value_name("URL")
                .takes_value(true)
                .global(true)
                .validator(is_url)
                .help("JSON RPC URL for the cluster [default: value from configuration file]"),
        )
        .arg(
            Arg::with_name("program_id")
                .long("program")
                .value_name("PROGRAM_ID")
                .takes_value(true)
                .global(true)
                .validator(is_valid_pubkey)
                .default_value("GovER5Lthms3bLBqWub97yVrMmEogzX7xNjdXpPPCVZw")
                .help("SPL Governance Program ID"),
        )
        .subcommand(
            SubCommand::with_name("grind-native-treasury")
                .about("Grind a native treasury with a given prefix")
                .arg(
                    Arg::with_name("realm")
                        .value_name("REALM_ADDRESS")
                        .validator(is_valid_pubkey)
                        .index(1)
                        .required(true)
                        .help(
                            "The address of the realm the native treasury will be associated with",
                        ),
                )
                .arg(
                    Arg::with_name("prefix")
                        .value_name("PREFIX")
                        .required(true)
                        .help("Prefix of the native treasury address"),
                )
                .arg(
                    Arg::with_name("ignore-case")
                        .short("i")
                        .takes_value(false)
                        .help("Match prefix case insensitive"),
                ),
        )
        .get_matches();

    let (sub_command, sub_matches) = app_matches.subcommand();
    let matches = sub_matches.unwrap();

    let config = {
        let cli_config = if let Some(config_file) = matches.value_of("config_file") {
            solana_cli_config::Config::load(config_file).unwrap_or_default()
        } else {
            solana_cli_config::Config::default()
        };

        Config {
            json_rpc_url: matches
                .value_of("json_rpc_url")
                .unwrap_or(&cli_config.json_rpc_url)
                .to_string(),
            keypair: read_keypair_file(
                matches
                    .value_of("keypair")
                    .unwrap_or(&cli_config.keypair_path),
            )?,
            verbose: matches.is_present("verbose"),
        }
    };
    solana_logger::setup_with_default("solana=info");
    let rpc_client =
        RpcClient::new_with_commitment(config.json_rpc_url.clone(), CommitmentConfig::confirmed());

    match (sub_command, sub_matches) {
        ("grind-native-treasury", Some(arg_matches)) => {
            let program_id = pubkey_of(arg_matches, "program_id").unwrap();
            let realm_address = pubkey_of(arg_matches, "realm").unwrap();

            println!("Realm Address: {}", realm_address);

            let prefix = matches.value_of("prefix").unwrap();
            let ignore_case = matches.is_present("ignore-case");
            let prefix = if ignore_case {
                prefix.to_lowercase()
            } else {
                prefix.to_string()
            };
            let prefix_str = prefix.as_str();

            loop {
                let governance_seed = Keypair::new().pubkey();
                let governance_address =
                    get_governance_address(&program_id, &realm_address, &governance_seed);
                let native_treasury_address =
                    get_native_treasury_address(&program_id, &governance_address);
                let base58 = if ignore_case {
                    native_treasury_address.to_string().to_lowercase()
                } else {
                    native_treasury_address.to_string()
                };

                if base58.starts_with(prefix_str) {
                    println!("Governance Seed: {}", governance_seed);
                    println!("Governance Address: {}", governance_address);
                    println!("Native Treasury: {}", native_treasury_address);

                    break;
                }
            }

            Ok(())
        }
        _ => unreachable!(),
    }
}
