use async_trait::async_trait;
use solana_client::rpc_client::RpcClient;
use solana_program_test::{tokio::sync::Mutex, BanksClient, ProgramTestContext};
use solana_sdk::{
    account::Account, hash::Hash, pubkey::Pubkey, signature::Signature, transaction::Transaction,
};
use std::{fmt, future::Future, pin::Pin, sync::Arc};

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Basic trait for sending transactions to validator.
pub trait SendTransaction {
    type Output;
}

/// Extends basic `SendTransaction` trait with function `send` where client is `&mut BanksClient`.
/// Required for `TokenBanksClient`.
pub trait SendTransactionBanksClient: SendTransaction {
    fn send<'a>(
        &self,
        client: &'a mut BanksClient,
        transaction: Transaction,
    ) -> BoxFuture<'a, TokenClientResult<Self::Output>>;
}

/// Send transaction to validator using `BanksClient::process_transaction`.
#[derive(Debug, Clone, Copy, Default)]
pub struct TokenBanksClientProcessTransaction;

impl SendTransaction for TokenBanksClientProcessTransaction {
    type Output = ();
}

impl SendTransactionBanksClient for TokenBanksClientProcessTransaction {
    fn send<'a>(
        &self,
        client: &'a mut BanksClient,
        transaction: Transaction,
    ) -> BoxFuture<'a, TokenClientResult<Self::Output>> {
        Box::pin(async move {
            client
                .process_transaction(transaction)
                .await
                .map_err(Into::into)
        })
    }
}

/// Extends basic `SendTransaction` trait with function `send` where client is `&RpcClient`.
/// Required for `TokenRpcClient`.
pub trait SendTransactionRpc: SendTransaction {
    fn send<'a>(
        &self,
        client: &'a RpcClient,
        transaction: Transaction,
    ) -> BoxFuture<'a, TokenClientResult<Self::Output>>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TokenRpcClientSendTransaction;

impl SendTransaction for TokenRpcClientSendTransaction {
    type Output = Signature;
}

impl SendTransactionRpc for TokenRpcClientSendTransaction {
    fn send<'a>(
        &self,
        client: &'a RpcClient,
        transaction: Transaction,
    ) -> BoxFuture<'a, TokenClientResult<Self::Output>> {
        Box::pin(async move { client.send_transaction(&transaction).map_err(Into::into) })
    }
}

//
pub type TokenClientError = Box<dyn std::error::Error + Send + Sync>;
pub type TokenClientResult<T> = Result<T, TokenClientError>;

/// Token client interface.
#[async_trait]
pub trait TokenClient<ST>
where
    ST: SendTransaction,
{
    async fn get_minimum_balance_for_rent_exemption(
        &self,
        data_len: usize,
    ) -> TokenClientResult<u64>;

    async fn get_recent_blockhash(&self) -> TokenClientResult<Hash>;

    async fn send_transaction(&self, transaction: Transaction) -> TokenClientResult<ST::Output>;

    async fn get_account(&self, address: Pubkey) -> TokenClientResult<Option<Account>>;
}

enum TokenBanksClientContext {
    Client(Arc<Mutex<BanksClient>>),
    Context(Arc<Mutex<ProgramTestContext>>),
}

/// Token client for `BanksClient` from crate `solana-program-test`.
pub struct TokenBanksClient<ST> {
    context: TokenBanksClientContext,
    send: ST,
}

impl<ST> fmt::Debug for TokenBanksClient<ST> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TokenBanksClient").finish()
    }
}

impl<ST> TokenBanksClient<ST> {
    fn new(context: TokenBanksClientContext, send: ST) -> Self {
        Self { context, send }
    }

    pub fn new_from_client(client: Arc<Mutex<BanksClient>>, send: ST) -> Self {
        Self::new(TokenBanksClientContext::Client(client), send)
    }

    pub fn new_from_context(context: Arc<Mutex<ProgramTestContext>>, send: ST) -> Self {
        Self::new(TokenBanksClientContext::Context(context), send)
    }

    async fn run_in_lock<F, O>(&self, f: F) -> O
    where
        for<'a> F: Fn(&'a mut BanksClient) -> BoxFuture<'a, O>,
    {
        match &self.context {
            TokenBanksClientContext::Client(client) => {
                let mut lock = client.lock().await;
                f(&mut lock).await
            }
            TokenBanksClientContext::Context(context) => {
                let mut lock = context.lock().await;
                f(&mut lock.banks_client).await
            }
        }
    }
}

#[async_trait]
impl<ST> TokenClient<ST> for TokenBanksClient<ST>
where
    ST: SendTransactionBanksClient + Send + Sync,
{
    async fn get_minimum_balance_for_rent_exemption(
        &self,
        data_len: usize,
    ) -> TokenClientResult<u64> {
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

    async fn send_transaction(&self, transaction: Transaction) -> TokenClientResult<ST::Output> {
        self.run_in_lock(|client| {
            let transaction = transaction.clone(); // How to remove extra clone?
            self.send.send(client, transaction)
        })
        .await
    }

    async fn get_account(&self, address: Pubkey) -> TokenClientResult<Option<Account>> {
        self.run_in_lock(|client| {
            Box::pin(async move { client.get_account(address).await.map_err(Into::into) })
        })
        .await
    }
}

/// Token client for `RpcClient` from crate `solana-client`.
pub struct TokenRpcClient<'a, ST> {
    client: &'a RpcClient,
    send: ST,
}

impl<ST> fmt::Debug for TokenRpcClient<'_, ST> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TokenRpcClient").finish()
    }
}

impl<'a, ST> TokenRpcClient<'a, ST> {
    pub fn new(client: &'a RpcClient, send: ST) -> Self {
        Self { client, send }
    }
}

#[async_trait]
impl<ST> TokenClient<ST> for TokenRpcClient<'_, ST>
where
    ST: SendTransactionRpc + Send + Sync,
{
    async fn get_minimum_balance_for_rent_exemption(
        &self,
        data_len: usize,
    ) -> TokenClientResult<u64> {
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

    async fn send_transaction(&self, transaction: Transaction) -> TokenClientResult<ST::Output> {
        self.send.send(self.client, transaction).await
    }

    async fn get_account(&self, address: Pubkey) -> TokenClientResult<Option<Account>> {
        Ok(self
            .client
            .get_account_with_commitment(&address, self.client.commitment())?
            .value)
    }
}
