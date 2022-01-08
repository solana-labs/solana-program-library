use {
    chrono::{DateTime, NaiveDateTime, SecondsFormat, Utc},
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
    spl_feature_proposal::state::{AcceptanceCriteria, FeatureProposal},
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
        .subcommand(
            SubCommand::with_name("address")
                .about("Display address information for the feature proposal")
                .arg(
                    Arg::with_name("feature_proposal")
                        .value_name("FEATURE_PROPOSAL_ADDRESS")
                        .validator(is_valid_pubkey)
                        .index(1)
                        .required(true)
                        .help("The address of the feature proposal"),
                ),
        )
        .subcommand(
            SubCommand::with_name("propose")
                .about("Initiate a feature proposal")
                .arg(
                    Arg::with_name("feature_proposal")
                        .value_name("FEATURE_PROPOSAL_KEYPAIR")
                        .validator(is_keypair)
                        .index(1)
                        .required(true)
                        .help("The keypair of the feature proposal"),
                )
                .arg(
                    Arg::with_name("percent_stake_required")
                        .long("percent-stake-required")
                        .value_name("PERCENTAGE")
                        .validator(is_valid_percentage)
                        .required(true)
                        .default_value("67")
                        .help("Percentage of the active stake required for the proposal to pass"),
                )
                .arg(
                    Arg::with_name("distribution_file")
                        .long("distribution-file")
                        .value_name("FILENAME")
                        .required(true)
                        .default_value("feature-proposal.csv")
                        .help("Allocations CSV file for use with solana-tokens"),
                )
                .arg(
                    Arg::with_name("confirm")
                        .long("confirm")
                        .help("Confirm that the feature proposal should actually be initiated"),
                ),
        )
        .subcommand(
            SubCommand::with_name("tally")
                .about("Tally the current results for a proposed feature")
                .arg(
                    Arg::with_name("feature_proposal")
                        .value_name("FEATURE_PROPOSAL_ADDRESS")
                        .validator(is_valid_pubkey)
                        .index(1)
                        .required(true)
                        .help("The address of the feature proposal"),
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
        ("address", Some(arg_matches)) => {
            let feature_proposal_address = pubkey_of(arg_matches, "feature_proposal").unwrap();

            println!(
                "Feature Id: {}",
                spl_feature_proposal::get_feature_id_address(&feature_proposal_address)
            );
            println!(
                "Token Mint Address: {}",
                spl_feature_proposal::get_mint_address(&feature_proposal_address)
            );
            println!(
                "Acceptance Token Address: {}",
                spl_feature_proposal::get_acceptance_token_address(&feature_proposal_address)
            );

            Ok(())
        }
        ("propose", Some(arg_matches)) => {
            let feature_proposal_keypair = keypair_of(arg_matches, "feature_proposal").unwrap();
            let distribution_file = value_t_or_exit!(arg_matches, "distribution_file", String);
            let percent_stake_required =
                value_t_or_exit!(arg_matches, "percent_stake_required", u8);

            // Hard code deadline for now...
            let fortnight = Duration::from_secs(60 * 60 * 24 * 14);
            let deadline = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .checked_add(fortnight)
                .unwrap()
                .as_secs() as UnixTimestamp;

            process_propose(
                &rpc_client,
                &config,
                &feature_proposal_keypair,
                distribution_file,
                percent_stake_required,
                deadline,
                arg_matches.is_present("confirm"),
            )
        }
        ("tally", Some(arg_matches)) => {
            if config.verbose {
                println!("JSON RPC URL: {}", config.json_rpc_url);
            }

            let feature_proposal_address = pubkey_of(arg_matches, "feature_proposal").unwrap();
            process_tally(&rpc_client, &config, &feature_proposal_address)
        }
        _ => unreachable!(),
    }
}

fn get_feature_proposal(
    rpc_client: &RpcClient,
    feature_proposal_address: &Pubkey,
) -> Result<FeatureProposal, String> {
    let account = rpc_client
        .get_multiple_accounts(&[*feature_proposal_address])
        .map_err(|err| err.to_string())?
        .into_iter()
        .next()
        .unwrap();

    match account {
        None => Err(format!(
            "Feature proposal {} does not exist",
            feature_proposal_address
        )),
        Some(account) => FeatureProposal::unpack_from_slice(&account.data).map_err(|err| {
            format!(
                "Failed to deserialize feature proposal {}: {}",
                feature_proposal_address, err
            )
        }),
    }
}

fn unix_timestamp_to_string(unix_timestamp: UnixTimestamp) -> String {
    format!(
        "{} (UnixTimestamp: {})",
        match NaiveDateTime::from_timestamp_opt(unix_timestamp, 0) {
            Some(ndt) =>
                DateTime::<Utc>::from_utc(ndt, Utc).to_rfc3339_opts(SecondsFormat::Secs, true),
            None => "unknown".to_string(),
        },
        unix_timestamp,
    )
}

fn process_propose(
    rpc_client: &RpcClient,
    config: &Config,
    feature_proposal_keypair: &Keypair,
    distribution_file: String,
    percent_stake_required: u8,
    deadline: UnixTimestamp,
    confirm: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let distributor_token_address =
        spl_feature_proposal::get_distributor_token_address(&feature_proposal_keypair.pubkey());
    let feature_id_address =
        spl_feature_proposal::get_feature_id_address(&feature_proposal_keypair.pubkey());
    let acceptance_token_address =
        spl_feature_proposal::get_acceptance_token_address(&feature_proposal_keypair.pubkey());
    let mint_address = spl_feature_proposal::get_mint_address(&feature_proposal_keypair.pubkey());

    println!("Feature Id: {}", feature_id_address);
    println!("Token Mint Address: {}", mint_address);
    println!("Distributor Token Address: {}", distributor_token_address);
    println!("Acceptance Token Address: {}", acceptance_token_address);

    let vote_accounts = rpc_client.get_vote_accounts()?;
    let mut distribution = HashMap::new();
    for (pubkey, activated_stake) in vote_accounts
        .current
        .into_iter()
        .chain(vote_accounts.delinquent)
        .map(|vote_account| (vote_account.node_pubkey, vote_account.activated_stake))
    {
        distribution
            .entry(pubkey)
            .and_modify(|e| *e += activated_stake)
            .or_insert(activated_stake);
    }

    let tokens_to_mint: u64 = distribution.iter().map(|x| x.1).sum();
    let tokens_required = tokens_to_mint * percent_stake_required as u64 / 100;

    println!("Number of validators: {}", distribution.len());
    println!(
        "Tokens to be minted: {}",
        spl_feature_proposal::amount_to_ui_amount(tokens_to_mint)
    );
    println!(
        "Tokens required for acceptance: {} ({}%)",
        spl_feature_proposal::amount_to_ui_amount(tokens_required),
        percent_stake_required
    );

    println!("Token distribution file: {}", distribution_file);
    {
        let mut file = File::create(&distribution_file)?;
        file.write_all(b"recipient,amount\n")?;
        for (node_address, activated_stake) in distribution.iter() {
            file.write_all(format!("{},{}\n", node_address, activated_stake).as_bytes())?;
        }
    }

    let mut transaction = Transaction::new_with_payer(
        &[spl_feature_proposal::instruction::propose(
            &config.keypair.pubkey(),
            &feature_proposal_keypair.pubkey(),
            tokens_to_mint,
            AcceptanceCriteria {
                tokens_required,
                deadline,
            },
        )],
        Some(&config.keypair.pubkey()),
    );
    let blockhash = rpc_client.get_latest_blockhash()?;
    transaction.try_sign(&[&config.keypair, feature_proposal_keypair], blockhash)?;

    println!("JSON RPC URL: {}", config.json_rpc_url);

    println!();
    println!("Distribute the proposal tokens to all validators by running:");
    println!(
        "    $ solana-tokens distribute-spl-tokens \
                  --from {} \
                  --input-csv {} \
                  --db-path db.{} \
                  --fee-payer ~/.config/solana/id.json \
                  --owner <FEATURE_PROPOSAL_KEYPAIR>",
        distributor_token_address,
        distribution_file,
        &feature_proposal_keypair.pubkey().to_string()[..8]
    );
    println!(
        "    $ solana-tokens spl-token-balances \
                 --mint {} --input-csv {}",
        mint_address, distribution_file
    );
    println!();

    println!(
        "Once the distribution is complete, request validators vote for \
        the proposal by first looking up their token account address:"
    );
    println!(
        "    $ spl-token --owner ~/validator-keypair.json accounts {}",
        mint_address
    );
    println!("and then submit their vote by running:");
    println!(
        "    $ spl-token --owner ~/validator-keypair.json transfer <TOKEN_ACCOUNT_ADDRESS> ALL {}",
        acceptance_token_address
    );
    println!();
    println!("Periodically the votes must be tallied by running:");
    println!(
        "  $ spl-feature-proposal tally {}",
        feature_proposal_keypair.pubkey()
    );
    println!("Tallying is permissionless and may be run by anybody.");
    println!("Once this feature proposal is accepted, the {} feature will be activated at the next epoch.", feature_id_address);

    println!();
    println!(
        "Proposal will expire at {}",
        unix_timestamp_to_string(deadline)
    );
    println!();
    if !confirm {
        println!("Add --confirm flag to initiate the feature proposal");
        return Ok(());
    }
    rpc_client.send_and_confirm_transaction_with_spinner(&transaction)?;

    println!();
    println!("Feature proposal created!");
    Ok(())
}

fn process_tally(
    rpc_client: &RpcClient,
    config: &Config,
    feature_proposal_address: &Pubkey,
) -> Result<(), Box<dyn std::error::Error>> {
    let feature_proposal = get_feature_proposal(rpc_client, feature_proposal_address)?;

    let feature_id_address = spl_feature_proposal::get_feature_id_address(feature_proposal_address);
    let acceptance_token_address =
        spl_feature_proposal::get_acceptance_token_address(feature_proposal_address);

    println!("Feature Id: {}", feature_id_address);
    println!("Acceptance Token Address: {}", acceptance_token_address);

    match feature_proposal {
        FeatureProposal::Uninitialized => {
            return Err("Feature proposal is uninitialized".into());
        }
        FeatureProposal::Pending(acceptance_criteria) => {
            let acceptance_token_address =
                spl_feature_proposal::get_acceptance_token_address(feature_proposal_address);
            let acceptance_token_balance = rpc_client
                .get_token_account_balance(&acceptance_token_address)?
                .amount
                .parse::<u64>()
                .unwrap_or(0);

            println!();
            println!(
                "{} tokens required to accept the proposal",
                spl_feature_proposal::amount_to_ui_amount(acceptance_criteria.tokens_required)
            );
            println!(
                "{} tokens have been received",
                spl_feature_proposal::amount_to_ui_amount(acceptance_token_balance)
            );
            println!(
                "Proposal will expire at {}",
                unix_timestamp_to_string(acceptance_criteria.deadline)
            );
            println!();

            // Don't bother issuing a transaction if it's clear the Tally won't succeed
            if acceptance_token_balance < acceptance_criteria.tokens_required
                && (SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as UnixTimestamp)
                    < acceptance_criteria.deadline
            {
                println!("Feature proposal pending");
                return Ok(());
            }
        }
        FeatureProposal::Accepted { .. } => {
            println!("Feature proposal accepted");
            return Ok(());
        }
        FeatureProposal::Expired => {
            println!("Feature proposal expired");
            return Ok(());
        }
    }

    let mut transaction = Transaction::new_with_payer(
        &[spl_feature_proposal::instruction::tally(
            feature_proposal_address,
        )],
        Some(&config.keypair.pubkey()),
    );
    let blockhash = rpc_client.get_latest_blockhash()?;
    transaction.try_sign(&[&config.keypair], blockhash)?;

    rpc_client.send_and_confirm_transaction_with_spinner(&transaction)?;

    // Check the status of the proposal after the tally completes
    let feature_proposal = get_feature_proposal(rpc_client, feature_proposal_address)?;
    match feature_proposal {
        FeatureProposal::Uninitialized => Err("Feature proposal is uninitialized".into()),
        FeatureProposal::Pending { .. } => {
            println!("Feature proposal pending");
            Ok(())
        }
        FeatureProposal::Accepted { .. } => {
            println!("Feature proposal accepted");
            Ok(())
        }
        FeatureProposal::Expired => {
            println!("Feature proposal expired");
            Ok(())
        }
    }
}
