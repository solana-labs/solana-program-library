use {
    crate::{utils, utils::Swap},
    solana_farm_client::client::FarmClient,
    solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair, signer::Signer},
    std::{thread, time},
};

const MAX_SOL_BALANCE_TO_USE: f64 = 0.1;
const INITIAL_CRANK_DELAY: u64 = 400;
const CRANK_INTERVAL: u64 = 100;

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
    vault_name: &str,
    max_token_a_ui_amount: f64,
    max_token_b_ui_amount: f64,
) -> f64 {
    println!(
        ">> Add liquidity to {}: {}, {}",
        vault_name, max_token_a_ui_amount, max_token_b_ui_amount
    );
    let (token_a_str, token_b_str, vt_token_name) =
        client.get_vault_token_names(vault_name).unwrap();
    let vt_balance = utils::get_token_or_native_balance(client, &keypair.pubkey(), &vt_token_name);
    println!(
        "  Done: {}",
        client
            .add_liquidity_vault(
                keypair,
                vault_name,
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
    let _ = utils::get_balance(client, &keypair.pubkey(), &vt_token_name, "VT");
    let _ = utils::get_vault_stake_balance(client, vault_name);
    utils::get_token_or_native_balance(client, &keypair.pubkey(), &vt_token_name) - vt_balance
}

pub fn do_crank(client: &FarmClient, keypair: &Keypair, vault_name: &str, step: u64) {
    println!(">> Crank {} with step {}", vault_name, step);
    let initial_info = client.get_vault_info(vault_name).unwrap();
    println!(
        "  Done: {}",
        client.crank_vault(keypair, vault_name, step).unwrap()
    );
    let after_crank_info = client.get_vault_info(vault_name).unwrap();
    println!(
        "  Rewards received: {}, {}",
        after_crank_info.tokens_a_rewards - initial_info.tokens_a_rewards,
        after_crank_info.tokens_b_rewards - initial_info.tokens_b_rewards
    );
    let _ = utils::get_vault_stake_balance(client, vault_name);
}

pub fn do_remove_liquidity(client: &FarmClient, keypair: &Keypair, vault_name: &str, amount: f64) {
    println!(">> Remove liquidity from {}: {}", vault_name, amount);
    let (token_a_str, token_b_str, vt_token_name) =
        client.get_vault_token_names(vault_name).unwrap();
    println!(
        "  Done: {}",
        client
            .remove_liquidity_vault(keypair, vault_name, amount)
            .unwrap()
    );
    let _ = utils::get_balances(
        client,
        &keypair.pubkey(),
        &token_a_str,
        &token_b_str,
        "After remove liquidity",
    );
    let _ = utils::get_balance(client, &keypair.pubkey(), &vt_token_name, "VT");
    let _ = utils::get_vault_stake_balance(client, vault_name);
}

pub fn cleanup(client: &FarmClient, keypair: &Keypair, vault_name: &str, cleanup_swaps: Vec<Swap>) {
    println!("\n>>> Clean-up {}...", vault_name);
    let wallet = keypair.pubkey();
    let (token_a_str, token_b_str, vt_token_name) =
        client.get_vault_token_names(vault_name).unwrap();

    let vt_token_balance = utils::get_token_or_native_balance(client, &wallet, &vt_token_name);
    if vt_token_balance > 0.0 {
        do_remove_liquidity(client, keypair, vault_name, vt_token_balance);
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

    let _ = utils::get_vault_stake_balance(client, vault_name);
}

pub fn run_test(vault_name: &str, swaps: Vec<Swap>, cleanup_swaps: Vec<Swap>) {
    let (endpoint, keypair) = utils::get_endpoint_and_keypair();
    let client = FarmClient::new_with_commitment(&endpoint, CommitmentConfig::confirmed());
    let wallet = keypair.pubkey();

    cleanup(&client, &keypair, vault_name, cleanup_swaps.clone());

    println!("\n>>> Testing {}...", vault_name);
    let (token_a_str, token_b_str, _) = client.get_vault_token_names(vault_name).unwrap();

    let (_, _) = utils::get_balances(&client, &wallet, &token_a_str, &token_b_str, "Initial");
    let _ = utils::get_vault_stake_balance(&client, vault_name);
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

    // main tests
    let mut vt_received;
    if vault_name.starts_with("SBR.") {
        if token_a_str == "USDC" {
            assert!(token_a_balance > 0.0);
            vt_received = do_add_liquidity(
                &client,
                &keypair,
                vault_name,
                token_a_balance * 2.0 / 3.0,
                0.0,
            );
        } else {
            assert!(token_b_balance > 0.0);
            vt_received = do_add_liquidity(
                &client,
                &keypair,
                vault_name,
                0.0,
                token_b_balance * 2.0 / 3.0,
            );
        }
    } else {
        assert!(token_a_balance > 0.0 && token_b_balance > 0.0);
        vt_received = do_add_liquidity(&client, &keypair, vault_name, token_a_balance / 3.0, 0.0);
        assert!(vt_received > 0.0);
        vt_received += do_add_liquidity(&client, &keypair, vault_name, 0.0, token_b_balance / 3.0);
    }

    println!("Waiting {} secs for rewards...", INITIAL_CRANK_DELAY);
    thread::sleep(time::Duration::from_secs(INITIAL_CRANK_DELAY));
    do_crank(&client, &keypair, vault_name, 1);

    let cranks = if vault_name.starts_with("SBR.") { 6 } else { 4 };
    for step in 2..cranks {
        println!("Waiting {} secs before next crank...", CRANK_INTERVAL);
        thread::sleep(time::Duration::from_secs(CRANK_INTERVAL));
        do_crank(&client, &keypair, vault_name, step);
    }

    do_remove_liquidity(&client, &keypair, vault_name, vt_received / 2.0);
    do_remove_liquidity(&client, &keypair, vault_name, 0.0);

    cleanup(&client, &keypair, vault_name, cleanup_swaps);

    let (_, _) = utils::get_balances(&client, &wallet, &token_a_str, &token_b_str, "Final");
    let _ = utils::get_vault_stake_balance(&client, vault_name);
}
