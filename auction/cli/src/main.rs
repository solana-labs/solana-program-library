use {
    clap::{crate_description, crate_name, crate_version, App, Arg, ArgMatches, SubCommand},
    solana_clap_utils::{
        input_parsers::pubkey_of,
        input_validators::{is_url, is_valid_pubkey, is_valid_signer},
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{borsh::try_from_slice_unchecked, hash, program_pack::Pack},
    solana_sdk::{
        pubkey::Pubkey,
        signature::{read_keypair_file, Keypair, Signer},
        system_instruction::create_account,
        transaction::Transaction,
    },
    spl_token::{
        instruction::{initialize_account, initialize_mint, mint_to},
        state::{Account, Mint},
    },
    spl_auction,
    rand::Rng,
    std::str::FromStr,
};

const PROGRAM_PUBKEY: &str = "35tkNdRotULfEg7j5Ymh2X13y6JmLeS9fy7DTeNUptCA";
const TOKEN_PROGRAM_PUBKEY: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

fn create_auction(app_matches: &ArgMatches, payer: Keypair, client: RpcClient) {
    use spl_auction::{
        PREFIX,
        instruction,
        processor::{
            CreateAuctionArgs,
            PriceFloor,
            WinnerLimit,
        },
    };

    // Fixed Addresses.
    let program_key = Pubkey::from_str(PROGRAM_PUBKEY).unwrap();
    let token_key = Pubkey::from_str(TOKEN_PROGRAM_PUBKEY).unwrap();

    // Item being bid on, payer if nothing provided.
    let resource = pubkey_of(app_matches, "resource").unwrap_or_else(|| payer.pubkey());

    // Mint msut be a keypair file, as its used in further commands.
    let mint = read_keypair_file(
        app_matches
            .value_of("mint")
            .unwrap()
    )
    .unwrap();

    // Auction seeds.
    let seeds = &[PREFIX.as_bytes(), &program_key.as_ref(), &resource.as_ref()];
    let (auction_pubkey, _) = Pubkey::find_program_address(seeds, &program_key);

    // Configure a price floor
    let salt: u64 = rand::thread_rng().gen();
    let floor = app_matches.value_of("minimum").map(|price| {
        let price = price.parse::<u64>().unwrap();
        if app_matches.is_present("blind") {
            let hash = hash::hashv(&[
                &price.to_be_bytes(),
                &salt.to_be_bytes(),
            ]);
            PriceFloor::BlindedPrice(hash)
        } else {
            PriceFloor::MinimumPrice(price)
        }
    });

    println!(
        "Creating Auction:\n\
        - Auction: {}\n\
        - Payer: {}\n\
        - Mint: {}\n\
        - Resource: {}\n\
        - Salt: {}\n\n\
        Use the salt when revealing the price.
    ",
        auction_pubkey,
        payer.pubkey(),
        mint.pubkey(),
        resource,
        salt,
    );

    let instructions = [
        // Create a new mint to test this auction with.
        create_account(
            &payer.pubkey(),
            &mint.pubkey(),
            client
                .get_minimum_balance_for_rent_exemption(Mint::LEN)
                .unwrap(),
            Mint::LEN as u64,
            &token_key,
        ),

        // Initialize a mint to fund the bidder with.
        initialize_mint(
            &token_key,
            &mint.pubkey(),
            &payer.pubkey(),
            Some(&payer.pubkey()),
            0,
        )
        .unwrap(),
    ];

    // Sign and Submit
    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    let signers = vec![
        &payer,
        &mint,
    ];

    transaction.sign(&signers, recent_blockhash);
    client.send_and_confirm_transaction(&transaction);

    let instructions = [
        // Create an auction for the auction seller as their own resource.
        instruction::create_auction_instruction(
            program_key,
            payer.pubkey(),
            CreateAuctionArgs {
                authority: payer.pubkey(),
                end_auction_at: None,
                end_auction_gap: None,
                resource: resource,
                token_mint: mint.pubkey(),
                winners: WinnerLimit::Capped(5),
                price_floor: floor.unwrap_or(PriceFloor::None),
            },
        ),
    ];

    // Sign and Submit
    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    let signers = vec![
        &payer,
    ];

    transaction.sign(&signers, recent_blockhash);
    client.send_and_confirm_transaction(&transaction).unwrap();
}

fn inspect_auction(app_matches: &ArgMatches, _payer: Keypair, client: RpcClient) {
    use spl_auction::processor::{BidState, PriceFloor};

    // Fixed Addresses.
    let auction_pubkey = pubkey_of(app_matches, "auction").unwrap();

    // Load Auction data.
    let auction_data = client.get_account(&auction_pubkey).unwrap();
    let auction: spl_auction::processor::AuctionData = try_from_slice_unchecked(&auction_data.data).unwrap();

    println!(
        "\n\
        Inspecting Auction:\n\
        - Running: {:?} / {:?}\n\
        - Auction: {}\n\
        - Authority: {}\n\
        - Resource: {}\n\
        - Mint: {}\n\
        - Last Bid Placed At: {}\n\
        - Will End At: {}\n\
        - Gap Time: {}\n\
        - Ended At: {}\n\
        - Price Floor: {}
    ",
        auction.state,
        !auction.ended(client.get_slot().unwrap()),
        auction_pubkey,
        auction.authority,
        auction.resource,
        auction.token_mint,
        auction.last_bid.unwrap_or(0),
        auction.end_auction_at.unwrap_or(0),
        auction.end_auction_gap.unwrap_or(0),
        auction.ended_at.unwrap_or(0),
        match auction.price_floor {
            PriceFloor::None => "No Floor".to_string(),
            PriceFloor::MinimumPrice(min) => format!("Minimum Bid: {}", min),
            PriceFloor::BlindedPrice(_) => "Minimum Price Concealed".to_string(),
        },
    );

    match auction.bid_state {
        BidState::EnglishAuction { ref bids, max } => {
            println!("Winning Bids (Max {}):", max);
            for bid in bids {
                println!("- {:?}", bid);
            }
        },
        _ => {},
    }
    println!("");
}

fn place_bid(app_matches: &ArgMatches, payer: Keypair, client: RpcClient) {
    use spl_auction::{
        PREFIX,
        instruction,
        processor::{
            PlaceBidArgs,
        },
    };

    // Fixed Addresses.
    let program_key = Pubkey::from_str(PROGRAM_PUBKEY).unwrap();
    let token_key = Pubkey::from_str(TOKEN_PROGRAM_PUBKEY).unwrap();

    // Item being bid on, payer if nothing provided.
    let resource = pubkey_of(app_matches, "resource").unwrap_or_else(|| payer.pubkey());

    // Generate a random pot.
    let bid_pot = Keypair::new();
    let transfer_authority = Keypair::new();

    // Mint must be a keypair file, this must be the same as the one used in create auction.
    let mint = read_keypair_file(
        app_matches
            .value_of("mint")
            .unwrap()
    )
    .unwrap();

    // Bidder must be a keypair file, this must be the same as the one used in create auction.
    let bidder = read_keypair_file(
        app_matches
            .value_of("bidder")
            .unwrap()
    )
    .unwrap();

    // Auction seeds.
    let seeds = &[PREFIX.as_bytes(), &program_key.as_ref(), &resource.as_ref()];
    let (auction_pubkey, _) = Pubkey::find_program_address(seeds, &program_key);

    // Parse CLI amount value, fail if not a number.
    let amount = app_matches
        .value_of("amount")
        .unwrap_or("100")
        .parse::<u64>()
        .unwrap();

    println!(
        "Placing Bid:\n\
        - Auction: {}\n\
        - Payer: {}\n\
        - Mint: {}\n\
        - Bidder: {}\n\
        - Authority: {}\n\
        - Resource: {}\n\
        - Amount: {}\n\
    ",
        auction_pubkey,
        payer.pubkey(),
        mint.pubkey(),
        bidder.pubkey(),
        transfer_authority.pubkey(),
        resource,
        amount
    );

    let instructions = [
        // Create the Bidder's SPL account
        create_account(
            &payer.pubkey(),
            &bidder.pubkey(),
            client
                .get_minimum_balance_for_rent_exemption(Account::LEN)
                .unwrap(),
            spl_token::state::Account::LEN as u64,
            &spl_token::id(),
        ),

        // Initialize the SPL account using the token program.
        initialize_account(
            &token_key,
            &bidder.pubkey(),
            &mint.pubkey(),
            &payer.pubkey(),
        )
        .unwrap(),

        // Mint a bunch of tokens into the account for the user to actually bid with.
        mint_to(
            &token_key,
            &mint.pubkey(),
            &bidder.pubkey(),
            &payer.pubkey(),
            &[&payer.pubkey()],
            100000,
        )
        .unwrap(),
    ];

    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    let signers = vec![
        &payer,
        &bidder,
    ];

    // Doesn't matter if this transaction fails.
    transaction.sign(&signers, recent_blockhash);
    client.send_and_confirm_transaction(&transaction);

    let instructions = [
        // Generate another SPL account to transfer into, owned by the program. The address for
        // this account is generated from seeds.
        create_account(
            &payer.pubkey(),
            &bid_pot.pubkey(),
            client
                .get_minimum_balance_for_rent_exemption(Account::LEN)
                .unwrap(),
            spl_token::state::Account::LEN as u64,
            &spl_token::id(),
        ),

        // Initialize the SPL account using the token program. But make the manager of this
        // the auction account itself (required).
        initialize_account(
            &token_key,
            &bid_pot.pubkey(),
            &mint.pubkey(),
            &auction_pubkey,
        )
        .unwrap(),

        // Approve a transfer authority to move some tokens out.
        spl_token::instruction::approve(
            &token_key,
            &bidder.pubkey(),
            &transfer_authority.pubkey(),
            &payer.pubkey(),
            &[&payer.pubkey()],
            amount,
        )
        .unwrap(),

        // Bid!
        // Source account is the bidder's SPL account.
        // Destination was created above, as an auction owned pot to contain the bid.
        instruction::place_bid_instruction(
            program_key,
            bidder.pubkey(),                // SPL Token Account (Source)
            bid_pot.pubkey(),               // SPL Token Account (Destination)
            mint.pubkey(),                  // Token Mint
            transfer_authority.pubkey(),    // Account Approved to Move Tokens
            payer.pubkey(),                 // Pays for Transactions
            PlaceBidArgs {
                amount: amount,
                resource: resource,
            },
        )
    ];

    // Sign and Submit
    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    let signers = vec![
        &payer,
        &bidder,
        &bid_pot,
        &transfer_authority,
    ];

    transaction.sign(&signers, recent_blockhash);
    client.send_and_confirm_transaction(&transaction).unwrap();
}

fn claim_bid(app_matches: &ArgMatches, payer: Keypair, client: RpcClient) {
    use spl_auction::{
        PREFIX,
        instruction,
        processor::{
            ClaimBidArgs,
        },
    };

    // Fixed Addresses.
    let program_key = Pubkey::from_str(PROGRAM_PUBKEY).unwrap();
    let token_key = Pubkey::from_str(TOKEN_PROGRAM_PUBKEY).unwrap();

    // Item being bid on, payer if nothing provided.
    let resource = pubkey_of(app_matches, "resource").unwrap_or_else(|| payer.pubkey());

    // Mint must be a keypair file, this must be the same as the one used in create auction.
    let mint = read_keypair_file(
        app_matches
            .value_of("mint")
            .unwrap()
    )
    .unwrap();

    // Bidder must be a keypair file, this must be the same as the one used in create auction.
    let bidder = read_keypair_file(
        app_matches
            .value_of("bidder")
            .unwrap()
    )
    .unwrap();

    // Destination must be a keypair file, this is an SPL account to deposit tokens into.
    let destination = read_keypair_file(
        app_matches
            .value_of("destination")
            .unwrap()
    )
    .unwrap();

    // Auction seeds.
    let seeds = &[PREFIX.as_bytes(), &program_key.as_ref(), &resource.as_ref()];
    let (auction, _) = Pubkey::find_program_address(seeds, &program_key);
    let bidder_pubkey = bidder.pubkey();
    let seeds = &[PREFIX.as_bytes(), &program_key.as_ref(), &auction.as_ref(), &bidder_pubkey.as_ref()];
    let (bidpot, _) = Pubkey::find_program_address(seeds, &program_key);
    let bidpot_data = client.get_account(&bidpot).unwrap();
    let bidpot: spl_auction::processor::BidderPot = try_from_slice_unchecked(&bidpot_data.data).unwrap();

    println!(
        "Claiming Bid:\n\
        - Auction: {}\n\
        - Mint: {}\n\
        - Bidder: {}\n\
        - Destination: {}\n\
        - Resource: {}\n\
    ",
        auction,
        mint.pubkey(),
        bidder.pubkey(),
        destination.pubkey(),
        resource,
    );

    let instructions = [
        // Create an SPL account at the destination.
        create_account(
            &payer.pubkey(),
            &destination.pubkey(),
            client
                .get_minimum_balance_for_rent_exemption(Account::LEN)
                .unwrap(),
            spl_token::state::Account::LEN as u64,
            &spl_token::id(),
        ),

        // Initialize the SPL account using the token program.
        initialize_account(
            &token_key,
            &destination.pubkey(),
            &mint.pubkey(),
            &payer.pubkey(),
        )
        .unwrap(),
    ];

    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    let signers = vec![
        &payer,
        &destination,
    ];

    // Doesn't matter if this transaction fails.
    transaction.sign(&signers, recent_blockhash);
    client.send_and_confirm_transaction(&transaction);

    let instructions = [
        instruction::claim_bid_instruction(
            program_key,
            payer.pubkey(),
            destination.pubkey(),
            bidder.pubkey(),
            bidpot.bidder_pot,
            mint.pubkey(),
            ClaimBidArgs { resource: resource },
        )
    ];

    // Sign and Submit
    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    let signers = vec![
        &payer,
    ];

    transaction.sign(&signers, recent_blockhash);
    client.send_and_confirm_transaction(&transaction).unwrap();
}

fn cancel_bid(app_matches: &ArgMatches, payer: Keypair, client: RpcClient) {
    use spl_auction::{
        PREFIX,
        instruction,
        processor::{
            CancelBidArgs,
        },
    };

    // Fixed Addresses.
    let program_key = Pubkey::from_str(PROGRAM_PUBKEY).unwrap();

    // Item being bid on, payer if nothing provided.
    let resource = pubkey_of(app_matches, "resource").unwrap_or_else(|| payer.pubkey());

    // Mint must be a keypair file, this must be the same as the one used in create auction.
    let mint = read_keypair_file(
        app_matches
            .value_of("mint")
            .unwrap()
    )
    .unwrap();

    // Mint must be a keypair file, this must be the same as the one used in create auction.
    let bidder = read_keypair_file(
        app_matches
            .value_of("bidder")
            .unwrap()
    )
    .unwrap();

    // Load Bidpot data.
    let seeds = &[PREFIX.as_bytes(), &program_key.as_ref(), &resource.as_ref()];
    let (auction, _) = Pubkey::find_program_address(seeds, &program_key);
    let bidder_pubkey = bidder.pubkey();
    let seeds = &[PREFIX.as_bytes(), &program_key.as_ref(), &auction.as_ref(), &bidder_pubkey.as_ref()];
    let (bidpot, _) = Pubkey::find_program_address(seeds, &program_key);
    let bidpot_data = client.get_account(&bidpot).unwrap();
    let bidpot: spl_auction::processor::BidderPot = try_from_slice_unchecked(&bidpot_data.data).unwrap();

    let instructions = [
        instruction::cancel_bid_instruction(
            program_key,
            bidder.pubkey(),                // SPL Token Account (Source)
            bidpot.bidder_pot,              // SPL Token Account (Destination)
            mint.pubkey(),                  // Token Mint
            CancelBidArgs { resource: resource },
        )
    ];

    // Sign and Submit
    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    let signers = vec![
        &payer,
        &bidder,
    ];

    transaction.sign(&signers, recent_blockhash);
    client.send_and_confirm_transaction(&transaction).unwrap();
}

fn end_auction(app_matches: &ArgMatches, payer: Keypair, client: RpcClient) {
    use spl_auction::{
        instruction,
        processor::{
            EndAuctionArgs,
        },
    };

    // Fixed Addresses.
    let program_key = Pubkey::from_str(PROGRAM_PUBKEY).unwrap();

    // Item being bid on, payer if nothing provided.
    let resource = pubkey_of(app_matches, "resource").unwrap_or_else(|| payer.pubkey());

    let revealer = if let Some(salt) = app_matches.value_of("salt") {
        let price = app_matches.value_of("minimum").unwrap();
        let price = price.parse::<u64>().unwrap();
        let salt = salt.parse::<u64>().unwrap();
        let hash = hash::hashv(&[
            &price.to_be_bytes(),
            &salt.to_be_bytes(),
        ]);
        println!("Revealing Hash: {}", hash);
        let hash = hash::hashv(&[
            &salt.to_be_bytes(),
            &price.to_be_bytes(),
        ]);
        println!("Revealing Hash: {}", hash);
        let hash = hash::hashv(&[
            &price.to_le_bytes(),
            &salt.to_le_bytes(),
        ]);
        println!("Revealing Hash: {}", hash);
        let hash = hash::hashv(&[
            &salt.to_le_bytes(),
            &price.to_le_bytes(),
        ]);
        println!("Revealing Hash: {}", hash);
        Some((price, salt))
    } else {
        None
    };

    let instructions = [
        // Create an auction for the auction seller as their own resource.
        instruction::end_auction_instruction(
            program_key,
            payer.pubkey(),
            EndAuctionArgs {
                resource: resource,
                reveal: revealer,
            },
        ),
    ];

    // Sign and Submit
    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    let signers = vec![
        &payer,
    ];

    transaction.sign(&signers, recent_blockhash);
    client.send_and_confirm_transaction(&transaction).unwrap();
}

fn start_auction(app_matches: &ArgMatches, payer: Keypair, client: RpcClient) {
    use spl_auction::{
        instruction,
        processor::{
            StartAuctionArgs,
        },
    };

    // Fixed Addresses.
    let program_key = Pubkey::from_str(PROGRAM_PUBKEY).unwrap();

    // Item being bid on, payer if nothing provided.
    let resource = pubkey_of(app_matches, "resource").unwrap_or_else(|| payer.pubkey());

    // Auction seeds.
    //let seeds = &[PREFIX.as_bytes(), &program_key.as_ref(), &resource.as_ref()];
    //let (auction_pubkey, _) = Pubkey::find_program_address(seeds, &program_key);

    let instructions = [
        // Create an auction for the auction seller as their own resource.
        instruction::start_auction_instruction(
            program_key,
            payer.pubkey(),
            StartAuctionArgs { resource: resource },
        ),
    ];

    // Sign and Submit
    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    let signers = vec![
        &payer,
    ];

    transaction.sign(&signers, recent_blockhash);
    client.send_and_confirm_transaction(&transaction).unwrap();
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
        .subcommand(
            SubCommand::with_name("create")
                .about("Create an Auction")
                .arg(
                    Arg::with_name("resource")
                        .long("resource")
                        .value_name("RESOURCE")
                        .required(true)
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help("Pubkey of the resource being bid upon."),
                )
                .arg(
                    Arg::with_name("mint")
                        .long("mint")
                        .value_name("MINT")
                        .required(true)
                        .validator(is_valid_signer)
                        .takes_value(true)
                        .help("Keypair of the account to create a mint into."),
                )
                .arg(
                    Arg::with_name("minimum")
                        .long("minimum")
                        .value_name("AMOUNT")
                        .required(false)
                        .takes_value(true)
                        .help("A price floor setting the minimum amount required for bidders to win."),
                )
                .arg(
                    Arg::with_name("blind")
                        .long("blind")
                        .value_name("AMOUNT")
                        .required(false)
                        .takes_value(false)
                        .help("If set, hide the minimum required bid price until the end of the auction."),
                )
        )
        .subcommand(
            SubCommand::with_name("inspect")
                .about("Inspect an auction state.")
                .arg(
                    Arg::with_name("auction")
                        .long("auction")
                        .value_name("PUBKEY")
                        .required(true)
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help("Pubkey of an auction account."),
                )
        )
        .subcommand(
            SubCommand::with_name("bid")
                .about("Place a bid on an existing Auction")
                .arg(
                    Arg::with_name("resource")
                        .long("resource")
                        .value_name("RESOURCE")
                        .required(true)
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help("Pubkey of the resource being bid upon."),
                )
                .arg(
                    Arg::with_name("mint")
                        .long("mint")
                        .value_name("MINT")
                        .required(true)
                        .validator(is_valid_signer)
                        .takes_value(true)
                        .help("Keypair of the account to create a mint into."),
                )
                .arg(
                    Arg::with_name("bidder")
                        .long("bidder")
                        .value_name("BIDDER")
                        .required(true)
                        .validator(is_valid_signer)
                        .takes_value(true)
                        .help("Keypair of a bidder's SPL account."),
                )
                .arg(
                    Arg::with_name("amount")
                        .long("amount")
                        .value_name("AMOUNT")
                        .required(true)
                        .takes_value(true)
                        .help("Amount of tokens to bid."),
                )
        )
        .subcommand(
            SubCommand::with_name("claim")
                .about("Claim the tokens inside a winning bid.")
                .arg(
                    Arg::with_name("resource")
                        .long("resource")
                        .value_name("PUBKEY")
                        .required(true)
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help("Pubkey of the resource being bid upon."),
                )
                .arg(
                    Arg::with_name("mint")
                        .long("mint")
                        .value_name("MINT")
                        .required(true)
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help("Keypair of the mint used in this auction."),
                )
                .arg(
                    Arg::with_name("bidder")
                        .long("bidder")
                        .value_name("BIDDER")
                        .required(true)
                        .validator(is_valid_signer)
                        .takes_value(true)
                        .help("Keypair of a bidder's SPL account we want to claim from."),
                )
                .arg(
                    Arg::with_name("destination")
                        .long("destination")
                        .value_name("PUBKEY")
                        .required(true)
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help("Destination SPL account to claim tokens into"),
                )
        )
        .subcommand(
            SubCommand::with_name("cancel")
                .about("Cancel a bid on an existing Auction")
                .arg(
                    Arg::with_name("resource")
                        .long("resource")
                        .value_name("PUBKEY")
                        .required(true)
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help("Pubkey of the resource being bid upon."),
                )
                .arg(
                    Arg::with_name("mint")
                        .long("mint")
                        .value_name("PUBKEY")
                        .required(true)
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help("Keypair of the account to create a mint into."),
                )
                .arg(
                    Arg::with_name("bidder")
                        .long("bidder")
                        .value_name("PUBKEY")
                        .required(true)
                        .validator(is_valid_signer)
                        .takes_value(true)
                        .help("Keypair of a bidder's SPL account."),
                )
        )
        .subcommand(
            SubCommand::with_name("end")
                .about("Force the end an auction")
                .arg(
                    Arg::with_name("resource")
                        .long("resource")
                        .value_name("PUBKEY")
                        .required(true)
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help("Pubkey of the resource being bid upon."),
                )
                .arg(
                    Arg::with_name("minimum")
                        .long("minimum")
                        .value_name("AMOUNT")
                        .required(false)
                        .takes_value(true)
                        .help("The minimum price that was set. Must be provided with salt for blinded reveal."),
                )
                .arg(
                    Arg::with_name("salt")
                        .long("salt")
                        .value_name("SALT")
                        .required(false)
                        .takes_value(true)
                        .help("Salt used for price reveal."),
                )
        )
        .subcommand(
            SubCommand::with_name("start")
                .about("Force the start an auction")
                .arg(
                    Arg::with_name("resource")
                        .long("resource")
                        .value_name("PUBKEY")
                        .required(true)
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help("Pubkey of the resource being bid upon."),
                )
        )
        //.subcommand(
        //    SubCommand::with_name("external_price_account_rewrite")
        //        .about("Rewrite (or create) an External Price Account")
        //        .arg(
        //            Arg::with_name("external_price_account")
        //                .long("external_price_account")
        //                .value_name("EXTERNAL_PRICE_ACCOUNT")
        //                .required(true)
        //                .validator(is_valid_signer)
        //                .takes_value(true)
        //                .help("Filepath or URL to a keypair"),
        //        )
        //        .arg(
        //            Arg::with_name("price_mint")
        //                .long("price_mint")
        //                .value_name("PRICE_MINT")
        //                .takes_value(true)
        //                .validator(is_valid_pubkey)
        //                .required(false)
        //                .help("Price mint that price per share uses"),
        //        )
        //        .arg(
        //            Arg::with_name("price_per_share")
        //                .long("price_per_share")
        //                .value_name("PRICE_PER_SHARE")
        //                .takes_value(true)
        //                .required(false)
        //                .help("Price per share"),
        //        )
        //        .arg(
        //            Arg::with_name("allowed_to_combine")
        //                .long("allowed_to_combine")
        //                .value_name("ALLOWED_TO_COMBINE")
        //                .takes_value(false)
        //                .required(false)
        //                .help("Whether or not combination is allowed in the vault"),
        //        )
        //        .arg(
        //            Arg::with_name("already_created")
        //                .long("already_created")
        //                .value_name("ALREADY_CREATED")
        //                .takes_value(false)
        //                .required(false)
        //                .help("If we should skip creation because this account already exists"),
        //        ),
        //)
        //.subcommand(
        //    SubCommand::with_name("add_token_to_vault")
        //        .about("Add Token of X amount (default 1) to Inactive Metaplex")
        //        .arg(
        //            Arg::with_name("vault_authority")
        //                .long("vault_authority")
        //                .value_name("VAULT_AUTHORITY")
        //                .required(false)
        //                .validator(is_valid_signer)
        //                .takes_value(true)
        //                .help("Filepath or URL to a keypair, defaults to you otherwise"),
        //        )
        //        .arg(
        //            Arg::with_name("vault_address")
        //                .long("vault_address")
        //                .value_name("VAULT_ADDRESS")
        //                .required(true)
        //                .validator(is_valid_pubkey)
        //                .takes_value(true)
        //                .help("Pubkey of vault"),
        //        )
        //        .arg(
        //            Arg::with_name("amount")
        //                .long("amount")
        //                .value_name("AMOUNT")
        //                .required(false)
        //                .takes_value(true)
        //                .help("Amount of this new token type to add to the vault"),
        //        ),
        //)
        //.subcommand(
        //    SubCommand::with_name("activate_vault")
        //        .about("Activate Metaplex")
        //        .arg(
        //            Arg::with_name("vault_authority")
        //                .long("vault_authority")
        //                .value_name("VAULT_AUTHORITY")
        //                .required(false)
        //                .validator(is_valid_signer)
        //                .takes_value(true)
        //                .help("Filepath or URL to a keypair, defaults to you otherwise"),
        //        )
        //        .arg(
        //            Arg::with_name("vault_address")
        //                .long("vault_address")
        //                .value_name("VAULT_ADDRESS")
        //                .required(true)
        //                .validator(is_valid_pubkey)
        //                .takes_value(true)
        //                .help("Pubkey of vault"),
        //        )
        //        .arg(
        //            Arg::with_name("number_of_shares")
        //                .long("number_of_shares")
        //                .value_name("NUMBER_OF_SHARES")
        //                .required(false)
        //                .takes_value(true)
        //                .help("Initial number of shares to produce, defaults to 100"),
        //        ),
        //)
        //.subcommand(
        //    SubCommand::with_name("combine_vault")
        //        .about("Combine Metaplex")
        //        .arg(
        //            Arg::with_name("vault_authority")
        //                .long("vault_authority")
        //                .value_name("VAULT_AUTHORITY")
        //                .required(false)
        //                .validator(is_valid_signer)
        //                .takes_value(true)
        //                .help("Filepath or URL to a keypair, defaults to you otherwise"),
        //        )
        //        .arg(
        //            Arg::with_name("vault_address")
        //                .long("vault_address")
        //                .value_name("VAULT_ADDRESS")
        //                .required(true)
        //                .validator(is_valid_pubkey)
        //                .takes_value(true)
        //                .help("Pubkey of vault"),
        //        ).arg(
        //            Arg::with_name("outstanding_shares_account")
        //                .long("outstanding_shares_account")
        //                .value_name("OUSTANDING_SHARES_ACCOUNT")
        //                .required(false)
        //                .validator(is_valid_pubkey)
        //                .takes_value(true)
        //                .help("Pubkey of oustanding shares account, an empty will be made if not provided"),
        //        ).arg(
        //            Arg::with_name("amount_of_money")
        //                .long("amount_of_money")
        //                .value_name("AMOUNT_OF_MONEY")
        //                .required(false)
        //                .takes_value(true)
        //                .help("Initial amount of money to provide to pay for buy out, defaults to 10000. You need to provide enough for a buy out!"),
        //        ),
        //)
        //.subcommand(
        //    SubCommand::with_name("redeem_shares")
        //        .about("Redeem Shares from a Combined Metaplex as a Shareholder")
        //        .arg(
        //            Arg::with_name("vault_authority")
        //                .long("vault_authority")
        //                .value_name("VAULT_AUTHORITY")
        //                .required(false)
        //                .validator(is_valid_signer)
        //                .takes_value(true)
        //                .help("Filepath or URL to a keypair, defaults to you otherwise"),
        //        )
        //        .arg(
        //            Arg::with_name("vault_address")
        //                .long("vault_address")
        //                .value_name("VAULT_ADDRESS")
        //                .required(true)
        //                .validator(is_valid_pubkey)
        //                .takes_value(true)
        //                .help("Pubkey of vault"),
        //        ).arg(
        //            Arg::with_name("outstanding_shares_account")
        //                .long("outstanding_shares_account")
        //                .value_name("OUSTANDING_SHARES_ACCOUNT")
        //                .required(true)
        //                .validator(is_valid_pubkey)
        //                .takes_value(true)
        //                .help("Pubkey of oustanding shares account"),
        //        ).arg(
        //            Arg::with_name("proceeds_account")
        //                .long("proceeds_account")
        //                .value_name("PROCEEDS_ACCOUNT")
        //                .required(false)
        //                .validator(is_valid_pubkey)
        //                .takes_value(true)
        //                .help("Pubkey of proceeds account, an empty will be made if not provided"),
        //        )
        //    )
        //.subcommand(
        //SubCommand::with_name("withdraw_tokens")
        //        .about("Withdraw Tokens from an Inactive or Combined Metaplex Safety Deposit Box")
        //        .arg(
        //            Arg::with_name("vault_authority")
        //                .long("vault_authority")
        //                .value_name("VAULT_AUTHORITY")
        //                .required(false)
        //                .validator(is_valid_signer)
        //                .takes_value(true)
        //                .help("Filepath or URL to a keypair, defaults to you otherwise"),
        //        )
        //        .arg(
        //            Arg::with_name("safety_deposit_address")
        //                .long("safety_deposit_address")
        //                .value_name("SAFETY_DEPOSIT_ADDRESS")
        //                .required(true)
        //                .validator(is_valid_pubkey)
        //                .takes_value(true)
        //                .help("Pubkey of safety deposit box"),
        //        ).arg(
        //            Arg::with_name("destination_account")
        //                .long("destination_account")
        //                .value_name("DESTINATION_ACCOUNT")
        //                .required(false)
        //                .validator(is_valid_pubkey)
        //                .takes_value(true)
        //                .help("Pubkey of destination shares account, an empty will be made if not provided"),
        //        ))
        //.subcommand(
        //    SubCommand::with_name("mint_shares")
        //        .about("Mint new shares to the fractional vault treasury")
        //        .arg(
        //            Arg::with_name("vault_authority")
        //                .long("vault_authority")
        //                .value_name("VAULT_AUTHORITY")
        //                .required(false)
        //                .validator(is_valid_signer)
        //                .takes_value(true)
        //                .help("Filepath or URL to a keypair, defaults to you otherwise"),
        //        )
        //        .arg(
        //            Arg::with_name("vault_address")
        //                .long("vault_address")
        //                .value_name("VAULT_ADDRESS")
        //                .required(true)
        //                .validator(is_valid_pubkey)
        //                .takes_value(true)
        //                .help("Pubkey of the vault"),
        //        )
        //        .arg(
        //            Arg::with_name("number_of_shares")
        //                .long("number_of_shares")
        //                .value_name("NUMBER_OF_SHARES")
        //                .required(false)
        //                .takes_value(true)
        //                .help("Initial number of shares to produce, defaults to 100"),
        //        ))
        //.subcommand(
        //    SubCommand::with_name("withdraw_shares")
        //        .about("Withdraw shares from the fractional treasury")
        //        .arg(
        //            Arg::with_name("vault_authority")
        //                .long("vault_authority")
        //                .value_name("VAULT_AUTHORITY")
        //                .required(false)
        //                .validator(is_valid_signer)
        //                .takes_value(true)
        //                .help("Filepath or URL to a keypair, defaults to you otherwise"),
        //        )
        //        .arg(
        //            Arg::with_name("vault_address")
        //                .long("vault_address")
        //                .value_name("VAULT_ADDRESS")
        //                .required(true)
        //                .validator(is_valid_pubkey)
        //                .takes_value(true)
        //                .help("Pubkey of the vault"),
        //        )
        //        .arg(
        //            Arg::with_name("number_of_shares")
        //                .long("number_of_shares")
        //                .value_name("NUMBER_OF_SHARES")
        //                .required(false)
        //                .takes_value(true)
        //                .help("Initial number of shares to produce, defaults to 100"),
        //        ).arg(
        //            Arg::with_name("destination_account")
        //                .long("destination_account")
        //                .value_name("DESTINATION_ACCOUNT")
        //                .required(false)
        //                .validator(is_valid_pubkey)
        //                .takes_value(true)
        //                .help("Pubkey of destination shares account, an empty will be made if not provided"),
        //        )).subcommand(
        //            SubCommand::with_name("add_shares")
        //                .about("Add shares to the fractional treasury")
        //                .arg(
        //                    Arg::with_name("vault_authority")
        //                        .long("vault_authority")
        //                        .value_name("VAULT_AUTHORITY")
        //                        .required(false)
        //                        .validator(is_valid_signer)
        //                        .takes_value(true)
        //                        .help("Filepath or URL to a keypair, defaults to you otherwise"),
        //                )
        //                .arg(
        //                    Arg::with_name("vault_address")
        //                        .long("vault_address")
        //                        .value_name("VAULT_ADDRESS")
        //                        .required(true)
        //                        .validator(is_valid_pubkey)
        //                        .takes_value(true)
        //                        .help("Pubkey of the vault"),
        //                )
        //                .arg(
        //                    Arg::with_name("number_of_shares")
        //                        .long("number_of_shares")
        //                        .value_name("NUMBER_OF_SHARES")
        //                        .required(false)
        //                        .takes_value(true)
        //                        .help("Initial number of shares to produce, defaults to 100"),
        //                ).arg(
        //                    Arg::with_name("source")
        //                        .long("source")
        //                        .value_name("SOURCE_ACCOUNT")
        //                        .required(true)
        //                        .validator(is_valid_pubkey)
        //                        .takes_value(true)
        //                        .help("Pubkey of source shares account"),
        //                ))
        .get_matches();

    let client = RpcClient::new(
        app_matches
            .value_of("json_rpc_url")
            .unwrap_or(&"https://testnet.solana.com".to_owned())
            .to_owned(),
    );

    let (sub_command, sub_matches) = app_matches.subcommand();
    let payer = read_keypair_file(app_matches.value_of("keypair").unwrap()).unwrap();

    match (sub_command, sub_matches) {
        ("create", Some(arg_matches)) => {
            create_auction(arg_matches, payer, client);
        }
        ("inspect", Some(arg_matches)) => {
            inspect_auction(arg_matches, payer, client);
        }
        ("bid", Some(arg_matches)) => {
            place_bid(arg_matches, payer, client);
        }
        ("claim", Some(arg_matches)) => {
            claim_bid(arg_matches, payer, client);
        }
        ("cancel", Some(arg_matches)) => {
            cancel_bid(arg_matches, payer, client);
        }
        ("start", Some(arg_matches)) => {
            start_auction(arg_matches, payer, client);
        }
        ("end", Some(arg_matches)) => {
            end_auction(arg_matches, payer, client);
        }
        // ("external_price_account_rewrite", Some(arg_matches)) => {
        //     println!(
        //         "Rewrote price account {:?}",
        //         rewrite_price_account(arg_matches, payer, client)
        //     );
        // }
        // ("add_token_to_vault", Some(arg_matches)) => {
        //     println!(
        //         "Added token to safety deposit account {:?} to vault {:?}",
        //         add_token_to_vault(arg_matches, payer, client),
        //         arg_matches.value_of("vault_address").unwrap()
        //     );
        // }
        // ("activate_vault", Some(arg_matches)) => {
        //     activate_vault(arg_matches, payer, client);
        //     println!("Completed command.");
        // }
        // ("combine_vault", Some(arg_matches)) => {
        //     combine_vault(arg_matches, payer, client);
        //     println!("Completed command.");
        // }
        // ("redeem_shares", Some(arg_matches)) => {
        //     println!(
        //         "Redeemed share(s) and put monies in account {:?}",
        //         redeem_shares(arg_matches, payer, client)
        //     );
        // }
        // ("withdraw_tokens", Some(arg_matches)) => {
        //     println!(
        //         "Withdrew token(s) to account {:?}",
        //         withdraw_tokens(arg_matches, payer, client)
        //     );
        // }
        // ("mint_shares", Some(arg_matches)) => {
        //     println!(
        //         "Minted share(s) to fractional treasury {:?}",
        //         mint_shares(arg_matches, payer, client)
        //     );
        // }
        // ("withdraw_shares", Some(arg_matches)) => {
        //     println!(
        //         "Withdrew share(s) to account {:?}",
        //         withdraw_shares(arg_matches, payer, client)
        //     );
        // }
        // ("add_shares", Some(arg_matches)) => {
        //     println!(
        //         "Added share(s) to fractional treasury account {:?}",
        //         add_shares(arg_matches, payer, client)
        //     );
        // }
        _ => unreachable!(),
    }
}
