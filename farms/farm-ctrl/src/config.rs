//! Configuration and command line arguments management.

use {
    clap::{crate_description, crate_name, App, AppSettings, Arg, ArgMatches, SubCommand},
    solana_clap_utils::{input_validators::is_url, keypair::signer_from_path},
    solana_farm_sdk::refdb,
    solana_sdk::{commitment_config::CommitmentConfig, signature::Signer},
    std::str::FromStr,
};

#[derive(Debug)]
pub struct Config {
    pub farm_client_url: String,
    pub commitment: CommitmentConfig,
    pub keypair: Box<dyn Signer>,
    pub max_instructions: u32,
    pub no_pretty_print: bool,
    pub skip_existing: bool,
}

impl Config {
    pub fn new(matches: &ArgMatches) -> Self {
        let cli_config = if let Some(config_file) = matches.value_of("config_file") {
            match solana_cli_config::Config::load(config_file) {
                Err(e) => {
                    panic!(
                        "Failed to load config file \"{}\":{}",
                        config_file,
                        e.to_string()
                    );
                }
                Ok(config) => config,
            }
        } else {
            solana_cli_config::Config::default()
        };

        let farm_client_url = matches
            .value_of("farm_client_url")
            .unwrap_or(&cli_config.json_rpc_url);
        let keypair_path = matches
            .value_of("keypair")
            .unwrap_or(&cli_config.keypair_path);
        let commitment = matches
            .value_of("commitment")
            .unwrap_or(&cli_config.commitment);
        let max_instructions = matches
            .value_of("max_instructions")
            .unwrap()
            .parse()
            .unwrap();

        Self {
            farm_client_url: farm_client_url.to_string(),
            commitment: CommitmentConfig::from_str(commitment).unwrap(),
            keypair: signer_from_path(matches, keypair_path, "signer", &mut None).unwrap(),
            max_instructions,
            no_pretty_print: matches.is_present("no_pretty_print"),
            skip_existing: matches.is_present("skip_existing"),
        }
    }
}

pub fn get_target(matches: &ArgMatches) -> refdb::StorageType {
    let target = matches.value_of("target").unwrap();
    let res = target
        .parse()
        .unwrap_or_else(|_| panic!("Invalid target type \"{}\"", target));
    if res == refdb::StorageType::Other {
        panic!("Invalid target type: {}", res);
    }
    res
}

pub fn get_objectname(matches: &ArgMatches) -> String {
    matches.value_of("objectname").unwrap().parse().unwrap()
}

pub fn get_vaultname(matches: &ArgMatches) -> String {
    matches
        .value_of("vaultname")
        .unwrap()
        .parse::<String>()
        .unwrap()
        .to_uppercase()
}

pub fn get_vaultparam(matches: &ArgMatches) -> f64 {
    matches.value_of("vaultparam").unwrap().parse().unwrap()
}

pub fn get_step(matches: &ArgMatches) -> u64 {
    matches.value_of("step").unwrap().parse().unwrap()
}

pub fn get_filename(matches: &ArgMatches) -> String {
    matches.value_of("filename").unwrap().parse().unwrap()
}

pub fn get_clap_app<'a, 'b>(version: &'b str) -> App<'a, 'b> {
    let target = Arg::with_name("target")
        .value_name("TARGET_TYPE")
        .required(true)
        .takes_value(true)
        .help("Target object type (program, vault, etc.)");

    let filename = Arg::with_name("filename")
        .value_name("FILE_NAME")
        .required(true)
        .takes_value(true)
        .help("Input file name");

    let objectname = Arg::with_name("objectname")
        .value_name("OBJECT_NAME")
        .required(true)
        .takes_value(true)
        .help("Target object name");

    let vaultname = Arg::with_name("vaultname")
        .value_name("VAULT_NAME")
        .required(true)
        .takes_value(true)
        .help("Vault name");

    let vaultparam = Arg::with_name("vaultparam")
        .value_name("VAULT_PARAM")
        .required(true)
        .takes_value(true)
        .help("Vault param");

    let step = Arg::with_name("step")
        .value_name("STEP")
        .required(true)
        .takes_value(true)
        .validator(|p| match p.parse::<u64>() {
            Err(_) => Err(String::from("Must be unsigned integer")),
            Ok(_) => Ok(()),
        })
        .help("Instruction step");

    App::new(crate_name!())
        .about(crate_description!())
        .version(version)
        .arg(
            Arg::with_name("log_level")
                .short("L")
                .long("log-level")
                .takes_value(true)
                .default_value("info")
                .global(true)
                .help("Log verbosity level (debug, info, warning, error)")
                .validator(|p| {
                    let allowed = ["debug", "info", "warning", "error"];
                    if allowed.contains(&p.as_str()) {
                        Ok(())
                    } else {
                        Err(String::from("Must be one of: debug, info, warning, error"))
                    }
                }),
        )
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
            Arg::with_name("farm_client_url")
                .short("f")
                .long("farm-client-url")
                .value_name("STR")
                .takes_value(true)
                .global(true)
                .validator(is_url)
                .help("RPC URL to use with Farm Client"),
        )
        .arg(
            Arg::with_name("keypair")
                .short("k")
                .long("keypair")
                .value_name("KEYPAIR")
                .global(true)
                .takes_value(true)
                .help("Filepath or URL to a keypair"),
        )
        .arg(
            Arg::with_name("max_instructions")
                .short("m")
                .long("max-instructions")
                .value_name("NUM")
                .global(true)
                .takes_value(true)
                .default_value("1")
                .validator(|p| match p.parse::<u32>() {
                    Err(_) => Err(String::from("Must be unsigned integer")),
                    Ok(_) => Ok(()),
                })
                .help("Max instructions per transaction"),
        )
        .arg(
            Arg::with_name("commitment")
                .long("commitment")
                .short("c")
                .takes_value(true)
                .possible_values(&[
                    "processed",
                    "confirmed",
                    "finalized",
                ])
                .value_name("COMMITMENT_LEVEL")
                .hide_possible_values(true)
                .global(true)
                .help("Return information at the selected commitment level [possible values: processed, confirmed, finalized]"),
        )
        .arg(
            Arg::with_name("no_pretty_print")
                .short("n")
                .long("no-pretty-print")
                .global(true)
                .takes_value(false)
                .help("Print entire record in one line"),
        )
        .arg(
            Arg::with_name("skip_existing")
                .short("s")
                .long("skip-existing")
                .global(true)
                .takes_value(false)
                .help("Do not update existing records on-chain"),
        )
        .subcommand(
            SubCommand::with_name("init")
                .about("Initialize Reference DB on-chain")
                .arg(target.clone()),
        )
        .subcommand(
            SubCommand::with_name("init-all")
                .about("Initialize Reference DB of all storage types on-chain"),
        )
        .subcommand(
            SubCommand::with_name("drop")
                .about("Drop on-chain Reference DB")
                .arg(target.clone()),
        )
        .subcommand(
            SubCommand::with_name("drop-all")
                .about("Drop on-chain Reference DB for all storage types"),
        )
        .subcommand(
            SubCommand::with_name("load")
                .about("Load objects from file and send to blockchain")
                .arg(target.clone())
                .arg(filename.clone()),
        )
        .subcommand(
            SubCommand::with_name("load-all")
                .about("Same as \"load\"")
                .arg(target.clone())
                .arg(filename.clone()),
        )
        .subcommand(
            SubCommand::with_name("remove")
                .about("Remove specified object from blockchain")
                .arg(target.clone())
                .arg(objectname.clone()),
        )
        .subcommand(
            SubCommand::with_name("remove-ref")
                .about("Remove specified reference from blockchain")
                .arg(target.clone())
                .arg(objectname.clone()),
        )
        .subcommand(
            SubCommand::with_name("remove-all")
                .about("Remove all objects of the given type from blockchain")
                .arg(target.clone()),
        )
        .subcommand(
            SubCommand::with_name("remove-all-with-file")
                .about("Remove all objects in the file from blockchain")
                .arg(target.clone())
                .arg(filename.clone()),
        )
        .subcommand(
            SubCommand::with_name("get")
                .about("Query specified object in blockchain and print")
                .arg(target.clone())
                .arg(objectname.clone()),
        )
        .subcommand(
            SubCommand::with_name("get-ref")
                .about("Query specified object by reference address and print")
                .arg(target.clone())
                .arg(objectname.clone()),
        )
        .subcommand(
            SubCommand::with_name("get-all")
                .about("Query all objects of the given type and print")
                .arg(target.clone()),
        )
        .subcommand(
            SubCommand::with_name("list-all")
                .about("Query all objects of the given type and print")
                .arg(target.clone()),
        )
        .subcommand(
            SubCommand::with_name("vault-init")
                .about("Initialize the Vault")
                .arg(vaultname.clone())
                .arg(step.clone())
        )
        .subcommand(
            SubCommand::with_name("vault-shutdown")
                .about("Shutdown the Vault")
                .arg(vaultname.clone()),
        )
        .subcommand(
            SubCommand::with_name("vault-withdraw-fees")
                .about("Withdraw collected fees from the Vault")
                .arg(vaultname.clone())
                .arg(
                    Arg::with_name("fee_token")
                    .value_name("FEE_TOKEN")
                    .required(true)
                    .takes_value(true)
                    .help("Fees token account to withdraw from - 0 or 1"),
                )
                .arg(
                    Arg::with_name("amount")
                    .value_name("AMOUNT")
                    .required(true)
                    .takes_value(true)
                    .validator(|p| match p.parse::<f64>() {
                        Err(_) => Err(String::from("Must be unsigned decimal")),
                        Ok(val) => {
                            if val >= 0.0 {
                                Ok(())
                            } else {
                                Err(String::from("Must be unsigned decimal"))
                            }
                        }
                    })
                    .help("Fees amount or zero for all"),
                )
                .arg(
                    Arg::with_name("receiver")
                        .value_name("PUBKEY")
                        .required(true)
                        .takes_value(true)
                        .help("Fees receiver"),
                )
        )
        .subcommand(
            SubCommand::with_name("vault-crank")
                .about("Crank the Vault")
                .arg(vaultname.clone())
                .arg(step.clone())
        )
        .subcommand(
            SubCommand::with_name("vault-crank-all")
                .about("Crank all Vaults")
                .arg(step.clone())
        )
        .subcommand(
            SubCommand::with_name("vault-set-fee")
                .about("Set new fee percent for the Vault")
                .arg(vaultname.clone())
                .arg(vaultparam.clone()),
        )
        .subcommand(
            SubCommand::with_name("vault-set-external-fee")
                .about("Set new external fee percent for the Vault")
                .arg(vaultname.clone())
                .arg(vaultparam.clone()),
        )
        .subcommand(
            SubCommand::with_name("vault-set-min-crank-interval")
                .about("Set new min crank interval in seconds for the Vault")
                .arg(vaultname.clone())
                .arg(vaultparam.clone()),
        )
        .subcommand(
            SubCommand::with_name("vault-disable-deposit")
                .about("Disable deposits for the specified object")
                .arg(vaultname.clone()),
        )
        .subcommand(
            SubCommand::with_name("vault-enable-deposit")
                .about("Enable deposits for the specified object")
                .arg(vaultname.clone()),
        )
        .subcommand(
            SubCommand::with_name("vault-disable-withdrawal")
                .about("Disable withdrawals for the specified object")
                .arg(vaultname.clone()),
        )
        .subcommand(
            SubCommand::with_name("vault-enable-withdrawal")
                .about("Enable withdrawals for the specified object")
                .arg(vaultname.clone()),
        )
        .subcommand(
            SubCommand::with_name("vault-get-info")
                .about("Print current stats for the Vault")
                .arg(vaultname.clone()),
        )
        .subcommand(
            SubCommand::with_name("print-pda-all")
                .about("Derive Reference DB addresses for all objects"),
        )
        .subcommand(
            SubCommand::with_name("print-size")
                .about("Print Reference DB and specified object sizes")
                .arg(target.clone()),
        )
        .subcommand(
            SubCommand::with_name("print-size-all")
                .about("Print Reference DB and all object sizes"),
        )
        .subcommand(
            SubCommand::with_name("generate")
                .about("Generate json boilerplate for the specified object")
                .arg(target.clone())
                .arg(objectname.clone())
                .arg(
                    Arg::with_name("param1")
                        .index(3)
                        .value_name("PARAM1")
                        .required(true)
                        .takes_value(true)
                        .help("Object specific parameter 1"),
                )
                .arg(
                    Arg::with_name("param2")
                        .index(4)
                        .value_name("PARAM2")
                        .required(true)
                        .takes_value(true)
                        .help("Object specific parameter 2"),
                ),
        )
        .subcommand(
            SubCommand::with_name("governance")
                .about("Governance commands. See `solana-farm-ctrl governance help`")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .subcommand(
                    SubCommand::with_name("init")
                    .about("Initialize a new DAO")
                    .arg(
                        Arg::with_name("governance-program-address")
                            .value_name("DAO-PROGRAM")
                            .required(true)
                            .takes_value(true)
                            .help("Address of the governance program"),
                    )
                    .arg(
                        Arg::with_name("mint-ui-amount")
                        .value_name("MINT_UI_AMOUNT")
                        .required(true)
                        .takes_value(true)
                        .validator(|p| match p.parse::<f64>() {
                            Err(_) => Err(String::from("Must be unsigned integer")),
                            Ok(_) => Ok(()),
                        })
                        .help("Amount of governance tokens to mint")
                    )
                )
        )
}
