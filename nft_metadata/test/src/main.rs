use solana_client::rpc_client::RpcClient;
use solana_program::program_pack::Pack;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{read_keypair_file, Signer},
    system_instruction::create_account,
    transaction::Transaction,
};
use spl_nft_metadata::{
    instruction::{
        create_nft_metadata_accounts, init_nft_metadata_accounts, update_nft_metadata_accounts,
    },
    state::nft_metadata::{NFTMetadata, NAME_LENGTH, SYMBOL_LENGTH},
    state::PREFIX,
};
use spl_token::{instruction::initialize_mint, state::Mint};
use std::str::FromStr;
// -------- UPDATE START -------

/*
const KEYPAIR_PATH: &str = "/your/path";
const NFT_METADATA_PROGRAM_PUBKEY_PATH: &str = "/your/path";
const NEW_MINT_PATH: &str = "/your/path";
const TOKEN_PROGRAM_PUBKEY: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const CLUSTER_ADDRESS: &str = "https://api.mainnet-beta.solana.com";
*/
const KEYPAIR_PATH: &str = "/Users/jprince/.config/solana/id.json";
const NFT_METADATA_PROGRAM_PUBKEY_PATH: &str = "/Users/jprince/Documents/other/solana-program-library/target/deploy/spl_nft_metadata-keypair.json";
const TOKEN_PROGRAM_PUBKEY: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const CLUSTER_ADDRESS: &str = "https://devnet.solana.com";
const NEW_MINT_PATH: &str = "/Users/jprince/.config/solana/mint.json";

// -------- UPDATE END ---------
pub fn main() {
    let client = RpcClient::new(CLUSTER_ADDRESS.to_owned());
    let payer = read_keypair_file(KEYPAIR_PATH).unwrap();
    let program_key = read_keypair_file(NFT_METADATA_PROGRAM_PUBKEY_PATH).unwrap();
    let token_key = Pubkey::from_str(TOKEN_PROGRAM_PUBKEY).unwrap();
    let new_mint = read_keypair_file(NEW_MINT_PATH).unwrap();

    let program_id = program_key.pubkey();
    let new_mint_key = new_mint.pubkey();
    let nft_metadata_seeds = &[
        PREFIX.as_bytes(),
        &program_id.as_ref(),
        new_mint_key.as_ref(),
    ];
    let (nft_metadata_key, _) =
        Pubkey::find_program_address(nft_metadata_seeds, &program_key.pubkey());
    let mut name: [u8; NAME_LENGTH] = [0; NAME_LENGTH];
    let mut symbol: [u8; SYMBOL_LENGTH] = [0; SYMBOL_LENGTH];

    let name_bytes = "Billy".as_bytes();
    for n in 0..(NAME_LENGTH - 1) {
        if n < name_bytes.len() {
            name[n] = name_bytes[n];
        }
    }

    let symbol_bytes = "Bob".as_bytes();
    for n in 0..(SYMBOL_LENGTH - 1) {
        if n < symbol_bytes.len() {
            symbol[n] = symbol_bytes[n];
        }
    }
    let nft_owner_seeds = &[PREFIX.as_bytes(), &program_id.as_ref(), &name, &symbol];
    let (nft_owner_key, _) = Pubkey::find_program_address(nft_owner_seeds, &program_key.pubkey());

    let mut transaction = Transaction::new_with_payer(
        &[
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
            create_nft_metadata_accounts(
                program_key.pubkey(),
                nft_owner_key,
                nft_metadata_key,
                new_mint.pubkey(),
                payer.pubkey(),
                payer.pubkey(),
                "Billy",
                "Bob",
            ),
            init_nft_metadata_accounts(
                program_key.pubkey(),
                nft_owner_key,
                nft_metadata_key,
                new_mint.pubkey(),
                payer.pubkey(),
                payer.pubkey(),
                "Billy",
                "Bob",
                "www.billybob.com",
                "actors",
                "me",
            ),
            update_nft_metadata_accounts(
                program_key.pubkey(),
                nft_metadata_key,
                nft_owner_key,
                payer.pubkey(),
                "www.aol.com",
                "thespians",
                "you",
            ),
        ],
        Some(&payer.pubkey()),
    );
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    transaction.sign(&[&payer, &new_mint], recent_blockhash);
    client.send_and_confirm_transaction(&transaction).unwrap();
    let account = client.get_account(&nft_metadata_key).unwrap();
    let metadata = NFTMetadata::unpack(&account.data).unwrap();
    println!("If this worked correctly, updated metadata should have aol and thespians and you: {:?} {:?} {:?}", std::str::from_utf8(&metadata.uri).unwrap(), std::str::from_utf8(&metadata.category).unwrap(), std::str::from_utf8(&metadata.creator).unwrap());
}
