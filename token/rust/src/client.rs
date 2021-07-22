use async_trait::async_trait;
use solana_client::rpc_client::RpcClient;
use solana_program_test::{
    tokio::sync::Mutex, BanksClient, ProgramTestContext,
};
use solana_sdk::{hash::Hash, transaction::Transaction};
use std::{fmt, future::Future, pin::Pin, sync::Arc};

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub type TokenClientError = Box<dyn std::error::Error>;
pub type TokenClientResult<T> = Result<T, TokenClientError>;

#[async_trait]
pub trait TokenClient {
    async fn get_minimum_balance_for_rent_exemption(&self, data_len: usize) -> TokenClientResult<u64>;
    async fn get_recent_blockhash(&self) -> TokenClientResult<Hash>;

    async fn send_transaction(&self, transaction: &Transaction) -> TokenClientResult<()>;
}

pub struct TokenBanksClient {
    client: Option<Arc<Mutex<BanksClient>>>,
    context: Option<Arc<Mutex<ProgramTestContext>>>,
}

impl fmt::Debug for TokenBanksClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TokenBanksClient").finish()
    }
}

impl TokenBanksClient {
    pub fn new(client: Arc<Mutex<BanksClient>>) -> Self {
        Self {
            client: Some(client),
            context: None,
        }
    }

    pub fn new_from_context(context: Arc<Mutex<ProgramTestContext>>) -> Self {
        Self {
            client: None,
            context: Some(context),
        }
    }

    async fn run_in_lock<F, O>(&self, f: F) -> O
    where
        for<'a> F: Fn(&'a mut BanksClient) -> BoxFuture<'a, O>,
    {
        match (self.client.as_ref(), self.context.as_ref()) {
            (None, None) => unreachable!(),
            (None, Some(context)) => {
                let mut lock = context.lock().await;
                f(&mut lock.banks_client).await
            }
            (Some(client), None) => {
                let mut lock = client.lock().await;
                f(&mut lock).await
            }
            (Some(_), Some(_)) => unreachable!(),
        }
    }
}

#[async_trait]
impl TokenClient for TokenBanksClient {
    async fn get_minimum_balance_for_rent_exemption(&self, data_len: usize) -> TokenClientResult<u64> {
        self.run_in_lock(|client| {
            Box::pin(async move {
                let rent = client.get_rent().await?;
                Ok(rent.minimum_balance(data_len))
            })
        })
        .await
    }

    async fn get_recent_blockhash(&self) -> TokenClientResult<Hash> {
        self.run_in_lock(|client| {
            Box::pin(async move { client.get_recent_blockhash().await.map_err(Into::into) })
        })
        .await
    }

    async fn send_transaction(&self, transaction: &Transaction) -> TokenClientResult<()> {
        self.run_in_lock(|client| {
            let transaction = transaction.clone();
            Box::pin(async move {
                client
                    .process_transaction(transaction)
                    .await
                    .map_err(Into::into)
            })
        })
        .await
    }
}

pub struct TokenRpcClient<'a> {
    client: &'a RpcClient,
}

impl fmt::Debug for TokenRpcClient<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TokenRpcClient").finish()
    }
}

impl<'a> TokenRpcClient<'a> {
    pub fn new(client: &'a RpcClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl TokenClient for TokenRpcClient<'_> {
    async fn get_minimum_balance_for_rent_exemption(&self, data_len: usize) -> TokenClientResult<u64> {
        self.client
            .get_minimum_balance_for_rent_exemption(data_len)
            .map_err(Into::into)
    }

    async fn get_recent_blockhash(&self) -> TokenClientResult<Hash> {
        self.client
            .get_recent_blockhash()
            .map(|(hash, _fee_calculator)| hash)
            .map_err(Into::into)
    }

    async fn send_transaction(&self, transaction: &Transaction) -> TokenClientResult<()> {
        self.client
            .send_transaction(transaction)
            .map(|_signature| ())
            .map_err(Into::into)
    }
}
