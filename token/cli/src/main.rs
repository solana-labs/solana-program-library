use clap::{
    crate_description, crate_name, crate_version, value_t_or_exit, App, AppSettings, Arg,
    SubCommand,
};
use console::Emoji;
use solana_account_decoder::{
    parse_token::{TokenAccountType, UiAccountState, UiTokenAmount},
    UiAccountData,
};
use solana_clap_utils::{
    input_parsers::{pubkey_of_signer, signer_of},
    input_validators::{is_amount, is_url, is_valid_pubkey, is_valid_signer},
    keypair::DefaultSigner,
};
use solana_cli_output::display::println_name_value;
use solana_client::{rpc_client::RpcClient, rpc_request::TokenAccountsFilter};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::Instruction,
    native_token::*,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use spl_token::{
    self,
    instruction::*,
    native_mint,
    state::{Account, Mint},
};
use std::{process::exit, str::FromStr};

static WARNING: Emoji = Emoji("⚠️", "!");

struct Config {
    rpc_client: RpcClient,
    verbose: bool,
    owner: Pubkey,
    fee_payer: Pubkey,
    commitment_config: CommitmentConfig,
    default_signer: DefaultSigner,
}

type Error = Box<dyn std::error::Error>;
type CommandResult = Result<Option<(u64, Vec<Instruction>)>, Error>;

fn new_throwaway_signer() -> (Option<Box<dyn Signer>>, Option<Pubkey>) {
    let keypair = Keypair::new();
    let pubkey = keypair.pubkey();
    (Some(Box::new(keypair) as Box<dyn Signer>), Some(pubkey))
}

fn check_fee_payer_balance(config: &Config, required_balance: u64) -> Result<(), Error> {
    let balance = config.rpc_client.get_balance(&config.fee_payer)?;
    if balance < required_balance {
        Err(format!(
            "Fee payer, {}, has insufficient balance: {} required, {} available",
            config.fee_payer,
            lamports_to_sol(required_balance),
            lamports_to_sol(balance)
        )
        .into())
    } else {
        Ok(())
    }
}

fn check_owner_balance(config: &Config, required_balance: u64) -> Result<(), Error> {
    let balance = config.rpc_client.get_balance(&config.owner)?;
    if balance < required_balance {
        Err(format!(
            "Owner, {}, has insufficient balance: {} required, {} available",
            config.owner,
            lamports_to_sol(required_balance),
            lamports_to_sol(balance)
        )
        .into())
    } else {
        Ok(())
    }
}

fn command_create_token(
    config: &Config,
    decimals: u8,
    token: Pubkey,
    enable_freeze: bool,
) -> CommandResult {
    println!("Creating token {}", token);

    let minimum_balance_for_rent_exemption = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(Mint::LEN)?;
    let freeze_authority_pubkey = if enable_freeze {
        Some(config.owner)
    } else {
        None
    };

    let instructions = vec![
        system_instruction::create_account(
            &config.fee_payer,
            &token,
            minimum_balance_for_rent_exemption,
            Mint::LEN as u64,
            &spl_token::id(),
        ),
        initialize_mint(
            &spl_token::id(),
            &token,
            &config.owner,
            freeze_authority_pubkey.as_ref(),
            decimals,
        )?,
    ];
    Ok(Some((minimum_balance_for_rent_exemption, instructions)))
}

fn command_create_account(config: &Config, token: Pubkey, account: Pubkey) -> CommandResult {
    println!("Creating account {}", account);

    let minimum_balance_for_rent_exemption = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(Account::LEN)?;

    let instructions = vec![
        system_instruction::create_account(
            &config.fee_payer,
            &account,
            minimum_balance_for_rent_exemption,
            Account::LEN as u64,
            &spl_token::id(),
        ),
        initialize_account(&spl_token::id(), &account, &token, &config.owner)?,
    ];
    Ok(Some((minimum_balance_for_rent_exemption, instructions)))
}

fn command_authorize(
    config: &Config,
    account: Pubkey,
    authority_type: AuthorityType,
    new_owner: Option<Pubkey>,
) -> CommandResult {
    let auth_str = match authority_type {
        AuthorityType::MintTokens => "mint authority",
        AuthorityType::FreezeAccount => "freeze authority",
        AuthorityType::AccountOwner => "owner",
        AuthorityType::CloseAccount => "close authority",
    };
    println!(
        "Updating {}\n  Current {}: {}\n  New {}: {}",
        account,
        auth_str,
        config.owner,
        auth_str,
        new_owner
            .map(|pubkey| pubkey.to_string())
            .unwrap_or_else(|| "disabled".to_string())
    );

    let instructions = vec![set_authority(
        &spl_token::id(),
        &account,
        new_owner.as_ref(),
        authority_type,
        &config.owner,
        &[],
    )?];
    Ok(Some((0, instructions)))
}

fn command_transfer(
    config: &Config,
    sender: Pubkey,
    ui_amount: f64,
    recipient: Pubkey,
) -> CommandResult {
    println!(
        "Transfer {} tokens\n  Sender: {}\n  Recipient: {}",
        ui_amount, sender, recipient
    );

    let source_account = config
        .rpc_client
        .get_token_account_with_commitment(&sender, config.commitment_config)?
        .value
        .ok_or_else(|| format!("Could not find token account {}", sender))?;
    let mint_pubkey = Pubkey::from_str(&source_account.mint)?;
    let amount = spl_token::ui_amount_to_amount(ui_amount, source_account.token_amount.decimals);

    let instructions = vec![transfer_checked(
        &spl_token::id(),
        &sender,
        &mint_pubkey,
        &recipient,
        &config.owner,
        &[],
        amount,
        source_account.token_amount.decimals,
    )?];
    Ok(Some((0, instructions)))
}

fn command_burn(config: &Config, source: Pubkey, ui_amount: f64) -> CommandResult {
    println!("Burn {} tokens\n  Source: {}", ui_amount, source);

    let source_account = config
        .rpc_client
        .get_token_account_with_commitment(&source, config.commitment_config)?
        .value
        .ok_or_else(|| format!("Could not find token account {}", source))?;
    let mint_pubkey = Pubkey::from_str(&source_account.mint)?;
    let amount = spl_token::ui_amount_to_amount(ui_amount, source_account.token_amount.decimals);

    let instructions = vec![burn_checked(
        &spl_token::id(),
        &source,
        &mint_pubkey,
        &config.owner,
        &[],
        amount,
        source_account.token_amount.decimals,
    )?];
    Ok(Some((0, instructions)))
}

fn command_mint(
    config: &Config,
    token: Pubkey,
    ui_amount: f64,
    recipient: Pubkey,
) -> CommandResult {
    println!(
        "Minting {} tokens\n  Token: {}\n  Recipient: {}",
        ui_amount, token, recipient
    );

    let recipient_token_balance = config
        .rpc_client
        .get_token_account_balance_with_commitment(&recipient, config.commitment_config)?
        .value;
    let amount = spl_token::ui_amount_to_amount(ui_amount, recipient_token_balance.decimals);

    let instructions = vec![mint_to_checked(
        &spl_token::id(),
        &token,
        &recipient,
        &config.owner,
        &[],
        amount,
        recipient_token_balance.decimals,
    )?];
    Ok(Some((0, instructions)))
}

fn command_freeze(config: &Config, account: Pubkey) -> CommandResult {
    let token_account = config
        .rpc_client
        .get_token_account_with_commitment(&account, config.commitment_config)?
        .value
        .ok_or_else(|| format!("Could not find token account {}", account))?;
    let token = Pubkey::from_str(&token_account.mint)?;

    println!("Freezing account: {}\n  Token: {}", account, token);

    let instructions = vec![freeze_account(
        &spl_token::id(),
        &account,
        &token,
        &config.owner,
        &[],
    )?];
    Ok(Some((0, instructions)))
}

fn command_thaw(config: &Config, account: Pubkey) -> CommandResult {
    let token_account = config
        .rpc_client
        .get_token_account_with_commitment(&account, config.commitment_config)?
        .value
        .ok_or_else(|| format!("Could not find token account {}", account))?;
    let token = Pubkey::from_str(&token_account.mint)?;

    println!("Freezing account: {}\n  Token: {}", account, token);

    let instructions = vec![thaw_account(
        &spl_token::id(),
        &account,
        &token,
        &config.owner,
        &[],
    )?];
    Ok(Some((0, instructions)))
}

fn command_wrap(config: &Config, sol: f64, account: Pubkey) -> CommandResult {
    let lamports = sol_to_lamports(sol);
    println!("Wrapping {} SOL into {}", sol, account);

    let instructions = vec![
        system_instruction::create_account(
            &config.owner,
            &account,
            lamports,
            Account::LEN as u64,
            &spl_token::id(),
        ),
        initialize_account(
            &spl_token::id(),
            &account,
            &native_mint::id(),
            &config.owner,
        )?,
    ];
    check_owner_balance(config, lamports)?;
    Ok(Some((0, instructions)))
}

fn command_unwrap(config: &Config, address: Pubkey) -> CommandResult {
    println!("Unwrapping {}", address);
    println!(
        "  Amount: {} SOL\n  Recipient: {}",
        lamports_to_sol(
            config
                .rpc_client
                .get_balance_with_commitment(&address, config.commitment_config)?
                .value
        ),
        &config.owner,
    );

    let instructions = vec![close_account(
        &spl_token::id(),
        &address,
        &config.owner,
        &config.owner,
        &[],
    )?];
    Ok(Some((0, instructions)))
}

fn command_approve(
    config: &Config,
    account: Pubkey,
    ui_amount: f64,
    delegate: Pubkey,
) -> CommandResult {
    println!(
        "Approve {} tokens\n  Account: {}\n  Delegate: {}",
        ui_amount, account, delegate
    );

    let source_account = config
        .rpc_client
        .get_token_account_with_commitment(&account, config.commitment_config)?
        .value
        .ok_or_else(|| format!("Could not find token account {}", account))?;
    let mint_pubkey = Pubkey::from_str(&source_account.mint)?;
    let amount = spl_token::ui_amount_to_amount(ui_amount, source_account.token_amount.decimals);

    let instructions = vec![approve_checked(
        &spl_token::id(),
        &account,
        &mint_pubkey,
        &delegate,
        &config.owner,
        &[],
        amount,
        source_account.token_amount.decimals,
    )?];
    Ok(Some((0, instructions)))
}

fn command_revoke(config: &Config, account: Pubkey) -> CommandResult {
    let source_account = config
        .rpc_client
        .get_token_account_with_commitment(&account, config.commitment_config)?
        .value
        .ok_or_else(|| format!("Could not find token account {}", account))?;
    let delegate = source_account.delegate;

    if let Some(delegate) = delegate {
        println!(
            "Revoking approval\n  Account: {}\n  Delegate: {}",
            account, delegate
        );
    } else {
        return Err(format!("No delegate on account {}", account).into());
    }

    let instructions = vec![revoke(&spl_token::id(), &account, &config.owner, &[])?];
    Ok(Some((0, instructions)))
}

fn command_close(config: &Config, account: Pubkey, destination: Pubkey) -> CommandResult {
    let source_account = config
        .rpc_client
        .get_token_account_with_commitment(&account, config.commitment_config)?
        .value
        .ok_or_else(|| format!("Could not find token account {}", account))?;

    if !source_account.is_native && source_account.token_amount.ui_amount > 0.0 {
        return Err(format!(
            "Account {} still has {} tokens; empty the account in order to close it.",
            account, source_account.token_amount.ui_amount
        )
        .into());
    }

    let instructions = vec![close_account(
        &spl_token::id(),
        &account,
        &destination,
        &config.owner,
        &[],
    )?];
    Ok(Some((0, instructions)))
}

fn command_balance(config: &Config, address: Pubkey) -> CommandResult {
    let balance = config
        .rpc_client
        .get_token_account_balance_with_commitment(&address, config.commitment_config)?
        .value;

    if config.verbose {
        println!("ui amount: {}", balance.ui_amount);
        println!("decimals: {}", balance.decimals);
        println!("amount: {}", balance.amount);
    } else {
        println!("{}", balance.ui_amount);
    }
    Ok(None)
}

fn command_supply(config: &Config, address: Pubkey) -> CommandResult {
    let supply = config
        .rpc_client
        .get_token_supply_with_commitment(&address, config.commitment_config)?
        .value;

    println!("{}", supply.ui_amount);
    Ok(None)
}

fn command_accounts(config: &Config, token: Option<Pubkey>) -> CommandResult {
    let accounts = config
        .rpc_client
        .get_token_accounts_by_owner_with_commitment(
            &config.owner,
            match token {
                Some(token) => TokenAccountsFilter::Mint(token),
                None => TokenAccountsFilter::ProgramId(spl_token::id()),
            },
            config.commitment_config,
        )?
        .value;
    if accounts.is_empty() {
        println!("None");
    }

    println!("Account                                      Token                                        Balance");
    println!("-------------------------------------------------------------------------------------------------");
    for keyed_account in accounts {
        let address = keyed_account.pubkey;

        if let UiAccountData::Json(parsed_account) = keyed_account.account.data {
            if parsed_account.program != "spl-token" {
                println!(
                    "{:<44} Unsupported account program: {}",
                    address, parsed_account.program
                );
            } else {
                match serde_json::from_value(parsed_account.parsed) {
                    Ok(TokenAccountType::Account(ui_token_account)) => {
                        let maybe_frozen = if let UiAccountState::Frozen = ui_token_account.state {
                            format!(" {}  Frozen", WARNING)
                        } else {
                            "".to_string()
                        };
                        println!(
                            "{:<44} {:<44} {}{}",
                            address,
                            ui_token_account.mint,
                            ui_token_account.token_amount.ui_amount,
                            maybe_frozen
                        )
                    }
                    Ok(_) => println!("{:<44} Unsupported token account", address),
                    Err(err) => println!("{:<44} Account parse failure: {}", address, err),
                }
            }
        } else {
            println!("{:<44} Unsupported account data format", address);
        }
    }
    Ok(None)
}

fn stringify_ui_token_amount(amount: &UiTokenAmount) -> String {
    let decimals = amount.decimals as usize;
    if decimals > 0 {
        let amount = u64::from_str(&amount.amount).unwrap();

        // Left-pad zeros to decimals + 1, so we at least have an integer zero
        let mut s = format!("{:01$}", amount, decimals + 1);

        // Add the decimal point (Sorry, "," locales!)
        s.insert(s.len() - decimals, '.');
        s
    } else {
        amount.amount.clone()
    }
}

fn stringify_ui_token_amount_trimmed(amount: &UiTokenAmount) -> String {
    let s = stringify_ui_token_amount(amount);
    let zeros_trimmed = s.trim_end_matches('0');
    let decimal_trimmed = zeros_trimmed.trim_end_matches('.');
    decimal_trimmed.to_string()
}

fn command_account(config: &Config, address: Pubkey) -> CommandResult {
    let account = config
        .rpc_client
        .get_token_account_with_commitment(&address, config.commitment_config)?
        .value
        .unwrap();
    println!();
    println_name_value("Address:", &address.to_string());
    println_name_value(
        "Balance:",
        &stringify_ui_token_amount_trimmed(&account.token_amount),
    );
    let mint = format!(
        "{}{}",
        account.mint,
        if account.is_native { " (native)" } else { "" }
    );
    println_name_value("Mint:", &mint);
    println_name_value("Owner:", &account.owner);
    println_name_value("State:", &format!("{:?}", account.state));
    if let Some(delegate) = &account.delegate {
        println!("Delegation:");
        println_name_value("  Delegate:", delegate);
        let allowance = account.delegated_amount.as_ref().unwrap();
        println_name_value(
            "  Allowance:",
            &stringify_ui_token_amount_trimmed(&allowance),
        );
    } else {
        println_name_value("Delegation:", "");
    }
    println_name_value(
        "Close authority:",
        &account.close_authority.as_ref().unwrap_or(&String::new()),
    );
    Ok(None)
}

fn main() {
    let default_decimals = &format!("{}", native_mint::DECIMALS);
    let matches = App::new(crate_name!())
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
                arg.default_value(&config_file)
            } else {
                arg
            }
        })
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
                .help("JSON RPC URL for the cluster.  Default from the configuration file."),
        )
        .arg(
            Arg::with_name("owner")
                .long("owner")
                .value_name("KEYPAIR")
                .validator(is_valid_signer)
                .takes_value(true)
                .global(true)
                .help(
                    "Specify the token owner account. \
                     This may be a keypair file, the ASK keyword. \
                     Defaults to the client keypair.",
                ),
        )
        .arg(
            Arg::with_name("fee_payer")
                .long("fee-payer")
                .value_name("KEYPAIR")
                .validator(is_valid_signer)
                .takes_value(true)
                .global(true)
                .help(
                    "Specify the fee-payer account. \
                     This may be a keypair file, the ASK keyword. \
                     Defaults to the client keypair.",
                ),
        )
        .subcommand(SubCommand::with_name("create-token").about("Create a new token")
                .arg(
                    Arg::with_name("decimals")
                        .long("decimals")
                        .validator(|s| {
                            s.parse::<u8>().map_err(|e| format!("{}", e))?;
                            Ok(())
                        })
                        .value_name("DECIMALS")
                        .takes_value(true)
                        .default_value(&default_decimals)
                        .help("Number of base 10 digits to the right of the decimal place"),
                )
                .arg(
                    Arg::with_name("token_keypair")
                        .value_name("KEYPAIR")
                        .validator(is_valid_signer)
                        .takes_value(true)
                        .index(1)
                        .help(
                            "Specify the token keypair. \
                             This may be a keypair file or the ASK keyword. \
                             [default: randomly generated keypair]"
                        ),
                )
                .arg(
                    Arg::with_name("enable_freeze")
                        .long("enable-freeze")
                        .takes_value(false)
                        .help(
                            "Enable the mint authority to freeze associated token accounts."
                        ),
                ),
        )
        .subcommand(
            SubCommand::with_name("create-account")
                .about("Create a new token account")
                .arg(
                    Arg::with_name("token")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The token that the account will hold"),
                )
                .arg(
                    Arg::with_name("account_keypair")
                        .value_name("KEYPAIR")
                        .validator(is_valid_signer)
                        .takes_value(true)
                        .index(2)
                        .help(
                            "Specify the account keypair. \
                             This may be a keypair file or the ASK keyword. \
                             [default: randomly generated keypair]"
                        ),
                ),
        )
        .subcommand(
            SubCommand::with_name("authorize")
                .about("Authorize a new signing keypair to a token or token account")
                .arg(
                    Arg::with_name("address")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The address of the token account"),
                )
                .arg(
                    Arg::with_name("authority_type")
                        .value_name("AUTHORITY_TYPE")
                        .takes_value(true)
                        .possible_values(&["mint", "freeze", "owner", "close"])
                        .index(2)
                        .required(true)
                        .help("The new authority type. \
                            Token mints support `mint` and `freeze` authorities;\
                            Token accounts support `owner` and `close` authorities."),
                )
                .arg(
                    Arg::with_name("new_authority")
                        .validator(is_valid_pubkey)
                        .value_name("AUTHORITY_ADDRESS")
                        .takes_value(true)
                        .index(3)
                        .required_unless("disable")
                        .help("The address of the new authority"),
                )
                .arg(
                    Arg::with_name("disable")
                        .long("disable")
                        .takes_value(false)
                        .conflicts_with("new_authority")
                        .help("Disable mint, freeze, or close functionality by setting authority to None.")
                ),
        )
        .subcommand(
            SubCommand::with_name("transfer")
                .about("Transfer tokens between accounts")
                .arg(
                    Arg::with_name("sender")
                        .validator(is_valid_pubkey)
                        .value_name("SENDER_TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The token account address of the sender"),
                )
                .arg(
                    Arg::with_name("amount")
                        .validator(is_amount)
                        .value_name("TOKEN_AMOUNT")
                        .takes_value(true)
                        .index(2)
                        .required(true)
                        .help("Amount to send, in tokens"),
                )
                .arg(
                    Arg::with_name("recipient")
                        .validator(is_valid_pubkey)
                        .value_name("RECIPIENT_TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(3)
                        .required(true)
                        .help("The token account address of recipient"),
                ),
        )
        .subcommand(
            SubCommand::with_name("burn")
                .about("Burn tokens from an account")
                .arg(
                    Arg::with_name("source")
                        .validator(is_valid_pubkey)
                        .value_name("SOURCE_TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The token account address to burn from"),
                )
                .arg(
                    Arg::with_name("amount")
                        .validator(is_amount)
                        .value_name("TOKEN_AMOUNT")
                        .takes_value(true)
                        .index(2)
                        .required(true)
                        .help("Amount to burn, in tokens"),
                ),
        )
        .subcommand(
            SubCommand::with_name("mint")
                .about("Mint new tokens")
                .arg(
                    Arg::with_name("token")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The token to mint"),
                )
                .arg(
                    Arg::with_name("amount")
                        .validator(is_amount)
                        .value_name("TOKEN_AMOUNT")
                        .takes_value(true)
                        .index(2)
                        .required(true)
                        .help("Amount to mint, in tokens"),
                )
                .arg(
                    Arg::with_name("recipient")
                        .validator(is_valid_pubkey)
                        .value_name("RECIPIENT_TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(3)
                        .required(true)
                        .help("The token account address of recipient"),
                ),
        )
        .subcommand(
            SubCommand::with_name("freeze")
                .about("Freeze a token account")
                .arg(
                    Arg::with_name("account")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The address of the token account to freeze"),
                ),
        )
        .subcommand(
            SubCommand::with_name("thaw")
                .about("Thaw a token account")
                .arg(
                    Arg::with_name("account")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The address of the token account to thaw"),
                ),
        )
        .subcommand(
            SubCommand::with_name("balance")
                .about("Get token account balance")
                .arg(
                    Arg::with_name("address")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The token account address"),
                ),
        )
        .subcommand(
            SubCommand::with_name("supply")
                .about("Get token supply")
                .arg(
                    Arg::with_name("address")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The token address"),
                ),
        )
        .subcommand(
            SubCommand::with_name("accounts")
                .about("List all token accounts by owner")
                .arg(
                    Arg::with_name("token")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .help("Limit results to the given token. [Default: list accounts for all tokens]"),
                ),
        )
        .subcommand(
            SubCommand::with_name("wrap")
                .about("Wrap native SOL in a SOL token account")
                .arg(
                    Arg::with_name("amount")
                        .validator(is_amount)
                        .value_name("AMOUNT")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("Amount of SOL to wrap"),
                ),
        )
        .subcommand(
            SubCommand::with_name("unwrap")
                .about("Unwrap a SOL token account")
                .arg(
                    Arg::with_name("address")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The address of the token account to unwrap"),
                ),
        )
        .subcommand(
            SubCommand::with_name("account-info")
                .about("Query details of an SPL Token account by address")
                .arg(
                    Arg::with_name("address")
                    .validator(is_valid_pubkey)
                    .value_name("TOKEN_ACCOUNT_ADDRESS")
                    .takes_value(true)
                    .index(1)
                    .required(true)
                    .help("The address of the SPL Token account to query"),
                ),
            )
        .subcommand(
            SubCommand::with_name("approve")
                .about("Approve a delegate for a token account")
                .arg(
                    Arg::with_name("account")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The address of the token account to delegate"),
                )
                .arg(
                    Arg::with_name("amount")
                        .validator(is_amount)
                        .value_name("TOKEN_AMOUNT")
                        .takes_value(true)
                        .index(2)
                        .required(true)
                        .help("Amount to approve, in tokens"),
                )
                .arg(
                    Arg::with_name("delegate")
                        .validator(is_valid_pubkey)
                        .value_name("DELEGATE_TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(3)
                        .required(true)
                        .help("The token account address of delegate"),
                ),
        )
        .subcommand(
            SubCommand::with_name("revoke")
                .about("Revoke a delegate's authority")
                .arg(
                    Arg::with_name("account")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The address of the token account"),
                ),
        )
        .subcommand(
            SubCommand::with_name("close")
                .about("Close a token account")
                .arg(
                    Arg::with_name("account")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The address of the token account to close"),
                )
                .arg(
                    Arg::with_name("destination")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(2)
                        .required(true)
                        .help("The address of the account to receive remaining SOL"),
                ),
        )
        .get_matches();

    let mut wallet_manager = None;
    let mut bulk_signers: Vec<Option<Box<dyn Signer>>> = Vec::new();

    let config = {
        let cli_config = if let Some(config_file) = matches.value_of("config_file") {
            solana_cli_config::Config::load(config_file).unwrap_or_default()
        } else {
            solana_cli_config::Config::default()
        };
        let json_rpc_url = matches
            .value_of("json_rpc_url")
            .unwrap_or(&cli_config.json_rpc_url)
            .to_string();

        let default_signer_arg_name = "owner".to_string();
        let default_signer_path = matches
            .value_of(&default_signer_arg_name)
            .map(|s| s.to_string())
            .unwrap_or(cli_config.keypair_path);
        let default_signer = DefaultSigner {
            path: default_signer_path,
            arg_name: default_signer_arg_name,
        };
        let owner = default_signer
            .signer_from_path(&matches, &mut wallet_manager)
            .unwrap_or_else(|e| {
                eprintln!("error: {}", e);
                exit(1);
            })
            .pubkey();
        bulk_signers.push(None);
        let (signer, fee_payer) = signer_of(&matches, "fee_payer", &mut wallet_manager)
            .unwrap_or_else(|e| {
                eprintln!("error: {}", e);
                exit(1);
            });
        let fee_payer = fee_payer.unwrap_or(owner);
        bulk_signers.push(signer);

        let verbose = matches.is_present("verbose");

        Config {
            rpc_client: RpcClient::new(json_rpc_url),
            verbose,
            owner,
            fee_payer,
            commitment_config: CommitmentConfig::single_gossip(),
            default_signer,
        }
    };

    solana_logger::setup_with_default("solana=info");

    let _ = match matches.subcommand() {
        ("create-token", Some(arg_matches)) => {
            let decimals = value_t_or_exit!(arg_matches, "decimals", u8);
            let (signer, token) = if arg_matches.is_present("token_keypair") {
                signer_of(&arg_matches, "token_keypair", &mut wallet_manager).unwrap_or_else(|e| {
                    eprintln!("error: {}", e);
                    exit(1);
                })
            } else {
                new_throwaway_signer()
            };
            let token = token.unwrap();
            bulk_signers.push(signer);

            command_create_token(
                &config,
                decimals,
                token,
                arg_matches.is_present("enable_freeze"),
            )
        }
        ("create-account", Some(arg_matches)) => {
            let token = pubkey_of_signer(arg_matches, "token", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let (signer, account) = if arg_matches.is_present("account_keypair") {
                signer_of(&arg_matches, "account_keypair", &mut wallet_manager).unwrap_or_else(
                    |e| {
                        eprintln!("error: {}", e);
                        exit(1);
                    },
                )
            } else {
                new_throwaway_signer()
            };
            let account = account.unwrap();
            bulk_signers.push(signer);

            command_create_account(&config, token, account)
        }
        ("authorize", Some(arg_matches)) => {
            let address = pubkey_of_signer(arg_matches, "address", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let authority_type = arg_matches.value_of("authority_type").unwrap();
            let authority_type = match authority_type {
                "mint" => AuthorityType::MintTokens,
                "freeze" => AuthorityType::FreezeAccount,
                "owner" => AuthorityType::AccountOwner,
                "close" => AuthorityType::CloseAccount,
                _ => unreachable!(),
            };
            let new_authority =
                pubkey_of_signer(arg_matches, "new_authority", &mut wallet_manager).unwrap();
            command_authorize(&config, address, authority_type, new_authority)
        }
        ("transfer", Some(arg_matches)) => {
            let sender = pubkey_of_signer(arg_matches, "sender", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let amount = value_t_or_exit!(arg_matches, "amount", f64);
            let recipient = pubkey_of_signer(arg_matches, "recipient", &mut wallet_manager)
                .unwrap()
                .unwrap();
            command_transfer(&config, sender, amount, recipient)
        }
        ("burn", Some(arg_matches)) => {
            let source = pubkey_of_signer(arg_matches, "source", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let amount = value_t_or_exit!(arg_matches, "amount", f64);
            command_burn(&config, source, amount)
        }
        ("mint", Some(arg_matches)) => {
            let token = pubkey_of_signer(arg_matches, "token", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let amount = value_t_or_exit!(arg_matches, "amount", f64);
            let recipient = pubkey_of_signer(arg_matches, "recipient", &mut wallet_manager)
                .unwrap()
                .unwrap();
            command_mint(&config, token, amount, recipient)
        }
        ("freeze", Some(arg_matches)) => {
            let account = pubkey_of_signer(arg_matches, "account", &mut wallet_manager)
                .unwrap()
                .unwrap();
            command_freeze(&config, account)
        }
        ("thaw", Some(arg_matches)) => {
            let account = pubkey_of_signer(arg_matches, "account", &mut wallet_manager)
                .unwrap()
                .unwrap();
            command_thaw(&config, account)
        }
        ("wrap", Some(arg_matches)) => {
            let amount = value_t_or_exit!(arg_matches, "amount", f64);
            let (signer, account) = new_throwaway_signer();
            let account = account.unwrap();
            bulk_signers.push(signer);
            command_wrap(&config, amount, account)
        }
        ("unwrap", Some(arg_matches)) => {
            let address = pubkey_of_signer(arg_matches, "address", &mut wallet_manager)
                .unwrap()
                .unwrap();
            command_unwrap(&config, address)
        }
        ("approve", Some(arg_matches)) => {
            let account = pubkey_of_signer(arg_matches, "account", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let amount = value_t_or_exit!(arg_matches, "amount", f64);
            let delegate = pubkey_of_signer(arg_matches, "delegate", &mut wallet_manager)
                .unwrap()
                .unwrap();
            command_approve(&config, account, amount, delegate)
        }
        ("revoke", Some(arg_matches)) => {
            let account = pubkey_of_signer(arg_matches, "account", &mut wallet_manager)
                .unwrap()
                .unwrap();
            command_revoke(&config, account)
        }
        ("close", Some(arg_matches)) => {
            let account = pubkey_of_signer(arg_matches, "account", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let destination = pubkey_of_signer(arg_matches, "destination", &mut wallet_manager)
                .unwrap()
                .unwrap();
            command_close(&config, account, destination)
        }
        ("balance", Some(arg_matches)) => {
            let address = pubkey_of_signer(arg_matches, "address", &mut wallet_manager)
                .unwrap()
                .unwrap();
            command_balance(&config, address)
        }
        ("supply", Some(arg_matches)) => {
            let address = pubkey_of_signer(arg_matches, "address", &mut wallet_manager)
                .unwrap()
                .unwrap();
            command_supply(&config, address)
        }
        ("accounts", Some(arg_matches)) => {
            let token = pubkey_of_signer(arg_matches, "token", &mut wallet_manager).unwrap();
            command_accounts(&config, token)
        }
        ("account-info", Some(arg_matches)) => {
            let address = pubkey_of_signer(arg_matches, "address", &mut wallet_manager)
                .unwrap()
                .unwrap();
            command_account(&config, address)
        }
        _ => unreachable!(),
    }
    .and_then(|transaction_info| {
        if let Some((minimum_balance_for_rent_exemption, instructions)) = transaction_info {
            let mut transaction =
                Transaction::new_with_payer(&instructions, Some(&config.fee_payer));
            let (recent_blockhash, fee_calculator) = config
                .rpc_client
                .get_recent_blockhash()
                .unwrap_or_else(|e| {
                    eprintln!("error: {}", e);
                    exit(1);
                });
            check_fee_payer_balance(
                &config,
                minimum_balance_for_rent_exemption
                    + fee_calculator.calculate_fee(&transaction.message()),
            )?;
            let signer_info = config
                .default_signer
                .generate_unique_signers(bulk_signers, &matches, &mut wallet_manager)
                .unwrap_or_else(|e| {
                    eprintln!("error: {}", e);
                    exit(1);
                });
            transaction.sign(&signer_info.signers, recent_blockhash);

            let signature = config
                .rpc_client
                .send_and_confirm_transaction_with_spinner_and_commitment(
                    &transaction,
                    config.commitment_config,
                )?;
            println!("Signature: {}", signature);
        }
        Ok(())
    })
    .map_err(|err| {
        eprintln!("{}", err);
        exit(1);
    });
}
