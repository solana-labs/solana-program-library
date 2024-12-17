use {
    async_trait::async_trait,
    solana_banks_interface::BanksTransactionResultWithSimulation,
    solana_program_test::{tokio::sync::Mutex, BanksClient, ProgramTestContext},
    solana_rpc_client::nonblocking::rpc_client::RpcClient,
    solana_rpc_client_api::response::RpcSimulateTransactionResult,
    solana_sdk::{
        account::Account, hash::Hash, pubkey::Pubkey, signature::Signature,
        transaction::Transaction,
    },
    std::{fmt, future::Future, pin::Pin, sync::Arc},
};

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Basic trait for sending transactions to validator.
pub trait SendTransaction {
    type Output;
}

/// Basic trait for simulating transactions in a validator.
pub trait SimulateTransaction {
    type SimulationOutput: SimulationResult;
}

/// Trait for the output of a simulation
pub trait SimulationResult {
    fn get_compute_units_consumed(&self) -> ProgramClientResult<u64>;
}

/// Extends basic `SendTransaction` trait with function `send` where client is
/// `&mut BanksClient`. Required for `ProgramBanksClient`.
pub trait SendTransactionBanksClient: SendTransaction {
    fn send<'a>(
        &self,
        client: &'a mut BanksClient,
        transaction: Transaction,
    ) -> BoxFuture<'a, ProgramClientResult<Self::Output>>;
}

/// Extends basic `SimulateTransaction` trait with function `simulation` where
/// client is `&mut BanksClient`. Required for `ProgramBanksClient`.
pub trait SimulateTransactionBanksClient: SimulateTransaction {
    fn simulate<'a>(
        &self,
        client: &'a mut BanksClient,
        transaction: Transaction,
    ) -> BoxFuture<'a, ProgramClientResult<Self::SimulationOutput>>;
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

impl SimulationResult for BanksTransactionResultWithSimulation {
    fn get_compute_units_consumed(&self) -> ProgramClientResult<u64> {
        self.simulation_details
            .as_ref()
            .map(|x| x.units_consumed)
            .ok_or("No simulation results found".into())
    }
}

impl SimulateTransaction for ProgramBanksClientProcessTransaction {
    type SimulationOutput = BanksTransactionResultWithSimulation;
}

impl SimulateTransactionBanksClient for ProgramBanksClientProcessTransaction {
    fn simulate<'a>(
        &self,
        client: &'a mut BanksClient,
        transaction: Transaction,
    ) -> BoxFuture<'a, ProgramClientResult<Self::SimulationOutput>> {
        Box::pin(async move {
            client
                .simulate_transaction(transaction)
                .await
                .map_err(Into::into)
        })
    }
}

/// Extends basic `SendTransaction` trait with function `send` where client is
/// `&RpcClient`. Required for `ProgramRpcClient`.
pub trait SendTransactionRpc: SendTransaction {
    fn send<'a>(
        &self,
        client: &'a RpcClient,
        transaction: &'a Transaction,
    ) -> BoxFuture<'a, ProgramClientResult<Self::Output>>;
}

/// Extends basic `SimulateTransaction` trait with function `simulate` where
/// client is `&RpcClient`. Required for `ProgramRpcClient`.
pub trait SimulateTransactionRpc: SimulateTransaction {
    fn simulate<'a>(
        &self,
        client: &'a RpcClient,
        transaction: &'a Transaction,
    ) -> BoxFuture<'a, ProgramClientResult<Self::SimulationOutput>>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ProgramRpcClientSendTransaction {
    /// Confirm the transaction after sending it
    confirm: bool,
}

impl ProgramRpcClientSendTransaction {
    /// Create an instance that sends the transaction **without** waiting for
    /// confirmation.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an instance that sends and confirms the transaction.
    pub fn new_with_confirmation() -> Self {
        Self { confirm: true }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RpcClientResponse {
    Signature(Signature),
    Transaction(Transaction),
    Simulation(RpcSimulateTransactionResult),
}

impl SendTransaction for ProgramRpcClientSendTransaction {
    type Output = RpcClientResponse;
}

impl SendTransactionRpc for ProgramRpcClientSendTransaction {
    fn send<'a>(
        &self,
        client: &'a RpcClient,
        transaction: &'a Transaction,
    ) -> BoxFuture<'a, ProgramClientResult<Self::Output>> {
        let confirm = self.confirm;
        Box::pin(async move {
            if !transaction.is_signed() {
                return Err("Cannot send transaction: not fully signed".into());
            }

            if confirm {
                client.send_and_confirm_transaction(transaction).await
            } else {
                client.send_transaction(transaction).await
            }
            .map(RpcClientResponse::Signature)
            .map_err(Into::into)
        })
    }
}

impl SimulationResult for RpcClientResponse {
    fn get_compute_units_consumed(&self) -> ProgramClientResult<u64> {
        match self {
            // `Transaction` is the result of an offline simulation. The error
            // should be properly handled by a caller that supports offline
            // signing
            Self::Signature(_) | Self::Transaction(_) => Err("Not a simulation result".into()),
            Self::Simulation(simulation_result) => simulation_result
                .units_consumed
                .ok_or("No simulation results found".into()),
        }
    }
}

impl SimulateTransaction for ProgramRpcClientSendTransaction {
    type SimulationOutput = RpcClientResponse;
}

impl SimulateTransactionRpc for ProgramRpcClientSendTransaction {
    fn simulate<'a>(
        &self,
        client: &'a RpcClient,
        transaction: &'a Transaction,
    ) -> BoxFuture<'a, ProgramClientResult<Self::SimulationOutput>> {
        Box::pin(async move {
            client
                .simulate_transaction(transaction)
                .await
                .map(|r| RpcClientResponse::Simulation(r.value))
                .map_err(Into::into)
        })
    }
}

pub type ProgramClientError = Box<dyn std::error::Error + Send + Sync>;
pub type ProgramClientResult<T> = Result<T, ProgramClientError>;

/// Generic client interface for programs.
#[async_trait]
pub trait ProgramClient<ST>
where
    ST: SendTransaction + SimulateTransaction,
{
    async fn get_minimum_balance_for_rent_exemption(
        &self,
        data_len: usize,
    ) -> ProgramClientResult<u64>;

    async fn get_latest_blockhash(&self) -> ProgramClientResult<Hash>;

    async fn send_transaction(&self, transaction: &Transaction) -> ProgramClientResult<ST::Output>;

    async fn get_account(&self, address: Pubkey) -> ProgramClientResult<Option<Account>>;

    async fn simulate_transaction(
        &self,
        transaction: &Transaction,
    ) -> ProgramClientResult<ST::SimulationOutput>;
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
    ST: SendTransactionBanksClient + SimulateTransactionBanksClient + Send + Sync,
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

    async fn simulate_transaction(
        &self,
        transaction: &Transaction,
    ) -> ProgramClientResult<ST::SimulationOutput> {
        self.run_in_lock(|client| {
            let transaction = transaction.clone();
            self.send.simulate(client, transaction)
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
pub struct ProgramRpcClient<ST> {
    client: Arc<RpcClient>,
    send: ST,
}

impl<ST> fmt::Debug for ProgramRpcClient<ST> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProgramRpcClient").finish()
    }
}

impl<ST> ProgramRpcClient<ST> {
    pub fn new(client: Arc<RpcClient>, send: ST) -> Self {
        Self { client, send }
    }
}

#[async_trait]
impl<ST> ProgramClient<ST> for ProgramRpcClient<ST>
where
    ST: SendTransactionRpc + SimulateTransactionRpc + Send + Sync,
{
    async fn get_minimum_balance_for_rent_exemption(
        &self,
        data_len: usize,
    ) -> ProgramClientResult<u64> {
        self.client
            .get_minimum_balance_for_rent_exemption(data_len)
            .await
            .map_err(Into::into)
    }

    async fn get_latest_blockhash(&self) -> ProgramClientResult<Hash> {
        self.client.get_latest_blockhash().await.map_err(Into::into)
    }

    async fn send_transaction(&self, transaction: &Transaction) -> ProgramClientResult<ST::Output> {
        self.send.send(&self.client, transaction).await
    }

    async fn simulate_transaction(
        &self,
        transaction: &Transaction,
    ) -> ProgramClientResult<ST::SimulationOutput> {
        self.send.simulate(&self.client, transaction).await
    }

    async fn get_account(&self, address: Pubkey) -> ProgramClientResult<Option<Account>> {
        Ok(self
            .client
            .get_account_with_commitment(&address, self.client.commitment())
            .await?
            .value)
    }
}

/// Program client for offline signing.
pub struct ProgramOfflineClient<ST> {
    blockhash: Hash,
    _send: ST,
}

impl<ST> fmt::Debug for ProgramOfflineClient<ST> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProgramOfflineClient").finish()
    }
}

impl<ST> ProgramOfflineClient<ST> {
    pub fn new(blockhash: Hash, send: ST) -> Self {
        Self {
            blockhash,
            _send: send,
        }
    }
}

#[async_trait]
impl<ST> ProgramClient<ST> for ProgramOfflineClient<ST>
where
    ST: SendTransaction<Output = RpcClientResponse>
        + SimulateTransaction<SimulationOutput = RpcClientResponse>
        + Send
        + Sync,
{
    async fn get_minimum_balance_for_rent_exemption(
        &self,
        _data_len: usize,
    ) -> ProgramClientResult<u64> {
        Err("Unable to fetch minimum balance for rent exemption in offline mode".into())
    }

    async fn get_latest_blockhash(&self) -> ProgramClientResult<Hash> {
        Ok(self.blockhash)
    }

    async fn send_transaction(&self, transaction: &Transaction) -> ProgramClientResult<ST::Output> {
        Ok(RpcClientResponse::Transaction(transaction.clone()))
    }

    async fn simulate_transaction(
        &self,
        transaction: &Transaction,
    ) -> ProgramClientResult<ST::SimulationOutput> {
        Ok(RpcClientResponse::Transaction(transaction.clone()))
    }

    async fn get_account(&self, _address: Pubkey) -> ProgramClientResult<Option<Account>> {
        Err("Unable to fetch account in offline mode".into())
    }
}
