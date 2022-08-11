//! Pools loader.

use {
    crate::{config::Config, loaders::utils::*},
    log::info,
    serde::Deserialize,
    serde_json::{json, Value},
    solana_farm_client::client::FarmClient,
    solana_farm_sdk::{
        pack::{optional_pubkey_deserialize, pubkey_deserialize},
        pool::{Pool, PoolRoute, PoolType},
        refdb::StorageType,
        string::str_to_as64,
        token::GitToken,
    },
    solana_sdk::pubkey::Pubkey,
    std::collections::HashMap,
    std::str::FromStr,
};

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct JsonRaydiumPoolLegacy {
    pub name: String,
    pub coin: String,
    pub pc: String,
    pub lp: String,
    pub version: u8,
    #[serde(rename = "programId")]
    pub program_id: String,
    #[serde(rename = "ammId", deserialize_with = "pubkey_deserialize")]
    pub amm_id: Pubkey,
    #[serde(rename = "ammAuthority", deserialize_with = "pubkey_deserialize")]
    pub amm_authority: Pubkey,
    #[serde(rename = "ammOpenOrders", deserialize_with = "pubkey_deserialize")]
    pub amm_open_orders: Pubkey,
    #[serde(rename = "ammTargetOrders", deserialize_with = "pubkey_deserialize")]
    pub amm_target_orders: Pubkey,
    #[serde(rename = "ammQuantities", deserialize_with = "pubkey_deserialize")]
    pub amm_quantities: Pubkey,
    #[serde(
        rename = "poolCoinTokenAccount",
        deserialize_with = "pubkey_deserialize"
    )]
    pub pool_coin_token_account: Pubkey,
    #[serde(rename = "poolPcTokenAccount", deserialize_with = "pubkey_deserialize")]
    pub pool_pc_token_account: Pubkey,
    #[serde(rename = "poolWithdrawQueue", deserialize_with = "pubkey_deserialize")]
    pub pool_withdraw_queue: Pubkey,
    #[serde(
        rename = "poolTempLpTokenAccount",
        deserialize_with = "pubkey_deserialize"
    )]
    pub pool_temp_lp_token_account: Pubkey,
    #[serde(rename = "serumProgramId")]
    pub serum_program_id: String,
    #[serde(rename = "serumMarket", deserialize_with = "pubkey_deserialize")]
    pub serum_market: Pubkey,
    #[serde(
        rename = "serumBids",
        deserialize_with = "optional_pubkey_deserialize",
        default
    )]
    pub serum_bids: Option<Pubkey>,
    #[serde(
        rename = "serumAsks",
        deserialize_with = "optional_pubkey_deserialize",
        default
    )]
    pub serum_asks: Option<Pubkey>,
    #[serde(
        rename = "serumEventQueue",
        deserialize_with = "optional_pubkey_deserialize",
        default
    )]
    pub serum_event_queue: Option<Pubkey>,
    #[serde(
        rename = "serumCoinVaultAccount",
        deserialize_with = "pubkey_deserialize"
    )]
    pub serum_coin_vault_account: Pubkey,
    #[serde(
        rename = "serumPcVaultAccount",
        deserialize_with = "pubkey_deserialize"
    )]
    pub serum_pc_vault_account: Pubkey,
    #[serde(rename = "serumVaultSigner", deserialize_with = "pubkey_deserialize")]
    pub serum_vault_signer: Pubkey,
    pub official: bool,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct JsonRaydiumPool {
    #[serde(deserialize_with = "pubkey_deserialize")]
    pub id: Pubkey,
    #[serde(rename = "baseMint", deserialize_with = "pubkey_deserialize")]
    pub base_mint: Pubkey,
    #[serde(rename = "quoteMint", deserialize_with = "pubkey_deserialize")]
    pub quote_mint: Pubkey,
    #[serde(rename = "lpMint", deserialize_with = "pubkey_deserialize")]
    pub lp_mint: Pubkey,
    #[serde(rename = "baseDecimals")]
    pub base_decimals: u8,
    #[serde(rename = "quoteDecimals")]
    pub quote_decimals: u8,
    #[serde(rename = "lpDecimals")]
    pub lp_decimals: u8,
    pub version: u8,
    #[serde(rename = "programId", deserialize_with = "pubkey_deserialize")]
    pub program_id: Pubkey,
    #[serde(deserialize_with = "pubkey_deserialize")]
    pub authority: Pubkey,
    #[serde(rename = "openOrders", deserialize_with = "pubkey_deserialize")]
    pub open_orders: Pubkey,
    #[serde(rename = "targetOrders", deserialize_with = "pubkey_deserialize")]
    pub target_orders: Pubkey,
    #[serde(rename = "baseVault", deserialize_with = "pubkey_deserialize")]
    pub base_vault: Pubkey,
    #[serde(rename = "quoteVault", deserialize_with = "pubkey_deserialize")]
    pub quote_vault: Pubkey,
    #[serde(rename = "withdrawQueue", deserialize_with = "pubkey_deserialize")]
    pub withdraw_queue: Pubkey,
    #[serde(rename = "lpVault", deserialize_with = "pubkey_deserialize")]
    pub lp_vault: Pubkey,
    #[serde(rename = "marketVersion")]
    pub market_version: u8,
    #[serde(rename = "marketProgramId", deserialize_with = "pubkey_deserialize")]
    pub market_program_id: Pubkey,
    #[serde(rename = "marketId", deserialize_with = "pubkey_deserialize")]
    pub market_id: Pubkey,
    #[serde(rename = "marketAuthority", deserialize_with = "pubkey_deserialize")]
    pub market_authority: Pubkey,
    #[serde(rename = "marketBaseVault", deserialize_with = "pubkey_deserialize")]
    pub market_base_vault: Pubkey,
    #[serde(rename = "marketQuoteVault", deserialize_with = "pubkey_deserialize")]
    pub market_quote_vault: Pubkey,
    #[serde(rename = "marketBids", deserialize_with = "pubkey_deserialize")]
    pub market_bids: Pubkey,
    #[serde(rename = "marketAsks", deserialize_with = "pubkey_deserialize")]
    pub market_asks: Pubkey,
    #[serde(rename = "marketEventQueue", deserialize_with = "pubkey_deserialize")]
    pub market_event_queue: Pubkey,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct JsonSaberPool {
    pub name: String,
    pub tokens: Vec<GitToken>,
    #[serde(rename = "lpToken")]
    pub lp_token: GitToken,

    #[serde(deserialize_with = "pubkey_deserialize")]
    pub quarry: Pubkey,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct JsonOrcaToken {
    pub tag: String,
    pub name: String,
    #[serde(deserialize_with = "pubkey_deserialize")]
    pub mint: Pubkey,
    pub scale: u8,
    #[serde(deserialize_with = "pubkey_deserialize")]
    pub addr: Pubkey,
}

#[derive(Deserialize, Debug)]
pub struct JsonOrcaPool {
    pub name: String,
    #[serde(deserialize_with = "pubkey_deserialize")]
    pub address: Pubkey,
    pub nonce: u8,
    #[serde(deserialize_with = "pubkey_deserialize")]
    pub authority: Pubkey,
    #[serde(rename = "poolTokenMint", deserialize_with = "pubkey_deserialize")]
    pub pool_token_mint: Pubkey,
    #[serde(rename = "poolTokenDecimals")]
    pub pool_token_decimals: u8,
    #[serde(rename = "feeAccount", deserialize_with = "pubkey_deserialize")]
    pub fee_account: Pubkey,
    #[serde(rename = "tokenIds")]
    pub token_ids: Vec<String>,
    pub tokens: HashMap<String, JsonOrcaToken>,
    #[serde(rename = "curveType")]
    pub curve_type: u8,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

pub fn load(client: &FarmClient, config: &Config, data: &str, remove_mode: bool) {
    let parsed: Value = serde_json::from_str(data).unwrap();
    let last_index = client
        .get_refdb_last_index(&StorageType::Pool.to_string())
        .expect("Pool RefDB query error");

    if parsed["name"] == "Raydium Pools" {
        load_raydium_pool_legacy(client, config, remove_mode, &parsed, last_index);
    } else if parsed["name"] == "Raydium Mainnet Liquidity Pools" {
        load_raydium_pool(client, config, remove_mode, &parsed, last_index);
    } else if parsed["name"] == "Orca Pools" {
        load_orca_pool(client, config, remove_mode, &parsed, last_index);
    } else if parsed["pools"] != json!(null) && parsed["addresses"] != json!(null) {
        load_saber_pool(client, config, remove_mode, &parsed, last_index);
    } else {
        panic!("Unsupported pools file");
    }
}

fn load_raydium_pool_legacy(
    client: &FarmClient,
    config: &Config,
    remove_mode: bool,
    parsed: &Value,
    last_index: u32,
) {
    let mut last_index = last_index;
    let pools = parsed["pools"].as_array().unwrap();
    let router_id = client.get_program_id("RaydiumRouter").unwrap();
    for val in pools {
        let json_pool: JsonRaydiumPoolLegacy = serde_json::from_value(val.clone()).unwrap();
        let token_a = client
            .get_token_with_account(&json_pool.pool_coin_token_account)
            .unwrap();
        let token_b = client
            .get_token_with_account(&json_pool.pool_pc_token_account)
            .unwrap();
        let name = format!(
            "RDM.{}-{}-V{}",
            token_a.name, token_b.name, json_pool.version
        );
        if !remove_mode {
            if config.skip_existing && client.get_pool(&name).is_ok() {
                info!("Skipping existing Pool \"{}\"...", name);
                continue;
            }
            info!("Writing Pool \"{}\" to on-chain RefDB...", name);
        } else {
            info!("Removing Pool \"{}\" from on-chain RefDB...", name);
            client.remove_pool(config.keypair.as_ref(), &name).unwrap();
            continue;
        }
        let (index, counter) = if let Ok(pool) = client.get_pool(&name) {
            (pool.refdb_index, pool.refdb_counter)
        } else {
            last_index += 1;
            (Some(last_index - 1), 0u16)
        };

        let pool = Pool {
            name: str_to_as64(&name).unwrap(),
            version: json_pool.version as u16,
            pool_type: PoolType::Amm,
            official: json_pool.official,
            refdb_index: index,
            refdb_counter: counter,
            token_a_ref: Some(client.get_token_ref(&token_a.name).unwrap()),
            token_b_ref: Some(client.get_token_ref(&token_b.name).unwrap()),
            lp_token_ref: Some(client.get_token_ref(&json_pool.lp.to_uppercase()).unwrap()),
            token_a_account: Some(json_pool.pool_coin_token_account),
            token_b_account: Some(json_pool.pool_pc_token_account),
            router_program_id: router_id,
            pool_program_id: convert_raydium_program_id(client, &json_pool.program_id),
            route: PoolRoute::Raydium {
                amm_id: json_pool.amm_id,
                amm_authority: json_pool.amm_authority,
                amm_open_orders: json_pool.amm_open_orders,
                amm_target: if json_pool.version == 4 {
                    json_pool.amm_target_orders
                } else {
                    json_pool.amm_quantities
                },
                pool_withdraw_queue: json_pool.pool_withdraw_queue,
                pool_temp_lp_token_account: json_pool.pool_temp_lp_token_account,
                serum_program_id: convert_serum_program_id(client, &json_pool.serum_program_id),
                serum_market: json_pool.serum_market,
                serum_coin_vault_account: json_pool.serum_coin_vault_account,
                serum_pc_vault_account: json_pool.serum_pc_vault_account,
                serum_vault_signer: json_pool.serum_vault_signer,
                serum_bids: json_pool.serum_bids,
                serum_asks: json_pool.serum_asks,
                serum_event_queue: json_pool.serum_event_queue,
            },
        };

        client.add_pool(config.keypair.as_ref(), pool).unwrap();
    }
}

fn load_raydium_pool(
    client: &FarmClient,
    config: &Config,
    remove_mode: bool,
    parsed: &Value,
    last_index: u32,
) {
    let mut last_index = last_index;
    let pools = parsed["official"].as_array().unwrap();
    let router_id = client.get_program_id("RaydiumRouter").unwrap();
    for val in pools {
        let json_pool: JsonRaydiumPool = serde_json::from_value(val.clone()).unwrap();
        let token_a = client.get_token_with_mint(&json_pool.base_mint).unwrap();
        let token_b = client.get_token_with_mint(&json_pool.quote_mint).unwrap();
        let lp_token = client.get_token_with_mint(&json_pool.lp_mint).unwrap();
        let pool_type = get_raydium_pool_type(&json_pool);
        let name = format!(
            "RDM.{}-{}-V{}",
            token_a.name, token_b.name, json_pool.version
        );
        if !remove_mode {
            if config.skip_existing && client.get_pool(&name).is_ok() {
                info!("Skipping existing Pool \"{}\"...", name);
                continue;
            } else if pool_type == PoolType::AmmStable {
                info!("Skipping stablecoin Pool \"{}\"...", name);
                continue;
            }
            info!("Writing Pool \"{}\" to on-chain RefDB...", name);
        } else {
            info!("Removing Pool \"{}\" from on-chain RefDB...", name);
            client.remove_pool(config.keypair.as_ref(), &name).unwrap();
            continue;
        }
        let (index, counter) = if let Ok(pool) = client.get_pool(&name) {
            (pool.refdb_index, pool.refdb_counter)
        } else {
            last_index += 1;
            (Some(last_index - 1), 0u16)
        };

        let pool = Pool {
            name: str_to_as64(&name).unwrap(),
            version: json_pool.version as u16,
            pool_type: PoolType::Amm,
            official: true,
            refdb_index: index,
            refdb_counter: counter,
            token_a_ref: Some(client.get_token_ref(&token_a.name).unwrap()),
            token_b_ref: Some(client.get_token_ref(&token_b.name).unwrap()),
            lp_token_ref: Some(client.get_token_ref(&lp_token.name).unwrap()),
            token_a_account: Some(json_pool.base_vault),
            token_b_account: Some(json_pool.quote_vault),
            router_program_id: router_id,
            pool_program_id: json_pool.program_id,
            route: PoolRoute::Raydium {
                amm_id: json_pool.id,
                amm_authority: json_pool.authority,
                amm_open_orders: json_pool.open_orders,
                amm_target: json_pool.target_orders,
                pool_withdraw_queue: json_pool.withdraw_queue,
                pool_temp_lp_token_account: json_pool.lp_vault,
                serum_program_id: json_pool.market_program_id,
                serum_market: json_pool.market_id,
                serum_coin_vault_account: json_pool.market_base_vault,
                serum_pc_vault_account: json_pool.market_quote_vault,
                serum_vault_signer: json_pool.market_authority,
                serum_bids: if json_pool.market_bids == Pubkey::default() {
                    None
                } else {
                    Some(json_pool.market_bids)
                },
                serum_asks: if json_pool.market_asks == Pubkey::default() {
                    None
                } else {
                    Some(json_pool.market_asks)
                },
                serum_event_queue: if json_pool.market_event_queue == Pubkey::default() {
                    None
                } else {
                    Some(json_pool.market_event_queue)
                },
            },
        };

        client.add_pool(config.keypair.as_ref(), pool).unwrap();
    }
}

fn load_saber_pool(
    client: &FarmClient,
    config: &Config,
    remove_mode: bool,
    parsed: &Value,
    last_index: u32,
) {
    let mut last_index = last_index;
    let pools = parsed["pools"].as_array().unwrap();
    let router_id = client.get_program_id("SaberRouter").unwrap();
    let decimal_wrapper_program = client.get_program_id("SaberDecimalWrapper").unwrap();
    for val in pools {
        let json_pool: JsonSaberPool = serde_json::from_value(val.clone()).unwrap();
        let name = get_saber_pool_name(client, &json_pool.tokens[0], &json_pool.tokens[1]);
        if !remove_mode {
            if config.skip_existing && client.get_pool(&name).is_ok() {
                info!("Skipping existing Pool \"{}\"...", name);
                continue;
            }
            info!("Writing Pool \"{}\" to on-chain RefDB...", name);
        } else {
            info!("Removing Pool \"{}\" from on-chain RefDB...", name);
            client.remove_pool(config.keypair.as_ref(), &name).unwrap();
            continue;
        }
        let (index, counter) = if let Ok(pool) = client.get_pool(&name) {
            (pool.refdb_index, pool.refdb_counter)
        } else {
            last_index += 1;
            (Some(last_index - 1), 0u16)
        };
        let pool_token_name = get_saber_token_name(client, &json_pool.lp_token);
        if json_pool.tokens[0].address
            != convert_pubkey(val["swap"]["state"]["tokenA"]["mint"].as_str().unwrap())
            || json_pool.tokens[1].address
                != convert_pubkey(val["swap"]["state"]["tokenB"]["mint"].as_str().unwrap())
        {
            panic!("Pool metadata mismatch");
        }

        // check if there are Saber wrapped symbols
        let token1_wrapped = is_saber_wrapped(&json_pool.tokens[0]);
        let token2_wrapped = is_saber_wrapped(&json_pool.tokens[1]);
        let token_a_symbol = get_saber_token_name(client, &json_pool.tokens[0]);
        let token_b_symbol = get_saber_token_name(client, &json_pool.tokens[1]);

        // wrapped token refs
        let wrapped_token_a_ref = if token1_wrapped {
            Some(get_token_ref_with_mint(
                client,
                &json_pool.tokens[0].address,
            ))
        } else {
            None
        };
        let wrapped_token_b_ref = if token2_wrapped {
            Some(get_token_ref_with_mint(
                client,
                &json_pool.tokens[1].address,
            ))
        } else {
            None
        };

        // wrappers
        let (decimal_wrapper_token_a, wrapped_token_a_vault) = if token1_wrapped {
            let (a, b) = get_saber_wrappers(client, &json_pool.tokens[0].symbol, &token_a_symbol);
            (Some(a), Some(b))
        } else {
            (None, None)
        };
        let (decimal_wrapper_token_b, wrapped_token_b_vault) = if token2_wrapped {
            let (a, b) = get_saber_wrappers(client, &json_pool.tokens[1].symbol, &token_b_symbol);
            (Some(a), Some(b))
        } else {
            (None, None)
        };

        let pool = Pool {
            name: str_to_as64(&name).unwrap(),
            version: 1u16,
            pool_type: PoolType::AmmStable,
            official: true,
            refdb_index: index,
            refdb_counter: counter,
            token_a_ref: Some(client.get_token_ref(&token_a_symbol).unwrap()),
            token_b_ref: Some(client.get_token_ref(&token_b_symbol).unwrap()),
            lp_token_ref: Some(client.get_token_ref(&pool_token_name).unwrap()),
            token_a_account: Some(json_to_pubkey(&val["swap"]["state"]["tokenA"]["reserve"])),
            token_b_account: Some(json_to_pubkey(&val["swap"]["state"]["tokenB"]["reserve"])),
            router_program_id: router_id,
            pool_program_id: json_to_pubkey(&val["swap"]["config"]["swapProgramID"]),
            route: PoolRoute::Saber {
                swap_account: json_to_pubkey(&val["swap"]["config"]["swapAccount"]),
                swap_authority: json_to_pubkey(&val["swap"]["config"]["authority"]),
                fees_account_a: json_to_pubkey(&val["swap"]["state"]["tokenA"]["adminFeeAccount"]),
                fees_account_b: json_to_pubkey(&val["swap"]["state"]["tokenB"]["adminFeeAccount"]),
                decimal_wrapper_program,
                wrapped_token_a_ref,
                wrapped_token_a_vault,
                decimal_wrapper_token_a,
                wrapped_token_b_ref,
                wrapped_token_b_vault,
                decimal_wrapper_token_b,
            },
        };

        client.add_pool(config.keypair.as_ref(), pool).unwrap();
    }
}

fn load_orca_pool(
    client: &FarmClient,
    config: &Config,
    remove_mode: bool,
    parsed: &Value,
    last_index: u32,
) {
    let mut last_index = last_index;
    let pools = parsed["pools"].as_array().unwrap();
    let router_id = client.get_program_id("OrcaRouter").unwrap();
    let pool_program_id = client.get_program_id("OrcaSwap").unwrap();
    for val in pools {
        let json_pool: JsonOrcaPool = serde_json::from_value(val.clone()).unwrap();
        let token_a = client
            .get_token_with_mint(&convert_pubkey(&json_pool.token_ids[0]))
            .unwrap();
        let token_b = client
            .get_token_with_mint(&convert_pubkey(&json_pool.token_ids[1]))
            .unwrap();
        let pool_type = get_orca_pool_type(&json_pool);
        let name = format!("ORC.{}-{}-V1", token_a.name, token_b.name);
        if !remove_mode {
            if config.skip_existing && client.get_pool(&name).is_ok() {
                info!("Skipping existing Pool \"{}\"...", name);
                continue;
            } else if pool_type == PoolType::AmmStable {
                info!("Skipping stablecoin Pool \"{}\"...", name);
                continue;
            }
            info!("Writing Pool \"{}\" to on-chain RefDB...", name);
        } else {
            info!("Removing Pool \"{}\" from on-chain RefDB...", name);
            client.remove_pool(config.keypair.as_ref(), &name).unwrap();
            continue;
        }
        let (index, counter) = if let Ok(pool) = client.get_pool(&name) {
            (pool.refdb_index, pool.refdb_counter)
        } else {
            last_index += 1;
            (Some(last_index - 1), 0u16)
        };
        // validate mints
        if token_a.mint != json_pool.tokens[&json_pool.token_ids[0]].mint
            || token_b.mint != json_pool.tokens[&json_pool.token_ids[1]].mint
        {
            panic!("Pool metadata mismatch");
        }
        let pool = Pool {
            name: str_to_as64(&name).unwrap(),
            version: 1,
            pool_type: if json_pool.curve_type == 0 {
                PoolType::Amm
            } else {
                PoolType::AmmStable
            },
            official: true,
            refdb_index: index,
            refdb_counter: counter,
            token_a_ref: Some(client.get_token_ref(&token_a.name).unwrap()),
            token_b_ref: Some(client.get_token_ref(&token_b.name).unwrap()),
            lp_token_ref: Some(get_token_ref_with_mint(client, &json_pool.pool_token_mint)),
            token_a_account: Some(json_pool.tokens[&json_pool.token_ids[0]].addr),
            token_b_account: Some(json_pool.tokens[&json_pool.token_ids[1]].addr),
            router_program_id: router_id,
            pool_program_id,
            route: PoolRoute::Orca {
                amm_id: json_pool.address,
                amm_authority: json_pool.authority,
                fees_account: json_pool.fee_account,
            },
        };

        client.add_pool(config.keypair.as_ref(), pool).unwrap();
    }
}

fn get_raydium_pool_type(pool: &JsonRaydiumPool) -> PoolType {
    match pool.version {
        0..=4 => PoolType::Amm,
        5 => PoolType::AmmStable,
        _ => panic!("Unrecognized Raydium pool type: {}", pool.version),
    }
}

fn get_orca_pool_type(pool: &JsonOrcaPool) -> PoolType {
    match pool.curve_type {
        0 => PoolType::Amm,
        2 => PoolType::AmmStable,
        _ => panic!("Unrecognized Orca pool type: {}", pool.curve_type),
    }
}

fn get_saber_wrappers(
    client: &FarmClient,
    saber_symbol: &str,
    original_symbol: &str,
) -> (Pubkey, Pubkey) {
    let token = client.get_token(original_symbol).unwrap();
    let decimals = saber_symbol
        .split('-')
        .last()
        .unwrap()
        .parse::<u8>()
        .unwrap();
    let decimal_wrapper_program = client.get_program_id("SaberDecimalWrapper").unwrap();

    let wrapper = Pubkey::find_program_address(
        &[b"anchor", &token.mint.to_bytes(), &[decimals]],
        &decimal_wrapper_program,
    )
    .0;
    // wrapper_vault can be fetched with:
    // async function fetch_wrapper_vault(wrapper_program, wrapper) {
    //   const idl = JSON.parse(
    //     require("fs").readFileSync(
    //       "./add_decimals_idl.json", "utf8"
    //     )
    //   );
    //   const programId = new anchor.web3.PublicKey(wrapper_program);
    //   const program = new anchor.Program(idl, programId);
    //   console.log(
    //     (
    //       await program.account.wrappedToken.fetch(wrapper)
    //     ).wrapperUnderlyingTokens.toString()
    //   );
    // }
    let wrapper_vault = match saber_symbol {
        "swhETH-9" => "4fUL9yLbFZEuG32SaCjWqJXwDTBFNnipteBWxMvvFoC8",
        "swFTT-9" => "5yugfArBAUZJJBUCRWPuiLyi6CWp1f67H9xgg3hcgSkx",
        "srenBTC-10" => "764FaQrrREvNTpaH2yXyrPZgVBaXA7AXM8vyCaevXitD",
        "srenBTC-9" => "C39Wq6X98TLcrnYCMkcHQhwUurkQMUdibUCpf2fVBDsm",
        "srenLUNA-9" => "4R6PmC8BJcPDBsEMGpXpLCnFFkUZhEgZy6pMNtc2LqA4",
        "sUSDC-8" => "AQhP39mE4o6BYNwnwYqnz7ZobkPBSLpCg8WvEESq1viZ",
        "sUSDC-9" => "77XHXCWYQ76E9Q3uCuz1geTaxsqJZf9RfX5ZY7yyLDYt",
        "sUSDT-8" => "3cjAWoyDcco8UVCN17keNUNHoyz37ctgDa7G6zkeb81Y",
        "sUSDT-9" => "BSTjdztBrsptuxfz9JHS31Wc9CknpLeL1wqZjeVs1Ths",
        "sBTC-8" => "6hYDFhZ5ddfzoqaAbzRHm8mzG2MQzYQV9295sQHsvNBV",
        "sBTC-9" => "B22gDMgN2tNWmvyzhb5tamJKanWcUUUw2zN3h3qjgQg8",
        "sETH-8" => "4JWyJ4ZYsQ8uiYue2tTEqcHcFXrDuaQ1rsyjNFfrZm65",
        "sETH-9" => "4fUL9yLbFZEuG32SaCjWqJXwDTBFNnipteBWxMvvFoC8",
        "sFTT-9" => "H5tnZcfHCzHueNnfd6foeBBUUW4g7qXKt6rKzT7wg6oP",
        "ssoFTT-8" => "7dVPR6jx3hKyNfuHPo3WtWdUpH4eh4Up4rfFhLHZqwy3",
        "sagEUR-9" => "8YC5eCS99umbK9K9LnHnTMMjnr7EWg1gam5maNB6uf9d",
        "sCASH-8" => "5s2et753hMXV945U3p5uz6RQqMkZGCPEjKjNPdUcCLLF",
        "sCASH-9" => "3YCGgStAV9H7TdPYdBnRP8yoH4Zqdmyt7xo6KB4Wa8xt",
        "sLUNA-9" => "AvqMJWHsZscPWTAUcj8dZi2ch6XQEHMpiCMprfFovaU",
        "sUST-8" => "9YB1zRL4ETuQFG8ZK1yD4GHBVDmH81EzwuSj75zdnKhK",
        "sUST-9" => "GxpyQZi5VkZDSq5TUycMau11sCkQkVCa8xYhBgiPMsyK",
        "ssoFTT-9" => "H5tnZcfHCzHueNnfd6foeBBUUW4g7qXKt6rKzT7wg6oP",
        "ssoETH-8" => "4JWyJ4ZYsQ8uiYue2tTEqcHcFXrDuaQ1rsyjNFfrZm65",
        _ => {
            panic!(
                "Unknown Saber wrapped token {} with wrapper {} and program {}",
                saber_symbol, wrapper, decimal_wrapper_program
            );
        }
    };
    (wrapper, Pubkey::from_str(wrapper_vault).unwrap())
}
