use solana_client::rpc_client::RpcClient;
use solana_program::{message::Message, program_pack::Pack};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{read_keypair_file, Signer},
    system_instruction::create_account,
    transaction::Transaction,
};
use spl_hello_world_escrow::instruction::release_escrow_instruction;
use spl_token::{
    instruction::initialize_account, instruction::initialize_mint, instruction::mint_to,
    state::Account, state::Mint,
};
use std::str::FromStr;
// -------- UPDATE START -------
const KEYPAIR_PATH: &str = "/Users/jprince/.config/solana/id.json";
const HELLO_WORLD_PROGRAM_KEYPATH: &str = "/Users/jprince/Documents/other/solana-program-library/target/deploy/spl_hello_world_escrow-keypair.json";
const MINT_PUBKEY_PATH: &str = "/Users/jprince/.config/solana/mint.json";
const MY_ACCT_PUBKEY_PATH: &str = "/Users/jprince/.config/solana/account.json";
const ESCROW_ACCT_PUBKEY_PATH: &str = "/Users/jprince/.config/solana/escrow.json";
const TOKEN_PROGRAM_PUBKEY: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const CLUSTER_ADDRESS: &str = "https://devnet.solana.com";

pub fn main() {
    let client = RpcClient::new(CLUSTER_ADDRESS.to_owned());
    let payer = read_keypair_file(KEYPAIR_PATH).unwrap();
    let token_program = Pubkey::from_str(TOKEN_PROGRAM_PUBKEY).unwrap();
    let hello_world = read_keypair_file(HELLO_WORLD_PROGRAM_KEYPATH).unwrap();
    let mint_key = read_keypair_file(MINT_PUBKEY_PATH).unwrap();
    let escrow = read_keypair_file(ESCROW_ACCT_PUBKEY_PATH).unwrap();
    let my_acct = read_keypair_file(MY_ACCT_PUBKEY_PATH).unwrap();
    let (authority_key, bump_seed) =
        Pubkey::find_program_address(&[escrow.pubkey().as_ref()], &hello_world.pubkey());

    let mut transaction = Transaction::new_with_payer(
        &[
            create_account(
                &payer.pubkey(),
                &mint_key.pubkey(),
                client
                    .get_minimum_balance_for_rent_exemption(Mint::LEN)
                    .unwrap(),
                Mint::LEN as u64,
                &token_program,
            ),
            create_account(
                &payer.pubkey(),
                &my_acct.pubkey(),
                client
                    .get_minimum_balance_for_rent_exemption(Account::LEN)
                    .unwrap(),
                Account::LEN as u64,
                &token_program,
            ),
            create_account(
                &payer.pubkey(),
                &escrow.pubkey(),
                client
                    .get_minimum_balance_for_rent_exemption(Account::LEN)
                    .unwrap(),
                Account::LEN as u64,
                &token_program,
            ),
            initialize_mint(
                &token_program,
                &mint_key.pubkey(),
                &payer.pubkey(),
                Some(&payer.pubkey()),
                0,
            )
            .unwrap(),
            initialize_account(
                &token_program,
                &my_acct.pubkey(),
                &mint_key.pubkey(),
                &payer.pubkey(),
            )
            .unwrap(),
            initialize_account(
                &token_program,
                &escrow.pubkey(),
                &mint_key.pubkey(),
                &authority_key,
            )
            .unwrap(),
            mint_to(
                &token_program,
                &mint_key.pubkey(),
                &escrow.pubkey(),
                &payer.pubkey(),
                &[&payer.pubkey()],
                100,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    transaction.sign(&[&payer, &mint_key, &escrow, &my_acct], recent_blockhash);
    client.send_and_confirm_transaction(&transaction).unwrap();
    let account = client.get_account(&my_acct.pubkey()).unwrap();
    let _program = Account::unpack(&account.data).unwrap();
    println!("Created your account with pubkey: {}", my_acct.pubkey());

    let release_msg = Message::new(
        &[release_escrow_instruction(
            hello_world.pubkey(),
            authority_key,
            escrow.pubkey(),
            my_acct.pubkey(),
            token_program,
            100,
        )],
        Some(&payer.pubkey()),
    );

    println!(
        "This is the release command to paste into your proposal: {:?}:",
        base64::encode(release_msg.serialize().as_slice())
    )
}
