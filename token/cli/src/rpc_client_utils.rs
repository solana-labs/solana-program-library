// TODO: In v1.8 timeframe delete this module and use `send_and_confirm_messages_with_spinner()`
//       from the Solana monorepo
use {
    solana_client::{
        rpc_client::RpcClient,
        rpc_config::RpcSendTransactionConfig,
        rpc_request::MAX_GET_SIGNATURE_STATUSES_QUERY_ITEMS,
        rpc_response::Fees,
        tpu_client::{TpuClient, TpuClientConfig},
    },
    solana_sdk::{
        message::Message,
        signers::Signers,
        transaction::{Transaction, TransactionError},
    },
    solana_transaction_status::TransactionConfirmationStatus,
    std::{collections::HashMap, error, sync::Arc, thread::sleep, time::Duration},
};

/// TODO: In v1.8 timeframe switch to using `solana_cli_output::display::new_spinner_progress_bar()`
fn new_spinner_progress_bar() -> indicatif::ProgressBar {
    let progress_bar = indicatif::ProgressBar::new(42);
    progress_bar.set_style(
        indicatif::ProgressStyle::default_spinner().template("{spinner:.green} {wide_msg}"),
    );
    progress_bar.enable_steady_tick(100);
    progress_bar
}

pub fn send_and_confirm_messages_with_spinner<T: Signers>(
    rpc_client: Arc<RpcClient>,
    websocket_url: &str,
    messages: &[Message],
    signers: &T,
) -> Result<Vec<Option<TransactionError>>, Box<dyn error::Error>> {
    let progress_bar = new_spinner_progress_bar();
    let mut expired_blockhash_retries = 5;
    let send_transaction_interval = Duration::from_millis(10); /* ~100 TPS */

    progress_bar.set_message("Connecting...");
    let tpu_client = TpuClient::new(
        rpc_client.clone(),
        websocket_url,
        TpuClientConfig::default(),
    )?;

    let mut transactions = messages
        .iter()
        .enumerate()
        .map(|(i, message)| (i, Transaction::new_unsigned(message.clone())))
        .collect::<Vec<_>>();
    let mut transaction_errors = vec![None; messages.len()];
    let set_message =
        |confirmed_transactions, block_height: u64, last_valid_block_height: u64, status: &str| {
            progress_bar.set_message(format!(
                "{:>5.1}% | {:<40}[block height {}; block hash valid for {} blocks]",
                confirmed_transactions as f64 * 100. / messages.len() as f64,
                status,
                block_height,
                last_valid_block_height.saturating_sub(block_height),
            ));
        };

    let mut confirmed_transactions = 0;
    let mut block_height = rpc_client.get_block_height()?;
    while expired_blockhash_retries > 0 {
        let Fees {
            blockhash,
            fee_calculator: _,
            last_valid_block_height,
        } = rpc_client.get_fees()?;

        let mut pending_transactions = HashMap::new();
        for (i, mut transaction) in transactions {
            transaction.try_sign(signers, blockhash)?;
            pending_transactions.insert(transaction.signatures[0], (i, transaction));
        }

        loop {
            // Send all pending transactions
            let num_transactions = pending_transactions.len();
            for (index, (_i, transaction)) in pending_transactions.values().enumerate() {
                if !tpu_client.send_transaction(transaction) {
                    let _ = rpc_client.send_transaction_with_config(
                        transaction,
                        RpcSendTransactionConfig {
                            skip_preflight: true,
                            ..RpcSendTransactionConfig::default()
                        },
                    );
                }
                set_message(
                    confirmed_transactions,
                    block_height,
                    last_valid_block_height,
                    &format!("Sending {}/{} transactions", index + 1, num_transactions,),
                );
                sleep(send_transaction_interval);
            }

            // Wait for the next block before checking fro transaction statuses
            set_message(
                confirmed_transactions,
                block_height,
                last_valid_block_height,
                &format!("Waiting for next block, {} pending...", num_transactions),
            );

            block_height = rpc_client.get_block_height()?;
            let mut new_block_height = block_height;
            while block_height == new_block_height {
                sleep(Duration::from_millis(200));
                new_block_height = rpc_client.get_block_height()?;
            }

            if new_block_height > last_valid_block_height {
                break;
            }

            // Collect statuses for the transactions, drop those that are confirmed
            let pending_signatures = pending_transactions.keys().cloned().collect::<Vec<_>>();
            for pending_signatures_chunk in
                pending_signatures.chunks(MAX_GET_SIGNATURE_STATUSES_QUERY_ITEMS)
            {
                if let Ok(result) = rpc_client.get_signature_statuses(pending_signatures_chunk) {
                    let statuses = result.value;
                    for (signature, status) in
                        pending_signatures_chunk.iter().zip(statuses.into_iter())
                    {
                        if let Some(status) = status {
                            if let Some(confirmation_status) = &status.confirmation_status {
                                if *confirmation_status != TransactionConfirmationStatus::Processed
                                {
                                    if let Some((i, _)) = pending_transactions.remove(signature) {
                                        confirmed_transactions += 1;
                                        transaction_errors[i] = status.err;
                                    }
                                }
                            } else if status.confirmations.is_none()
                                || status.confirmations.unwrap() > 1
                            {
                                if let Some((i, _)) = pending_transactions.remove(signature) {
                                    confirmed_transactions += 1;
                                    transaction_errors[i] = status.err;
                                }
                            }
                        }
                    }
                }
                set_message(
                    confirmed_transactions,
                    block_height,
                    last_valid_block_height,
                    "Checking transaction status...",
                );
            }

            if pending_transactions.is_empty() {
                return Ok(transaction_errors);
            }
        }

        transactions = pending_transactions.into_iter().map(|(_k, v)| v).collect();
        progress_bar.println(format!(
            "Blockhash expired. {} retries remaining",
            expired_blockhash_retries
        ));
        expired_blockhash_retries -= 1;
    }
    Err("Max retries exceeded".into())
}
