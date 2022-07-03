use {
    solana_farm_client::client::FarmClient,
    solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair, signer::Signer},
};

use crate::{utils, utils::Swap};

const MAX_SOL_BALANCE_TO_USE: f64 = 0.1;

pub fn do_swap(client: &FarmClient, keypair: &Keypair, swap: &Swap) {
    let amount = if swap.amount == 0.0 {
        utils::get_token_or_native_balance(client, &keypair.pubkey(), swap.from_token)
    } else if swap.amount < 0.0 {
        -1.0 * swap.amount
            * utils::get_token_or_native_balance(client, &keypair.pubkey(), swap.from_token)
    } else {
        swap.amount
    };
    if amount < 0.0001 {
        return;
    }
    println!(
        ">> Swap {} {} to {}",
        amount, swap.from_token, swap.to_token
    );
    println!(
        "  Done: {}",
        client
            .swap(
                keypair,
                swap.protocol,
                swap.from_token,
                swap.to_token,
                amount,
                0.0,
            )
            .unwrap()
    );
    let _ = utils::get_balances(
        client,
        &keypair.pubkey(),
        swap.from_token,
        swap.to_token,
        "After swap",
    );
}

pub fn do_add_liquidity(
    client: &FarmClient,
    keypair: &Keypair,
    pool_name: &str,
    max_token_a_ui_amount: f64,
    max_token_b_ui_amount: f64,
) -> f64 {
    println!(
        ">> Add liquidity to {}: {}, {}",
        pool_name, max_token_a_ui_amount, max_token_b_ui_amount
    );
    let (token_a_str, token_b_str, lp_token_name) = client.get_pool_token_names(pool_name).unwrap();
    let lp_balance = utils::get_token_or_native_balance(client, &keypair.pubkey(), &lp_token_name);
    println!(
        "  Done: {}",
        client
            .add_liquidity_pool(
                keypair,
                pool_name,
                max_token_a_ui_amount,
                max_token_b_ui_amount,
            )
            .unwrap()
    );
    let _ = utils::get_balances(
        client,
        &keypair.pubkey(),
        &token_a_str,
        &token_b_str,
        "After add liquidity",
    );
    let _ = utils::get_balance(client, &keypair.pubkey(), &lp_token_name, "LP");
    utils::get_token_or_native_balance(client, &keypair.pubkey(), &lp_token_name) - lp_balance
}

pub fn do_stake(client: &FarmClient, keypair: &Keypair, farm_name: &str, amount: f64) {
    println!(">> Stake liquidity to {}: {}", farm_name, amount);
    let (token_a_str, token_b_str, lp_token_name) = client.get_farm_token_names(farm_name).unwrap();
    println!(
        "  Done: {}",
        client.stake(keypair, farm_name, amount).unwrap()
    );
    let _ = utils::get_balances(
        client,
        &keypair.pubkey(),
        &token_a_str,
        &token_b_str,
        "After stake",
    );
    let _ = utils::get_balance(client, &keypair.pubkey(), &lp_token_name, "LP after stake");
}

pub fn do_harvest(client: &FarmClient, keypair: &Keypair, farm_name: &str) {
    println!(">> Harvest from {}", farm_name);
    let (token_a_str, token_b_str, lp_token_name) = client.get_farm_token_names(farm_name).unwrap();
    println!("  Done: {}", client.harvest(keypair, farm_name).unwrap());
    let _ = utils::get_balances(
        client,
        &keypair.pubkey(),
        &token_a_str,
        &token_b_str,
        "After harvest",
    );
    let _ = utils::get_balance(
        client,
        &keypair.pubkey(),
        &lp_token_name,
        "LP after harvest",
    );
}

pub fn do_unstake(client: &FarmClient, keypair: &Keypair, farm_name: &str, amount: f64) {
    println!(">> Unstake liquidity from {}: {}", farm_name, amount);
    let (token_a_str, token_b_str, lp_token_name) = client.get_farm_token_names(farm_name).unwrap();
    println!(
        "  Done: {}",
        client.unstake(keypair, farm_name, amount).unwrap()
    );
    let _ = utils::get_balances(
        client,
        &keypair.pubkey(),
        &token_a_str,
        &token_b_str,
        "After unstake",
    );
    let _ = utils::get_balance(
        client,
        &keypair.pubkey(),
        &lp_token_name,
        "LP after unstake",
    );
}

pub fn do_remove_liquidity(client: &FarmClient, keypair: &Keypair, pool_name: &str, amount: f64) {
    println!(">> Remove liquidity from {}: {}", pool_name, amount);
    let (token_a_str, token_b_str, lp_token_name) = client.get_pool_token_names(pool_name).unwrap();
    println!(
        "  Done: {}",
        client
            .remove_liquidity_pool(keypair, pool_name, amount)
            .unwrap()
    );
    let _ = utils::get_balances(
        client,
        &keypair.pubkey(),
        &token_a_str,
        &token_b_str,
        "After remove liquidity",
    );
    let _ = utils::get_balance(client, &keypair.pubkey(), &lp_token_name, "LP");
}

pub fn cleanup(
    client: &FarmClient,
    keypair: &Keypair,
    pool_name: &str,
    cleanup_swaps: Vec<Swap>,
    pool_only: bool,
) {
    println!("\n>>> Clean-up {}...", pool_name);
    let wallet = keypair.pubkey();
    let (token_a_str, token_b_str, lp_token_name) = client.get_pool_token_names(pool_name).unwrap();

    if !pool_only {
        let farms = client.find_farms_with_lp(&lp_token_name).unwrap();
        for farm in farms.iter() {
            let farm_token_name =
                "LP.".to_string() + &farm.name.as_str()[..farm.name.as_str().len() - 3];
            if let Ok(dd_farms) = client.find_farms_with_lp(&farm_token_name) {
                for farm in dd_farms.iter() {
                    if let Ok(staked_balance) =
                        client.get_user_stake_balance(&wallet, farm.name.as_str())
                    {
                        if staked_balance > 0.0 {
                            do_unstake(client, keypair, farm.name.as_str(), staked_balance);
                        }
                    }
                }
            }

            if let Ok(staked_balance) = client.get_user_stake_balance(&wallet, farm.name.as_str()) {
                if staked_balance > 0.0 {
                    do_unstake(client, keypair, farm.name.as_str(), staked_balance);
                }
            }
        }
    }

    let lp_token_balance = utils::get_token_or_native_balance(client, &wallet, &lp_token_name);
    if lp_token_balance > 0.0 {
        do_remove_liquidity(client, keypair, pool_name, lp_token_balance);
    }

    for swap in cleanup_swaps {
        do_swap(client, keypair, &swap);
    }

    if token_a_str != "SOL" {
        let token_a_balance = utils::get_token_or_native_balance(client, &wallet, &token_a_str);
        if token_a_balance > 0.0 {
            do_swap(
                client,
                keypair,
                &Swap {
                    protocol: "RDM",
                    from_token: token_a_str.as_str(),
                    to_token: "SOL",
                    amount: token_a_balance,
                },
            );
        }
    }

    if token_b_str != "SOL" {
        let token_b_balance = utils::get_token_or_native_balance(client, &wallet, &token_b_str);
        if token_b_balance > 0.0 {
            do_swap(
                client,
                keypair,
                &Swap {
                    protocol: "RDM",
                    from_token: token_b_str.as_str(),
                    to_token: "SOL",
                    amount: token_b_balance,
                },
            );
        }
    }
}

pub fn run_test(pool_name: &str, swaps: Vec<Swap>, cleanup_swaps: Vec<Swap>, pool_only: bool) {
    let (endpoint, keypair) = utils::get_endpoint_and_keypair();
    let client = FarmClient::new_with_commitment(&endpoint, CommitmentConfig::confirmed());
    let wallet = keypair.pubkey();

    cleanup(
        &client,
        &keypair,
        pool_name,
        cleanup_swaps.clone(),
        pool_only,
    );

    println!("\n>>> Testing {}...", pool_name);
    let (token_a_str, token_b_str, lp_token_name) = client.get_pool_token_names(pool_name).unwrap();

    let (_, _) = utils::get_balances(&client, &wallet, &token_a_str, &token_b_str, "Initial");
    //initial swaps
    for swap in swaps {
        do_swap(&client, &keypair, &swap);
    }

    let token_a_balance = if token_a_str == "SOL" {
        MAX_SOL_BALANCE_TO_USE.min(utils::get_token_or_native_balance(
            &client,
            &wallet,
            &token_a_str,
        ))
    } else {
        utils::get_token_or_native_balance(&client, &wallet, &token_a_str)
    };
    let token_b_balance = if token_b_str == "SOL" {
        MAX_SOL_BALANCE_TO_USE.min(utils::get_token_or_native_balance(
            &client,
            &wallet,
            &token_b_str,
        ))
    } else {
        utils::get_token_or_native_balance(&client, &wallet, &token_b_str)
    };

    assert!(token_a_balance > 0.0 && token_b_balance > 0.0);

    // main tests
    let mut lp_received =
        do_add_liquidity(&client, &keypair, pool_name, token_a_balance / 3.0, 0.0);
    assert!(lp_received > 0.0);
    lp_received += do_add_liquidity(&client, &keypair, pool_name, 0.0, token_b_balance / 3.0);

    if !pool_only {
        let farms = client.find_farms_with_lp(&lp_token_name).unwrap();
        for farm in farms.iter() {
            do_stake(&client, &keypair, farm.name.as_str(), lp_received / 2.0);
            do_stake(&client, &keypair, farm.name.as_str(), 0.0);
            do_harvest(&client, &keypair, farm.name.as_str());

            // orca double-dip farms
            let farm_token_name =
                "LP.".to_string() + &farm.name.as_str()[..farm.name.as_str().len() - 3];
            if let Ok(dd_farms) = client.find_farms_with_lp(&farm_token_name) {
                for dd_farm in dd_farms.iter() {
                    do_stake(&client, &keypair, dd_farm.name.as_str(), lp_received / 2.0);
                    do_stake(&client, &keypair, dd_farm.name.as_str(), 0.0);
                    do_harvest(&client, &keypair, dd_farm.name.as_str());
                    do_unstake(&client, &keypair, dd_farm.name.as_str(), lp_received / 2.0);
                    do_unstake(&client, &keypair, dd_farm.name.as_str(), 0.0);
                }
            }

            do_unstake(&client, &keypair, farm.name.as_str(), lp_received / 2.0);
            do_unstake(&client, &keypair, farm.name.as_str(), 0.0);
        }
    }
    do_remove_liquidity(&client, &keypair, pool_name, lp_received / 2.0);
    do_remove_liquidity(&client, &keypair, pool_name, 0.0);

    cleanup(&client, &keypair, pool_name, cleanup_swaps, pool_only);

    let (_, _) = utils::get_balances(&client, &wallet, &token_a_str, &token_b_str, "Final");
}
