use {
    clap::{crate_description, crate_name, crate_version, Arg, Command},
    solana_clap_v3_utils::{
        input_parsers::{pubkey_of, pubkey_of_signer},
        input_validators::{
            is_url_or_moniker, is_valid_pubkey, is_valid_signer, normalize_to_url_if_moniker,
        },
        keypair::DefaultSigner,
    },
    solana_client::nonblocking::rpc_client::RpcClient,
    solana_remote_wallet::remote_wallet::RemoteWalletManager,
    solana_sdk::{
        commitment_config::CommitmentConfig,
        instruction::{AccountMeta, Instruction},
        message::Message,
        pubkey::Pubkey,
        signature::{Signature, Signer},
        transaction::Transaction,
    },
    std::{process::exit, sync::Arc},
};

struct Config {
    commitment_config: CommitmentConfig,
    default_signer: Box<dyn Signer>,
    json_rpc_url: String,
    verbose: bool,
}

async fn process_give(
    rpc_client: &RpcClient,
    signer: &dyn Signer,
    program_id: &Pubkey,
    mint: &Pubkey,
    destination: &Pubkey,
) -> Result<Signature, Box<dyn std::error::Error>> {
    let token_program_id = spl_token::id();
    let (pda, _) = Pubkey::find_program_address(&[b"authority"], program_id);

    let source = spl_associated_token_account::get_associated_token_address_with_program_id(
        &pda,
        mint,
        &token_program_id,
    );

    let mut transaction = Transaction::new_unsigned(Message::new(
        &[Instruction::new_with_bincode(
            *program_id,
            &(),
            vec![
                AccountMeta::new(source, false),
                AccountMeta::new_readonly(*mint, false),
                AccountMeta::new(*destination, false),
                AccountMeta::new_readonly(pda, false),
                AccountMeta::new_readonly(token_program_id, false),
            ],
        )],
        Some(&signer.pubkey()),
    ));

    let blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|err| format!("error: unable to get latest blockhash: {err}"))?;

    transaction
        .try_sign(&vec![signer], blockhash)
        .map_err(|err| format!("error: failed to sign transaction: {err}"))?;

    let signature = rpc_client
        .send_and_confirm_transaction_with_spinner(&transaction)
        .await
        .map_err(|err| format!("error: send transaction: {err}"))?;

    Ok(signature)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app_matches = Command::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg({
            let arg = Arg::new("config_file")
                .short('C')
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
            Arg::new("keypair")
                .long("keypair")
                .value_name("KEYPAIR")
                .validator(|s| is_valid_signer(s))
                .takes_value(true)
                .global(true)
                .help("Filepath or URL to a keypair [default: client keypair]"),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .takes_value(false)
                .global(true)
                .help("Show additional information"),
        )
        .arg(
            Arg::new("json_rpc_url")
                .short('u')
                .long("url")
                .value_name("URL")
                .takes_value(true)
                .global(true)
                .validator(|s| is_url_or_moniker(s))
                .help("JSON RPC URL for the cluster [default: value from configuration file]"),
        )
        .subcommand(
            Command::new("pda")
                .about("Get program-derived address")
                .arg(
                    Arg::new("program_id")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("PROGRAM_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .help("Program id to derive the PDA for"),
                ),
        )
        .subcommand(
            Command::new("give")
                .about("Send a transaction to give all tokens back")
                .arg(
                    Arg::new("program_id")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("PROGRAM_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .help("Program id to target"),
                )
                .arg(
                    Arg::new("mint")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("MINT_ADDRESS")
                        .takes_value(true)
                        .index(2)
                        .help("Mint for which to transfer"),
                )
                .arg(
                    Arg::new("destination")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(3)
                        .help("Token account to receive the tokens"),
                ),
        )
        .get_matches();

    let (command, matches) = app_matches.subcommand().unwrap();
    let mut wallet_manager: Option<Arc<RemoteWalletManager>> = None;

    let config = {
        let cli_config = if let Some(config_file) = matches.value_of("config_file") {
            solana_cli_config::Config::load(config_file).unwrap_or_default()
        } else {
            solana_cli_config::Config::default()
        };

        let default_signer = DefaultSigner::new(
            "keypair",
            matches
                .value_of("keypair")
                .map(|s| s.to_string())
                .unwrap_or_else(|| cli_config.keypair_path.clone()),
        );

        let json_rpc_url = normalize_to_url_if_moniker(
            matches
                .value_of("json_rpc_url")
                .unwrap_or(&cli_config.json_rpc_url),
        );

        Config {
            commitment_config: CommitmentConfig::confirmed(),
            default_signer: default_signer
                .signer_from_path(matches, &mut wallet_manager)
                .unwrap_or_else(|err| {
                    eprintln!("error: {err}");
                    exit(1);
                }),
            json_rpc_url,
            verbose: matches.is_present("verbose"),
        }
    };
    solana_logger::setup_with_default("solana=info");

    if config.verbose {
        println!("JSON RPC URL: {}", config.json_rpc_url);
    }
    let rpc_client =
        RpcClient::new_with_commitment(config.json_rpc_url.clone(), config.commitment_config);

    match (command, matches) {
        ("pda", arg_matches) => {
            let program_id = pubkey_of(arg_matches, "program_id").unwrap();
            let (pda, _) = Pubkey::find_program_address(&[b"authority"], &program_id);
            println!("PDA is {pda} for program-id {program_id}");
        }
        ("give", arg_matches) => {
            let program_id = pubkey_of_signer(arg_matches, "program_id", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let mint = pubkey_of_signer(arg_matches, "mint", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let destination = pubkey_of_signer(arg_matches, "destination", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let signature = process_give(
                &rpc_client,
                config.default_signer.as_ref(),
                &program_id,
                &mint,
                &destination,
            )
            .await
            .unwrap_or_else(|err| {
                eprintln!("error: send transaction: {err}");
                exit(1);
            });
            println!("Signature: {signature}");
        }
        _ => unreachable!(),
    };

    Ok(())
}
