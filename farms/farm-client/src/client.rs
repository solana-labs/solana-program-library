//! Solana Farm Client
//!
//! Solana Farm Client provides an easy way to interact with pools, farms, and vaults,
//! query on-chain objects metadata, and perform common operations with accounts.
//!
//! Client's methods accept human readable names (tokens, polls, etc.) and UI (decimal)
//! amounts, so you can simply call client.swap(&keypair, "RDM", "SOL", "RAY", 0.1, 0.0)
//! to swap 0.1 SOL for RAY in a Raydium pool. All metadata required to lookup account
//! addresses, decimals, etc. is stored on-chain.
//!
//! Under the hood it leverages the official Solana RPC Client which can be accessed with
//! client.rpc_client, for example: client.rpc_client.get_latest_blockhash().
//!
//! Naming convention for Pools and Farms is [PROTOCOL].[TOKEN_A]-[TOKEN_B]-[VERSION]
//! Naming convention for Vaults is [PROTOCOL].[STRATEGY].[TOKEN_A]-[TOKEN_B]-[VERSION]
//! There are single token pools where TOKEN_B is not present.
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
//! #  client.find_pools("RDM", "RAY", "SRM").unwrap();
//! #
//! #  // find Saber pools with USDC and USDT tokens
//! #  client.find_pools("SBR", "USDC", "USDT").unwrap();
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
//! #  // list official program IDs
//! #  client.get_program_ids().unwrap();
//! #
//! #  // swap in the Raydium pool
//! #  client.swap(&keypair, "RDM", "SOL", "RAY", 0.01, 0.0).unwrap();
//! #
//! #  // swap in the Saber pool
//! #  client.swap(&keypair, "SBR", "USDC", "USDT", 0.01, 0.0).unwrap();
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

use {
    crate::{cache::Cache, error::FarmClientError},
    arrayref::array_ref,
    solana_account_decoder::parse_token::{
        parse_token, TokenAccountType, UiAccountState, UiTokenAccount,
    },
    solana_client::{
        client_error::ClientErrorKind,
        rpc_client::RpcClient,
        rpc_config::RpcProgramAccountsConfig,
        rpc_custom_error, rpc_filter,
        rpc_request::{RpcError, TokenAccountsFilter},
    },
    solana_farm_sdk::{
        farm::{Farm, FarmRoute},
        id::{
            main_router, main_router_admin, zero, ProgramIDType, DAO_CUSTODY_NAME, DAO_MINT_NAME,
            DAO_PROGRAM_NAME, DAO_TOKEN_NAME,
        },
        instruction::orca::OrcaUserInit,
        pool::{Pool, PoolRoute},
        program::pda::find_refdb_pda,
        program::protocol::{
            raydium::{RaydiumUserStakeInfo, RaydiumUserStakeInfoV4},
            saber::Miner,
        },
        refdb,
        refdb::RefDB,
        string::str_to_as64,
        token::{Token, TokenSelector, TokenType},
        vault::{UserInfo, Vault, VaultInfo, VaultStrategy},
    },
    solana_sdk::{
        borsh::try_from_slice_unchecked,
        clock::UnixTimestamp,
        commitment_config::CommitmentConfig,
        hash::Hasher,
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        program_pack::Pack,
        pubkey::Pubkey,
        signature::{read_keypair, read_keypair_file, Keypair, Signature, Signer},
        signers::Signers,
        system_program,
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
    token_refs: RefCell<Cache<Pubkey>>,
    pool_refs: RefCell<Cache<Pubkey>>,
    farm_refs: RefCell<Cache<Pubkey>>,
    vault_refs: RefCell<Cache<Pubkey>>,
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
            token_refs: RefCell::new(Cache::<Pubkey>::default()),
            pool_refs: RefCell::new(Cache::<Pubkey>::default()),
            farm_refs: RefCell::new(Cache::<Pubkey>::default()),
            vault_refs: RefCell::new(Cache::<Pubkey>::default()),
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

    /// Returns the Vault struct for the given name
    pub fn get_vault(&self, name: &str) -> Result<Vault, FarmClientError> {
        let mut vault_name = if let Some(val) = self.latest_vaults.borrow().get(name) {
            val.clone()
        } else {
            name.to_string()
        };
        // reload Vaults if stale
        if self.vaults.borrow().is_stale() {
            self.vaults.borrow_mut().reset();
        } else {
            // if Vault is in cache return it
            if let Some(vault) = self.vaults.borrow().data.get(&vault_name) {
                return Ok(*vault);
            }
        }
        // reload Vault refs if stale
        if self.reload_vault_refs_if_stale()? {
            vault_name = if let Some(val) = self.latest_vaults.borrow().get(name) {
                val.clone()
            } else {
                name.to_string()
            };
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
        if !self.vaults.borrow().is_stale() {
            return Ok(self.vaults.borrow().data.clone());
        }
        self.reload_vault_refs_if_stale()?;
        self.reload_vaults_if_stale()?;
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

    /// Returns the Pool struct for the given name
    pub fn get_pool(&self, name: &str) -> Result<Pool, FarmClientError> {
        let mut pool_name = if let Some(val) = self.latest_pools.borrow().get(name) {
            val.clone()
        } else {
            name.to_string()
        };
        // reload Pools if stale
        if self.pools.borrow().is_stale() {
            self.pools.borrow_mut().reset();
        } else {
            // if Pool is in cache return it
            if let Some(pool) = self.pools.borrow().data.get(&pool_name) {
                return Ok(*pool);
            }
        }
        // reload Pool refs if stale
        if self.reload_pool_refs_if_stale()? {
            pool_name = if let Some(val) = self.latest_pools.borrow().get(name) {
                val.clone()
            } else {
                name.to_string()
            };
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
        if !self.pools.borrow().is_stale() {
            return Ok(self.pools.borrow().data.clone());
        }
        self.reload_pool_refs_if_stale()?;
        self.reload_pools_if_stale()?;
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
        protocol: &str,
        token_a: &str,
        token_b: &str,
    ) -> Result<Vec<Pool>, FarmClientError> {
        self.reload_pool_refs_if_stale()?;
        let pattern1 = format!("{}.{}-{}-", protocol, token_a, token_b);
        let pattern2 = format!("{}.{}-{}-", protocol, token_b, token_a);
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
        let (protocol, token_a, token_b) = FarmClient::extract_token_names(lp_token_name)?;
        let pools = self.find_pools(&protocol, &token_a, &token_b)?;
        let mut res = vec![];
        for pool in pools {
            if let Some(lp_token) = self.get_token_by_ref_from_cache(&pool.lp_token_ref)? {
                if lp_token.name.as_str() == lp_token_name {
                    res.push(pool);
                }
            }
        }

        if res.is_empty() {
            Err(FarmClientError::RecordNotFound(format!(
                "{} Pool with LP token {}",
                protocol, lp_token_name
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
        let mut farm_name = if let Some(val) = self.latest_farms.borrow().get(name) {
            val.clone()
        } else {
            name.to_string()
        };
        // reload Farms if stale
        if self.farms.borrow().is_stale() {
            self.farms.borrow_mut().reset();
        } else {
            // if Farm is in cache return it
            if let Some(farm) = self.farms.borrow().data.get(&farm_name) {
                return Ok(*farm);
            }
        }
        // reload Farm refs if stale
        if self.reload_farm_refs_if_stale()? {
            farm_name = if let Some(val) = self.latest_farms.borrow().get(name) {
                val.clone()
            } else {
                name.to_string()
            };
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
        if !self.farms.borrow().is_stale() {
            return Ok(self.farms.borrow().data.clone());
        }
        self.reload_farm_refs_if_stale()?;
        self.reload_farms_if_stale()?;
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
        self.reload_farm_refs_if_stale()?;
        let (protocol, token_a, token_b) = FarmClient::extract_token_names(lp_token_name)?;
        let pattern1 = format!("{}.{}-{}-", protocol, token_a, token_b);
        let pattern2 = format!("{}.{}-{}-", protocol, token_b, token_a);
        let mut res = vec![];
        for (name, _) in self.farm_refs.borrow().data.iter() {
            if name.contains(&pattern1) || name.contains(&pattern2) {
                let farm = self.get_farm(name)?;
                if let Some(lp_token) = self.get_token_by_ref_from_cache(&farm.lp_token_ref)? {
                    if lp_token.name.as_str() == lp_token_name {
                        res.push(farm);
                    }
                }
            }
        }

        if res.is_empty() {
            Err(FarmClientError::RecordNotFound(format!(
                "{} Farm with LP token {}",
                protocol, lp_token_name
            )))
        } else {
            res.sort_by(|a, b| b.version.cmp(&a.version));
            Ok(res)
        }
    }

    /// Returns the Token struct for the given name
    pub fn get_token(&self, name: &str) -> Result<Token, FarmClientError> {
        // reload Tokens if stale
        if self.tokens.borrow().is_stale() {
            self.tokens.borrow_mut().reset();
        } else {
            // if Token is in cache return it
            if let Some(token) = self.tokens.borrow().data.get(name) {
                return Ok(*token);
            }
        }
        // reload Token refs if stale
        self.reload_token_refs_if_stale()?;
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
        if !self.tokens.borrow().is_stale() {
            return Ok(self.tokens.borrow().data.clone());
        }
        self.reload_token_refs_if_stale()?;
        self.reload_tokens_if_stale()?;
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
        self.get_refdb_pubkey_map(&refdb::StorageType::Token.to_string())
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
        for (_name, token) in tokens.iter() {
            if token_mint == &token.mint {
                return Ok(*token);
            }
        }
        Err(FarmClientError::RecordNotFound(format!(
            "Token with mint {}",
            token_mint
        )))
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
        self.get_refdb_pubkey_map(&refdb::StorageType::Program.to_string())
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
        let mut transaction =
            Transaction::new_with_payer(instructions, Some(&signers.pubkeys()[0]));

        for i in 0..20 {
            let recent_blockhash = self.rpc_client.get_latest_blockhash()?;
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
                                thread::sleep(time::Duration::from_secs(5));
                                continue;
                            }
                        } else if let RpcError::ForUser(msg) = rpc_error {
                            if msg.starts_with("unable to confirm transaction") {
                                println!("Unable to confirm transaction, re-trying in 5 secs...");
                                thread::sleep(time::Duration::from_secs(5));
                                continue;
                            }
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

    /// Wait for the transaction to become finalized
    pub fn confirm_async_transaction(&self, signature: &Signature) -> Result<(), FarmClientError> {
        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;
        self.rpc_client
            .confirm_transaction_with_spinner(
                signature,
                &recent_blockhash,
                CommitmentConfig::finalized(),
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

    /// Creates a new system account
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

    /// Transfers native SOL from the wallet to the associated Wrapped SOL account.
    pub fn transfer_sol_to_wsol(
        &self,
        signer: &dyn Signer,
        sol_ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        let target_account = self.get_associated_token_address(&signer.pubkey(), "SOL")?;
        let mut inst = Vec::<Instruction>::new();
        if !self.has_active_token_account(&signer.pubkey(), "SOL") {
            inst.push(self.new_instruction_create_token_account(&signer.pubkey(), "SOL")?);
        }
        inst.push(self.new_instruction_transfer(
            &signer.pubkey(),
            &target_account,
            sol_ui_amount,
        )?);
        self.sign_and_send_instructions(&[signer], inst.as_slice())
    }

    /// Transfers tokens from the wallet to the destination
    pub fn token_transfer(
        &self,
        signer: &dyn Signer,
        token_name: &str,
        destination_wallet: &Pubkey,
        ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        let mut inst = vec![];
        if !self.has_active_token_account(&signer.pubkey(), token_name) {
            return Err(FarmClientError::RecordNotFound(format!(
                "Source account with token {}",
                token_name
            )));
        }
        if !self.has_active_token_account(destination_wallet, token_name) {
            let token = self.get_token(token_name)?;
            inst.push(create_associated_token_account(
                &signer.pubkey(),
                destination_wallet,
                &token.mint,
            ));
        }
        inst.push(self.new_instruction_token_transfer(
            &signer.pubkey(),
            token_name,
            destination_wallet,
            ui_amount,
        )?);
        self.sign_and_send_instructions(&[signer], inst.as_slice())
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
            let token_address = Pubkey::from_str(&acc.pubkey).map_err(|_| {
                FarmClientError::ValueError(format!(
                    "Failed to convert the String to a Pubkey {}",
                    acc.pubkey
                ))
            })?;

            let data = self.rpc_client.get_account_data(&token_address)?;
            let token_info = parse_token(data.as_slice(), Some(0))?;
            if let TokenAccountType::Account(ui_account) = token_info {
                let token_mint = Pubkey::from_str(&ui_account.mint).map_err(|_| {
                    FarmClientError::ValueError(format!(
                        "Failed to convert the String to a Pubkey {}",
                        ui_account.mint
                    ))
                })?;
                if let Ok(token) = self.get_token_with_mint(&token_mint) {
                    res.push(token.name.as_str().to_string());
                } else {
                    res.push(acc.pubkey.clone());
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
        let token_address = self.get_associated_token_address(wallet_address, token_name)?;
        let balance = self.rpc_client.get_token_account_balance(&token_address)?;
        if let Some(ui_amount) = balance.ui_amount {
            Ok(ui_amount)
        } else {
            Err(FarmClientError::ParseError(format!(
                "Failed to parse balance for token {}",
                token_name
            )))
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
                &wallet_address.to_bytes()[..],
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

    /// Returns user stats for specific Vault
    pub fn get_vault_user_info(
        &self,
        wallet_address: &Pubkey,
        vault_name: &str,
    ) -> Result<UserInfo, FarmClientError> {
        let user_info_account = self.get_vault_user_info_account(wallet_address, vault_name)?;
        let data = self.rpc_client.get_account_data(&user_info_account)?;
        if !RefDB::is_initialized(data.as_slice()) {
            return Err(ProgramError::UninitializedAccount.into());
        }
        let mut user_info = UserInfo::default();
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

    /// Returns User's stacked balance
    pub fn get_user_stake_balance(
        &self,
        wallet_address: &Pubkey,
        farm_name: &str,
    ) -> Result<f64, FarmClientError> {
        let farm = self.get_farm(farm_name)?;
        match farm.route {
            FarmRoute::Raydium { .. } => {
                if let Some(stake_account) = self.get_stake_account(wallet_address, farm_name)? {
                    let stake_data = self.rpc_client.get_account_data(&stake_account)?;
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
                if let Some(stake_account) = self.get_stake_account(wallet_address, farm_name)? {
                    let stake_data = self.rpc_client.get_account_data(&stake_account)?;
                    if !stake_data.is_empty() {
                        let deposit_balance = Miner::unpack(stake_data.as_slice())?.balance;
                        let farm_token = self.get_token_by_ref(&farm.lp_token_ref.unwrap())?;
                        return Ok(self.tokens_to_ui_amount_with_decimals(
                            deposit_balance,
                            farm_token.decimals,
                        ));
                    }
                }
                Ok(0.0)
            }
            FarmRoute::Orca { farm_token_ref, .. } => {
                let farm_token = self.get_token_by_ref(&farm_token_ref)?;
                self.get_token_account_balance(wallet_address, &farm_token.name)
            }
        }
    }

    /// Returns Vault's stacked balance
    pub fn get_vault_stake_balance(&self, vault_name: &str) -> Result<f64, FarmClientError> {
        let vault = self.get_vault(vault_name)?;
        match vault.strategy {
            VaultStrategy::StakeLpCompoundRewards {
                farm_id_ref,
                vault_stake_info,
                ..
            } => {
                let farm = self.get_farm_by_ref(&farm_id_ref)?;
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
                                FarmRoute::Orca { .. } => 0,
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
        if max_token_a_ui_amount < 0.0
            || max_token_b_ui_amount < 0.0
            || (max_token_a_ui_amount == 0.0 && max_token_b_ui_amount == 0.0)
        {
            return Err(FarmClientError::ValueError(format!(
                "Invalid add liquidity amounts {} and {} specified for Vault {}: Must be greater or equal to zero and at least one non-zero.",
                max_token_a_ui_amount, max_token_b_ui_amount, vault_name
            )));
        }
        // if one of the tokens is SOL and amount is zero, we need to estimate that
        // amount to get it transfered to WSOL
        let is_saber_vault = vault_name.starts_with("SBR.");
        let (is_token_a_sol, is_token_b_sol) = self.vault_has_sol_tokens(vault_name)?;
        let token_a_ui_amount = if max_token_a_ui_amount == 0.0 && is_token_a_sol && !is_saber_vault
        {
            let pool_price = self.get_vault_price(vault_name)?;
            if pool_price > 0.0 {
                max_token_b_ui_amount * 1.03 / pool_price
            } else {
                0.0
            }
        } else {
            max_token_a_ui_amount
        };
        let token_b_ui_amount = if max_token_b_ui_amount == 0.0 && is_token_b_sol && !is_saber_vault
        {
            max_token_a_ui_amount * self.get_vault_price(vault_name)? * 1.03
        } else {
            max_token_b_ui_amount
        };

        // check user accounts
        let mut inst = Vec::<Instruction>::new();
        self.check_vault_accounts(
            signer,
            vault_name,
            token_a_ui_amount,
            token_b_ui_amount,
            0.0,
            true,
            true,
            &mut inst,
        )?;

        if !inst.is_empty() {
            self.sign_and_send_instructions(&[signer], inst.as_slice())?;
            inst.clear();
        }

        // check if tokens must be wrapped to Saber decimal token
        if is_saber_vault {
            let pool_name = self.get_underlying_pool(vault_name)?.name.to_string();
            let (is_token_a_wrapped, is_token_b_wrapped) =
                self.pool_has_saber_wrapped_tokens(&pool_name)?;
            if is_token_a_wrapped && max_token_a_ui_amount > 0.0 {
                inst.push(self.new_instruction_wrap_token(
                    &signer.pubkey(),
                    &pool_name,
                    TokenSelector::TokenA,
                    max_token_a_ui_amount,
                )?);
            }
            if is_token_b_wrapped && max_token_b_ui_amount > 0.0 {
                inst.push(self.new_instruction_wrap_token(
                    &signer.pubkey(),
                    &pool_name,
                    TokenSelector::TokenB,
                    max_token_b_ui_amount,
                )?);
            }
        }

        // insert add liquidity instruction
        inst.push(self.new_instruction_add_liquidity_vault(
            &signer.pubkey(),
            vault_name,
            max_token_a_ui_amount,
            max_token_b_ui_amount,
        )?);
        if is_token_a_sol || is_token_b_sol {
            inst.push(self.new_instruction_close_token_account(&signer.pubkey(), "SOL")?);
        }

        // lock liquidity if required by the vault
        let vault = self.get_vault(vault_name)?;
        if vault.lock_required {
            let lp_debt_initial = self
                .get_vault_user_info(&signer.pubkey(), vault_name)?
                .lp_tokens_debt;
            let _ = self.sign_and_send_instructions(&[signer], inst.as_slice())?;

            let lp_debt = self
                .get_vault_user_info(&signer.pubkey(), vault_name)?
                .lp_tokens_debt;
            if lp_debt > lp_debt_initial {
                let pool_token_decimals = self.get_vault_lp_token_decimals(vault_name)?;
                let locked_amount = self.tokens_to_ui_amount_with_decimals(
                    lp_debt - lp_debt_initial,
                    pool_token_decimals,
                );

                let lock_inst = self.new_instruction_lock_liquidity_vault(
                    &signer.pubkey(),
                    vault_name,
                    locked_amount,
                )?;
                self.sign_and_send_instructions(&[signer], &[lock_inst])
            } else {
                Err(FarmClientError::InsufficientBalance(
                    "No tokens were locked".to_string(),
                ))
            }
        } else {
            self.sign_and_send_instructions(&[signer], inst.as_slice())
        }
    }

    /// Adds locked liquidity to the Vault.
    /// Useful if add liquidity operation partially failed.
    pub fn add_locked_liquidity_vault(
        &self,
        signer: &dyn Signer,
        vault_name: &str,
        ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        // check user accounts
        let mut inst = Vec::<Instruction>::new();
        self.check_vault_accounts(signer, vault_name, 0.0, 0.0, 0.0, true, false, &mut inst)?;
        if !inst.is_empty() {
            self.sign_and_send_instructions(&[signer], inst.as_slice())?;
            inst.clear();
        }

        // check if the user has locked balance
        if ui_amount > 0.0 {
            let lp_debt = self
                .get_vault_user_info(&signer.pubkey(), vault_name)?
                .lp_tokens_debt;
            let pool_token_decimals = self.get_vault_lp_token_decimals(vault_name)?;
            if self.tokens_to_ui_amount_with_decimals(lp_debt, pool_token_decimals) < ui_amount {
                return Err(FarmClientError::InsufficientBalance(
                    "Not enough locked tokens to deposit".to_string(),
                ));
            }
        }

        inst.push(self.new_instruction_lock_liquidity_vault(
            &signer.pubkey(),
            vault_name,
            ui_amount,
        )?);
        self.sign_and_send_instructions(&[signer], inst.as_slice())
    }

    /// Removes liquidity from the Vault
    pub fn remove_liquidity_vault(
        &self,
        signer: &dyn Signer,
        vault_name: &str,
        ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        // check user accounts
        let vault = self.get_vault(vault_name)?;
        let mut inst = Vec::<Instruction>::new();
        self.check_vault_accounts(
            signer, vault_name, 0.0, 0.0, ui_amount, true, false, &mut inst,
        )?;
        if !inst.is_empty() {
            self.sign_and_send_instructions(&[signer], inst.as_slice())?;
            inst.clear();
        }

        // unlock liquidity first if required by the vault
        let mut unlocked_amount = ui_amount;
        if vault.unlock_required {
            let lp_debt_initial = self
                .get_vault_user_info(&signer.pubkey(), vault_name)?
                .lp_tokens_debt;
            let unlock_inst = self.new_instruction_unlock_liquidity_vault(
                &signer.pubkey(),
                vault_name,
                ui_amount,
            )?;
            self.sign_and_send_instructions(&[signer], &[unlock_inst])?;
            let lp_debt = self
                .get_vault_user_info(&signer.pubkey(), vault_name)?
                .lp_tokens_debt;
            if lp_debt > lp_debt_initial {
                let pool_token_decimals = self.get_vault_lp_token_decimals(vault_name)?;
                unlocked_amount = self.tokens_to_ui_amount_with_decimals(
                    lp_debt - lp_debt_initial,
                    pool_token_decimals,
                );
            } else {
                return Err(FarmClientError::InsufficientBalance(
                    "No tokens were unlocked".to_string(),
                ));
            }
        }

        // remove liquidity
        inst.push(self.new_instruction_remove_liquidity_vault(
            &signer.pubkey(),
            vault_name,
            unlocked_amount,
        )?);

        // check if tokens need to be unwrapped
        let (is_token_a_sol, is_token_b_sol) = self.vault_has_sol_tokens(vault_name)?;
        let pool_name = self.get_underlying_pool(vault_name)?.name.to_string();
        let (is_token_a_wrapped, is_token_b_wrapped) =
            self.pool_has_saber_wrapped_tokens(&pool_name)?;

        if is_token_a_wrapped {
            inst.push(self.new_instruction_unwrap_token(
                &signer.pubkey(),
                &pool_name,
                TokenSelector::TokenA,
                0.0,
            )?);
        }
        if is_token_b_wrapped {
            inst.push(self.new_instruction_unwrap_token(
                &signer.pubkey(),
                &pool_name,
                TokenSelector::TokenB,
                0.0,
            )?);
        }
        if is_token_a_sol || is_token_b_sol {
            inst.push(self.new_instruction_close_token_account(&signer.pubkey(), "SOL")?);
        }

        self.sign_and_send_instructions(&[signer], inst.as_slice())
    }

    /// Removes unlocked liquidity from the Vault.
    /// Useful if remove liquidity operation failed after unlock step.
    pub fn remove_unlocked_liquidity_vault(
        &self,
        signer: &dyn Signer,
        vault_name: &str,
        ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        // check user accounts
        let mut inst = Vec::<Instruction>::new();
        self.check_vault_accounts(signer, vault_name, 0.0, 0.0, 0.0, false, false, &mut inst)?;
        if !inst.is_empty() {
            self.sign_and_send_instructions(&[signer], inst.as_slice())?;
            inst.clear();
        }

        // check if the user has unlocked balance
        if ui_amount > 0.0 {
            let lp_debt = self
                .get_vault_user_info(&signer.pubkey(), vault_name)?
                .lp_tokens_debt;
            let pool_token_decimals = self.get_vault_lp_token_decimals(vault_name)?;
            if self.tokens_to_ui_amount_with_decimals(lp_debt, pool_token_decimals) < ui_amount {
                return Err(FarmClientError::InsufficientBalance(
                    "Not enough unlocked tokens to remove".to_string(),
                ));
            }
        }

        inst.push(self.new_instruction_remove_liquidity_vault(
            &signer.pubkey(),
            vault_name,
            ui_amount,
        )?);

        // check if tokens need to be unwrapped
        let (is_token_a_sol, is_token_b_sol) = self.vault_has_sol_tokens(vault_name)?;
        let pool_name = self.get_underlying_pool(vault_name)?.name.to_string();
        let (is_token_a_wrapped, is_token_b_wrapped) =
            self.pool_has_saber_wrapped_tokens(&pool_name)?;

        if is_token_a_wrapped {
            inst.push(self.new_instruction_unwrap_token(
                &signer.pubkey(),
                &pool_name,
                TokenSelector::TokenA,
                0.0,
            )?);
        }
        if is_token_b_wrapped {
            inst.push(self.new_instruction_unwrap_token(
                &signer.pubkey(),
                &pool_name,
                TokenSelector::TokenB,
                0.0,
            )?);
        }
        if is_token_a_sol || is_token_b_sol {
            inst.push(self.new_instruction_close_token_account(&signer.pubkey(), "SOL")?);
        }

        self.sign_and_send_instructions(&[signer], inst.as_slice())
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
        if max_token_a_ui_amount < 0.0
            || max_token_b_ui_amount < 0.0
            || (max_token_a_ui_amount == 0.0 && max_token_b_ui_amount == 0.0)
        {
            return Err(FarmClientError::ValueError(format!(
                "Invalid add liquidity amounts {} and {} specified for Pool {}: Must be greater or equal to zero and at least one non-zero.",
                max_token_a_ui_amount, max_token_b_ui_amount, pool_name
            )));
        }
        // if one of the tokens is SOL and amount is zero, we need to estimate that
        // amount to get it transfered to WSOL
        let is_saber_pool = pool_name.starts_with("SBR.");
        let (is_token_a_sol, is_token_b_sol) = self.pool_has_sol_tokens(pool_name)?;
        let token_a_ui_amount = if max_token_a_ui_amount == 0.0 && is_token_a_sol && !is_saber_pool
        {
            let pool_price = self.get_pool_price(pool_name)?;
            if pool_price > 0.0 {
                max_token_b_ui_amount * 1.03 / pool_price
            } else {
                0.0
            }
        } else {
            max_token_a_ui_amount
        };
        let token_b_ui_amount = if max_token_b_ui_amount == 0.0 && is_token_b_sol && !is_saber_pool
        {
            max_token_a_ui_amount * self.get_pool_price(pool_name)? * 1.03
        } else {
            max_token_b_ui_amount
        };

        let mut inst = Vec::<Instruction>::new();
        let _ = self.check_pool_accounts(
            signer,
            pool_name,
            token_a_ui_amount,
            token_b_ui_amount,
            0.0,
            true,
            &mut inst,
        )?;

        // check if tokens need to be wrapped to a Saber decimal token
        if is_saber_pool {
            let (is_token_a_wrapped, is_token_b_wrapped) =
                self.pool_has_saber_wrapped_tokens(pool_name)?;
            if is_token_a_wrapped && max_token_a_ui_amount > 0.0 {
                inst.push(self.new_instruction_wrap_token(
                    &signer.pubkey(),
                    pool_name,
                    TokenSelector::TokenA,
                    max_token_a_ui_amount,
                )?);
            }
            if is_token_b_wrapped && max_token_b_ui_amount > 0.0 {
                inst.push(self.new_instruction_wrap_token(
                    &signer.pubkey(),
                    pool_name,
                    TokenSelector::TokenB,
                    max_token_b_ui_amount,
                )?);
            }
        }

        // create and send instruction
        inst.push(self.new_instruction_add_liquidity_pool(
            &signer.pubkey(),
            pool_name,
            max_token_a_ui_amount,
            max_token_b_ui_amount,
        )?);
        if is_token_a_sol || is_token_b_sol {
            inst.push(self.new_instruction_close_token_account(&signer.pubkey(), "SOL")?);
        }
        self.sign_and_send_instructions(&[signer], inst.as_slice())
    }

    /// Removes liquidity from the Pool.
    /// If the amount is set to zero entire balance will be removed from the pool.
    pub fn remove_liquidity_pool(
        &self,
        signer: &dyn Signer,
        pool_name: &str,
        ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        let mut inst = Vec::<Instruction>::new();
        let _ =
            self.check_pool_accounts(signer, pool_name, 0.0, 0.0, ui_amount, true, &mut inst)?;

        inst.push(self.new_instruction_remove_liquidity_pool(
            &signer.pubkey(),
            pool_name,
            ui_amount,
        )?);

        // check if tokens need to be unwrapped
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

        self.sign_and_send_instructions(&[signer], inst.as_slice())
    }

    /// Swaps tokens
    pub fn swap(
        &self,
        signer: &dyn Signer,
        protocol: &str,
        from_token: &str,
        to_token: &str,
        ui_amount_in: f64,
        min_ui_amount_out: f64,
    ) -> Result<Signature, FarmClientError> {
        // find pool to swap in
        let pool = self.find_pools(protocol, from_token, to_token)?[0];

        // check amount
        if ui_amount_in < 0.0 {
            return Err(FarmClientError::ValueError(format!(
                "Invalid token amount {} specified for pool {}: Must be zero or greater.",
                ui_amount_in,
                pool.name.as_str()
            )));
        }

        // if amount is zero use entire balance
        let ui_amount_in = if ui_amount_in == 0.0 {
            if from_token == "SOL" {
                return Err(FarmClientError::ValueError(format!(
                    "Invalid SOL amount {} specified for pool {}: Must be greater than zero.",
                    ui_amount_in,
                    pool.name.as_str()
                )));
            }
            let balance = self.get_token_account_balance(&signer.pubkey(), from_token)?;
            if balance == 0.0 {
                return Err(FarmClientError::InsufficientBalance(from_token.to_string()));
            }
            balance
        } else {
            ui_amount_in
        };

        // check token accounts
        let mut inst = Vec::<Instruction>::new();
        let reverse = FarmClient::pool_has_reverse_tokens(&pool.name, from_token)?;
        if reverse {
            let _ = self.check_pool_accounts(
                signer,
                &pool.name.to_string(),
                0.0,
                ui_amount_in,
                0.0,
                false,
                &mut inst,
            )?;
        } else {
            let _ = self.check_pool_accounts(
                signer,
                &pool.name.to_string(),
                ui_amount_in,
                0.0,
                0.0,
                false,
                &mut inst,
            )?;
        }

        // check if tokens must be wrapped to Saber decimal token
        let (is_token_a_wrapped, is_token_b_wrapped) =
            self.pool_has_saber_wrapped_tokens(&pool.name)?;
        if is_token_a_wrapped && !reverse {
            inst.push(self.new_instruction_wrap_token(
                &signer.pubkey(),
                &pool.name,
                TokenSelector::TokenA,
                ui_amount_in,
            )?);
        }
        if is_token_b_wrapped && reverse {
            inst.push(self.new_instruction_wrap_token(
                &signer.pubkey(),
                &pool.name,
                TokenSelector::TokenB,
                ui_amount_in,
            )?);
        }

        // create and send instruction
        inst.push(self.new_instruction_swap(
            &signer.pubkey(),
            protocol,
            from_token,
            to_token,
            ui_amount_in,
            min_ui_amount_out,
        )?);
        if is_token_b_wrapped && !reverse {
            inst.push(self.new_instruction_unwrap_token(
                &signer.pubkey(),
                &pool.name,
                TokenSelector::TokenB,
                0.0,
            )?);
        }
        if is_token_a_wrapped && reverse {
            inst.push(self.new_instruction_unwrap_token(
                &signer.pubkey(),
                &pool.name,
                TokenSelector::TokenA,
                0.0,
            )?);
        }
        if to_token == "SOL" {
            inst.push(self.new_instruction_close_token_account(&signer.pubkey(), "SOL")?);
        }
        self.sign_and_send_instructions(&[signer], inst.as_slice())
    }

    /// Stakes tokens to the Farm.
    /// If the amount is set to zero entire LP tokens balance will be staked.
    pub fn stake(
        &self,
        signer: &dyn Signer,
        farm_name: &str,
        ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        let mut inst = Vec::<Instruction>::new();
        let mut signers = Vec::<Box<dyn Signer>>::new();
        let _ = self.check_farm_accounts(signer, farm_name, ui_amount, &mut inst, &mut signers)?;
        inst.push(self.new_instruction_stake(&signer.pubkey(), farm_name, ui_amount)?);

        let mut unboxed_signers: Vec<&dyn Signer> = vec![signer];
        unboxed_signers.append(&mut signers.iter().map(|x| x.as_ref()).collect());
        self.sign_and_send_instructions(&unboxed_signers, inst.as_slice())
    }

    /// Unstakes tokens from the Farm.
    /// If the amount is set to zero entire balance will be unstaked.
    pub fn unstake(
        &self,
        signer: &dyn Signer,
        farm_name: &str,
        ui_amount: f64,
    ) -> Result<Signature, FarmClientError> {
        let mut inst = Vec::<Instruction>::new();
        let mut signers = Vec::<Box<dyn Signer>>::new();
        let _ = self.check_farm_accounts(signer, farm_name, 0.0, &mut inst, &mut signers)?;
        inst.push(self.new_instruction_unstake(&signer.pubkey(), farm_name, ui_amount)?);

        let mut unboxed_signers: Vec<&dyn Signer> = vec![signer];
        unboxed_signers.append(&mut signers.iter().map(|x| x.as_ref()).collect());
        self.sign_and_send_instructions(&unboxed_signers, inst.as_slice())
    }

    /// Harvests rewards from the Pool
    pub fn harvest(
        &self,
        signer: &dyn Signer,
        farm_name: &str,
    ) -> Result<Signature, FarmClientError> {
        let mut inst = Vec::<Instruction>::new();
        let mut signers = Vec::<Box<dyn Signer>>::new();
        let _ = self.check_farm_accounts(signer, farm_name, 0.0, &mut inst, &mut signers)?;
        inst.push(self.new_instruction_harvest(&signer.pubkey(), farm_name)?);

        let mut unboxed_signers: Vec<&dyn Signer> = vec![signer];
        unboxed_signers.append(&mut signers.iter().map(|x| x.as_ref()).collect());
        self.sign_and_send_instructions(&unboxed_signers, inst.as_slice())
    }

    /// Clears cache records to force re-pull from blockchain
    pub fn reset_cache(&self) {
        self.tokens.borrow_mut().reset();
        self.pools.borrow_mut().reset();
        self.vaults.borrow_mut().reset();
        self.token_refs.borrow_mut().reset();
        self.pool_refs.borrow_mut().reset();
        self.vault_refs.borrow_mut().reset();
        self.official_ids.borrow_mut().reset();
        self.latest_pools.borrow_mut().clear();
        self.latest_farms.borrow_mut().clear();
        self.latest_vaults.borrow_mut().clear();
    }

    /// Reads records from the RefDB PDA into a Pubkey map
    pub fn get_refdb_pubkey_map(&self, refdb_name: &str) -> Result<PubkeyMap, FarmClientError> {
        let refdb_address = find_refdb_pda(refdb_name).0;
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
        Ok(map)
    }

    /// Returns raw RefDB data, can be further used with refdb::RefDB
    pub fn get_refdb_data(&self, refdb_name: &str) -> Result<Vec<u8>, FarmClientError> {
        let refdb_address = find_refdb_pda(refdb_name).0;
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
        let refdb_address = find_refdb_pda(refdb_name).0;
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
            if admin_signer.pubkey() != main_router_admin::id() {
                return Err(FarmClientError::ValueError(
                    "Admin keypair must match main_router_admin::id()".to_string(),
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

        let inst = self.new_instruction_refdb_init(
            &admin_signer.pubkey(),
            refdb_name,
            reference_type,
            max_records as u32,
            init_account,
        )?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
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

    /// Removes the Program ID metadata from chain
    pub fn remove_reference(
        &self,
        admin_signer: &dyn Signer,
        storage_type: refdb::StorageType,
        object_name: &str,
        refdb_index: Option<usize>,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_remove_reference(
            &admin_signer.pubkey(),
            storage_type,
            object_name,
            refdb_index,
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
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Removes the Program ID metadata from chain
    pub fn remove_program_id(
        &self,
        admin_signer: &dyn Signer,
        name: &str,
        refdb_index: Option<usize>,
    ) -> Result<Signature, FarmClientError> {
        let inst =
            self.new_instruction_remove_program_id(&admin_signer.pubkey(), name, refdb_index)?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Records the Vault metadata on-chain
    pub fn add_vault(
        &self,
        admin_signer: &dyn Signer,
        vault: Vault,
    ) -> Result<Signature, FarmClientError> {
        self.vaults
            .borrow_mut()
            .data
            .insert(vault.name.to_string(), vault);
        self.vault_refs.borrow_mut().reset();
        let inst = self.new_instruction_add_vault(&admin_signer.pubkey(), vault)?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Removes the Vault's on-chain metadata
    pub fn remove_vault(
        &self,
        admin_signer: &dyn Signer,
        vault_name: &str,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_remove_vault(&admin_signer.pubkey(), vault_name)?;
        self.vaults.borrow_mut().data.remove(vault_name);
        self.vault_refs.borrow_mut().data.remove(vault_name);
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Records the Pool metadata on-chain
    pub fn add_pool(
        &self,
        admin_signer: &dyn Signer,
        pool: Pool,
    ) -> Result<Signature, FarmClientError> {
        self.pools
            .borrow_mut()
            .data
            .insert(pool.name.to_string(), pool);
        self.pool_refs.borrow_mut().reset();
        let inst = self.new_instruction_add_pool(&admin_signer.pubkey(), pool)?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Removes the Pool's on-chain metadata
    pub fn remove_pool(
        &self,
        admin_signer: &dyn Signer,
        pool_name: &str,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_remove_pool(&admin_signer.pubkey(), pool_name)?;
        self.pools.borrow_mut().data.remove(pool_name);
        self.pool_refs.borrow_mut().data.remove(pool_name);
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Records the Farm metadata on-chain
    pub fn add_farm(
        &self,
        admin_signer: &dyn Signer,
        farm: Farm,
    ) -> Result<Signature, FarmClientError> {
        self.farms
            .borrow_mut()
            .data
            .insert(farm.name.to_string(), farm);
        self.farm_refs.borrow_mut().reset();
        let inst = self.new_instruction_add_farm(&admin_signer.pubkey(), farm)?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Removes the Farm's on-chain metadata
    pub fn remove_farm(
        &self,
        admin_signer: &dyn Signer,
        farm_name: &str,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_remove_farm(&admin_signer.pubkey(), farm_name)?;
        self.farms.borrow_mut().data.remove(farm_name);
        self.farm_refs.borrow_mut().data.remove(farm_name);
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Records the Token metadata on-chain
    pub fn add_token(
        &self,
        admin_signer: &dyn Signer,
        token: Token,
    ) -> Result<Signature, FarmClientError> {
        self.tokens
            .borrow_mut()
            .data
            .insert(token.name.to_string(), token);
        self.token_refs.borrow_mut().reset();
        let inst = self.new_instruction_add_token(&admin_signer.pubkey(), token)?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Removes the Token's on-chain metadata
    pub fn remove_token(
        &self,
        admin_signer: &dyn Signer,
        token_name: &str,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_remove_token(&admin_signer.pubkey(), token_name)?;
        self.tokens.borrow_mut().data.remove(token_name);
        self.token_refs.borrow_mut().data.remove(token_name);
        self.sign_and_send_instructions(&[admin_signer], &[inst])
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
        for (vault_name, _) in vaults.iter() {
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
    pub fn disable_deposit_vault(
        &self,
        admin_signer: &dyn Signer,
        vault_name: &str,
    ) -> Result<Signature, FarmClientError> {
        let inst =
            self.new_instruction_disable_deposit_vault(&admin_signer.pubkey(), vault_name)?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Enables deposits to the Vault
    pub fn enable_deposit_vault(
        &self,
        admin_signer: &dyn Signer,
        vault_name: &str,
    ) -> Result<Signature, FarmClientError> {
        let inst = self.new_instruction_enable_deposit_vault(&admin_signer.pubkey(), vault_name)?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Disables withdrawal from the Vault
    pub fn disable_withdrawal_vault(
        &self,
        admin_signer: &dyn Signer,
        vault_name: &str,
    ) -> Result<Signature, FarmClientError> {
        let inst =
            self.new_instruction_disable_withdrawal_vault(&admin_signer.pubkey(), vault_name)?;
        self.sign_and_send_instructions(&[admin_signer], &[inst])
    }

    /// Enables withdrawals from the Vault
    pub fn enable_withdrawal_vault(
        &self,
        admin_signer: &dyn Signer,
        vault_name: &str,
    ) -> Result<Signature, FarmClientError> {
        let inst =
            self.new_instruction_enable_withdrawal_vault(&admin_signer.pubkey(), vault_name)?;
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

    /////////////// helpers
    pub fn ui_amount_to_tokens(
        &self,
        ui_amount: f64,
        token_name: &str,
    ) -> Result<u64, FarmClientError> {
        if ui_amount == 0.0 {
            return Ok(0);
        }
        let multiplier = 10usize.pow(self.get_token(token_name)?.decimals as u32);
        Ok((ui_amount * multiplier as f64).round() as u64)
    }

    pub fn tokens_to_ui_amount(
        &self,
        amount: u64,
        token_name: &str,
    ) -> Result<f64, FarmClientError> {
        if amount == 0 {
            return Ok(0.0);
        }
        let divisor = 10usize.pow(self.get_token(token_name)?.decimals as u32);
        Ok(amount as f64 / divisor as f64)
    }

    pub fn ui_amount_to_tokens_with_decimals(&self, ui_amount: f64, decimals: u8) -> u64 {
        if ui_amount == 0.0 {
            return 0;
        }
        let multiplier = 10usize.pow(decimals as u32);
        (ui_amount * multiplier as f64).round() as u64
    }

    pub fn tokens_to_ui_amount_with_decimals(&self, amount: u64, decimals: u8) -> f64 {
        if amount == 0 {
            return 0.0;
        }
        let divisor = 10usize.pow(decimals as u32);
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
        let token_a = self.get_token_by_ref_from_cache(&farm.reward_token_a_ref)?;
        let token_b = self.get_token_by_ref_from_cache(&farm.reward_token_b_ref)?;
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
            VaultStrategy::StakeLpCompoundRewards { pool_id_ref, .. } => {
                let pool = self.get_pool_by_ref(&pool_id_ref)?;
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

        self.sign_and_send_instructions(&[signer], inst.as_slice())
    }

    ////////////// private helpers
    fn to_token_amount(&self, ui_amount: f64, token: &Token) -> u64 {
        self.ui_amount_to_tokens_with_decimals(ui_amount, token.decimals)
    }

    fn to_token_amount_option(
        &self,
        ui_amount: f64,
        token: &Option<Token>,
    ) -> Result<u64, FarmClientError> {
        if let Some(tkn) = token {
            Ok(self.to_token_amount(ui_amount, tkn))
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

    fn extract_version(name: &str) -> Result<u16, FarmClientError> {
        if &name[..1].to_uppercase() == "V" {
            if let Ok(ver) = name[1..].parse::<u16>() {
                return Ok(ver);
            }
        }
        Err(FarmClientError::ProgramError(ProgramError::InvalidArgument))
    }

    fn extract_name_and_version(name: &str) -> Result<(String, u16), FarmClientError> {
        let dot_split = name.split('.').collect::<Vec<&str>>();
        if dot_split.len() < 2 || dot_split[0].is_empty() {
            return Err(FarmClientError::ProgramError(ProgramError::InvalidArgument));
        }
        let dash_split = dot_split.last().unwrap().split('-').collect::<Vec<&str>>();
        if dash_split.len() < 2 {
            return Err(FarmClientError::ProgramError(ProgramError::InvalidArgument));
        }
        let ver_string = dash_split.last().unwrap();
        let ver = FarmClient::extract_version(ver_string)?;
        Ok((name[..name.len() - ver_string.len() - 1].to_string(), ver))
    }

    // insert version-stripped names that point to the latest version
    fn reinsert_latest_versions(
        source: &HashMap<String, Pubkey>,
        dest: &mut HashMap<String, String>,
    ) {
        let mut latest = HashMap::<String, (String, u16)>::default();
        for (full_name, _) in source.iter() {
            if let Ok((name_no_ver, ver)) = FarmClient::extract_name_and_version(full_name) {
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

    fn reload_vault_refs_if_stale(&self) -> Result<bool, FarmClientError> {
        if self.vault_refs.borrow().is_stale() {
            let vault_refs = self.get_refdb_pubkey_map(&refdb::StorageType::Vault.to_string())?;
            FarmClient::reinsert_latest_versions(&vault_refs, &mut self.latest_vaults.borrow_mut());
            self.vault_refs.borrow_mut().set(vault_refs);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn reload_vaults_if_stale(&self) -> Result<bool, FarmClientError> {
        if self.vaults.borrow().is_stale() {
            let refs_map = &self.vault_refs.borrow().data;
            let refs: Vec<Pubkey> = refs_map.values().copied().collect();
            if refs.is_empty() {
                return Ok(false);
            }
            let mut vault_map = VaultMap::new();

            let mut idx = 0;
            while idx < refs.len() - 1 {
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

            self.vaults.borrow_mut().set(vault_map);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn reload_pool_refs_if_stale(&self) -> Result<bool, FarmClientError> {
        if self.pool_refs.borrow().is_stale() {
            let pool_refs = self.get_refdb_pubkey_map(&refdb::StorageType::Pool.to_string())?;
            FarmClient::reinsert_latest_versions(&pool_refs, &mut self.latest_pools.borrow_mut());
            self.pool_refs.borrow_mut().set(pool_refs);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn reload_pools_if_stale(&self) -> Result<bool, FarmClientError> {
        if self.pools.borrow().is_stale() {
            let refs_map = &self.pool_refs.borrow().data;
            let refs: Vec<Pubkey> = refs_map.values().copied().collect();
            if refs.is_empty() {
                return Ok(false);
            }
            let mut pool_map = PoolMap::new();

            let mut idx = 0;
            while idx < refs.len() - 1 {
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

            self.pools.borrow_mut().set(pool_map);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn reload_farm_refs_if_stale(&self) -> Result<bool, FarmClientError> {
        if self.farm_refs.borrow().is_stale() {
            let farm_refs = self.get_refdb_pubkey_map(&refdb::StorageType::Farm.to_string())?;
            FarmClient::reinsert_latest_versions(&farm_refs, &mut self.latest_farms.borrow_mut());
            self.farm_refs.borrow_mut().set(farm_refs);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn reload_farms_if_stale(&self) -> Result<bool, FarmClientError> {
        if self.farms.borrow().is_stale() {
            let refs_map = &self.farm_refs.borrow().data;
            let refs: Vec<Pubkey> = refs_map.values().copied().collect();
            if refs.is_empty() {
                return Ok(false);
            }
            let mut farm_map = FarmMap::new();

            let mut idx = 0;
            while idx < refs.len() - 1 {
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

            self.farms.borrow_mut().set(farm_map);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn reload_token_refs_if_stale(&self) -> Result<bool, FarmClientError> {
        if self.token_refs.borrow().is_stale() {
            let token_refs = self.get_refdb_pubkey_map(&refdb::StorageType::Token.to_string())?;
            self.token_refs.borrow_mut().set(token_refs);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn reload_tokens_if_stale(&self) -> Result<bool, FarmClientError> {
        if self.tokens.borrow().is_stale() {
            let refs_map = &self.token_refs.borrow().data;
            let refs: Vec<Pubkey> = refs_map.values().copied().collect();
            if refs.is_empty() {
                return Ok(false);
            }
            let mut token_map = TokenMap::new();

            let mut idx = 0;
            while idx < refs.len() - 1 {
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

            self.tokens.borrow_mut().set(token_map);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn reload_program_ids_if_stale(&self) -> Result<bool, FarmClientError> {
        if self.official_ids.borrow().is_stale() {
            let official_ids =
                self.get_refdb_pubkey_map(&refdb::StorageType::Program.to_string())?;
            self.official_ids.borrow_mut().set(official_ids);
            Ok(true)
        } else {
            Ok(false)
        }
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

    fn extract_token_names(name: &str) -> Result<(String, String, String), FarmClientError> {
        let dot_split = if name.starts_with("LP.") || name.starts_with("VT.") {
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
            dot_split[0].to_string(),
            dash_split[0].to_string(),
            if dash_split.len() > 1 && FarmClient::extract_version(dash_split[1]).is_err() {
                dash_split[1].to_string()
            } else {
                String::default()
            },
        ))
    }

    fn pool_has_reverse_tokens(pool_name: &str, token_a: &str) -> Result<bool, FarmClientError> {
        let (_, pool_token_a, _) = FarmClient::extract_token_names(pool_name)?;
        Ok(pool_token_a != token_a)
    }

    fn get_raydium_stake_account(
        &self,
        wallet_address: &Pubkey,
        farm: &Farm,
    ) -> Result<Option<Pubkey>, FarmClientError> {
        let farm_id = match farm.route {
            FarmRoute::Raydium { farm_id, .. } => farm_id,
            _ => unreachable!(),
        };
        // look-up in cache
        if let Some(addr_map) = self.stake_accounts.borrow()[0].get(&wallet_address.to_string()) {
            if let Some(stake_acc) = addr_map.get(&farm_id.to_string()) {
                return Ok(Some(*stake_acc));
            }
        }
        // search on-chain
        let filters = Some(vec![rpc_filter::RpcFilterType::Memcmp(
            rpc_filter::Memcmp {
                offset: 40,
                bytes: rpc_filter::MemcmpEncodedBytes::Base58(
                    bs58::encode(wallet_address).into_string(),
                ),
                encoding: Some(rpc_filter::MemcmpEncoding::Binary),
            },
        )]);
        let acc_vec = self.rpc_client.get_program_accounts_with_config(
            &farm.farm_program_id,
            RpcProgramAccountsConfig {
                filters,
                ..RpcProgramAccountsConfig::default()
            },
        )?;
        let user_acc_str = wallet_address.to_string();
        let stake_accounts_map = &mut self.stake_accounts.borrow_mut()[0];
        let mut user_acc_map = stake_accounts_map.get_mut(&user_acc_str);
        if user_acc_map.is_none() {
            stake_accounts_map.insert(user_acc_str.clone(), StakeAccMap::new());
            user_acc_map = stake_accounts_map.get_mut(&user_acc_str);
        }
        let user_acc_map = user_acc_map.unwrap();
        let target_farm_id_str = farm_id.to_string();
        let mut stake_acc = None;
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
            if farm_id_str == target_farm_id_str {
                stake_acc = Some(*stake_acc_key);
            }
        }
        Ok(stake_acc)
    }

    fn get_saber_stake_account(
        &self,
        wallet_address: &Pubkey,
        farm: &Farm,
    ) -> Result<Option<Pubkey>, FarmClientError> {
        let quarry = match farm.route {
            FarmRoute::Saber { quarry, .. } => quarry,
            _ => unreachable!(),
        };
        // look-up in cache
        if let Some(addr_map) = self.stake_accounts.borrow()[1].get(&wallet_address.to_string()) {
            if let Some(stake_acc) = addr_map.get(&quarry.to_string()) {
                return Ok(Some(*stake_acc));
            }
        }

        // update cache
        let user_acc_str = wallet_address.to_string();
        let stake_accounts_map = &mut self.stake_accounts.borrow_mut()[1];
        if stake_accounts_map.get(&user_acc_str).is_none() {
            stake_accounts_map.insert(user_acc_str, StakeAccMap::new());
        }

        // check if account exists on-chain
        let (miner, _) = Pubkey::find_program_address(
            &[b"Miner", &quarry.to_bytes(), &wallet_address.to_bytes()],
            &quarry_mine::id(),
        );
        if let Ok(data) = self.rpc_client.get_account_data(&miner) {
            if !data.is_empty() {
                return Ok(Some(miner));
            }
        }
        Ok(None)
    }

    fn get_orca_stake_account(
        &self,
        wallet_address: &Pubkey,
        farm: &Farm,
    ) -> Result<Option<Pubkey>, FarmClientError> {
        let farm_id = match farm.route {
            FarmRoute::Orca { farm_id, .. } => farm_id,
            _ => unreachable!(),
        };
        // look-up in cache
        if let Some(addr_map) = self.stake_accounts.borrow()[2].get(&wallet_address.to_string()) {
            if let Some(stake_acc) = addr_map.get(&farm_id.to_string()) {
                return Ok(Some(*stake_acc));
            }
        }

        // update cache
        let user_acc_str = wallet_address.to_string();
        let stake_accounts_map = &mut self.stake_accounts.borrow_mut()[2];
        if stake_accounts_map.get(&user_acc_str).is_none() {
            stake_accounts_map.insert(user_acc_str, StakeAccMap::new());
        }

        // check if account exists on-chain
        let farmer = Pubkey::find_program_address(
            &[
                &farm_id.to_bytes(),
                &wallet_address.to_bytes(),
                &spl_token::id().to_bytes(),
            ],
            &farm.farm_program_id,
        )
        .0;
        if let Ok(data) = self.rpc_client.get_account_data(&farmer) {
            if !data.is_empty() {
                return Ok(Some(farmer));
            }
        }
        Ok(None)
    }

    fn get_stake_account(
        &self,
        wallet_address: &Pubkey,
        farm_name: &str,
    ) -> Result<Option<Pubkey>, FarmClientError> {
        let farm = self.get_farm(farm_name)?;
        match farm.route {
            FarmRoute::Raydium { .. } => self.get_raydium_stake_account(wallet_address, &farm),
            FarmRoute::Saber { .. } => self.get_saber_stake_account(wallet_address, &farm),
            FarmRoute::Orca { .. } => self.get_orca_stake_account(wallet_address, &farm),
        }
    }

    fn create_raydium_stake_account(
        &self,
        wallet_address: &Pubkey,
        farm: &Farm,
        instruction_vec: &mut Vec<Instruction>,
        signers: &mut Vec<Box<dyn Signer>>,
    ) -> Result<(), FarmClientError> {
        let farm_id = match farm.route {
            FarmRoute::Raydium { farm_id, .. } => farm_id,
            _ => unreachable!(),
        };
        let new_keypair = Keypair::new();
        let new_pubkey = new_keypair.pubkey();
        let target_farm_id_str = farm_id.to_string();
        let user_acc_str = wallet_address.to_string();
        let stake_accounts_map = &mut self.stake_accounts.borrow_mut()[0];
        let user_acc_map = stake_accounts_map.get_mut(&user_acc_str).unwrap();
        user_acc_map.insert(target_farm_id_str, new_pubkey);
        signers.push(Box::new(new_keypair));
        instruction_vec.push(self.new_instruction_create_system_account(
            wallet_address,
            &new_pubkey,
            0,
            if farm.version >= 4 {
                RaydiumUserStakeInfoV4::LEN
            } else {
                RaydiumUserStakeInfo::LEN
            },
            &farm.farm_program_id,
        )?);
        Ok(())
    }

    fn create_saber_stake_account(
        &self,
        wallet_address: &Pubkey,
        farm: &Farm,
        instruction_vec: &mut Vec<Instruction>,
        _signers: &mut Vec<Box<dyn Signer>>,
    ) -> Result<(), FarmClientError> {
        let (quarry, rewarder) = match farm.route {
            FarmRoute::Saber {
                quarry, rewarder, ..
            } => (quarry, rewarder),
            _ => unreachable!(),
        };

        let lp_token = self.get_token_by_ref_from_cache(&farm.lp_token_ref)?;
        let lp_mint = lp_token.ok_or(ProgramError::UninitializedAccount)?.mint;

        let (miner, bump) = Pubkey::find_program_address(
            &[b"Miner", &quarry.to_bytes(), &wallet_address.to_bytes()],
            &quarry_mine::id(),
        );

        let miner_vault =
            spl_associated_token_account::get_associated_token_address(&miner, &lp_mint);

        let mut hasher = Hasher::default();
        hasher.hash(b"global:create_miner");

        let mut data = hasher.result().as_ref()[..8].to_vec();
        data.push(bump);

        let accounts = vec![
            AccountMeta::new_readonly(*wallet_address, true),
            AccountMeta::new(miner, false),
            AccountMeta::new(quarry, false),
            AccountMeta::new(rewarder, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(*wallet_address, true),
            AccountMeta::new(lp_mint, false),
            AccountMeta::new(miner_vault, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ];

        instruction_vec.push(Instruction {
            program_id: quarry_mine::id(),
            accounts,
            data,
        });

        // update cache
        let stake_accounts_map = &mut self.stake_accounts.borrow_mut()[1];
        let user_acc_map = stake_accounts_map
            .get_mut(&wallet_address.to_string())
            .unwrap();
        user_acc_map.insert(quarry.to_string(), miner);

        Ok(())
    }

    fn create_orca_stake_account(
        &self,
        wallet_address: &Pubkey,
        farm: &Farm,
        instruction_vec: &mut Vec<Instruction>,
        _signers: &mut Vec<Box<dyn Signer>>,
    ) -> Result<(), FarmClientError> {
        let farm_id = match farm.route {
            FarmRoute::Orca { farm_id, .. } => farm_id,
            _ => unreachable!(),
        };

        let farmer = Pubkey::find_program_address(
            &[
                &farm_id.to_bytes(),
                &wallet_address.to_bytes(),
                &spl_token::id().to_bytes(),
            ],
            &farm.farm_program_id,
        )
        .0;

        let orca_accounts = vec![
            AccountMeta::new_readonly(farm_id, false),
            AccountMeta::new(farmer, false),
            AccountMeta::new_readonly(*wallet_address, true),
            AccountMeta::new_readonly(system_program::id(), false),
        ];

        instruction_vec.push(Instruction {
            program_id: farm.farm_program_id,
            accounts: orca_accounts,
            data: OrcaUserInit {}.to_vec()?,
        });

        // update cache
        let stake_accounts_map = &mut self.stake_accounts.borrow_mut()[2];
        let user_acc_map = stake_accounts_map
            .get_mut(&wallet_address.to_string())
            .unwrap();
        user_acc_map.insert(farm_id.to_string(), farmer);

        Ok(())
    }

    fn check_user_stake_account(
        &self,
        wallet_address: &Pubkey,
        farm_name: &str,
        instruction_vec: &mut Vec<Instruction>,
        signers: &mut Vec<Box<dyn Signer>>,
    ) -> Result<(), FarmClientError> {
        // lookup in cache
        let farm = self.get_farm(farm_name)?;
        if self.get_stake_account(wallet_address, farm_name)?.is_some() {
            return Ok(());
        }
        // create new
        match farm.route {
            FarmRoute::Raydium { .. } => {
                self.create_raydium_stake_account(wallet_address, &farm, instruction_vec, signers)
            }
            FarmRoute::Saber { .. } => {
                self.create_saber_stake_account(wallet_address, &farm, instruction_vec, signers)
            }
            FarmRoute::Orca { .. } => {
                self.create_orca_stake_account(wallet_address, &farm, instruction_vec, signers)
            }
        }
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
        signer: &dyn Signer,
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
                &signer.pubkey(),
                &lp_token,
                ui_amount_lp_token,
                instruction_vec,
            )?;
        }
        let _ = self.check_token_account(
            &signer.pubkey(),
            &token_a,
            ui_amount_token_a,
            instruction_vec,
        )?;
        let _ = self.check_token_account(
            &signer.pubkey(),
            &token_b,
            ui_amount_token_b,
            instruction_vec,
        )?;

        if let PoolRoute::Saber {
            wrapped_token_a_ref,
            wrapped_token_b_ref,
            ..
        } = pool.route
        {
            if let Some(token) = self.get_token_by_ref_from_cache(&wrapped_token_a_ref)? {
                let _ = self.check_token_account_with_mint(
                    &signer.pubkey(),
                    &token.mint,
                    instruction_vec,
                )?;
            }
            if let Some(token) = self.get_token_by_ref_from_cache(&wrapped_token_b_ref)? {
                let _ = self.check_token_account_with_mint(
                    &signer.pubkey(),
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
        signer: &dyn Signer,
        farm_name: &str,
        ui_amount: f64,
        instruction_vec: &mut Vec<Instruction>,
        signers: &mut Vec<Box<dyn Signer>>,
    ) -> Result<(), FarmClientError> {
        let farm = self.get_farm(farm_name)?;
        let token_a = self.get_token_by_ref_from_cache(&farm.reward_token_a_ref)?;
        let token_b = self.get_token_by_ref_from_cache(&farm.reward_token_b_ref)?;
        let lp_token = self.get_token_by_ref_from_cache(&farm.lp_token_ref)?;

        let _ = self.check_token_account(&signer.pubkey(), &token_a, 0.0, instruction_vec)?;
        let _ = self.check_token_account(&signer.pubkey(), &token_b, 0.0, instruction_vec)?;
        let _ =
            self.check_token_account(&signer.pubkey(), &lp_token, ui_amount, instruction_vec)?;

        let _ =
            self.check_user_stake_account(&signer.pubkey(), farm_name, instruction_vec, signers)?;

        match farm.route {
            FarmRoute::Saber { .. } => {
                let user_info_account = self
                    .get_stake_account(&signer.pubkey(), farm_name)?
                    .unwrap();

                let user_vault_account = self
                    .get_token_account(&user_info_account, &lp_token)
                    .ok_or(ProgramError::UninitializedAccount)?;

                let data = self.rpc_client.get_account_data(&user_vault_account);
                if data.is_err() || data.unwrap().is_empty() {
                    instruction_vec.insert(
                        0,
                        create_associated_token_account(
                            &signer.pubkey(),
                            &user_info_account,
                            &lp_token.unwrap().mint,
                        ),
                    );
                }
            }
            FarmRoute::Orca { farm_token_ref, .. } => {
                let farm_lp_token = self.get_token_by_ref(&farm_token_ref)?;
                let user_farm_lp_token_account =
                    get_associated_token_address(&signer.pubkey(), &farm_lp_token.mint);
                let data = self
                    .rpc_client
                    .get_account_data(&user_farm_lp_token_account);
                if data.is_err() || data.unwrap().is_empty() {
                    instruction_vec.insert(
                        0,
                        create_associated_token_account(
                            &signer.pubkey(),
                            &signer.pubkey(),
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
        signer: &dyn Signer,
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
        let token_a_reward = self.get_token_by_ref_from_cache(&farm.reward_token_a_ref)?;
        let token_b_reward = self.get_token_by_ref_from_cache(&farm.reward_token_b_ref)?;

        if check_vt_token {
            let _ = self.check_token_account(
                &signer.pubkey(),
                &vault_token,
                ui_amount_vt_token,
                instruction_vec,
            )?;
        }
        if check_lp_token {
            let _ = self.check_token_account(&signer.pubkey(), &lp_token, 0.0, instruction_vec)?;
        }
        let _ =
            self.check_token_account(&signer.pubkey(), &token_a_reward, 0.0, instruction_vec)?;
        let _ =
            self.check_token_account(&signer.pubkey(), &token_b_reward, 0.0, instruction_vec)?;
        let _ = self.check_token_account(
            &signer.pubkey(),
            &token_a,
            ui_amount_token_a,
            instruction_vec,
        )?;
        let _ = self.check_token_account(
            &signer.pubkey(),
            &token_b,
            ui_amount_token_b,
            instruction_vec,
        )?;

        let user_info_account = self.get_vault_user_info_account(&signer.pubkey(), vault_name)?;
        let data = self.rpc_client.get_account_data(&user_info_account);
        if data.is_err() || !RefDB::is_initialized(data.unwrap().as_slice()) {
            instruction_vec
                .push(self.new_instruction_user_init_vault(&signer.pubkey(), vault_name)?);
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
            VaultStrategy::StakeLpCompoundRewards { pool_id_ref, .. } => {
                self.get_pool_by_ref(&pool_id_ref)
            }
            VaultStrategy::DynamicHedge { .. } => self.get_pool_by_ref(&zero::id()),
        }
    }

    // note: there could be multiple underlying farms in the future
    fn get_underlying_farm(&self, vault_name: &str) -> Result<Farm, FarmClientError> {
        let vault = self.get_vault(vault_name)?;
        match vault.strategy {
            VaultStrategy::StakeLpCompoundRewards { farm_id_ref, .. } => {
                self.get_farm_by_ref(&farm_id_ref)
            }
            VaultStrategy::DynamicHedge { .. } => self.get_farm_by_ref(&zero::id()),
        }
    }

    fn get_vault_price(&self, vault_name: &str) -> Result<f64, FarmClientError> {
        let pool_name = self.get_underlying_pool(vault_name)?.name.to_string();
        self.get_pool_price(&pool_name)
    }
}

mod farm_accounts_orca;
mod farm_accounts_raydium;
mod farm_accounts_saber;
mod farm_instructions;
mod governance_instructions;
mod pool_accounts_orca;
mod pool_accounts_raydium;
mod pool_accounts_saber;
mod pool_instructions;
mod refdb_instructions;
mod system_instructions;
mod vault_instructions;
mod vault_stc_accounts_raydium;
mod vault_stc_accounts_saber;

#[cfg(test)]
mod test {
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
}
