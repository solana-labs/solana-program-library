#[macro_use]
extern crate arrayref;

use anyhow::anyhow;
use clap::Parser;
use solana_cli_config::Config;
use solana_client::rpc_client::RpcClient;
use solana_client_helpers::Client;
use solana_sdk::instruction::Instruction;
use solana_sdk::signature::Signature;
use solana_sdk::transaction::Transaction;
use solana_sdk::{pubkey::Pubkey, signer::keypair::read_keypair_file};
use spl_token_swap::state::{SwapState, SwapVersion};
use std::fmt::Display;

const POOL_REGISTRY_SEED: &str = "poolregistry";

/// Search for a pattern in a file and display the lines that contain it.
#[derive(Parser)]
struct Cli {
    #[clap(
        short = 'p',
        long = "program-id",
        default_value = "SSwpMgqNDsyV7mAgN9ady4bDVu5ySjmmXejXvy2vLt1"
    )]
    program_id: Pubkey,
    #[clap(
        short = 'r',
        long = "registry-owner",
        default_value = "GkT2mRSujbydLUmA178ykHe7hZtaUpkmX2sfwS8suWb3"
    )]
    registry_owner: Pubkey,
    #[clap(
        short = 'o',
        long = "owner-fee",
        default_value = "5Cebzty8iwgAUx9jyfZVAT2iMvXBECLwEVgT6T8KYmvS"
    )]
    owner_fee: Pubkey,
}

fn main() -> Result<(), anyhow::Error> {
    let config_file = solana_cli_config::CONFIG_FILE
        .as_ref()
        .ok_or_else(|| anyhow!("unable to get config file path"))?;
    let cli_config = Config::load(config_file).map_err(|_| anyhow!("unable to read config"))?;
    let kp = read_keypair_file(&cli_config.keypair_path)
        .map_err(|_| anyhow!("unable to get config file path"))?;
    let args = Cli::parse();

    let client = Client {
        client: RpcClient::new(cli_config.json_rpc_url),
        payer: kp,
    };
    let pools = get_pools(&client, &args.registry_owner, &args.program_id)?;

    println!("pools: {}", pools);

    for pool_key in pools {
        let pool = get_pool(&client, &pool_key);
        if pool.is_ok() {
            let pool2 = pool?;
            if pool2.is_some() {
                let pool3 = pool2.unwrap();
                let fee_key = pool3.pool_fee_account();
                let res = client.get_token_account(&fee_key);
                println!(
                    "swap account {} has fee token account {:?}",
                    pool_key,
                    pool3.pool_fee_account()
                );
                if res.is_err() {
                    let mut line = String::new();
                    println!("Token account error, type 'repair' to repair this pool:");
                    std::io::stdin().read_line(&mut line)?;
                    if line.len() >= 5 && &line[0..6] == "repair" {
                        let tx_res = repair(
                            &client,
                            &args.program_id,
                            &pool_key,
                            &args.owner_fee,
                            fee_key,
                            &pool3.pool_mint(),
                        );
                        if let Err(e) = tx_res {
                            println!("failed to repair: {}", e);
                        } else {
                            println!("repaired in tx {}", tx_res.unwrap());
                        }
                    } else {
                        println!("not repaired");
                    }
                }
            } else {
                println!("{}: None", pool_key);
            }
        } else {
            println!("{}: invalid", pool_key);
        }
    }

    println!("complete");

    Ok(())
}

fn repair(
    client: &Client,
    program_id: &Pubkey,
    pool_key: &Pubkey,
    fee_owner: &Pubkey,
    old_fee_key: &Pubkey,
    mint_key: &Pubkey,
) -> Result<Signature, anyhow::Error> {
    let mut ixs: Vec<Instruction> = Vec::new();
    let ata = spl_associated_token_account::get_associated_token_address(fee_owner, mint_key);
    println!("looking for ata {}", ata);
    let res = client.get_token_account(&ata);
    if res.is_err() {
        println!(
            "ata not found, will create {} owned by {} for mint {}",
            ata, fee_owner, mint_key
        );
        ixs.push(
            spl_associated_token_account::create_associated_token_account(
                &client.payer_pubkey(),
                fee_owner,
                mint_key,
            ),
        );
    }
    ixs.push(spl_token_swap::instruction::repair_closed_fee_account(
        &program_id,
        &pool_key,
        &old_fee_key,
        &ata,
    )?);
    let mut tx = Transaction::new_with_payer(&ixs, Some(&client.payer_pubkey()));
    tx.sign(&[client.payer()], client.recent_blockhash()?);
    println!("sending tx");
    Ok(client.send_and_confirm_transaction_with_spinner(&tx)?)
}

fn get_pool(
    client: &RpcClient,
    pool_key: &Pubkey,
) -> Result<Option<Box<dyn SwapState>>, anyhow::Error> {
    let maybe_data = match client.get_account_data(pool_key) {
        Ok(s) => Ok(Some(s)),
        Err(_) => Ok(None), //todo how to see not found, this is wrong, want to do below also when specific not found err
        Err(e) => Err(anyhow!("cannot unpack pool account: {}", e)),
    }?;

    match maybe_data {
        Some(data) => SwapVersion::unpack(&data)
            .map(|s| Some(s))
            .map_err(|e| anyhow!("cannot unpack pool account: {} err: {}", pool_key, e)),
        None => Ok(None),
    }
}

fn get_pools(
    client: &RpcClient,
    registry_owner: &Pubkey,
    program: &Pubkey,
) -> Result<PoolRegistryIterator, anyhow::Error> {
    let a = Pubkey::create_with_seed(registry_owner, POOL_REGISTRY_SEED, program)
        .map_err(|_| anyhow!("cannot derive registry address"))?;
    let data = client
        .get_account_data(&a)
        .map_err(|_| anyhow!("cannot read registry account"))?;
    Ok(PoolRegistryIterator::new(data))
}

struct PoolRegistryIterator {
    size: usize,
    index: usize,
    data: Vec<u8>,
}

impl PoolRegistryIterator {
    pub fn new(data: Vec<u8>) -> Self {
        PoolRegistryIterator {
            size: u32::from_le_bytes(array_ref!(data, 1, 4).clone()) as usize,
            index: 0,
            data,
        }
    }
}

impl Iterator for PoolRegistryIterator {
    // we will be counting with usize
    type Item = Pubkey;

    // next() is the only required method
    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.size {
            None
        } else {
            let ret = Some(Pubkey::new(array_ref!(self.data[5..], self.index * 32, 32)));
            self.index = self.index + 1;
            ret
        }
    }
}

impl Display for PoolRegistryIterator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        writeln!(f, "size: {}, vec size: {}", self.size, self.data.len())?;
        Ok(())
    }
}
