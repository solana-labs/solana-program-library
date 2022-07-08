//! Configuration and command line arguments management.

use {
    clap::{crate_description, crate_name, App, AppSettings, Arg, ArgMatches, SubCommand},
    solana_clap_utils::{input_validators::is_url, keypair::signer_from_path},
    solana_farm_sdk::{program::multisig::Multisig, refdb},
    solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signer},
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
                    panic!("Failed to load config file \"{}\":{}", config_file, e);
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

pub fn get_str_val<'a>(matches: &ArgMatches<'a>, argname: &str) -> String {
    matches
        .value_of(argname)
        .unwrap()
        .parse::<String>()
        .unwrap()
        .to_uppercase()
}

pub fn get_str_val_raw<'a>(matches: &ArgMatches<'a>, argname: &str) -> String {
    matches
        .value_of(argname)
        .unwrap()
        .parse::<String>()
        .unwrap()
}

pub fn get_pubkey_val<'a>(matches: &ArgMatches<'a>, argname: &str) -> Pubkey {
    Pubkey::from_str(matches.value_of(argname).unwrap()).unwrap()
}

pub fn get_pubkey_multi_val<'a>(matches: &ArgMatches<'a>, argname: &str) -> Vec<Pubkey> {
    let args: Vec<_> = matches.values_of(argname).unwrap().collect();
    let mut keys = vec![];
    for arg in &args {
        keys.push(Pubkey::from_str(arg).unwrap());
    }
    keys
}

pub fn get_integer_val<'a>(matches: &ArgMatches<'a>, argname: &str) -> u64 {
    matches.value_of(argname).unwrap().parse::<u64>().unwrap()
}

pub fn get_floating_val<'a>(matches: &ArgMatches<'a>, argname: &str) -> f64 {
    matches.value_of(argname).unwrap().parse::<f64>().unwrap()
}

pub fn get_boolean_val<'a>(matches: &ArgMatches<'a>, argname: &str) -> bool {
    matches.value_of(argname).unwrap().parse::<bool>().unwrap()
}

fn get_arg(name: &str) -> Arg {
    Arg::with_name(name).required(true).takes_value(true)
}

fn get_multi_arg(name: &str, min_values: u64, max_values: u64) -> Arg {
    Arg::with_name(name)
        .required(true)
        .takes_value(true)
        .multiple(true)
        .min_values(min_values)
        .max_values(max_values)
}

fn get_integer_arg(name: &str) -> Arg {
    Arg::with_name(name)
        .takes_value(true)
        .required(true)
        .validator(|p| match p.parse::<u64>() {
            Err(_) => Err(String::from("Must be unsigned integer")),
            Ok(_) => Ok(()),
        })
}

fn get_floating_arg(name: &str) -> Arg {
    Arg::with_name(name)
        .takes_value(true)
        .required(true)
        .validator(|p| match p.parse::<f64>() {
            Err(_) => Err(String::from("Must be floating number")),
            Ok(_) => Ok(()),
        })
}

fn get_boolean_arg(name: &str) -> Arg {
    Arg::with_name(name)
        .takes_value(true)
        .required(true)
        .validator(|p| match p.parse::<bool>() {
            Err(_) => Err(String::from("Must be boolean")),
            Ok(_) => Ok(()),
        })
}

pub fn get_clap_app<'a, 'b>(version: &'b str) -> App<'a, 'b> {
    let target = Arg::with_name("target")
        .required(true)
        .takes_value(true)
        .help("Target object type (program, vault, etc.)");

    let filename = Arg::with_name("file_name")
        .required(true)
        .takes_value(true)
        .help("Input file name");

    let objectname = Arg::with_name("object_name")
        .required(true)
        .takes_value(true)
        .help("Target object name");

    let tokenname = Arg::with_name("token_name")
        .required(true)
        .takes_value(true)
        .help("Token name");

    let vaultname = Arg::with_name("vault_name")
        .required(true)
        .takes_value(true)
        .help("Vault name");

    let fundname = Arg::with_name("fund_name")
        .required(true)
        .takes_value(true)
        .help("Fund name");

    let amount = Arg::with_name("amount")
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
        .help("Token amount");

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
                .help("Log verbosity level")
                .possible_values(&["debug", "info", "warning", "error"])
                .hide_possible_values(false),
        )
        .arg({
            let arg = Arg::with_name("config_file")
                .short("C")
                .long("config")
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
                .takes_value(true)
                .global(true)
                .validator(is_url)
                .help("RPC URL to use with Farm Client"),
        )
        .arg(
            Arg::with_name("keypair")
                .short("k")
                .long("keypair")
                .global(true)
                .takes_value(true)
                .help("Filepath or URL to a keypair"),
        )
        .arg(
            Arg::with_name("max_instructions")
                .short("m")
                .long("max-instructions")
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
                .possible_values(&["processed", "confirmed", "finalized"])
                .hide_possible_values(false)
                .global(true)
                .help("Return information at the selected commitment level"),
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
            SubCommand::with_name("get-admins")
                .about("Print current admin signers for the Main Router"),
        )
        .subcommand(
            SubCommand::with_name("set-admins")
                .about("Set new admins for the Main Router")
                .arg(get_integer_arg("min_signatures"))
                .arg(get_multi_arg(
                    "admin_signers",
                    1,
                    Multisig::MAX_SIGNERS as u64,
                )),
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
            SubCommand::with_name("program-get-admins")
                .about("Print current admin signers for the program")
                .arg(get_arg("program_id")),
        )
        .subcommand(
            SubCommand::with_name("program-set-admins")
                .about("Set new admin signers for the program")
                .arg(get_arg("program_id"))
                .arg(get_integer_arg("min_signatures"))
                .arg(get_multi_arg(
                    "admin_signers",
                    1,
                    Multisig::MAX_SIGNERS as u64,
                )),
        )
        .subcommand(
            SubCommand::with_name("program-set-single-authority")
                .about("Set single upgrade authority for the program")
                .arg(get_arg("program_id"))
                .arg(get_arg("upgrade_authority")),
        )
        .subcommand(
            SubCommand::with_name("program-upgrade")
                .about("Upgrade the program from the data buffer")
                .arg(get_arg("program_id"))
                .arg(get_arg("buffer_address")),
        )
        .subcommand(
            SubCommand::with_name("vault-init")
                .about("Initialize the Vault")
                .arg(vaultname.clone())
                .arg(get_integer_arg("step")),
        )
        .subcommand(
            SubCommand::with_name("vault-set-admins")
                .about("Set new admins for the Vault")
                .arg(vaultname.clone())
                .arg(get_integer_arg("min_signatures"))
                .arg(get_multi_arg(
                    "admin_signers",
                    1,
                    Multisig::MAX_SIGNERS as u64,
                )),
        )
        .subcommand(
            SubCommand::with_name("vault-get-admins")
                .about("Print current admin signers for the Vault")
                .arg(vaultname.clone()),
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
                        .required(true)
                        .takes_value(true)
                        .help("Fees token account to withdraw from - 0 or 1"),
                )
                .arg(amount.clone())
                .arg(
                    Arg::with_name("receiver")
                        .required(true)
                        .takes_value(true)
                        .help("Fees receiver address"),
                ),
        )
        .subcommand(
            SubCommand::with_name("vault-crank")
                .about("Crank the Vault")
                .arg(vaultname.clone())
                .arg(get_integer_arg("step")),
        )
        .subcommand(
            SubCommand::with_name("vault-set-fee")
                .about("Set new fee percent for the Vault")
                .arg(vaultname.clone())
                .arg(get_floating_arg("fee_percent")),
        )
        .subcommand(
            SubCommand::with_name("vault-set-external-fee")
                .about("Set new external fee percent for the Vault")
                .arg(vaultname.clone())
                .arg(get_floating_arg("external_fee_percent")),
        )
        .subcommand(
            SubCommand::with_name("vault-set-min-crank-interval")
                .about("Set new min crank interval in seconds for the Vault")
                .arg(vaultname.clone())
                .arg(get_integer_arg("min_crank_interval")),
        )
        .subcommand(
            SubCommand::with_name("vault-disable-deposits")
                .about("Disable deposits for the specified object")
                .arg(vaultname.clone()),
        )
        .subcommand(
            SubCommand::with_name("vault-enable-deposits")
                .about("Enable deposits for the specified object")
                .arg(vaultname.clone()),
        )
        .subcommand(
            SubCommand::with_name("vault-disable-withdrawals")
                .about("Disable withdrawals for the specified object")
                .arg(vaultname.clone()),
        )
        .subcommand(
            SubCommand::with_name("vault-enable-withdrawals")
                .about("Enable withdrawals for the specified object")
                .arg(vaultname.clone()),
        )
        .subcommand(
            SubCommand::with_name("vault-get-info")
                .about("Print current stats for the Vault")
                .arg(vaultname.clone()),
        )
        .subcommand(
            SubCommand::with_name("fund-init")
                .about("Initialize the Fund")
                .arg(fundname.clone())
                .arg(get_integer_arg("step")),
        )
        .subcommand(
            SubCommand::with_name("fund-set-admins")
                .about("Set new admins for the Fund")
                .arg(fundname.clone())
                .arg(get_integer_arg("min_signatures"))
                .arg(get_multi_arg(
                    "admin_signers",
                    1,
                    Multisig::MAX_SIGNERS as u64,
                )),
        )
        .subcommand(
            SubCommand::with_name("fund-get-admins")
                .about("Print current admin signers for the Fund")
                .arg(fundname.clone()),
        )
        .subcommand(
            SubCommand::with_name("fund-set-manager")
                .about("Set a new manager for the Fund")
                .arg(fundname.clone())
                .arg(get_arg("manager")),
        )
        .subcommand(
            SubCommand::with_name("fund-add-custody")
                .about("Add a new custody to the Fund")
                .arg(fundname.clone())
                .arg(tokenname.clone())
                .arg(get_arg("custody_type")),
        )
        .subcommand(
            SubCommand::with_name("fund-remove-custody")
                .about("Remove the custody from the Fund")
                .arg(fundname.clone())
                .arg(tokenname.clone())
                .arg(get_arg("custody_type")),
        )
        .subcommand(
            SubCommand::with_name("fund-add-vault")
                .about("Add a new Vault to the Fund")
                .arg(fundname.clone())
                .arg(vaultname.clone())
                .arg(get_arg("vault_type")),
        )
        .subcommand(
            SubCommand::with_name("fund-remove-vault")
                .about("Remove the Vault from the Fund")
                .arg(fundname.clone())
                .arg(vaultname.clone())
                .arg(get_arg("vault_type")),
        )
        .subcommand(
            SubCommand::with_name("fund-set-assets-tracking-config")
                .about("Set a new assets tracking config for the Fund")
                .arg(fundname.clone())
                .arg(get_floating_arg("assets_limit_usd"))
                .arg(get_integer_arg("max_update_age_sec"))
                .arg(get_floating_arg("max_price_error"))
                .arg(get_integer_arg("max_price_age_sec"))
                .arg(get_boolean_arg("issue_virtual_tokens")),
        )
        .subcommand(
            SubCommand::with_name("fund-set-deposit-schedule")
                .about("Set a new deposit schedule for the Fund")
                .arg(fundname.clone())
                .arg(get_integer_arg("start_time"))
                .arg(get_integer_arg("end_time"))
                .arg(get_arg("approval_required"))
                .arg(get_floating_arg("limit_usd"))
                .arg(get_floating_arg("fee")),
        )
        .subcommand(
            SubCommand::with_name("fund-disable-deposits")
                .about("Disables deposits to the Fund")
                .arg(fundname.clone()),
        )
        .subcommand(
            SubCommand::with_name("fund-approve-deposit")
                .about("Approve pending deposit to the Fund")
                .arg(fundname.clone())
                .arg(get_arg("user_address"))
                .arg(tokenname.clone())
                .arg(amount.clone()),
        )
        .subcommand(
            SubCommand::with_name("fund-deny-deposit")
                .about("Deny pending deposit to the Fund")
                .arg(fundname.clone())
                .arg(get_arg("user_address"))
                .arg(tokenname.clone())
                .arg(get_arg("deny_reason")),
        )
        .subcommand(
            SubCommand::with_name("fund-set-withdrawal-schedule")
                .about("Set a new withdrawal schedule for the Fund")
                .arg(fundname.clone())
                .arg(get_integer_arg("start_time"))
                .arg(get_integer_arg("end_time"))
                .arg(get_arg("approval_required"))
                .arg(get_floating_arg("limit_usd"))
                .arg(get_floating_arg("fee")),
        )
        .subcommand(
            SubCommand::with_name("fund-disable-withdrawals")
                .about("Disables withdrawals from the Fund")
                .arg(fundname.clone()),
        )
        .subcommand(
            SubCommand::with_name("fund-approve-withdrawal")
                .about("Approve pending withdrawal from the Fund")
                .arg(fundname.clone())
                .arg(get_arg("user_address"))
                .arg(tokenname.clone())
                .arg(amount.clone()),
        )
        .subcommand(
            SubCommand::with_name("fund-deny-withdrawal")
                .about("Deny pending withdrawal from the Fund")
                .arg(fundname.clone())
                .arg(get_arg("user_address"))
                .arg(tokenname.clone())
                .arg(get_arg("deny_reason")),
        )
        .subcommand(
            SubCommand::with_name("fund-lock-assets")
                .about("Moves assets from Deposit/Withdraw custody to the Fund")
                .arg(fundname.clone())
                .arg(tokenname.clone())
                .arg(amount.clone()),
        )
        .subcommand(
            SubCommand::with_name("fund-unlock-assets")
                .about("Releases assets from the Fund to Deposit/Withdraw custody")
                .arg(fundname.clone())
                .arg(tokenname.clone())
                .arg(amount.clone()),
        )
        .subcommand(
            SubCommand::with_name("fund-withdraw-fees")
                .about("Withdraw collected fees from the Fund")
                .arg(fundname.clone())
                .arg(tokenname.clone())
                .arg(get_arg("custody_type"))
                .arg(amount.clone())
                .arg(get_arg("receiver")),
        )
        .subcommand(
            SubCommand::with_name("fund-update-assets-with-custody")
                .about("Update Fund assets info based on custody holdings")
                .arg(fundname.clone())
                .arg(get_integer_arg("custody_id")),
        )
        .subcommand(
            SubCommand::with_name("fund-update-assets-with-custodies")
                .about("Update Fund assets info based on all custodies")
                .arg(fundname.clone()),
        )
        .subcommand(
            SubCommand::with_name("fund-update-assets-with-vault")
                .about("Update Fund assets info based on Vault holdings")
                .arg(fundname.clone())
                .arg(get_integer_arg("vault_id")),
        )
        .subcommand(
            SubCommand::with_name("fund-update-assets-with-vaults")
                .about("Update Fund assets info based on all Vaults")
                .arg(fundname.clone()),
        )
        .subcommand(
            SubCommand::with_name("fund-stop-liquidation")
                .about("Stop the Fund liquidation")
                .arg(fundname.clone()),
        )
        .subcommand(
            SubCommand::with_name("fund-get-info")
                .about("Print current stats for the Fund")
                .arg(fundname.clone()),
        )
        .subcommand(
            SubCommand::with_name("fund-deposit-pool")
                .about("Add liquidity to the Pool in the Fund")
                .arg(fundname.clone())
                .arg(get_arg("pool_name"))
                .arg(get_floating_arg("max_token_a_ui_amount"))
                .arg(get_floating_arg("max_token_b_ui_amount")),
        )
        .subcommand(
            SubCommand::with_name("fund-withdraw-pool")
                .about("Remove liquidity from the Pool in the Fund")
                .arg(fundname.clone())
                .arg(get_arg("pool_name"))
                .arg(amount.clone()),
        )
        .subcommand(
            SubCommand::with_name("fund-swap")
                .about("Swap tokens in the Fund")
                .arg(fundname.clone())
                .arg(get_arg("protocol"))
                .arg(get_arg("from_token"))
                .arg(get_arg("to_token"))
                .arg(get_floating_arg("amount_in"))
                .arg(get_floating_arg("min_amount_out")),
        )
        .subcommand(
            SubCommand::with_name("fund-stake")
                .about("Stake LP tokens to the Farm in the Fund")
                .arg(fundname.clone())
                .arg(get_arg("farm_name"))
                .arg(amount.clone()),
        )
        .subcommand(
            SubCommand::with_name("fund-unstake")
                .about("Unstake LP tokens from the Farm in the Fund")
                .arg(fundname.clone())
                .arg(get_arg("farm_name"))
                .arg(amount.clone()),
        )
        .subcommand(
            SubCommand::with_name("fund-harvest")
                .about("Harvest rewards from the Farm in the Fund")
                .arg(fundname.clone())
                .arg(get_arg("farm_name")),
        )
        .subcommand(
            SubCommand::with_name("fund-deposit-vault")
                .about("Add liquidity to the Vault in the Fund")
                .arg(fundname.clone())
                .arg(vaultname.clone())
                .arg(get_floating_arg("max_token_a_amount"))
                .arg(get_floating_arg("max_token_b_amount")),
        )
        .subcommand(
            SubCommand::with_name("fund-deposit-vault-locked")
                .about("Add locked liquidity to the Vault in the Fund")
                .arg(fundname.clone())
                .arg(vaultname.clone())
                .arg(get_floating_arg("amount")),
        )
        .subcommand(
            SubCommand::with_name("fund-withdraw-vault")
                .about("Remove liquidity from the Vault in the Fund")
                .arg(fundname.clone())
                .arg(vaultname.clone())
                .arg(get_floating_arg("amount")),
        )
        .subcommand(
            SubCommand::with_name("fund-withdraw-vault-unlocked")
                .about("Remove unlocked liquidity from the Vault in the Fund")
                .arg(fundname.clone())
                .arg(vaultname.clone())
                .arg(get_floating_arg("amount")),
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
                        .required(true)
                        .takes_value(true)
                        .help("Object specific parameter 1"),
                )
                .arg(
                    Arg::with_name("param2")
                        .index(4)
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
                                .required(true)
                                .takes_value(true)
                                .help("Address of the governance program"),
                        )
                        .arg(
                            Arg::with_name("mint-ui-amount")
                                .required(true)
                                .takes_value(true)
                                .validator(|p| match p.parse::<f64>() {
                                    Err(_) => Err(String::from("Must be unsigned integer")),
                                    Ok(_) => Ok(()),
                                })
                                .help("Amount of governance tokens to mint"),
                        ),
                ),
        )
}
