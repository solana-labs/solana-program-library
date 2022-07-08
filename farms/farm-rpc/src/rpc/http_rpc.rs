//! JSON RPC service

use {
    crate::{
        config::Config,
        fund_stats::{FundStats, FundStatsRecord},
    },
    rocket::{
        fairing::{AdHoc, Fairing, Info, Kind},
        form::{
            error::{Error, Errors},
            FromFormField, ValueField,
        },
        fs::{relative, FileServer},
        http::{ContentType, Header},
        request::{FromParam, Request},
        response,
        response::{status::NotFound, Responder, Response},
        serde::json::Json,
        Build, Rocket, State,
    },
    serde_json::{from_str, from_value, json, Value},
    solana_account_decoder::parse_token::UiTokenAccount,
    solana_farm_client::client::{
        FarmClient, FarmMap, FundMap, PoolMap, PubkeyMap, TokenMap, VaultMap,
    },
    solana_farm_sdk::{
        farm::Farm,
        fund::{
            Fund, FundAssets, FundCustody, FundCustodyWithBalance, FundInfo, FundSchedule,
            FundUserInfo, FundUserRequests, FundVault,
        },
        pool::Pool,
        program::multisig::Multisig,
        string::{instruction_to_string, pubkey_map_to_string},
        token::{GitToken, Token},
        vault::{Vault, VaultInfo, VaultUserInfo},
        ProtocolInfo,
    },
    solana_sdk::{
        commitment_config::CommitmentConfig, instruction::Instruction, pubkey::Pubkey,
        signature::Keypair,
    },
    std::{
        collections::HashMap,
        convert::Into,
        str::FromStr,
        sync::{Arc, Mutex},
    },
};

type Result<T, E = String> = std::result::Result<T, E>;
type GitTokens = HashMap<String, GitToken>;
type FarmClientArc = Arc<Mutex<FarmClient>>;
type Signature = String;

pub struct Cors;

#[rocket::async_trait]
impl Fairing for Cors {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "POST, GET, OPTIONS",
        ));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}

// Pubkey parameters handling
struct PubkeyParam {
    key: Pubkey,
}

impl<'r> FromParam<'r> for PubkeyParam {
    type Error = &'r str;
    fn from_param(param: &'r str) -> Result<Self, Self::Error> {
        Pubkey::from_str(param)
            .map(|value| PubkeyParam { key: value })
            .map_err(|_| "Failed to convert string parameter to Pubkey")
    }
}

impl<'r> FromFormField<'r> for PubkeyParam {
    fn from_value(field: ValueField<'r>) -> rocket::form::Result<'r, Self> {
        Pubkey::from_str(field.value)
            .map(|value| PubkeyParam { key: value })
            .map_err(|_| {
                Errors::from(Error::validation(
                    "Failed to convert string argument to Pubkey",
                ))
            })
    }
}

// Keypair parameters handling
struct KeypairParam {
    key: Keypair,
}

impl<'r> FromParam<'r> for KeypairParam {
    type Error = &'r str;
    fn from_param(param: &'r str) -> Result<Self, Self::Error> {
        let v = &bs58::decode(param)
            .into_vec()
            .map_err(|_| "Failed to convert parameter to Keypair")?;
        Keypair::from_bytes(v)
            .map(|value| KeypairParam { key: value })
            .map_err(|_| "Failed to convert parameter to Keypair")
    }
}

impl<'r> FromFormField<'r> for KeypairParam {
    fn from_value(field: ValueField<'r>) -> rocket::form::Result<'r, Self> {
        let v = &bs58::decode(field.value).into_vec().map_err(|_| {
            Errors::from(Error::validation(
                "Failed to convert string argument to Pubkey",
            ))
        })?;
        Keypair::from_bytes(v)
            .map(|value| KeypairParam { key: value })
            .map_err(|_| {
                Errors::from(Error::validation(
                    "Failed to convert string argument to Pubkey",
                ))
            })
    }
}

fn check_unwrap_pubkey(
    pubkey_param: Option<PubkeyParam>,
    param_name: &str,
) -> Result<Pubkey, NotFound<String>> {
    if let Some(pubkey) = pubkey_param {
        Ok(pubkey.key)
    } else {
        Err(NotFound(format!("Invalid {} argument", param_name)))
    }
}

fn check_unwrap_keypair(
    keypair_param: Option<KeypairParam>,
    param_name: &str,
) -> Result<Keypair, NotFound<String>> {
    if let Some(keypair) = keypair_param {
        Ok(keypair.key)
    } else {
        Err(NotFound(format!("Invalid {} argument", param_name)))
    }
}
// Custom Json responders
#[derive(Debug)]
struct JsonWithPubkeyMap {
    data: String,
}

impl JsonWithPubkeyMap {
    pub fn new(data: &PubkeyMap) -> Self {
        Self {
            data: pubkey_map_to_string(data),
        }
    }
}

impl<'r> Responder<'r, 'static> for JsonWithPubkeyMap {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'static> {
        Response::build()
            .merge(self.data.respond_to(request)?)
            .header(ContentType::JSON)
            .ok()
    }
}

#[derive(Debug)]
struct JsonWithInstruction {
    data: String,
}

impl JsonWithInstruction {
    pub fn new(data: &Instruction) -> Self {
        Self {
            data: instruction_to_string(data),
        }
    }
}

impl<'r> Responder<'r, 'static> for JsonWithInstruction {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'static> {
        Response::build()
            .merge(self.data.respond_to(request)?)
            .header(ContentType::JSON)
            .ok()
    }
}

#[derive(Debug)]
struct JsonWithInstructions {
    data: String,
}

impl JsonWithInstructions {
    pub fn new(data: &[Instruction]) -> Self {
        let mut res = <String as std::default::Default>::default();
        for inst in data {
            if res.is_empty() {
                res = "[".to_string() + &instruction_to_string(inst);
            } else {
                res += &(",".to_string() + &instruction_to_string(inst));
            }
        }
        Self { data: res + "]" }
    }
}

impl<'r> Responder<'r, 'static> for JsonWithInstructions {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'static> {
        Response::build()
            .merge(self.data.respond_to(request)?)
            .header(ContentType::JSON)
            .ok()
    }
}

// Routes

/// Returns description and stats of all supported protocols
#[get("/protocols")]
async fn get_protocols(
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Vec<ProtocolInfo>>, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let protocols = farm_client
        .get_protocols()
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(protocols))
}

/// Returns current admin signers for the Main Router
#[get("/admins")]
async fn get_admins(
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Multisig>, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let admins = farm_client
        .get_admins()
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(admins))
}

/// Returns program upgrade signers
#[get("/program_admins?<program_id>")]
async fn get_program_admins(
    program_id: Option<PubkeyParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Multisig>, NotFound<String>> {
    let program_id = check_unwrap_pubkey(program_id, "program_id")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let admins = farm_client
        .get_program_admins(&program_id)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(admins))
}

/// Returns Token metadata from Github
#[get("/git_token?<name>")]
async fn get_git_token(
    name: &str,
    git_tokens: &State<GitTokens>,
) -> Result<Json<GitToken>, NotFound<String>> {
    if !git_tokens.inner().contains_key(name) {
        return Err(NotFound(format!("Record not found: Token {}", name)));
    }
    Ok(Json(git_tokens.inner()[name].clone()))
}

/// Returns all Tokens from Github
#[get("/git_tokens")]
async fn get_git_tokens(git_tokens: &State<GitTokens>) -> Result<Json<GitTokens>> {
    Ok(Json(git_tokens.inner().clone()))
}

/// Returns the Fund struct for the given name
#[get("/fund?<name>")]
async fn get_fund(
    name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Fund>, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let fund = farm_client
        .get_fund(name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(fund))
}

/// Returns all Funds available
#[get("/funds")]
async fn get_funds(farm_client: &State<FarmClientArc>) -> Result<Json<FundMap>, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let funds = farm_client
        .get_funds()
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(funds))
}

/// Returns the Fund metadata address for the given name
#[get("/fund_ref?<name>")]
async fn get_fund_ref(
    name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<String, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let fund_ref = farm_client
        .get_fund_ref(name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(fund_ref.to_string())
}

/// Returns Fund refs: a map of Fund name to account address with metadata
#[get("/fund_refs")]
async fn get_fund_refs(
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithPubkeyMap, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let fund_refs = farm_client
        .get_fund_refs()
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithPubkeyMap::new(&fund_refs))
}

/// Returns the Fund metadata at the specified address
#[get("/fund_by_ref?<fund_ref>")]
async fn get_fund_by_ref(
    fund_ref: Option<PubkeyParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Fund>, NotFound<String>> {
    let fund_ref = check_unwrap_pubkey(fund_ref, "fund_ref")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let fund = farm_client
        .get_fund_by_ref(&fund_ref)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(fund))
}

/// Returns the Fund name for the given metadata address
#[get("/fund_name?<fund_ref>")]
async fn get_fund_name(
    fund_ref: Option<PubkeyParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<String, NotFound<String>> {
    let fund_ref = check_unwrap_pubkey(fund_ref, "fund_ref")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let fund_name = farm_client
        .get_fund_name(&fund_ref)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(fund_name)
}

/// Returns all Funds that have Vaults with the name matching the pattern sorted by version
#[get("/find_funds?<vault_name_pattern>")]
async fn find_funds(
    vault_name_pattern: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Vec<Fund>>, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let funds = farm_client
        .find_funds(vault_name_pattern)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(funds))
}

/// Returns the Vault struct for the given name
#[get("/vault?<name>")]
async fn get_vault(
    name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Vault>, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let vault = farm_client
        .get_vault(name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(vault))
}

/// Returns all Vaults available
#[get("/vaults")]
async fn get_vaults(
    farm_client: &State<FarmClientArc>,
) -> Result<Json<VaultMap>, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let vaults = farm_client
        .get_vaults()
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(vaults))
}

/// Returns the Vault metadata address for the given name
#[get("/vault_ref?<name>")]
async fn get_vault_ref(
    name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<String, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let vault_ref = farm_client
        .get_vault_ref(name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(vault_ref.to_string())
}

/// Returns Vault refs: a map of Vault name to account address with metadata
#[get("/vault_refs")]
async fn get_vault_refs(
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithPubkeyMap, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let vault_refs = farm_client
        .get_vault_refs()
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithPubkeyMap::new(&vault_refs))
}

/// Returns the Vault metadata at the specified address
#[get("/vault_by_ref?<vault_ref>")]
async fn get_vault_by_ref(
    vault_ref: Option<PubkeyParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Vault>, NotFound<String>> {
    let vault_ref = check_unwrap_pubkey(vault_ref, "vault_ref")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let vault = farm_client
        .get_vault_by_ref(&vault_ref)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(vault))
}

/// Returns the Vault name for the given metadata address
#[get("/vault_name?<vault_ref>")]
async fn get_vault_name(
    vault_ref: Option<PubkeyParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<String, NotFound<String>> {
    let vault_ref = check_unwrap_pubkey(vault_ref, "vault_ref")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let vault_name = farm_client
        .get_vault_name(&vault_ref)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(vault_name)
}

/// Returns all Vaults with tokens A and B sorted by version
#[get("/find_vaults?<token_a>&<token_b>")]
async fn find_vaults(
    token_a: &str,
    token_b: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Vec<Vault>>, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let vaults = farm_client
        .find_vaults(token_a, token_b)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(vaults))
}

/// Returns all Vaults with tokens A and B sorted by version
#[get("/find_vaults_with_vt?<vt_token_name>")]
async fn find_vaults_with_vt(
    vt_token_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Vec<Vault>>, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let vaults = farm_client
        .find_vaults_with_vt(vt_token_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(vaults))
}

/// Returns the Pool struct for the given name
#[get("/pool?<name>")]
async fn get_pool(
    name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Pool>, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let pool = farm_client
        .get_pool(name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(pool))
}

/// Returns all Pools available
#[get("/pools")]
async fn get_pools(farm_client: &State<FarmClientArc>) -> Result<Json<PoolMap>, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let pool_map = farm_client
        .get_pools()
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(pool_map))
}

/// Returns the Pool metadata address for the given name
#[get("/pool_ref?<name>")]
async fn get_pool_ref(
    name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<String, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let pool_ref = farm_client
        .get_pool_ref(name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(pool_ref.to_string())
}

/// Returns Pool refs: a map of Pool name to account address with metadata
#[get("/pool_refs")]
async fn get_pool_refs(
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithPubkeyMap, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let pool_refs = farm_client
        .get_pool_refs()
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithPubkeyMap::new(&pool_refs))
}

/// Returns the Pool metadata at the specified address
#[get("/pool_by_ref?<pool_ref>")]
async fn get_pool_by_ref(
    pool_ref: Option<PubkeyParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Pool>, NotFound<String>> {
    let pool_ref = check_unwrap_pubkey(pool_ref, "pool_ref")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let pool = farm_client
        .get_pool_by_ref(&pool_ref)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(pool))
}

/// Returns the Pool name for the given metadata address
#[get("/pool_name?<pool_ref>")]
async fn get_pool_name(
    pool_ref: Option<PubkeyParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<String, NotFound<String>> {
    let pool_ref = check_unwrap_pubkey(pool_ref, "pool_ref")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let pool_name = farm_client
        .get_pool_name(&pool_ref)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(pool_name)
}

/// Returns all Pools with tokens A and B sorted by version for the given protocol
#[get("/find_pools?<protocol>&<token_a>&<token_b>")]
async fn find_pools(
    protocol: &str,
    token_a: &str,
    token_b: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Vec<Pool>>, NotFound<String>> {
    let protocol = protocol
        .parse()
        .map_err(|_| NotFound("Invalid protocol argument".to_string()))?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let pools = farm_client
        .find_pools(protocol, token_a, token_b)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(pools))
}

/// Returns all Pools sorted by version for the given LP token
#[get("/find_pools_with_lp?<lp_token>")]
async fn find_pools_with_lp(
    lp_token: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Vec<Pool>>, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let pools = farm_client
        .find_pools_with_lp(lp_token)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(pools))
}

/// Returns pair's price based on the ratio of tokens in the pool
#[get("/pool_price?<name>")]
async fn get_pool_price(
    name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<String, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let pool_price = farm_client
        .get_pool_price(name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(pool_price.to_string())
}

/// Returns oracle address for the given token
#[get("/oracle?<symbol>")]
async fn get_oracle(
    symbol: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Pubkey>, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let oracle = farm_client
        .get_oracle(symbol)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(oracle.1.ok_or_else(|| {
        NotFound(format!("Oracle for {} is not configured", symbol))
    })?))
}

/// Returns the price in USD for the given token
#[get("/oracle_price?<symbol>&<max_price_age_sec>&<max_price_error>")]
async fn get_oracle_price(
    symbol: &str,
    max_price_age_sec: u64,
    max_price_error: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<String, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let oracle_price = farm_client
        .get_oracle_price(symbol, max_price_age_sec, max_price_error)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(oracle_price.to_string())
}

/// Returns the Farm struct for the given name
#[get("/farm?<name>")]
async fn get_farm(
    name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Farm>, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let farm = farm_client
        .get_farm(name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(farm))
}

/// Returns all Farms available
#[get("/farms")]
async fn get_farms(farm_client: &State<FarmClientArc>) -> Result<Json<FarmMap>, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let farms = farm_client
        .get_farms()
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(farms))
}

/// Returns the Farm metadata address for the given name
#[get("/farm_ref?<name>")]
async fn get_farm_ref(
    name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<String, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let farm_ref = farm_client
        .get_farm_ref(name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(farm_ref.to_string())
}

/// Returns Farm refs: a map of Farm name to account address with metadata
#[get("/farm_refs")]
async fn get_farm_refs(
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithPubkeyMap, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let farm_refs = farm_client
        .get_farm_refs()
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithPubkeyMap::new(&farm_refs))
}

/// Returns the Farm metadata at the specified address
#[get("/farm_by_ref?<farm_ref>")]
async fn get_farm_by_ref(
    farm_ref: Option<PubkeyParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Farm>, NotFound<String>> {
    let farm_ref = check_unwrap_pubkey(farm_ref, "farm_ref")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let farm = farm_client
        .get_farm_by_ref(&farm_ref)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(farm))
}

/// Returns the Farm name for the given metadata address
#[get("/farm_name?<farm_ref>")]
async fn get_farm_name(
    farm_ref: Option<PubkeyParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<String, NotFound<String>> {
    let farm_ref = check_unwrap_pubkey(farm_ref, "farm_ref")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let farm_name = farm_client
        .get_farm_name(&farm_ref)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(farm_name)
}

/// Returns all Farms for the given LP token
#[get("/find_farms_with_lp?<lp_token>")]
async fn find_farms_with_lp(
    lp_token: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Vec<Farm>>, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let farms = farm_client
        .find_farms_with_lp(lp_token)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(farms))
}

/// Returns the Token struct for the given name
#[get("/token?<name>")]
async fn get_token(
    name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Token>, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let token = farm_client
        .get_token(name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(token))
}

/// Returns all Tokens available
#[get("/tokens")]
async fn get_tokens(
    farm_client: &State<FarmClientArc>,
) -> Result<Json<TokenMap>, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let token = farm_client
        .get_tokens()
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(token))
}

/// Returns the Token metadata address for the given name
#[get("/token_ref?<name>")]
async fn get_token_ref(
    name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<String, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let token_ref = farm_client
        .get_token_ref(name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(token_ref.to_string())
}

/// Returns Token refs: a map of Token name to account address with metadata
#[get("/token_refs")]
async fn get_token_refs(
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithPubkeyMap, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let token_refs = farm_client
        .get_token_refs()
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithPubkeyMap::new(&token_refs))
}

/// Returns the Token metadata at the specified address
#[get("/token_by_ref?<token_ref>")]
async fn get_token_by_ref(
    token_ref: Option<PubkeyParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Token>, NotFound<String>> {
    let token_ref = check_unwrap_pubkey(token_ref, "token_ref")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let token = farm_client
        .get_token_by_ref(&token_ref)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(token))
}

/// Returns the Token name for the given metadata address
#[get("/token_name?<token_ref>")]
async fn get_token_name(
    token_ref: Option<PubkeyParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<String, NotFound<String>> {
    let token_ref = check_unwrap_pubkey(token_ref, "token_ref")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let token_name = farm_client
        .get_token_name(&token_ref)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(token_name)
}

/// Returns the Token metadata for the specified mint
#[get("/token_with_mint?<token_mint>")]
async fn get_token_with_mint(
    token_mint: Option<PubkeyParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Token>, NotFound<String>> {
    let token_mint = check_unwrap_pubkey(token_mint, "token_mint")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let token = farm_client
        .get_token_with_mint(&token_mint)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(token))
}

/// Returns the Token metadata for the specified token account
#[get("/token_with_account?<token_account>")]
async fn get_token_with_account(
    token_account: Option<PubkeyParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Token>, NotFound<String>> {
    let token_account = check_unwrap_pubkey(token_account, "token_account")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let token = farm_client
        .get_token_with_account(&token_account)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(token))
}

/// Returns the official Program ID for the given name
#[get("/program_id?<name>")]
async fn get_program_id(
    name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<String, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let program_id = farm_client
        .get_program_id(name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(program_id.to_string())
}

/// Returns all official Program IDs available
#[get("/program_ids")]
async fn get_program_ids(
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithPubkeyMap, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let program_ids = farm_client
        .get_program_ids()
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithPubkeyMap::new(&program_ids))
}

/// Returns the official program name for the given Program ID
#[get("/program_name?<prog_id>")]
async fn get_program_name(
    prog_id: Option<PubkeyParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<String, NotFound<String>> {
    let prog_id = check_unwrap_pubkey(prog_id, "prog_id")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let program_name = farm_client
        .get_program_name(&prog_id)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(program_name)
}

/// Checks if the given address is the official Program ID
#[get("/is_official_id?<prog_id>")]
async fn is_official_id(
    prog_id: Option<PubkeyParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<bool>, NotFound<String>> {
    let prog_id = check_unwrap_pubkey(prog_id, "prog_id")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let is_official = farm_client
        .is_official_id(&prog_id)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(is_official))
}

/// Checks if the given address is the Fund manager
#[get("/is_fund_manager?<wallet_address>")]
async fn is_fund_manager(
    wallet_address: Option<PubkeyParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<bool>, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let is_fund_manager = farm_client
        .is_fund_manager(&wallet_address)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(is_fund_manager))
}

/// Returns all Funds managed by the given address
#[get("/managed_funds?<wallet_address>")]
async fn get_managed_funds(
    wallet_address: Option<PubkeyParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Vec<Fund>>, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let funds = farm_client
        .get_funds()
        .map_err(|e| NotFound(e.to_string()))?
        .values()
        .filter(|&f| f.fund_manager == wallet_address)
        .copied()
        .collect::<Vec<Fund>>();

    Ok(Json(funds))
}

/// Creates a new system account
#[post("/create_system_account?<wallet_keypair>&<new_account_keypair>&<lamports>&<space>&<owner>")]
async fn create_system_account(
    wallet_keypair: Option<KeypairParam>,
    new_account_keypair: Option<KeypairParam>,
    lamports: u64,
    space: usize,
    owner: Option<PubkeyParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let new_account_keypair = check_unwrap_keypair(new_account_keypair, "new_account_keypair")?;
    let owner = check_unwrap_pubkey(owner, "owner")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .create_system_account(
            &wallet_keypair,
            &new_account_keypair,
            lamports,
            space,
            &owner,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Creates a new system account with seed
#[post("/create_system_account_with_seed?<wallet_keypair>&<base_address>&<seed>&<lamports>&<space>&<owner>")]
async fn create_system_account_with_seed(
    wallet_keypair: Option<KeypairParam>,
    base_address: Option<PubkeyParam>,
    seed: &str,
    lamports: u64,
    space: usize,
    owner: Option<PubkeyParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let base_address = check_unwrap_pubkey(base_address, "base_address")?;
    let owner = check_unwrap_pubkey(owner, "owner")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .create_system_account_with_seed(
            &wallet_keypair,
            &base_address,
            seed,
            lamports,
            space,
            &owner,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Assigns system account to a program
#[post("/assign_system_account?<wallet_keypair>&<program_address>")]
async fn assign_system_account(
    wallet_keypair: Option<KeypairParam>,
    program_address: Option<PubkeyParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let program_address = check_unwrap_pubkey(program_address, "program_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .assign_system_account(&wallet_keypair, &program_address)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Closes existing system account
#[post("/close_system_account?<wallet_keypair>&<target_account_keypair>")]
async fn close_system_account(
    wallet_keypair: Option<KeypairParam>,
    target_account_keypair: Option<KeypairParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let target_account_keypair =
        check_unwrap_keypair(target_account_keypair, "target_account_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .close_system_account(&wallet_keypair, &target_account_keypair)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Transfers native SOL from the wallet to the destination
#[post("/transfer?<wallet_keypair>&<destination_wallet>&<sol_ui_amount>")]
async fn transfer(
    wallet_keypair: Option<KeypairParam>,
    destination_wallet: Option<PubkeyParam>,
    sol_ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let destination_wallet = check_unwrap_pubkey(destination_wallet, "destination_wallet")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .transfer(&wallet_keypair, &destination_wallet, sol_ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Transfers tokens from the wallet to the destination
#[post("/token_transfer?<wallet_keypair>&<token_name>&<destination_wallet>&<ui_amount>")]
async fn token_transfer(
    wallet_keypair: Option<KeypairParam>,
    token_name: &str,
    destination_wallet: Option<PubkeyParam>,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let destination_wallet = check_unwrap_pubkey(destination_wallet, "destination_wallet")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .token_transfer(&wallet_keypair, token_name, &destination_wallet, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Transfers native SOL from the wallet to the associated Wrapped SOL account
#[post("/wrap_sol?<wallet_keypair>&<ui_amount>")]
async fn wrap_sol(
    wallet_keypair: Option<KeypairParam>,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .wrap_sol(&wallet_keypair, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Transfers Wrapped SOL back to SOL by closing the associated Wrapped SOL account
#[post("/unwrap_sol?<wallet_keypair>")]
async fn unwrap_sol(
    wallet_keypair: Option<KeypairParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .unwrap_sol(&wallet_keypair)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Updates token balance of the account, usefull after transfer SOL to WSOL account
#[post("/sync_token_balance?<wallet_keypair>&<token_name>")]
async fn sync_token_balance(
    wallet_keypair: Option<KeypairParam>,
    token_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .sync_token_balance(&wallet_keypair, token_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Returns the associated token account for the given user's main account or creates one
#[post("/create_token_account?<wallet_keypair>&<token_name>")]
async fn get_or_create_token_account(
    wallet_keypair: Option<KeypairParam>,
    token_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .get_or_create_token_account(&wallet_keypair, token_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Closes existing token account associated with the given user's main account
#[post("/close_token_account?<wallet_keypair>&<token_name>")]
async fn close_token_account(
    wallet_keypair: Option<KeypairParam>,
    token_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .close_token_account(&wallet_keypair, token_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Returns token supply as UI amount
#[get("/token_supply?<token_name>")]
async fn get_token_supply(
    token_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<String, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let token_supply = farm_client
        .get_token_supply(token_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(token_supply.to_string())
}

/// Returns the associated token account address for the given token name
#[get("/associated_token_address?<wallet_address>&<token_name>")]
async fn get_associated_token_address(
    wallet_address: Option<PubkeyParam>,
    token_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<String, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let token_address = farm_client
        .get_associated_token_address(&wallet_address, token_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(token_address.to_string())
}

/// Returns all tokens with active account in the wallet
#[get("/wallet_tokens?<wallet_address>")]
async fn get_wallet_tokens(
    wallet_address: Option<PubkeyParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Vec<String>>, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let tokens = farm_client
        .get_wallet_tokens(&wallet_address)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(tokens))
}

/// Returns UiTokenAccount struct data for the associated token account address
#[get("/token_account_data?<wallet_address>&<token_name>")]
async fn get_token_account_data(
    wallet_address: Option<PubkeyParam>,
    token_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<UiTokenAccount>, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let token_data = farm_client
        .get_token_account_data(&wallet_address, token_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(token_data))
}

/// Returns native SOL balance
#[get("/account_balance?<wallet_address>")]
async fn get_account_balance(
    wallet_address: Option<PubkeyParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<String, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let balance = farm_client
        .get_account_balance(&wallet_address)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(balance.to_string())
}

/// Returns token balance for the associated token account address
#[get("/token_account_balance?<wallet_address>&<token_name>")]
async fn get_token_account_balance(
    wallet_address: Option<PubkeyParam>,
    token_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<String, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let token_balance = farm_client
        .get_token_account_balance(&wallet_address, token_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(token_balance.to_string())
}

/// Returns token balance for the specified token account address
#[get("/token_account_balance_with_address?<token_account>")]
async fn get_token_account_balance_with_address(
    token_account: Option<PubkeyParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<String, NotFound<String>> {
    let token_account = check_unwrap_pubkey(token_account, "token_account")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let token_balance = farm_client
        .get_token_account_balance_with_address(&token_account)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(token_balance.to_string())
}

/// Returns true if the associated token account exists and is initialized
#[get("/has_active_token_account?<wallet_address>&<token_name>")]
async fn has_active_token_account(
    wallet_address: Option<PubkeyParam>,
    token_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<bool>, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let has_active_account = farm_client.has_active_token_account(&wallet_address, token_name);

    Ok(Json(has_active_account))
}

/// Returns current admin signers for the Fund
#[get("/fund_admins?<name>")]
async fn get_fund_admins(
    name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Multisig>, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let admins = farm_client
        .get_fund_admins(name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(admins))
}

/// Returns user stats for specific Fund
#[get("/fund_user_info?<wallet_address>&<fund_name>")]
async fn get_fund_user_info(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<FundUserInfo>, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let user_info = farm_client
        .get_fund_user_info(&wallet_address, fund_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(user_info))
}

/// Returns user stats for all Funds
#[get("/all_fund_user_infos?<wallet_address>")]
async fn get_all_fund_user_infos(
    wallet_address: Option<PubkeyParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Vec<FundUserInfo>>, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let user_infos = farm_client
        .get_all_fund_user_infos(&wallet_address)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(user_infos))
}

/// Returns user requests for specific Fund and token
#[get("/fund_user_requests?<wallet_address>&<fund_name>&<token_name>")]
async fn get_fund_user_requests(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    token_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<FundUserRequests>, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let user_requests = farm_client
        .get_fund_user_requests(&wallet_address, fund_name, token_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(user_requests))
}

/// Returns user requests for all tokens accepted by the Fund
#[get("/all_fund_user_requests?<fund_name>")]
async fn get_all_fund_user_requests(
    fund_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Vec<FundUserRequests>>, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let user_requests = farm_client
        .get_all_fund_user_requests(fund_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(user_requests))
}

/// Returns Fund stats and config
#[get("/fund_info?<fund_name>")]
async fn get_fund_info(
    fund_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<FundInfo>, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let fund_info = farm_client
        .get_fund_info(fund_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(fund_info))
}

/// Returns Fund info and config for all Funds
#[get("/all_fund_infos")]
async fn get_all_fund_infos(
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Vec<FundInfo>>, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let fund_infos = farm_client
        .get_all_fund_infos()
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(fund_infos))
}

/// Returns the Fund assets info
#[get("/fund_assets?<fund_name>&<asset_type>")]
async fn get_fund_assets(
    fund_name: &str,
    asset_type: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<FundAssets>, NotFound<String>> {
    let asset_type = asset_type
        .parse()
        .map_err(|_| NotFound("Invalid asset_type argument".to_string()))?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let fund_assets = farm_client
        .get_fund_assets(fund_name, asset_type)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(fund_assets))
}

/// Returns the Fund custody info
#[get("/fund_custody?<fund_name>&<token_name>&<custody_type>")]
async fn get_fund_custody(
    fund_name: &str,
    token_name: &str,
    custody_type: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<FundCustody>, NotFound<String>> {
    let custody_type = custody_type
        .parse()
        .map_err(|_| NotFound("Invalid custody_type argument".to_string()))?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let fund_custody = farm_client
        .get_fund_custody(fund_name, token_name, custody_type)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(fund_custody))
}

/// Returns the Fund custody extended info
#[get("/fund_custody_with_balance?<fund_name>&<token_name>&<custody_type>")]
async fn get_fund_custody_with_balance(
    fund_name: &str,
    token_name: &str,
    custody_type: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<FundCustodyWithBalance>, NotFound<String>> {
    let custody_type = custody_type
        .parse()
        .map_err(|_| NotFound("Invalid custody_type argument".to_string()))?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let fund_custody = farm_client
        .get_fund_custody_with_balance(fund_name, token_name, custody_type)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(fund_custody))
}

/// Returns all custodies belonging to the Fund sorted by custody_id
#[get("/fund_custodies?<fund_name>")]
async fn get_fund_custodies(
    fund_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Vec<FundCustody>>, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let fund_custodies = farm_client
        .get_fund_custodies(fund_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(fund_custodies))
}

/// Returns all custodies belonging to the Fund with extended info
#[get("/fund_custodies_with_balance?<fund_name>")]
async fn get_fund_custodies_with_balance(
    fund_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Vec<FundCustodyWithBalance>>, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let fund_custodies = farm_client
        .get_fund_custodies_with_balance(fund_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(fund_custodies))
}

/// Returns the Fund Vault info
#[get("/fund_vault?<fund_name>&<vault_name>&<vault_type>")]
async fn get_fund_vault(
    fund_name: &str,
    vault_name: &str,
    vault_type: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<FundVault>, NotFound<String>> {
    let vault_type = vault_type
        .parse()
        .map_err(|_| NotFound("Invalid vault_type argument".to_string()))?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let fund_vault = farm_client
        .get_fund_vault(fund_name, vault_name, vault_type)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(fund_vault))
}

/// Returns all Vaults belonging to the Fund sorted by vault_id
#[get("/fund_vaults?<fund_name>")]
async fn get_fund_vaults(
    fund_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Vec<FundVault>>, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let fund_vaults = farm_client
        .get_fund_vaults(fund_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(fund_vaults))
}

/// Returns Fund's historical performance
#[get("/fund_stats?<fund_name>&<timeframe>&<start_time>&<limit>")]
async fn get_fund_stats(
    fund_name: &str,
    timeframe: &str,
    start_time: i64,
    limit: u32,
    fund_stats: &State<Arc<Mutex<FundStats>>>,
) -> Result<Json<Vec<FundStatsRecord>>, NotFound<String>> {
    let timeframe = timeframe
        .parse()
        .map_err(|_| NotFound("Invalid timeframe argument".to_string()))?;
    let fund_stats = fund_stats
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let data = fund_stats
        .select(fund_name, timeframe, start_time, limit)
        .map_err(NotFound)?;

    Ok(Json(data))
}

/// Returns User's stacked balance
#[get("/user_stake_balance?<wallet_address>&<farm_name>")]
async fn get_user_stake_balance(
    wallet_address: Option<PubkeyParam>,
    farm_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<String, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let balance = farm_client
        .get_user_stake_balance(&wallet_address, farm_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(balance.to_string())
}

/// Returns Vault's stacked balance
#[get("/vault_stake_balance?<vault_name>")]
async fn get_vault_stake_balance(
    vault_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<String, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let balance = farm_client
        .get_vault_stake_balance(vault_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(balance.to_string())
}

/// Returns current admin signers for the Vault
#[get("/vault_admins?<name>")]
async fn get_vault_admins(
    name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Multisig>, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let admins = farm_client
        .get_vault_admins(name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(admins))
}

/// Returns user stats for specific Vault
#[get("/vault_user_info?<wallet_address>&<vault_name>")]
async fn get_vault_user_info(
    wallet_address: Option<PubkeyParam>,
    vault_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<VaultUserInfo>, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let user_info = farm_client
        .get_vault_user_info(&wallet_address, vault_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(user_info))
}

/// Returns Vault stats
#[get("/vault_info?<vault_name>")]
async fn get_vault_info(
    vault_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<VaultInfo>, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let vault_info = farm_client
        .get_vault_info(vault_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(vault_info))
}

/// Returns Vault stats for all Vaults
#[get("/all_vault_infos")]
async fn get_all_vault_infos(
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Vec<VaultInfo>>, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let vault_infos = farm_client
        .get_all_vault_infos()
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(vault_infos))
}

/// Returns number of decimal digits of the Vault token
#[get("/vault_token_decimals?<vault_name>")]
async fn get_vault_token_decimals(
    vault_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<String, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let decimals = farm_client
        .get_vault_token_decimals(vault_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(decimals.to_string())
}

/// Returns number of decimal digits of the Vault token
#[get("/pool_tokens_decimals?<pool_name>")]
async fn get_pool_tokens_decimals(
    pool_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Json<Vec<u8>>, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let decimals = farm_client
        .get_pool_tokens_decimals(pool_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(Json(decimals))
}

/// Initializes a new User for the Vault
#[post("/user_init_vault?<wallet_keypair>&<vault_name>")]
async fn user_init_vault(
    wallet_keypair: Option<KeypairParam>,
    vault_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .user_init_vault(&wallet_keypair, vault_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Adds liquidity to the Vault
#[post("/add_liquidity_vault?<wallet_keypair>&<vault_name>&<max_token_a_ui_amount>&<max_token_b_ui_amount>")]
async fn add_liquidity_vault(
    wallet_keypair: Option<KeypairParam>,
    vault_name: &str,
    max_token_a_ui_amount: f64,
    max_token_b_ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .add_liquidity_vault(
            &wallet_keypair,
            vault_name,
            max_token_a_ui_amount,
            max_token_b_ui_amount,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Adds locked liquidity to the Vault
#[post("/add_locked_liquidity_vault?<wallet_keypair>&<vault_name>&<ui_amount>")]
async fn add_locked_liquidity_vault(
    wallet_keypair: Option<KeypairParam>,
    vault_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .add_locked_liquidity_vault(&wallet_keypair, vault_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Removes liquidity from the Vault
#[post("/remove_liquidity_vault?<wallet_keypair>&<vault_name>&<ui_amount>")]
async fn remove_liquidity_vault(
    wallet_keypair: Option<KeypairParam>,
    vault_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .remove_liquidity_vault(&wallet_keypair, vault_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Removes unlocked liquidity from the Vault
#[post("/remove_unlocked_liquidity_vault?<wallet_keypair>&<vault_name>&<ui_amount>")]
async fn remove_unlocked_liquidity_vault(
    wallet_keypair: Option<KeypairParam>,
    vault_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .remove_unlocked_liquidity_vault(&wallet_keypair, vault_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Adds liquidity to the Pool
#[post("/add_liquidity_pool?<wallet_keypair>&<pool_name>&<max_token_a_ui_amount>&<max_token_b_ui_amount>")]
async fn add_liquidity_pool(
    wallet_keypair: Option<KeypairParam>,
    pool_name: &str,
    max_token_a_ui_amount: f64,
    max_token_b_ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .add_liquidity_pool(
            &wallet_keypair,
            pool_name,
            max_token_a_ui_amount,
            max_token_b_ui_amount,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Removes liquidity from the Pool
#[post("/remove_liquidity_pool?<wallet_keypair>&<pool_name>&<ui_amount>")]
async fn remove_liquidity_pool(
    wallet_keypair: Option<KeypairParam>,
    pool_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .remove_liquidity_pool(&wallet_keypair, pool_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Swaps tokens
#[post(
    "/swap?<wallet_keypair>&<protocol>&<from_token>&<to_token>&<ui_amount_in>&<min_ui_amount_out>"
)]
async fn swap(
    wallet_keypair: Option<KeypairParam>,
    protocol: &str,
    from_token: &str,
    to_token: &str,
    ui_amount_in: f64,
    min_ui_amount_out: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let protocol = protocol
        .parse()
        .map_err(|_| NotFound("Invalid protocol argument".to_string()))?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .swap(
            &wallet_keypair,
            protocol,
            from_token,
            to_token,
            ui_amount_in,
            min_ui_amount_out,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Initializes a new User for the Farm
#[post("/user_init?<wallet_keypair>&<farm_name>")]
async fn user_init(
    wallet_keypair: Option<KeypairParam>,
    farm_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .user_init(&wallet_keypair, farm_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Stakes tokens to the Farm
#[post("/stake?<wallet_keypair>&<farm_name>&<ui_amount>")]
async fn stake(
    wallet_keypair: Option<KeypairParam>,
    farm_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .stake(&wallet_keypair, farm_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Unstakes tokens from the Farm
#[post("/unstake?<wallet_keypair>&<farm_name>&<ui_amount>")]
async fn unstake(
    wallet_keypair: Option<KeypairParam>,
    farm_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .unstake(&wallet_keypair, farm_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Harvests rewards from the Farm
#[post("/harvest?<wallet_keypair>&<farm_name>")]
async fn harvest(
    wallet_keypair: Option<KeypairParam>,
    farm_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .harvest(&wallet_keypair, farm_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Cranks single Vault
#[post("/crank_vault?<wallet_keypair>&<vault_name>&<step>")]
async fn crank_vault(
    wallet_keypair: Option<KeypairParam>,
    vault_name: &str,
    step: u64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .crank_vault(&wallet_keypair, vault_name, step)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Cranks all Vaults
#[post("/crank_vaults?<wallet_keypair>&<step>")]
async fn crank_vaults(
    wallet_keypair: Option<KeypairParam>,
    step: u64,
    farm_client: &State<FarmClientArc>,
) -> Result<String, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let cranked = farm_client
        .crank_vaults(&wallet_keypair, step)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(cranked.to_string())
}

/// Clears cache records to force re-pull from blockchain
#[post("/reset_cache")]
async fn reset_cache(farm_client: &State<FarmClientArc>) -> Result<String, NotFound<String>> {
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    farm_client.reset_cache();

    Ok("OK".to_string())
}

/// Initializes a new User for the Fund
#[post("/user_init_fund?<wallet_keypair>&<fund_name>&<token_name>")]
async fn user_init_fund(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    token_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .user_init_fund(&wallet_keypair, fund_name, token_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Requests a new deposit to the Fund
#[post("/request_deposit_fund?<wallet_keypair>&<fund_name>&<token_name>&<ui_amount>")]
async fn request_deposit_fund(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    token_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .request_deposit_fund(&wallet_keypair, fund_name, token_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Cancels pending deposit to the Fund
#[post("/cancel_deposit_fund?<wallet_keypair>&<fund_name>&<token_name>")]
async fn cancel_deposit_fund(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    token_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .cancel_deposit_fund(&wallet_keypair, fund_name, token_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Requests a new withdrawal from the Fund
#[post("/request_withdrawal_fund?<wallet_keypair>&<fund_name>&<token_name>&<ui_amount>")]
async fn request_withdrawal_fund(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    token_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .request_withdrawal_fund(&wallet_keypair, fund_name, token_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Cancels pending deposit to the Fund
#[post("/cancel_withdrawal_fund?<wallet_keypair>&<fund_name>&<token_name>")]
async fn cancel_withdrawal_fund(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    token_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .cancel_withdrawal_fund(&wallet_keypair, fund_name, token_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Starts the Fund liquidation
#[post("/start_liquidation_fund?<wallet_keypair>&<fund_name>")]
async fn start_liquidation_fund(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .start_liquidation_fund(&wallet_keypair, fund_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Sets a new deposit schedule for the Fund
#[post("/set_fund_deposit_schedule?<wallet_keypair>&<fund_name>&<start_time>&<end_time>&<approval_required>&<min_amount_usd>&<max_amount_usd>&<fee>")]
#[allow(clippy::too_many_arguments)]
async fn set_fund_deposit_schedule(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    start_time: i64,
    end_time: i64,
    approval_required: bool,
    min_amount_usd: f64,
    max_amount_usd: f64,
    fee: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .set_fund_deposit_schedule(
            &wallet_keypair,
            fund_name,
            &FundSchedule {
                start_time,
                end_time,
                approval_required,
                min_amount_usd,
                max_amount_usd,
                fee,
            },
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Disables deposits to the Fund
#[post("/disable_deposits_fund?<wallet_keypair>&<fund_name>")]
async fn disable_deposits_fund(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .disable_deposits_fund(&wallet_keypair, fund_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Approves pending deposit to the Fund
#[post(
    "/approve_deposit_fund?<wallet_keypair>&<fund_name>&<user_address>&<token_name>&<ui_amount>"
)]
async fn approve_deposit_fund(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    user_address: Option<PubkeyParam>,
    token_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let user_address = check_unwrap_pubkey(user_address, "user_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .approve_deposit_fund(
            &wallet_keypair,
            fund_name,
            &user_address,
            token_name,
            ui_amount,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Denies pending deposit to the Fund
#[post("/deny_deposit_fund?<wallet_keypair>&<fund_name>&<user_address>&<token_name>&<deny_reason>")]
async fn deny_deposit_fund(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    user_address: Option<PubkeyParam>,
    token_name: &str,
    deny_reason: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let user_address = check_unwrap_pubkey(user_address, "user_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .deny_deposit_fund(
            &wallet_keypair,
            fund_name,
            &user_address,
            token_name,
            deny_reason,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Sets a new withdrawal schedule for the Fund
#[post("/set_fund_withdrawal_schedule?<wallet_keypair>&<fund_name>&<start_time>&<end_time>&<approval_required>&<min_amount_usd>&<max_amount_usd>&<fee>")]
#[allow(clippy::too_many_arguments)]
async fn set_fund_withdrawal_schedule(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    start_time: i64,
    end_time: i64,
    approval_required: bool,
    min_amount_usd: f64,
    max_amount_usd: f64,
    fee: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .set_fund_withdrawal_schedule(
            &wallet_keypair,
            fund_name,
            &FundSchedule {
                start_time,
                end_time,
                approval_required,
                min_amount_usd,
                max_amount_usd,
                fee,
            },
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Disables withdrawals from the Fund
#[post("/disable_withdrawals_fund?<wallet_keypair>&<fund_name>")]
async fn disable_withdrawals_fund(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .disable_withdrawals_fund(&wallet_keypair, fund_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Approves pending withdrawal from the Fund
#[post(
    "/approve_withdrawal_fund?<wallet_keypair>&<fund_name>&<user_address>&<token_name>&<ui_amount>"
)]
async fn approve_withdrawal_fund(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    user_address: Option<PubkeyParam>,
    token_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let user_address = check_unwrap_pubkey(user_address, "user_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .approve_withdrawal_fund(
            &wallet_keypair,
            fund_name,
            &user_address,
            token_name,
            ui_amount,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Denies pending withdrawal from the Fund
#[post(
    "/deny_withdrawal_fund?<wallet_keypair>&<fund_name>&<user_address>&<token_name>&<deny_reason>"
)]
async fn deny_withdrawal_fund(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    user_address: Option<PubkeyParam>,
    token_name: &str,
    deny_reason: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let user_address = check_unwrap_pubkey(user_address, "user_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .deny_withdrawal_fund(
            &wallet_keypair,
            fund_name,
            &user_address,
            token_name,
            deny_reason,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Moves deposited assets from Deposit/Withdraw custody to the Fund
#[post("/lock_assets_fund?<wallet_keypair>&<fund_name>&<token_name>&<ui_amount>")]
async fn lock_assets_fund(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    token_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .lock_assets_fund(&wallet_keypair, fund_name, token_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Releases assets from the Fund to Deposit/Withdraw custody
#[post("/unlock_assets_fund?<wallet_keypair>&<fund_name>&<token_name>&<ui_amount>")]
async fn unlock_assets_fund(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    token_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .unlock_assets_fund(&wallet_keypair, fund_name, token_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Update Fund assets info based on custody holdings
#[post("/update_fund_assets_with_custody?<wallet_keypair>&<fund_name>&<custody_id>")]
async fn update_fund_assets_with_custody(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    custody_id: u32,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .update_fund_assets_with_custody(&wallet_keypair, fund_name, custody_id)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Update Fund assets info based on all custodies
#[post("/update_fund_assets_with_custodies?<wallet_keypair>&<fund_name>")]
async fn update_fund_assets_with_custodies(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<String, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let updated = farm_client
        .update_fund_assets_with_custodies(&wallet_keypair, fund_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(updated.to_string())
}

/// Update Fund assets info based on Vault holdings
#[post("/update_fund_assets_with_vault?<wallet_keypair>&<fund_name>&<vault_id>")]
async fn update_fund_assets_with_vault(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    vault_id: u32,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .update_fund_assets_with_vault(&wallet_keypair, fund_name, vault_id)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Update Fund assets info based on Vault holdings
#[post("/update_fund_assets_with_vaults?<wallet_keypair>&<fund_name>")]
async fn update_fund_assets_with_vaults(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<String, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let updated = farm_client
        .update_fund_assets_with_vaults(&wallet_keypair, fund_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(updated.to_string())
}

/// Swaps tokens in the Fund
#[post(
    "/fund_swap?<wallet_keypair>&<fund_name>&<protocol>&<from_token>&<to_token>&<ui_amount_in>&<min_ui_amount_out>"
)]
#[allow(clippy::too_many_arguments)]
async fn fund_swap(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    protocol: &str,
    from_token: &str,
    to_token: &str,
    ui_amount_in: f64,
    min_ui_amount_out: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let protocol = protocol
        .parse()
        .map_err(|_| NotFound("Invalid protocol argument".to_string()))?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .fund_swap(
            &wallet_keypair,
            fund_name,
            protocol,
            from_token,
            to_token,
            ui_amount_in,
            min_ui_amount_out,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Adds liquidity to the Pool in the Fund
#[post("/fund_add_liquidity_pool?<wallet_keypair>&<fund_name>&<pool_name>&<max_token_a_ui_amount>&<max_token_b_ui_amount>")]
async fn fund_add_liquidity_pool(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    pool_name: &str,
    max_token_a_ui_amount: f64,
    max_token_b_ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .fund_add_liquidity_pool(
            &wallet_keypair,
            fund_name,
            pool_name,
            max_token_a_ui_amount,
            max_token_b_ui_amount,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Removes liquidity from the Pool in the Fund
#[post("/fund_remove_liquidity_pool?<wallet_keypair>&<fund_name>&<pool_name>&<ui_amount>")]
async fn fund_remove_liquidity_pool(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    pool_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .fund_remove_liquidity_pool(&wallet_keypair, fund_name, pool_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Initializes a new User for the Farm in the Fund
#[post("/fund_user_init_farm?<wallet_keypair>&<fund_name>&<farm_name>")]
async fn fund_user_init_farm(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    farm_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .fund_user_init_farm(&wallet_keypair, fund_name, farm_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Stakes tokens to the Farm in the Fund
#[post("/fund_stake?<wallet_keypair>&<fund_name>&<farm_name>&<ui_amount>")]
async fn fund_stake(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    farm_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .fund_stake(&wallet_keypair, fund_name, farm_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Unstakes tokens from the Farm in the Fund
#[post("/fund_unstake?<wallet_keypair>&<fund_name>&<farm_name>&<ui_amount>")]
async fn fund_unstake(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    farm_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .fund_unstake(&wallet_keypair, fund_name, farm_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Harvests rewards from the Farm in the Fund
#[post("/fund_harvest?<wallet_keypair>&<fund_name>&<farm_name>")]
async fn fund_harvest(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    farm_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .fund_harvest(&wallet_keypair, fund_name, farm_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Initializes a new User for the Vault in the Fund
#[post("/fund_user_init_vault?<wallet_keypair>&<fund_name>&<vault_name>")]
async fn fund_user_init_vault(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    vault_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .fund_user_init_vault(&wallet_keypair, fund_name, vault_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Adds liquidity to the Vault in the Fund
#[post("/fund_add_liquidity_vault?<wallet_keypair>&<fund_name>&<vault_name>&<max_token_a_ui_amount>&<max_token_b_ui_amount>")]
async fn fund_add_liquidity_vault(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    vault_name: &str,
    max_token_a_ui_amount: f64,
    max_token_b_ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .fund_add_liquidity_vault(
            &wallet_keypair,
            fund_name,
            vault_name,
            max_token_a_ui_amount,
            max_token_b_ui_amount,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Adds locked liquidity to the Vault in the Fund
#[post("/fund_add_locked_liquidity_vault?<wallet_keypair>&<fund_name>&<vault_name>&<ui_amount>")]
async fn fund_add_locked_liquidity_vault(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    vault_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .fund_add_locked_liquidity_vault(&wallet_keypair, fund_name, vault_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Removes liquidity from the Vault in the Fund
#[post("/fund_remove_liquidity_vault?<wallet_keypair>&<fund_name>&<vault_name>&<ui_amount>")]
async fn fund_remove_liquidity_vault(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    vault_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .fund_remove_liquidity_vault(&wallet_keypair, fund_name, vault_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Removes unlocked liquidity from the Vault in the Fund
#[post(
    "/fund_remove_unlocked_liquidity_vault?<wallet_keypair>&<fund_name>&<vault_name>&<ui_amount>"
)]
async fn fund_remove_unlocked_liquidity_vault(
    wallet_keypair: Option<KeypairParam>,
    fund_name: &str,
    vault_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<Signature, NotFound<String>> {
    let wallet_keypair = check_unwrap_keypair(wallet_keypair, "wallet_keypair")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let signature = farm_client
        .fund_remove_unlocked_liquidity_vault(&wallet_keypair, fund_name, vault_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(signature.to_string())
}

/// Returns a new Instruction for creating system account
#[get("/new_instruction_create_system_account?<wallet_address>&<new_address>&<lamports>&<space>&<owner>")]
async fn new_instruction_create_system_account(
    wallet_address: Option<PubkeyParam>,
    new_address: Option<PubkeyParam>,
    lamports: u64,
    space: usize,
    owner: Option<PubkeyParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let new_address = check_unwrap_pubkey(new_address, "new_address")?;
    let owner = check_unwrap_pubkey(owner, "owner")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_create_system_account(
            &wallet_address,
            &new_address,
            lamports,
            space,
            &owner,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for creating system account with seed
#[get("/new_instruction_create_system_account_with_seed?<wallet_address>&<base_address>&<seed>&<lamports>&<space>&<owner>")]
async fn new_instruction_create_system_account_with_seed(
    wallet_address: Option<PubkeyParam>,
    base_address: Option<PubkeyParam>,
    seed: &str,
    lamports: u64,
    space: usize,
    owner: Option<PubkeyParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let base_address = check_unwrap_pubkey(base_address, "base_address")?;
    let owner = check_unwrap_pubkey(owner, "owner")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_create_system_account_with_seed(
            &wallet_address,
            &base_address,
            seed,
            lamports,
            space,
            &owner,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for closing system account
#[get("/new_instruction_close_system_account?<wallet_address>&<target_address>")]
async fn new_instruction_close_system_account(
    wallet_address: Option<PubkeyParam>,
    target_address: Option<PubkeyParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let target_address = check_unwrap_pubkey(target_address, "target_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_close_system_account(&wallet_address, &target_address)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns the native SOL transfer instruction
#[get("/new_instruction_transfer?<wallet_address>&<destination_wallet>&<sol_ui_amount>")]
async fn new_instruction_transfer(
    wallet_address: Option<PubkeyParam>,
    destination_wallet: Option<PubkeyParam>,
    sol_ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let destination_wallet = check_unwrap_pubkey(destination_wallet, "destination_wallet")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_transfer(&wallet_address, &destination_wallet, sol_ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a tokens transfer instruction
#[get("/new_instruction_token_transfer?<wallet_address>&<token_name>&<destination_wallet>&<ui_amount>")]
async fn new_instruction_token_transfer(
    wallet_address: Option<PubkeyParam>,
    token_name: &str,
    destination_wallet: Option<PubkeyParam>,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let destination_wallet = check_unwrap_pubkey(destination_wallet, "destination_wallet")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_token_transfer(&wallet_address, token_name, &destination_wallet, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for syncing token balance for the specified account
#[get("/new_instruction_sync_token_balance?<wallet_address>&<token_name>")]
async fn new_instruction_sync_token_balance(
    wallet_address: Option<PubkeyParam>,
    token_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_sync_token_balance(&wallet_address, token_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for creating associated token account
#[get("/new_instruction_create_token_account?<wallet_address>&<token_name>")]
async fn new_instruction_create_token_account(
    wallet_address: Option<PubkeyParam>,
    token_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_create_token_account(&wallet_address, token_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for closing associated token account
#[get("/new_instruction_close_token_account?<wallet_address>&<token_name>")]
async fn new_instruction_close_token_account(
    wallet_address: Option<PubkeyParam>,
    token_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_close_token_account(&wallet_address, token_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for initializing a new User for the Vault
#[get("/new_instruction_user_init_vault?<wallet_address>&<vault_name>")]
async fn new_instruction_user_init_vault(
    wallet_address: Option<PubkeyParam>,
    vault_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_user_init_vault(&wallet_address, vault_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for adding liquidity to the Vault
#[get("/new_instruction_add_liquidity_vault?<wallet_address>&<vault_name>&<max_token_a_ui_amount>&<max_token_b_ui_amount>")]
async fn new_instruction_add_liquidity_vault(
    wallet_address: Option<PubkeyParam>,
    vault_name: &str,
    max_token_a_ui_amount: f64,
    max_token_b_ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_add_liquidity_vault(
            &wallet_address,
            vault_name,
            max_token_a_ui_amount,
            max_token_b_ui_amount,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for locking liquidity in the Vault
#[get("/new_instruction_lock_liquidity_vault?<wallet_address>&<vault_name>&<ui_amount>")]
async fn new_instruction_lock_liquidity_vault(
    wallet_address: Option<PubkeyParam>,
    vault_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_lock_liquidity_vault(&wallet_address, vault_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for unlocking liquidity from the Vault
#[get("/new_instruction_unlock_liquidity_vault?<wallet_address>&<vault_name>&<ui_amount>")]
async fn new_instruction_unlock_liquidity_vault(
    wallet_address: Option<PubkeyParam>,
    vault_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_unlock_liquidity_vault(&wallet_address, vault_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for removing liquidity from the Vault
#[get("/new_instruction_remove_liquidity_vault?<wallet_address>&<vault_name>&<ui_amount>")]
async fn new_instruction_remove_liquidity_vault(
    wallet_address: Option<PubkeyParam>,
    vault_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_remove_liquidity_vault(&wallet_address, vault_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for adding liquidity to the Pool
#[get("/new_instruction_add_liquidity_pool?<wallet_address>&<pool_name>&<max_token_a_ui_amount>&<max_token_b_ui_amount>")]
async fn new_instruction_add_liquidity_pool(
    wallet_address: Option<PubkeyParam>,
    pool_name: &str,
    max_token_a_ui_amount: f64,
    max_token_b_ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_add_liquidity_pool(
            &wallet_address,
            pool_name,
            max_token_a_ui_amount,
            max_token_b_ui_amount,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for removing liquidity from the Pool
#[get("/new_instruction_remove_liquidity_pool?<wallet_address>&<pool_name>&<ui_amount>")]
async fn new_instruction_remove_liquidity_pool(
    wallet_address: Option<PubkeyParam>,
    pool_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_remove_liquidity_pool(&wallet_address, pool_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for wrapping the token into protocol specific token
#[get("/new_instruction_wrap_token?<wallet_address>&<pool_name>&<token_to_wrap>&<ui_amount>")]
async fn new_instruction_wrap_token(
    wallet_address: Option<PubkeyParam>,
    pool_name: &str,
    token_to_wrap: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let token_to_wrap = token_to_wrap
        .parse()
        .map_err(|_| NotFound("Invalid token_to_wrap argument".to_string()))?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_wrap_token(&wallet_address, pool_name, token_to_wrap, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for unwrapping the token from protocol specific token
#[get("/new_instruction_unwrap_token?<wallet_address>&<pool_name>&<token_to_unwrap>&<ui_amount>")]
async fn new_instruction_unwrap_token(
    wallet_address: Option<PubkeyParam>,
    pool_name: &str,
    token_to_unwrap: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let token_to_unwrap = token_to_unwrap
        .parse()
        .map_err(|_| NotFound("Invalid token_to_unwrap argument".to_string()))?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_unwrap_token(&wallet_address, pool_name, token_to_unwrap, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for tokens swap
#[get("/new_instruction_swap?<wallet_address>&<protocol>&<from_token>&<to_token>&<ui_amount_in>&<min_ui_amount_out>")]
async fn new_instruction_swap(
    wallet_address: Option<PubkeyParam>,
    protocol: &str,
    from_token: &str,
    to_token: &str,
    ui_amount_in: f64,
    min_ui_amount_out: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let protocol = protocol
        .parse()
        .map_err(|_| NotFound("Invalid protocol argument".to_string()))?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_swap(
            &wallet_address,
            protocol,
            from_token,
            to_token,
            ui_amount_in,
            min_ui_amount_out,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for initializing a new User in the Farm
#[get("/new_instruction_user_init?<wallet_address>&<farm_name>")]
async fn new_instruction_user_init(
    wallet_address: Option<PubkeyParam>,
    farm_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_user_init(&wallet_address, farm_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for tokens staking
#[get("/new_instruction_stake?<wallet_address>&<farm_name>&<ui_amount>")]
async fn new_instruction_stake(
    wallet_address: Option<PubkeyParam>,
    farm_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_stake(&wallet_address, farm_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for tokens unstaking
#[get("/new_instruction_unstake?<wallet_address>&<farm_name>&<ui_amount>")]
async fn new_instruction_unstake(
    wallet_address: Option<PubkeyParam>,
    farm_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_unstake(&wallet_address, farm_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for rewards harvesting
#[get("/new_instruction_harvest?<wallet_address>&<farm_name>")]
async fn new_instruction_harvest(
    wallet_address: Option<PubkeyParam>,
    farm_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_harvest(&wallet_address, farm_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Vault Crank Instruction
#[get("/new_instruction_crank_vault?<wallet_address>&<vault_name>&<step>")]
async fn new_instruction_crank_vault(
    wallet_address: Option<PubkeyParam>,
    vault_name: &str,
    step: u64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_crank_vault(&wallet_address, vault_name, step)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for initializing a new User for the Fund
#[get("/new_instruction_user_init_fund?<wallet_address>&<fund_name>&<token_name>")]
async fn new_instruction_user_init_fund(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    token_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_user_init_fund(&wallet_address, fund_name, token_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for requesting deposit to the Fund
#[get(
    "/new_instruction_request_deposit_fund?<wallet_address>&<fund_name>&<token_name>&<ui_amount>"
)]
async fn new_instruction_request_deposit_fund(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    token_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_request_deposit_fund(&wallet_address, fund_name, token_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for canceling pending deposit to the Fund
#[get("/new_instruction_cancel_deposit_fund?<wallet_address>&<fund_name>&<token_name>")]
async fn new_instruction_cancel_deposit_fund(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    token_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_cancel_deposit_fund(&wallet_address, fund_name, token_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for requesting withdrawal from the Fund
#[get("/new_instruction_request_withdrawal_fund?<wallet_address>&<fund_name>&<token_name>&<ui_amount>")]
async fn new_instruction_request_withdrawal_fund(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    token_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_request_withdrawal_fund(&wallet_address, fund_name, token_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for canceling pending withdrawal from the Fund
#[get("/new_instruction_cancel_withdrawal_fund?<wallet_address>&<fund_name>&<token_name>")]
async fn new_instruction_cancel_withdrawal_fund(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    token_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_cancel_withdrawal_fund(&wallet_address, fund_name, token_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for initiating liquidation of the Fund
#[get("/new_instruction_start_liquidation_fund?<wallet_address>&<fund_name>")]
async fn new_instruction_start_liquidation_fund(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_start_liquidation_fund(&wallet_address, fund_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new set deposit schedule Instruction
#[get("/new_instruction_set_fund_deposit_schedule?<wallet_address>&<fund_name>&<start_time>&<end_time>&<approval_required>&<min_amount_usd>&<max_amount_usd>&<fee>")]
#[allow(clippy::too_many_arguments)]
async fn new_instruction_set_fund_deposit_schedule(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    start_time: i64,
    end_time: i64,
    approval_required: bool,
    min_amount_usd: f64,
    max_amount_usd: f64,
    fee: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_set_fund_deposit_schedule(
            &wallet_address,
            fund_name,
            &FundSchedule {
                start_time,
                end_time,
                approval_required,
                min_amount_usd,
                max_amount_usd,
                fee,
            },
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for disabling deposits to the Fund
#[get("/new_instruction_disable_deposits_fund?<wallet_address>&<fund_name>")]
async fn new_instruction_disable_deposits_fund(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_disable_deposits_fund(&wallet_address, fund_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for approving deposit to the Fund
#[get("/new_instruction_approve_deposit_fund?<wallet_address>&<user_address>&<fund_name>&<token_name>&<ui_amount>")]
async fn new_instruction_approve_deposit_fund(
    wallet_address: Option<PubkeyParam>,
    user_address: Option<PubkeyParam>,
    fund_name: &str,
    token_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let user_address = check_unwrap_pubkey(user_address, "user_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_approve_deposit_fund(
            &wallet_address,
            &user_address,
            fund_name,
            token_name,
            ui_amount,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for denying deposit to the Fund
#[get("/new_instruction_deny_deposit_fund?<wallet_address>&<user_address>&<fund_name>&<token_name>&<deny_reason>")]
async fn new_instruction_deny_deposit_fund(
    wallet_address: Option<PubkeyParam>,
    user_address: Option<PubkeyParam>,
    fund_name: &str,
    token_name: &str,
    deny_reason: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let user_address = check_unwrap_pubkey(user_address, "user_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_deny_deposit_fund(
            &wallet_address,
            &user_address,
            fund_name,
            token_name,
            deny_reason,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new set withdrawal schedule Instruction
#[get("/new_instruction_set_fund_withdrawal_schedule?<wallet_address>&<fund_name>&<start_time>&<end_time>&<approval_required>&<min_amount_usd>&<max_amount_usd>&<fee>")]
#[allow(clippy::too_many_arguments)]
async fn new_instruction_set_fund_withdrawal_schedule(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    start_time: i64,
    end_time: i64,
    approval_required: bool,
    min_amount_usd: f64,
    max_amount_usd: f64,
    fee: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_set_fund_withdrawal_schedule(
            &wallet_address,
            fund_name,
            &FundSchedule {
                start_time,
                end_time,
                approval_required,
                min_amount_usd,
                max_amount_usd,
                fee,
            },
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for disabling withdrawals from the Fund
#[get("/new_instruction_disable_withdrawals_fund?<wallet_address>&<fund_name>")]
async fn new_instruction_disable_withdrawals_fund(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_disable_withdrawals_fund(&wallet_address, fund_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for approving withdrawal from the Fund
#[get("/new_instruction_approve_withdrawal_fund?<wallet_address>&<user_address>&<fund_name>&<token_name>&<ui_amount>")]
async fn new_instruction_approve_withdrawal_fund(
    wallet_address: Option<PubkeyParam>,
    user_address: Option<PubkeyParam>,
    fund_name: &str,
    token_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let user_address = check_unwrap_pubkey(user_address, "user_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_approve_withdrawal_fund(
            &wallet_address,
            &user_address,
            fund_name,
            token_name,
            ui_amount,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for denying withdrawal from the Fund
#[get("/new_instruction_deny_withdrawal_fund?<wallet_address>&<user_address>&<fund_name>&<token_name>&<deny_reason>")]
async fn new_instruction_deny_withdrawal_fund(
    wallet_address: Option<PubkeyParam>,
    user_address: Option<PubkeyParam>,
    fund_name: &str,
    token_name: &str,
    deny_reason: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let user_address = check_unwrap_pubkey(user_address, "user_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_deny_withdrawal_fund(
            &wallet_address,
            &user_address,
            fund_name,
            token_name,
            deny_reason,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for moving deposited assets to the Fund
#[get("/new_instruction_lock_assets_fund?<wallet_address>&<fund_name>&<token_name>&<ui_amount>")]
async fn new_instruction_lock_assets_fund(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    token_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_lock_assets_fund(&wallet_address, fund_name, token_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for releasing assets from the Fund to Deposit/Withdraw custody
#[get("/new_instruction_unlock_assets_fund?<wallet_address>&<fund_name>&<token_name>&<ui_amount>")]
async fn new_instruction_unlock_assets_fund(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    token_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_unlock_assets_fund(&wallet_address, fund_name, token_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for updating Fund assets based on custody holdings
#[get("/new_instruction_update_fund_assets_with_custody?<wallet_address>&<fund_name>&<custody_id>")]
async fn new_instruction_update_fund_assets_with_custody(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    custody_id: u32,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_update_fund_assets_with_custody(&wallet_address, fund_name, custody_id)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for updating Fund assets with Vault holdings
#[get("/new_instruction_update_fund_assets_with_vault?<wallet_address>&<fund_name>&<vault_id>")]
async fn new_instruction_update_fund_assets_with_vault(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    vault_id: u32,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_update_fund_assets_with_vault(&wallet_address, fund_name, vault_id)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for tokens swap in the Fund
#[get("/new_instruction_fund_swap?<wallet_address>&<fund_name>&<protocol>&<from_token>&<to_token>&<ui_amount_in>&<min_ui_amount_out>")]
#[allow(clippy::too_many_arguments)]
async fn new_instruction_fund_swap(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    protocol: &str,
    from_token: &str,
    to_token: &str,
    ui_amount_in: f64,
    min_ui_amount_out: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let protocol = protocol
        .parse()
        .map_err(|_| NotFound("Invalid protocol argument".to_string()))?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_fund_swap(
            &wallet_address,
            fund_name,
            protocol,
            from_token,
            to_token,
            ui_amount_in,
            min_ui_amount_out,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for adding liquidity to the Pool in the Fund
#[get("/new_instruction_fund_add_liquidity_pool?<wallet_address>&<fund_name>&<pool_name>&<max_token_a_ui_amount>&<max_token_b_ui_amount>")]
async fn new_instruction_fund_add_liquidity_pool(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    pool_name: &str,
    max_token_a_ui_amount: f64,
    max_token_b_ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_fund_add_liquidity_pool(
            &wallet_address,
            fund_name,
            pool_name,
            max_token_a_ui_amount,
            max_token_b_ui_amount,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for removing liquidity from the Pool in the Fund
#[get("/new_instruction_fund_remove_liquidity_pool?<wallet_address>&<fund_name>&<pool_name>&<ui_amount>")]
async fn new_instruction_fund_remove_liquidity_pool(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    pool_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_fund_remove_liquidity_pool(
            &wallet_address,
            fund_name,
            pool_name,
            ui_amount,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for initializing a new User for the Farm in the Fund
#[get("/new_instruction_fund_user_init_farm?<wallet_address>&<fund_name>&<farm_name>")]
async fn new_instruction_fund_user_init_farm(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    farm_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_fund_user_init_farm(&wallet_address, fund_name, farm_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for tokens staking to the Farm in the Fund
#[get("/new_instruction_fund_stake?<wallet_address>&<fund_name>&<farm_name>&<ui_amount>")]
async fn new_instruction_fund_stake(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    farm_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_fund_stake(&wallet_address, fund_name, farm_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for tokens unstaking from the Farm in the Fund
#[get("/new_instruction_fund_unstake?<wallet_address>&<fund_name>&<farm_name>&<ui_amount>")]
async fn new_instruction_fund_unstake(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    farm_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_fund_unstake(&wallet_address, fund_name, farm_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for rewards harvesting from the Farm in the Fund
#[get("/new_instruction_fund_harvest?<wallet_address>&<fund_name>&<farm_name>")]
async fn new_instruction_fund_harvest(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    farm_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_fund_harvest(&wallet_address, fund_name, farm_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for initializing a new User for the Vault in the Fund
#[get("/new_instruction_fund_user_init_vault?<wallet_address>&<fund_name>&<vault_name>")]
async fn new_instruction_fund_user_init_vault(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    vault_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_fund_user_init_vault(&wallet_address, fund_name, vault_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for adding liquidity to the Vault in the Fund
#[get("/new_instruction_fund_add_liquidity_vault?<wallet_address>&<fund_name>&<vault_name>&<max_token_a_ui_amount>&<max_token_b_ui_amount>")]
async fn new_instruction_fund_add_liquidity_vault(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    vault_name: &str,
    max_token_a_ui_amount: f64,
    max_token_b_ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_fund_add_liquidity_vault(
            &wallet_address,
            fund_name,
            vault_name,
            max_token_a_ui_amount,
            max_token_b_ui_amount,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for locking liquidity in the Vault in the Fund
#[get("/new_instruction_fund_lock_liquidity_vault?<wallet_address>&<fund_name>&<vault_name>&<ui_amount>")]
async fn new_instruction_fund_lock_liquidity_vault(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    vault_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_fund_lock_liquidity_vault(
            &wallet_address,
            fund_name,
            vault_name,
            ui_amount,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for unlocking liquidity from the Vault in the Fund
#[get("/new_instruction_fund_unlock_liquidity_vault?<wallet_address>&<fund_name>&<vault_name>&<ui_amount>")]
async fn new_instruction_fund_unlock_liquidity_vault(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    vault_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_fund_unlock_liquidity_vault(
            &wallet_address,
            fund_name,
            vault_name,
            ui_amount,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new Instruction for removing liquidity from the Vault in the Fund
#[get("/new_instruction_fund_remove_liquidity_vault?<wallet_address>&<fund_name>&<vault_name>&<ui_amount>")]
async fn new_instruction_fund_remove_liquidity_vault(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    vault_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstruction, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instruction = farm_client
        .new_instruction_fund_remove_liquidity_vault(
            &wallet_address,
            fund_name,
            vault_name,
            ui_amount,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstruction::new(&instruction))
}

/// Returns a new complete set of instructions for tokens transfer
#[get("/all_instructions_token_transfer?<wallet_address>&<token_name>&<destination_wallet>&<ui_amount>")]
async fn all_instructions_token_transfer(
    wallet_address: Option<PubkeyParam>,
    token_name: &str,
    destination_wallet: Option<PubkeyParam>,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstructions, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let destination_wallet = check_unwrap_pubkey(destination_wallet, "destination_wallet")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instructions = farm_client
        .all_instructions_token_transfer(
            &wallet_address,
            token_name,
            &destination_wallet,
            ui_amount,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstructions::new(&instructions))
}

/// Returns a new complete set of instructions for SOL wrapping
#[get("/all_instructions_wrap_sol?<wallet_address>&<ui_amount>")]
async fn all_instructions_wrap_sol(
    wallet_address: Option<PubkeyParam>,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstructions, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instructions = farm_client
        .all_instructions_wrap_sol(&wallet_address, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstructions::new(&instructions))
}

/// Returns a new complete set of instructions for SOL unwrapping
#[get("/all_instructions_unwrap_sol?<wallet_address>")]
async fn all_instructions_unwrap_sol(
    wallet_address: Option<PubkeyParam>,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstructions, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instructions = farm_client
        .all_instructions_unwrap_sol(&wallet_address)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstructions::new(&instructions))
}

/// Returns a new complete set of instructions for adding liquidity to the Vault
#[get("/all_instructions_add_liquidity_vault?<wallet_address>&<vault_name>&<max_token_a_ui_amount>&<max_token_b_ui_amount>")]
async fn all_instructions_add_liquidity_vault(
    wallet_address: Option<PubkeyParam>,
    vault_name: &str,
    max_token_a_ui_amount: f64,
    max_token_b_ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstructions, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instructions = farm_client
        .all_instructions_add_liquidity_vault(
            &wallet_address,
            vault_name,
            max_token_a_ui_amount,
            max_token_b_ui_amount,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstructions::new(&instructions))
}

/// Returns a new complete set of instructions for adding locked liquidity to the Vault
#[get("/all_instructions_add_locked_liquidity_vault?<wallet_address>&<vault_name>&<ui_amount>")]
async fn all_instructions_add_locked_liquidity_vault(
    wallet_address: Option<PubkeyParam>,
    vault_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstructions, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instructions = farm_client
        .all_instructions_add_locked_liquidity_vault(&wallet_address, vault_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstructions::new(&instructions))
}

/// Returns a new complete set of Instructions for removing liquidity from the Vault
#[get("/all_instructions_remove_liquidity_vault?<wallet_address>&<vault_name>&<ui_amount>")]
async fn all_instructions_remove_liquidity_vault(
    wallet_address: Option<PubkeyParam>,
    vault_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstructions, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instructions = farm_client
        .all_instructions_remove_liquidity_vault(&wallet_address, vault_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstructions::new(&instructions))
}

/// Returns a new complete set of Instructions for removing unlocked liquidity from the Vault
#[get(
    "/all_instructions_remove_unlocked_liquidity_vault?<wallet_address>&<vault_name>&<ui_amount>"
)]
async fn all_instructions_remove_unlocked_liquidity_vault(
    wallet_address: Option<PubkeyParam>,
    vault_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstructions, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instructions = farm_client
        .all_instructions_remove_unlocked_liquidity_vault(&wallet_address, vault_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstructions::new(&instructions))
}

/// Returns a new complete set of Instructions for adding liquidity to the Pool
#[get(
    "/all_instructions_add_liquidity_pool?<wallet_address>&<pool_name>&<max_token_a_ui_amount>&<max_token_b_ui_amount>"
)]
async fn all_instructions_add_liquidity_pool(
    wallet_address: Option<PubkeyParam>,
    pool_name: &str,
    max_token_a_ui_amount: f64,
    max_token_b_ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstructions, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instructions = farm_client
        .all_instructions_add_liquidity_pool(
            &wallet_address,
            pool_name,
            max_token_a_ui_amount,
            max_token_b_ui_amount,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstructions::new(&instructions))
}

/// Returns a new complete set of Instructions for removing liquidity from the Pool
#[get("/all_instructions_remove_liquidity_pool?<wallet_address>&<pool_name>&<ui_amount>")]
async fn all_instructions_remove_liquidity_pool(
    wallet_address: Option<PubkeyParam>,
    pool_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstructions, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instructions = farm_client
        .all_instructions_remove_liquidity_pool(&wallet_address, pool_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstructions::new(&instructions))
}

/// Returns a new complete set of Instructions for swapping tokens
#[get("/all_instructions_swap?<wallet_address>&<protocol>&<from_token>&<to_token>&<ui_amount_in>&<min_ui_amount_out>")]
async fn all_instructions_swap(
    wallet_address: Option<PubkeyParam>,
    protocol: &str,
    from_token: &str,
    to_token: &str,
    ui_amount_in: f64,
    min_ui_amount_out: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstructions, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let protocol = protocol
        .parse()
        .map_err(|_| NotFound("Invalid protocol argument".to_string()))?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instructions = farm_client
        .all_instructions_swap(
            &wallet_address,
            protocol,
            from_token,
            to_token,
            ui_amount_in,
            min_ui_amount_out,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstructions::new(&instructions))
}

/// Returns a new complete set of Instructions for staking tokens to the Farm
#[get("/all_instructions_stake?<wallet_address>&<farm_name>&<ui_amount>")]
async fn all_instructions_stake(
    wallet_address: Option<PubkeyParam>,
    farm_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstructions, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instructions = farm_client
        .all_instructions_stake(&wallet_address, farm_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstructions::new(&instructions))
}

/// Returns a new complete set of Instructions for unstaking tokens from the Farm
#[get("/all_instructions_unstake?<wallet_address>&<farm_name>&<ui_amount>")]
async fn all_instructions_unstake(
    wallet_address: Option<PubkeyParam>,
    farm_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstructions, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instructions = farm_client
        .all_instructions_unstake(&wallet_address, farm_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstructions::new(&instructions))
}

/// Returns a new complete set of Instructions for harvesting rewards from the Farm
#[get("/all_instructions_harvest?<wallet_address>&<farm_name>")]
async fn all_instructions_harvest(
    wallet_address: Option<PubkeyParam>,
    farm_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstructions, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instructions = farm_client
        .all_instructions_harvest(&wallet_address, farm_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstructions::new(&instructions))
}

/// Returns a new complete set of Instructions for requesting a new deposit to the Fund
#[get(
    "/all_instructions_request_deposit_fund?<wallet_address>&<fund_name>&<token_name>&<ui_amount>"
)]
async fn all_instructions_request_deposit_fund(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    token_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstructions, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instructions = farm_client
        .all_instructions_request_deposit_fund(&wallet_address, fund_name, token_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstructions::new(&instructions))
}

/// Returns a new complete set of Instructions for requesting a new withdrawal from the Fund
#[get(
    "/all_instructions_request_withdrawal_fund?<wallet_address>&<fund_name>&<token_name>&<ui_amount>"
)]
async fn all_instructions_request_withdrawal_fund(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    token_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstructions, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instructions = farm_client
        .all_instructions_request_withdrawal_fund(&wallet_address, fund_name, token_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstructions::new(&instructions))
}

/// Returns a new complete set of Instructions for swapping tokens in the Fund
#[get("/all_instructions_fund_swap?<wallet_address>&<fund_name>&<protocol>&<from_token>&<to_token>&<ui_amount_in>&<min_ui_amount_out>")]
#[allow(clippy::too_many_arguments)]
async fn all_instructions_fund_swap(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    protocol: &str,
    from_token: &str,
    to_token: &str,
    ui_amount_in: f64,
    min_ui_amount_out: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstructions, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let protocol = protocol
        .parse()
        .map_err(|_| NotFound("Invalid protocol argument".to_string()))?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instructions = farm_client
        .all_instructions_fund_swap(
            &wallet_address,
            fund_name,
            protocol,
            from_token,
            to_token,
            ui_amount_in,
            min_ui_amount_out,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstructions::new(&instructions))
}

/// Returns a new complete set of Instructions for adding liquidity to the Pool in the Fund
#[get(
    "/all_instructions_fund_add_liquidity_pool?<wallet_address>&<fund_name>&<pool_name>&<max_token_a_ui_amount>&<max_token_b_ui_amount>"
)]
async fn all_instructions_fund_add_liquidity_pool(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    pool_name: &str,
    max_token_a_ui_amount: f64,
    max_token_b_ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstructions, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instructions = farm_client
        .all_instructions_fund_add_liquidity_pool(
            &wallet_address,
            fund_name,
            pool_name,
            max_token_a_ui_amount,
            max_token_b_ui_amount,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstructions::new(&instructions))
}

/// Returns a new complete set of Instructions for removing liquidity from the Pool in the Fund
#[get("/all_instructions_fund_remove_liquidity_pool?<wallet_address>&<fund_name>&<pool_name>&<ui_amount>")]
async fn all_instructions_fund_remove_liquidity_pool(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    pool_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstructions, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instructions = farm_client
        .all_instructions_fund_remove_liquidity_pool(
            &wallet_address,
            fund_name,
            pool_name,
            ui_amount,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstructions::new(&instructions))
}

/// Returns a new complete set of Instructions for staking tokens to the Farm in the Fund
#[get("/all_instructions_fund_stake?<wallet_address>&<fund_name>&<farm_name>&<ui_amount>")]
async fn all_instructions_fund_stake(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    farm_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstructions, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instructions = farm_client
        .all_instructions_fund_stake(&wallet_address, fund_name, farm_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstructions::new(&instructions))
}

/// Returns a new complete set of Instructions for unstaking tokens from the Farm in the Fund
#[get("/all_instructions_fund_unstake?<wallet_address>&<fund_name>&<farm_name>&<ui_amount>")]
async fn all_instructions_fund_unstake(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    farm_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstructions, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instructions = farm_client
        .all_instructions_fund_unstake(&wallet_address, fund_name, farm_name, ui_amount)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstructions::new(&instructions))
}

/// Returns a new complete set of Instructions for harvesting rewards from the Farm in the Fund
#[get("/all_instructions_fund_harvest?<wallet_address>&<fund_name>&<farm_name>")]
async fn all_instructions_fund_harvest(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    farm_name: &str,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstructions, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instructions = farm_client
        .all_instructions_fund_harvest(&wallet_address, fund_name, farm_name)
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstructions::new(&instructions))
}

/// Returns a new complete set of instructions for adding liquidity to the Vault in the Fund
#[get("/all_instructions_fund_add_liquidity_vault?<wallet_address>&<fund_name>&<vault_name>&<max_token_a_ui_amount>&<max_token_b_ui_amount>")]
async fn all_instructions_fund_add_liquidity_vault(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    vault_name: &str,
    max_token_a_ui_amount: f64,
    max_token_b_ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstructions, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instructions = farm_client
        .all_instructions_fund_add_liquidity_vault(
            &wallet_address,
            fund_name,
            vault_name,
            max_token_a_ui_amount,
            max_token_b_ui_amount,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstructions::new(&instructions))
}

/// Returns a new complete set of instructions for adding locked liquidity to the Vault in the Fund
#[get(
    "/all_instructions_fund_add_locked_liquidity_vault?<wallet_address>&<fund_name>&<vault_name>&<ui_amount>"
)]
async fn all_instructions_fund_add_locked_liquidity_vault(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    vault_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstructions, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instructions = farm_client
        .all_instructions_fund_add_locked_liquidity_vault(
            &wallet_address,
            fund_name,
            vault_name,
            ui_amount,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstructions::new(&instructions))
}

/// Returns a new complete set of Instructions for removing liquidity from the Vault in the Fund
#[get("/all_instructions_fund_remove_liquidity_vault?<wallet_address>&<fund_name>&<vault_name>&<ui_amount>")]
async fn all_instructions_fund_remove_liquidity_vault(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    vault_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstructions, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instructions = farm_client
        .all_instructions_fund_remove_liquidity_vault(
            &wallet_address,
            fund_name,
            vault_name,
            ui_amount,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstructions::new(&instructions))
}

/// Returns a new complete set of Instructions for removing unlocked liquidity from the Vault in the Fund
#[get(
    "/all_instructions_fund_remove_unlocked_liquidity_vault?<wallet_address>&<fund_name>&<vault_name>&<ui_amount>"
)]
async fn all_instructions_fund_remove_unlocked_liquidity_vault(
    wallet_address: Option<PubkeyParam>,
    fund_name: &str,
    vault_name: &str,
    ui_amount: f64,
    farm_client: &State<FarmClientArc>,
) -> Result<JsonWithInstructions, NotFound<String>> {
    let wallet_address = check_unwrap_pubkey(wallet_address, "wallet_address")?;
    let farm_client = farm_client
        .inner()
        .lock()
        .map_err(|e| NotFound(e.to_string()))?;
    let instructions = farm_client
        .all_instructions_fund_remove_unlocked_liquidity_vault(
            &wallet_address,
            fund_name,
            vault_name,
            ui_amount,
        )
        .map_err(|e| NotFound(e.to_string()))?;

    Ok(JsonWithInstructions::new(&instructions))
}

/// Retrieves data from URL as JSON
async fn get_url_data_as_json(url: &str) -> Result<Value> {
    let response = reqwest::get(url).await.map_err(|err| err.to_string())?;
    let text = response.text().await.map_err(|err| err.to_string())?;
    let value = from_str(text.as_str()).map_err(|err| err.to_string())?;
    Ok(value)
}

/// Initializes network service
async fn init_rpc(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket
}

/// Initilizes data to be served
async fn init_db(
    config: &Config,
    farm_client: &FarmClientArc,
    git_tokens: &mut GitTokens,
) -> Result<()> {
    // load tokens from GitHub
    info!("Loading tokens from {}", config.token_list_url);
    let dict: Value = get_url_data_as_json(&config.token_list_url).await.unwrap();
    assert!(dict.is_object());
    assert_ne!(dict["tokens"], json!(null));
    let loaded_tokens = dict["tokens"].as_array().unwrap();

    info!("Loading data from the blockchain, this may take a few mins...");
    let farm_client = farm_client.lock().map_err(|e| e.to_string())?;
    info!("Loading pools...");
    let _ = farm_client.get_pools().unwrap();
    info!("Loading farms...");
    let _ = farm_client.get_farms().unwrap();
    info!("Loading vaults...");
    let _ = farm_client.get_vaults().unwrap();
    info!("Loading funds...");
    let _ = farm_client.get_funds().unwrap();
    info!("Loading programs...");
    let _ = farm_client.get_program_ids().unwrap();
    info!("Loading tokens...");
    let _ = farm_client.get_tokens().unwrap();

    for val in loaded_tokens {
        let git_token: GitToken = from_value(val.clone()).unwrap();
        if git_token.chain_id == 101 {
            if let Ok(spl_token) = farm_client.get_token(&git_token.symbol) {
                if spl_token.mint == git_token.address {
                    git_tokens.insert(git_token.symbol.clone(), git_token.clone());
                }
            }
        }
    }

    info!("Done!");

    Ok(())
}

/// Entry point for JSON RPC, called from main
pub async fn stage(config: &Config) -> AdHoc {
    info!("Connecting Farm Client to {}", config.farm_client_url);
    let client_mutex = Arc::new(Mutex::new(FarmClient::new_with_commitment(
        &config.farm_client_url,
        CommitmentConfig::confirmed(),
    )));
    // check Cluster connectivity and version
    {
        let farm_client = client_mutex
            .lock()
            .expect("Failed to get lock on Farm Client");
        let version = farm_client
            .rpc_client
            .get_version()
            .expect("Failed to get Cluster version; Check Farm Client URL");
        info!("Cluster version: {}", version);
    }

    let fund_stats = Arc::new(Mutex::new(FundStats::new(&config.sqlite_db_path).unwrap()));

    let mut git_tokens: GitTokens = GitTokens::new();
    init_db(config, &client_mutex, &mut git_tokens)
        .await
        .unwrap();

    AdHoc::on_ignite("JSON RPC Stage", |rocket| async {
        rocket
            .manage(git_tokens)
            .manage(client_mutex)
            .manage(fund_stats)
            .attach(Cors)
            .attach(AdHoc::on_ignite("JSON RPC Init", init_rpc))
            .mount("/", FileServer::from(relative!("static")))
            .mount(
                "/api/v1",
                routes![
                    get_admins,
                    get_program_admins,
                    get_git_token,
                    get_git_tokens,
                    get_fund,
                    get_funds,
                    get_fund_refs,
                    get_fund_by_ref,
                    get_fund_name,
                    find_funds,
                    get_vault,
                    get_vaults,
                    get_vault_refs,
                    get_vault_by_ref,
                    get_vault_name,
                    find_vaults,
                    find_vaults_with_vt,
                    get_pool,
                    get_pools,
                    get_pool_refs,
                    get_pool_by_ref,
                    get_pool_name,
                    find_pools,
                    find_pools_with_lp,
                    get_farm,
                    get_farms,
                    get_farm_refs,
                    get_farm_by_ref,
                    get_farm_name,
                    find_farms_with_lp,
                    get_token,
                    get_tokens,
                    get_token_refs,
                    get_token_by_ref,
                    get_token_name,
                    get_token_with_mint,
                    get_token_with_account,
                    get_program_id,
                    get_program_ids,
                    get_program_name,
                    get_fund_ref,
                    get_vault_ref,
                    get_pool_ref,
                    get_farm_ref,
                    get_token_ref,
                    get_vault_admins,
                    get_vault_user_info,
                    get_vault_info,
                    get_all_vault_infos,
                    get_fund_admins,
                    get_fund_user_info,
                    get_all_fund_user_infos,
                    get_fund_user_requests,
                    get_all_fund_user_requests,
                    get_fund_info,
                    get_all_fund_infos,
                    get_fund_assets,
                    get_fund_custody,
                    get_fund_custodies,
                    get_fund_custody_with_balance,
                    get_fund_custodies_with_balance,
                    get_fund_vault,
                    get_fund_vaults,
                    get_fund_stats,
                    get_pool_price,
                    get_oracle,
                    get_oracle_price,
                    get_account_balance,
                    get_protocols,
                    is_official_id,
                    is_fund_manager,
                    get_managed_funds,
                    create_system_account,
                    create_system_account_with_seed,
                    assign_system_account,
                    close_system_account,
                    transfer,
                    token_transfer,
                    wrap_sol,
                    unwrap_sol,
                    sync_token_balance,
                    get_or_create_token_account,
                    close_token_account,
                    get_token_supply,
                    get_associated_token_address,
                    get_wallet_tokens,
                    get_token_account_data,
                    get_token_account_balance,
                    get_token_account_balance_with_address,
                    has_active_token_account,
                    get_user_stake_balance,
                    get_vault_stake_balance,
                    get_vault_token_decimals,
                    get_pool_tokens_decimals,
                    user_init_vault,
                    add_liquidity_vault,
                    add_locked_liquidity_vault,
                    remove_liquidity_vault,
                    remove_unlocked_liquidity_vault,
                    add_liquidity_pool,
                    remove_liquidity_pool,
                    swap,
                    user_init,
                    stake,
                    unstake,
                    harvest,
                    crank_vault,
                    crank_vaults,
                    reset_cache,
                    user_init_fund,
                    request_deposit_fund,
                    cancel_deposit_fund,
                    request_withdrawal_fund,
                    cancel_withdrawal_fund,
                    start_liquidation_fund,
                    set_fund_deposit_schedule,
                    disable_deposits_fund,
                    approve_deposit_fund,
                    deny_deposit_fund,
                    set_fund_withdrawal_schedule,
                    disable_withdrawals_fund,
                    approve_withdrawal_fund,
                    deny_withdrawal_fund,
                    lock_assets_fund,
                    unlock_assets_fund,
                    update_fund_assets_with_custody,
                    update_fund_assets_with_custodies,
                    update_fund_assets_with_vault,
                    update_fund_assets_with_vaults,
                    fund_swap,
                    fund_add_liquidity_pool,
                    fund_remove_liquidity_pool,
                    fund_user_init_farm,
                    fund_stake,
                    fund_unstake,
                    fund_harvest,
                    fund_user_init_vault,
                    fund_add_liquidity_vault,
                    fund_add_locked_liquidity_vault,
                    fund_remove_liquidity_vault,
                    fund_remove_unlocked_liquidity_vault,
                    new_instruction_create_system_account,
                    new_instruction_create_system_account_with_seed,
                    new_instruction_close_system_account,
                    new_instruction_transfer,
                    new_instruction_token_transfer,
                    new_instruction_sync_token_balance,
                    new_instruction_create_token_account,
                    new_instruction_close_token_account,
                    new_instruction_user_init_vault,
                    new_instruction_add_liquidity_vault,
                    new_instruction_lock_liquidity_vault,
                    new_instruction_unlock_liquidity_vault,
                    new_instruction_remove_liquidity_vault,
                    new_instruction_crank_vault,
                    new_instruction_add_liquidity_pool,
                    new_instruction_remove_liquidity_pool,
                    new_instruction_wrap_token,
                    new_instruction_unwrap_token,
                    new_instruction_swap,
                    new_instruction_user_init,
                    new_instruction_stake,
                    new_instruction_unstake,
                    new_instruction_harvest,
                    new_instruction_user_init_fund,
                    new_instruction_request_deposit_fund,
                    new_instruction_cancel_deposit_fund,
                    new_instruction_request_withdrawal_fund,
                    new_instruction_cancel_withdrawal_fund,
                    new_instruction_start_liquidation_fund,
                    new_instruction_set_fund_deposit_schedule,
                    new_instruction_disable_deposits_fund,
                    new_instruction_approve_deposit_fund,
                    new_instruction_deny_deposit_fund,
                    new_instruction_set_fund_withdrawal_schedule,
                    new_instruction_disable_withdrawals_fund,
                    new_instruction_approve_withdrawal_fund,
                    new_instruction_deny_withdrawal_fund,
                    new_instruction_lock_assets_fund,
                    new_instruction_unlock_assets_fund,
                    new_instruction_update_fund_assets_with_custody,
                    new_instruction_update_fund_assets_with_vault,
                    new_instruction_fund_swap,
                    new_instruction_fund_add_liquidity_pool,
                    new_instruction_fund_remove_liquidity_pool,
                    new_instruction_fund_user_init_farm,
                    new_instruction_fund_stake,
                    new_instruction_fund_unstake,
                    new_instruction_fund_harvest,
                    new_instruction_fund_user_init_vault,
                    new_instruction_fund_add_liquidity_vault,
                    new_instruction_fund_lock_liquidity_vault,
                    new_instruction_fund_remove_liquidity_vault,
                    new_instruction_fund_unlock_liquidity_vault,
                    all_instructions_token_transfer,
                    all_instructions_wrap_sol,
                    all_instructions_unwrap_sol,
                    all_instructions_add_liquidity_vault,
                    all_instructions_add_locked_liquidity_vault,
                    all_instructions_remove_liquidity_vault,
                    all_instructions_remove_unlocked_liquidity_vault,
                    all_instructions_add_liquidity_pool,
                    all_instructions_remove_liquidity_pool,
                    all_instructions_swap,
                    all_instructions_stake,
                    all_instructions_unstake,
                    all_instructions_harvest,
                    all_instructions_request_deposit_fund,
                    all_instructions_request_withdrawal_fund,
                    all_instructions_fund_swap,
                    all_instructions_fund_add_liquidity_pool,
                    all_instructions_fund_remove_liquidity_pool,
                    all_instructions_fund_stake,
                    all_instructions_fund_unstake,
                    all_instructions_fund_harvest,
                    all_instructions_fund_add_liquidity_vault,
                    all_instructions_fund_add_locked_liquidity_vault,
                    all_instructions_fund_remove_liquidity_vault,
                    all_instructions_fund_remove_unlocked_liquidity_vault,
                ],
            )
    })
}
