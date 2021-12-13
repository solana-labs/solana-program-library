use {
    clap::{
        crate_description, crate_name, crate_version, value_t, App, AppSettings, Arg, ArgMatches,
        SubCommand,
    },
    solana_clap_utils::{
        fee_payer::fee_payer_arg,
        input_parsers::{keypair_of, pubkey_of, value_of},
        input_validators::{is_amount, is_keypair, is_parsable, is_pubkey, is_url},
        keypair::signer_from_path,
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{native_token::lamports_to_sol, program_pack::Pack, pubkey::Pubkey},
    solana_sdk::{
        commitment_config::CommitmentConfig,
        signature::{Keypair, Signer},
        system_instruction,
        transaction::Transaction,
    },
    spl_token::{
        instruction::{approve, revoke},
        state::{Account as Token, Mint},
        ui_amount_to_amount,
    },
    spl_token_lending::{
        self,
        instruction::{init_lending_market, init_reserve},
        math::WAD,
        state::{LendingMarket, Reserve, ReserveConfig, ReserveFees},
    },
    std::{borrow::Borrow, process::exit, str::FromStr},
    system_instruction::create_account,
};

struct Config {
    rpc_client: RpcClient,
    fee_payer: Box<dyn Signer>,
    lending_program_id: Pubkey,
    verbose: bool,
    dry_run: bool,
}

type Error = Box<dyn std::error::Error>;
type CommandResult = Result<(), Error>;

const PYTH_PROGRAM_ID: &str = "gSbePebfvPy7tRqimPoVecS2UsBvYv46ynrzWocc92s";

fn main() {
    solana_logger::setup_with_default("solana=info");

    let default_lending_program_id: &str = &spl_token_lending::id().to_string();

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
                arg.default_value(config_file)
            } else {
                arg
            }
        })
        .arg(
            Arg::with_name("json_rpc_url")
                .long("url")
                .value_name("URL")
                .takes_value(true)
                .validator(is_url)
                .help("JSON RPC URL for the cluster.  Default from the configuration file."),
        )
        .arg(
            fee_payer_arg()
                .short("p")
                .global(true)
        )
        .arg(
            Arg::with_name("lending_program_id")
                .long("program")
                .validator(is_pubkey)
                .value_name("PUBKEY")
                .takes_value(true)
                .required(true)
                .default_value(default_lending_program_id)
                .help("Lending program ID"),
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
            Arg::with_name("dry_run")
                .long("dry-run")
                .takes_value(false)
                .global(true)
                .help("Simulate transaction instead of executing"),
        )
        .subcommand(
            SubCommand::with_name("create-market")
                .about("Create a new lending market")
                .arg(
                    Arg::with_name("lending_market_owner")
                        .long("market-owner")
                        .validator(is_pubkey)
                        .value_name("PUBKEY")
                        .takes_value(true)
                        .required(true)
                        .help("Owner that can add reserves to the market"),
                )
                .arg(
                    Arg::with_name("oracle_program_id")
                        .long("oracle")
                        .validator(is_pubkey)
                        .value_name("PUBKEY")
                        .takes_value(true)
                        .required(true)
                        .default_value(PYTH_PROGRAM_ID)
                        .help("Oracle (Pyth) program ID for quoting market prices"),
                )
                .arg(
                    Arg::with_name("quote_currency")
                        .long("quote")
                        .value_name("STRING")
                        .takes_value(true)
                        .required(true)
                        .default_value("USD")
                        .help("Currency market prices are quoted in"),
                ),
        )
        .subcommand(
            SubCommand::with_name("add-reserve")
                .about("Add a reserve to a lending market")
                // @TODO: use is_valid_signer
                .arg(
                    Arg::with_name("lending_market_owner")
                        .long("market-owner")
                        .validator(is_keypair)
                        .value_name("KEYPAIR")
                        .takes_value(true)
                        .required(true)
                        .help("Owner of the lending market"),
                )
                // @TODO: use is_valid_signer
                .arg(
                    Arg::with_name("source_liquidity_owner")
                        .long("source-owner")
                        .validator(is_keypair)
                        .value_name("KEYPAIR")
                        .takes_value(true)
                        .required(true)
                        .help("Owner of the SPL Token account to deposit initial liquidity from"),
                )
                .arg(
                    Arg::with_name("lending_market")
                        .long("market")
                        .validator(is_pubkey)
                        .value_name("PUBKEY")
                        .takes_value(true)
                        .required(true)
                        .help("Lending market address"),
                )
                .arg(
                    Arg::with_name("source_liquidity")
                        .long("source")
                        .validator(is_pubkey)
                        .value_name("PUBKEY")
                        .takes_value(true)
                        .required(true)
                        .help("SPL Token account to deposit initial liquidity from"),
                )
                // @TODO: use is_amount_or_all
                .arg(
                    Arg::with_name("liquidity_amount")
                        .long("amount")
                        .validator(is_amount)
                        .value_name("DECIMAL_AMOUNT")
                        .takes_value(true)
                        .required(true)
                        .help("Initial amount of liquidity to deposit into the new reserve"),
                )
                .arg(
                    Arg::with_name("pyth_product")
                        .long("pyth-product")
                        .validator(is_pubkey)
                        .value_name("PUBKEY")
                        .takes_value(true)
                        .required(true)
                        .help("Pyth product account: https://pyth.network/developers/consumers/accounts"),
                )
                .arg(
                    Arg::with_name("pyth_price")
                        .long("pyth-price")
                        .validator(is_pubkey)
                        .value_name("PUBKEY")
                        .takes_value(true)
                        .required(true)
                        .help("Pyth price account: https://pyth.network/developers/consumers/accounts"),
                )
                .arg(
                    Arg::with_name("optimal_utilization_rate")
                        .long("optimal-utilization-rate")
                        .validator(is_parsable::<u8>)
                        .value_name("INTEGER_PERCENT")
                        .takes_value(true)
                        .required(true)
                        .default_value("80")
                        .help("Optimal utilization rate: [0, 100]"),
                )
                .arg(
                    Arg::with_name("loan_to_value_ratio")
                        .long("loan-to-value-ratio")
                        .validator(is_parsable::<u8>)
                        .value_name("INTEGER_PERCENT")
                        .takes_value(true)
                        .required(true)
                        .default_value("50")
                        .help("Target ratio of the value of borrows to deposits: [0, 100)"),
                )
                .arg(
                    Arg::with_name("liquidation_bonus")
                        .long("liquidation-bonus")
                        .validator(is_parsable::<u8>)
                        .value_name("INTEGER_PERCENT")
                        .takes_value(true)
                        .required(true)
                        .default_value("5")
                        .help("Bonus a liquidator gets when repaying part of an unhealthy obligation: [0, 100]"),
                )
                .arg(
                    Arg::with_name("liquidation_threshold")
                        .long("liquidation-threshold")
                        .validator(is_parsable::<u8>)
                        .value_name("INTEGER_PERCENT")
                        .takes_value(true)
                        .required(true)
                        .default_value("55")
                        .help("Loan to value ratio at which an obligation can be liquidated: (LTV, 100]"),
                )
                .arg(
                    Arg::with_name("min_borrow_rate")
                        .long("min-borrow-rate")
                        .validator(is_parsable::<u8>)
                        .value_name("INTEGER_PERCENT")
                        .takes_value(true)
                        .required(true)
                        .default_value("0")
                        .help("Min borrow APY: min <= optimal <= max"),
                )
                .arg(
                    Arg::with_name("optimal_borrow_rate")
                        .long("optimal-borrow-rate")
                        .validator(is_parsable::<u8>)
                        .value_name("INTEGER_PERCENT")
                        .takes_value(true)
                        .required(true)
                        .default_value("4")
                        .help("Optimal (utilization) borrow APY: min <= optimal <= max"),
                )
                .arg(
                    Arg::with_name("max_borrow_rate")
                        .long("max-borrow-rate")
                        .validator(is_parsable::<u8>)
                        .value_name("INTEGER_PERCENT")
                        .takes_value(true)
                        .required(true)
                        .default_value("30")
                        .help("Max borrow APY: min <= optimal <= max"),
                )
                .arg(
                    Arg::with_name("borrow_fee")
                        .long("borrow-fee")
                        .validator(is_parsable::<f64>)
                        .value_name("DECIMAL_PERCENT")
                        .takes_value(true)
                        .required(true)
                        .default_value("0.00001")
                        .help("Fee assessed on borrow, expressed as a percentage: [0, 1)"),
                )
                .arg(
                    Arg::with_name("flash_loan_fee")
                        .long("flash-loan-fee")
                        .validator(is_parsable::<f64>)
                        .value_name("DECIMAL_PERCENT")
                        .takes_value(true)
                        .required(true)
                        .default_value(".3")
                        .help("Fee assessed for flash loans, expressed as a percentage: [0, 1)"),
                )
                .arg(
                    Arg::with_name("host_fee_percentage")
                        .long("host-fee-percentage")
                        .validator(is_parsable::<u8>)
                        .value_name("INTEGER_PERCENT")
                        .takes_value(true)
                        .required(true)
                        .default_value("20")
                        .help("Amount of fee going to host account: [0, 100]"),
                )
        )
        .get_matches();

    let mut wallet_manager = None;
    let config = {
        let cli_config = if let Some(config_file) = matches.value_of("config_file") {
            solana_cli_config::Config::load(config_file).unwrap_or_default()
        } else {
            solana_cli_config::Config::default()
        };
        let json_rpc_url = value_t!(matches, "json_rpc_url", String)
            .unwrap_or_else(|_| cli_config.json_rpc_url.clone());

        let fee_payer = signer_from_path(
            &matches,
            matches
                .value_of("fee_payer")
                .unwrap_or(&cli_config.keypair_path),
            "fee_payer",
            &mut wallet_manager,
        )
        .unwrap_or_else(|e| {
            eprintln!("error: {}", e);
            exit(1);
        });

        let lending_program_id = pubkey_of(&matches, "lending_program_id").unwrap();
        let verbose = matches.is_present("verbose");
        let dry_run = matches.is_present("dry_run");

        Config {
            rpc_client: RpcClient::new_with_commitment(json_rpc_url, CommitmentConfig::confirmed()),
            fee_payer,
            lending_program_id,
            verbose,
            dry_run,
        }
    };

    let _ = match matches.subcommand() {
        ("create-market", Some(arg_matches)) => {
            let lending_market_owner = pubkey_of(arg_matches, "lending_market_owner").unwrap();
            let quote_currency = quote_currency_of(arg_matches, "quote_currency").unwrap();
            let oracle_program_id = pubkey_of(arg_matches, "oracle_program_id").unwrap();
            command_create_lending_market(
                &config,
                lending_market_owner,
                quote_currency,
                oracle_program_id,
            )
        }
        ("add-reserve", Some(arg_matches)) => {
            let lending_market_owner_keypair =
                keypair_of(arg_matches, "lending_market_owner").unwrap();
            let source_liquidity_owner_keypair =
                keypair_of(arg_matches, "source_liquidity_owner").unwrap();
            let lending_market_pubkey = pubkey_of(arg_matches, "lending_market").unwrap();
            let source_liquidity_pubkey = pubkey_of(arg_matches, "source_liquidity").unwrap();
            let ui_amount = value_of(arg_matches, "liquidity_amount").unwrap();
            let pyth_product_pubkey = pubkey_of(arg_matches, "pyth_product").unwrap();
            let pyth_price_pubkey = pubkey_of(arg_matches, "pyth_price").unwrap();
            let optimal_utilization_rate =
                value_of(arg_matches, "optimal_utilization_rate").unwrap();
            let loan_to_value_ratio = value_of(arg_matches, "loan_to_value_ratio").unwrap();
            let liquidation_bonus = value_of(arg_matches, "liquidation_bonus").unwrap();
            let liquidation_threshold = value_of(arg_matches, "liquidation_threshold").unwrap();
            let min_borrow_rate = value_of(arg_matches, "min_borrow_rate").unwrap();
            let optimal_borrow_rate = value_of(arg_matches, "optimal_borrow_rate").unwrap();
            let max_borrow_rate = value_of(arg_matches, "max_borrow_rate").unwrap();
            let borrow_fee = value_of::<f64>(arg_matches, "borrow_fee").unwrap();
            let flash_loan_fee = value_of::<f64>(arg_matches, "flash_loan_fee").unwrap();
            let host_fee_percentage = value_of(arg_matches, "host_fee_percentage").unwrap();

            let borrow_fee_wad = (borrow_fee * WAD as f64) as u64;
            let flash_loan_fee_wad = (flash_loan_fee * WAD as f64) as u64;

            command_add_reserve(
                &config,
                ui_amount,
                ReserveConfig {
                    optimal_utilization_rate,
                    loan_to_value_ratio,
                    liquidation_bonus,
                    liquidation_threshold,
                    min_borrow_rate,
                    optimal_borrow_rate,
                    max_borrow_rate,
                    fees: ReserveFees {
                        borrow_fee_wad,
                        flash_loan_fee_wad,
                        host_fee_percentage,
                    },
                },
                source_liquidity_pubkey,
                source_liquidity_owner_keypair,
                lending_market_pubkey,
                lending_market_owner_keypair,
                pyth_product_pubkey,
                pyth_price_pubkey,
            )
        }
        _ => unreachable!(),
    }
    .map_err(|err| {
        eprintln!("{}", err);
        exit(1);
    });
}

// COMMANDS

fn command_create_lending_market(
    config: &Config,
    lending_market_owner: Pubkey,
    quote_currency: [u8; 32],
    oracle_program_id: Pubkey,
) -> CommandResult {
    let lending_market_keypair = Keypair::new();
    println!(
        "Creating lending market {}",
        lending_market_keypair.pubkey()
    );

    let lending_market_balance = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(LendingMarket::LEN)?;

    let mut transaction = Transaction::new_with_payer(
        &[
            // Account for the lending market
            create_account(
                &config.fee_payer.pubkey(),
                &lending_market_keypair.pubkey(),
                lending_market_balance,
                LendingMarket::LEN as u64,
                &config.lending_program_id,
            ),
            // Initialize lending market account
            init_lending_market(
                config.lending_program_id,
                lending_market_owner,
                quote_currency,
                lending_market_keypair.pubkey(),
                oracle_program_id,
            ),
        ],
        Some(&config.fee_payer.pubkey()),
    );

    let (recent_blockhash, fee_calculator) = config.rpc_client.get_recent_blockhash()?;
    check_fee_payer_balance(
        config,
        lending_market_balance + fee_calculator.calculate_fee(transaction.message()),
    )?;
    transaction.sign(
        &vec![config.fee_payer.as_ref(), &lending_market_keypair],
        recent_blockhash,
    );
    send_transaction(config, transaction)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn command_add_reserve(
    config: &Config,
    ui_amount: f64,
    reserve_config: ReserveConfig,
    source_liquidity_pubkey: Pubkey,
    source_liquidity_owner_keypair: Keypair,
    lending_market_pubkey: Pubkey,
    lending_market_owner_keypair: Keypair,
    pyth_product_pubkey: Pubkey,
    pyth_price_pubkey: Pubkey,
) -> CommandResult {
    let source_liquidity_account = config.rpc_client.get_account(&source_liquidity_pubkey)?;
    let source_liquidity = Token::unpack_from_slice(source_liquidity_account.data.borrow())?;

    let source_liquidity_mint_account = config.rpc_client.get_account(&source_liquidity.mint)?;
    let source_liquidity_mint =
        Mint::unpack_from_slice(source_liquidity_mint_account.data.borrow())?;
    let liquidity_amount = ui_amount_to_amount(ui_amount, source_liquidity_mint.decimals);

    let reserve_keypair = Keypair::new();
    let collateral_mint_keypair = Keypair::new();
    let collateral_supply_keypair = Keypair::new();
    let liquidity_supply_keypair = Keypair::new();
    let liquidity_fee_receiver_keypair = Keypair::new();
    let user_collateral_keypair = Keypair::new();
    let user_transfer_authority_keypair = Keypair::new();

    println!("Adding reserve {}", reserve_keypair.pubkey());
    if config.verbose {
        println!(
            "Adding collateral mint {}",
            collateral_mint_keypair.pubkey()
        );
        println!(
            "Adding collateral supply {}",
            collateral_supply_keypair.pubkey()
        );
        println!(
            "Adding liquidity supply {}",
            liquidity_supply_keypair.pubkey()
        );
        println!(
            "Adding liquidity fee receiver {}",
            liquidity_fee_receiver_keypair.pubkey()
        );
        println!(
            "Adding user collateral {}",
            user_collateral_keypair.pubkey()
        );
        println!(
            "Adding user transfer authority {}",
            user_transfer_authority_keypair.pubkey()
        );
    }

    let reserve_balance = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(Reserve::LEN)?;
    let collateral_mint_balance = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(Mint::LEN)?;
    let token_account_balance = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(Token::LEN)?;
    let collateral_supply_balance = token_account_balance;
    let user_collateral_balance = token_account_balance;
    let liquidity_supply_balance = token_account_balance;
    let liquidity_fee_receiver_balance = token_account_balance;

    let total_balance = reserve_balance
        + collateral_mint_balance
        + collateral_supply_balance
        + user_collateral_balance
        + liquidity_supply_balance
        + liquidity_fee_receiver_balance;

    let mut transaction_1 = Transaction::new_with_payer(
        &[
            create_account(
                &config.fee_payer.pubkey(),
                &reserve_keypair.pubkey(),
                reserve_balance,
                Reserve::LEN as u64,
                &config.lending_program_id,
            ),
            create_account(
                &config.fee_payer.pubkey(),
                &collateral_mint_keypair.pubkey(),
                collateral_mint_balance,
                Mint::LEN as u64,
                &spl_token::id(),
            ),
            create_account(
                &config.fee_payer.pubkey(),
                &collateral_supply_keypair.pubkey(),
                collateral_supply_balance,
                Token::LEN as u64,
                &spl_token::id(),
            ),
            create_account(
                &config.fee_payer.pubkey(),
                &user_collateral_keypair.pubkey(),
                user_collateral_balance,
                Token::LEN as u64,
                &spl_token::id(),
            ),
        ],
        Some(&config.fee_payer.pubkey()),
    );

    let mut transaction_2 = Transaction::new_with_payer(
        &[
            create_account(
                &config.fee_payer.pubkey(),
                &liquidity_supply_keypair.pubkey(),
                liquidity_supply_balance,
                Token::LEN as u64,
                &spl_token::id(),
            ),
            create_account(
                &config.fee_payer.pubkey(),
                &liquidity_fee_receiver_keypair.pubkey(),
                liquidity_fee_receiver_balance,
                Token::LEN as u64,
                &spl_token::id(),
            ),
        ],
        Some(&config.fee_payer.pubkey()),
    );

    let mut transaction_3 = Transaction::new_with_payer(
        &[
            approve(
                &spl_token::id(),
                &source_liquidity_pubkey,
                &user_transfer_authority_keypair.pubkey(),
                &source_liquidity_owner_keypair.pubkey(),
                &[],
                liquidity_amount,
            )
            .unwrap(),
            init_reserve(
                config.lending_program_id,
                liquidity_amount,
                reserve_config,
                source_liquidity_pubkey,
                user_collateral_keypair.pubkey(),
                reserve_keypair.pubkey(),
                source_liquidity.mint,
                liquidity_supply_keypair.pubkey(),
                liquidity_fee_receiver_keypair.pubkey(),
                collateral_mint_keypair.pubkey(),
                collateral_supply_keypair.pubkey(),
                pyth_product_pubkey,
                pyth_price_pubkey,
                lending_market_pubkey,
                lending_market_owner_keypair.pubkey(),
                user_transfer_authority_keypair.pubkey(),
            ),
            revoke(
                &spl_token::id(),
                &source_liquidity_pubkey,
                &source_liquidity_owner_keypair.pubkey(),
                &[],
            )
            .unwrap(),
        ],
        Some(&config.fee_payer.pubkey()),
    );

    let (recent_blockhash, fee_calculator) = config.rpc_client.get_recent_blockhash()?;
    check_fee_payer_balance(
        config,
        total_balance
            + fee_calculator.calculate_fee(transaction_1.message())
            + fee_calculator.calculate_fee(transaction_2.message())
            + fee_calculator.calculate_fee(transaction_3.message()),
    )?;
    transaction_1.sign(
        &vec![
            config.fee_payer.as_ref(),
            &reserve_keypair,
            &collateral_mint_keypair,
            &collateral_supply_keypair,
            &user_collateral_keypair,
        ],
        recent_blockhash,
    );
    transaction_2.sign(
        &vec![
            config.fee_payer.as_ref(),
            &liquidity_supply_keypair,
            &liquidity_fee_receiver_keypair,
        ],
        recent_blockhash,
    );
    transaction_3.sign(
        &vec![
            config.fee_payer.as_ref(),
            &source_liquidity_owner_keypair,
            &lending_market_owner_keypair,
            &user_transfer_authority_keypair,
        ],
        recent_blockhash,
    );
    send_transaction(config, transaction_1)?;
    send_transaction(config, transaction_2)?;
    send_transaction(config, transaction_3)?;
    Ok(())
}

// HELPERS

fn check_fee_payer_balance(config: &Config, required_balance: u64) -> Result<(), Error> {
    let balance = config.rpc_client.get_balance(&config.fee_payer.pubkey())?;
    if balance < required_balance {
        Err(format!(
            "Fee payer, {}, has insufficient balance: {} required, {} available",
            config.fee_payer.pubkey(),
            lamports_to_sol(required_balance),
            lamports_to_sol(balance)
        )
        .into())
    } else {
        Ok(())
    }
}

fn send_transaction(
    config: &Config,
    transaction: Transaction,
) -> solana_client::client_error::Result<()> {
    if config.dry_run {
        let result = config.rpc_client.simulate_transaction(&transaction)?;
        println!("Simulate result: {:?}", result);
    } else {
        let signature = config
            .rpc_client
            .send_and_confirm_transaction_with_spinner(&transaction)?;
        println!("Signature: {}", signature);
    }
    Ok(())
}

fn quote_currency_of(matches: &ArgMatches<'_>, name: &str) -> Option<[u8; 32]> {
    if let Some(value) = matches.value_of(name) {
        if value == "USD" {
            Some(*b"USD\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0")
        } else if value.len() <= 32 {
            let mut bytes32 = [0u8; 32];
            bytes32[0..value.len()].clone_from_slice(value.as_bytes());
            Some(bytes32)
        } else {
            Some(Pubkey::from_str(value).unwrap().to_bytes())
        }
    } else {
        None
    }
}
