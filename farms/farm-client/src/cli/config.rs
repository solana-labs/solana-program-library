//! Configuration and command line arguments management.

use {
    clap::{crate_description, crate_name, App, AppSettings, Arg, ArgMatches, SubCommand},
    solana_clap_utils::{input_validators::is_url, keypair::signer_from_path},
    solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signer},
    std::str::FromStr,
};

#[derive(Debug)]
pub struct Config {
    pub farm_client_url: String,
    pub commitment: CommitmentConfig,
    pub keypair: Box<dyn Signer>,
    pub no_pretty_print: bool,
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

        Self {
            farm_client_url: farm_client_url.to_string(),
            commitment: CommitmentConfig::from_str(commitment).unwrap(),
            keypair: signer_from_path(matches, keypair_path, "signer", &mut None).unwrap(),
            no_pretty_print: matches.is_present("no_pretty_print"),
        }
    }
}

pub fn get_target(matches: &ArgMatches) -> String {
    matches
        .value_of("target")
        .unwrap()
        .parse::<String>()
        .unwrap()
        .to_lowercase()
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

pub fn get_vec_str_val<'a>(matches: &ArgMatches<'a>, argname: &str) -> Vec<String> {
    matches
        .value_of(argname)
        .unwrap()
        .parse::<String>()
        .unwrap()
        .to_uppercase()
        .split(',')
        .collect::<Vec<&str>>()
        .iter()
        .map(|s| s.to_string())
        .collect()
}

pub fn get_vec_str_val_raw<'a>(matches: &ArgMatches<'a>, argname: &str) -> Vec<String> {
    matches
        .value_of(argname)
        .unwrap()
        .parse::<String>()
        .unwrap()
        .split(',')
        .collect::<Vec<&str>>()
        .iter()
        .map(|s| s.to_string())
        .collect()
}

pub fn get_amount_val<'a>(matches: &ArgMatches<'a>, argname: &str) -> f64 {
    matches.value_of(argname).unwrap().parse::<f64>().unwrap()
}

pub fn get_pubkey_val<'a>(matches: &ArgMatches<'a>, argname: &str) -> Pubkey {
    Pubkey::from_str(matches.value_of(argname).unwrap()).unwrap()
}

pub fn get_integer_val<'a>(matches: &ArgMatches<'a>, argname: &str) -> u64 {
    matches.value_of(argname).unwrap().parse::<u64>().unwrap()
}

fn get_arg(name: &str) -> Arg {
    Arg::with_name(name).required(true).takes_value(true)
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

pub fn get_clap_app<'a, 'b>(version: &'b str) -> App<'a, 'b> {
    let target = Arg::with_name("target")
        .required(true)
        .takes_value(true)
        .possible_values(&["program", "vault", "farm", "pool", "token"])
        .hide_possible_values(true)
        .help("Target object type (program, vault, etc.)");

    let objectname = Arg::with_name("object_name")
        .required(true)
        .takes_value(true)
        .help("Target object name");

    let tokenname = Arg::with_name("token_name")
        .required(true)
        .takes_value(true)
        .help("Token name");

    let tokenname2 = Arg::with_name("token_name2")
        .required(true)
        .takes_value(true)
        .help("Second token name");

    let amount = Arg::with_name("amount")
        .takes_value(true)
        .required(true)
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

    let amount2 = Arg::with_name("amount2")
        .takes_value(true)
        .required(false)
        .default_value("0")
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
        .help("Second token amount");

    let wallet = Arg::with_name("wallet")
        .takes_value(true)
        .required(true)
        .validator(|p| match Pubkey::from_str(&p) {
            Err(_) => Err(String::from("Must be public key")),
            Ok(_) => Ok(()),
        })
        .help("Wallet address");

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
                .help("Print every record in one line"),
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
                .about("Query all object names of the given type and print")
                .arg(target.clone()),
        )
        .subcommand(
            SubCommand::with_name("pool-price")
                .about("Print pool price")
                .arg(get_arg("pool_name")),
        )
        .subcommand(
            SubCommand::with_name("transfer")
                .about("Transfer SOL to another wallet")
                .arg(wallet.clone())
                .arg(amount.clone()),
        )
        .subcommand(
            SubCommand::with_name("token-transfer")
                .about("Transfer tokens to another wallet")
                .arg(tokenname.clone())
                .arg(wallet.clone())
                .arg(amount.clone()),
        )
        .subcommand(
            SubCommand::with_name("token-address")
                .about("Print associated token account address")
                .arg(tokenname.clone()),
        )
        .subcommand(SubCommand::with_name("balance").about("Print SOL balance"))
        .subcommand(
            SubCommand::with_name("token-balance")
                .about("Print token balance")
                .arg(tokenname.clone()),
        )
        .subcommand(
            SubCommand::with_name("stake-balance")
                .about("Print user's stake balance in the farm")
                .arg(get_arg("farm_name")),
        )
        .subcommand(
            SubCommand::with_name("wallet-balances")
                .about("Print all token balances for the wallet")
        )
        .subcommand(
            SubCommand::with_name("token-create")
                .about("Create associated token account")
                .arg(tokenname.clone()),
        )
        .subcommand(
            SubCommand::with_name("vault-info")
                .about("Print vault stats")
                .arg(get_arg("vault_name")),
        )
        .subcommand(
            SubCommand::with_name("vault-user-info")
                .about("Print user stats for the vault")
                .arg(get_arg("vault_name")),
        )
        .subcommand(
            SubCommand::with_name("find-pools")
                .about("Find all Pools with tokens A and B")
                .arg(get_arg("protocol"))
                .arg(tokenname.clone())
                .arg(tokenname2.clone())
        )
        .subcommand(
            SubCommand::with_name("find-pools-with-lp")
                .about("Find all Pools for the given LP token")
                .arg(tokenname.clone())
        )
        .subcommand(
            SubCommand::with_name("find-farms-with-lp")
                .about("Find all Farms for the given LP token")
                .arg(tokenname.clone())
        )
        .subcommand(
            SubCommand::with_name("find-vaults")
                .about("Find all Vaults with tokens A and B")
                .arg(tokenname.clone())
                .arg(tokenname2.clone())
        )
        .subcommand(
            SubCommand::with_name("swap")
                .about("Swap tokens in the pool")
                .arg(get_arg("protocol"))
                .arg(tokenname.clone())
                .arg(tokenname2.clone())
                .arg(amount.clone())
                .arg(amount2.clone()),
        )
        .subcommand(
            SubCommand::with_name("deposit-pool")
                .about("Add liquidity to the pool")
                .arg(get_arg("pool_name"))
                .arg(amount.clone())
                .arg(amount2.clone()),
        )
        .subcommand(
            SubCommand::with_name("withdraw-pool")
                .about("Remove liquidity from the pool")
                .arg(get_arg("pool_name"))
                .arg(amount.clone()),
        )
        .subcommand(
            SubCommand::with_name("stake")
                .about("Stake LP tokens to the farm")
                .arg(get_arg("farm_name"))
                .arg(amount.clone()),
        )
        .subcommand(
            SubCommand::with_name("harvest")
                .about("Harvest farm rewards")
                .arg(get_arg("farm_name")),
        )
        .subcommand(
            SubCommand::with_name("unstake")
                .about("Unstake LP tokens from the farm")
                .arg(get_arg("farm_name"))
                .arg(amount.clone()),
        )
        .subcommand(
            SubCommand::with_name("deposit-vault")
                .about("Add liquidity to the vault")
                .arg(get_arg("vault_name"))
                .arg(amount.clone())
                .arg(amount2.clone()),
        )
        .subcommand(
            SubCommand::with_name("deposit-vault-locked")
                .about("Add locked liquidity to the vault")
                .arg(get_arg("vault_name"))
                .arg(amount.clone()),
        )
        .subcommand(
            SubCommand::with_name("withdraw-vault")
                .about("Remove liquidity from the vault")
                .arg(get_arg("vault_name"))
                .arg(amount.clone()),
        )
        .subcommand(
            SubCommand::with_name("withdraw-vault-unlocked")
                .about("Remove unlocked liquidity from the vault")
                .arg(get_arg("vault_name"))
                .arg(amount.clone()),
        )
        .subcommand(
            SubCommand::with_name("governance")
                .about("Governance commands. See `solana-farm-client governance help`")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .subcommand(
                    SubCommand::with_name("get-config")
                    .about("Get governance config")
                    .arg(get_arg("governance_name"))
                )
                .subcommand(
                    SubCommand::with_name("get-address")
                    .about("Get governance account address")
                    .arg(get_arg("governance_name"))
                )
                .subcommand(
                    SubCommand::with_name("get-instruction")
                    .about("Print stored instruction in the proposal")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_integer_arg("instruction_index"))
                )
                .subcommand(
                    SubCommand::with_name("custody-new")
                    .about("Create new token custody account")
                    .arg(get_arg("token_name"))
                )
                .subcommand(
                    SubCommand::with_name("tokens-deposit")
                    .about("Deposit governing tokens")
                    .arg(amount.clone()),
                )
                .subcommand(
                    SubCommand::with_name("tokens-withdraw")
                    .about("Withdraw governing tokens")
                )
                .subcommand(
                    SubCommand::with_name("proposal-new")
                    .about("Create a new proposal")
                    .arg(get_arg("governance_name"))
                    .arg(get_arg("proposal_name"))
                    .arg(get_arg("proposal_link"))
                    .arg(get_integer_arg("proposal_index"))
                )
                .subcommand(
                    SubCommand::with_name("proposal-cancel")
                    .about("Cancel the proposal")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                )
                .subcommand(
                    SubCommand::with_name("proposal-state")
                    .about("Get proposal state")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                )
                .subcommand(
                    SubCommand::with_name("signatory-add")
                    .about("Add a signatory to the proposal")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_arg("signatory"))
                )
                .subcommand(
                    SubCommand::with_name("signatory-remove")
                    .about("Remove the signatory from the proposal")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_arg("signatory"))
                )
                .subcommand(
                    SubCommand::with_name("sign-off")
                    .about("Sign off the proposal")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                )
                .subcommand(
                    SubCommand::with_name("vote-cast")
                    .about("Cast a vote on the proposal")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_integer_arg("vote"))
                )
                .subcommand(
                    SubCommand::with_name("vote-relinquish")
                    .about("Remove the vote from the proposal")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                )
                .subcommand(
                    SubCommand::with_name("vote-finalize")
                    .about("Finalize the vote on the proposal")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                )
                .subcommand(
                    SubCommand::with_name("instruction-execute")
                    .about("Execute the instruction in the proposal")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_integer_arg("instruction_index"))
                )
                .subcommand(
                    SubCommand::with_name("instruction-flag-error")
                    .about("Mark the instruction as failed")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_integer_arg("instruction_index"))
                )
                .subcommand(
                    SubCommand::with_name("instruction-remove")
                    .about("Remove the instruction from the proposal")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_integer_arg("instruction_index"))
                )
                .subcommand(
                    SubCommand::with_name("instruction-insert")
                    .about("Add a new custom instruction to the proposal. Must be serialized with base64::encode(bincode::serialize(&inst).unwrap().as_slice())")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_integer_arg("instruction_index"))
                    .arg(get_arg("base64_instruction"))
                )
                .subcommand(
                    SubCommand::with_name("instruction-verify")
                    .about("Verify custom instruction in the proposal. Must be serialized with base64::encode(bincode::serialize(&inst).unwrap().as_slice())")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_integer_arg("instruction_index"))
                    .arg(get_arg("base64_instruction"))
                )
                .subcommand(
                    SubCommand::with_name("instruction-insert-token-transfer")
                    .about("Add a new token transfer instruction to the proposal")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_integer_arg("instruction_index"))
                    .arg(tokenname.clone())
                    .arg(wallet.clone())
                    .arg(amount.clone()),
                )
                .subcommand(
                    SubCommand::with_name("instruction-verify-token-transfer")
                    .about("Verify that instruction in the proposal is a token transfer")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_integer_arg("instruction_index"))
                    .arg(tokenname.clone())
                    .arg(wallet.clone())
                    .arg(amount.clone()),
                )
                .subcommand(
                    SubCommand::with_name("instruction-insert-swap")
                    .about("Add a new swap instruction to the proposal")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_integer_arg("instruction_index"))
                    .arg(get_arg("protocol"))
                    .arg(tokenname.clone())
                    .arg(tokenname2.clone())
                    .arg(amount.clone())
                    .arg(amount2.clone()),
                )
                .subcommand(
                    SubCommand::with_name("instruction-verify-swap")
                    .about("Verify that instruction in the proposal is a swap")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_integer_arg("instruction_index"))
                    .arg(get_arg("protocol"))
                    .arg(tokenname.clone())
                    .arg(tokenname2.clone())
                    .arg(amount.clone())
                    .arg(amount2.clone()),
                )
                .subcommand(
                    SubCommand::with_name("instruction-insert-deposit-pool")
                    .about("Add a new add liquidity to the pool instruction to the proposal")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_integer_arg("instruction_index"))
                    .arg(get_arg("pool_name"))
                    .arg(amount.clone())
                    .arg(amount2.clone()),
                )
                .subcommand(
                    SubCommand::with_name("instruction-verify-deposit-pool")
                    .about("Verify that instruction in the proposal is an add liquidity to the pool")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_integer_arg("instruction_index"))
                    .arg(get_arg("pool_name"))
                    .arg(amount.clone())
                    .arg(amount2.clone()),
                )
                .subcommand(
                    SubCommand::with_name("instruction-insert-withdraw-pool")
                    .about("Add a new remove liquidity from the pool instruction to the proposal")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_integer_arg("instruction_index"))
                    .arg(get_arg("pool_name"))
                    .arg(amount.clone()),
                )
                .subcommand(
                    SubCommand::with_name("instruction-verify-withdraw-pool")
                    .about("Verify that instruction in the proposal is a remove liquidity from the pool")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_integer_arg("instruction_index"))
                    .arg(get_arg("pool_name"))
                    .arg(amount.clone()),
                )
                .subcommand(
                    SubCommand::with_name("instruction-insert-stake")
                    .about("Add a new stake instruction to the proposal")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_integer_arg("instruction_index"))
                    .arg(get_arg("farm_name"))
                    .arg(amount.clone()),
                )
                .subcommand(
                    SubCommand::with_name("instruction-verify-stake")
                    .about("Verify that instruction in the proposal is a stake")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_integer_arg("instruction_index"))
                    .arg(get_arg("farm_name"))
                    .arg(amount.clone()),
                )
                .subcommand(
                    SubCommand::with_name("instruction-insert-harvest")
                    .about("Add a new harvest instruction to the proposal")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_integer_arg("instruction_index"))
                    .arg(get_arg("farm_name")),
                )
                .subcommand(
                    SubCommand::with_name("instruction-verify-harvest")
                    .about("Verify that instruction in the proposal is a harvest")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_integer_arg("instruction_index"))
                    .arg(get_arg("farm_name")),
                )
                .subcommand(
                    SubCommand::with_name("instruction-insert-unstake")
                    .about("Add a new unstake instruction to the proposal")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_integer_arg("instruction_index"))
                    .arg(get_arg("farm_name"))
                    .arg(amount.clone()),
                )
                .subcommand(
                    SubCommand::with_name("instruction-verify-unstake")
                    .about("Verify that instruction in the proposal is an unstake")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_integer_arg("instruction_index"))
                    .arg(get_arg("farm_name"))
                    .arg(amount.clone()),
                )
                .subcommand(
                    SubCommand::with_name("instruction-insert-deposit-vault")
                    .about("Add a new add liquidity to the vault instruction to the proposal")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_integer_arg("instruction_index"))
                    .arg(get_arg("vault_name"))
                    .arg(amount.clone())
                    .arg(amount2.clone()),
                )
                .subcommand(
                    SubCommand::with_name("instruction-verify-deposit-vault")
                    .about("Verify that instruction in the proposal is an add liquidity to the vault")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_integer_arg("instruction_index"))
                    .arg(get_arg("vault_name"))
                    .arg(amount.clone())
                    .arg(amount2.clone()),
                )
                .subcommand(
                    SubCommand::with_name("instruction-insert-withdraw-vault")
                    .about("Add a new remove liquidity from the vault instruction to the proposal")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_integer_arg("instruction_index"))
                    .arg(get_arg("vault_name"))
                    .arg(amount.clone()),
                )
                .subcommand(
                    SubCommand::with_name("instruction-verify-withdraw-vault")
                    .about("Verify that instruction in the proposal is a remove liquidity from the vault")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_integer_arg("instruction_index"))
                    .arg(get_arg("vault_name"))
                    .arg(amount.clone()),
                )
                .subcommand(
                    SubCommand::with_name("instruction-insert-withdraw-fees-vault")
                    .about("Add a new withdraw fees from the vault instruction to the proposal")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_integer_arg("instruction_index"))
                    .arg(get_arg("vault_name"))
                    .arg(get_integer_arg("fee_token"))
                    .arg(amount.clone())
                    .arg(get_arg("receiver"))
                )
                .subcommand(
                    SubCommand::with_name("instruction-verify-withdraw-fees-vault")
                    .about("Verify that instruction in the proposal is a withdraw fees from the vault")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_integer_arg("instruction_index"))
                    .arg(get_arg("vault_name"))
                    .arg(get_integer_arg("fee_token"))
                    .arg(amount.clone())
                    .arg(get_arg("receiver"))
                )
                .subcommand(
                    SubCommand::with_name("instruction-insert-program-upgrade")
                    .about("Add a new program upgrade instruction to the proposal")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_integer_arg("instruction_index"))
                    .arg(get_arg("buffer_address"))
                )
                .subcommand(
                    SubCommand::with_name("instruction-verify-program-upgrade")
                    .about("Verify that instruction in the proposal is a program upgrade")
                    .arg(get_arg("governance_name"))
                    .arg(get_integer_arg("proposal_index"))
                    .arg(get_integer_arg("instruction_index"))
                    .arg(get_arg("buffer_address"))
                )
        )
}
