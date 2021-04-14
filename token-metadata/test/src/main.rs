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
        signature::{read_keypair_file, Keypair, Signer},
        system_instruction::create_account,
        transaction::Transaction,
    },
    spl_token::{instruction::initialize_mint, state::Mint},
    spl_token_metadata::{
        instruction::{
            create_metadata_accounts, transfer_update_authority, update_metadata_accounts,
        },
        state::{Metadata, PREFIX},
    },
    std::str::FromStr,
};

const METADATA_PROGRAM_PUBKEY: &str = "metaTA73sFPqA8whreUbBsbn3SLJH2vhrW9fP5dmfdC";
const TOKEN_PROGRAM_PUBKEY: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

fn transfer_authority_call(
    app_matches: &ArgMatches,
    payer: Keypair,
    client: RpcClient,
) -> (Metadata, Pubkey) {
    let update_authority = read_keypair_file(
        app_matches
            .value_of("update_authority")
            .unwrap_or_else(|| app_matches.value_of("keypair").unwrap()),
    )
    .unwrap();

    let program_key = Pubkey::from_str(METADATA_PROGRAM_PUBKEY).unwrap();
    let mint_key = pubkey_of(app_matches, "mint").unwrap();
    let new_update_authority = pubkey_of(app_matches, "new_update_authority").unwrap();

    let metadata_seeds = &[PREFIX.as_bytes(), &program_key.as_ref(), mint_key.as_ref()];
    let (metadata_key, _) = Pubkey::find_program_address(metadata_seeds, &program_key);

    let metadata_account = client.get_account(&metadata_key).unwrap();
    let metadata: Metadata = try_from_slice_unchecked(&metadata_account.data).unwrap();

    let name_symbol_seeds = &[
        PREFIX.as_bytes(),
        &program_key.as_ref(),
        &metadata.data.name.as_bytes(),
        &metadata.data.symbol.as_bytes(),
    ];
    let (name_symbol_key, _) = Pubkey::find_program_address(name_symbol_seeds, &program_key);

    let object = match metadata.non_unique_specific_update_authority {
        Some(_) => metadata_key,
        None => name_symbol_key,
    };

    let instructions = [transfer_update_authority(
        program_key,
        object,
        update_authority.pubkey(),
        new_update_authority,
    )];

    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    let signers = vec![&update_authority];

    transaction.sign(&signers, recent_blockhash);
    client.send_and_confirm_transaction(&transaction).unwrap();
    (metadata, metadata_key)
}

fn update_metadata_account_call(
    app_matches: &ArgMatches,
    payer: Keypair,
    client: RpcClient,
) -> (Metadata, Pubkey) {
    let update_authority = read_keypair_file(
        app_matches
            .value_of("update_authority")
            .unwrap_or_else(|| app_matches.value_of("keypair").unwrap()),
    )
    .unwrap();
    let program_key = Pubkey::from_str(METADATA_PROGRAM_PUBKEY).unwrap();
    let mint_key = pubkey_of(app_matches, "mint").unwrap();
    let metadata_seeds = &[PREFIX.as_bytes(), &program_key.as_ref(), mint_key.as_ref()];
    let (metadata_key, _) = Pubkey::find_program_address(metadata_seeds, &program_key);

    let metadata_account = client.get_account(&metadata_key).unwrap();
    let metadata: Metadata = try_from_slice_unchecked(&metadata_account.data).unwrap();

    let uri = app_matches.value_of("uri").unwrap().to_owned();

    let name_symbol_seeds = &[
        PREFIX.as_bytes(),
        &program_key.as_ref(),
        &metadata.data.name.as_bytes(),
        &metadata.data.symbol.as_bytes(),
    ];
    let (name_symbol_key, _) = Pubkey::find_program_address(name_symbol_seeds, &program_key);

    let instructions = [update_metadata_accounts(
        program_key,
        metadata_key,
        name_symbol_key,
        update_authority.pubkey(),
        Some(update_authority.pubkey()),
        uri.to_owned(),
    )];

    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    let signers = vec![&update_authority];

    transaction.sign(&signers, recent_blockhash);
    client.send_and_confirm_transaction(&transaction).unwrap();
    let metadata_account = client.get_account(&metadata_key).unwrap();
    let metadata: Metadata = try_from_slice_unchecked(&metadata_account.data).unwrap();
    (metadata, metadata_key)
}

fn create_metadata_account_call(
    app_matches: &ArgMatches,
    payer: Keypair,
    client: RpcClient,
) -> (Metadata, Pubkey) {
    let allow_duplicates = app_matches.is_present("allow_duplicates");
    let update_authority = read_keypair_file(
        app_matches
            .value_of("update_authority")
            .unwrap_or_else(|| app_matches.value_of("keypair").unwrap()),
    )
    .unwrap();

    let program_key = Pubkey::from_str(METADATA_PROGRAM_PUBKEY).unwrap();
    let token_key = Pubkey::from_str(TOKEN_PROGRAM_PUBKEY).unwrap();
    let new_mint = Keypair::new();
    let name = app_matches.value_of("name").unwrap().to_owned();
    let symbol = app_matches.value_of("symbol").unwrap().to_owned();
    let uri = app_matches.value_of("uri").unwrap().to_owned();
    let new_mint_key = new_mint.pubkey();
    let metadata_seeds = &[
        PREFIX.as_bytes(),
        &program_key.as_ref(),
        new_mint_key.as_ref(),
    ];
    let (metadata_key, _) = Pubkey::find_program_address(metadata_seeds, &program_key);

    let name_symbol_seeds = &[
        PREFIX.as_bytes(),
        &program_key.as_ref(),
        &name.as_bytes(),
        &symbol.as_bytes(),
    ];
    let (name_symbol_key, _) = Pubkey::find_program_address(name_symbol_seeds, &program_key);

    let instructions = [
        create_account(
            &payer.pubkey(),
            &new_mint.pubkey(),
            client
                .get_minimum_balance_for_rent_exemption(Mint::LEN)
                .unwrap(),
            Mint::LEN as u64,
            &token_key,
        ),
        initialize_mint(
            &token_key,
            &new_mint.pubkey(),
            &payer.pubkey(),
            Some(&payer.pubkey()),
            0,
        )
        .unwrap(),
        create_metadata_accounts(
            program_key,
            name_symbol_key,
            metadata_key,
            new_mint.pubkey(),
            payer.pubkey(),
            payer.pubkey(),
            update_authority.pubkey(),
            name,
            symbol,
            uri,
            allow_duplicates,
            update_authority.pubkey() != payer.pubkey(),
        ),
    ];

    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    let mut signers = vec![&payer, &new_mint];

    if update_authority.pubkey() != payer.pubkey() {
        signers.push(&update_authority)
    }

    transaction.sign(&signers, recent_blockhash);
    client.send_and_confirm_transaction(&transaction).unwrap();
    let account = client.get_account(&metadata_key).unwrap();
    let metadata: Metadata = try_from_slice_unchecked(&account.data).unwrap();
    (metadata, metadata_key)
}

fn main() {
    let app_matches = App::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .arg(
            Arg::with_name("keypair")
                .long("keypair")
                .value_name("KEYPAIR")
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
        .arg(
            Arg::with_name("update_authority")
                .long("update_authority")
                .value_name("UPDATE_AUTHORITY")
                .takes_value(true)
                .global(true)
                .help("Update authority filepath or url to keypair besides yourself, defaults to normal keypair"),
        )
        .subcommand(
            SubCommand::with_name("create_metadata_accounts")
                .about("Create Metadata Accounts")
                .arg(
                    Arg::with_name("name")
                        .long("name")
                        .global(true)
                        .value_name("NAME")
                        .takes_value(true)
                        .help("name for the Mint"),
                )
                .arg(
                    Arg::with_name("symbol")
                        .long("symbol")
                        .value_name("SYMBOL")
                        .takes_value(true)
                        .global(true)
                        .help("symbol for the Mint"),
                )
        .arg(
            Arg::with_name("uri")
                .long("uri")
                .value_name("URI")
                .takes_value(true)
                .required(true)
                .help("URI for the Mint"),
        )
        .arg(
            Arg::with_name("allow_duplicates")
                .long("allow_duplicates")
                .value_name("ALLOW_DUPLICATES")
                .takes_value(false)
                .required(false)
                .help("Allow duplicates"),
        )
    ).subcommand(
        SubCommand::with_name("update_metadata_accounts")
            .about("Update Metadata Accounts")
    .arg(
                Arg::with_name("mint")
                    .long("mint")
                    .value_name("MINT")
                    .required(false)
                    .validator(is_valid_pubkey)
                    .takes_value(true)
                    .help("Mint of the Metadata"),
            )
    .arg(
        Arg::with_name("uri")
            .long("uri")
            .value_name("URI")
            .takes_value(true)
            .required(true)
            .help("new URI for the Metadata"),
    )).subcommand(
        SubCommand::with_name("transfer_update_authority")
            .about("Transfer Update Authority")
            .arg(
                Arg::with_name("mint")
                    .long("mint")
                    .value_name("MINT")
                    .required(false)
                    .validator(is_valid_pubkey)
                    .takes_value(true)
                    .help("Metadata mint to update"),
            ).arg(
                Arg::with_name("new_update_authority")
                    .long("new_update_authority")
                    .value_name("NEW_UPDATE_AUTHORITY")
                    .required(true)
                    .validator(is_valid_pubkey)
                    .takes_value(true)
                    .help("New update authority"),
            )).get_matches();

    let client = RpcClient::new(
        app_matches
            .value_of("json_rpc_url")
            .unwrap_or(&"https://devnet.solana.com".to_owned())
            .to_owned(),
    );

    let payer = read_keypair_file(app_matches.value_of("keypair").unwrap()).unwrap();

    let (sub_command, sub_matches) = app_matches.subcommand();
    match (sub_command, sub_matches) {
        ("create_metadata_accounts", Some(arg_matches)) => {
            let (metadata, metadata_key) = create_metadata_account_call(arg_matches, payer, client);
            println!(
                "Create metadata account with mint {:?} and key {:?} and name of {:?} and symbol of {:?}",
                metadata.mint, metadata_key, metadata.data.name, metadata.data.symbol
            );
        }
        ("update_metadata_accounts", Some(arg_matches)) => {
            let (metadata, metadata_key) = update_metadata_account_call(arg_matches, payer, client);
            println!(
                "Update metadata account with mint {:?} and key {:?} which now has URI of {:?}",
                metadata.mint, metadata_key, metadata.data.uri
            );
        }
        ("transfer_update_authority", Some(arg_matches)) => {
            let (metadata, metadata_key) = transfer_authority_call(arg_matches, payer, client);
            println!(
                "Transfer authority on account mint {:?} and key {:?}",
                metadata.mint, metadata_key
            );
        }
        _ => unreachable!(),
    }
}
