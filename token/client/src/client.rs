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
/// Required for `ProgramBanksClient`.
pub trait SendTransactionBanksClient: SendTransaction {
    fn send<'a>(
        &self,
        client: &'a mut BanksClient,
        transaction: Transaction,
    ) -> BoxFuture<'a, ProgramClientResult<Self::Output>>;
}

/// Send transaction to validator using `BanksClient::process_transaction`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProgramBanksClientProcessTransaction;

impl SendTransaction for ProgramBanksClientProcessTransaction {
    type Output = ();
}

impl SendTransactionBanksClient for ProgramBanksClientProcessTransaction {
    fn send<'a>(
        &self,
        client: &'a mut BanksClient,
        transaction: Transaction,
    ) -> BoxFuture<'a, ProgramClientResult<Self::Output>> {
        Box::pin(async move {
            client
                .process_transaction(transaction)
                .await
                .map_err(Into::into)
        })
    }
}

/// Extends basic `SendTransaction` trait with function `send` where client is `&RpcClient`.
/// Required for `ProgramRpcClient`.
pub trait SendTransactionRpc: SendTransaction {
    fn send<'a>(
        &self,
        client: &'a RpcClient,
        transaction: &'a Transaction,
    ) -> BoxFuture<'a, ProgramClientResult<Self::Output>>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ProgramRpcClientSendTransaction;

impl SendTransaction for ProgramRpcClientSendTransaction {
    type Output = Signature;
}

impl SendTransactionRpc for ProgramRpcClientSendTransaction {
    fn send<'a>(
        &self,
        client: &'a RpcClient,
        transaction: &'a Transaction,
    ) -> BoxFuture<'a, ProgramClientResult<Self::Output>> {
        Box::pin(async move { client.send_transaction(transaction).map_err(Into::into) })
    }
}

//
pub type ProgramClientError = Box<dyn std::error::Error + Send + Sync>;
pub type ProgramClientResult<T> = Result<T, ProgramClientError>;

/// Generic client interface for programs.
#[async_trait]
pub trait ProgramClient<ST>
where
    ST: SendTransaction,
{
    async fn get_minimum_balance_for_rent_exemption(
        &self,
        data_len: usize,
    ) -> ProgramClientResult<u64>;

    async fn get_latest_blockhash(&self) -> ProgramClientResult<Hash>;

    async fn send_transaction(&self, transaction: &Transaction) -> ProgramClientResult<ST::Output>;

    async fn get_account(&self, address: Pubkey) -> ProgramClientResult<Option<Account>>;
}

enum ProgramBanksClientContext {
    Client(Arc<Mutex<BanksClient>>),
    Context(Arc<Mutex<ProgramTestContext>>),
}

/// Program client for `BanksClient` from crate `solana-program-test`.
pub struct ProgramBanksClient<ST> {
    context: ProgramBanksClientContext,
    send: ST,
}

impl<ST> fmt::Debug for ProgramBanksClient<ST> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProgramBanksClient").finish()
    }
}

impl<ST> ProgramBanksClient<ST> {
    fn new(context: ProgramBanksClientContext, send: ST) -> Self {
        Self { context, send }
    }

    pub fn new_from_client(client: Arc<Mutex<BanksClient>>, send: ST) -> Self {
        Self::new(ProgramBanksClientContext::Client(client), send)
    }

    pub fn new_from_context(context: Arc<Mutex<ProgramTestContext>>, send: ST) -> Self {
        Self::new(ProgramBanksClientContext::Context(context), send)
    }

    async fn run_in_lock<F, O>(&self, f: F) -> O
    where
        for<'a> F: Fn(&'a mut BanksClient) -> BoxFuture<'a, O>,
    {
        match &self.context {
            ProgramBanksClientContext::Client(client) => {
                let mut lock = client.lock().await;
                f(&mut lock).await
            }
            ProgramBanksClientContext::Context(context) => {
                let mut lock = context.lock().await;
                f(&mut lock.banks_client).await
            }
        }
    }
}

#[async_trait]
impl<ST> ProgramClient<ST> for ProgramBanksClient<ST>
where
    ST: SendTransactionBanksClient + Send + Sync,
{
    async fn get_minimum_balance_for_rent_exemption(
        &self,
        data_len: usize,
    ) -> ProgramClientResult<u64> {
        self.run_in_lock(|client| {
            Box::pin(async move {
                let rent = client.get_rent().await?;
                Ok(rent.minimum_balance(data_len))
            })
        })
        .await
    }

    async fn get_latest_blockhash(&self) -> ProgramClientResult<Hash> {
        self.run_in_lock(|client| {
            Box::pin(async move { client.get_latest_blockhash().await.map_err(Into::into) })
        })
        .await
    }

    async fn send_transaction(&self, transaction: &Transaction) -> ProgramClientResult<ST::Output> {
        self.run_in_lock(|client| {
            let transaction = transaction.clone();
            self.send.send(client, transaction)
        })
        .await
    }

    async fn get_account(&self, address: Pubkey) -> ProgramClientResult<Option<Account>> {
        self.run_in_lock(|client| {
            Box::pin(async move { client.get_account(address).await.map_err(Into::into) })
        })
        .await
    }
}

/// Program client for `RpcClient` from crate `solana-client`.
pub struct ProgramRpcClient<'a, ST> {
    client: &'a RpcClient,
    send: ST,
}

impl<ST> fmt::Debug for ProgramRpcClient<'_, ST> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProgramRpcClient").finish()
    }
}

impl<'a, ST> ProgramRpcClient<'a, ST> {
    pub fn new(client: &'a RpcClient, send: ST) -> Self {
        Self { client, send }
    }
}

#[async_trait]
impl<ST> ProgramClient<ST> for ProgramRpcClient<'_, ST>
where
    ST: SendTransactionRpc + Send + Sync,
{
    async fn get_minimum_balance_for_rent_exemption(
        &self,
        data_len: usize,
    ) -> ProgramClientResult<u64> {
        self.client
            .get_minimum_balance_for_rent_exemption(data_len)
            .map_err(Into::into)
    }

    async fn get_latest_blockhash(&self) -> ProgramClientResult<Hash> {
        self.client.get_latest_blockhash().map_err(Into::into)
    }

    async fn send_transaction(&self, transaction: &Transaction) -> ProgramClientResult<ST::Output> {
        self.send.send(self.client, transaction).await
    }

    async fn get_account(&self, address: Pubkey) -> ProgramClientResult<Option<Account>> {
        Ok(self
            .client
            .get_account_with_commitment(&address, self.client.commitment())?
            .value)
    }
}
