// TODO: In v1.8 timeframe delete this module and use `send_and_confirm_messages_with_spinner()`
//       from the Solana monorepo
use {
    solana_cli_output::display::new_spinner_progress_bar,
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

pub fn send_and_confirm_messages_with_spinner<T: Signers>(
    rpc_client: Arc<RpcClient>,
    websocket_url: &str,
    messages: &[Message],
    signers: &T,
) -> Result<Vec<Option<TransactionError>>, Box<dyn error::Error>> {
    let commitment = rpc_client.commitment();
    let progress_bar = new_spinner_progress_bar();
    let mut send_retries = 5;
    let send_transaction_interval = Duration::from_millis(10); /* ~100 TPS */

    let Fees {
        blockhash,
        fee_calculator: _,
        mut last_valid_block_height,
    } = rpc_client.get_fees()?;

    let mut transactions = vec![];
    let mut transaction_errors = vec![None; messages.len()];
    for (i, message) in messages.iter().enumerate() {
        let mut transaction = Transaction::new_unsigned(message.clone());
        transaction.try_sign(signers, blockhash)?;
        transactions.push((i, transaction));
    }

    progress_bar.set_message("Finding leader nodes...");
    let tpu_client = TpuClient::new(
        rpc_client.clone(),
        websocket_url,
        TpuClientConfig::default(),
    )?;
    loop {
        // Send all transactions
        let mut pending_transactions = HashMap::new();
        let num_transactions = transactions.len();
        for (i, transaction) in transactions {
            if !tpu_client.send_transaction(&transaction) {
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
            pending_transactions.insert(transaction.signatures[0], (i, transaction));
            progress_bar.set_message(&format!(
                "[{}/{}] Transactions sent",
                pending_transactions.len(),
                num_transactions
            ));

            sleep(send_transaction_interval);
        }

        // Collect statuses for all the transactions, drop those that are confirmed
        loop {
            let mut block_height = 0;
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
                                        transaction_errors[i] = status.err;
                                    }
                                }
                            } else if status.confirmations.is_none()
                                || status.confirmations.unwrap() > 1
                            {
                                if let Some((i, _)) = pending_transactions.remove(signature) {
                                    transaction_errors[i] = status.err;
                                }
                            }
                        }
                    }
                }

                block_height = rpc_client.get_block_height()?;
                progress_bar.set_message(&format!(
                    "[{}/{}] Transactions confirmed. Retrying in {} blocks",
                    num_transactions - pending_transactions.len(),
                    num_transactions,
                    last_valid_block_height.saturating_sub(block_height)
                ));
            }

            if pending_transactions.is_empty() {
                return Ok(transaction_errors);
            }

            if block_height > last_valid_block_height {
                break;
            }

            for (_i, transaction) in pending_transactions.values() {
                if !tpu_client.send_transaction(transaction) {
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
        let Fees {
            blockhash,
            fee_calculator: _,
            last_valid_block_height: new_last_valid_block_height,
        } = rpc_client.get_fees()?;

        last_valid_block_height = new_last_valid_block_height;
        transactions = vec![];
        for (_, (i, mut transaction)) in pending_transactions.into_iter() {
            transaction.try_sign(signers, blockhash)?;
            transactions.push((i, transaction));
        }
    }
}
