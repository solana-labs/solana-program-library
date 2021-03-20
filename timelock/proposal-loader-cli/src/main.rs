use bincode::serialize;
use solana_client::{
    client_error::ClientError, rpc_client::RpcClient, rpc_config::RpcSendTransactionConfig,
    rpc_request::MAX_GET_SIGNATURE_STATUSES_QUERY_ITEMS,
};

use solana_client::{client_error::Result as ClientResult, rpc_response::RpcLeaderSchedule};
use solana_program::{
    bpf_loader_upgradeable::{self, UpgradeableLoaderState},
    epoch_schedule::Slot,
    fee_calculator::FeeCalculator,
    loader_upgradeable_instruction::UpgradeableLoaderInstruction,
    message::Message,
    msg,
    native_token::lamports_to_sol,
    program_pack::Pack,
    system_instruction::{self, SystemError},
    sysvar,
};

use solana_client::rpc_response::RpcContactInfo;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction, InstructionError},
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signer},
    signers::Signers,
    system_instruction::create_account,
    transaction::Transaction,
};
use solana_transaction_status::TransactionConfirmationStatus;
use std::{
    cmp::min,
    collections::HashMap,
    net::{SocketAddr, UdpSocket},
    thread::sleep,
    time::Duration,
};

use spl_timelock::{instruction::init_timelock_program, state::timelock_program::TimelockProgram};
use spl_token;
use std::{error, str::FromStr, time::Instant};
// -------- UPDATE START -------
const CLUSTER_ADDRESS: &str = "https://devnet.solana.com";
const KEYPAIR_PATH: &str = "/Users/jprince/.config/solana/id.json";

solana_program::declare_id!("BPFLoaderUpgradeab1e11111111111111111111111");
const BUFFER_PATH: &str = "/Users/jprince/.config/solana/bpf_place.json";
const DEPLOY_PATH: &str =
    "/Users/jprince/Documents/other/solana-program-library/target/deploy/spl_hello_world_escrow.so";
const TIMELOCK_PROGRAM_ID: &str = "7SH5hE7uBecnfMpGjdPyJupgBhFHaXcNMCEgJbmoVV7t";
const TIMELOCK_PROGRAM_ACCOUNT_ID: &str = "8KkpkoDAQaQqjnkCtNXAyk2A8GLmsmWPjBLK7jmahhxZ";
const MINT_ID: &str = "GiGdHFswGhwMsgiHJzARNNiTFXgzLgZYMWYukpSnAUKZ";
const PROGRAM_ID: &str = "Fi21py7sZRjjj6TC2MyRTdB3a4apyFe89nQBQLGKcBBs";
// -------- UPDATE END ---------

pub fn main() {
    let client = RpcClient::new(CLUSTER_ADDRESS.to_owned());

    let payer = read_keypair_file(KEYPAIR_PATH).unwrap();
    let buffer_key = read_keypair_file(BUFFER_PATH).unwrap();
    let bytes = std::fs::read(DEPLOY_PATH).unwrap();

    let timelock_program_account_key = Pubkey::from_str(TIMELOCK_PROGRAM_ACCOUNT_ID).unwrap();
    let timelock_program_id = Pubkey::from_str(TIMELOCK_PROGRAM_ID).unwrap();
    let program_id = Pubkey::from_str(PROGRAM_ID).unwrap();
    let mint_id = Pubkey::from_str(MINT_ID).unwrap();

    let (authority_key, bump_seed) = Pubkey::find_program_address(
        &[
            timelock_program_account_key.as_ref(),
            mint_id.as_ref(),
            program_id.as_ref(),
        ],
        &timelock_program_id,
    );
    let final_message = do_process_program_partial_upgrade(
        &client,
        &bytes.as_slice(),
        &program_id,
        &payer,
        &buffer_key,
        &authority_key,
    );
    println!("Check out program buffer at {:?}", buffer_key.pubkey());
    println!(
        "This is the upgrade command to paste into your proposal: {:?}:",
        base64::encode(final_message.serialize().as_slice())
    )
}

const DATA_CHUNK_SIZE: usize = 800;

fn do_process_program_partial_upgrade(
    rpc_client: &RpcClient,
    program_data: &[u8],
    program_id: &Pubkey,
    user: &Keypair,
    buffer: &Keypair,
    timelock_authority: &Pubkey,
) -> Message {
    let data_len = program_data.len();
    let minimum_balance = rpc_client
        .get_minimum_balance_for_rent_exemption(
            UpgradeableLoaderState::programdata_len(data_len).unwrap(),
        )
        .unwrap();

    // Build messages to calculate fees
    let mut create_messages: Vec<Message> = vec![Message::new(
        &bpf_loader_upgradeable::create_buffer(
            &user.pubkey(),
            &buffer.pubkey(),
            &user.pubkey(),
            minimum_balance,
            data_len,
        )
        .unwrap()
        .as_slice(),
        Some(&user.pubkey()),
    )];

    let mut write_messages: Vec<Message> = vec![];
    for (chunk, i) in program_data.chunks(DATA_CHUNK_SIZE).zip(0..) {
        write_messages.push(Message::new(
            &[bpf_loader_upgradeable::write(
                &buffer.pubkey(),
                &user.pubkey(),
                (i * DATA_CHUNK_SIZE) as u32,
                chunk.to_vec(),
            )],
            Some(&user.pubkey()),
        ));
    }

    let set_authority_messages: Vec<Message> = vec![
        Message::new(
            &[bpf_loader_upgradeable::set_buffer_authority(
                &buffer.pubkey(),
                &user.pubkey(),
                &timelock_authority,
            )],
            Some(&user.pubkey()),
        ),
        Message::new(
            &[bpf_loader_upgradeable::set_upgrade_authority(
                &program_id,
                &user.pubkey(),
                Some(timelock_authority),
            )],
            Some(&user.pubkey()),
        ),
    ];

    let final_message = Message::new(
        &[bpf_loader_upgradeable::upgrade(
            program_id,
            &buffer.pubkey(),
            &timelock_authority,
            &timelock_authority,
        )],
        Some(&user.pubkey()),
    );
    send_deploy_messages(
        &rpc_client,
        &create_messages,
        &write_messages,
        &set_authority_messages,
        &buffer,
        &user,
    )
    .unwrap();
    return final_message;
}
fn send_deploy_messages(
    rpc_client: &RpcClient,
    create_messages: &Vec<Message>,
    write_messages: &Vec<Message>,
    set_authority_messages: &Vec<Message>,
    buffer_signer: &Keypair,
    user: &Keypair,
) -> Result<(), Box<dyn std::error::Error>> {
    let (blockhash, _, last_valid_slot) = rpc_client
        .get_recent_blockhash_with_commitment(CommitmentConfig::confirmed())?
        .value;
    let mut write_transactions = vec![];
    for message in create_messages.iter() {
        let mut tx = Transaction::new_unsigned(message.clone());
        tx.try_sign(&[user, buffer_signer], blockhash)?;
        write_transactions.push(tx);
    }
    for message in write_messages.iter() {
        let mut tx = Transaction::new_unsigned(message.clone());
        tx.try_sign(&[user], blockhash)?;
        write_transactions.push(tx);
    }
    send_and_confirm_transactions_with_spinner(
        &rpc_client,
        write_transactions,
        &[user, buffer_signer],
        CommitmentConfig::confirmed(),
        last_valid_slot,
    )
    .map_err(|err| format!("Data writes to account failed: {}", err))?;

    let mut write_transactions = vec![];

    for message in set_authority_messages.iter() {
        let mut tx = Transaction::new_unsigned(message.clone());
        tx.try_sign(&[user], blockhash)?;
        write_transactions.push(tx);
    }

    send_and_confirm_transactions_with_spinner(
        &rpc_client,
        write_transactions,
        &[user, buffer_signer],
        CommitmentConfig::confirmed(),
        last_valid_slot,
    )
    .map_err(|err| format!("Data writes to account failed: {}", err))?;

    Ok(())
}

fn send_and_confirm_transactions_with_spinner<T: Signers>(
    rpc_client: &RpcClient,
    mut transactions: Vec<Transaction>,
    signer_keys: &T,
    commitment: CommitmentConfig,
    mut last_valid_slot: Slot,
) -> Result<(), Box<dyn error::Error>> {
    let mut send_retries = 5;
    let mut leader_schedule: Option<RpcLeaderSchedule> = None;
    let mut leader_schedule_epoch = 0;
    let send_socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    let cluster_nodes = rpc_client.get_cluster_nodes().ok();

    loop {
        msg!("Finding leader node...");
        let epoch_info = rpc_client.get_epoch_info()?;
        let mut slot = epoch_info.absolute_slot;
        let mut last_epoch_fetch = Instant::now();
        if epoch_info.epoch > leader_schedule_epoch || leader_schedule.is_none() {
            leader_schedule = rpc_client.get_leader_schedule(Some(epoch_info.absolute_slot))?;
            leader_schedule_epoch = epoch_info.epoch;
        }

        let mut tpu_address = get_leader_tpu(
            min(epoch_info.slot_index + 1, epoch_info.slots_in_epoch),
            leader_schedule.as_ref(),
            cluster_nodes.as_ref(),
        );

        // Send all transactions
        let mut pending_transactions = HashMap::new();
        let num_transactions = transactions.len();
        for transaction in transactions {
            if let Some(tpu_address) = tpu_address {
                let wire_transaction =
                    serialize(&transaction).expect("serialization should succeed");
                send_transaction_tpu(&send_socket, &tpu_address, &wire_transaction);
            } else {
                let _result = rpc_client
                    .send_transaction_with_config(
                        &transaction,
                        RpcSendTransactionConfig {
                            preflight_commitment: Some(commitment.commitment),
                            ..RpcSendTransactionConfig::default()
                        },
                    )
                    .ok();
            }
            pending_transactions.insert(transaction.signatures[0], transaction);
            msg!(&format!(
                "[{}/{}] Transactions sent",
                pending_transactions.len(),
                num_transactions
            ));

            // Throttle transactions to about 100 TPS
            sleep(Duration::from_millis(10));

            // Update leader periodically
            if last_epoch_fetch.elapsed() > Duration::from_millis(400) {
                let epoch_info = rpc_client.get_epoch_info()?;
                last_epoch_fetch = Instant::now();
                tpu_address = get_leader_tpu(
                    min(epoch_info.slot_index + 1, epoch_info.slots_in_epoch),
                    leader_schedule.as_ref(),
                    cluster_nodes.as_ref(),
                );
            }
        }

        // Collect statuses for all the transactions, drop those that are confirmed
        loop {
            let pending_signatures = pending_transactions.keys().cloned().collect::<Vec<_>>();
            for pending_signatures_chunk in
                pending_signatures.chunks(MAX_GET_SIGNATURE_STATUSES_QUERY_ITEMS)
            {
                if let Ok(result) =
                    rpc_client.get_signature_statuses_with_history(pending_signatures_chunk)
                {
                    let statuses = result.value;
                    for (signature, status) in
                        pending_signatures_chunk.iter().zip(statuses.into_iter())
                    {
                        if let Some(status) = status {
                            if let Some(confirmation_status) = &status.confirmation_status {
                                if *confirmation_status != TransactionConfirmationStatus::Processed
                                {
                                    let _ = pending_transactions.remove(signature);
                                }
                            } else if status.confirmations.is_none()
                                || status.confirmations.unwrap() > 1
                            {
                                let _ = pending_transactions.remove(signature);
                            }
                        }
                    }
                }

                slot = rpc_client.get_slot()?;
                msg!(&format!(
                    "[{}/{}] Transactions confirmed. Retrying in {} slots",
                    num_transactions - pending_transactions.len(),
                    num_transactions,
                    last_valid_slot.saturating_sub(slot)
                ));
            }

            if pending_transactions.is_empty() {
                return Ok(());
            }

            if slot > last_valid_slot {
                break;
            }

            let epoch_info = rpc_client.get_epoch_info()?;
            tpu_address = get_leader_tpu(
                min(epoch_info.slot_index + 1, epoch_info.slots_in_epoch),
                leader_schedule.as_ref(),
                cluster_nodes.as_ref(),
            );

            for transaction in pending_transactions.values() {
                if let Some(tpu_address) = tpu_address {
                    let wire_transaction =
                        serialize(transaction).expect("serialization should succeed");
                    send_transaction_tpu(&send_socket, &tpu_address, &wire_transaction);
                } else {
                    let _result = rpc_client
                        .send_transaction_with_config(
                            transaction,
                            RpcSendTransactionConfig {
                                preflight_commitment: Some(commitment.commitment),
                                ..RpcSendTransactionConfig::default()
                            },
                        )
                        .ok();
                }
            }

            if cfg!(not(test)) {
                // Retry twice a second
                sleep(Duration::from_millis(500));
            }
        }

        if send_retries == 0 {
            return Err("Transactions failed".into());
        }
        send_retries -= 1;

        // Re-sign any failed transactions with a new blockhash and retry
        let (blockhash, _fee_calculator, new_last_valid_slot) = rpc_client
            .get_recent_blockhash_with_commitment(commitment)?
            .value;
        last_valid_slot = new_last_valid_slot;
        transactions = vec![];
        for (_, mut transaction) in pending_transactions.into_iter() {
            transaction.try_sign(signer_keys, blockhash)?;
            transactions.push(transaction);
        }
    }
}

pub fn get_leader_tpu(
    slot_index: u64,
    leader_schedule: Option<&RpcLeaderSchedule>,
    cluster_nodes: Option<&Vec<RpcContactInfo>>,
) -> Option<SocketAddr> {
    leader_schedule?
        .iter()
        .find(|(_pubkey, slots)| slots.iter().any(|slot| *slot as u64 == slot_index))
        .and_then(|(pubkey, _)| {
            cluster_nodes?
                .iter()
                .find(|contact_info| contact_info.pubkey == *pubkey)
                .and_then(|contact_info| contact_info.tpu)
        })
}

pub fn send_transaction_tpu(
    send_socket: &UdpSocket,
    tpu_address: &SocketAddr,
    wire_transaction: &[u8],
) {
    if let Err(err) = send_socket.send_to(wire_transaction, tpu_address) {
        msg!("Failed to send transaction to {}: {:?}", tpu_address, err);
    }
}
