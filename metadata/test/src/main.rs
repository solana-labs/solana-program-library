use solana_client::rpc_client::RpcClient;
use solana_program::{borsh::try_from_slice_unchecked, program_pack::Pack};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{read_keypair_file, Signer},
    system_instruction::create_account,
    transaction::Transaction,
};
use spl_metadata::{
    instruction::{create_metadata_accounts, init_metadata_accounts, update_metadata_accounts},
    state::{Metadata, PREFIX},
};

use spl_token::{instruction::initialize_mint, state::Mint};
use std::str::FromStr;
// -------- UPDATE START -------

const KEYPAIR_PATH: &str = "/Users/jprince/.config/solana/id.json";
const METADATA_PROGRAM_PUBKEY: &str = "Xk2Cihp7vnSWdcfDjH6W3H79WoFfADyqQQW6cENNXDS";
const NEW_MINT_PATH: &str = "/Users/jprince/.config/solana/mint.json";
const TOKEN_PROGRAM_PUBKEY: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const CLUSTER_ADDRESS: &str = "https://devnet.solana.com";

// -------- UPDATE END ---------
pub fn main() {
    let client = RpcClient::new(CLUSTER_ADDRESS.to_owned());
    let payer = read_keypair_file(KEYPAIR_PATH).unwrap();
    let program_key = Pubkey::from_str(METADATA_PROGRAM_PUBKEY).unwrap();
    let token_key = Pubkey::from_str(TOKEN_PROGRAM_PUBKEY).unwrap();
    let new_mint = read_keypair_file(NEW_MINT_PATH).unwrap();

    let new_mint_key = new_mint.pubkey();
    let metadata_seeds = &[
        PREFIX.as_bytes(),
        &program_key.as_ref(),
        new_mint_key.as_ref(),
    ];
    let (metadata_key, _) = Pubkey::find_program_address(metadata_seeds, &program_key);

    let owner_seeds = &[
        PREFIX.as_bytes(),
        &program_key.as_ref(),
        &"Billy3".as_bytes(),
        &"Bob3".as_bytes(),
    ];
    let (owner_key, _) = Pubkey::find_program_address(owner_seeds, &program_key);

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
            create_metadata_accounts(
                program_key,
                owner_key,
                metadata_key,
                new_mint.pubkey(),
                payer.pubkey(),
                payer.pubkey(),
                "Billy3".to_owned(),
                "Bob3".to_owned(),
            ),
            init_metadata_accounts(
                program_key,
                owner_key,
                metadata_key,
                new_mint.pubkey(),
                payer.pubkey(),
                payer.pubkey(),
                "Billy3".to_owned(),
                "Bob3".to_owned(),
                "www.billybob.com".to_owned(),
            ),
            update_metadata_accounts(
                program_key,
                metadata_key,
                owner_key,
                payer.pubkey(),
                "www.aol.com".to_owned(),
            ),
        ],
        Some(&payer.pubkey()),
    );
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    transaction.sign(&[&payer, &new_mint], recent_blockhash);
    client.send_and_confirm_transaction(&transaction).unwrap();
    let account = client.get_account(&metadata_key).unwrap();
    let metadata: Metadata = try_from_slice_unchecked(&account.data).unwrap();
    println!(
        "If this worked correctly, updated metadata should have aol: {:?} ",
        metadata.uri
    );
}
