//! Pools loader.

use {
    crate::{config::Config, loaders::utils::*},
    log::info,
    serde::Deserialize,
    serde_json::{json, Value},
    solana_farm_client::client::FarmClient,
    solana_farm_sdk::{
        git_token::GitToken,
        pack::{optional_pubkey_deserialize, pubkey_deserialize},
        pool::{Pool, PoolRoute, PoolType},
        refdb::StorageType,
        string::str_to_as64,
    },
    solana_sdk::pubkey::Pubkey,
    std::collections::HashMap,
    std::str::FromStr,
};

#[derive(Deserialize, Debug)]
struct JsonRaydiumPool {
    name: String,
    coin: String,
    pc: String,
    lp: String,
    version: u8,
    #[serde(rename = "programId")]
    program_id: String,
    #[serde(rename = "ammId", deserialize_with = "pubkey_deserialize")]
    amm_id: Pubkey,
    #[serde(rename = "ammAuthority", deserialize_with = "pubkey_deserialize")]
    amm_authority: Pubkey,
    #[serde(rename = "ammOpenOrders", deserialize_with = "pubkey_deserialize")]
    amm_open_orders: Pubkey,
    #[serde(rename = "ammTargetOrders", deserialize_with = "pubkey_deserialize")]
    amm_target_orders: Pubkey,
    #[serde(rename = "ammQuantities", deserialize_with = "pubkey_deserialize")]
    amm_quantities: Pubkey,
    #[serde(
        rename = "poolCoinTokenAccount",
        deserialize_with = "pubkey_deserialize"
    )]
    pool_coin_token_account: Pubkey,
    #[serde(rename = "poolPcTokenAccount", deserialize_with = "pubkey_deserialize")]
    pool_pc_token_account: Pubkey,
    #[serde(rename = "poolWithdrawQueue", deserialize_with = "pubkey_deserialize")]
    pool_withdraw_queue: Pubkey,
    #[serde(
        rename = "poolTempLpTokenAccount",
        deserialize_with = "pubkey_deserialize"
    )]
    pool_temp_lp_token_account: Pubkey,
    #[serde(rename = "serumProgramId")]
    serum_program_id: String,
    #[serde(rename = "serumMarket", deserialize_with = "pubkey_deserialize")]
    serum_market: Pubkey,
    #[serde(
        rename = "serumBids",
        deserialize_with = "optional_pubkey_deserialize",
        default
    )]
    serum_bids: Option<Pubkey>,
    #[serde(
        rename = "serumAsks",
        deserialize_with = "optional_pubkey_deserialize",
        default
    )]
    serum_asks: Option<Pubkey>,
    #[serde(
        rename = "serumEventQueue",
        deserialize_with = "optional_pubkey_deserialize",
        default
    )]
    serum_event_queue: Option<Pubkey>,
    #[serde(
        rename = "serumCoinVaultAccount",
        deserialize_with = "pubkey_deserialize"
    )]
    serum_coin_vault_account: Pubkey,
    #[serde(
        rename = "serumPcVaultAccount",
        deserialize_with = "pubkey_deserialize"
    )]
    serum_pc_vault_account: Pubkey,
    #[serde(rename = "serumVaultSigner", deserialize_with = "pubkey_deserialize")]
    serum_vault_signer: Pubkey,
    official: bool,
}

#[derive(Deserialize, Debug)]
struct JsonSaberPool {
    name: String,
    tokens: Vec<GitToken>,
    #[serde(rename = "lpToken")]
    lp_token: GitToken,
    #[serde(deserialize_with = "pubkey_deserialize")]
    quarry: Pubkey,
}

#[derive(Deserialize, Debug)]
pub struct JsonOrcaToken {
    tag: String,
    name: String,
    #[serde(deserialize_with = "pubkey_deserialize")]
    mint: Pubkey,
    scale: u8,
    #[serde(deserialize_with = "pubkey_deserialize")]
    addr: Pubkey,
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
        load_raydium_pool(client, config, remove_mode, &parsed, last_index);
    } else if parsed["name"] == "Orca Pools" {
        load_orca_pool(client, config, remove_mode, &parsed, last_index);
    } else if parsed["pools"] != json!(null) && parsed["addresses"] != json!(null) {
        load_saber_pool(client, config, remove_mode, &parsed, last_index);
    } else {
        panic!("Unsupported pools file");
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
    let pools = parsed["pools"].as_array().unwrap();
    let router_id = client.get_program_id(&"RaydiumRouter".to_string()).unwrap();
    for val in pools {
        let json_pool: JsonRaydiumPool = serde_json::from_value(val.clone()).unwrap();
        let name = format!(
            "RDM.{}-V{}",
            json_pool.name.to_uppercase(),
            json_pool.version
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
            token_a_ref: Some(
                client
                    .get_token_ref(&json_pool.coin.to_uppercase())
                    .unwrap(),
            ),
            token_b_ref: Some(client.get_token_ref(&json_pool.pc.to_uppercase()).unwrap()),
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

fn load_saber_pool(
    client: &FarmClient,
    config: &Config,
    remove_mode: bool,
    parsed: &Value,
    last_index: u32,
) {
    let mut last_index = last_index;
    let pools = parsed["pools"].as_array().unwrap();
    let router_id = client.get_program_id(&"SaberRouter".to_string()).unwrap();
    let decimal_wrapper_program = client
        .get_program_id(&"SaberDecimalWrapper".to_string())
        .unwrap();
    for val in pools {
        let json_pool: JsonSaberPool = serde_json::from_value(val.clone()).unwrap();
        let name = get_saber_pool_name(&json_pool.tokens[0], &json_pool.tokens[1]);
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
        let pool_token_name = get_saber_lp_token_name(&json_pool.lp_token.name);
        if json_pool.tokens[0].address != val["swap"]["state"]["tokenA"]["mint"]
            || json_pool.tokens[1].address != val["swap"]["state"]["tokenB"]["mint"]
        {
            panic!("Pool metadata mismatch");
        }

        // check if there are Saber wrapped symbols
        let token1_wrapped = is_saber_wrapped(&json_pool.tokens[0]);
        let token2_wrapped = is_saber_wrapped(&json_pool.tokens[1]);
        let symbol1 = normalize_name(&json_pool.tokens[0].symbol, false);
        let symbol2 = normalize_name(&json_pool.tokens[1].symbol, false);

        let token_a_symbol = if token1_wrapped {
            let symbol = extract_saber_wrapped_token_name(&symbol1);
            if client.get_token(&symbol).unwrap().mint.to_string()
                != json_pool.tokens[0].extra["extensions"]["assetContract"]
                    .as_str()
                    .unwrap()
            {
                panic!(
                    "Unwrapped token address mismatch for token {}",
                    json_pool.tokens[0].symbol
                );
            }
            symbol
        } else {
            symbol1.clone()
        };

        let token_b_symbol = if token2_wrapped {
            let symbol = extract_saber_wrapped_token_name(&symbol2);
            if client.get_token(&symbol).unwrap().mint.to_string()
                != json_pool.tokens[1].extra["extensions"]["assetContract"]
                    .as_str()
                    .unwrap()
            {
                panic!(
                    "Unwrapped token address mismatch for token {}",
                    json_pool.tokens[1].symbol
                );
            }
            symbol
        } else {
            symbol2.clone()
        };

        // wrapped token refs
        let wrapped_token_a_ref = if token1_wrapped {
            Some(client.get_token_ref(&symbol1).unwrap())
        } else {
            None
        };
        let wrapped_token_b_ref = if token2_wrapped {
            Some(client.get_token_ref(&symbol2).unwrap())
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
            token_a_ref: Some(
                client
                    .get_token_ref(&normalize_name(&token_a_symbol, false))
                    .unwrap(),
            ),
            token_b_ref: Some(
                client
                    .get_token_ref(&normalize_name(&token_b_symbol, false))
                    .unwrap(),
            ),
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
    let router_id = client.get_program_id(&"OrcaRouter".to_string()).unwrap();
    let pool_program_id = client.get_program_id(&"OrcaSwap".to_string()).unwrap();
    for val in pools {
        let json_pool: JsonOrcaPool = serde_json::from_value(val.clone()).unwrap();
        let name = format!("ORC.{}-V1", json_pool.name.to_uppercase().replace("_", "-"));
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
            version: 1,
            pool_type: if json_pool.curve_type == 0 {
                PoolType::Amm
            } else {
                PoolType::AmmStable
            },
            official: true,
            refdb_index: index,
            refdb_counter: counter,
            token_a_ref: Some(get_token_ref_with_mint(
                client,
                &convert_pubkey(&json_pool.token_ids[0]),
            )),
            token_b_ref: Some(get_token_ref_with_mint(
                client,
                &convert_pubkey(&json_pool.token_ids[1]),
            )),
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
    let decimal_wrapper_program = client
        .get_program_id(&"SaberDecimalWrapper".to_string())
        .unwrap();

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
        "sUSDT-9" => "BSTjdztBrsptuxfz9JHS31Wc9CknpLeL1wqZjeVs1Ths",
        "sBTC-8" => "6hYDFhZ5ddfzoqaAbzRHm8mzG2MQzYQV9295sQHsvNBV",
        "sBTC-9" => "B22gDMgN2tNWmvyzhb5tamJKanWcUUUw2zN3h3qjgQg8",
        "sETH-8" => "4JWyJ4ZYsQ8uiYue2tTEqcHcFXrDuaQ1rsyjNFfrZm65",
        "sFTT-9" => "H5tnZcfHCzHueNnfd6foeBBUUW4g7qXKt6rKzT7wg6oP",
        _ => {
            panic!("Unknown Saber wrapped token {}", saber_symbol);
        }
    };
    (wrapper, Pubkey::from_str(wrapper_vault).unwrap())
}
