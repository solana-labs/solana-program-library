use solana_client::rpc_client::RpcClient;
use solana_program::program_pack::Pack;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{read_keypair_file, Signer},
    system_instruction::create_account,
    transaction::Transaction,
};
use spl_timelock::{instruction::init_timelock_program, state::timelock_program::TimelockProgram};
use spl_token;
use std::str::FromStr;

// -------- UPDATE START -------
const KEYPAIR_PATH: &str = "/your/path";
const TIMELOCK_PROGRAM_PUBKEY_PATH: &str = "/your/path";
const CLUSTER_ADDRESS: &str = "https://api.mainnet-beta.solana.com";
solana_program::declare_id!("TimeLock11111111111111111111111111111111111");

// -------- UPDATE END ---------

pub fn main() {
    let client = RpcClient::new(CLUSTER_ADDRESS.to_owned());

    let payer = read_keypair_file(KEYPAIR_PATH).unwrap();
    let timelock_program_key = read_keypair_file(TIMELOCK_PROGRAM_PUBKEY_PATH).unwrap();
    let timelock_pub = timelock_program_key.pubkey();

    let mut transaction = Transaction::new_with_payer(
        &[
            create_account(
                &payer.pubkey(),
                &timelock_pub,
                client
                    .get_minimum_balance_for_rent_exemption(TimelockProgram::LEN)
                    .unwrap(),
                TimelockProgram::LEN as u64,
                &id(),
            ),
            init_timelock_program(id(), timelock_pub, spl_token::id()),
        ],
        Some(&payer.pubkey()),
    );

    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    transaction.sign(&[&payer, &timelock_program_key], recent_blockhash);
    client.send_and_confirm_transaction(&transaction).unwrap();

    let account = client.get_account(&timelock_pub).unwrap();
    let program = TimelockProgram::unpack(&account.data).unwrap();
    println!("Created timelock program with pubkey: {}", timelock_pub);
}
