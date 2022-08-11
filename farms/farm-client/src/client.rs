//! Solana Farm Client
//!
//! Solana Farm Client provides an easy way to interact with pools, farms, vaults, and funds,
//! query on-chain objects metadata, and perform common operations with accounts.
//!
//! Client's methods accept human readable names (tokens, polls, etc.) and UI (decimal)
//! amounts, so you can simply call client.swap(&keypair, Protocol::Orca, "SOL", "USDC", 0.1, 0.0)
//! to swap 0.1 SOL for RAY in a Raydium pool. All metadata required to lookup account
//! addresses, decimals, etc. is stored on-chain.
//!
//! Under the hood it leverages the official Solana RPC Client which can be accessed with
//! client.rpc_client, for example: client.rpc_client.get_latest_blockhash().
//!
//! Naming convention for Pools and Farms is [PROTOCOL].[TOKEN_A]-[TOKEN_B]-[VERSION]
//! Naming convention for Vaults is [PROTOCOL].[STRATEGY].[TOKEN_A]-[TOKEN_B]-[VERSION]
//! There are single token pools where TOKEN_B is not present.
//! A list of supported protocols can be obtained with get_protocols().
//! If VERSION is omitted then Pool, Farm, or Vault with the latest version will be used.
//!
//! A few examples:
//! #  use {
//! #      solana_farm_client::client::FarmClient,
//! #      solana_sdk::{pubkey::Pubkey, signer::Signer},
//! #  };
//! #
//! #  let client = FarmClient::new("https://api.mainnet-beta.solana.com");
//! #  let keypair = FarmClient::read_keypair_from_file(
//! #      &(std::env::var("HOME").unwrap().to_string() + "/.config/solana/id.json"),
//! #  )
//! #  .unwrap();
//! #
//! #  // get SOL account balance
//! #  client.get_account_balance(&keypair.pubkey()).unwrap();
//! #
//! #  // get SPL token account balance
//! #  client
//! #      .get_token_account_balance(&keypair.pubkey(), "SRM")
//! #      .unwrap();
//! #
//! #  // get token metadata
//! #  client.get_token("SRM").unwrap();
//! #
//! #  // find Raydium pools with RAY and SRM tokens
//! #  client.find_pools(Protocol::Raydium, "RAY", "SRM").unwrap();
//! #
//! #  // find Saber pools with USDC and USDT tokens
//! #  client.find_pools(Protocol::Saber, "USDC", "USDT").unwrap();
//! #
//! #  // get pool metadata
//! #  client.get_pool("RDM.RAY-SRM").unwrap();
//! #
//! #  // get farm metadata
//! #  client.get_farm("RDM.RAY-SRM").unwrap();
//! #
//! #  // find all vaults with RAY and SRM tokens
//! #  client.find_vaults("RAY", "SRM").unwrap();
//! #
//! #  // get vault metadata
//! #  client.get_vault("RDM.STC.RAY-SRM").unwrap();
//! #
//! #  // get fund metadata
//! #  client.get_fund("TestFund").unwrap();
//! #
//! #  // get the list of all pools
//! #  client.get_pools().unwrap();
//! #
//! #  // find farms for specific LP token
//! #  client.find_farms_with_lp("LP.RDM.RAY-SRM-V4").unwrap();
//! #
//! #  // get Raydium pool price
//! #  client.get_pool_price("RDM.RAY-SRM").unwrap();
//! #  // or specify version for specific pool
//! #  client.get_pool_price("RDM.RAY-SRM-V4").unwrap();
//! #
//! #  // get oracle price
//! #  client.get_oracle_price("SOL", 0, 0.0).unwrap();
//! #
//! #  // list official program IDs
//! #  client.get_program_ids().unwrap();
//! #
//! #  // swap in the Raydium pool
//! #  client.swap(&keypair, Protocol::Raydium, "SOL", "RAY", 0.01, 0.0).unwrap();
//! #
//! #  // swap in the Saber pool
//! #  client.swap(&keypair, Protocol::Saber, "USDC", "USDT", 0.01, 0.0).unwrap();
//! #
//! #  // deposit liquidity to the Raydium pool (zero second token amount means calculate it automatically)
//! #  client
//! #      .add_liquidity_pool(&keypair, "RDM.GRAPE-USDC", 0.1, 0.0)
//! #      .unwrap();
//! #
//! #  // withdraw your liquidity from the Raydium pool (zero amount means remove all tokens)
//! #  client
//! #      .remove_liquidity_pool(&keypair, "RDM.GRAPE-USDC", 0.0)
//! #      .unwrap();
//! #
//! #  // stake LP tokens to the Raydium farm (zero amount means stake all)
//! #  client.stake(&keypair, "RDM.GRAPE-USDC", 0.0).unwrap();
//! #
//! #  // get staked balance
//! #  client.get_user_stake_balance(&keypair.pubkey(), "RDM.GRAPE-USDC").unwrap();
//! #
//! #  // harvest rewards
//! #  client.harvest(&keypair, "RDM.GRAPE-USDC").unwrap();
//! #
//! #  // unstake LP tokens from the farm (zero amount means unstake all)
//! #  client.unstake(&keypair, "RDM.GRAPE-USDC", 0.0).unwrap();
//! #
//! #  // deposit liquidity to the vault (zero second token amount means calculate it automatically)
//! #  client
//! #      .add_liquidity_vault(&keypair, "RDM.STC.RAY-SRM", 0.01, 0.0)
//! #      .unwrap();
//! #
//! #  // withdraw liquidity from the vault (zero amount means remove all tokens)
//! #  client
//! #      .remove_liquidity_vault(&keypair, "RDM.STC.RAY-SRM", 0.0)
//! #      .unwrap();
//! #
//! #  // request liquidity deposit to the fund
//! #  client
//! #      .request_deposit_fund(&keypair, "TestFund", "USDC", 0.01)
//! #      .unwrap();
//! #
//! #  // request liquidity withdrawal from the fund (zero amount means withdraw everything)
//! #  client
//! #      .request_withdrawal_fund(&keypair, "TestFund", "USDC", 0.0)
//! #      .unwrap();
//! #
//! #  // list all vaults that belong to particular fund
//! #  client.get_fund_vaults("TestFund").unwrap();
//! #
//! #  // transfer SOL to another wallet
//! #  client
//! #      .transfer(&keypair, &Pubkey::new_unique(), 0.001)
//! #      .unwrap();
//! #
//! #  // transfer SPL tokens to another wallet
//! #  client
//! #      .token_transfer(&keypair, "SRM", &Pubkey::new_unique(), 0.001)
//! #      .unwrap();
//! #
//! #  // create associated token account for the wallet
//! #  client.get_or_create_token_account(&keypair, "SRM").unwrap();
//! #
//! #  // list all active token accounts for the wallet
//! #  client.get_wallet_tokens(&keypair.pubkey()).unwrap();
//! #
//! #  // get vault stats
//! #  client.get_vault_info("RDM.STC.RAY-SRM").unwrap();
//! #
//! #  // get user stats for particular vault
//! #  client
//! #      .get_vault_user_info(&keypair.pubkey(), "RDM.STC.RAY-SRM")
//! #      .unwrap();
//! #
//! #  // create a new instruction for depositing liquidity to the vault, neither sign nor send it
//! #  client
//! #      .new_instruction_add_liquidity_vault(&keypair.pubkey(), "RDM.STC.RAY-SRM", 0.1, 0.0)
//! #      .unwrap();
//! #
//! #  // get fund stats and parameters
//! #  client.get_fund_info("TestFund").unwrap();
//! #
//! #  // get fund custody info
//! #  client.get_fund_custody("TestFund", "USDC", FundCustodyType::DepositWithdraw).unwrap();
//! #
//! #  // get information about fund assets
//! #  client.get_fund_assets(&fund_name, FundAssetType::Vault).unwrap();
//! #  client.get_fund_assets(&fund_name, FundAssetType::Custody).unwrap();

use {
    crate::{cache::Cache, error::FarmClientError},
    arrayref::array_ref,
    pyth_client::{PriceStatus, PriceType},
    solana_account_decoder::{
        parse_bpf_loader::{parse_bpf_upgradeable_loader, BpfUpgradeableLoaderAccountType},
        parse_token::{parse_token, TokenAccountType, UiAccountState, UiMint, UiTokenAccount},
        UiAccountEncoding,
    },
    solana_client::{
        client_error::ClientErrorKind,
        rpc_client::RpcClient,
        rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
        rpc_custom_error, rpc_filter,
        rpc_request::{RpcError, TokenAccountsFilter},
    },
    solana_farm_sdk::{
        farm::{Farm, FarmRoute},
        fund::{
            Fund, FundAssetType, FundAssets, FundAssetsTrackingConfig, FundCustody,
            FundCustodyType, FundCustodyWithBalance, FundInfo, FundSchedule, FundUserInfo,
            FundUserRequests, FundVault, FundVaultType, DISCRIMINATOR_FUND_CUSTODY,
            DISCRIMINATOR_FUND_USER_REQUESTS, DISCRIMINATOR_FUND_VAULT,
        },
        id::{
            main_router, main_router_admin, main_router_multisig, zero, DAO_CUSTODY_NAME,
            DAO_MINT_NAME, DAO_PROGRAM_NAME, DAO_TOKEN_NAME,
        },
        math,
        pool::{Pool, PoolRoute},
        program::{
            multisig::Multisig,
            protocol::{
                orca::OrcaUserStakeInfo,
                raydium::{RaydiumUserStakeInfo, RaydiumUserStakeInfoV4},
                saber::Miner,
            },
        },
        refdb,
        refdb::{Header, RefDB},
        string::str_to_as64,
        token::{OracleType, Token, TokenSelector, TokenType},
        traits::Packed,
        vault::{Vault, VaultInfo, VaultStrategy, VaultUserInfo},
        ProgramIDType, Protocol, ProtocolInfo,
    },
    solana_sdk::{
        account::Account,
        borsh::try_from_slice_unchecked,
        bpf_loader_upgradeable,
        clock::UnixTimestamp,
        commitment_config::{CommitmentConfig, CommitmentLevel},
        instruction::Instruction,
        message::Message,
        program_error::ProgramError,
        program_pack::Pack,
        pubkey::Pubkey,
        signature::{read_keypair, read_keypair_file, Keypair, Signature, Signer},
        signers::Signers,
        transaction::Transaction,
    },
    spl_associated_token_account::{create_associated_token_account, get_associated_token_address},
    spl_governance::state::{
        enums::GovernanceAccountType,
        governance::{
            get_account_governance_address, get_mint_governance_address,
            get_program_governance_address, Governance, GovernanceConfig,
        },
        proposal::{get_proposal_address, ProposalV2},
        proposal_instruction::{
            get_proposal_instruction_address, InstructionData, ProposalInstructionV2,
        },
        realm::get_realm_address,
    },
    spl_token::state::Mint,
    stable_swap_client::state::SwapInfo,
    stable_swap_math::price::SaberSwap,
    std::{
        cell::RefCell, collections::HashMap, str::FromStr, thread, time, time::Duration, vec::Vec,
    },
};

pub type VaultMap = HashMap<String, Vault>;
pub type FundMap = HashMap<String, Fund>;
pub type PoolMap = HashMap<String, Pool>;
pub type FarmMap = HashMap<String, Farm>;
pub type TokenMap = HashMap<String, Token>;
pub type PubkeyMap = HashMap<String, Pubkey>;
pub type StakeAccMap = HashMap<String, Pubkey>;
pub type U64Map = HashMap<String, u64>;

/// Farm Client
pub struct FarmClient {
    pub rpc_client: RpcClient,
    tokens: RefCell<Cache<Token>>,
    pools: RefCell<Cache<Pool>>,
    farms: RefCell<Cache<Farm>>,
    vaults: RefCell<Cache<Vault>>,
    funds: RefCell<Cache<Fund>>,
    token_refs: RefCell<Cache<Pubkey>>,
    pool_refs: RefCell<Cache<Pubkey>>,
    farm_refs: RefCell<Cache<Pubkey>>,
    vault_refs: RefCell<Cache<Pubkey>>,
    fund_refs: RefCell<Cache<Pubkey>>,
    official_ids: RefCell<Cache<Pubkey>>,
    stake_accounts: RefCell<Vec<HashMap<String, StakeAccMap>>>,
    latest_pools: RefCell<HashMap<String, String>>,
    latest_farms: RefCell<HashMap<String, String>>,
    latest_vaults: RefCell<HashMap<String, String>>,
}

impl Default for FarmClient {
    fn default() -> Self {
        Self {
            rpc_client: RpcClient::new("".to_string()),
            tokens: RefCell::new(Cache::<Token>::default()),
            pools: RefCell::new(Cache::<Pool>::default()),
            farms: RefCell::new(Cache::<Farm>::default()),
            vaults: RefCell::new(Cache::<Vault>::default()),
            funds: RefCell::new(Cache::<Fund>::default()),
            token_refs: RefCell::new(Cache::<Pubkey>::default()),
            pool_refs: RefCell::new(Cache::<Pubkey>::default()),
            farm_refs: RefCell::new(Cache::<Pubkey>::default()),
            vault_refs: RefCell::new(Cache::<Pubkey>::default()),
            fund_refs: RefCell::new(Cache::<Pubkey>::default()),
            official_ids: RefCell::new(Cache::<Pubkey>::default()),
            stake_accounts: RefCell::new(vec![HashMap::<String, StakeAccMap>::new(); 3]),
            latest_pools: RefCell::new(HashMap::<String, String>::new()),
            latest_farms: RefCell::new(HashMap::<String, String>::new()),
            latest_vaults: RefCell::new(HashMap::<String, String>::new()),
        }
    }
}

impl FarmClient {
    /// Creates a new FarmClient object
    /// RPC URLs:
    /// Devnet: https://api.devnet.solana.com
    /// Testnet: https://api.testnet.solana.com
    /// Mainnet-beta: https://api.mainnet-beta.solana.com
    /// local node: http://localhost:8899
    pub fn new(url: &str) -> Self {
        Self {
            rpc_client: RpcClient::new(url.to_string()),
            ..FarmClient::default()
        }
    }

    /// Creates a new FarmClient object with commitment config
    pub fn new_with_commitment(url: &str, commitment_config: CommitmentConfig) -> Self {
        Self {
            rpc_client: RpcClient::new_with_commitment(url.to_string(), commitment_config),
            ..FarmClient::default()
        }
    }

    /// Creates a new FarmClient object with timeout and config
    pub fn new_with_timeout_and_commitment(
        url: &str,
        timeout: Duration,
        commitment_config: CommitmentConfig,
    ) -> Self {
        Self {
            rpc_client: RpcClient::new_with_timeout_and_commitment(
                url.to_string(),
                timeout,
                commitment_config,
            ),
            ..FarmClient::default()
        }
    }

    pub fn new_mock(url: &str) -> Self {
        Self {
            rpc_client: RpcClient::new_mock(url.to_string()),
            ..FarmClient::default()
        }
    }

    /// Returns the Fund struct for the given name
    pub fn get_fund(&self, name: &str) -> Result<Fund, FarmClientError> {
        // reload Fund refs if stale
        self.reload_fund_refs_if_stale()?;
        // if Fund is in cache return it
        if let Some(fund) = self.funds.borrow().data.get(name) {
            return Ok(*fund);
        }
        // load Fund data from blockchain
        if let Some(key) = self.fund_refs.borrow().data.get(name) {
            let fund = self.load_fund_by_ref(key)?;
            self.funds.borrow_mut().data.insert(name.to_string(), fund);
            return Ok(fund);
        }
        Err(FarmClientError::RecordNotFound(format!("Fund {}", name)))
    }

    /// Returns all Funds available
    pub fn get_funds(&self) -> Result<FundMap, FarmClientError> {
        self.reload_fund_refs_if_stale()?;
        self.reload_funds_if_empty()?;
        Ok(self.funds.borrow().data.clone())
    }

    /// Returns the Fund metadata address for the given name
    pub fn get_fund_ref(&self, name: &str) -> Result<Pubkey, FarmClientError> {
        // reload Fund refs if stale
        self.reload_fund_refs_if_stale()?;
        // return the address from cache
        if let Some(key) = self.fund_refs.borrow().data.get(name) {
            return Ok(*key);
        }
        Err(FarmClientError::RecordNotFound(format!("Fund {}", name)))
    }

    /// Returns Fund refs: a map of Fund name to account address with metadata
    pub fn get_fund_refs(&self) -> Result<PubkeyMap, FarmClientError> {
        self.reload_fund_refs_if_stale()?;
        Ok(self
            .get_refdb_pubkey_map(&refdb::StorageType::Fund.to_string())?
            .1)
    }

    /// Returns the Fund metadata at the specified address
    pub fn get_fund_by_ref(&self, fund_ref: &Pubkey) -> Result<Fund, FarmClientError> {
        let name = &self.get_fund_name(fund_ref)?;
        self.get_fund(name)
    }

    /// Returns the Fund name for the given metadata address
    pub fn get_fund_name(&self, fund_ref: &Pubkey) -> Result<String, FarmClientError> {
        // reload Fund refs if stale
        self.reload_fund_refs_if_stale()?;
        // return the name from cache
        for (name, key) in self.fund_refs.borrow().data.iter() {
            if key == fund_ref {
                return Ok(name.to_string());
            }
        }
        Err(FarmClientError::RecordNotFound(format!(
            "Fund reference {}",
            fund_ref
        )))
    }

    /// Returns all Funds that have Vaults with the name matching the pattern sorted by version
    pub fn find_funds(&self, vault_name_pattern: &str) -> Result<Vec<Fund>, FarmClientError> {
        let mut res = vec![];
        let funds = self.get_funds()?;
        for (fund_name, fund) in &funds {
            let vaults = self.get_fund_vaults(fund_name)?;
            for vault in &vaults {
                if let Ok(vault) = self.get_vault_by_ref(&vault.vault_ref) {
                    if vault.name.contains(&vault_name_pattern) {
                        res.push(*fund);
                    }
                }
            }
        }
        if res.is_empty() {
            Err(FarmClientError::RecordNotFound(format!(
                "Funds with Vault name pattern {}",
                vault_name_pattern
            )))
        } else {
            res.sort_by(|a, b| b.version.cmp(&a.version));
            Ok(res)
        }
    }

    /// Returns the Vault struct for the given name
    pub fn get_vault(&self, name: &str) -> Result<Vault, FarmClientError> {
        // reload Vault refs if stale
        self.reload_vault_refs_if_stale()?;
        let vault_name = if let Some(val) = self.latest_vaults.borrow().get(name) {
            val.clone()
        } else {
            name.to_string()
        };
        // if Vault is in cache return it
        if let Some(vault) = self.vaults.borrow().data.get(&vault_name) {
            return Ok(*vault);
        }
        // load Vault data from blockchain
        if let Some(key) = self.vault_refs.borrow().data.get(&vault_name) {
            let vault = self.load_vault_by_ref(key)?;
            self.vaults.borrow_mut().data.insert(vault_name, vault);
            return Ok(vault);
        }
        Err(FarmClientError::RecordNotFound(format!("Vault {}", name)))
    }

    /// Returns all Vaults available
    pub fn get_vaults(&self) -> Result<VaultMap, FarmClientError> {
        self.reload_vault_refs_if_stale()?;
        self.reload_vaults_if_empty()?;
        Ok(self.vaults.borrow().data.clone())
    }

    /// Returns the Vault metadata address for the given name
    pub fn get_vault_ref(&self, name: &str) -> Result<Pubkey, FarmClientError> {
        // reload Vault refs if stale
        self.reload_vault_refs_if_stale()?;
        // return the address from cache
        let vault_name = if let Some(val) = self.latest_vaults.borrow().get(name) {
            val.clone()
        } else {
            name.to_string()
        };
        if let Some(key) = self.vault_refs.borrow().data.get(&vault_name) {
            return Ok(*key);
        }
        Err(FarmClientError::RecordNotFound(format!("Vault {}", name)))
    }

    /// Returns Vault refs: a map of Vault name to account address with metadata
    pub fn get_vault_refs(&self) -> Result<PubkeyMap, FarmClientError> {
        self.reload_vault_refs_if_stale()?;
        Ok(self.vault_refs.borrow().data.clone())
    }

    /// Returns the Vault metadata at the specified address
    pub fn get_vault_by_ref(&self, vault_ref: &Pubkey) -> Result<Vault, FarmClientError> {
        let name = &self.get_vault_name(vault_ref)?;
        self.get_vault(name)
    }

    /// Returns the Vault name for the given metadata address
    pub fn get_vault_name(&self, vault_ref: &Pubkey) -> Result<String, FarmClientError> {
        // reload Vault refs if stale
        self.reload_vault_refs_if_stale()?;
        // return the name from cache
        for (name, key) in self.vault_refs.borrow().data.iter() {
            if key == vault_ref {
                return Ok(name.to_string());
            }
        }
        Err(FarmClientError::RecordNotFound(format!(
            "Vault reference {}",
            vault_ref
        )))
    }

    /// Returns all Vaults with tokens A and B sorted by version
    pub fn find_vaults(&self, token_a: &str, token_b: &str) -> Result<Vec<Vault>, FarmClientError> {
        self.reload_vault_refs_if_stale()?;
        let pattern1 = format!(".{}-{}-", token_a, token_b);
        let pattern2 = format!(".{}-{}-", token_b, token_a);
        let mut res = vec![];
        for (name, _) in self.vault_refs.borrow().data.iter() {
            if name.contains(&pattern1) || name.contains(&pattern2) {
                res.push(self.get_vault(name)?);
            }
        }
        if res.is_empty() {
            Err(FarmClientError::RecordNotFound(format!(
                "Vault with tokens {} and {}",
                token_a, token_b
            )))
        } else {
            res.sort_by(|a, b| b.version.cmp(&a.version));
            Ok(res)
        }
    }

    /// Returns all Vaults sorted by version for the given VT token
    pub fn find_vaults_with_vt(&self, vt_token_name: &str) -> Result<Vec<Vault>, FarmClientError> {
        let (protocol, token_a, token_b) = FarmClient::extract_token_names(vt_token_name)?;
        let vaults = self.find_vaults(&token_a, &token_b)?;
        let mut res = vec![];
        for vault in &vaults {
            if self.get_token_by_ref(&vault.vault_token_ref)?.name.as_str() == vt_token_name {
                res.push(*vault);
            }
        }

        if res.is_empty() {
            Err(FarmClientError::RecordNotFound(format!(
                "{} Vault with VT token {}",
                protocol, vt_token_name
            )))
        } else {
            res.sort_by(|a, b| b.version.cmp(&a.version));
            Ok(res)
        }
    }

    /// Returns the Pool struct for the given name
    pub fn get_pool(&self, name: &str) -> Result<Pool, FarmClientError> {
        // reload Pool refs if stale
        self.reload_pool_refs_if_stale()?;
        let pool_name = if let Some(val) = self.latest_pools.borrow().get(name) {
            val.clone()
        } else {
            name.to_string()
        };
        // if Pool is in cache return it
        if let Some(pool) = self.pools.borrow().data.get(&pool_name) {
            return Ok(*pool);
        }
        // load Pool data from blockchain
        if let Some(key) = self.pool_refs.borrow().data.get(&pool_name) {
            let pool = self.load_pool_by_ref(key)?;
            self.pools.borrow_mut().data.insert(pool_name, pool);
            return Ok(pool);
        }
        Err(FarmClientError::RecordNotFound(format!("Pool {}", name)))
    }

    /// Returns all Pools available
    pub fn get_pools(&self) -> Result<PoolMap, FarmClientError> {
        self.reload_pool_refs_if_stale()?;
        self.reload_pools_if_empty()?;
        Ok(self.pools.borrow().data.clone())
    }

    /// Returns the Pool metadata address for the given name
    pub fn get_pool_ref(&self, name: &str) -> Result<Pubkey, FarmClientError> {
        // reload Pool refs if stale
        self.reload_pool_refs_if_stale()?;
        // return the address from cache
        let pool_name = if let Some(val) = self.latest_pools.borrow().get(name) {
            val.clone()
        } else {
            name.to_string()
        };
        if let Some(key) = self.pool_refs.borrow().data.get(&pool_name) {
            return Ok(*key);
        }
        Err(FarmClientError::RecordNotFound(format!("Pool {}", name)))
    }

    /// Returns Pool refs: a map of Pool name to account address with metadata
    pub fn get_pool_refs(&self) -> Result<PubkeyMap, FarmClientError> {
        self.reload_pool_refs_if_stale()?;
        Ok(self.pool_refs.borrow().data.clone())
    }

    /// Returns the Pool metadata at the specified address
    pub fn get_pool_by_ref(&self, pool_ref: &Pubkey) -> Result<Pool, FarmClientError> {
        let name = &self.get_pool_name(pool_ref)?;
        self.get_pool(name)
    }

    /// Returns the Pool name for the given metadata address
    pub fn get_pool_name(&self, pool_ref: &Pubkey) -> Result<String, FarmClientError> {
        // reload Pool refs if stale
        self.reload_pool_refs_if_stale()?;
        // return the name from cache
        for (name, key) in self.pool_refs.borrow().data.iter() {
            if key == pool_ref {
                return Ok(name.to_string());
            }
        }
        Err(FarmClientError::RecordNotFound(format!(
            "Pool reference {}",
            pool_ref
        )))
    }

    /// Returns all Pools with tokens A and B sorted by version for the given protocol
    pub fn find_pools(
        &self,
        protocol: Protocol,
        token_a: &str,
        token_b: &str,
    ) -> Result<Vec<Pool>, FarmClientError> {
        self.reload_pool_refs_if_stale()?;
        let pattern1 = format!("{}.{}-{}-", protocol.id(), token_a, token_b);
        let pattern2 = format!("{}.{}-{}-", protocol.id(), token_b, token_a);
        let mut res = vec![];
        for (name, _) in self.pool_refs.borrow().data.iter() {
            if name.starts_with(&pattern1) || name.starts_with(&pattern2) {
                res.push(self.get_pool(name)?);
            }
        }
        if res.is_empty() {
            Err(FarmClientError::RecordNotFound(format!(
                "{} Pool with tokens {} and {}",
                protocol, token_a, token_b
            )))
        } else {
            res.sort_by(|a, b| b.version.cmp(&a.version));
            Ok(res)
        }
    }

    /// Returns all Pools sorted by version for the given LP token
    pub fn find_pools_with_lp(&self, lp_token_name: &str) -> Result<Vec<Pool>, FarmClientError> {
        let lp_token_ref = self.get_token_ref(lp_token_name)?;
        let pools = self.get_pools()?;
        let mut res = vec![];
        for pool in pools.values() {
            if let Some(pool_lp_token_ref) = pool.lp_token_ref {
                if lp_token_ref == pool_lp_token_ref {
                    res.push(*pool);
                }
            }
        }

        if res.is_empty() {
            Err(FarmClientError::RecordNotFound(format!(
                "Pool with LP token {}",
                lp_token_name
            )))
        } else {
            res.sort_by(|a, b| b.version.cmp(&a.version));
            Ok(res)
        }
    }

    /// Returns pair's price based on the ratio of tokens in the pool
    pub fn get_pool_price(&self, pool_name: &str) -> Result<f64, FarmClientError> {
        let pool = self.get_pool(pool_name)?;
        if pool.token_a_ref.is_none() || pool.token_b_ref.is_none() {
            return Ok(0.0);
        }
        let token_a = self.get_token_by_ref(&pool.token_a_ref.unwrap())?;
        let token_b = self.get_token_by_ref(&pool.token_b_ref.unwrap())?;
        let token_a_balance = self
            .rpc_client
            .get_token_account_balance(
                &pool
                    .token_a_account
                    .ok_or(ProgramError::UninitializedAccount)?,
            )?
            .amount
            .parse::<u64>()
            .unwrap();
        let token_b_balance = self
            .rpc_client
            .get_token_account_balance(
                &pool
                    .token_b_account
                    .ok_or(ProgramError::UninitializedAccount)?,
            )?
            .amount
            .parse::<u64>()
            .unwrap();

        match pool.route {
            PoolRoute::Raydium {
                amm_id,
                amm_open_orders,
                ..
            } => self.get_pool_price_raydium(
                token_a_balance,
                token_b_balance,
                token_a.decimals,
                token_b.decimals,
                &amm_id,
                &amm_open_orders,
            ),
            PoolRoute::Saber { swap_account, .. } => {
                let lp_token = self.get_token_by_ref(&pool.lp_token_ref.unwrap())?;
                self.get_pool_price_saber(
                    &swap_account,
                    token_a_balance,
                    token_b_balance,
                    &lp_token,
                )
            }
            PoolRoute::Orca { .. } => self.get_pool_price_orca(
                token_a_balance,
                token_b_balance,
                token_a.decimals,
                token_b.decimals,
            ),
        }
    }

    /// Returns the Farm struct for the given name
    pub fn get_farm(&self, name: &str) -> Result<Farm, FarmClientError> {
        // reload Farm refs if stale
        self.reload_farm_refs_if_stale()?;
        let farm_name = if let Some(val) = self.latest_farms.borrow().get(name) {
            val.clone()
        } else {
            name.to_string()
        };
        // if Farm is in cache return it
        if let Some(farm) = self.farms.borrow().data.get(&farm_name) {
            return Ok(*farm);
        }
        // load Farm data from blockchain
        if let Some(key) = self.farm_refs.borrow().data.get(&farm_name) {
            let farm = self.load_farm_by_ref(key)?;
            self.farms.borrow_mut().data.insert(farm_name, farm);
            return Ok(farm);
        }
        Err(FarmClientError::RecordNotFound(format!("Farm {}", name)))
    }

    /// Returns all Farms available
    pub fn get_farms(&self) -> Result<FarmMap, FarmClientError> {
        self.reload_farm_refs_if_stale()?;
        self.reload_farms_if_empty()?;
        Ok(self.farms.borrow().data.clone())
    }

    /// Returns the Farm metadata address for the given name
    pub fn get_farm_ref(&self, name: &str) -> Result<Pubkey, FarmClientError> {
        // reload Farm refs if stale
        self.reload_farm_refs_if_stale()?;
        // return the address from cache
        let farm_name = if let Some(val) = self.latest_farms.borrow().get(name) {
            val.clone()
        } else {
            name.to_string()
        };
        if let Some(key) = self.farm_refs.borrow().data.get(&farm_name) {
            return Ok(*key);
        }
        Err(FarmClientError::RecordNotFound(format!("Farm {}", name)))
    }

    /// Returns Farm refs: a map of Farm name to account address with metadata
    pub fn get_farm_refs(&self) -> Result<PubkeyMap, FarmClientError> {
        self.reload_farm_refs_if_stale()?;
        Ok(self.farm_refs.borrow().data.clone())
    }

    /// Returns the Farm metadata at the specified address
    pub fn get_farm_by_ref(&self, farm_ref: &Pubkey) -> Result<Farm, FarmClientError> {
        let name = &self.get_farm_name(farm_ref)?;
        self.get_farm(name)
    }

    /// Returns the Farm name for the given metadata address
    pub fn get_farm_name(&self, farm_ref: &Pubkey) -> Result<String, FarmClientError> {
        // reload Farm refs if stale
        self.reload_farm_refs_if_stale()?;
        // return the name from cache
        for (name, key) in self.farm_refs.borrow().data.iter() {
            if key == farm_ref {
                return Ok(name.to_string());
            }
        }
        Err(FarmClientError::RecordNotFound(format!(
            "Farm reference {}",
            farm_ref
        )))
    }

    /// Returns all Farms for the given LP token
    pub fn find_farms_with_lp(&self, lp_token_name: &str) -> Result<Vec<Farm>, FarmClientError> {
        let lp_token_ref = self.get_token_ref(lp_token_name)?;
        let farms = self.get_farms()?;
        let mut res = vec![];
        for farm in farms.values() {
            if let Some(farm_lp_token_ref) = farm.lp_token_ref {
                if lp_token_ref == farm_lp_token_ref {
                    res.push(*farm);
                }
            }
        }

        if res.is_empty() {
            Err(FarmClientError::RecordNotFound(format!(
                "Farm with LP token {}",
                lp_token_name
            )))
        } else {
            res.sort_by(|a, b| b.version.cmp(&a.version));
            Ok(res)
        }
    }

    /// Returns the Token struct for the given name
    pub fn get_token(&self, name: &str) -> Result<Token, FarmClientError> {
        // reload Token refs if stale
        self.reload_token_refs_if_stale()?;
        // if Token is in cache return it
        if let Some(token) = self.tokens.borrow().data.get(name) {
            return Ok(*token);
        }
        // load Token data from blockchain
        if let Some(key) = self.token_refs.borrow().data.get(name) {
            let token = self.load_token_by_ref(key)?;
            self.tokens
                .borrow_mut()
                .data
                .insert(name.to_string(), token);
            return Ok(token);
        }
        Err(FarmClientError::RecordNotFound(format!("Token {}", name)))
    }

    /// Returns all Tokens available
    pub fn get_tokens(&self) -> Result<TokenMap, FarmClientError> {
        self.reload_token_refs_if_stale()?;
        self.reload_tokens_if_empty()?;
        Ok(self.tokens.borrow().data.clone())
    }

    /// Returns the Token metadata address for the given name
    pub fn get_token_ref(&self, name: &str) -> Result<Pubkey, FarmClientError> {
        // reload Token refs if stale
        self.reload_token_refs_if_stale()?;
        // return the address from cache
        if let Some(key) = self.token_refs.borrow().data.get(name) {
            return Ok(*key);
        }
        Err(FarmClientError::RecordNotFound(format!("Token {}", name)))
    }

    /// Returns Token refs: a map of Token name to account address with metadata
    pub fn get_token_refs(&self) -> Result<PubkeyMap, FarmClientError> {
        self.reload_token_refs_if_stale()?;
        Ok(self
            .get_refdb_pubkey_map(&refdb::StorageType::Token.to_string())?
            .1)
    }

    /// Returns the Token metadata at the specified address
    pub fn get_token_by_ref(&self, token_ref: &Pubkey) -> Result<Token, FarmClientError> {
        let name = &self.get_token_name(token_ref)?;
        self.get_token(name)
    }

    /// Returns the Token name for the given metadata address
    pub fn get_token_name(&self, token_ref: &Pubkey) -> Result<String, FarmClientError> {
        // reload Token refs if stale
        self.reload_token_refs_if_stale()?;
        // return the name from cache
        for (name, key) in self.token_refs.borrow().data.iter() {
            if key == token_ref {
                return Ok(name.to_string());
            }
        }
        Err(FarmClientError::RecordNotFound(format!(
            "Token reference {}",
            token_ref
        )))
    }

    /// Returns the Token metadata for the specified mint
    /// This function loads all tokens to the cache, slow on the first call.
    pub fn get_token_with_mint(&self, token_mint: &Pubkey) -> Result<Token, FarmClientError> {
        let tokens = self.get_tokens()?;
        for token in tokens.values() {
            if token_mint == &token.mint {
                return Ok(*token);
            }
        }
        Err(FarmClientError::RecordNotFound(format!(
            "Token with mint {}",
            token_mint
        )))
    }

    /// Returns the Token metadata for the specified token account
    /// This function loads all tokens to the cache, slow on the first call.
    pub fn get_token_with_account(&self, token_account: &Pubkey) -> Result<Token, FarmClientError> {
        let data = self.rpc_client.get_account_data(token_account)?;
        let res = parse_token(data.as_slice(), Some(0))?;
        if let TokenAccountType::Account(ui_account) = res {
            self.get_token_with_mint(&Pubkey::from_str(&ui_account.mint).map_err(|_| {
                FarmClientError::ValueError(format!(
                    "Failed to parse mint in token account {}",
                    token_account
                ))
            })?)
        } else {
            Err(FarmClientError::ValueError(format!(
                "No account data found in token account {}",
                token_account
            )))
        }
    }

    /// Returns token supply as UI amount
    pub fn get_token_supply(&self, name: &str) -> Result<f64, FarmClientError> {
        self.rpc_client
            .get_token_supply(&self.get_token(name)?.mint)?
            .ui_amount
            .ok_or_else(|| FarmClientError::ValueError("Invalid UI token amount".to_string()))
    }

    /// Returns the official Program ID for the given name
    pub fn get_program_id(&self, name: &str) -> Result<Pubkey, FarmClientError> {
        // reload program ids if stale
        self.reload_program_ids_if_stale()?;
        // if program id is in cache return it
        if let Some(pubkey) = self.official_ids.borrow().data.get(name) {
            return Ok(*pubkey);
        }
        Err(FarmClientError::RecordNotFound(format!("Program {}", name)))
    }

    /// Returns all official Program IDs available
    pub fn get_program_ids(&self) -> Result<PubkeyMap, FarmClientError> {
        self.reload_program_ids_if_stale()?;
        Ok(self
            .get_refdb_pubkey_map(&refdb::StorageType::Program.to_string())?
            .1)
    }

    /// Returns the official program name for the given Program ID
    pub fn get_program_name(&self, prog_id: &Pubkey) -> Result<String, FarmClientError> {
        // reload program ids if stale
        self.reload_program_ids_if_stale()?;
        for (name, key) in self.official_ids.borrow().data.iter() {
            if key == prog_id {
                return Ok(name.to_string());
            }
        }
        Err(FarmClientError::RecordNotFound(format!(
            "Program ID {}",
            prog_id
        )))
    }

    /// Checks if the given address is the official Program ID
    pub fn is_official_id(&self, prog_id: &Pubkey) -> Result<bool, FarmClientError> {
        Ok(*prog_id == main_router::id() || self.get_program_name(prog_id).is_ok())
    }

    /// Returns program upgrade authority
    pub fn get_program_upgrade_authority(
        &self,
        prog_id: &Pubkey,
    ) -> Result<Pubkey, FarmClientError> {
        let program_account_data = self.rpc_client.get_account_data(prog_id)?;
        let program_account = parse_bpf_upgradeable_loader(&program_account_data)?;

        match program_account {
            BpfUpgradeableLoaderAccountType::Program(ui_program) => {
                let program_data_account_key =
                    FarmClient::pubkey_from_str(&ui_program.program_data)?;
                let program_data_account_data = self
                    .rpc_client
                    .get_account_data(&program_data_account_key)?;
                let program_data_account =
                    parse_bpf_upgradeable_loader(&program_data_account_data)?;

                match program_data_account {
                    BpfUpgradeableLoaderAccountType::ProgramData(ui_program_data) => {
                        if let Some(authority) = ui_program_data.authority {
                            Ok(FarmClient::pubkey_from_str(&authority)?)
                        } else {
                            Ok(zero::id())
                        }
                    }
                    _ => {
                        return Err(FarmClientError::ValueError(format!(
                            "Invalid program data account {}",
                            program_data_account_key
                        )))
                    }
                }
            }
            _ => {
                return Err(FarmClientError::ValueError(format!(
                    "Invalid program account {}",
                    prog_id
                )))
            }
        }
    }

    /// Returns multisig account address for the program
    pub fn get_program_multisig_account(
        &self,
        prog_id: &Pubkey,
    ) -> Result<Pubkey, FarmClientError> {
        Ok(Pubkey::find_program_address(&[b"multisig", prog_id.as_ref()], &main_router::id()).0)
    }

    /// Returns data buffer account address for the program
    pub fn get_program_buffer_account(&self, prog_id: &Pubkey) -> Result<Pubkey, FarmClientError> {
        Ok(Pubkey::find_program_address(&[prog_id.as_ref()], &bpf_loader_upgradeable::id()).0)
    }

    /// Returns program upgrade signers
    pub fn get_program_admins(&self, prog_id: &Pubkey) -> Result<Multisig, FarmClientError> {
        let upgrade_authority = self.get_program_upgrade_authority(prog_id)?;
        let multisig = self.get_program_multisig_account(prog_id)?;

        if upgrade_authority == multisig {
            if let Ok(data) = self.rpc_client.get_account_data(&multisig) {
                Multisig::unpack(&data).map_err(|e| e.into())
            } else {
                Err(FarmClientError::ValueError(format!(
                    "Invalid multisig account {}",
                    multisig
                )))
            }
        } else {
            Ok(Multisig {
                num_signers: 1,
                num_signed: 0,
                min_signatures: 1,
                instruction_accounts_len: 0,
                instruction_data_len: 0,
                instruction_hash: 0,
                signers: [
                    upgrade_authority,
                    zero::id(),
                    zero::id(),
                    zero::id(),
                    zero::id(),
                    zero::id(),
                ],
                signed: [false, false, false, false, false, false],
            })
        }
    }

    /// Sets new program upgrade signers
    pub fn set_program_admins(
        &self,
        admin_signer: &dyn Signer,
        prog_id: &Pubkey,
        admin_signers: &[Pubkey],
        min_signatures: u8,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_set_program_admins(
            &admin_signer.pubkey(),
            prog_id,
            admin_signers,
            min_signatures,
        )?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Sets single upgrade authority for the program removing multisig if present
    pub fn set_program_single_authority(
        &self,
        admin_signer: &dyn Signer,
        prog_id: &Pubkey,
        upgrade_authority: &Pubkey,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_set_program_single_authority(
            &admin_signer.pubkey(),
            prog_id,
            upgrade_authority,
        )?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Upgrades the program from the data buffer
    pub fn upgrade_program(
        &self,
        admin_signer: &dyn Signer,
        prog_id: &Pubkey,
        source_buffer_address: &Pubkey,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_upgrade_program(
            &admin_signer.pubkey(),
            prog_id,
            source_buffer_address,
        )?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Reads the Keypair from stdin
    pub fn read_keypair_from_stdin() -> Result<Keypair, FarmClientError> {
        let mut stdin = std::io::stdin();
        read_keypair(&mut stdin).map_err(|e| FarmClientError::IOError(e.to_string()))
    }

    /// Reads the Keypair from the file
    pub fn read_keypair_from_file(path: &str) -> Result<Keypair, FarmClientError> {
        read_keypair_file(path).map_err(|e| FarmClientError::IOError(e.to_string()))
    }

    /// Signs and sends instructions
    pub fn sign_and_send_instructions<S: Signers>(
        &self,
        signers: &S,
        instructions: &[Instruction],
    ) -> Result<Signature, FarmClientError> {
        if signers.pubkeys().is_empty() {
            return Err(FarmClientError::ValueError(
                "No signers provided for instruction".to_string(),
            ));
        }
        let mut transaction =
            Transaction::new_with_payer(instructions, Some(&signers.pubkeys()[0]));
        let mut recent_blockhash = self.rpc_client.get_latest_blockhash()?;
        let mut prev_signature = Signature::default();

        for i in 0..20 {
            if i > 0
                && !self
                    .rpc_client
                    .is_blockhash_valid(&recent_blockhash, self.rpc_client.commitment())?
            {
                recent_blockhash = self.rpc_client.get_latest_blockhash()?;
            }
            transaction.sign(signers, recent_blockhash);

            let result = self
                .rpc_client
                .send_and_confirm_transaction_with_spinner(&transaction);
            if let Ok(signature) = result {
                return Ok(signature);
            } else if i != 19 {
                if let Err(ref error) = result {
                    if let ClientErrorKind::RpcError(ref rpc_error) = error.kind {
                        if let RpcError::RpcResponseError { code, message, .. } = rpc_error {
                            if *code == rpc_custom_error::JSON_RPC_SERVER_ERROR_NODE_UNHEALTHY
                                || *code
                                    == rpc_custom_error::JSON_RPC_SERVER_ERROR_BLOCK_NOT_AVAILABLE
                                    || (*code == rpc_custom_error::JSON_RPC_SERVER_ERROR_SEND_TRANSACTION_PREFLIGHT_FAILURE
                                        && message.ends_with("Blockhash not found"))
                            {
                                println!("Node is unhealthy, re-trying in 5 secs...");
                                prev_signature = transaction.signatures[0];
                                thread::sleep(time::Duration::from_secs(5));
                                continue;
                            } else if *code == rpc_custom_error::JSON_RPC_SERVER_ERROR_SEND_TRANSACTION_PREFLIGHT_FAILURE
                            && message.ends_with("transaction has already been processed") {
                                return Ok(prev_signature);
                            }
                        } else if let RpcError::ForUser(msg) = rpc_error {
                            if msg.starts_with("unable to confirm transaction")
                                || msg.ends_with("Please retry.")
                            {
                                println!("Unable to confirm transaction, re-trying in 5 secs...");
                                prev_signature = transaction.signatures[0];
                                thread::sleep(time::Duration::from_secs(5));
                                continue;
                            }
                        }
                    } else if let ClientErrorKind::Reqwest(ref error) = error.kind {
                        if error.is_timeout() {
                            println!("Response timed out, re-trying in 5 secs...");
                            prev_signature = transaction.signatures[0];
                            thread::sleep(time::Duration::from_secs(5));
                            continue;
                        }
                    }
                }
                return Err(FarmClientError::RpcClientError(result.unwrap_err()));
            } else {
                return Err(FarmClientError::RpcClientError(result.unwrap_err()));
            }
        }
        unreachable!();
    }

    /// Returns serialized and encoded transaction size
    pub fn get_transaction_size(transaction: &Transaction) -> Result<usize, FarmClientError> {
        Ok(
            base64::encode(bincode::serialize(&transaction).map_err(|_| {
                FarmClientError::ParseError("Failed to serialize transaction".to_string())
            })?)
            .len(),
        )
    }

    /// Creates a new transaction with as many instructions as possible that fit transaction size limit
    pub fn create_transaction(
        instructions: &[Instruction],
        payer: &Pubkey,
    ) -> Result<(Transaction, usize), FarmClientError> {
        let transaction = Transaction::new_with_payer(instructions, Some(payer));
        if instructions.len() <= 1 || FarmClient::get_transaction_size(&transaction)? <= 1644 {
            return Ok((transaction, instructions.len()));
        }

        for i in 2..(instructions.len() + 1) {
            let transaction = Transaction::new_with_payer(&instructions[0..i], Some(payer));
            if FarmClient::get_transaction_size(&transaction)? > 1644 {
                return Ok((
                    Transaction::new_with_payer(&instructions[0..i - 1], Some(payer)),
                    i - 1,
                ));
            }
        }

        unreachable!();
    }

    /// Signs and sends instructions
    pub fn sign_and_send_instructions_in_batches<S: Signers>(
        &self,
        signers: &S,
        instructions: &[Instruction],
    ) -> Result<Vec<Signature>, FarmClientError> {
        if signers.pubkeys().is_empty() {
            return Err(FarmClientError::ValueError(
                "No signers provided for instruction".to_string(),
            ));
        }
        // process instructions in batches
        let mut processed = 0;
        let mut signatures = vec![];
        while processed < instructions.len() {
            let (_, batch_size) =
                FarmClient::create_transaction(&instructions[processed..], &signers.pubkeys()[0])?;
            let res = self.sign_and_send_instructions(
                signers,
                &instructions[processed..processed + batch_size],
            );
            if let Ok(signature) = res {
                signatures.push(signature);
            } else {
                return res.map(|_| signatures);
            }

            processed += batch_size;
        }
        Ok(signatures)
    }

    /// Wait for the transaction to become finalized
    pub fn confirm_async_transaction(
        &self,
        signature: &Signature,
        commitment: CommitmentLevel,
    ) -> Result<(), FarmClientError> {
        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;
        self.rpc_client
            .confirm_transaction_with_spinner(
                signature,
                &recent_blockhash,
                CommitmentConfig { commitment },
            )
            .map_err(Into::into)
    }

    /// Creates a new system account
    pub fn create_system_account(
        &self,
        signer: &dyn Signer,
        new_account_signer: &dyn Signer,
        lamports: u64,
        space: usize,
        owner: &Pubkey,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_create_system_account(
            &signer.pubkey(),
            &new_account_signer.pubkey(),
            lamports,
            space,
            owner,
        )?;
        self.sign_and_send_instructions(&[signer, new_account_signer], &[inst])
    }

    /// Closes the system account
    pub fn close_system_account(
        &self,
        signer: &dyn Signer,
        target_account_signer: &dyn Signer,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_close_system_account(
            &signer.pubkey(),
            &target_account_signer.pubkey(),
        )?;
        self.sign_and_send_instructions(&[signer, target_account_signer], &[inst])
    }

    /// Creates a new system account with seed
    pub fn create_system_account_with_seed(
        &self,
        signer: &dyn Signer,
        base_address: &Pubkey,
        seed: &str,
        lamports: u64,
        space: usize,
        owner: &Pubkey,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_create_system_account_with_seed(
            &signer.pubkey(),
            base_address,
            seed,
            lamports,
            space,
            owner,
        )?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Assigns system account to a program
    pub fn assign_system_account(
        &self,
        signer: &dyn Signer,
        program_address: &Pubkey,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_assign_system_account(&signer.pubkey(), program_address)?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Transfers native SOL from the wallet to the destination
    pub fn transfer(
        &self,
        signer: &dyn Signer,
        destination_wallet: &Pubkey,
        sol_ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        let inst =
            self.new_instruction_transfer(&signer.pubkey(), destination_wallet, sol_ui_amount)?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Transfers native SOL from the wallet to the associated Wrapped SOL account
    pub fn wrap_sol(
        &self,
        signer: &dyn Signer,
        ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.all_instructions_wrap_sol(&signer.pubkey(), ui_amount)?;
        self.sign_and_send_instructions(&[signer], &inst)
    }

    /// Transfers Wrapped SOL back to SOL by closing the associated Wrapped SOL account
    pub fn unwrap_sol(&self, signer: &dyn Signer) -> Result<Signature, FarmClientError> {
        let inst = self.all_instructions_unwrap_sol(&signer.pubkey())?;
        self.sign_and_send_instructions(&[signer], &inst)
    }

    /// Transfers tokens from the wallet to the destination
    pub fn token_transfer(
        &self,
        signer: &dyn Signer,
        token_name: &str,
        destination_wallet: &Pubkey,
        ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.all_instructions_token_transfer(
            &signer.pubkey(),
            token_name,
            destination_wallet,
            ui_amount,
        )?;
        self.sign_and_send_instructions(&[signer], &inst)
    }

    /// Updates token balance of the account, usefull after transfer SOL to WSOL account
    pub fn sync_token_balance(
        &self,
        signer: &dyn Signer,
        token_name: &str,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_sync_token_balance(&signer.pubkey(), token_name)?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Returns the associated token account for the given user's main account or creates one
    /// if it doesn't exist
    pub fn get_or_create_token_account(
        &self,
        signer: &dyn Signer,
        token_name: &str,
    ) -> Result<Pubkey, FarmClientError> {
        let wallet_address = signer.pubkey();
        let token_addr = self.get_associated_token_address(&wallet_address, token_name)?;
        if !self.has_active_token_account(&wallet_address, token_name) {
            let inst = self.new_instruction_create_token_account(&wallet_address, token_name)?;
            self.sign_and_send_instructions(&[signer], &[inst])?;
        } else {
            self.check_ata_owner(&signer.pubkey(), token_name)?;
        }
        Ok(token_addr)
    }

    /// Closes the associated token account for the given user's main account
    pub fn close_token_account(
        &self,
        signer: &dyn Signer,
        token_name: &str,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_close_token_account(&signer.pubkey(), token_name)?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Returns the associated token account address for the given token name
    pub fn get_associated_token_address(
        &self,
        wallet_address: &Pubkey,
        token_name: &str,
    ) -> Result<Pubkey, FarmClientError> {
        let token = self.get_token(token_name)?;
        Ok(get_associated_token_address(wallet_address, &token.mint))
    }

    /// Returns all tokens with active account in the wallet.
    /// This function loads all tokens to the cache, slow on the first call.
    pub fn get_wallet_tokens(
        &self,
        wallet_address: &Pubkey,
    ) -> Result<Vec<String>, FarmClientError> {
        let accounts = self.rpc_client.get_token_accounts_by_owner(
            wallet_address,
            TokenAccountsFilter::ProgramId(spl_token::id()),
        )?;
        let mut res = Vec::<String>::new();
        for acc in accounts.iter() {
            let token_address = FarmClient::pubkey_from_str(&acc.pubkey)?;

            let data = self.rpc_client.get_account_data(&token_address)?;
            let token_info = parse_token(data.as_slice(), Some(0))?;
            if let TokenAccountType::Account(ui_account) = token_info {
                let token_mint = FarmClient::pubkey_from_str(&ui_account.mint)?;
                if let Ok(token) = self.get_token_with_mint(&token_mint) {
                    res.push(token.name.as_str().to_string());
                } else {
                    res.push("B58.".to_string() + acc.pubkey.clone().as_str());
                }
            }
        }
        Ok(res)
    }

    /// Returns UiTokenAccount struct data for the associated token account address
    pub fn get_token_account_data(
        &self,
        wallet_address: &Pubkey,
        token_name: &str,
    ) -> Result<UiTokenAccount, FarmClientError> {
        let token_address = self.get_associated_token_address(wallet_address, token_name)?;
        let data = self.rpc_client.get_account_data(&token_address)?;
        let token = self.get_token(token_name)?;
        let res = parse_token(data.as_slice(), Some(token.decimals))?;
        if let TokenAccountType::Account(ui_account) = res {
            Ok(ui_account)
        } else {
            Err(FarmClientError::ValueError(format!(
                "No account data found for token {}",
                token_name
            )))
        }
    }

    /// Returns UiMint struct data for the associated token account address
    pub fn get_token_mint_data(
        &self,
        wallet_address: &Pubkey,
        token_name: &str,
    ) -> Result<UiMint, FarmClientError> {
        let token_address = self.get_associated_token_address(wallet_address, token_name)?;
        let data = self.rpc_client.get_account_data(&token_address)?;
        let res = parse_token(data.as_slice(), None)?;
        if let TokenAccountType::Mint(ui_mint) = res {
            Ok(ui_mint)
        } else {
            Err(FarmClientError::ValueError(format!(
                "No mint data found for token {}",
                token_name
            )))
        }
    }

    /// Returns native SOL balance
    pub fn get_account_balance(&self, wallet_address: &Pubkey) -> Result<f64, FarmClientError> {
        Ok(self.tokens_to_ui_amount_with_decimals(
            self.rpc_client.get_balance(wallet_address)?,
            spl_token::native_mint::DECIMALS,
        ))
    }

    /// Returns token balance for the associated token account address
    pub fn get_token_account_balance(
        &self,
        wallet_address: &Pubkey,
        token_name: &str,
    ) -> Result<f64, FarmClientError> {
        let token_name = if token_name == "WSOL" {
            "SOL"
        } else {
            token_name
        };
        let token_address = if token_name.len() > 4 && token_name.starts_with("B58.") {
            FarmClient::pubkey_from_str(&token_name[4..])?
        } else {
            self.get_associated_token_address(wallet_address, token_name)?
        };
        self.get_token_account_balance_with_address(&token_address)
    }

    /// Returns token balance for the specified token account address
    pub fn get_token_account_balance_with_address(
        &self,
        token_account: &Pubkey,
    ) -> Result<f64, FarmClientError> {
        if let Ok(balance) = self.rpc_client.get_token_account_balance(token_account) {
            if let Some(ui_amount) = balance.ui_amount {
                Ok(ui_amount)
            } else {
                Err(FarmClientError::ParseError(format!(
                    "Failed to parse balance for token address {}",
                    token_account
                )))
            }
        } else {
            Ok(0.0)
        }
    }

    /// Returns true if the associated token account exists and is initialized
    pub fn has_active_token_account(&self, wallet_address: &Pubkey, token_name: &str) -> bool {
        if let Ok(account) = self.get_token_account_data(wallet_address, token_name) {
            account.state == UiAccountState::Initialized
        } else {
            false
        }
    }

    /// Returns the account address where Vault stats are stored for the user
    pub fn get_vault_user_info_account(
        &self,
        wallet_address: &Pubkey,
        vault_name: &str,
    ) -> Result<Pubkey, FarmClientError> {
        let vault = self.get_vault(vault_name)?;
        Ok(Pubkey::find_program_address(
            &[
                b"user_info_account",
                wallet_address.as_ref(),
                vault.name.as_bytes(),
            ],
            &vault.vault_program_id,
        )
        .0)
    }

    /// Returns number of decimal digits of the Vault token
    pub fn get_vault_token_decimals(&self, vault_name: &str) -> Result<u8, FarmClientError> {
        let vault = self.get_vault(vault_name)?;
        if let Some(vault_token) = self.get_token_by_ref_from_cache(&Some(vault.vault_token_ref))? {
            Ok(vault_token.decimals)
        } else {
            Err(FarmClientError::RecordNotFound(format!(
                "Vault token for {}",
                vault_name
            )))
        }
    }

    /// Returns number of decimal digits for the Pool tokens
    pub fn get_pool_tokens_decimals(&self, pool_name: &str) -> Result<Vec<u8>, FarmClientError> {
        let pool = self.get_pool(pool_name)?;
        let mut res = vec![];
        if let Some(token) = self.get_token_by_ref_from_cache(&pool.lp_token_ref)? {
            res.push(token.decimals);
        }
        if let Some(token) = self.get_token_by_ref_from_cache(&pool.token_a_ref)? {
            res.push(token.decimals);
        }
        if let Some(token) = self.get_token_by_ref_from_cache(&pool.token_b_ref)? {
            res.push(token.decimals);
        }
        Ok(res)
    }

    /// Returns multisig account address for the Vault
    pub fn get_vault_multisig_account(&self, vault_name: &str) -> Result<Pubkey, FarmClientError> {
        let vault = self.get_vault(vault_name)?;
        Ok(Pubkey::find_program_address(
            &[b"multisig", vault.name.as_bytes()],
            &vault.vault_program_id,
        )
        .0)
    }

    /// Returns multisig address for the Vault or Main Router's multisig if former it not initialized
    pub fn get_vault_active_multisig_account(
        &self,
        vault_name: &str,
    ) -> Result<Pubkey, FarmClientError> {
        let vault_multisig_account = self.get_vault_multisig_account(vault_name)?;
        if let Ok(data) = self.rpc_client.get_account_data(&vault_multisig_account) {
            let _ = Multisig::unpack(&data)?;
            Ok(vault_multisig_account)
        } else {
            Ok(main_router_multisig::id())
        }
    }

    /// Returns current admin signers for the Vault
    pub fn get_vault_admins(&self, vault_name: &str) -> Result<Multisig, FarmClientError> {
        if let Ok(data) = self
            .rpc_client
            .get_account_data(&self.get_vault_active_multisig_account(vault_name)?)
        {
            Multisig::unpack(&data).map_err(|e| e.into())
        } else {
            Ok(Multisig::default())
        }
    }

    /// Initializes Vault multisig with a new set of signers
    pub fn set_vault_admins(
        &self,
        admin_signer: &dyn Signer,
        vault_name: &str,
        admin_signers: &[Pubkey],
        min_signatures: u8,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_set_vault_admins(
            &admin_signer.pubkey(),
            vault_name,
            admin_signers,
            min_signatures,
        )?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Removes Vault specific multisig, Main Router's will be used instead
    pub fn remove_vault_multisig(
        &self,
        admin_signer: &dyn Signer,
        vault_name: &str,
    ) -> Result<Signature, FarmClientError> {
        let inst =
            self.new_instruction_remove_vault_multisig(&admin_signer.pubkey(), vault_name)?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Returns user stats for specific Vault
    pub fn get_vault_user_info(
        &self,
        wallet_address: &Pubkey,
        vault_name: &str,
    ) -> Result<VaultUserInfo, FarmClientError> {
        let user_info_account = self.get_vault_user_info_account(wallet_address, vault_name)?;
        let data = self.rpc_client.get_account_data(&user_info_account)?;
        if !RefDB::is_initialized(data.as_slice()) {
            return Err(ProgramError::UninitializedAccount.into());
        }
        let mut user_info = VaultUserInfo::default();
        let rec_vec = RefDB::read_all(data.as_slice())?;
        for rec in rec_vec.iter() {
            if let refdb::Reference::U64 { data } = rec.reference {
                match rec.name.as_str() {
                    "LastDeposit" => user_info.last_deposit_time = data as UnixTimestamp,
                    "LastWithdrawal" => user_info.last_withdrawal_time = data as UnixTimestamp,
                    "TokenAAdded" => user_info.tokens_a_added = data,
                    "TokenBAdded" => user_info.tokens_b_added = data,
                    "TokenARemoved" => user_info.tokens_a_removed = data,
                    "TokenBRemoved" => user_info.tokens_b_removed = data,
                    "LpTokensDebt" => user_info.lp_tokens_debt = data,
                    _ => {}
                }
            }
        }

        Ok(user_info)
    }

    /// Returns Vault stats
    pub fn get_vault_info(&self, vault_name: &str) -> Result<VaultInfo, FarmClientError> {
        let vault = self.get_vault(vault_name)?;
        let data = self.rpc_client.get_account_data(&vault.info_account)?;
        if !RefDB::is_initialized(data.as_slice()) {
            return Err(ProgramError::UninitializedAccount.into());
        }
        let mut vault_info = VaultInfo::default();
        let rec_vec = RefDB::read_all(data.as_slice())?;
        for rec in rec_vec.iter() {
            if let refdb::Reference::U64 { data } = rec.reference {
                match rec.name.as_str() {
                    "CrankTime" => vault_info.crank_time = data as UnixTimestamp,
                    "CrankStep" => vault_info.crank_step = data,
                    "TokenAAdded" => vault_info.tokens_a_added = data,
                    "TokenBAdded" => vault_info.tokens_b_added = data,
                    "TokenARemoved" => vault_info.tokens_a_removed = data,
                    "TokenBRemoved" => vault_info.tokens_b_removed = data,
                    "TokenARewards" => vault_info.tokens_a_rewards = data,
                    "TokenBRewards" => vault_info.tokens_b_rewards = data,
                    "DepositAllowed" => vault_info.deposit_allowed = data > 0,
                    "WithdrawalAllowed" => vault_info.withdrawal_allowed = data > 0,
                    "MinCrankInterval" => vault_info.min_crank_interval = data,
                    "Fee" => vault_info.fee = f64::from_bits(data),
                    "ExternalFee" => vault_info.external_fee = f64::from_bits(data),
                    _ => {}
                }
            }
        }
        vault_info.stake_balance = self.get_vault_stake_balance(vault_name)?;

        Ok(vault_info)
    }

    /// Returns Vault stats for all Vaults
    pub fn get_all_vault_infos(&self) -> Result<Vec<VaultInfo>, FarmClientError> {
        let mut vault_infos = vec![];
        let vaults = self.get_vaults()?;
        for vault in vaults.keys() {
            vault_infos.push(self.get_vault_info(vault)?);
        }

        Ok(vault_infos)
    }

    /// Returns User's stacked balance
    pub fn get_user_stake_balance(
        &self,
        wallet_address: &Pubkey,
        farm_name: &str,
    ) -> Result<f64, FarmClientError> {
        let farm = self.get_farm(farm_name)?;
        match farm.route {
            FarmRoute::Raydium { .. } => {
                let stake_account = self.get_stake_account(wallet_address, farm_name)?;
                if let Ok(stake_data) = self.rpc_client.get_account_data(&stake_account) {
                    if !stake_data.is_empty() {
                        let deposit_balance = if farm.version >= 4 {
                            RaydiumUserStakeInfoV4::unpack(stake_data.as_slice())?.deposit_balance
                        } else {
                            RaydiumUserStakeInfo::unpack(stake_data.as_slice())?.deposit_balance
                        };
                        let farm_token = self.get_token_by_ref(&farm.lp_token_ref.unwrap())?;
                        Ok(self.tokens_to_ui_amount_with_decimals(
                            deposit_balance,
                            farm_token.decimals,
                        ))
                    } else {
                        Ok(0.0)
                    }
                } else {
                    Ok(0.0)
                }
            }
            FarmRoute::Saber { .. } => {
                let stake_account = self.get_stake_account(wallet_address, farm_name)?;
                if let Ok(stake_data) = self.rpc_client.get_account_data(&stake_account) {
                    if !stake_data.is_empty() {
                        let deposit_balance = Miner::unpack(stake_data.as_slice())?.balance;
                        let farm_token = self.get_token_by_ref(&farm.lp_token_ref.unwrap())?;
                        Ok(self.tokens_to_ui_amount_with_decimals(
                            deposit_balance,
                            farm_token.decimals,
                        ))
                    } else {
                        Ok(0.0)
                    }
                } else {
                    Ok(0.0)
                }
            }
            FarmRoute::Orca { farm_token_ref, .. } => {
                if let Ok(farm_token) = self.get_token_by_ref(&farm_token_ref) {
                    self.get_token_account_balance(wallet_address, &farm_token.name)
                } else {
                    Ok(0.0)
                }
            }
        }
    }

    /// Returns Vault's stacked balance
    pub fn get_vault_stake_balance(&self, vault_name: &str) -> Result<f64, FarmClientError> {
        let vault = self.get_vault(vault_name)?;
        match vault.strategy {
            VaultStrategy::StakeLpCompoundRewards {
                farm_ref,
                vault_stake_info,
                ..
            } => {
                let farm = self.get_farm_by_ref(&farm_ref)?;
                let farm_token = self.get_token_by_ref(&farm.lp_token_ref.unwrap())?;

                let balance =
                    if let Ok(stake_data) = self.rpc_client.get_account_data(&vault_stake_info) {
                        if !stake_data.is_empty() {
                            match farm.route {
                                FarmRoute::Raydium { .. } => {
                                    if farm.version >= 4 {
                                        RaydiumUserStakeInfoV4::unpack(stake_data.as_slice())?
                                            .deposit_balance
                                    } else {
                                        RaydiumUserStakeInfo::unpack(stake_data.as_slice())?
                                            .deposit_balance
                                    }
                                }
                                FarmRoute::Saber { .. } => {
                                    Miner::unpack(stake_data.as_slice())?.balance
                                }
                                FarmRoute::Orca { .. } => {
                                    OrcaUserStakeInfo::unpack(stake_data.as_slice())?
                                        .base_tokens_converted
                                }
                            }
                        } else {
                            0
                        }
                    } else {
                        0
                    };
                Ok(self.tokens_to_ui_amount_with_decimals(balance, farm_token.decimals))
            }
            _ => Ok(0.0),
        }
    }

    /// Initializes a new User for the Vault
    pub fn user_init_vault(
        &self,
        signer: &dyn Signer,
        vault_name: &str,
    ) -> Result<Signature, FarmClientError> {
        // create and send the instruction
        let inst = self.new_instruction_user_init_vault(&signer.pubkey(), vault_name)?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Adds liquidity to the Vault
    pub fn add_liquidity_vault(
        &self,
        signer: &dyn Signer,
        vault_name: &str,
        max_token_a_ui_amount: f64,
        max_token_b_ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.all_instructions_add_liquidity_vault(
            &signer.pubkey(),
            vault_name,
            max_token_a_ui_amount,
            max_token_b_ui_amount,
        )?;
        Ok(*self
            .sign_and_send_instructions_in_batches(&[signer], &inst)?
            .last()
            .unwrap())
    }

    /// Adds locked liquidity to the Vault.
    /// Useful if add liquidity operation partially failed.
    pub fn add_locked_liquidity_vault(
        &self,
        signer: &dyn Signer,
        vault_name: &str,
        ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.all_instructions_add_locked_liquidity_vault(
            &signer.pubkey(),
            vault_name,
            ui_amount,
        )?;
        self.sign_and_send_instructions(&[signer], &inst)
    }

    /// Removes liquidity from the Vault
    pub fn remove_liquidity_vault(
        &self,
        signer: &dyn Signer,
        vault_name: &str,
        ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        let inst =
            self.all_instructions_remove_liquidity_vault(&signer.pubkey(), vault_name, ui_amount)?;
        Ok(*self
            .sign_and_send_instructions_in_batches(&[signer], &inst)?
            .last()
            .unwrap())
    }

    /// Removes unlocked liquidity from the Vault.
    /// Useful if remove liquidity operation failed after unlock step.
    pub fn remove_unlocked_liquidity_vault(
        &self,
        signer: &dyn Signer,
        vault_name: &str,
        ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.all_instructions_remove_unlocked_liquidity_vault(
            &signer.pubkey(),
            vault_name,
            ui_amount,
        )?;
        self.sign_and_send_instructions(&[signer], &inst)
    }

    /// Adds liquidity to the Pool.
    /// If one of token amounts is set to zero it will be determined based on the pool
    /// price and the specified amount of another token.
    pub fn add_liquidity_pool(
        &self,
        signer: &dyn Signer,
        pool_name: &str,
        max_token_a_ui_amount: f64,
        max_token_b_ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.all_instructions_add_liquidity_pool(
            &signer.pubkey(),
            pool_name,
            max_token_a_ui_amount,
            max_token_b_ui_amount,
        )?;
        self.sign_and_send_instructions(&[signer], &inst)
    }

    /// Removes liquidity from the Pool.
    /// If the amount is set to zero entire balance will be removed from the pool.
    pub fn remove_liquidity_pool(
        &self,
        signer: &dyn Signer,
        pool_name: &str,
        ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        let inst =
            self.all_instructions_remove_liquidity_pool(&signer.pubkey(), pool_name, ui_amount)?;
        self.sign_and_send_instructions(&[signer], &inst)
    }

    /// Swaps tokens
    pub fn swap(
        &self,
        signer: &dyn Signer,
        protocol: Protocol,
        from_token: &str,
        to_token: &str,
        ui_amount_in: f64,
        min_ui_amount_out: f64,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.all_instructions_swap(
            &signer.pubkey(),
            protocol,
            from_token,
            to_token,
            ui_amount_in,
            min_ui_amount_out,
        )?;
        self.sign_and_send_instructions(&[signer], &inst)
    }

    /// Initializes a new User for the Farm
    pub fn user_init(
        &self,
        signer: &dyn Signer,
        farm_name: &str,
    ) -> Result<Signature, FarmClientError> {
        // create and send the instruction
        let inst = self.new_instruction_user_init(&signer.pubkey(), farm_name)?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Stakes tokens to the Farm.
    /// If the amount is set to zero entire LP tokens balance will be staked.
    pub fn stake(
        &self,
        signer: &dyn Signer,
        farm_name: &str,
        ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.all_instructions_stake(&signer.pubkey(), farm_name, ui_amount)?;
        self.sign_and_send_instructions(&[signer], &inst)
    }

    /// Unstakes tokens from the Farm.
    /// If the amount is set to zero entire balance will be unstaked.
    pub fn unstake(
        &self,
        signer: &dyn Signer,
        farm_name: &str,
        ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.all_instructions_unstake(&signer.pubkey(), farm_name, ui_amount)?;
        self.sign_and_send_instructions(&[signer], &inst)
    }

    /// Harvests rewards from the Farm
    pub fn harvest(
        &self,
        signer: &dyn Signer,
        farm_name: &str,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.all_instructions_harvest(&signer.pubkey(), farm_name)?;
        self.sign_and_send_instructions(&[signer], &inst)
    }

    /// Clears cache records to force re-pull from blockchain
    pub fn reset_cache(&self) {
        self.tokens.borrow_mut().reset();
        self.pools.borrow_mut().reset();
        self.vaults.borrow_mut().reset();
        self.funds.borrow_mut().reset();
        self.token_refs.borrow_mut().reset();
        self.pool_refs.borrow_mut().reset();
        self.vault_refs.borrow_mut().reset();
        self.fund_refs.borrow_mut().reset();
        self.official_ids.borrow_mut().reset();
        self.latest_pools.borrow_mut().clear();
        self.latest_farms.borrow_mut().clear();
        self.latest_vaults.borrow_mut().clear();
    }

    /// Reads records from the RefDB PDA into a Pubkey map
    pub fn get_refdb_pubkey_map(
        &self,
        refdb_name: &str,
    ) -> Result<(Header, PubkeyMap), FarmClientError> {
        let refdb_address = refdb::find_refdb_pda(refdb_name).0;
        let data = self.rpc_client.get_account_data(&refdb_address)?;
        if !RefDB::is_initialized(data.as_slice()) {
            return Err(ProgramError::UninitializedAccount.into());
        }
        let mut map = PubkeyMap::default();
        let rec_vec = RefDB::read_all(data.as_slice())?;
        for rec in rec_vec.iter() {
            if let refdb::Reference::Pubkey { data } = rec.reference {
                map.insert(rec.name.to_string(), data);
            }
        }
        Ok((RefDB::get_storage_header(data.as_slice())?, map))
    }

    /// Returns raw RefDB data, can be further used with refdb::RefDB
    pub fn get_refdb_data(&self, refdb_name: &str) -> Result<Vec<u8>, FarmClientError> {
        let refdb_address = refdb::find_refdb_pda(refdb_name).0;
        self.rpc_client
            .get_account_data(&refdb_address)
            .map_err(Into::into)
    }

    /// Returns the index of the record with the specified name
    pub fn get_refdb_index(
        &self,
        refdb_name: &str,
        object_name: &str,
    ) -> Result<Option<usize>, FarmClientError> {
        RefDB::find_index(
            self.get_refdb_data(refdb_name)?.as_slice(),
            &str_to_as64(object_name)?,
        )
        .map_err(Into::into)
    }

    /// Returns the index of the first empty record at the back of the RefDB storage,
    /// i.e. there will be no active records after the index
    pub fn get_refdb_last_index(&self, refdb_name: &str) -> Result<u32, FarmClientError> {
        RefDB::find_last_index(self.get_refdb_data(refdb_name)?.as_slice()).map_err(Into::into)
    }

    /// Returns the index of the next available record to write to in the RefDB storage
    pub fn get_refdb_next_index(&self, refdb_name: &str) -> Result<u32, FarmClientError> {
        RefDB::find_next_index(self.get_refdb_data(refdb_name)?.as_slice()).map_err(Into::into)
    }

    /// Checks if RefDB is initialized
    pub fn is_refdb_initialized(&self, refdb_name: &str) -> Result<bool, FarmClientError> {
        let refdb_address = refdb::find_refdb_pda(refdb_name).0;
        if let Ok(data) = self.rpc_client.get_account_data(&refdb_address) {
            Ok(RefDB::is_initialized(data.as_slice()))
        } else {
            Ok(false)
        }
    }

    /// Initializes a new RefDB storage
    pub fn initialize_refdb(
        &self,
        admin_signer: &dyn Signer,
        refdb_name: &str,
        reference_type: refdb::ReferenceType,
        max_records: usize,
        init_account: bool,
    ) -> Result<Signature, FarmClientError> {
        if init_account && !refdb::REFDB_ONCHAIN_INIT {
            let refdb_address = refdb::find_refdb_pda(refdb_name).0;
            if let Ok(refdb_account) = self.rpc_client.get_account(&refdb_address) {
                if refdb_account.owner != main_router::id() {
                    return Err(FarmClientError::ValueError(format!(
                        "RefDB account owner mismatch {}",
                        refdb_address
                    )));
                }
            } else {
                if admin_signer.pubkey() != main_router_admin::id() {
                    return Err(FarmClientError::ValueError(
                        "RefDB init must be initially called with main_router_admin::id() if on-chain init is disabled"
                            .to_string(),
                    ));
                }
                self.create_system_account_with_seed(
                    admin_signer,
                    &admin_signer.pubkey(),
                    refdb_name,
                    0,
                    refdb::StorageType::get_storage_size_for_records(reference_type, max_records),
                    &main_router::id(),
                )?;
            }
        }

        let inst = self.new_instruction_refdb_init(
            &admin_signer.pubkey(),
            refdb_name,
            reference_type,
            max_records as u32,
            init_account,
        )?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Initializes Main Router multisig with a new set of signers
    pub fn set_admins(
        &self,
        admin_signer: &dyn Signer,
        admin_signers: &[Pubkey],
        min_signatures: u8,
    ) -> Result<Signature, FarmClientError> {
        let inst =
            self.new_instruction_set_admins(&admin_signer.pubkey(), admin_signers, min_signatures)?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Returns current admin signers for the Main Router
    pub fn get_admins(&self) -> Result<Multisig, FarmClientError> {
        if let Ok(data) = self
            .rpc_client
            .get_account_data(&main_router_multisig::id())
        {
            Multisig::unpack(&data).map_err(|e| e.into())
        } else {
            Ok(Multisig::default())
        }
    }

    /// Removes the RefDB storage
    pub fn drop_refdb(
        &self,
        admin_signer: &dyn Signer,
        refdb_name: &str,
        close_account: bool,
    ) -> Result<Signature, FarmClientError> {
        let inst =
            self.new_instruction_refdb_drop(&admin_signer.pubkey(), refdb_name, close_account)?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Removes referenced metadata from chain
    pub fn remove_reference(
        &self,
        admin_signer: &dyn Signer,
        storage_type: refdb::StorageType,
        object_name: &str,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_remove_reference(
            &admin_signer.pubkey(),
            storage_type,
            object_name,
        )?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Records the Program ID metadata on-chain
    pub fn add_program_id(
        &self,
        admin_signer: &dyn Signer,
        name: &str,
        program_id: &Pubkey,
        program_id_type: ProgramIDType,
        refdb_index: Option<usize>,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_add_program_id(
            &admin_signer.pubkey(),
            name,
            program_id,
            program_id_type,
            refdb_index,
        )?;
        let res = self.sign_and_send_instructions(&[admin_signer], &[inst]);
        if res.is_ok() {
            self.official_ids
                .borrow_mut()
                .data
                .insert(name.to_string(), *program_id);
        }
        res
    }

    /// Removes the Program ID metadata from chain
    pub fn remove_program_id(
        &self,
        admin_signer: &dyn Signer,
        name: &str,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_remove_program_id(&admin_signer.pubkey(), name)?;
        let res = self.sign_and_send_instructions(&[admin_signer], &[inst]);
        if res.is_ok() {
            self.official_ids.borrow_mut().data.remove(name);
        }
        res
    }

    /// Records the Fund metadata
    pub fn add_fund(
        &self,
        admin_signer: &dyn Signer,
        fund: Fund,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_add_fund(&admin_signer.pubkey(), fund)?;
        let res = self.sign_and_send_instructions(&[admin_signer], &[inst]);
        if res.is_ok() {
            self.funds
                .borrow_mut()
                .data
                .insert(fund.name.to_string(), fund);
            self.fund_refs.borrow_mut().data.insert(
                fund.name.to_string(),
                refdb::find_target_pda(refdb::StorageType::Fund, &fund.name).0,
            );
        }
        res
    }

    /// Removes the Fund's on-chain metadata
    pub fn remove_fund(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_remove_fund(&admin_signer.pubkey(), fund_name)?;
        let res = self.sign_and_send_instructions(&[admin_signer], &[inst]);
        if res.is_ok() {
            self.funds.borrow_mut().data.remove(fund_name);
            self.fund_refs.borrow_mut().data.remove(fund_name);
        }
        res
    }

    /// Records the Vault metadata on-chain
    pub fn add_vault(
        &self,
        admin_signer: &dyn Signer,
        vault: Vault,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_add_vault(&admin_signer.pubkey(), vault)?;
        let res = self.sign_and_send_instructions(&[admin_signer], &[inst]);
        if res.is_ok() {
            self.vaults
                .borrow_mut()
                .data
                .insert(vault.name.to_string(), vault);
            self.vault_refs.borrow_mut().data.insert(
                vault.name.to_string(),
                refdb::find_target_pda(refdb::StorageType::Vault, &vault.name).0,
            );
            FarmClient::reinsert_latest_versions(
                &self.vault_refs.borrow().data,
                &mut self.latest_vaults.borrow_mut(),
            );
        }
        res
    }

    /// Removes the Vault's on-chain metadata
    pub fn remove_vault(
        &self,
        admin_signer: &dyn Signer,
        vault_name: &str,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_remove_vault(&admin_signer.pubkey(), vault_name)?;
        let res = self.sign_and_send_instructions(&[admin_signer], &[inst]);
        if res.is_ok() {
            self.vaults.borrow_mut().data.remove(vault_name);
            self.vault_refs.borrow_mut().data.remove(vault_name);
            FarmClient::reinsert_latest_versions(
                &self.vault_refs.borrow().data,
                &mut self.latest_vaults.borrow_mut(),
            );
        }
        res
    }

    /// Records the Pool metadata on-chain
    pub fn add_pool(
        &self,
        admin_signer: &dyn Signer,
        pool: Pool,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_add_pool(&admin_signer.pubkey(), pool)?;
        let res = self.sign_and_send_instructions(&[admin_signer], &[inst]);
        if res.is_ok() {
            self.pools
                .borrow_mut()
                .data
                .insert(pool.name.to_string(), pool);
            self.pool_refs.borrow_mut().data.insert(
                pool.name.to_string(),
                refdb::find_target_pda(refdb::StorageType::Pool, &pool.name).0,
            );
            FarmClient::reinsert_latest_versions(
                &self.pool_refs.borrow().data,
                &mut self.latest_pools.borrow_mut(),
            );
        }
        res
    }

    /// Removes the Pool's on-chain metadata
    pub fn remove_pool(
        &self,
        admin_signer: &dyn Signer,
        pool_name: &str,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_remove_pool(&admin_signer.pubkey(), pool_name)?;
        let res = self.sign_and_send_instructions(&[admin_signer], &[inst]);
        if res.is_ok() {
            self.pools.borrow_mut().data.remove(pool_name);
            self.pool_refs.borrow_mut().data.remove(pool_name);
            FarmClient::reinsert_latest_versions(
                &self.pool_refs.borrow().data,
                &mut self.latest_pools.borrow_mut(),
            );
        }
        res
    }

    /// Records the Farm metadata on-chain
    pub fn add_farm(
        &self,
        admin_signer: &dyn Signer,
        farm: Farm,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_add_farm(&admin_signer.pubkey(), farm)?;
        let res = self.sign_and_send_instructions(&[admin_signer], &[inst]);
        if res.is_ok() {
            self.farms
                .borrow_mut()
                .data
                .insert(farm.name.to_string(), farm);
            self.farm_refs.borrow_mut().data.insert(
                farm.name.to_string(),
                refdb::find_target_pda(refdb::StorageType::Farm, &farm.name).0,
            );
            FarmClient::reinsert_latest_versions(
                &self.farm_refs.borrow().data,
                &mut self.latest_farms.borrow_mut(),
            );
        }
        res
    }

    /// Removes the Farm's on-chain metadata
    pub fn remove_farm(
        &self,
        admin_signer: &dyn Signer,
        farm_name: &str,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_remove_farm(&admin_signer.pubkey(), farm_name)?;
        let res = self.sign_and_send_instructions(&[admin_signer], &[inst]);
        if res.is_ok() {
            self.farms.borrow_mut().data.remove(farm_name);
            self.farm_refs.borrow_mut().data.remove(farm_name);
            FarmClient::reinsert_latest_versions(
                &self.farm_refs.borrow().data,
                &mut self.latest_farms.borrow_mut(),
            );
        }
        res
    }

    /// Records the Token metadata on-chain
    pub fn add_token(
        &self,
        admin_signer: &dyn Signer,
        token: Token,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_add_token(&admin_signer.pubkey(), token)?;
        let res = self.sign_and_send_instructions(&[admin_signer], &[inst]);
        if res.is_ok() {
            self.tokens
                .borrow_mut()
                .data
                .insert(token.name.to_string(), token);
            self.token_refs.borrow_mut().data.insert(
                token.name.to_string(),
                refdb::find_target_pda(refdb::StorageType::Token, &token.name).0,
            );
        }
        res
    }

    /// Removes the Token's on-chain metadata
    pub fn remove_token(
        &self,
        admin_signer: &dyn Signer,
        token_name: &str,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_remove_token(&admin_signer.pubkey(), token_name)?;
        let res = self.sign_and_send_instructions(&[admin_signer], &[inst]);
        if res.is_ok() {
            self.tokens.borrow_mut().data.remove(token_name);
            self.token_refs.borrow_mut().data.remove(token_name);
        }
        res
    }

    /// Initializes a Vault
    pub fn init_vault(
        &self,
        admin_signer: &dyn Signer,
        vault_name: &str,
        step: u64,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_init_vault(&admin_signer.pubkey(), vault_name, step)?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Shutdowns a Vault
    pub fn shutdown_vault(
        &self,
        admin_signer: &dyn Signer,
        vault_name: &str,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_shutdown_vault(&admin_signer.pubkey(), vault_name)?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Cranks single Vault
    pub fn crank_vault(
        &self,
        signer: &dyn Signer,
        vault_name: &str,
        step: u64,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_crank_vault(&signer.pubkey(), vault_name, step)?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Cranks all Vaults
    pub fn crank_vaults(&self, signer: &dyn Signer, step: u64) -> Result<usize, FarmClientError> {
        let vaults = self.get_vaults()?;
        for vault_name in vaults.keys() {
            let _ = self.crank_vault(signer, vault_name, step)?;
        }
        Ok(vaults.len())
    }

    /// Withdraw collected fees from the Vault
    pub fn withdraw_fees_vault(
        &self,
        signer: &dyn Signer,
        vault_name: &str,
        fee_token: TokenSelector,
        ui_amount: f64,
        receiver: &Pubkey,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_withdraw_fees_vault(
            &signer.pubkey(),
            vault_name,
            fee_token,
            ui_amount,
            receiver,
        )?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Sets the Vault's min crank interval
    pub fn set_min_crank_interval_vault(
        &self,
        admin_signer: &dyn Signer,
        vault_name: &str,
        min_crank_interval: u32,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_set_min_crank_interval_vault(
            &admin_signer.pubkey(),
            vault_name,
            min_crank_interval,
        )?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Sets the Vault's fee
    pub fn set_fee_vault(
        &self,
        admin_signer: &dyn Signer,
        vault_name: &str,
        fee_percent: f32,
    ) -> Result<Signature, FarmClientError> {
        let inst =
            self.new_instruction_set_fee_vault(&admin_signer.pubkey(), vault_name, fee_percent)?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Sets the Vault's external fee
    pub fn set_external_fee_vault(
        &self,
        admin_signer: &dyn Signer,
        vault_name: &str,
        external_fee_percent: f32,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_set_external_fee_vault(
            &admin_signer.pubkey(),
            vault_name,
            external_fee_percent,
        )?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Disables deposits to the Vault
    pub fn disable_deposits_vault(
        &self,
        admin_signer: &dyn Signer,
        vault_name: &str,
    ) -> Result<Signature, FarmClientError> {
        let inst =
            self.new_instruction_disable_deposits_vault(&admin_signer.pubkey(), vault_name)?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Enables deposits to the Vault
    pub fn enable_deposits_vault(
        &self,
        admin_signer: &dyn Signer,
        vault_name: &str,
    ) -> Result<Signature, FarmClientError> {
        let inst =
            self.new_instruction_enable_deposits_vault(&admin_signer.pubkey(), vault_name)?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Disables withdrawal from the Vault
    pub fn disable_withdrawals_vault(
        &self,
        admin_signer: &dyn Signer,
        vault_name: &str,
    ) -> Result<Signature, FarmClientError> {
        let inst =
            self.new_instruction_disable_withdrawals_vault(&admin_signer.pubkey(), vault_name)?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Enables withdrawals from the Vault
    pub fn enable_withdrawals_vault(
        &self,
        admin_signer: &dyn Signer,
        vault_name: &str,
    ) -> Result<Signature, FarmClientError> {
        let inst =
            self.new_instruction_enable_withdrawals_vault(&admin_signer.pubkey(), vault_name)?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Deposits governing tokens to the farms realm
    pub fn governance_tokens_deposit(
        &self,
        signer: &dyn Signer,
        ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_governance_tokens_deposit(&signer.pubkey(), ui_amount)?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Withdraws governing tokens from the farms realm
    pub fn governance_tokens_withdraw(
        &self,
        signer: &dyn Signer,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_governance_tokens_withdraw(&signer.pubkey())?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Creates a new governance proposal
    pub fn governance_proposal_new(
        &self,
        signer: &dyn Signer,
        governance_name: &str,
        proposal_name: &str,
        proposal_link: &str,
        proposal_index: u32,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_governance_proposal_new(
            &signer.pubkey(),
            governance_name,
            proposal_name,
            proposal_link,
            proposal_index,
        )?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Cancels governance proposal
    pub fn governance_proposal_cancel(
        &self,
        signer: &dyn Signer,
        governance_name: &str,
        proposal_index: u32,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_governance_proposal_cancel(
            &signer.pubkey(),
            governance_name,
            proposal_index,
        )?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Adds a signatory to governance proposal
    pub fn governance_signatory_add(
        &self,
        signer: &dyn Signer,
        governance_name: &str,
        proposal_index: u32,
        signatory: &Pubkey,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_governance_signatory_add(
            &signer.pubkey(),
            governance_name,
            proposal_index,
            signatory,
        )?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Removes the signatory from governance proposal
    pub fn governance_signatory_remove(
        &self,
        signer: &dyn Signer,
        governance_name: &str,
        proposal_index: u32,
        signatory: &Pubkey,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_governance_signatory_remove(
            &signer.pubkey(),
            governance_name,
            proposal_index,
            signatory,
        )?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Signs off governance proposal
    pub fn governance_sign_off(
        &self,
        signer: &dyn Signer,
        governance_name: &str,
        proposal_index: u32,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_governance_sign_off(
            &signer.pubkey(),
            governance_name,
            proposal_index,
        )?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Casts a vote on governance proposal
    pub fn governance_vote_cast(
        &self,
        signer: &dyn Signer,
        governance_name: &str,
        proposal_index: u32,
        vote: u8,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_governance_vote_cast(
            &signer.pubkey(),
            governance_name,
            proposal_index,
            vote,
        )?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Removes the vote from governance proposal
    pub fn governance_vote_relinquish(
        &self,
        signer: &dyn Signer,
        governance_name: &str,
        proposal_index: u32,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_governance_vote_relinquish(
            &signer.pubkey(),
            governance_name,
            proposal_index,
        )?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Finalizes the vote on governance proposal
    pub fn governance_vote_finalize(
        &self,
        signer: &dyn Signer,
        governance_name: &str,
        proposal_index: u32,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_governance_vote_finalize(
            &signer.pubkey(),
            governance_name,
            proposal_index,
        )?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Adds a new instruction to governance proposal
    pub fn governance_instruction_insert(
        &self,
        signer: &dyn Signer,
        governance_name: &str,
        proposal_index: u32,
        instruction_index: u16,
        instruction: &Instruction,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_governance_instruction_insert(
            &signer.pubkey(),
            governance_name,
            proposal_index,
            instruction_index,
            instruction,
        )?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Removes the instruction from governance proposal
    pub fn governance_instruction_remove(
        &self,
        signer: &dyn Signer,
        governance_name: &str,
        proposal_index: u32,
        instruction_index: u16,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_governance_instruction_remove(
            &signer.pubkey(),
            governance_name,
            proposal_index,
            instruction_index,
        )?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Executes the instruction in governance proposal
    pub fn governance_instruction_execute(
        &self,
        signer: &dyn Signer,
        governance_name: &str,
        proposal_index: u32,
        instruction_index: u16,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_governance_instruction_execute(
            &signer.pubkey(),
            governance_name,
            proposal_index,
            instruction_index,
        )?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Marks the instruction in governance proposal as failed
    pub fn governance_instruction_flag_error(
        &self,
        signer: &dyn Signer,
        governance_name: &str,
        proposal_index: u32,
        instruction_index: u16,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_governance_instruction_flag_error(
            &signer.pubkey(),
            governance_name,
            proposal_index,
            instruction_index,
        )?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Returns current governance config
    pub fn governance_get_config(
        &self,
        governance_name: &str,
    ) -> Result<GovernanceConfig, FarmClientError> {
        let governance = self.governance_get_address(governance_name)?;
        let governance_data = self.rpc_client.get_account_data(&governance)?;

        let account: Governance = try_from_slice_unchecked(&governance_data)
            .map_err(|e| FarmClientError::IOError(e.to_string()))?;
        if account.account_type == GovernanceAccountType::AccountGovernance
            || account.account_type == GovernanceAccountType::ProgramGovernance
            || account.account_type == GovernanceAccountType::MintGovernance
            || account.account_type == GovernanceAccountType::TokenGovernance
        {
            Ok(account.config)
        } else {
            Err(ProgramError::UninitializedAccount.into())
        }
    }

    // Returns account address of the governance
    pub fn governance_get_address(&self, governance_name: &str) -> Result<Pubkey, FarmClientError> {
        let dao_program = self.get_program_id(DAO_PROGRAM_NAME)?;
        let realm_address = get_realm_address(&dao_program, DAO_PROGRAM_NAME);
        match governance_name {
            DAO_MINT_NAME => {
                let dao_token = self.get_token(DAO_TOKEN_NAME)?;
                Ok(get_mint_governance_address(
                    &dao_program,
                    &realm_address,
                    &dao_token.mint,
                ))
            }
            DAO_CUSTODY_NAME => {
                let governed_account =
                    Pubkey::find_program_address(&[DAO_CUSTODY_NAME.as_bytes()], &dao_program).0;
                Ok(get_account_governance_address(
                    &dao_program,
                    &realm_address,
                    &governed_account,
                ))
            }
            _ => {
                let governed_program = self.get_program_id(governance_name)?;
                Ok(get_program_governance_address(
                    &dao_program,
                    &realm_address,
                    &governed_program,
                ))
            }
        }
    }

    // Returns stored instruction in the proposal
    pub fn governance_get_instruction(
        &self,
        governance_name: &str,
        proposal_index: u32,
        instruction_index: u16,
    ) -> Result<Instruction, FarmClientError> {
        let dao_program = self.get_program_id(DAO_PROGRAM_NAME)?;
        let dao_token = self.get_token(DAO_TOKEN_NAME)?;
        let governance = self.governance_get_address(governance_name)?;
        let proposal_address = get_proposal_address(
            &dao_program,
            &governance,
            &dao_token.mint,
            &proposal_index.to_le_bytes(),
        );

        let instruction_address = get_proposal_instruction_address(
            &dao_program,
            &proposal_address,
            &0u16.to_le_bytes(),
            &instruction_index.to_le_bytes(),
        );

        let data = self.rpc_client.get_account_data(&instruction_address)?;
        let ins_data: InstructionData =
            try_from_slice_unchecked::<ProposalInstructionV2>(data.as_slice())
                .map_err(|e| FarmClientError::IOError(e.to_string()))?
                .instruction;
        Ok((&ins_data).into())
    }

    /// Returns the state of the proposal
    pub fn governance_get_proposal_state(
        &self,
        governance_name: &str,
        proposal_index: u32,
    ) -> Result<ProposalV2, FarmClientError> {
        let dao_program = self.get_program_id(DAO_PROGRAM_NAME)?;
        let dao_token = self.get_token(DAO_TOKEN_NAME)?;
        let governance = self.governance_get_address(governance_name)?;
        let proposal_address = get_proposal_address(
            &dao_program,
            &governance,
            &dao_token.mint,
            &proposal_index.to_le_bytes(),
        );

        let proposal_data = self.rpc_client.get_account_data(&proposal_address)?;
        let proposal_state: ProposalV2 = try_from_slice_unchecked(&proposal_data)
            .map_err(|e| FarmClientError::IOError(e.to_string()))?;

        Ok(proposal_state)
    }

    /// Returns multisig account address for the Fund
    pub fn get_fund_multisig_account(&self, fund_name: &str) -> Result<Pubkey, FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        Ok(Pubkey::find_program_address(
            &[b"multisig", fund.name.as_bytes()],
            &fund.fund_program_id,
        )
        .0)
    }

    /// Returns multisig address for the Fund or Main Router's multisig if former it not initialized
    pub fn get_fund_active_multisig_account(
        &self,
        fund_name: &str,
    ) -> Result<Pubkey, FarmClientError> {
        let fund_multisig_account = self.get_fund_multisig_account(fund_name)?;
        if let Ok(data) = self.rpc_client.get_account_data(&fund_multisig_account) {
            let _ = Multisig::unpack(&data)?;
            Ok(fund_multisig_account)
        } else {
            Ok(main_router_multisig::id())
        }
    }

    /// Returns current admin signers for the Fund
    pub fn get_fund_admins(&self, fund_name: &str) -> Result<Multisig, FarmClientError> {
        if let Ok(data) = self
            .rpc_client
            .get_account_data(&self.get_fund_active_multisig_account(fund_name)?)
        {
            Multisig::unpack(&data).map_err(|e| e.into())
        } else {
            Ok(Multisig::default())
        }
    }

    /// Initializes Fund multisig with a new set of signers
    pub fn set_fund_admins(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
        admin_signers: &[Pubkey],
        min_signatures: u8,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_set_fund_admins(
            &admin_signer.pubkey(),
            fund_name,
            admin_signers,
            min_signatures,
        )?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Removes Fund specific multisig, Main Router's will be used instead
    pub fn remove_fund_multisig(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_remove_fund_multisig(&admin_signer.pubkey(), fund_name)?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Returns the account address where Fund stats are stored for the user
    pub fn get_fund_user_info_account(
        &self,
        wallet_address: &Pubkey,
        fund_name: &str,
    ) -> Result<Pubkey, FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        Ok(Pubkey::find_program_address(
            &[
                b"user_info_account",
                wallet_address.as_ref(),
                fund.name.as_bytes(),
            ],
            &fund.fund_program_id,
        )
        .0)
    }

    /// Returns user stats for specific Fund
    pub fn get_fund_user_info(
        &self,
        wallet_address: &Pubkey,
        fund_name: &str,
    ) -> Result<FundUserInfo, FarmClientError> {
        let user_info_account = self.get_fund_user_info_account(wallet_address, fund_name)?;
        let data = self.rpc_client.get_account_data(&user_info_account)?;
        if !RefDB::is_initialized(data.as_slice()) {
            return Err(ProgramError::UninitializedAccount.into());
        }
        let mut fund_user_info = FundUserInfo::default();
        let rec_vec = RefDB::read_all(data.as_slice())?;
        for rec in rec_vec.iter() {
            if let refdb::Reference::U64 { data } = rec.reference {
                if rec.name.as_str() == "VirtualTokensBalance" {
                    fund_user_info.virtual_tokens_balance = data
                }
            }
        }

        Ok(fund_user_info)
    }

    /// Returns user stats for all Funds
    pub fn get_all_fund_user_infos(
        &self,
        wallet_address: &Pubkey,
    ) -> Result<Vec<FundUserInfo>, FarmClientError> {
        let mut user_infos = vec![];
        let funds = self.get_funds()?;
        for fund in funds.keys() {
            user_infos.push(self.get_fund_user_info(wallet_address, fund)?);
        }

        Ok(user_infos)
    }

    /// Returns the account address where user requests are stored for the Fund
    pub fn get_fund_user_requests_account(
        &self,
        wallet_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
    ) -> Result<Pubkey, FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        let token = self.get_token(token_name)?;
        Ok(Pubkey::find_program_address(
            &[
                b"user_requests_account",
                token.name.as_bytes(),
                wallet_address.as_ref(),
                fund.name.as_bytes(),
            ],
            &fund.fund_program_id,
        )
        .0)
    }

    /// Returns user requests for specific Fund and token
    pub fn get_fund_user_requests(
        &self,
        wallet_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
    ) -> Result<FundUserRequests, FarmClientError> {
        let user_requests_account =
            self.get_fund_user_requests_account(wallet_address, fund_name, token_name)?;
        let data = self.rpc_client.get_account_data(&user_requests_account)?;
        FundUserRequests::unpack(data.as_slice()).map_err(|e| e.into())
    }

    /// Returns user requests for all tokens accepted by the Fund
    pub fn get_all_fund_user_requests(
        &self,
        fund_name: &str,
    ) -> Result<Vec<FundUserRequests>, FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        // search for user requests accounts
        let bytes = [
            &DISCRIMINATOR_FUND_USER_REQUESTS.to_le_bytes(),
            fund_ref.as_ref(),
        ]
        .concat()
        .to_vec();
        let acc_vec = self.get_accounts_with_filter(&fund.fund_program_id, 0, bytes)?;

        let mut res = vec![];
        for (_, acc) in &acc_vec {
            res.push(FundUserRequests::unpack(acc.data.as_slice())?);
        }

        Ok(res)
    }

    /// Returns Fund info and config
    pub fn get_fund_info(&self, fund_name: &str) -> Result<FundInfo, FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        let data = self.rpc_client.get_account_data(&fund.info_account)?;
        if !RefDB::is_initialized(data.as_slice()) {
            return Err(ProgramError::UninitializedAccount.into());
        }
        let mut fund_info = FundInfo::default();
        let rec_vec = RefDB::read_all(data.as_slice())?;
        for rec in rec_vec.iter() {
            if let refdb::Reference::U64 { data } = rec.reference {
                match rec.name.as_str() {
                    "DepositStartTime" => {
                        fund_info.deposit_schedule.start_time = data as UnixTimestamp
                    }
                    "DepositEndTime" => fund_info.deposit_schedule.end_time = data as UnixTimestamp,
                    "DepositApprovalRequired" => {
                        fund_info.deposit_schedule.approval_required = data != 0
                    }
                    "DepositMinAmountUsd" => {
                        fund_info.deposit_schedule.min_amount_usd = f64::from_bits(data)
                    }
                    "DepositMaxAmountUsd" => {
                        fund_info.deposit_schedule.max_amount_usd = f64::from_bits(data)
                    }
                    "DepositFee" => fund_info.deposit_schedule.fee = f64::from_bits(data),
                    "WithdrawalStartTime" => {
                        fund_info.withdrawal_schedule.start_time = data as UnixTimestamp
                    }
                    "WithdrawalEndTime" => {
                        fund_info.withdrawal_schedule.end_time = data as UnixTimestamp
                    }
                    "WithdrawalApprovalRequired" => {
                        fund_info.withdrawal_schedule.approval_required = data != 0
                    }
                    "WithdrawalMinAmountUsd" => {
                        fund_info.withdrawal_schedule.min_amount_usd = f64::from_bits(data)
                    }
                    "WithdrawalMaxAmountUsd" => {
                        fund_info.withdrawal_schedule.max_amount_usd = f64::from_bits(data)
                    }
                    "WithdrawalFee" => fund_info.withdrawal_schedule.fee = f64::from_bits(data),
                    "AssetsLimitUsd" => {
                        fund_info.assets_config.assets_limit_usd = f64::from_bits(data)
                    }
                    "AssetsMaxUpdateAgeSec" => fund_info.assets_config.max_update_age_sec = data,
                    "AssetsMaxPriceError" => {
                        fund_info.assets_config.max_price_error = f64::from_bits(data)
                    }
                    "AssetsMaxPriceAgeSec" => fund_info.assets_config.max_price_age_sec = data,
                    "IssueVirtualTokens" => fund_info.assets_config.issue_virtual_tokens = data > 0,
                    "VirtualTokensSupply" => fund_info.virtual_tokens_supply = data,
                    "AmountInvestedUsd" => fund_info.amount_invested_usd = f64::from_bits(data),
                    "AmountRemovedUsd" => fund_info.amount_removed_usd = f64::from_bits(data),
                    "CurrentAssetsUsd" => fund_info.current_assets_usd = f64::from_bits(data),
                    "AssetsUpdateTime" => fund_info.assets_update_time = data as UnixTimestamp,
                    "AdminActionTime" => fund_info.admin_action_time = data as UnixTimestamp,
                    "LastTradeTime" => fund_info.last_trade_time = data as UnixTimestamp,
                    "LiquidationStartTime" => {
                        fund_info.liquidation_start_time = data as UnixTimestamp
                    }
                    "LiquidationAmountUsd" => {
                        fund_info.liquidation_amount_usd = f64::from_bits(data)
                    }
                    "LiquidationAmountTokens" => fund_info.liquidation_amount_tokens = data,
                    _ => {}
                }
            }
        }

        Ok(fund_info)
    }

    /// Returns Fund info and config for all Funds
    pub fn get_all_fund_infos(&self) -> Result<Vec<FundInfo>, FarmClientError> {
        let mut fund_infos = vec![];
        let funds = self.get_funds()?;
        for fund in funds.keys() {
            fund_infos.push(self.get_fund_info(fund)?);
        }

        Ok(fund_infos)
    }

    /// Returns the account address where Fund assets info is stored
    pub fn get_fund_assets_account(
        &self,
        fund_name: &str,
        asset_type: FundAssetType,
    ) -> Result<Pubkey, FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        match asset_type {
            FundAssetType::Vault => Ok(Pubkey::find_program_address(
                &[b"vaults_assets_info", fund.name.as_bytes()],
                &fund.fund_program_id,
            )
            .0),
            FundAssetType::Custody => Ok(Pubkey::find_program_address(
                &[b"custodies_assets_info", fund.name.as_bytes()],
                &fund.fund_program_id,
            )
            .0),
        }
    }

    /// Returns the Fund assets info
    pub fn get_fund_assets(
        &self,
        fund_name: &str,
        asset_type: FundAssetType,
    ) -> Result<FundAssets, FarmClientError> {
        let assets_account = self.get_fund_assets_account(fund_name, asset_type)?;
        let data = self.rpc_client.get_account_data(&assets_account)?;
        FundAssets::unpack(data.as_slice()).map_err(|e| e.into())
    }

    /// Returns the token account address for the Fund assets custody
    pub fn get_fund_custody_token_account(
        &self,
        fund_name: &str,
        token_name: &str,
        custody_type: FundCustodyType,
    ) -> Result<Pubkey, FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        let token = self.get_token(token_name)?;

        if matches!(custody_type, FundCustodyType::DepositWithdraw) {
            Ok(Pubkey::find_program_address(
                &[
                    b"fund_wd_custody_account",
                    token.name.as_bytes(),
                    fund.name.as_bytes(),
                ],
                &fund.fund_program_id,
            )
            .0)
        } else {
            self.get_associated_token_address(&fund.fund_authority, token_name)
        }
    }

    /// Returns the token account address for the Fund fees custody
    pub fn get_fund_custody_fees_token_account(
        &self,
        fund_name: &str,
        token_name: &str,
        custody_type: FundCustodyType,
    ) -> Result<Pubkey, FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        let token = self.get_token(token_name)?;
        let custody_seed_str: &[u8] = match custody_type {
            FundCustodyType::DepositWithdraw => b"fund_wd_custody_fees_account",
            FundCustodyType::Trading => b"fund_td_custody_fees_account",
        };
        Ok(Pubkey::find_program_address(
            &[
                custody_seed_str,
                token.name.as_bytes(),
                fund.name.as_bytes(),
            ],
            &fund.fund_program_id,
        )
        .0)
    }

    /// Returns the account address where Fund custody info is stored
    pub fn get_fund_custody_account(
        &self,
        fund_name: &str,
        token_name: &str,
        custody_type: FundCustodyType,
    ) -> Result<Pubkey, FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        let token = self.get_token(token_name)?;
        let custody_seed_str: &[u8] = match custody_type {
            FundCustodyType::DepositWithdraw => b"fund_wd_custody_info",
            FundCustodyType::Trading => b"fund_td_custody_info",
        };
        Ok(Pubkey::find_program_address(
            &[
                custody_seed_str,
                token.name.as_bytes(),
                fund.name.as_bytes(),
            ],
            &fund.fund_program_id,
        )
        .0)
    }

    /// Returns the Fund custody info
    pub fn get_fund_custody(
        &self,
        fund_name: &str,
        token_name: &str,
        custody_type: FundCustodyType,
    ) -> Result<FundCustody, FarmClientError> {
        let custody_info_account =
            self.get_fund_custody_account(fund_name, token_name, custody_type)?;
        let data = self.rpc_client.get_account_data(&custody_info_account)?;
        FundCustody::unpack(data.as_slice()).map_err(|e| e.into())
    }

    /// Returns all custodies belonging to the Fund sorted by custody_id
    pub fn get_fund_custodies(&self, fund_name: &str) -> Result<Vec<FundCustody>, FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        // search for custody accounts
        let bytes = [&DISCRIMINATOR_FUND_CUSTODY.to_le_bytes(), fund_ref.as_ref()]
            .concat()
            .to_vec();
        let acc_vec = self.get_accounts_with_filter(&fund.fund_program_id, 0, bytes)?;

        let mut res = vec![];
        for (_, acc) in &acc_vec {
            res.push(FundCustody::unpack(acc.data.as_slice())?);
        }

        res.sort_by_key(|k| k.custody_id);
        Ok(res)
    }

    /// Returns the Fund custody extended info
    pub fn get_fund_custody_with_balance(
        &self,
        fund_name: &str,
        token_name: &str,
        custody_type: FundCustodyType,
    ) -> Result<FundCustodyWithBalance, FarmClientError> {
        let custody = self.get_fund_custody(fund_name, token_name, custody_type)?;
        let token = self.get_token_by_ref(&custody.token_ref)?;
        let fund = self.get_fund_by_ref(&custody.fund_ref)?;
        let balance = self.get_token_account_balance_with_address(&custody.address)?;
        let fees_balance = self.get_token_account_balance_with_address(&custody.fees_address)?;
        Ok(FundCustodyWithBalance {
            fund_name: fund.name,
            token_name: token.name,
            balance,
            fees_balance,
            discriminator: custody.discriminator,
            fund_ref: custody.fund_ref,
            custody_id: custody.custody_id,
            custody_type: custody.custody_type,
            token_ref: custody.token_ref,
            address: custody.address,
            fees_address: custody.fees_address,
            bump: custody.bump,
        })
    }

    /// Returns all custodies belonging to the Fund with extended info
    pub fn get_fund_custodies_with_balance(
        &self,
        fund_name: &str,
    ) -> Result<Vec<FundCustodyWithBalance>, FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        let custodies = self.get_fund_custodies(fund_name)?;
        let mut res = vec![];
        for custody in &custodies {
            let token = self.get_token_by_ref(&custody.token_ref)?;
            let balance = self.get_token_account_balance_with_address(&custody.address)?;
            let fees_balance =
                self.get_token_account_balance_with_address(&custody.fees_address)?;
            res.push(FundCustodyWithBalance {
                fund_name: fund.name,
                token_name: token.name,
                balance,
                fees_balance,
                discriminator: custody.discriminator,
                fund_ref: custody.fund_ref,
                custody_id: custody.custody_id,
                custody_type: custody.custody_type,
                token_ref: custody.token_ref,
                address: custody.address,
                fees_address: custody.fees_address,
                bump: custody.bump,
            });
        }
        Ok(res)
    }

    /// Adds a new custody to the Fund
    pub fn add_fund_custody(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
        token_name: &str,
        custody_type: FundCustodyType,
    ) -> Result<Signature, FarmClientError> {
        // create and send the instruction
        let inst = self.new_instruction_add_fund_custody(
            &admin_signer.pubkey(),
            fund_name,
            token_name,
            custody_type,
        )?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Removes the custody from the Fund
    pub fn remove_fund_custody(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
        token_name: &str,
        custody_type: FundCustodyType,
    ) -> Result<Signature, FarmClientError> {
        // create and send the instruction
        let inst = self.new_instruction_remove_fund_custody(
            &admin_signer.pubkey(),
            fund_name,
            token_name,
            custody_type,
        )?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Returns the account address where Fund Vault info is stored
    pub fn get_fund_vault_account(
        &self,
        fund_name: &str,
        vault_name: &str,
        vault_type: FundVaultType,
    ) -> Result<Pubkey, FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        let vault_seed_str: &[u8] = match vault_type {
            FundVaultType::Vault => b"fund_vault_info",
            FundVaultType::Pool => b"fund_pool_info",
            FundVaultType::Farm => b"fund_farm_info",
        };
        Ok(Pubkey::find_program_address(
            &[vault_seed_str, vault_name.as_bytes(), fund_name.as_bytes()],
            &fund.fund_program_id,
        )
        .0)
    }

    /// Returns the Fund Vault info
    pub fn get_fund_vault(
        &self,
        fund_name: &str,
        vault_name: &str,
        vault_type: FundVaultType,
    ) -> Result<FundVault, FarmClientError> {
        let vault_info_account = self.get_fund_vault_account(fund_name, vault_name, vault_type)?;
        let data = self.rpc_client.get_account_data(&vault_info_account)?;
        FundVault::unpack(data.as_slice()).map_err(|e| e.into())
    }

    /// Returns all Vaults belonging to the Fund sorted by vault_id
    pub fn get_fund_vaults(&self, fund_name: &str) -> Result<Vec<FundVault>, FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        // search for custody accounts
        let bytes = [&DISCRIMINATOR_FUND_VAULT.to_le_bytes(), fund_ref.as_ref()]
            .concat()
            .to_vec();
        let acc_vec = self.get_accounts_with_filter(&fund.fund_program_id, 0, bytes)?;

        let mut res = vec![];
        for (_, acc) in &acc_vec {
            res.push(FundVault::unpack(acc.data.as_slice())?);
        }

        res.sort_by_key(|k| k.vault_id);
        Ok(res)
    }

    /// Adds a new Vault to the Fund
    pub fn add_fund_vault(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
        vault_name: &str,
        vault_type: FundVaultType,
    ) -> Result<Signature, FarmClientError> {
        // create and send the instruction
        let inst = self.new_instruction_add_fund_vault(
            &admin_signer.pubkey(),
            fund_name,
            vault_name,
            vault_type,
        )?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Removes the Vault from the Fund
    pub fn remove_fund_vault(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
        vault_name: &str,
        vault_type: FundVaultType,
    ) -> Result<Signature, FarmClientError> {
        // create and send the instruction
        let inst = self.new_instruction_remove_fund_vault(
            &admin_signer.pubkey(),
            fund_name,
            vault_name,
            vault_type,
        )?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Initializes a Fund
    pub fn init_fund(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
        step: u64,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_init_fund(&admin_signer.pubkey(), fund_name, step)?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Initializes a new User for the Fund
    pub fn user_init_fund(
        &self,
        signer: &dyn Signer,
        fund_name: &str,
        token_name: &str,
    ) -> Result<Signature, FarmClientError> {
        // create and send the instruction
        let inst = self.new_instruction_user_init_fund(&signer.pubkey(), fund_name, token_name)?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Sets a new assets tracking config for the Fund
    pub fn set_fund_assets_tracking_config(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
        config: &FundAssetsTrackingConfig,
    ) -> Result<Signature, FarmClientError> {
        // create and send the instruction
        let inst = self.new_instruction_set_fund_assets_tracking_config(
            &admin_signer.pubkey(),
            fund_name,
            config,
        )?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Sets a new deposit schedule for the Fund
    pub fn set_fund_deposit_schedule(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
        schedule: &FundSchedule,
    ) -> Result<Signature, FarmClientError> {
        // create and send the instruction
        let inst = self.new_instruction_set_fund_deposit_schedule(
            &admin_signer.pubkey(),
            fund_name,
            schedule,
        )?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Disables deposits to the Fund.
    /// Same outcome can be achieved with set_fund_deposit_schedule(),
    /// disable_deposits_fund() function is just more explicit.
    pub fn disable_deposits_fund(
        &self,
        signer: &dyn Signer,
        fund_name: &str,
    ) -> Result<Signature, FarmClientError> {
        // create and send the instruction
        let inst = self.new_instruction_disable_deposits_fund(&signer.pubkey(), fund_name)?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Requests a new deposit to the Fund
    pub fn request_deposit_fund(
        &self,
        signer: &dyn Signer,
        fund_name: &str,
        token_name: &str,
        ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.all_instructions_request_deposit_fund(
            &signer.pubkey(),
            fund_name,
            token_name,
            ui_amount,
        )?;
        self.sign_and_send_instructions(&[signer], &inst)
    }

    /// Cancels pending deposit to the Fund
    pub fn cancel_deposit_fund(
        &self,
        signer: &dyn Signer,
        fund_name: &str,
        token_name: &str,
    ) -> Result<Signature, FarmClientError> {
        // create and send the instruction
        let inst =
            self.new_instruction_cancel_deposit_fund(&signer.pubkey(), fund_name, token_name)?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Approves pending deposit to the Fund
    pub fn approve_deposit_fund(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
        user_address: &Pubkey,
        token_name: &str,
        ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        // create and send the instruction
        let inst = self.new_instruction_approve_deposit_fund(
            &admin_signer.pubkey(),
            user_address,
            fund_name,
            token_name,
            ui_amount,
        )?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Denies pending deposit to the Fund
    pub fn deny_deposit_fund(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
        user_address: &Pubkey,
        token_name: &str,
        deny_reason: &str,
    ) -> Result<Signature, FarmClientError> {
        // create and send the instruction
        let inst = self.new_instruction_deny_deposit_fund(
            &admin_signer.pubkey(),
            user_address,
            fund_name,
            token_name,
            deny_reason,
        )?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Sets a new withdrawal schedule for the Fund
    pub fn set_fund_withdrawal_schedule(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
        schedule: &FundSchedule,
    ) -> Result<Signature, FarmClientError> {
        // create and send the instruction
        let inst = self.new_instruction_set_fund_withdrawal_schedule(
            &admin_signer.pubkey(),
            fund_name,
            schedule,
        )?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Disables withdrawals from the Fund.
    /// Same outcome can be achieved with set_fund_withdrawal_schedule(),
    /// disable_withdrawals_fund() function is just more explicit.
    pub fn disable_withdrawals_fund(
        &self,
        signer: &dyn Signer,
        fund_name: &str,
    ) -> Result<Signature, FarmClientError> {
        // create and send the instruction
        let inst = self.new_instruction_disable_withdrawals_fund(&signer.pubkey(), fund_name)?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Requests a new withdrawal from the Fund
    pub fn request_withdrawal_fund(
        &self,
        signer: &dyn Signer,
        fund_name: &str,
        token_name: &str,
        ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.all_instructions_request_withdrawal_fund(
            &signer.pubkey(),
            fund_name,
            token_name,
            ui_amount,
        )?;
        self.sign_and_send_instructions(&[signer], &inst)
    }

    /// Cancels pending withdrawal from the Fund
    pub fn cancel_withdrawal_fund(
        &self,
        signer: &dyn Signer,
        fund_name: &str,
        token_name: &str,
    ) -> Result<Signature, FarmClientError> {
        // create and send the instruction
        let inst =
            self.new_instruction_cancel_withdrawal_fund(&signer.pubkey(), fund_name, token_name)?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Approves pending withdrawal from the Fund
    pub fn approve_withdrawal_fund(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
        user_address: &Pubkey,
        token_name: &str,
        ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        // create and send the instruction
        let inst = self.new_instruction_approve_withdrawal_fund(
            &admin_signer.pubkey(),
            user_address,
            fund_name,
            token_name,
            ui_amount,
        )?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Denies pending withdrawal from the Fund
    pub fn deny_withdrawal_fund(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
        user_address: &Pubkey,
        token_name: &str,
        deny_reason: &str,
    ) -> Result<Signature, FarmClientError> {
        // create and send the instruction
        let inst = self.new_instruction_deny_withdrawal_fund(
            &admin_signer.pubkey(),
            user_address,
            fund_name,
            token_name,
            deny_reason,
        )?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Moves deposited assets from Deposit/Withdraw custody to the Fund
    pub fn lock_assets_fund(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
        token_name: &str,
        ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        // create and send the instruction
        let inst = self.new_instruction_lock_assets_fund(
            &admin_signer.pubkey(),
            fund_name,
            token_name,
            ui_amount,
        )?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Releases assets from the Fund to Deposit/Withdraw custody
    pub fn unlock_assets_fund(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
        token_name: &str,
        ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        // create and send the instruction
        let inst = self.new_instruction_unlock_assets_fund(
            &admin_signer.pubkey(),
            fund_name,
            token_name,
            ui_amount,
        )?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Updates Fund assets info based on custody holdings
    pub fn update_fund_assets_with_custody(
        &self,
        signer: &dyn Signer,
        fund_name: &str,
        custody_id: u32,
    ) -> Result<Signature, FarmClientError> {
        // create and send the instruction
        let inst = self.new_instruction_update_fund_assets_with_custody(
            &signer.pubkey(),
            fund_name,
            custody_id,
        )?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Updates Fund assets info based on all custodies
    pub fn update_fund_assets_with_custodies(
        &self,
        signer: &dyn Signer,
        fund_name: &str,
    ) -> Result<usize, FarmClientError> {
        let custodies = self.get_fund_custodies(fund_name)?;
        for custody in &custodies {
            if !custody.is_vault_token {
                self.update_fund_assets_with_custody(signer, fund_name, custody.custody_id)?;
            }
        }
        Ok(custodies.len())
    }

    /// Updates Fund assets info based on Vault holdings
    pub fn update_fund_assets_with_vault(
        &self,
        signer: &dyn Signer,
        fund_name: &str,
        vault_id: u32,
    ) -> Result<Signature, FarmClientError> {
        // create and send the instruction
        let inst = self.new_instruction_update_fund_assets_with_vault(
            &signer.pubkey(),
            fund_name,
            vault_id,
        )?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Updates Fund assets info based on all Vaults
    pub fn update_fund_assets_with_vaults(
        &self,
        signer: &dyn Signer,
        fund_name: &str,
    ) -> Result<usize, FarmClientError> {
        let vaults = self.get_fund_vaults(fund_name)?;
        for vault in &vaults {
            if vault.vault_type != FundVaultType::Farm {
                self.update_fund_assets_with_vault(signer, fund_name, vault.vault_id)?;
            }
        }
        Ok(vaults.len())
    }

    /// Starts the Fund liquidation
    pub fn start_liquidation_fund(
        &self,
        signer: &dyn Signer,
        fund_name: &str,
    ) -> Result<Signature, FarmClientError> {
        // create and send the instruction
        let inst = self.new_instruction_start_liquidation_fund(&signer.pubkey(), fund_name)?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Stops the Fund liquidation
    pub fn stop_liquidation_fund(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
    ) -> Result<Signature, FarmClientError> {
        // create and send the instruction
        let inst = self.new_instruction_stop_liquidation_fund(&admin_signer.pubkey(), fund_name)?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Withdraw collected fees from the Fund
    pub fn withdraw_fees_fund(
        &self,
        signer: &dyn Signer,
        fund_name: &str,
        token_name: &str,
        custody_type: FundCustodyType,
        ui_amount: f64,
        receiver: &Pubkey,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_withdraw_fees_fund(
            &signer.pubkey(),
            fund_name,
            token_name,
            custody_type,
            ui_amount,
            receiver,
        )?;
        self.sign_and_send_instructions(&[signer], &[inst])
    }

    /// Adds liquidity to the Pool in the Fund
    pub fn fund_add_liquidity_pool(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
        pool_name: &str,
        max_token_a_ui_amount: f64,
        max_token_b_ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.all_instructions_fund_add_liquidity_pool(
            &admin_signer.pubkey(),
            fund_name,
            pool_name,
            max_token_a_ui_amount,
            max_token_b_ui_amount,
        )?;
        Ok(*self
            .sign_and_send_instructions_in_batches(&[admin_signer], &inst)?
            .last()
            .unwrap())
    }

    /// Removes liquidity from the Pool in the Fund
    pub fn fund_remove_liquidity_pool(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
        pool_name: &str,
        ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.all_instructions_fund_remove_liquidity_pool(
            &admin_signer.pubkey(),
            fund_name,
            pool_name,
            ui_amount,
        )?;
        Ok(*self
            .sign_and_send_instructions_in_batches(&[admin_signer], &inst)?
            .last()
            .unwrap())
    }

    /// Swaps tokens in the Fund
    #[allow(clippy::too_many_arguments)]
    pub fn fund_swap(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
        protocol: Protocol,
        from_token: &str,
        to_token: &str,
        ui_amount_in: f64,
        min_ui_amount_out: f64,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.all_instructions_fund_swap(
            &admin_signer.pubkey(),
            fund_name,
            protocol,
            from_token,
            to_token,
            ui_amount_in,
            min_ui_amount_out,
        )?;
        Ok(*self
            .sign_and_send_instructions_in_batches(&[admin_signer], &inst)?
            .last()
            .unwrap())
    }

    /// Initializes a new User for the Farm in the Fund
    pub fn fund_user_init_farm(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
        farm_name: &str,
    ) -> Result<Signature, FarmClientError> {
        // create and send the instruction
        let inst =
            self.new_instruction_fund_user_init_farm(&admin_signer.pubkey(), fund_name, farm_name)?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Stakes tokens to the Farm in the Fund
    pub fn fund_stake(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
        farm_name: &str,
        ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.all_instructions_fund_stake(
            &admin_signer.pubkey(),
            fund_name,
            farm_name,
            ui_amount,
        )?;
        Ok(*self
            .sign_and_send_instructions_in_batches(&[admin_signer], &inst)?
            .last()
            .unwrap())
    }

    /// Unstakes tokens from the Farm in the Fund
    pub fn fund_unstake(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
        farm_name: &str,
        ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.all_instructions_fund_unstake(
            &admin_signer.pubkey(),
            fund_name,
            farm_name,
            ui_amount,
        )?;
        Ok(*self
            .sign_and_send_instructions_in_batches(&[admin_signer], &inst)?
            .last()
            .unwrap())
    }

    /// Harvests rewards from the Farm in the Fund
    pub fn fund_harvest(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
        farm_name: &str,
    ) -> Result<Signature, FarmClientError> {
        let inst =
            self.all_instructions_fund_harvest(&admin_signer.pubkey(), fund_name, farm_name)?;
        Ok(*self
            .sign_and_send_instructions_in_batches(&[admin_signer], &inst)?
            .last()
            .unwrap())
    }

    /// Initializes a new User for the Vault in the Fund
    pub fn fund_user_init_vault(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
        vault_name: &str,
    ) -> Result<Signature, FarmClientError> {
        // create and send the instruction
        let inst = self.new_instruction_fund_user_init_vault(
            &admin_signer.pubkey(),
            fund_name,
            vault_name,
        )?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Adds liquidity to the Vault in the Fund
    pub fn fund_add_liquidity_vault(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
        vault_name: &str,
        max_token_a_ui_amount: f64,
        max_token_b_ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.all_instructions_fund_add_liquidity_vault(
            &admin_signer.pubkey(),
            fund_name,
            vault_name,
            max_token_a_ui_amount,
            max_token_b_ui_amount,
        )?;
        Ok(*self
            .sign_and_send_instructions_in_batches(&[admin_signer], &inst)?
            .last()
            .unwrap())
    }

    /// Adds locked liquidity to the Vault in the Fund.
    /// Useful if add liquidity operation partially failed.
    pub fn fund_add_locked_liquidity_vault(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
        vault_name: &str,
        ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.all_instructions_fund_add_locked_liquidity_vault(
            &admin_signer.pubkey(),
            fund_name,
            vault_name,
            ui_amount,
        )?;
        Ok(*self
            .sign_and_send_instructions_in_batches(&[admin_signer], &inst)?
            .last()
            .unwrap())
    }

    /// Removes liquidity from the Vault in the Fund
    pub fn fund_remove_liquidity_vault(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
        vault_name: &str,
        ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.all_instructions_fund_remove_liquidity_vault(
            &admin_signer.pubkey(),
            fund_name,
            vault_name,
            ui_amount,
        )?;
        Ok(*self
            .sign_and_send_instructions_in_batches(&[admin_signer], &inst)?
            .last()
            .unwrap())
    }

    /// Removes unlocked liquidity from the Vault in the Fund.
    /// Useful if remove liquidity operation failed after unlock step.
    pub fn fund_remove_unlocked_liquidity_vault(
        &self,
        admin_signer: &dyn Signer,
        fund_name: &str,
        vault_name: &str,
        ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.all_instructions_fund_remove_unlocked_liquidity_vault(
            &admin_signer.pubkey(),
            fund_name,
            vault_name,
            ui_amount,
        )?;
        Ok(*self
            .sign_and_send_instructions_in_batches(&[admin_signer], &inst)?
            .last()
            .unwrap())
    }

    /// Returns oracle type and address for the given token
    pub fn get_oracle(
        &self,
        symbol: &str,
    ) -> Result<(OracleType, Option<Pubkey>), FarmClientError> {
        let token = self.get_token(symbol)?;
        Ok((token.oracle_type, token.oracle_account))
    }

    /// Returns the price in USD for the given token
    pub fn get_oracle_price(
        &self,
        symbol: &str,
        max_price_age_sec: u64,
        max_price_error: f64,
    ) -> Result<f64, FarmClientError> {
        let (oracle_type, oracle_account) = self.get_oracle(symbol)?;
        if oracle_type == OracleType::Unsupported {
            return Err(FarmClientError::ValueError(format!(
                "Oracle for {} is not configured",
                symbol
            )));
        } else if oracle_type != OracleType::Pyth {
            return Err(FarmClientError::ValueError(
                "Unsupported oracle type".to_string(),
            ));
        }
        let pyth_price_data = self
            .rpc_client
            .get_account_data(&oracle_account.ok_or(ProgramError::UninitializedAccount)?)?;
        let pyth_price = pyth_client::load_price(pyth_price_data.as_slice())?;

        if !matches!(pyth_price.agg.status, PriceStatus::Trading)
            || !matches!(pyth_price.ptype, PriceType::Price)
        {
            return Err(FarmClientError::ValueError(
                "Error: Pyth oracle price has invalid state".to_string(),
            ));
        }

        if max_price_age_sec > 0 {
            let current_slot = self.rpc_client.get_slot()?;
            let last_update_age_sec = if current_slot > pyth_price.valid_slot {
                (current_slot - pyth_price.valid_slot) * solana_sdk::clock::DEFAULT_MS_PER_SLOT
                    / 1000
            } else {
                0
            };
            if last_update_age_sec > max_price_age_sec {
                return Err(FarmClientError::ValueError(
                    "Error: Pyth oracle price is stale".to_string(),
                ));
            }
        }

        if pyth_price.agg.price <= 0
            || (max_price_error > 0.0
                && pyth_price.agg.conf as f64 / pyth_price.agg.price as f64 > max_price_error)
        {
            return Err(FarmClientError::ValueError(
                "Error: Pyth oracle price is out of bounds".to_string(),
            ));
        }

        Ok(pyth_price.agg.price as f64 * math::checked_powi(10.0, pyth_price.expo)?)
    }

    /// Returns description and stats of all supported protocols
    pub fn get_protocols(&self) -> Result<Vec<ProtocolInfo>, FarmClientError> {
        let (raydium_pools, raydium_farms, raydium_vaults) =
            self.get_protocol_stats(Protocol::Raydium)?;
        let (saber_pools, saber_farms, saber_vaults) = self.get_protocol_stats(Protocol::Saber)?;
        let (orca_pools, orca_farms, orca_vaults) = self.get_protocol_stats(Protocol::Orca)?;
        Ok(vec![
            ProtocolInfo {
                protocol: Protocol::Raydium,
                description: "Raydium protocol".to_string(),
                link: "www.raydium.io".to_string(),
                pools: raydium_pools,
                farms: raydium_farms,
                vaults: raydium_vaults,
            },
            ProtocolInfo {
                protocol: Protocol::Saber,
                description: "Saber protocol".to_string(),
                link: "www.saber.so".to_string(),
                pools: saber_pools,
                farms: saber_farms,
                vaults: saber_vaults,
            },
            ProtocolInfo {
                protocol: Protocol::Orca,
                description: "Orca protocol".to_string(),
                link: "www.orca.so".to_string(),
                pools: orca_pools,
                farms: orca_farms,
                vaults: orca_vaults,
            },
        ])
    }

    /////////////// helpers
    pub fn ui_amount_to_tokens(
        &self,
        ui_amount: f64,
        token_name: &str,
    ) -> Result<u64, FarmClientError> {
        if ui_amount == 0.0 {
            Ok(0)
        } else if ui_amount < 0.0 {
            Err(FarmClientError::ValueError(format!(
                "Invalid ui_amount: {}",
                ui_amount
            )))
        } else {
            let multiplier =
                math::checked_pow(10u64, self.get_token(token_name)?.decimals as usize)?;
            Ok(math::checked_as_u64(
                (ui_amount * multiplier as f64).round(),
            )?)
        }
    }

    pub fn tokens_to_ui_amount(
        &self,
        amount: u64,
        token_name: &str,
    ) -> Result<f64, FarmClientError> {
        if amount == 0 {
            return Ok(0.0);
        }
        let divisor = math::checked_pow(10u64, self.get_token(token_name)?.decimals as usize)?;
        Ok(amount as f64 / divisor as f64)
    }

    pub fn ui_amount_to_tokens_with_decimals(
        &self,
        ui_amount: f64,
        decimals: u8,
    ) -> Result<u64, FarmClientError> {
        if ui_amount <= 0.0 {
            return Ok(0);
        }
        let multiplier = math::checked_pow(10u64, decimals as usize)?;
        Ok(math::checked_as_u64(
            (ui_amount * multiplier as f64).round(),
        )?)
    }

    pub fn tokens_to_ui_amount_with_decimals(&self, amount: u64, decimals: u8) -> f64 {
        if amount == 0 {
            return 0.0;
        }
        let divisor = math::checked_pow(10u64, decimals as usize).unwrap();
        amount as f64 / divisor as f64
    }

    pub fn pool_has_sol_tokens(&self, pool_name: &str) -> Result<(bool, bool), FarmClientError> {
        let pool = self.get_pool(pool_name)?;
        let mut is_token_a_sol = false;
        let mut is_token_b_sol = false;
        if let Some(token_a_ref) = pool.token_a_ref {
            let token_a = self.get_token_by_ref(&token_a_ref)?;
            if token_a.token_type == TokenType::WrappedSol {
                is_token_a_sol = true;
            }
        }
        if let Some(token_b_ref) = pool.token_b_ref {
            let token_b = self.get_token_by_ref(&token_b_ref)?;
            if token_b.token_type == TokenType::WrappedSol {
                is_token_b_sol = true;
            }
        }
        Ok((is_token_a_sol, is_token_b_sol))
    }

    pub fn pool_has_saber_wrapped_tokens(
        &self,
        pool_name: &str,
    ) -> Result<(bool, bool), FarmClientError> {
        let pool = self.get_pool(pool_name)?;

        match pool.route {
            PoolRoute::Saber {
                wrapped_token_a_ref,
                wrapped_token_b_ref,
                ..
            } => Ok((wrapped_token_a_ref.is_some(), wrapped_token_b_ref.is_some())),
            _ => Ok((false, false)),
        }
    }

    pub fn vault_has_sol_tokens(&self, vault_name: &str) -> Result<(bool, bool), FarmClientError> {
        let pool_name = self.get_underlying_pool(vault_name)?.name.to_string();
        self.pool_has_sol_tokens(&pool_name)
    }

    pub fn get_protocol(vault_or_pool_name: &str) -> Result<Protocol, FarmClientError> {
        let protocol_str =
            &vault_or_pool_name[..vault_or_pool_name.find('.').ok_or_else(|| {
                FarmClientError::ValueError(format!(
                    "Invalid vault or pool name: {}",
                    vault_or_pool_name
                ))
            })?];
        Ok(match protocol_str {
            "RDM" => Protocol::Raydium,
            "SBR" => Protocol::Saber,
            "ORC" => Protocol::Orca,
            _ => {
                return Err(FarmClientError::ValueError(format!(
                    "Unrecognized protocol: {}",
                    protocol_str
                )))
            }
        })
    }

    pub fn get_pool_token_names(
        &self,
        pool_name: &str,
    ) -> Result<(String, String, String), FarmClientError> {
        let pool = self.get_pool(pool_name)?;
        let token_a = self.get_token_by_ref_from_cache(&pool.token_a_ref)?;
        let token_b = self.get_token_by_ref_from_cache(&pool.token_b_ref)?;
        let lp_token = self.get_token_by_ref_from_cache(&pool.lp_token_ref)?;
        Ok((
            if let Some(token) = token_a {
                token.name.to_string()
            } else {
                String::default()
            },
            if let Some(token) = token_b {
                token.name.to_string()
            } else {
                String::default()
            },
            if let Some(token) = lp_token {
                token.name.to_string()
            } else {
                String::default()
            },
        ))
    }

    pub fn get_farm_token_names(
        &self,
        farm_name: &str,
    ) -> Result<(String, String, String), FarmClientError> {
        let farm = self.get_farm(farm_name)?;
        let token_a = self.get_token_by_ref_from_cache(&farm.first_reward_token_ref)?;
        let token_b = self.get_token_by_ref_from_cache(&farm.second_reward_token_ref)?;
        let lp_token = self.get_token_by_ref_from_cache(&farm.lp_token_ref)?;
        Ok((
            if let Some(token) = token_a {
                token.name.to_string()
            } else {
                String::default()
            },
            if let Some(token) = token_b {
                token.name.to_string()
            } else {
                String::default()
            },
            if let Some(token) = lp_token {
                token.name.to_string()
            } else {
                String::default()
            },
        ))
    }

    pub fn get_vault_token_names(
        &self,
        vault_name: &str,
    ) -> Result<(String, String, String), FarmClientError> {
        let vault = self.get_vault(vault_name)?;
        let vt_token = self.get_token_by_ref_from_cache(&Some(vault.vault_token_ref))?;
        match vault.strategy {
            VaultStrategy::StakeLpCompoundRewards { pool_ref, .. } => {
                let pool = self.get_pool_by_ref(&pool_ref)?;
                let token_a = self.get_token_by_ref_from_cache(&pool.token_a_ref)?;
                let token_b = self.get_token_by_ref_from_cache(&pool.token_b_ref)?;

                Ok((
                    if let Some(token) = token_a {
                        token.name.to_string()
                    } else {
                        String::default()
                    },
                    if let Some(token) = token_b {
                        token.name.to_string()
                    } else {
                        String::default()
                    },
                    if let Some(token) = vt_token {
                        token.name.to_string()
                    } else {
                        String::default()
                    },
                ))
            }
            _ => {
                unreachable!();
            }
        }
    }

    pub fn unwrap_pool_tokens(
        &self,
        signer: &dyn Signer,
        pool_name: &str,
    ) -> Result<Signature, FarmClientError> {
        let mut inst = Vec::<Instruction>::new();

        let (is_token_a_sol, is_token_b_sol) = self.pool_has_sol_tokens(pool_name)?;
        let (is_token_a_wrapped, is_token_b_wrapped) =
            self.pool_has_saber_wrapped_tokens(pool_name)?;

        if is_token_a_wrapped {
            inst.push(self.new_instruction_unwrap_token(
                &signer.pubkey(),
                pool_name,
                TokenSelector::TokenA,
                0.0,
            )?);
        }
        if is_token_b_wrapped {
            inst.push(self.new_instruction_unwrap_token(
                &signer.pubkey(),
                pool_name,
                TokenSelector::TokenB,
                0.0,
            )?);
        }
        if is_token_a_sol || is_token_b_sol {
            inst.push(self.new_instruction_close_token_account(&signer.pubkey(), "SOL")?);
        }

        self.sign_and_send_instructions(&[signer], &inst)
    }

    pub fn get_accounts_with_filter(
        &self,
        program: &Pubkey,
        offset: usize,
        bytes: Vec<u8>,
    ) -> Result<Vec<(Pubkey, Account)>, FarmClientError> {
        let filters = Some(vec![rpc_filter::RpcFilterType::Memcmp(
            rpc_filter::Memcmp {
                offset,
                bytes: rpc_filter::MemcmpEncodedBytes::Base58(bs58::encode(bytes).into_string()),
                encoding: Some(rpc_filter::MemcmpEncoding::Binary),
            },
        )]);
        Ok(self.rpc_client.get_program_accounts_with_config(
            program,
            RpcProgramAccountsConfig {
                filters,
                account_config: RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    ..RpcAccountInfoConfig::default()
                },
                ..RpcProgramAccountsConfig::default()
            },
        )?)
    }

    /// Checks if associated token account owner matches base wallet owner
    pub fn check_ata_owner(
        &self,
        base_wallet: &Pubkey,
        token_name: &str,
    ) -> Result<bool, FarmClientError> {
        let token_account = self
            .rpc_client
            .get_account(&self.get_associated_token_address(base_wallet, token_name)?)?;
        if token_account.owner != spl_token::id() {
            return Ok(false);
        }
        let token_data = self.get_token_account_data(base_wallet, token_name)?;
        let base_account = self.rpc_client.get_account(base_wallet)?;
        Ok(base_account.owner == FarmClient::pubkey_from_str(token_data.owner.as_str())?)
    }

    pub fn get_stake_account(
        &self,
        wallet_address: &Pubkey,
        farm_name: &str,
    ) -> Result<Pubkey, FarmClientError> {
        let farm = self.get_farm(farm_name)?;
        match farm.route {
            FarmRoute::Raydium { .. } => self.get_raydium_stake_account(wallet_address, farm_name),
            FarmRoute::Saber { .. } => self.get_saber_stake_account(wallet_address, farm_name),
            FarmRoute::Orca { .. } => self.get_orca_stake_account(wallet_address, farm_name),
        }
    }

    pub fn get_vault_stake_account(&self, vault_name: &str) -> Result<Pubkey, FarmClientError> {
        let vault = self.get_vault(vault_name)?;
        match vault.strategy {
            VaultStrategy::StakeLpCompoundRewards {
                vault_stake_info, ..
            } => Ok(vault_stake_info),
            _ => unreachable!(),
        }
    }

    /// Checks if the given address is the Fund manager
    pub fn is_fund_manager(&self, wallet_address: &Pubkey) -> Result<bool, FarmClientError> {
        Ok(self
            .get_funds()?
            .values()
            .any(|&f| &f.fund_manager == wallet_address))
    }

    /// Extracts version from the full pool name
    pub fn extract_pool_version(name: &str) -> Result<u16, FarmClientError> {
        if name.len() > 3
            && &name[name.len() - 2..name.len() - 1].to_uppercase() == "V"
            && &name[name.len() - 3..name.len() - 2] == "-"
        {
            if let Ok(ver) = name[name.len() - 1..name.len()].parse::<u16>() {
                return Ok(ver);
            }
        }
        Err(FarmClientError::ProgramError(ProgramError::InvalidArgument))
    }

    /// Extracts name and version from the pool or liquidity token name
    pub fn extract_pool_name_and_version(name: &str) -> Result<(String, u16), FarmClientError> {
        if FarmClient::is_liquidity_token(name) {
            if name.len() > 6 {
                return Ok((
                    name[3..name.len() - 3].to_string(),
                    FarmClient::extract_pool_version(name)?,
                ));
            }
        } else if name.len() > 3 {
            return Ok((
                name[..name.len() - 3].to_string(),
                FarmClient::extract_pool_version(name)?,
            ));
        }
        Err(FarmClientError::ProgramError(ProgramError::InvalidArgument))
    }

    /// Checks if token is a liquidity token
    pub fn is_liquidity_token(name: &str) -> bool {
        name.len() > 3 && ["LP.", "VT.", "FD."].contains(&&name[..3])
    }

    /// Extracts individual token names and protocol from the pool or liquidity token name
    pub fn extract_token_names(name: &str) -> Result<(Protocol, String, String), FarmClientError> {
        let dot_split = if FarmClient::is_liquidity_token(name) {
            name[3..].split('.').collect::<Vec<&str>>()
        } else {
            name.split('.').collect::<Vec<&str>>()
        };
        if dot_split.len() < 2 || dot_split[0].is_empty() {
            return Err(FarmClientError::ValueError(format!(
                "Can't extract token names from {}",
                name
            )));
        }
        let dash_split = dot_split.last().unwrap().split('-').collect::<Vec<&str>>();
        if dash_split.is_empty()
            || dash_split[0].is_empty()
            || (dash_split.len() > 1 && dash_split[1].is_empty())
        {
            return Err(FarmClientError::ValueError(format!(
                "Can't extract token names from {}",
                name
            )));
        }
        Ok((
            dot_split[0].parse()?,
            dash_split[0].to_string(),
            if dash_split.len() > 1
                && (FarmClient::extract_pool_version(name).is_err() || dash_split.len() > 2)
            {
                dash_split[1].to_string()
            } else {
                String::default()
            },
        ))
    }

    ////////////// private helpers
    fn pubkey_from_str(input: &str) -> Result<Pubkey, FarmClientError> {
        Pubkey::from_str(input).map_err(|_| {
            FarmClientError::ValueError(format!(
                "Failed to convert the String to a Pubkey {}",
                input
            ))
        })
    }

    fn to_token_amount(&self, ui_amount: f64, token: &Token) -> Result<u64, FarmClientError> {
        self.ui_amount_to_tokens_with_decimals(ui_amount, token.decimals)
    }

    fn to_token_amount_option(
        &self,
        ui_amount: f64,
        token: &Option<Token>,
    ) -> Result<u64, FarmClientError> {
        if let Some(tkn) = token {
            self.to_token_amount(ui_amount, tkn)
        } else {
            Err(ProgramError::UninitializedAccount.into())
        }
    }

    fn load_token_by_ref(&self, token_ref: &Pubkey) -> Result<Token, FarmClientError> {
        let data = self.rpc_client.get_account_data(token_ref)?;
        Ok(Token::unpack(data.as_slice())?)
    }

    fn load_pool_by_ref(&self, pool_ref: &Pubkey) -> Result<Pool, FarmClientError> {
        let data = self.rpc_client.get_account_data(pool_ref)?;
        Ok(Pool::unpack(data.as_slice())?)
    }

    fn load_vault_by_ref(&self, vault_ref: &Pubkey) -> Result<Vault, FarmClientError> {
        let data = self.rpc_client.get_account_data(vault_ref)?;
        Ok(Vault::unpack(data.as_slice())?)
    }

    fn load_farm_by_ref(&self, farm_ref: &Pubkey) -> Result<Farm, FarmClientError> {
        let data = self.rpc_client.get_account_data(farm_ref)?;
        Ok(Farm::unpack(data.as_slice())?)
    }

    fn load_fund_by_ref(&self, fund_ref: &Pubkey) -> Result<Fund, FarmClientError> {
        let data = self.rpc_client.get_account_data(fund_ref)?;
        Ok(Fund::unpack(data.as_slice())?)
    }

    // insert version-stripped names that point to the latest version
    fn reinsert_latest_versions(
        source: &HashMap<String, Pubkey>,
        dest: &mut HashMap<String, String>,
    ) {
        let mut latest = HashMap::<String, (String, u16)>::default();
        for (full_name, _) in source.iter() {
            if let Ok((name_no_ver, ver)) = FarmClient::extract_pool_name_and_version(full_name) {
                if let Some((_, cur_ver)) = latest.get(&name_no_ver) {
                    if *cur_ver < ver {
                        latest.insert(name_no_ver, (full_name.clone(), ver));
                    }
                } else {
                    latest.insert(name_no_ver, (full_name.clone(), ver));
                }
            }
        }
        for (name, (full_name, _)) in latest {
            dest.insert(name, full_name);
        }
    }

    fn reload_fund_refs_if_stale(&self) -> Result<bool, FarmClientError> {
        if self.fund_refs.borrow().is_stale() {
            let (header, fund_refs) =
                self.get_refdb_pubkey_map(&refdb::StorageType::Fund.to_string())?;
            if self.fund_refs.borrow().is_updated(header.counter) {
                self.fund_refs.borrow_mut().set(fund_refs, header.counter);
                self.funds.borrow_mut().reset();
                return Ok(true);
            } else {
                self.fund_refs.borrow_mut().mark_not_stale();
            }
        }
        Ok(false)
    }

    fn reload_funds_if_empty(&self) -> Result<bool, FarmClientError> {
        if self.funds.borrow().is_empty() || self.funds.borrow().is_updated(1) {
            let refs_map = &self.fund_refs.borrow().data;
            let refs: Vec<Pubkey> = refs_map.values().copied().collect();
            if refs.is_empty() {
                return Ok(false);
            }
            let mut fund_map = FundMap::new();

            let mut idx = 0;
            while idx < refs.len() {
                let refs_slice = &refs.as_slice()[idx..std::cmp::min(idx + 100, refs.len())];
                let accounts = self.rpc_client.get_multiple_accounts(refs_slice)?;

                for (account_option, account_ref) in accounts.iter().zip(refs_slice.iter()) {
                    if let Some(account) = account_option {
                        let fund = Fund::unpack(account.data.as_slice())?;
                        fund_map.insert(fund.name.as_str().to_string(), fund);
                    } else {
                        return Err(FarmClientError::RecordNotFound(format!(
                            "Fund with ref {}",
                            account_ref
                        )));
                    }
                }
                idx += 100;
            }

            self.funds.borrow_mut().set(fund_map, 1);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn reload_vault_refs_if_stale(&self) -> Result<bool, FarmClientError> {
        if self.vault_refs.borrow().is_stale() {
            let (header, vault_refs) =
                self.get_refdb_pubkey_map(&refdb::StorageType::Vault.to_string())?;
            if self.vault_refs.borrow().is_updated(header.counter) {
                FarmClient::reinsert_latest_versions(
                    &vault_refs,
                    &mut self.latest_vaults.borrow_mut(),
                );
                self.vault_refs.borrow_mut().set(vault_refs, header.counter);
                self.vaults.borrow_mut().reset();
                return Ok(true);
            } else {
                self.vault_refs.borrow_mut().mark_not_stale();
            }
        }
        Ok(false)
    }

    fn reload_vaults_if_empty(&self) -> Result<bool, FarmClientError> {
        if self.vaults.borrow().is_empty() || self.vaults.borrow().is_updated(1) {
            let refs_map = &self.vault_refs.borrow().data;
            let refs: Vec<Pubkey> = refs_map.values().copied().collect();
            if refs.is_empty() {
                return Ok(false);
            }
            let mut vault_map = VaultMap::new();

            let mut idx = 0;
            while idx < refs.len() {
                let refs_slice = &refs.as_slice()[idx..std::cmp::min(idx + 100, refs.len())];
                let accounts = self.rpc_client.get_multiple_accounts(refs_slice)?;

                for (account_option, account_ref) in accounts.iter().zip(refs_slice.iter()) {
                    if let Some(account) = account_option {
                        let vault = Vault::unpack(account.data.as_slice())?;
                        vault_map.insert(vault.name.as_str().to_string(), vault);
                    } else {
                        return Err(FarmClientError::RecordNotFound(format!(
                            "Vault with ref {}",
                            account_ref
                        )));
                    }
                }
                idx += 100;
            }

            self.vaults.borrow_mut().set(vault_map, 1);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn reload_pool_refs_if_stale(&self) -> Result<bool, FarmClientError> {
        if self.pool_refs.borrow().is_stale() {
            let (header, pool_refs) =
                self.get_refdb_pubkey_map(&refdb::StorageType::Pool.to_string())?;
            if self.pool_refs.borrow().is_updated(header.counter) {
                FarmClient::reinsert_latest_versions(
                    &pool_refs,
                    &mut self.latest_pools.borrow_mut(),
                );
                self.pool_refs.borrow_mut().set(pool_refs, header.counter);
                self.pools.borrow_mut().reset();
                return Ok(true);
            } else {
                self.pool_refs.borrow_mut().mark_not_stale();
            }
        }
        Ok(false)
    }

    fn reload_pools_if_empty(&self) -> Result<bool, FarmClientError> {
        if self.pools.borrow().is_empty() || self.pools.borrow().is_updated(1) {
            let refs_map = &self.pool_refs.borrow().data;
            let refs: Vec<Pubkey> = refs_map.values().copied().collect();
            if refs.is_empty() {
                return Ok(false);
            }
            let mut pool_map = PoolMap::new();

            let mut idx = 0;
            while idx < refs.len() {
                let refs_slice = &refs.as_slice()[idx..std::cmp::min(idx + 100, refs.len())];
                let accounts = self.rpc_client.get_multiple_accounts(refs_slice)?;

                for (account_option, account_ref) in accounts.iter().zip(refs_slice.iter()) {
                    if let Some(account) = account_option {
                        let pool = Pool::unpack(account.data.as_slice())?;
                        pool_map.insert(pool.name.as_str().to_string(), pool);
                    } else {
                        return Err(FarmClientError::RecordNotFound(format!(
                            "Pool with ref {}",
                            account_ref
                        )));
                    }
                }
                idx += 100;
            }

            self.pools.borrow_mut().set(pool_map, 1);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn reload_farm_refs_if_stale(&self) -> Result<bool, FarmClientError> {
        if self.farm_refs.borrow().is_stale() {
            let (header, farm_refs) =
                self.get_refdb_pubkey_map(&refdb::StorageType::Farm.to_string())?;
            if self.farm_refs.borrow().is_updated(header.counter) {
                FarmClient::reinsert_latest_versions(
                    &farm_refs,
                    &mut self.latest_farms.borrow_mut(),
                );
                self.farm_refs.borrow_mut().set(farm_refs, header.counter);
                self.farms.borrow_mut().reset();
                return Ok(true);
            } else {
                self.farm_refs.borrow_mut().mark_not_stale();
            }
        }
        Ok(false)
    }

    fn reload_farms_if_empty(&self) -> Result<bool, FarmClientError> {
        if self.farms.borrow().is_empty() || self.farms.borrow().is_updated(1) {
            let refs_map = &self.farm_refs.borrow().data;
            let refs: Vec<Pubkey> = refs_map.values().copied().collect();
            if refs.is_empty() {
                return Ok(false);
            }
            let mut farm_map = FarmMap::new();

            let mut idx = 0;
            while idx < refs.len() {
                let refs_slice = &refs.as_slice()[idx..std::cmp::min(idx + 100, refs.len())];
                let accounts = self.rpc_client.get_multiple_accounts(refs_slice)?;

                for (account_option, account_ref) in accounts.iter().zip(refs_slice.iter()) {
                    if let Some(account) = account_option {
                        let farm = Farm::unpack(account.data.as_slice())?;
                        farm_map.insert(farm.name.as_str().to_string(), farm);
                    } else {
                        return Err(FarmClientError::RecordNotFound(format!(
                            "Farm with ref {}",
                            account_ref
                        )));
                    }
                }
                idx += 100;
            }

            self.farms.borrow_mut().set(farm_map, 1);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn reload_token_refs_if_stale(&self) -> Result<bool, FarmClientError> {
        if self.token_refs.borrow().is_stale() {
            let (header, token_refs) =
                self.get_refdb_pubkey_map(&refdb::StorageType::Token.to_string())?;
            if self.token_refs.borrow().is_updated(header.counter) {
                self.token_refs.borrow_mut().set(token_refs, header.counter);
                self.tokens.borrow_mut().reset();
                return Ok(true);
            } else {
                self.token_refs.borrow_mut().mark_not_stale();
            }
        }
        Ok(false)
    }

    fn reload_tokens_if_empty(&self) -> Result<bool, FarmClientError> {
        if self.tokens.borrow().is_empty() || self.tokens.borrow().is_updated(1) {
            let refs_map = &self.token_refs.borrow().data;
            let refs: Vec<Pubkey> = refs_map.values().copied().collect();
            if refs.is_empty() {
                return Ok(false);
            }
            let mut token_map = TokenMap::new();

            let mut idx = 0;
            while idx < refs.len() {
                let refs_slice = &refs.as_slice()[idx..std::cmp::min(idx + 100, refs.len())];
                let accounts = self.rpc_client.get_multiple_accounts(refs_slice)?;

                for (account_option, account_ref) in accounts.iter().zip(refs_slice.iter()) {
                    if let Some(account) = account_option {
                        let token = Token::unpack(account.data.as_slice())?;
                        token_map.insert(token.name.as_str().to_string(), token);
                    } else {
                        return Err(FarmClientError::RecordNotFound(format!(
                            "Token with ref {}",
                            account_ref
                        )));
                    }
                }
                idx += 100;
            }

            self.tokens.borrow_mut().set(token_map, 1);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn reload_program_ids_if_stale(&self) -> Result<bool, FarmClientError> {
        if self.official_ids.borrow().is_stale() {
            let (header, official_ids) =
                self.get_refdb_pubkey_map(&refdb::StorageType::Program.to_string())?;
            if self.official_ids.borrow().is_updated(header.counter) {
                self.official_ids
                    .borrow_mut()
                    .set(official_ids, header.counter);
                return Ok(true);
            } else {
                self.official_ids.borrow_mut().mark_not_stale();
            }
        }
        Ok(false)
    }

    fn get_token_by_ref_from_cache(
        &self,
        token_ref: &Option<Pubkey>,
    ) -> Result<Option<Token>, FarmClientError> {
        if let Some(pubkey) = token_ref {
            let name = self.get_token_name(pubkey)?;
            Ok(Some(self.get_token(&name)?))
        } else {
            Ok(None)
        }
    }

    fn get_token_account(&self, wallet_address: &Pubkey, token: &Option<Token>) -> Option<Pubkey> {
        token.map(|token_info| get_associated_token_address(wallet_address, &token_info.mint))
    }

    fn pool_has_reverse_tokens(pool_name: &str, token_a: &str) -> Result<bool, FarmClientError> {
        let (_, pool_token_a, _) = FarmClient::extract_token_names(pool_name)?;
        Ok(pool_token_a != token_a)
    }

    fn get_raydium_stake_account(
        &self,
        wallet_address: &Pubkey,
        farm_name: &str,
    ) -> Result<Pubkey, FarmClientError> {
        let farm = self.get_farm(farm_name)?;
        let farm_id = match farm.route {
            FarmRoute::Raydium { farm_id, .. } => farm_id,
            _ => unreachable!(),
        };

        // lookup in cache
        let acc_key = farm_id.to_string();
        if let Some(addr_map) = self.stake_accounts.borrow()[0].get(&wallet_address.to_string()) {
            if let Some(stake_acc) = addr_map.get(&acc_key) {
                return Ok(*stake_acc);
            }
        }

        let mut stake_acc = None;
        {
            // search on-chain
            let acc_vec = self.get_accounts_with_filter(
                &farm.farm_program_id,
                40,
                wallet_address.as_ref().to_vec(),
            )?;
            let user_acc_str = wallet_address.to_string();
            let stake_accounts_map = &mut self.stake_accounts.borrow_mut()[0];
            if !stake_accounts_map.contains_key(&user_acc_str) {
                stake_accounts_map.insert(user_acc_str.clone(), StakeAccMap::new());
            }
            let user_acc_map = stake_accounts_map.get_mut(&user_acc_str).unwrap();
            for (stake_acc_key, account) in acc_vec.iter() {
                let farm_id_str = if farm.version >= 4 {
                    RaydiumUserStakeInfoV4::unpack(account.data.as_slice())?
                        .farm_id
                        .to_string()
                } else {
                    RaydiumUserStakeInfo::unpack(account.data.as_slice())?
                        .farm_id
                        .to_string()
                };
                user_acc_map.insert(farm_id_str.clone(), *stake_acc_key);
                if farm_id_str == acc_key {
                    stake_acc = Some(*stake_acc_key);
                }
            }
        }

        if let Some(acc) = stake_acc {
            Ok(acc)
        } else {
            let stake_acc = Pubkey::find_program_address(
                &[b"Miner", &farm_id.to_bytes(), &wallet_address.to_bytes()],
                &farm.router_program_id,
            )
            .0;
            self.update_stake_accounts_cache(wallet_address, farm_name, &acc_key, &stake_acc)?;
            Ok(stake_acc)
        }
    }

    fn get_saber_stake_account(
        &self,
        wallet_address: &Pubkey,
        farm_name: &str,
    ) -> Result<Pubkey, FarmClientError> {
        let farm = self.get_farm(farm_name)?;
        let quarry = match farm.route {
            FarmRoute::Saber { quarry, .. } => quarry,
            _ => unreachable!(),
        };

        // lookup in cache
        let acc_key = quarry.to_string();
        if let Some(addr_map) = self.stake_accounts.borrow()[1].get(&wallet_address.to_string()) {
            if let Some(stake_acc) = addr_map.get(&acc_key) {
                return Ok(*stake_acc);
            }
        }

        // update cache
        let (miner, _) = Pubkey::find_program_address(
            &[b"Miner", &quarry.to_bytes(), &wallet_address.to_bytes()],
            &quarry_mine::id(),
        );
        self.update_stake_accounts_cache(wallet_address, farm_name, &acc_key, &miner)?;

        Ok(miner)
    }

    fn get_orca_stake_account(
        &self,
        wallet_address: &Pubkey,
        farm_name: &str,
    ) -> Result<Pubkey, FarmClientError> {
        let farm = self.get_farm(farm_name)?;
        let farm_id = match farm.route {
            FarmRoute::Orca { farm_id, .. } => farm_id,
            _ => unreachable!(),
        };

        // lookup in cache
        let acc_key = farm_id.to_string();
        if let Some(addr_map) = self.stake_accounts.borrow()[2].get(&wallet_address.to_string()) {
            if let Some(stake_acc) = addr_map.get(&acc_key) {
                return Ok(*stake_acc);
            }
        }

        // update cache
        let farmer = Pubkey::find_program_address(
            &[
                &farm_id.to_bytes(),
                &wallet_address.to_bytes(),
                &spl_token::id().to_bytes(),
            ],
            &farm.farm_program_id,
        )
        .0;
        self.update_stake_accounts_cache(wallet_address, farm_name, &acc_key, &farmer)?;

        Ok(farmer)
    }

    fn update_stake_accounts_cache(
        &self,
        wallet_address: &Pubkey,
        farm_name: &str,
        acc_key: &str,
        acc_address: &Pubkey,
    ) -> Result<(), FarmClientError> {
        let farm = self.get_farm(farm_name)?;
        let index = match farm.route {
            FarmRoute::Raydium { .. } => 0,
            FarmRoute::Saber { .. } => 1,
            FarmRoute::Orca { .. } => 2,
        };
        let stake_accounts_map = &mut self.stake_accounts.borrow_mut()[index];
        let wallet_str = wallet_address.to_string();
        stake_accounts_map
            .entry(wallet_str)
            .or_insert_with(StakeAccMap::new);
        let user_acc_map = stake_accounts_map
            .get_mut(&wallet_address.to_string())
            .unwrap();
        user_acc_map.insert(acc_key.to_string(), *acc_address);
        Ok(())
    }

    fn check_user_stake_account(
        &self,
        wallet_address: &Pubkey,
        farm_name: &str,
        instruction_vec: &mut Vec<Instruction>,
    ) -> Result<(), FarmClientError> {
        let farm = self.get_farm(farm_name)?;
        let acc_address = match farm.route {
            FarmRoute::Raydium { .. } => {
                self.get_raydium_stake_account(wallet_address, farm_name)?
            }
            FarmRoute::Saber { .. } => self.get_saber_stake_account(wallet_address, farm_name)?,
            FarmRoute::Orca { .. } => self.get_orca_stake_account(wallet_address, farm_name)?,
        };
        let data = self.rpc_client.get_account_data(&acc_address);
        if data.is_err() || data.unwrap().is_empty() {
            instruction_vec.push(self.new_instruction_user_init(wallet_address, farm_name)?);
        }
        Ok(())
    }

    fn get_pool_price_raydium(
        &self,
        token_a_balance: u64,
        token_b_balance: u64,
        token_a_decimals: u8,
        token_b_decimals: u8,
        amm_id: &Pubkey,
        amm_open_orders: &Pubkey,
    ) -> Result<f64, FarmClientError> {
        // adjust with open orders
        let mut token_a_balance = token_a_balance;
        let mut token_b_balance = token_b_balance;
        let open_orders_data = self.rpc_client.get_account_data(amm_open_orders)?;
        if open_orders_data.len() == 3228 {
            let base_token_total = array_ref![open_orders_data, 85, 8];
            let quote_token_total = array_ref![open_orders_data, 101, 8];

            token_a_balance += u64::from_le_bytes(*base_token_total);
            token_b_balance += u64::from_le_bytes(*quote_token_total);
        }

        // adjust with amm take pnl
        let amm_id_data = self.rpc_client.get_account_data(amm_id)?;
        let (pnl_coin_offset, pnl_pc_offset) = if amm_id_data.len() == 624 {
            (136, 144)
        } else if amm_id_data.len() == 680 {
            (144, 152)
        } else if amm_id_data.len() == 752 {
            (192, 200)
        } else {
            (0, 0)
        };
        if pnl_coin_offset > 0 {
            let need_take_pnl_coin =
                u64::from_le_bytes(*array_ref![amm_id_data, pnl_coin_offset, 8]);
            let need_take_pnl_pc = u64::from_le_bytes(*array_ref![amm_id_data, pnl_pc_offset, 8]);

            token_a_balance -= if need_take_pnl_coin < token_a_balance {
                need_take_pnl_coin
            } else {
                token_a_balance
            };
            token_b_balance -= if need_take_pnl_pc < token_b_balance {
                need_take_pnl_pc
            } else {
                token_b_balance
            };
        }

        if token_a_balance == 0 || token_b_balance == 0 {
            Ok(0.0)
        } else {
            Ok(
                self.tokens_to_ui_amount_with_decimals(token_b_balance, token_b_decimals)
                    / self.tokens_to_ui_amount_with_decimals(token_a_balance, token_a_decimals),
            )
        }
    }

    fn get_pool_price_saber(
        &self,
        swap_account: &Pubkey,
        token_a_balance: u64,
        token_b_balance: u64,
        lp_token: &Token,
    ) -> Result<f64, FarmClientError> {
        let swap_data = self.rpc_client.get_account_data(swap_account)?;
        let swap_info = SwapInfo::unpack(swap_data.as_slice())?;

        let mint_data = self.rpc_client.get_account_data(&lp_token.mint)?;
        let lp_mint = Mint::unpack(mint_data.as_slice())?;

        let swap = SaberSwap {
            initial_amp_factor: swap_info.initial_amp_factor,
            target_amp_factor: swap_info.target_amp_factor,
            current_ts: chrono::Utc::now().timestamp(),
            start_ramp_ts: swap_info.start_ramp_ts,
            stop_ramp_ts: swap_info.stop_ramp_ts,
            lp_mint_supply: lp_mint.supply,
            token_a_reserve: token_a_balance,
            token_b_reserve: token_b_balance,
        };

        if let Some(price) = swap.calculate_virtual_price_of_pool_tokens(1000000) {
            Ok(price as f64 / 1000000.0)
        } else {
            Ok(0.0)
        }
    }

    fn get_pool_price_orca(
        &self,
        token_a_balance: u64,
        token_b_balance: u64,
        token_a_decimals: u8,
        token_b_decimals: u8,
    ) -> Result<f64, FarmClientError> {
        if token_a_balance == 0 || token_b_balance == 0 {
            Ok(0.0)
        } else {
            Ok(
                self.tokens_to_ui_amount_with_decimals(token_b_balance, token_b_decimals)
                    / self.tokens_to_ui_amount_with_decimals(token_a_balance, token_a_decimals),
            )
        }
    }

    fn send_sol_to_wsol(
        &self,
        wallet_address: &Pubkey,
        ui_amount: f64,
        instruction_vec: &mut Vec<Instruction>,
    ) -> Result<(), FarmClientError> {
        let token_addr = self.get_associated_token_address(wallet_address, "SOL")?;
        instruction_vec.push(self.new_instruction_transfer(
            wallet_address,
            &token_addr,
            ui_amount,
        )?);
        instruction_vec.push(self.new_instruction_sync_token_balance(wallet_address, "SOL")?);
        Ok(())
    }

    fn check_token_account(
        &self,
        wallet_address: &Pubkey,
        token: &Option<Token>,
        ui_amount: f64,
        instruction_vec: &mut Vec<Instruction>,
    ) -> Result<(), FarmClientError> {
        if let Some(tkn) = token {
            if !self.has_active_token_account(wallet_address, &tkn.name) {
                instruction_vec
                    .push(self.new_instruction_create_token_account(wallet_address, &tkn.name)?);
                if ui_amount > 0.0 {
                    if tkn.token_type == TokenType::WrappedSol {
                        if self.get_account_balance(wallet_address)? < ui_amount {
                            return Err(FarmClientError::InsufficientBalance(tkn.name.to_string()));
                        }
                        let _ =
                            self.send_sol_to_wsol(wallet_address, ui_amount, instruction_vec)?;
                    } else {
                        return Err(FarmClientError::InsufficientBalance(tkn.name.to_string()));
                    }
                }
            } else if ui_amount > 0.0 {
                let balance = self.get_token_account_balance(wallet_address, &tkn.name)?;
                if balance < ui_amount {
                    if tkn.token_type == TokenType::WrappedSol {
                        if self.get_account_balance(wallet_address)? < ui_amount - balance {
                            return Err(FarmClientError::InsufficientBalance(tkn.name.to_string()));
                        }
                        let _ = self.send_sol_to_wsol(
                            wallet_address,
                            ui_amount - balance,
                            instruction_vec,
                        )?;
                    } else {
                        return Err(FarmClientError::InsufficientBalance(tkn.name.to_string()));
                    }
                }
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn check_pool_accounts(
        &self,
        wallet_address: &Pubkey,
        pool_name: &str,
        ui_amount_token_a: f64,
        ui_amount_token_b: f64,
        ui_amount_lp_token: f64,
        check_lp_token: bool,
        instruction_vec: &mut Vec<Instruction>,
    ) -> Result<(), FarmClientError> {
        let pool = self.get_pool(pool_name)?;
        let token_a = self.get_token_by_ref_from_cache(&pool.token_a_ref)?;
        let token_b = self.get_token_by_ref_from_cache(&pool.token_b_ref)?;
        let lp_token = self.get_token_by_ref_from_cache(&pool.lp_token_ref)?;

        if check_lp_token {
            let _ = self.check_token_account(
                wallet_address,
                &lp_token,
                ui_amount_lp_token,
                instruction_vec,
            )?;
        }
        let _ =
            self.check_token_account(wallet_address, &token_a, ui_amount_token_a, instruction_vec)?;
        let _ =
            self.check_token_account(wallet_address, &token_b, ui_amount_token_b, instruction_vec)?;

        if let PoolRoute::Saber {
            wrapped_token_a_ref,
            wrapped_token_b_ref,
            ..
        } = pool.route
        {
            if let Some(token) = self.get_token_by_ref_from_cache(&wrapped_token_a_ref)? {
                let _ = self.check_token_account_with_mint(
                    wallet_address,
                    &token.mint,
                    instruction_vec,
                )?;
            }
            if let Some(token) = self.get_token_by_ref_from_cache(&wrapped_token_b_ref)? {
                let _ = self.check_token_account_with_mint(
                    wallet_address,
                    &token.mint,
                    instruction_vec,
                )?;
            }
        }

        Ok(())
    }

    fn check_token_account_with_mint(
        &self,
        wallet_address: &Pubkey,
        mint: &Pubkey,
        instruction_vec: &mut Vec<Instruction>,
    ) -> Result<(), FarmClientError> {
        let token_address = get_associated_token_address(wallet_address, mint);
        if let Ok(data) = self.rpc_client.get_account_data(&token_address) {
            if let Ok(TokenAccountType::Account(ui_account)) = parse_token(data.as_slice(), Some(0))
            {
                if ui_account.state == UiAccountState::Initialized {
                    return Ok(());
                }
            }
        }

        instruction_vec.push(create_associated_token_account(
            wallet_address,
            wallet_address,
            mint,
        ));
        Ok(())
    }

    fn check_farm_accounts(
        &self,
        wallet_address: &Pubkey,
        farm_name: &str,
        ui_amount: f64,
        instruction_vec: &mut Vec<Instruction>,
    ) -> Result<(), FarmClientError> {
        let farm = self.get_farm(farm_name)?;
        let token_a = self.get_token_by_ref_from_cache(&farm.first_reward_token_ref)?;
        let token_b = self.get_token_by_ref_from_cache(&farm.second_reward_token_ref)?;
        let lp_token = self.get_token_by_ref_from_cache(&farm.lp_token_ref)?;

        let _ = self.check_token_account(wallet_address, &token_a, 0.0, instruction_vec)?;
        let _ = self.check_token_account(wallet_address, &token_b, 0.0, instruction_vec)?;
        let _ = self.check_token_account(wallet_address, &lp_token, ui_amount, instruction_vec)?;

        let _ = self.check_user_stake_account(wallet_address, farm_name, instruction_vec)?;

        match farm.route {
            FarmRoute::Saber { .. } => {
                let user_info_account = self.get_stake_account(wallet_address, farm_name)?;

                let user_vault_account = self
                    .get_token_account(&user_info_account, &lp_token)
                    .ok_or(ProgramError::UninitializedAccount)?;

                let data = self.rpc_client.get_account_data(&user_vault_account);
                if data.is_err() || data.unwrap().is_empty() {
                    instruction_vec.insert(
                        0,
                        create_associated_token_account(
                            wallet_address,
                            &user_info_account,
                            &lp_token.unwrap().mint,
                        ),
                    );
                }
            }
            FarmRoute::Orca { farm_token_ref, .. } => {
                let farm_lp_token = self.get_token_by_ref(&farm_token_ref)?;
                let user_farm_lp_token_account =
                    get_associated_token_address(wallet_address, &farm_lp_token.mint);
                let data = self
                    .rpc_client
                    .get_account_data(&user_farm_lp_token_account);
                if data.is_err() || data.unwrap().is_empty() {
                    instruction_vec.insert(
                        0,
                        create_associated_token_account(
                            wallet_address,
                            wallet_address,
                            &farm_lp_token.mint,
                        ),
                    );
                }
            }
            _ => {}
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn check_vault_accounts(
        &self,
        wallet_address: &Pubkey,
        vault_name: &str,
        ui_amount_token_a: f64,
        ui_amount_token_b: f64,
        ui_amount_vt_token: f64,
        check_vt_token: bool,
        check_lp_token: bool,
        instruction_vec: &mut Vec<Instruction>,
    ) -> Result<(), FarmClientError> {
        let vault = self.get_vault(vault_name)?;
        let vault_token = self.get_token_by_ref_from_cache(&Some(vault.vault_token_ref))?;
        let pool = self.get_underlying_pool(vault_name)?;
        let farm = self.get_underlying_farm(vault_name)?;
        let token_a = self.get_token_by_ref_from_cache(&pool.token_a_ref)?;
        let token_b = self.get_token_by_ref_from_cache(&pool.token_b_ref)?;
        let lp_token = self.get_token_by_ref_from_cache(&pool.lp_token_ref)?;
        let token_a_reward = self.get_token_by_ref_from_cache(&farm.first_reward_token_ref)?;
        let token_b_reward = self.get_token_by_ref_from_cache(&farm.second_reward_token_ref)?;

        if check_vt_token {
            let _ = self.check_token_account(
                wallet_address,
                &vault_token,
                ui_amount_vt_token,
                instruction_vec,
            )?;
        }
        if check_lp_token {
            let _ = self.check_token_account(wallet_address, &lp_token, 0.0, instruction_vec)?;
        }
        let _ =
            self.check_token_account(wallet_address, &token_a, ui_amount_token_a, instruction_vec)?;
        let _ =
            self.check_token_account(wallet_address, &token_b, ui_amount_token_b, instruction_vec)?;

        if token_a_reward.is_some()
            && (token_a.is_none() || token_a.unwrap().name != token_a_reward.unwrap().name)
            && (token_b.is_none() || token_b.unwrap().name != token_a_reward.unwrap().name)
        {
            let _ =
                self.check_token_account(wallet_address, &token_a_reward, 0.0, instruction_vec)?;
        }
        if token_b_reward.is_some()
            && (token_a.is_none() || token_a.unwrap().name != token_b_reward.unwrap().name)
            && (token_b.is_none() || token_b.unwrap().name != token_b_reward.unwrap().name)
            && (token_a_reward.is_none()
                || token_a_reward.unwrap().name != token_b_reward.unwrap().name)
        {
            let _ =
                self.check_token_account(wallet_address, &token_b_reward, 0.0, instruction_vec)?;
        }

        if let PoolRoute::Saber {
            wrapped_token_a_ref,
            wrapped_token_b_ref,
            ..
        } = pool.route
        {
            if let Some(token) = self.get_token_by_ref_from_cache(&wrapped_token_a_ref)? {
                let _ = self.check_token_account_with_mint(
                    wallet_address,
                    &token.mint,
                    instruction_vec,
                )?;
            }
            if let Some(token) = self.get_token_by_ref_from_cache(&wrapped_token_b_ref)? {
                let _ = self.check_token_account_with_mint(
                    wallet_address,
                    &token.mint,
                    instruction_vec,
                )?;
            }
        }

        if self
            .get_vault_user_info(wallet_address, vault_name)
            .is_err()
        {
            instruction_vec.push(self.new_instruction_user_init_vault(wallet_address, vault_name)?);
        }

        Ok(())
    }

    fn check_fund_accounts(
        &self,
        wallet_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
        ui_amount: f64,
        instruction_vec: &mut Vec<Instruction>,
    ) -> Result<(), FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        let fund_token = Some(self.get_token_by_ref(&fund.fund_token_ref)?);
        let asset_token = Some(self.get_token(token_name)?);

        let _ = self.check_token_account(wallet_address, &fund_token, 0.0, instruction_vec)?;

        let ui_amount = if ui_amount == 0.0 && token_name == "SOL" {
            let balance = self.get_account_balance(wallet_address)?;
            let min_balance = self.rpc_client.get_minimum_balance_for_rent_exemption(0)?;
            let fees = self.rpc_client.get_fee_for_message(&Message::new(
                &[self.new_instruction_transfer(wallet_address, wallet_address, 1.0)?],
                None,
            ))? * 10;
            let to_leave = self.tokens_to_ui_amount(min_balance + fees, "SOL")?;
            if balance > to_leave {
                balance - to_leave
            } else {
                0.0
            }
        } else {
            ui_amount
        };
        let _ =
            self.check_token_account(wallet_address, &asset_token, ui_amount, instruction_vec)?;

        if self
            .get_fund_user_requests(wallet_address, fund_name, token_name)
            .is_err()
        {
            instruction_vec.push(self.new_instruction_user_init_fund(
                wallet_address,
                fund_name,
                token_name,
            )?);
        }

        Ok(())
    }

    fn is_wallet_single_fund_admin(
        &self,
        wallet_address: &Pubkey,
        fund_name: &str,
    ) -> Result<bool, FarmClientError> {
        let multisig = self.get_fund_admins(fund_name)?;
        Ok(multisig.num_signers == 1 && &multisig.signers[0] == wallet_address)
    }

    fn check_fund_custody(
        &self,
        wallet_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
        ui_amount: f64,
        instruction_vec: &mut Vec<Instruction>,
    ) -> Result<(), FarmClientError> {
        if self
            .get_fund_custody(fund_name, token_name, FundCustodyType::Trading)
            .is_err()
        {
            if ui_amount > 0.0 {
                return Err(FarmClientError::InsufficientBalance(token_name.to_string()));
            }
            if self.is_wallet_single_fund_admin(wallet_address, fund_name)? {
                instruction_vec.push(self.new_instruction_add_fund_custody(
                    wallet_address,
                    fund_name,
                    token_name,
                    FundCustodyType::Trading,
                )?);
            } else {
                return Err(FarmClientError::RecordNotFound(format!(
                    "Custody for token {} in the Fund {} not found",
                    token_name, fund_name
                )));
            }
        } else if ui_amount > 0.0 {
            let token_account = self.get_fund_custody_token_account(
                fund_name,
                token_name,
                FundCustodyType::Trading,
            )?;
            if self.get_token_account_balance_with_address(&token_account)? < ui_amount {
                return Err(FarmClientError::InsufficientBalance(token_name.to_string()));
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn check_fund_pool_custodies(
        &self,
        wallet_address: &Pubkey,
        fund_name: &str,
        pool_name: &str,
        ui_amount_token_a: f64,
        ui_amount_token_b: f64,
        ui_amount_lp_token: f64,
        check_lp_token: bool,
        instruction_vec: &mut Vec<Instruction>,
    ) -> Result<(), FarmClientError> {
        let pool = self.get_pool(pool_name)?;
        let token_a = self.get_token_by_ref_from_cache(&pool.token_a_ref)?;
        let token_b = self.get_token_by_ref_from_cache(&pool.token_b_ref)?;
        let lp_token = self.get_token_by_ref_from_cache(&pool.lp_token_ref)?;

        if check_lp_token {
            self.check_fund_custody(
                wallet_address,
                fund_name,
                &lp_token.unwrap().name,
                ui_amount_lp_token,
                instruction_vec,
            )?;
        }
        if token_a.is_some() {
            self.check_fund_custody(
                wallet_address,
                fund_name,
                &token_a.unwrap().name,
                ui_amount_token_a,
                instruction_vec,
            )?;
        }
        if token_b.is_some() {
            self.check_fund_custody(
                wallet_address,
                fund_name,
                &token_b.unwrap().name,
                ui_amount_token_b,
                instruction_vec,
            )?;
        }

        Ok(())
    }

    fn check_fund_farm_custodies(
        &self,
        wallet_address: &Pubkey,
        fund_name: &str,
        farm_name: &str,
        ui_amount: f64,
        instruction_vec: &mut Vec<Instruction>,
    ) -> Result<(), FarmClientError> {
        let farm = self.get_farm(farm_name)?;
        let token_a = self.get_token_by_ref_from_cache(&farm.first_reward_token_ref)?;
        let token_b = self.get_token_by_ref_from_cache(&farm.second_reward_token_ref)?;
        let lp_token = self.get_token_by_ref_from_cache(&farm.lp_token_ref)?;

        if lp_token.is_some() {
            self.check_fund_custody(
                wallet_address,
                fund_name,
                &lp_token.unwrap().name,
                ui_amount,
                instruction_vec,
            )?;
        }
        if token_a.is_some() {
            self.check_fund_custody(
                wallet_address,
                fund_name,
                &token_a.unwrap().name,
                0.0,
                instruction_vec,
            )?;
        }
        if token_b.is_some() {
            self.check_fund_custody(
                wallet_address,
                fund_name,
                &token_b.unwrap().name,
                0.0,
                instruction_vec,
            )?;
        }

        self.check_fund_farm_user_account(wallet_address, fund_name, farm_name, instruction_vec)
    }

    #[allow(clippy::too_many_arguments)]
    fn check_fund_vault_custodies(
        &self,
        wallet_address: &Pubkey,
        fund_name: &str,
        vault_name: &str,
        ui_amount_token_a: f64,
        ui_amount_token_b: f64,
        ui_amount_vt_token: f64,
        check_vt_token: bool,
        check_lp_token: bool,
        instruction_vec: &mut Vec<Instruction>,
    ) -> Result<(), FarmClientError> {
        let vault = self.get_vault(vault_name)?;
        let vault_token = self.get_token_by_ref_from_cache(&Some(vault.vault_token_ref))?;
        let pool = self.get_underlying_pool(vault_name)?;
        let farm = self.get_underlying_farm(vault_name)?;
        let token_a = self.get_token_by_ref_from_cache(&pool.token_a_ref)?;
        let token_b = self.get_token_by_ref_from_cache(&pool.token_b_ref)?;
        let lp_token = self.get_token_by_ref_from_cache(&pool.lp_token_ref)?;
        let token_a_reward = self.get_token_by_ref_from_cache(&farm.first_reward_token_ref)?;
        let token_b_reward = self.get_token_by_ref_from_cache(&farm.second_reward_token_ref)?;

        if check_vt_token {
            self.check_fund_custody(
                wallet_address,
                fund_name,
                &vault_token.unwrap().name,
                ui_amount_vt_token,
                instruction_vec,
            )?;
        }
        if check_lp_token {
            self.check_fund_custody(
                wallet_address,
                fund_name,
                &lp_token.unwrap().name,
                0.0,
                instruction_vec,
            )?;
        }
        if token_a_reward.is_some() {
            self.check_fund_custody(
                wallet_address,
                fund_name,
                &token_a_reward.unwrap().name,
                0.0,
                instruction_vec,
            )?;
        }
        if token_b_reward.is_some() {
            self.check_fund_custody(
                wallet_address,
                fund_name,
                &token_b_reward.unwrap().name,
                0.0,
                instruction_vec,
            )?;
        }
        if token_a.is_some() {
            self.check_fund_custody(
                wallet_address,
                fund_name,
                &token_a.unwrap().name,
                ui_amount_token_a,
                instruction_vec,
            )?;
        }
        if token_b.is_some() {
            self.check_fund_custody(
                wallet_address,
                fund_name,
                &token_b.unwrap().name,
                ui_amount_token_b,
                instruction_vec,
            )?;
        }

        self.check_fund_vault_user_account(wallet_address, fund_name, vault_name, instruction_vec)
    }

    fn check_fund_farm_user_account(
        &self,
        wallet_address: &Pubkey,
        fund_name: &str,
        farm_name: &str,
        instruction_vec: &mut Vec<Instruction>,
    ) -> Result<(), FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        let data = self
            .rpc_client
            .get_account_data(&self.get_stake_account(&fund.fund_authority, farm_name)?);
        if data.is_err() || data.unwrap().is_empty() {
            if &fund.fund_manager == wallet_address
                || self.is_wallet_single_fund_admin(wallet_address, fund_name)?
            {
                instruction_vec.push(self.new_instruction_fund_user_init_farm(
                    wallet_address,
                    fund_name,
                    farm_name,
                )?);
            } else {
                return Err(FarmClientError::RecordNotFound(format!(
                    "User is not initialized for the Farm {} in the Fund {}",
                    farm_name, fund_name
                )));
            }
        }
        Ok(())
    }

    fn check_fund_vault_user_account(
        &self,
        wallet_address: &Pubkey,
        fund_name: &str,
        vault_name: &str,
        instruction_vec: &mut Vec<Instruction>,
    ) -> Result<(), FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        let user_info_account =
            self.get_vault_user_info_account(&fund.fund_authority, vault_name)?;
        let data = self.rpc_client.get_account_data(&user_info_account);
        if data.is_err() || !RefDB::is_initialized(data.unwrap().as_slice()) {
            if &fund.fund_manager == wallet_address
                || self.is_wallet_single_fund_admin(wallet_address, fund_name)?
            {
                instruction_vec.push(self.new_instruction_fund_user_init_vault(
                    wallet_address,
                    fund_name,
                    vault_name,
                )?);
            } else {
                return Err(FarmClientError::RecordNotFound(format!(
                    "User is not initialized for the Vault {} in the Fund {}",
                    vault_name, fund_name
                )));
            }
        }
        Ok(())
    }

    fn get_vault_lp_token_decimals(&self, vault_name: &str) -> Result<u8, FarmClientError> {
        let pool = self.get_underlying_pool(vault_name)?;
        if let Some(pool_token) = self.get_token_by_ref_from_cache(&pool.lp_token_ref)? {
            Ok(pool_token.decimals)
        } else {
            Err(FarmClientError::RecordNotFound(format!(
                "LP token for {}",
                vault_name
            )))
        }
    }

    // note: there could be multiple underlying pools in the future
    fn get_underlying_pool(&self, vault_name: &str) -> Result<Pool, FarmClientError> {
        let vault = self.get_vault(vault_name)?;
        match vault.strategy {
            VaultStrategy::StakeLpCompoundRewards { pool_ref, .. } => {
                self.get_pool_by_ref(&pool_ref)
            }
            VaultStrategy::DynamicHedge { .. } => self.get_pool_by_ref(&zero::id()),
        }
    }

    // note: there could be multiple underlying farms in the future
    fn get_underlying_farm(&self, vault_name: &str) -> Result<Farm, FarmClientError> {
        let vault = self.get_vault(vault_name)?;
        match vault.strategy {
            VaultStrategy::StakeLpCompoundRewards { farm_ref, .. } => {
                self.get_farm_by_ref(&farm_ref)
            }
            VaultStrategy::DynamicHedge { .. } => self.get_farm_by_ref(&zero::id()),
        }
    }

    fn get_vault_price(&self, vault_name: &str) -> Result<f64, FarmClientError> {
        let pool_name = self.get_underlying_pool(vault_name)?.name.to_string();
        self.get_pool_price(&pool_name)
    }

    fn get_protocol_stats(&self, protocol: Protocol) -> Result<(u32, u32, u32), FarmClientError> {
        let pools = self.get_pools()?;
        let farms = self.get_farms()?;
        let vaults = self.get_vaults()?;
        let mut pools_num = 0u32;
        let mut farms_num = 0u32;
        let mut vaults_num = 0u32;
        let protocol = protocol.id().to_string() + ".";
        for name in pools.keys() {
            if name.starts_with(&protocol) {
                pools_num += 1;
            }
        }
        for name in farms.keys() {
            if name.starts_with(&protocol) {
                farms_num += 1;
            }
        }
        for name in vaults.keys() {
            if name.starts_with(&protocol) {
                vaults_num += 1;
            }
        }
        Ok((pools_num, farms_num, vaults_num))
    }
}

mod farm_accounts_orca;
mod farm_accounts_raydium;
mod farm_accounts_saber;
mod farm_instructions;
mod fund_instructions;
mod fund_instructions_pools;
mod governance_instructions;
mod main_router_instructions;
mod pool_accounts_orca;
mod pool_accounts_raydium;
mod pool_accounts_saber;
mod pool_instructions;
mod system_instructions;
mod vault_instructions;
mod vault_stc_accounts_orca;
mod vault_stc_accounts_raydium;
mod vault_stc_accounts_saber;

#[cfg(test)]
mod test {
    use super::*;
    use solana_farm_sdk::string::{str_to_as64, ArrayString64};

    #[test]
    fn test_to_array_string() {
        let arrstr: ArrayString64 = ArrayString64::try_from_str("test").unwrap();
        assert_eq!(arrstr, str_to_as64("test").unwrap());
        assert_eq!(arrstr.as_str(), "test");
        assert!(matches!(
            ArrayString64::try_from_str(
                "very long string, longer than 64 characters, conversion must fail"
            ),
            Err(_)
        ));
    }

    #[test]
    fn test_extract_pool_version() {
        assert_eq!(FarmClient::extract_pool_version("RDM.V-V3").unwrap(), 3);
        assert_eq!(FarmClient::extract_pool_version("V-V3").unwrap(), 3);
        assert_eq!(FarmClient::extract_pool_version("RDM.V-V-V3").unwrap(), 3);
        assert!(FarmClient::extract_pool_version("RDM.V-3").is_err());
        assert!(FarmClient::extract_pool_version("RDM.V-V03").is_err());
        assert!(FarmClient::extract_pool_version("RDM.V-V").is_err());
        assert!(FarmClient::extract_pool_version("RDM.V-VV").is_err());
        assert!(FarmClient::extract_pool_version("RDM.V3").is_err());
        assert!(FarmClient::extract_pool_version("-V3").is_err());
        assert!(FarmClient::extract_pool_version("V3").is_err());
        assert!(FarmClient::extract_pool_version("3").is_err());
    }

    #[test]
    fn test_extract_pool_name_and_version() {
        assert_eq!(
            FarmClient::extract_pool_name_and_version("RDM.Q-V-V3").unwrap(),
            ("RDM.Q-V".to_string(), 3)
        );
        assert_eq!(
            FarmClient::extract_pool_name_and_version("RDM.Q-V1-V3").unwrap(),
            ("RDM.Q-V1".to_string(), 3)
        );
        assert_eq!(
            FarmClient::extract_pool_name_and_version("RDM.Q-W-V-V3").unwrap(),
            ("RDM.Q-W-V".to_string(), 3)
        );
        assert_eq!(
            FarmClient::extract_pool_name_and_version("LP.RDM.V-V3").unwrap(),
            ("RDM.V".to_string(), 3)
        );
        assert_eq!(
            FarmClient::extract_pool_name_and_version("RDM.V-V3").unwrap(),
            ("RDM.V".to_string(), 3)
        );
    }

    #[test]
    fn test_extract_token_names() {
        assert_eq!(
            FarmClient::extract_token_names("RDM.Q-V-V3").unwrap(),
            (Protocol::Raydium, "Q".to_string(), "V".to_string())
        );
        assert_eq!(
            FarmClient::extract_token_names("RDM.Q-V1-V3").unwrap(),
            (Protocol::Raydium, "Q".to_string(), "V1".to_string())
        );
        assert_eq!(
            FarmClient::extract_token_names("RDM.Q-W-V-V3").unwrap(),
            (Protocol::Raydium, "Q".to_string(), "W".to_string())
        );
        assert_eq!(
            FarmClient::extract_token_names("RDM.RAY-V3").unwrap(),
            (Protocol::Raydium, "RAY".to_string(), String::default())
        );
        assert_eq!(
            FarmClient::extract_token_names("LP.RDM.Q-V-V3").unwrap(),
            (Protocol::Raydium, "Q".to_string(), "V".to_string())
        );
        assert_eq!(
            FarmClient::extract_token_names("LP.RDM.Q-W-V-V3").unwrap(),
            (Protocol::Raydium, "Q".to_string(), "W".to_string())
        );
        assert_eq!(
            FarmClient::extract_token_names("LP.RDM.RAY-V3").unwrap(),
            (Protocol::Raydium, "RAY".to_string(), String::default())
        );
        assert_eq!(
            FarmClient::extract_token_names("RDM.Q-V").unwrap(),
            (Protocol::Raydium, "Q".to_string(), "V".to_string())
        );
        assert_eq!(
            FarmClient::extract_token_names("RDM.Q").unwrap(),
            (Protocol::Raydium, "Q".to_string(), String::default())
        );
        assert_eq!(
            FarmClient::extract_token_names("LP.RDM.Q-V").unwrap(),
            (Protocol::Raydium, "Q".to_string(), "V".to_string())
        );
    }
}
