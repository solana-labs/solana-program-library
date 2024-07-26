/// Basic trait for sending transactions to validators
pub trait SendTransaction {
    type Output;
}

/// Basic trait for simulating transactions in a validator
pub trait SimulateTransaction {
    type SimulationOutput: SimulationResult;
}

/// Trait for the output of a simulation
pub trait SimulationResult {
    fn get_compute_units_consumed(&self) -> ProgramClientResult<u64>;
}

/// Extend basic `SendTransaction` trait with function `send` where client is
/// `&mut BankClient`. Required for `ProgramBanksClient`.
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
