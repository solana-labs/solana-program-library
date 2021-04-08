use solana_program::entrypoint::ProgramResult;
use solana_sdk::signature::Keypair;

use {
    clap::{crate_description, crate_name, crate_version, App, Arg, ArgMatches, SubCommand},
    solana_clap_utils::{
        input_parsers::pubkey_of,
        input_validators::{is_url, is_valid_pubkey, is_valid_signer},
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{borsh::try_from_slice_unchecked, program_pack::Pack},
    solana_sdk::{
        pubkey::Pubkey,
        signature::{read_keypair_file, Signer},
        system_instruction::create_account,
        transaction::Transaction,
    },
    spl_token::{
        instruction::{initialize_account, initialize_mint},
        state::{Account, Mint},
    },
    spl_token_vault::{
        instruction::{
            create_init_vault_instruction, create_update_external_price_account_instruction,
        },
        state::{MAX_EXTERNAL_ACCOUNT_SIZE, PREFIX},
    },
    std::str::FromStr,
};

const PROGRAM_PUBKEY: &str = "metaTA73sFPqA8whreUbBsbn3SLJH2vhrW9fP5dmfdC";
const TOKEN_PROGRAM_PUBKEY: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

fn initialize_vault(app_matches: &ArgMatches, payer: Keypair, client: RpcClient) -> Pubkey {
    let program_key = Pubkey::from_str(PROGRAM_PUBKEY).unwrap();
    let token_key = Pubkey::from_str(TOKEN_PROGRAM_PUBKEY).unwrap();
    let vault_authority =
        pubkey_of(app_matches, "vault_authority").unwrap_or_else(|| payer.pubkey());
    let external_account = pubkey_of(app_matches, "external_price_account").unwrap();
    let fraction_mint = Keypair::new();
    let redeem_mint = Keypair::new();
    let redeem_treasury = Keypair::new();
    let fraction_treasury = Keypair::new();
    let vault = Keypair::new();
    let allow_further_share_creation = app_matches.is_present("allow_further_share_creation");

    let instructions = [
        create_account(
            &payer.pubkey(),
            &fraction_mint.pubkey(),
            client
                .get_minimum_balance_for_rent_exemption(Mint::LEN)
                .unwrap(),
            Mint::LEN as u64,
            &token_key,
        ),
        create_account(
            &payer.pubkey(),
            &redeem_mint.pubkey(),
            client
                .get_minimum_balance_for_rent_exemption(Mint::LEN)
                .unwrap(),
            Mint::LEN as u64,
            &token_key,
        ),
        create_account(
            &payer.pubkey(),
            &redeem_treasury.pubkey(),
            client
                .get_minimum_balance_for_rent_exemption(Account::LEN)
                .unwrap(),
            Account::LEN as u64,
            &token_key,
        ),
        create_account(
            &payer.pubkey(),
            &fraction_treasury.pubkey(),
            client
                .get_minimum_balance_for_rent_exemption(Account::LEN)
                .unwrap(),
            Account::LEN as u64,
            &token_key,
        ),
        create_account(
            &payer.pubkey(),
            &vault.pubkey(),
            client
                .get_minimum_balance_for_rent_exemption(Account::LEN)
                .unwrap(),
            Account::LEN as u64,
            &token_key,
        ),
        initialize_mint(
            &token_key,
            &redeem_mint.pubkey(),
            &payer.pubkey(),
            Some(&payer.pubkey()),
            0,
        )
        .unwrap(),
        initialize_mint(
            &token_key,
            &fraction_mint.pubkey(),
            &payer.pubkey(),
            Some(&payer.pubkey()),
            0,
        )
        .unwrap(),
        initialize_account(
            &token_key,
            &redeem_treasury.pubkey(),
            &redeem_mint.pubkey(),
            &program_key,
        )
        .unwrap(),
        initialize_account(
            &token_key,
            &fraction_treasury.pubkey(),
            &fraction_mint.pubkey(),
            &program_key,
        )
        .unwrap(),
        create_init_vault_instruction(
            program_key,
            fraction_mint.pubkey(),
            redeem_treasury.pubkey(),
            fraction_treasury.pubkey(),
            vault.pubkey(),
            vault_authority,
            external_account,
            allow_further_share_creation,
        ),
    ];
    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    let signers = vec![
        &payer,
        &redeem_treasury,
        &redeem_mint,
        &fraction_treasury,
        &fraction_mint,
        &vault,
    ];

    transaction.sign(&signers, recent_blockhash);
    client.send_and_confirm_transaction(&transaction).unwrap();
    let _account = client.get_account(&vault.pubkey()).unwrap();
    vault.pubkey()
}

fn rewrite_price_account(app_matches: &ArgMatches, payer: Keypair, client: RpcClient) -> Pubkey {
    let program_key = Pubkey::from_str(PROGRAM_PUBKEY).unwrap();
    let external_account =
        read_keypair_file(app_matches.value_of("external_price_account").unwrap()).unwrap();
    let price_mint = pubkey_of(app_matches, "price_mint").unwrap_or_else(|| payer.pubkey());
    let price_per_share: u64 = app_matches
        .value_of("price_per_share")
        .unwrap_or_else(|| "0")
        .parse::<u64>()
        .unwrap();
    let allowed_to_combine = app_matches.is_present("allowed_to_combine");
    let already_created = app_matches.is_present("already_created");

    let mut instructions = vec![];

    if !already_created {
        instructions.push(create_account(
            &payer.pubkey(),
            &external_account.pubkey(),
            client
                .get_minimum_balance_for_rent_exemption(MAX_EXTERNAL_ACCOUNT_SIZE)
                .unwrap(),
            MAX_EXTERNAL_ACCOUNT_SIZE as u64,
            &payer.pubkey(),
        ));
    }

    instructions.push(create_update_external_price_account_instruction(
        program_key,
        external_account.pubkey(),
        price_per_share,
        price_mint,
        allowed_to_combine,
    ));

    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    let signers = vec![&payer, &external_account];

    transaction.sign(&signers, recent_blockhash);
    client.send_and_confirm_transaction(&transaction).unwrap();
    let _account = client.get_account(&external_account.pubkey()).unwrap();
    external_account.pubkey()
}

fn main() {
    let app_matches = App::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .arg(
            Arg::with_name("keypair")
                .long("keypair")
                .value_name("KEYPAIR")
                .required(true)
                .validator(is_valid_signer)
                .takes_value(true)
                .global(true)
                .help("Filepath or URL to a keypair"),
        )
        .arg(
            Arg::with_name("json_rpc_url")
                .long("url")
                .value_name("URL")
                .takes_value(true)
                .global(true)
                .validator(is_url)
                .help("JSON RPC URL for the cluster [default: devnet]"),
        )
        .subcommand(
            SubCommand::with_name("init")
                .about("Initialize a Vault")
                .arg(
                    Arg::with_name("vault_authority")
                        .long("vault_authority")
                        .value_name("VAULT_AUTHORITY")
                        .required(false)
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help("Pubkey of authority, defaults to you otherwise"),
                )
                .arg(
                    Arg::with_name("external_price_account")
                        .long("external_price_account")
                        .value_name("EXTERNAL_PRICE_ACCOUNT")
                        .required(true)
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help("Pubkey of external price account"),
                )
                .arg(
                    Arg::with_name("allow_further_share_creation")
                        .long("allow_further_share_creation")
                        .value_name("ALLOW_FURTHER_SHARE_CREATION")
                        .takes_value(false)
                        .required(false)
                        .help("Allows further share creation after activation of vault"),
                ),
        )
        .subcommand(
            SubCommand::with_name("external_price_account_rewrite")
                .about("Rewrite (or create) an External Price Account")
                .arg(
                    Arg::with_name("Authority")
                        .long("authority")
                        .value_name("AUTHORITY")
                        .required(false)
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help("Pubkey of authority, defaults to you otherwise"),
                )
                .arg(
                    Arg::with_name("external_price_account")
                        .long("external_price_account")
                        .value_name("EXTERNAL_PRICE_ACCOUNT")
                        .required(true)
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help("Pubkey of external price account"),
                )
                .arg(
                    Arg::with_name("price_mint")
                        .long("price_mint")
                        .value_name("PRICE_MINT")
                        .takes_value(true)
                        .validator(is_valid_pubkey)
                        .required(false)
                        .help("Price mint that price per share uses"),
                )
                .arg(
                    Arg::with_name("price_per_share")
                        .long("price_per_share")
                        .value_name("PRICE_PER_SHARE")
                        .takes_value(true)
                        .required(false)
                        .help("Price per share"),
                )
                .arg(
                    Arg::with_name("allowed_to_combine")
                        .long("allowed_to_combine")
                        .value_name("ALLOWED_TO_COMBINE")
                        .takes_value(false)
                        .required(false)
                        .help("Whether or not combination is allowed in the vault"),
                )
                .arg(
                    Arg::with_name("already_created")
                        .long("already_created")
                        .value_name("ALREADY_CREATED")
                        .takes_value(false)
                        .required(false)
                        .help("If we should skip creation because this account already exists"),
                ),
        )
        .get_matches();

    let client = RpcClient::new(
        app_matches
            .value_of("json_rpc_url")
            .unwrap_or(&"https://devnet.solana.com".to_owned())
            .to_owned(),
    );

    let (sub_command, sub_matches) = app_matches.subcommand();

    let payer = read_keypair_file(app_matches.value_of("keypair").unwrap()).unwrap();

    match (sub_command, sub_matches) {
        ("init", Some(arg_matches)) => {
            println!(
                "Created vault with address {:?}",
                initialize_vault(arg_matches, payer, client)
            );
        }
        ("external_price_account_rewrite", Some(arg_matches)) => {
            println!(
                "Rewrote price account {:?}",
                rewrite_price_account(arg_matches, payer, client)
            );
        }
        _ => unreachable!(),
    }
}
