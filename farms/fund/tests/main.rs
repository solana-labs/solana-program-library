mod fixture;
mod utils;

use {
    log::info,
    solana_farm_client::{client::FarmClient, error::FarmClientError},
    solana_farm_sdk::{
        fund::{
            FundAssetType, FundAssetsTrackingConfig, FundCustodyType, FundSchedule, FundVaultType,
            DISCRIMINATOR_FUND_CUSTODY, DISCRIMINATOR_FUND_USER_REQUESTS, DISCRIMINATOR_FUND_VAULT,
        },
        id::zero,
        string::str_to_as64,
        Protocol,
    },
    solana_sdk::{
        commitment_config::{CommitmentConfig, CommitmentLevel},
        signature::Keypair,
        signer::Signer,
    },
};

#[test]
#[ignore]
// Runs all integration tests. Default config should have rpc url set to
// localhost or devnet and kepair_path should point to the admin keypair.
fn run_tests() -> Result<(), FarmClientError> {
    solana_logger::setup_with_default("main=debug,solana=debug");

    let (endpoint, admin_keypair) = utils::get_endpoint_and_keypair();
    let user_keypair = Keypair::new();
    let manager_keypair = Keypair::new();
    let client = FarmClient::new_with_commitment(&endpoint, CommitmentConfig::confirmed());
    let wallet = user_keypair.pubkey();

    let fund_name = fixture::init_fund(
        &client,
        &admin_keypair,
        &manager_keypair.pubkey(),
        None,
        None,
    )?;
    //let fund_name = "FUND_2196727256".to_string();
    let vault_name = "RDM.COIN-PC-V4";
    let vault_type = FundVaultType::Pool;
    let vault_name2 = "RDM.STC.COIN-PC-V5";
    let token_a = "COIN";
    let token_b = "PC";
    let lp_token = "LP.RDM.COIN-PC-V4";
    let vt_token = "VT.RDM.STC.COIN-PC-V5";
    let amount = 0.2;
    let fund = client.get_fund(&fund_name)?;
    let fund_token = client.get_token_by_ref(&fund.fund_token_ref)?;
    let fund_info = client.get_fund_info(&fund_name)?;
    println!("{:#?}", fund_info);

    // init user for SOL deposit
    info!("Init user");
    let token_name = "SOL";
    assert!(client
        .get_fund_user_requests(&wallet, &fund_name, token_name)
        .is_err());
    client.confirm_async_transaction(
        &client.rpc_client.request_airdrop(
            &manager_keypair.pubkey(),
            client.ui_amount_to_tokens(2.0, "SOL")?,
        )?,
        CommitmentLevel::Confirmed,
    )?;
    for _ in 0..2 {
        client.confirm_async_transaction(
            &client
                .rpc_client
                .request_airdrop(&wallet, client.ui_amount_to_tokens(2.0, "SOL")?)?,
            CommitmentLevel::Confirmed,
        )?;
    }
    client.user_init_fund(&user_keypair, &fund_name, token_name)?;
    let user_requests = client.get_fund_user_requests(&wallet, &fund_name, token_name)?;
    println!("{:#?}", user_requests);
    assert_eq!(
        user_requests.discriminator,
        DISCRIMINATOR_FUND_USER_REQUESTS
    );

    // init SOL custody
    // deposit should fail while custody is missing
    info!("Init Deposit/Withdraw custody for SOL");
    assert!(client
        .get_fund_custody(&fund_name, token_name, FundCustodyType::DepositWithdraw)
        .is_err());
    assert!(client
        .add_fund_custody(
            &manager_keypair,
            &fund_name,
            token_name,
            FundCustodyType::DepositWithdraw,
        )
        .is_err());

    client.add_fund_custody(
        &admin_keypair,
        &fund_name,
        token_name,
        FundCustodyType::DepositWithdraw,
    )?;
    let custody =
        client.get_fund_custody(&fund_name, token_name, FundCustodyType::DepositWithdraw)?;
    println!("{:#?}", custody);
    assert_eq!(custody.discriminator, DISCRIMINATOR_FUND_CUSTODY);
    assert_eq!(custody.custody_type, FundCustodyType::DepositWithdraw);

    info!("Remove and re-init custody");
    client.remove_fund_custody(
        &admin_keypair,
        &fund_name,
        token_name,
        FundCustodyType::DepositWithdraw,
    )?;
    assert!(client
        .get_fund_custody(&fund_name, token_name, FundCustodyType::DepositWithdraw)
        .is_err());

    client.add_fund_custody(
        &admin_keypair,
        &fund_name,
        token_name,
        FundCustodyType::DepositWithdraw,
    )?;
    let custody =
        client.get_fund_custody(&fund_name, token_name, FundCustodyType::DepositWithdraw)?;
    assert_eq!(custody.custody_type, FundCustodyType::DepositWithdraw);

    // add a Vault
    info!("Add a Vault");
    assert!(client
        .get_fund_vault(&fund_name, vault_name, vault_type)
        .is_err());

    client.add_fund_vault(&admin_keypair, &fund_name, vault_name, vault_type)?;
    let vault = client.get_fund_vault(&fund_name, vault_name, vault_type)?;
    println!("{:#?}", vault);
    assert_eq!(vault.discriminator, DISCRIMINATOR_FUND_VAULT);
    assert_eq!(vault.vault_type, vault_type);

    info!("Remove and re-add the Vault");
    client.remove_fund_vault(&admin_keypair, &fund_name, vault_name, vault_type)?;
    assert!(client
        .get_fund_vault(&fund_name, vault_name, vault_type)
        .is_err());

    client.add_fund_vault(&admin_keypair, &fund_name, vault_name, vault_type)?;
    let vault = client.get_fund_vault(&fund_name, vault_name, vault_type)?;
    assert_eq!(vault.vault_type, vault_type);

    // set assets tracking config
    info!("Set assets tracking config");
    let config = FundAssetsTrackingConfig {
        assets_limit_usd: 1000.0,
        max_update_age_sec: 600,
        max_price_error: 0.1,
        max_price_age_sec: 600,
        issue_virtual_tokens: false,
    };
    client.set_fund_assets_tracking_config(&admin_keypair, &fund_name, &config)?;
    let fund_info = client.get_fund_info(&fund_name)?;
    assert_eq!(fund_info.assets_config, config);

    // set deposit schedule
    info!("Set deposit schedule");
    assert!(client
        .request_deposit_fund(&user_keypair, &fund_name, token_name, 1.123)
        .is_err());
    let schedule = FundSchedule {
        start_time: 0,
        end_time: utils::get_time() + 600,
        approval_required: true,
        min_amount_usd: 0.0,
        max_amount_usd: client.get_oracle_price("SOL", 0, 0.0)? * 1.5,
        fee: 0.01,
    };
    client.set_fund_deposit_schedule(&admin_keypair, &fund_name, &schedule)?;
    let fund_info = client.get_fund_info(&fund_name)?;
    assert_eq!(fund_info.deposit_schedule, schedule);

    // request deposit
    info!("Request deposit over the limit");
    assert!(client
        .request_deposit_fund(&user_keypair, &fund_name, token_name, 1.8)
        .is_err());
    info!("Request deposit");
    client.request_deposit_fund(&user_keypair, &fund_name, token_name, 1.123)?;
    let user_requests = client.get_fund_user_requests(&wallet, &fund_name, token_name)?;
    assert_eq!(
        user_requests.deposit_request.amount,
        client.ui_amount_to_tokens(1.123, "SOL")?
    );
    assert!(user_requests.deposit_request.time > 0);
    assert!(user_requests.deny_reason.is_empty());
    assert_eq!(
        client.get_token_account_balance(&wallet, fund_token.name.as_str())?,
        0.0
    );

    // cancel deposit
    info!("Cancel deposit");
    client.cancel_deposit_fund(&user_keypair, &fund_name, token_name)?;
    let user_requests = client.get_fund_user_requests(&wallet, &fund_name, token_name)?;
    assert_eq!(user_requests.deposit_request.amount, 0);
    assert_eq!(user_requests.deposit_request.time, 0);
    assert!(user_requests.deny_reason.is_empty());

    // request and deny
    info!("Request a new deposit and deny");
    client.request_deposit_fund(&user_keypair, &fund_name, token_name, 1.123)?;
    let user_balance_before = client.get_token_account_balance(&wallet, "SOL")?;
    client.deny_deposit_fund(&manager_keypair, &fund_name, &wallet, token_name, "test")?;
    let user_requests = client.get_fund_user_requests(&wallet, &fund_name, token_name)?;
    assert_eq!(user_requests.deposit_request.amount, 0);
    assert_eq!(user_requests.deposit_request.time, 0);
    assert_eq!(user_requests.deny_reason, str_to_as64("test")?);
    assert_eq!(
        user_requests.last_deposit.amount,
        client.ui_amount_to_tokens(1.123, "SOL")?
    );
    assert!(user_requests.last_deposit.time > 0);
    assert_eq!(
        user_balance_before,
        client.get_token_account_balance(&wallet, "SOL")?
    );

    // request and approve
    info!("Request a new deposit and approve");
    let fund_token_balance_before =
        client.get_token_account_balance(&wallet, fund_token.name.as_str())?;
    let fund_token_supply_before = client.get_token_supply(fund_token.name.as_str())?;
    client.request_deposit_fund(&user_keypair, &fund_name, token_name, 1.123)?;
    client.approve_deposit_fund(&admin_keypair, &fund_name, &wallet, token_name, 0.123)?;
    let user_requests = client.get_fund_user_requests(&wallet, &fund_name, token_name)?;
    assert_eq!(user_requests.deposit_request.amount, 0);
    assert_eq!(user_requests.deposit_request.time, 0);
    assert!(user_requests.deny_reason.is_empty());
    assert_eq!(
        user_requests.last_deposit.amount,
        client.ui_amount_to_tokens(0.123, "SOL")?
    );
    assert!(user_requests.last_deposit.time > 0);
    let fund_token_balance = client.get_token_account_balance(&wallet, fund_token.name.as_str())?;
    assert_eq!(
        client.get_token_supply(fund_token.name.as_str())? - fund_token_supply_before,
        fund_token_balance
    );
    assert!(fund_token_balance > fund_token_balance_before);
    let wd_custody_token_address = client.get_fund_custody_token_account(
        &fund_name,
        token_name,
        FundCustodyType::DepositWithdraw,
    )?;
    let wd_fees_custody_token_address = client.get_fund_custody_fees_token_account(
        &fund_name,
        token_name,
        FundCustodyType::DepositWithdraw,
    )?;
    let deposited_amount = client.ui_amount_to_tokens(0.123 - 0.123 * 0.01, "SOL")?;
    assert_eq!(
        deposited_amount,
        utils::get_token_balance(&client, &wd_custody_token_address)
    );
    assert_eq!(
        client.ui_amount_to_tokens(0.123, "SOL")? - deposited_amount,
        utils::get_token_balance(&client, &wd_fees_custody_token_address)
    );

    // update assets with vault
    info!("Update assets with vaults");
    let fund_assets = client.get_fund_assets(&fund_name, FundAssetType::Vault)?;
    assert!(fund_assets.target_hash > 0);
    let original_cycle = fund_assets.current_cycle;
    client.update_fund_assets_with_vaults(&user_keypair, &fund_name)?;
    let fund_assets = client.get_fund_assets(&fund_name, FundAssetType::Vault)?;
    assert_eq!(fund_assets.current_cycle, original_cycle + 1);
    assert!(fund_assets.cycle_end_time > 0);
    assert_eq!(fund_assets.current_assets_usd, 0.0);

    // update assets with custody
    info!("Update assets with custodies");
    let fund_assets = client.get_fund_assets(&fund_name, FundAssetType::Custody)?;
    assert!(fund_assets.target_hash > 0);
    let original_cycle = fund_assets.current_cycle;
    client.update_fund_assets_with_custodies(&user_keypair, &fund_name)?;
    let fund_assets = client.get_fund_assets(&fund_name, FundAssetType::Custody)?;
    assert_eq!(fund_assets.current_cycle, original_cycle + 1);
    assert!(fund_assets.cycle_end_time > 0);
    let expected_assets_usd = client.get_oracle_price("SOL", 0, 0.0)? * 0.123;
    assert!((fund_assets.current_assets_usd - expected_assets_usd).abs() < 1.0);

    let fund_info = client.get_fund_info(&fund_name)?;
    assert!((fund_info.current_assets_usd - fund_assets.current_assets_usd).abs() < 1.0);

    // init second user
    let user_keypair2 = Keypair::new();
    let wallet2 = user_keypair2.pubkey();
    client.confirm_async_transaction(
        &client
            .rpc_client
            .request_airdrop(&wallet2, client.ui_amount_to_tokens(2.0, "SOL")?)?,
        CommitmentLevel::Confirmed,
    )?;

    // enable fund multisig
    info!("Enable Fund multisig");
    let multisig = client.get_fund_admins(&fund_name)?;
    assert_eq!(multisig.num_signers, 1);
    assert_eq!(multisig.signers[0], admin_keypair.pubkey());
    assert_eq!(multisig.signers[1], zero::id());

    client.set_fund_admins(&admin_keypair, &fund_name, &[wallet, wallet2], 2)?;

    let multisig = client.get_fund_admins(&fund_name)?;
    assert_eq!(multisig.num_signers, 2);
    assert_eq!(multisig.num_signed, 0);
    assert!(!multisig.signed[0]);
    assert!(!multisig.signed[1]);
    assert_eq!(multisig.min_signatures, 2);
    assert_eq!(multisig.signers[0], wallet);
    assert_eq!(multisig.signers[1], wallet2);
    assert_eq!(multisig.signers[2], zero::id());

    // operations under admin should fail
    assert!(client
        .set_fund_deposit_schedule(&admin_keypair, &fund_name, &schedule)
        .is_err());
    assert!(client
        .add_fund_custody(
            &admin_keypair,
            &fund_name,
            token_name,
            FundCustodyType::Trading,
        )
        .is_err());

    // multisign should go thru
    info!("Test Fund multisig");
    client.add_fund_custody(
        &user_keypair,
        &fund_name,
        token_name,
        FundCustodyType::Trading,
    )?;
    let multisig = client.get_fund_admins(&fund_name)?;
    assert_eq!(multisig.num_signed, 1);
    assert!(multisig.signed[0]);
    assert!(!multisig.signed[1]);
    assert!(client
        .get_fund_custody(&fund_name, token_name, FundCustodyType::Trading)
        .is_err());
    client.add_fund_custody(
        &user_keypair2,
        &fund_name,
        token_name,
        FundCustodyType::Trading,
    )?;
    assert!(client
        .get_fund_custody(&fund_name, token_name, FundCustodyType::Trading)
        .is_ok());
    let multisig = client.get_fund_admins(&fund_name)?;
    assert_eq!(multisig.num_signed, 2);
    assert!(multisig.signed[0]);
    assert!(multisig.signed[1]);

    // disable multisig
    info!("Disable Fund multisig");
    client.set_fund_admins(&user_keypair, &fund_name, &[admin_keypair.pubkey()], 1)?;
    client.set_fund_admins(&user_keypair2, &fund_name, &[admin_keypair.pubkey()], 1)?;
    let multisig = client.get_fund_admins(&fund_name)?;
    assert_eq!(multisig.num_signers, 1);
    assert_eq!(multisig.signers[0], admin_keypair.pubkey());
    assert_eq!(multisig.signers[1], zero::id());

    client.remove_fund_custody(
        &admin_keypair,
        &fund_name,
        token_name,
        FundCustodyType::Trading,
    )?;
    client.remove_fund_multisig(&admin_keypair, &fund_name)?;

    // turn off approval requirement
    let schedule = FundSchedule {
        start_time: 0,
        end_time: utils::get_time() + 600,
        approval_required: false,
        min_amount_usd: 0.0,
        max_amount_usd: client.get_oracle_price("SOL", 0, 0.0)? * 1.5,
        fee: 0.01,
    };
    client.set_fund_deposit_schedule(&manager_keypair, &fund_name, &schedule)?;

    // request instant deposit
    info!("Request instant deposit");
    client.request_deposit_fund(&user_keypair2, &fund_name, token_name, 0.123)?;
    let user_requests = client.get_fund_user_requests(&wallet2, &fund_name, token_name)?;
    assert_eq!(user_requests.deposit_request.amount, 0);
    assert_eq!(user_requests.deposit_request.time, 0);
    assert!(user_requests.deny_reason.is_empty());
    assert!(user_requests.last_deposit.amount > 0);
    assert!(user_requests.last_deposit.time > 0);
    let fund_token_balance2 =
        client.get_token_account_balance(&wallet2, fund_token.name.as_str())?;
    assert!(fund_token_balance2 > 0.0);
    // some tolerence needed due to potential SOL/USD price change
    assert!((fund_token_balance2 - fund_token_balance).abs() / fund_token_balance < 0.01);
    assert_eq!(
        deposited_amount * 2,
        utils::get_token_balance(&client, &wd_custody_token_address)
    );
    assert_eq!(
        (client.ui_amount_to_tokens(0.123, "SOL")? - deposited_amount) * 2,
        utils::get_token_balance(&client, &wd_fees_custody_token_address)
    );

    // set withdrawal schedule
    info!("Set withdrawal schedule");
    assert!(client
        .request_withdrawal_fund(&user_keypair, &fund_name, token_name, 0.1)
        .is_err());
    let schedule = FundSchedule {
        start_time: 0,
        end_time: utils::get_time() + 600,
        approval_required: true,
        min_amount_usd: 0.0,
        max_amount_usd: client.get_oracle_price("SOL", 0, 0.0)? * 0.1,
        fee: 0.01,
    };
    client.set_fund_withdrawal_schedule(&admin_keypair, &fund_name, &schedule)?;
    let fund_info = client.get_fund_info(&fund_name)?;
    assert_eq!(fund_info.withdrawal_schedule, schedule);

    // request withdrawal
    info!("Request withdrawal over the limit");
    let fund_token_balance_after_deposit =
        client.get_token_account_balance(&wallet, fund_token.name.as_str())?;
    info!("Fund token balance: {}", fund_token_balance_after_deposit);
    assert!(client
        .request_withdrawal_fund(
            &user_keypair,
            &fund_name,
            token_name,
            fund_token_balance_after_deposit
        )
        .is_err());
    let schedule = FundSchedule {
        start_time: 0,
        end_time: utils::get_time() + 600,
        approval_required: true,
        min_amount_usd: 0.0,
        max_amount_usd: client.get_oracle_price("SOL", 0, 0.0)? * 0.2,
        fee: 0.01,
    };
    client.set_fund_withdrawal_schedule(&manager_keypair, &fund_name, &schedule)?;
    info!("Request withdrawal");
    client.request_withdrawal_fund(&user_keypair, &fund_name, token_name, 100.0)?;
    let user_requests = client.get_fund_user_requests(&wallet, &fund_name, token_name)?;
    assert_eq!(
        user_requests.withdrawal_request.amount,
        client.ui_amount_to_tokens_with_decimals(100.0, 6)?
    );
    assert!(user_requests.withdrawal_request.time > 0);
    assert!(user_requests.deny_reason.is_empty());
    assert_eq!(
        client.get_token_account_balance(&wallet, fund_token.name.as_str())?,
        fund_token_balance_after_deposit
    );

    // cancel withdrawal
    info!("Cancel withdrawal");
    client.cancel_withdrawal_fund(&user_keypair, &fund_name, token_name)?;
    let user_requests = client.get_fund_user_requests(&wallet, &fund_name, token_name)?;
    assert_eq!(user_requests.withdrawal_request.amount, 0);
    assert_eq!(user_requests.withdrawal_request.time, 0);
    assert!(user_requests.deny_reason.is_empty());

    // request and deny
    info!("Request a new withdrawal and deny");
    client.request_withdrawal_fund(&user_keypair, &fund_name, token_name, 111.0)?;
    client.deny_withdrawal_fund(
        &admin_keypair,
        &fund_name,
        &wallet,
        token_name,
        "not allowed",
    )?;
    let user_requests = client.get_fund_user_requests(&wallet, &fund_name, token_name)?;
    assert_eq!(user_requests.withdrawal_request.amount, 0);
    assert_eq!(user_requests.withdrawal_request.time, 0);
    assert_eq!(user_requests.deny_reason, str_to_as64("not allowed")?);
    assert_eq!(
        user_requests.last_withdrawal.amount,
        client.ui_amount_to_tokens_with_decimals(111.0, 6)?
    );
    assert!(user_requests.last_withdrawal.time > 0);

    // request and approve
    info!("Request a new withdrawal and approve");
    let initial_sol_balance = client.get_token_account_balance(&wallet, "SOL")?;
    let initial_custody_balance = utils::get_token_balance(&client, &wd_custody_token_address);
    client.request_withdrawal_fund(&user_keypair, &fund_name, token_name, 121.77)?;
    let user_requests = client.get_fund_user_requests(&wallet, &fund_name, token_name)?;
    assert_eq!(
        user_requests.withdrawal_request.amount,
        client.ui_amount_to_tokens_with_decimals(121.77, 6)?
    );
    assert!(user_requests.withdrawal_request.time > 0);
    assert!(user_requests.deny_reason.is_empty());
    client.approve_withdrawal_fund(&manager_keypair, &fund_name, &wallet, token_name, 100.0)?;
    let user_requests = client.get_fund_user_requests(&wallet, &fund_name, token_name)?;
    assert_eq!(user_requests.withdrawal_request.amount, 0);
    assert_eq!(user_requests.withdrawal_request.time, 0);
    assert!(user_requests.deny_reason.is_empty());
    assert_eq!(
        user_requests.last_withdrawal.amount,
        client.ui_amount_to_tokens_with_decimals(100.0, 6)?
    );
    assert!(user_requests.last_withdrawal.time > 0);
    let fund_token_balance3 =
        client.get_token_account_balance(&wallet, fund_token.name.as_str())?;
    assert!(fund_token_balance3 > 0.0 && fund_token_balance3 < fund_token_balance_after_deposit);
    assert!(client.get_token_account_balance(&wallet, "SOL")? - initial_sol_balance > 0.09);
    let new_custody_balance = utils::get_token_balance(&client, &wd_custody_token_address);
    assert!(
        (initial_custody_balance as f64
            - new_custody_balance as f64
            - client.ui_amount_to_tokens(0.1, "SOL")? as f64)
            .abs()
            < 1000000.0
    );
    assert!(
        (((client.ui_amount_to_tokens(0.123, "SOL")? - deposited_amount) * 2
            + client.ui_amount_to_tokens(0.1 * 0.01, "SOL")?) as f64
            - utils::get_token_balance(&client, &wd_fees_custody_token_address) as f64)
            .abs()
            < 10000.0
    );

    // turn off approval requirement
    let schedule = FundSchedule {
        start_time: 0,
        end_time: utils::get_time() + 600,
        approval_required: false,
        min_amount_usd: 0.0,
        max_amount_usd: client.get_oracle_price("SOL", 0, 0.0)? * 1.5,
        fee: 0.01,
    };
    client.set_fund_withdrawal_schedule(&admin_keypair, &fund_name, &schedule)?;

    // request instant withdrawal
    info!("Request instant withdrawal");
    let initial_sol_balance = client.get_token_account_balance(&wallet2, "SOL")?;
    let initial_custody_balance = utils::get_token_balance(&client, &wd_custody_token_address);
    client.request_withdrawal_fund(&user_keypair2, &fund_name, token_name, 100.0)?;
    let user_requests = client.get_fund_user_requests(&wallet2, &fund_name, token_name)?;
    assert_eq!(user_requests.withdrawal_request.amount, 0);
    assert_eq!(user_requests.withdrawal_request.time, 0);
    assert!(user_requests.deny_reason.is_empty());
    assert!(user_requests.last_withdrawal.amount > 0);
    assert!(user_requests.last_withdrawal.time > 0);
    let fund_token_balance4 =
        client.get_token_account_balance(&wallet2, fund_token.name.as_str())?;
    assert!(fund_token_balance4 > 0.0 && fund_token_balance4 < fund_token_balance2);
    // some tolerence needed due to potential SOL/USD price change
    assert!((fund_token_balance4 - fund_token_balance3).abs() / fund_token_balance3 < 0.05);
    assert!(client.get_token_account_balance(&wallet2, "SOL")? - initial_sol_balance > 0.09);
    let new_custody_balance = utils::get_token_balance(&client, &wd_custody_token_address);
    assert!(
        (initial_custody_balance as f64
            - new_custody_balance as f64
            - client.ui_amount_to_tokens(0.1, "SOL")? as f64)
            .abs()
            < 1000000.0
    );
    assert!(
        (((client.ui_amount_to_tokens(0.123, "SOL")? - deposited_amount) * 2
            + client.ui_amount_to_tokens(0.1 * 0.01, "SOL")? * 2) as f64
            - utils::get_token_balance(&client, &wd_fees_custody_token_address) as f64)
            .abs()
            < 10000.0
    );

    // init SOL trading custody
    // accept should fail while custody is missing
    info!("Init Trading custody for SOL");
    if client
        .get_fund_custody(&fund_name, token_name, FundCustodyType::Trading)
        .is_err()
    {
        client.add_fund_custody(
            &admin_keypair,
            &fund_name,
            token_name,
            FundCustodyType::Trading,
        )?;
    }
    let custody = client.get_fund_custody(&fund_name, token_name, FundCustodyType::Trading)?;
    println!("{:#?}", custody);
    assert_eq!(custody.discriminator, DISCRIMINATOR_FUND_CUSTODY);
    assert_eq!(custody.custody_type, FundCustodyType::Trading);

    // accept funds into trading custody
    info!("Accept funds into trading custody");
    let trading_custody_token_address =
        client.get_fund_custody_token_account(&fund_name, token_name, FundCustodyType::Trading)?;
    let wd_custody_balance = utils::get_token_balance(&client, &wd_custody_token_address);
    let trading_custody_balance = utils::get_token_balance(&client, &trading_custody_token_address);
    assert_eq!(trading_custody_balance, 0);
    client.lock_assets_fund(&manager_keypair, &fund_name, token_name, 0.0)?;
    assert_eq!(
        0,
        utils::get_token_balance(&client, &wd_custody_token_address)
    );
    assert_eq!(
        wd_custody_balance,
        utils::get_token_balance(&client, &trading_custody_token_address)
    );

    // release funds into w/d custody
    info!("Release funds into w/d custody");
    client.unlock_assets_fund(&admin_keypair, &fund_name, token_name, 0.0)?;
    assert_eq!(
        0,
        utils::get_token_balance(&client, &trading_custody_token_address)
    );
    assert_eq!(
        wd_custody_balance,
        utils::get_token_balance(&client, &wd_custody_token_address)
    );

    // swap
    info!("Update fund assets");
    info!(
        "Custodies processed: {}",
        client.update_fund_assets_with_custodies(&user_keypair, &fund_name)?
    );
    info!(
        "Vaults processed: {}",
        client.update_fund_assets_with_vaults(&user_keypair, &fund_name)?
    );

    if client
        .get_fund_custody(&fund_name, token_a, FundCustodyType::Trading)
        .is_err()
    {
        info!("Init trading custody for {}", token_a);
        client.add_fund_custody(
            &admin_keypair,
            &fund_name,
            token_a,
            FundCustodyType::Trading,
        )?;
    }

    if client
        .get_fund_custody(&fund_name, token_a, FundCustodyType::DepositWithdraw)
        .is_err()
    {
        info!("Init deposit custody for {}", token_a);
        client.add_fund_custody(
            &admin_keypair,
            &fund_name,
            token_a,
            FundCustodyType::DepositWithdraw,
        )?;
    }

    if client
        .get_fund_custody(&fund_name, token_b, FundCustodyType::Trading)
        .is_err()
    {
        info!("Init trading custody for {}", token_b);
        client.add_fund_custody(
            &admin_keypair,
            &fund_name,
            token_b,
            FundCustodyType::Trading,
        )?;
    }

    let trading_custody_token_a_address =
        client.get_fund_custody_token_account(&fund_name, token_a, FundCustodyType::Trading)?;
    let trading_custody_token_b_address =
        client.get_fund_custody_token_account(&fund_name, token_b, FundCustodyType::Trading)?;
    let trading_custody_token_a_balance =
        utils::get_token_ui_balance(&client, &trading_custody_token_a_address);

    if trading_custody_token_a_balance < amount * 2.0 + amount * 2.0 * 0.04 {
        info!("Set new deposit schedule");
        let schedule = FundSchedule {
            start_time: 0,
            end_time: utils::get_time() + 600,
            approval_required: false,
            min_amount_usd: 0.0,
            max_amount_usd: f64::MAX,
            fee: 0.01,
        };
        client.set_fund_deposit_schedule(&admin_keypair, &fund_name, &schedule)?;
        info!("Deposit {} to the Fund", token_a);
        client.request_deposit_fund(
            &admin_keypair,
            &fund_name,
            token_a,
            amount * 2.0 + amount * 2.0 * 0.04,
        )?;
        info!("Move {} to trading custody", token_a);
        client.lock_assets_fund(&admin_keypair, &fund_name, token_a, 0.0)?;
    }

    let trading_custody_token_a_balance =
        utils::get_token_ui_balance(&client, &trading_custody_token_a_address);
    let trading_custody_token_b_balance =
        utils::get_token_ui_balance(&client, &trading_custody_token_b_address);
    info!(
        "Trading custody balance {}: {}, {}: {}",
        token_a, trading_custody_token_a_balance, token_b, trading_custody_token_b_balance
    );

    info!("Swap {} to {}", token_a, token_b);
    info!(
        "{}",
        client.fund_swap(
            &manager_keypair,
            &fund_name,
            Protocol::Raydium,
            token_a,
            token_b,
            amount,
            0.0
        )?
    );
    let trading_custody_token_a_balance2 =
        utils::get_token_ui_balance(&client, &trading_custody_token_a_address);
    let trading_custody_token_b_balance2 =
        utils::get_token_ui_balance(&client, &trading_custody_token_b_address);
    assert!(
        (trading_custody_token_a_balance - trading_custody_token_a_balance2 - amount).abs() < 0.001
    );
    assert!(trading_custody_token_b_balance2 > trading_custody_token_b_balance);

    // add liquidity
    if client
        .get_fund_custody(&fund_name, lp_token, FundCustodyType::Trading)
        .is_err()
    {
        info!("Init trading custody for {}", lp_token);
        client.add_fund_custody(
            &admin_keypair,
            &fund_name,
            lp_token,
            FundCustodyType::Trading,
        )?;
    }

    let trading_custody_token_a_balance =
        utils::get_token_ui_balance(&client, &trading_custody_token_a_address);
    let trading_custody_token_b_balance =
        utils::get_token_ui_balance(&client, &trading_custody_token_b_address);
    let trading_custody_lp_token_address =
        client.get_fund_custody_token_account(&fund_name, lp_token, FundCustodyType::Trading)?;
    let trading_custody_lp_token_balance =
        utils::get_token_ui_balance(&client, &trading_custody_lp_token_address);

    info!("Add liquidity to {}", vault_name);
    info!(
        "{}",
        client.fund_add_liquidity_pool(
            &manager_keypair,
            &fund_name,
            vault_name,
            amount * 0.4,
            0.0
        )?
    );
    let trading_custody_token_a_balance2 =
        utils::get_token_ui_balance(&client, &trading_custody_token_a_address);
    let trading_custody_token_b_balance2 =
        utils::get_token_ui_balance(&client, &trading_custody_token_b_address);
    let trading_custody_lp_token_balance2 =
        utils::get_token_ui_balance(&client, &trading_custody_lp_token_address);
    assert!(
        (trading_custody_token_a_balance - trading_custody_token_a_balance2 - amount * 0.4).abs()
            < 0.001
    );
    assert!(trading_custody_token_b_balance > trading_custody_token_b_balance2);
    assert!(trading_custody_lp_token_balance2 > trading_custody_lp_token_balance);

    info!("Add liquidity to {}", vault_name);
    info!(
        "{}",
        client.fund_add_liquidity_pool(
            &manager_keypair,
            &fund_name,
            vault_name,
            0.0,
            amount * 0.4,
        )?
    );
    let trading_custody_token_a_balance3 =
        utils::get_token_ui_balance(&client, &trading_custody_token_a_address);
    let trading_custody_token_b_balance3 =
        utils::get_token_ui_balance(&client, &trading_custody_token_b_address);
    let trading_custody_lp_token_balance3 =
        utils::get_token_ui_balance(&client, &trading_custody_lp_token_address);
    assert!(
        (trading_custody_token_b_balance2 - trading_custody_token_b_balance3 - amount * 0.4).abs()
            < 0.001
    );
    assert!(trading_custody_token_a_balance2 > trading_custody_token_a_balance3);
    assert!(trading_custody_lp_token_balance3 > trading_custody_lp_token_balance2);

    // stake
    let farm = client.find_farms_with_lp(lp_token)?[0];
    info!("Stake to {}", farm.name);

    if client
        .get_fund_vault(&fund_name, &farm.name, FundVaultType::Farm)
        .is_err()
    {
        info!("Add a Farm");
        assert!(client
            .add_fund_vault(
                &manager_keypair,
                &fund_name,
                &farm.name,
                FundVaultType::Farm
            )
            .is_err());
        client.add_fund_vault(&admin_keypair, &fund_name, &farm.name, FundVaultType::Farm)?;
        let vault = client.get_fund_vault(&fund_name, &farm.name, FundVaultType::Farm)?;
        println!("{:#?}", vault);
        assert_eq!(vault.discriminator, DISCRIMINATOR_FUND_VAULT);
        assert_eq!(vault.vault_type, FundVaultType::Farm);
    }

    let trading_custody_lp_token_balance =
        utils::get_token_ui_balance(&client, &trading_custody_lp_token_address);
    let stake_balance =
        if let Ok(stake) = client.get_user_stake_balance(&fund.fund_authority, &farm.name) {
            stake
        } else {
            0.0
        };
    info!(
        "{}",
        client.fund_stake(
            &manager_keypair,
            &fund_name,
            &farm.name,
            trading_custody_lp_token_balance * 0.5,
        )?
    );
    let trading_custody_lp_token_balance2 =
        utils::get_token_ui_balance(&client, &trading_custody_lp_token_address);
    let stake_balance2 = client.get_user_stake_balance(&fund.fund_authority, &farm.name)?;
    assert!(
        (trading_custody_lp_token_balance * 0.5 - trading_custody_lp_token_balance2).abs() < 0.001
    );
    assert!(
        (stake_balance2 - stake_balance - trading_custody_lp_token_balance * 0.5).abs() < 0.001
    );

    info!("Stake to {}", farm.name);

    let trading_custody_lp_token_balance =
        utils::get_token_ui_balance(&client, &trading_custody_lp_token_address);
    let stake_balance =
        if let Ok(stake) = client.get_user_stake_balance(&fund.fund_authority, &farm.name) {
            stake
        } else {
            0.0
        };
    info!(
        "{}",
        client.fund_stake(&manager_keypair, &fund_name, &farm.name, 0.0,)?
    );
    let trading_custody_lp_token_balance2 =
        utils::get_token_ui_balance(&client, &trading_custody_lp_token_address);
    let stake_balance2 = client.get_user_stake_balance(&fund.fund_authority, &farm.name)?;
    assert!(trading_custody_lp_token_balance2 == 0.0);
    assert!((stake_balance2 - stake_balance - trading_custody_lp_token_balance).abs() < 0.001);

    // harvest
    info!("Harvest from {}", farm.name);
    info!(
        "{}",
        client.fund_harvest(&manager_keypair, &fund_name, &farm.name)?
    );

    // unstake
    info!("Unstake from {}", farm.name);
    let trading_custody_lp_token_balance =
        utils::get_token_ui_balance(&client, &trading_custody_lp_token_address);
    let stake_balance = client.get_user_stake_balance(&fund.fund_authority, &farm.name)?;
    info!(
        "{}",
        client.fund_unstake(
            &manager_keypair,
            &fund_name,
            &farm.name,
            stake_balance * 0.5
        )?
    );
    let stake_balance2 = client.get_user_stake_balance(&fund.fund_authority, &farm.name)?;
    let trading_custody_lp_token_balance2 =
        utils::get_token_ui_balance(&client, &trading_custody_lp_token_address);
    assert!(
        (trading_custody_lp_token_balance2
            - trading_custody_lp_token_balance
            - stake_balance * 0.5)
            .abs()
            < 0.001
    );
    assert!((stake_balance - stake_balance2 - stake_balance * 0.5).abs() < 0.001);

    info!("Unstake from {}", farm.name);
    let trading_custody_lp_token_balance =
        utils::get_token_ui_balance(&client, &trading_custody_lp_token_address);
    let stake_balance = client.get_user_stake_balance(&fund.fund_authority, &farm.name)?;
    info!(
        "{}",
        client.fund_unstake(&manager_keypair, &fund_name, &farm.name, 0.0)?
    );
    let stake_balance2 = client.get_user_stake_balance(&fund.fund_authority, &farm.name)?;
    let trading_custody_lp_token_balance2 =
        utils::get_token_ui_balance(&client, &trading_custody_lp_token_address);
    assert!(
        (trading_custody_lp_token_balance2 - trading_custody_lp_token_balance - stake_balance)
            .abs()
            < 0.001
    );
    assert!(stake_balance2 == 0.0);

    // remove liquidity
    let trading_custody_token_a_balance =
        utils::get_token_ui_balance(&client, &trading_custody_token_a_address);
    let trading_custody_token_b_balance =
        utils::get_token_ui_balance(&client, &trading_custody_token_b_address);
    let trading_custody_lp_token_balance =
        utils::get_token_ui_balance(&client, &trading_custody_lp_token_address);

    info!("Remove liquidity from {}", vault_name);
    info!(
        "{}",
        client.fund_remove_liquidity_pool(
            &manager_keypair,
            &fund_name,
            vault_name,
            trading_custody_lp_token_balance * 0.5,
        )?
    );
    let trading_custody_token_a_balance2 =
        utils::get_token_ui_balance(&client, &trading_custody_token_a_address);
    let trading_custody_token_b_balance2 =
        utils::get_token_ui_balance(&client, &trading_custody_token_b_address);
    let trading_custody_lp_token_balance2 =
        utils::get_token_ui_balance(&client, &trading_custody_lp_token_address);
    assert!(trading_custody_token_a_balance2 > trading_custody_token_a_balance);
    assert!(trading_custody_token_b_balance2 > trading_custody_token_b_balance);
    assert!(
        (trading_custody_lp_token_balance * 0.5 - trading_custody_lp_token_balance2).abs() < 0.001
    );

    info!("Remove liquidity from {}", vault_name);
    info!(
        "{}",
        client.fund_remove_liquidity_pool(&manager_keypair, &fund_name, vault_name, 0.0,)?
    );
    let trading_custody_token_a_balance3 =
        utils::get_token_ui_balance(&client, &trading_custody_token_a_address);
    let trading_custody_token_b_balance3 =
        utils::get_token_ui_balance(&client, &trading_custody_token_b_address);
    let trading_custody_lp_token_balance3 =
        utils::get_token_ui_balance(&client, &trading_custody_lp_token_address);
    assert!(trading_custody_token_a_balance3 > trading_custody_token_a_balance2);
    assert!(trading_custody_token_b_balance3 > trading_custody_token_b_balance2);
    assert!(trading_custody_lp_token_balance3 == 0.0);

    // init vault
    fixture::init_vault(&client, &admin_keypair, vault_name2, vt_token)?;
    if client
        .get_fund_vault(&fund_name, vault_name2, FundVaultType::Vault)
        .is_err()
    {
        info!("Add a Vault");
        assert!(client
            .add_fund_vault(
                &manager_keypair,
                &fund_name,
                vault_name2,
                FundVaultType::Vault,
            )
            .is_err());
        client.add_fund_vault(
            &admin_keypair,
            &fund_name,
            vault_name2,
            FundVaultType::Vault,
        )?;
        let vault = client.get_fund_vault(&fund_name, vault_name2, FundVaultType::Vault)?;
        println!("{:#?}", vault);
        assert_eq!(vault.discriminator, DISCRIMINATOR_FUND_VAULT);
        assert_eq!(vault.vault_type, FundVaultType::Vault);
    }

    // enable vault multisig
    info!("Enable Vault multisig");
    let multisig = client.get_vault_admins(vault_name2)?;
    assert_eq!(multisig.num_signers, 1);
    assert_eq!(multisig.signers[0], admin_keypair.pubkey());
    assert_eq!(multisig.signers[1], zero::id());

    client.set_vault_admins(&admin_keypair, vault_name2, &[wallet, wallet2], 2)?;

    let multisig = client.get_vault_admins(vault_name2)?;
    assert_eq!(multisig.num_signers, 2);
    assert_eq!(multisig.num_signed, 0);
    assert!(!multisig.signed[0]);
    assert!(!multisig.signed[1]);
    assert_eq!(multisig.min_signatures, 2);
    assert_eq!(multisig.signers[0], wallet);
    assert_eq!(multisig.signers[1], wallet2);
    assert_eq!(multisig.signers[2], zero::id());

    // operations under admin should fail
    assert!(client
        .disable_withdrawals_vault(&admin_keypair, vault_name2)
        .is_err());

    // multisign should go thru
    info!("Test Vault multisig");
    client.disable_withdrawals_vault(&user_keypair, vault_name2)?;
    let multisig = client.get_vault_admins(vault_name2)?;
    assert_eq!(multisig.num_signed, 1);
    assert!(multisig.signed[0]);
    assert!(!multisig.signed[1]);
    assert!(client.get_vault_info(vault_name2)?.withdrawal_allowed);
    client.disable_withdrawals_vault(&user_keypair2, vault_name2)?;
    assert!(!client.get_vault_info(vault_name2)?.withdrawal_allowed);
    let multisig = client.get_vault_admins(vault_name2)?;
    assert_eq!(multisig.num_signed, 2);
    assert!(multisig.signed[0]);
    assert!(multisig.signed[1]);

    // disable multisig
    info!("Disable Vault multisig");
    client.set_vault_admins(&user_keypair, vault_name2, &[admin_keypair.pubkey()], 1)?;
    client.set_vault_admins(&user_keypair2, vault_name2, &[admin_keypair.pubkey()], 1)?;
    let multisig = client.get_vault_admins(vault_name2)?;
    assert_eq!(multisig.num_signers, 1);
    assert_eq!(multisig.signers[0], admin_keypair.pubkey());
    assert_eq!(multisig.signers[1], zero::id());
    client.enable_withdrawals_vault(&admin_keypair, vault_name2)?;
    client.remove_vault_multisig(&admin_keypair, vault_name2)?;
    client.enable_withdrawals_vault(&admin_keypair, vault_name2)?;

    // add liquidity vault
    if client
        .get_fund_custody(&fund_name, vt_token, FundCustodyType::Trading)
        .is_err()
    {
        info!("Init trading custody for {}", vt_token);
        client.add_fund_custody(
            &admin_keypair,
            &fund_name,
            vt_token,
            FundCustodyType::Trading,
        )?;
    }

    let trading_custody_token_a_balance =
        utils::get_token_ui_balance(&client, &trading_custody_token_a_address);
    let trading_custody_token_b_balance =
        utils::get_token_ui_balance(&client, &trading_custody_token_b_address);
    let trading_custody_vt_token_address =
        client.get_fund_custody_token_account(&fund_name, vt_token, FundCustodyType::Trading)?;
    let trading_custody_vt_token_balance =
        utils::get_token_ui_balance(&client, &trading_custody_vt_token_address);

    info!("Add liquidity to {}", vault_name2);
    info!(
        "{}",
        client.fund_add_liquidity_vault(
            &manager_keypair,
            &fund_name,
            vault_name2,
            amount * 0.4,
            0.0
        )?
    );
    let trading_custody_token_a_balance2 =
        utils::get_token_ui_balance(&client, &trading_custody_token_a_address);
    let trading_custody_token_b_balance2 =
        utils::get_token_ui_balance(&client, &trading_custody_token_b_address);
    let trading_custody_vt_token_balance2 =
        utils::get_token_ui_balance(&client, &trading_custody_vt_token_address);
    assert!(
        (trading_custody_token_a_balance - trading_custody_token_a_balance2 - amount * 0.4).abs()
            < 0.001
    );
    assert!(trading_custody_token_b_balance > trading_custody_token_b_balance2);
    assert!(trading_custody_vt_token_balance2 > trading_custody_vt_token_balance);

    info!("Add liquidity to {}", vault_name2);
    info!(
        "{}",
        client.fund_add_liquidity_vault(
            &manager_keypair,
            &fund_name,
            vault_name2,
            0.0,
            amount * 0.4
        )?
    );
    let trading_custody_token_a_balance3 =
        utils::get_token_ui_balance(&client, &trading_custody_token_a_address);
    let trading_custody_token_b_balance3 =
        utils::get_token_ui_balance(&client, &trading_custody_token_b_address);
    let trading_custody_vt_token_balance3 =
        utils::get_token_ui_balance(&client, &trading_custody_vt_token_address);
    assert!(
        (trading_custody_token_b_balance2 - trading_custody_token_b_balance3 - amount * 0.4).abs()
            < 0.001
    );
    assert!(trading_custody_token_a_balance2 > trading_custody_token_a_balance3);
    assert!(trading_custody_vt_token_balance3 > trading_custody_vt_token_balance2);

    // remove liquidity vault
    let trading_custody_token_a_balance =
        utils::get_token_ui_balance(&client, &trading_custody_token_a_address);
    let trading_custody_token_b_balance =
        utils::get_token_ui_balance(&client, &trading_custody_token_b_address);
    let trading_custody_vt_token_balance =
        utils::get_token_ui_balance(&client, &trading_custody_vt_token_address);

    info!("Remove liquidity from {}", vault_name2);
    info!(
        "{}",
        client.fund_remove_liquidity_vault(
            &manager_keypair,
            &fund_name,
            vault_name2,
            trading_custody_vt_token_balance * 0.5,
        )?
    );
    let trading_custody_token_a_balance2 =
        utils::get_token_ui_balance(&client, &trading_custody_token_a_address);
    let trading_custody_token_b_balance2 =
        utils::get_token_ui_balance(&client, &trading_custody_token_b_address);
    let trading_custody_vt_token_balance2 =
        utils::get_token_ui_balance(&client, &trading_custody_vt_token_address);
    assert!(trading_custody_token_a_balance2 > trading_custody_token_a_balance);
    assert!(trading_custody_token_b_balance2 > trading_custody_token_b_balance);
    assert!(
        (trading_custody_vt_token_balance * 0.5 - trading_custody_vt_token_balance2).abs() < 0.001
    );

    info!("Remove liquidity from {}", vault_name2);
    info!(
        "{}",
        client.fund_remove_liquidity_vault(&manager_keypair, &fund_name, vault_name2, 0.0)?
    );
    let trading_custody_token_a_balance3 =
        utils::get_token_ui_balance(&client, &trading_custody_token_a_address);
    let trading_custody_token_b_balance3 =
        utils::get_token_ui_balance(&client, &trading_custody_token_b_address);
    let trading_custody_vt_token_balance3 =
        utils::get_token_ui_balance(&client, &trading_custody_vt_token_address);
    assert!(trading_custody_token_a_balance3 > trading_custody_token_a_balance2);
    assert!(trading_custody_token_b_balance3 > trading_custody_token_b_balance2);
    assert!(trading_custody_vt_token_balance3 == 0.0);

    // withdraw fees
    info!("Withdraw collected fees");
    assert!(test_custody_withdrawal(
        &client,
        &manager_keypair,
        &fund_name,
        token_name,
        FundCustodyType::DepositWithdraw,
    )
    .is_err());
    test_custody_withdrawal(
        &client,
        &admin_keypair,
        &fund_name,
        token_name,
        FundCustodyType::DepositWithdraw,
    )?;
    test_custody_withdrawal(
        &client,
        &admin_keypair,
        &fund_name,
        token_name,
        FundCustodyType::Trading,
    )?;
    test_custody_withdrawal(
        &client,
        &admin_keypair,
        &fund_name,
        token_a,
        FundCustodyType::DepositWithdraw,
    )?;
    test_custody_withdrawal(
        &client,
        &admin_keypair,
        &fund_name,
        token_a,
        FundCustodyType::Trading,
    )?;

    // test liquidation
    info!("Update fund assets");
    info!(
        "Custodies processed: {}",
        client.update_fund_assets_with_custodies(&manager_keypair, &fund_name)?
    );
    let fund_assets = client.get_fund_assets(&fund_name, FundAssetType::Custody)?;
    assert!(fund_assets.cycle_end_time > 0);
    info!(
        "Vaults processed: {}",
        client.update_fund_assets_with_vaults(&manager_keypair, &fund_name)?
    );
    let fund_assets = client.get_fund_assets(&fund_name, FundAssetType::Vault)?;
    assert!(fund_assets.cycle_end_time > 0);

    // request instant deposit
    info!("Deposit funds to get some stake");
    let schedule = FundSchedule {
        start_time: 0,
        end_time: utils::get_time() + 600,
        approval_required: false,
        min_amount_usd: 0.0,
        max_amount_usd: client.get_oracle_price("SOL", 0, 0.0)? * 3.0,
        fee: 0.01,
    };
    client.set_fund_deposit_schedule(&manager_keypair, &fund_name, &schedule)?;
    client.request_deposit_fund(&user_keypair, &fund_name, token_name, 2.222)?;
    client.fund_add_liquidity_vault(
        &manager_keypair,
        &fund_name,
        vault_name2,
        amount * 0.2,
        0.0,
    )?;

    // lock funds and make withdrawals not possible
    info!("Lock funds and disable withdrawals");
    let schedule = FundSchedule {
        start_time: 0,
        end_time: utils::get_time() + 600,
        approval_required: true,
        min_amount_usd: 0.0,
        max_amount_usd: 0.0001,
        fee: 1.0,
    };
    client.lock_assets_fund(&manager_keypair, &fund_name, token_name, 0.0)?;
    client.set_fund_withdrawal_schedule(&manager_keypair, &fund_name, &schedule)?;

    // unlock should fail
    assert!(client
        .unlock_assets_fund(&user_keypair, &fund_name, token_name, 0.0)
        .is_err());

    // initiate liquidation
    info!("Start liquidation");
    assert!(fund_info.liquidation_start_time == 0);
    client.start_liquidation_fund(&user_keypair, &fund_name)?;
    let fund_info = client.get_fund_info(&fund_name)?;
    assert!(fund_info.liquidation_start_time > 0);

    // new deposits should fail
    assert!(client
        .request_deposit_fund(&user_keypair, &fund_name, token_name, 0.1)
        .is_err());
    assert!(client
        .fund_add_liquidity_vault(&manager_keypair, &fund_name, vault_name2, amount * 0.2, 0.0)
        .is_err());

    // remove liquidity from the vault
    info!("Remove liquidity from {}", vault_name2);
    client.fund_remove_liquidity_vault(&user_keypair, &fund_name, vault_name2, 0.0)?;

    // unlock assets
    info!("Unlock assets");
    client.unlock_assets_fund(&user_keypair, &fund_name, token_name, 0.0)?;
    client.unlock_assets_fund(&user_keypair, &fund_name, token_a, 0.0)?;

    // withdraw funds
    info!("Withdraw {} funds", token_a);
    let wd_custody_token_address = client.get_fund_custody_token_account(
        &fund_name,
        token_a,
        FundCustodyType::DepositWithdraw,
    )?;
    client.get_or_create_token_account(&user_keypair, token_a)?;
    let initial_balance = client.get_token_account_balance(&wallet, token_a)?;
    let initial_custody_balance = utils::get_token_ui_balance(&client, &wd_custody_token_address);
    let fund_token_balance = client.get_token_account_balance(&wallet, fund_token.name.as_str())?;
    let fund_token_supply = client.get_token_supply(fund_token.name.as_str())?;
    let token_value_usd = initial_custody_balance * client.get_oracle_price(token_a, 0, 0.0)?;
    let mut tokens_to_withdraw =
        token_value_usd / fund_info.current_assets_usd * fund_token_supply * 0.99;
    if tokens_to_withdraw > fund_token_balance {
        tokens_to_withdraw = fund_token_balance;
    }
    println!("initial_balance {}", initial_balance);
    println!("initial_custody_balance {}", initial_custody_balance);
    println!("fund_token_balance {}", fund_token_balance);
    println!("fund_token_supply {}", fund_token_supply);
    println!("token_value_usd {}", token_value_usd);
    println!("tokens_to_withdraw {}", tokens_to_withdraw);
    println!(
        "fund_info.current_assets_usd {}",
        fund_info.current_assets_usd
    );
    client.request_withdrawal_fund(&user_keypair, &fund_name, token_a, tokens_to_withdraw)?;
    let user_requests = client.get_fund_user_requests(&wallet, &fund_name, token_a)?;
    assert_eq!(user_requests.withdrawal_request.amount, 0);
    assert_eq!(user_requests.withdrawal_request.time, 0);
    assert!(user_requests.deny_reason.is_empty());
    println!(
        "new_balance {}",
        client.get_token_account_balance(&wallet, token_a)?
    );
    assert!(
        (client.get_token_account_balance(&wallet, token_a)?
            - initial_balance
            - initial_custody_balance * 0.99)
            .abs()
            < 0.001
    );
    assert!(
        (utils::get_token_ui_balance(&client, &wd_custody_token_address)
            - initial_custody_balance * 0.01)
            .abs()
            < 0.001
    );

    info!("Withdraw {} funds", token_name);
    let wd_custody_token_address = client.get_fund_custody_token_account(
        &fund_name,
        token_name,
        FundCustodyType::DepositWithdraw,
    )?;
    let initial_balance = client.get_token_account_balance(&wallet, token_name)?;
    let initial_custody_balance = utils::get_token_ui_balance(&client, &wd_custody_token_address);
    client.request_withdrawal_fund(&user_keypair, &fund_name, token_name, 0.0)?;
    let user_requests = client.get_fund_user_requests(&wallet, &fund_name, token_name)?;
    assert_eq!(user_requests.withdrawal_request.amount, 0);
    assert_eq!(user_requests.withdrawal_request.time, 0);
    assert!(user_requests.deny_reason.is_empty());
    assert!(client.get_token_account_balance(&wallet, token_name)? > initial_balance);
    assert!(
        utils::get_token_ui_balance(&client, &wd_custody_token_address) < initial_custody_balance
    );

    client.stop_liquidation_fund(&admin_keypair, &fund_name)?;

    // test virtual tokens
    info!("Disable W/D approval requirement and enable virtual tokens");
    let fund_token_balance =
        client.get_token_account_balance(&wallet2, fund_token.name.as_str())?;
    assert!(fund_token_balance > 0.0);
    let user_info = client.get_fund_user_info(&wallet2, &fund_name)?;
    assert_eq!(user_info.virtual_tokens_balance, 0);

    let config = FundAssetsTrackingConfig {
        assets_limit_usd: 1000.0,
        max_update_age_sec: 600,
        max_price_error: 0.1,
        max_price_age_sec: 600,
        issue_virtual_tokens: true,
    };
    client.set_fund_assets_tracking_config(&admin_keypair, &fund_name, &config)?;
    let fund_info = client.get_fund_info(&fund_name)?;
    assert!(fund_info.assets_config.issue_virtual_tokens);

    let schedule = FundSchedule {
        start_time: 0,
        end_time: utils::get_time() + 600,
        approval_required: false,
        min_amount_usd: 0.0,
        max_amount_usd: 1000.0,
        fee: 0.01,
    };
    client.set_fund_withdrawal_schedule(&manager_keypair, &fund_name, &schedule)?;
    client.set_fund_deposit_schedule(&manager_keypair, &fund_name, &schedule)?;

    // request new deposit
    info!("Request a new deposit");
    client.request_deposit_fund(&user_keypair2, &fund_name, token_name, 0.123)?;
    let user_requests = client.get_fund_user_requests(&wallet2, &fund_name, token_name)?;
    assert_eq!(user_requests.deposit_request.amount, 0);
    assert_eq!(user_requests.deposit_request.time, 0);
    assert!(user_requests.deny_reason.is_empty());
    let fund_token_balance2 =
        client.get_token_account_balance(&wallet2, fund_token.name.as_str())?;
    assert_eq!(fund_token_balance2, fund_token_balance);
    let user_info = client.get_fund_user_info(&wallet2, &fund_name)?;
    assert!(user_info.virtual_tokens_balance > 0);
    let fund_info = client.get_fund_info(&fund_name)?;
    assert_eq!(
        fund_info.virtual_tokens_supply,
        user_info.virtual_tokens_balance
    );

    // request partial withdrawal
    info!("Request partial withdrawal");
    client.request_withdrawal_fund(
        &user_keypair2,
        &fund_name,
        token_name,
        fund_token_balance / 2.0,
    )?;
    let user_requests = client.get_fund_user_requests(&wallet2, &fund_name, token_name)?;
    assert_eq!(user_requests.withdrawal_request.amount, 0);
    assert_eq!(user_requests.withdrawal_request.time, 0);
    assert!(user_requests.deny_reason.is_empty());
    let fund_token_balance3 =
        client.get_token_account_balance(&wallet2, fund_token.name.as_str())?;
    assert!(
        fund_token_balance3 > 0.0
            && fund_token_balance - fund_token_balance3 - fund_token_balance / 2.0 < 0.01
    );
    let user_info2 = client.get_fund_user_info(&wallet2, &fund_name)?;
    assert_eq!(
        user_info2.virtual_tokens_balance,
        user_info.virtual_tokens_balance
    );

    // request full withdrawal
    info!("Request full withdrawal");
    client.request_withdrawal_fund(&user_keypair2, &fund_name, token_name, 0.0)?;
    let user_requests = client.get_fund_user_requests(&wallet2, &fund_name, token_name)?;
    assert_eq!(user_requests.withdrawal_request.amount, 0);
    assert_eq!(user_requests.withdrawal_request.time, 0);
    assert!(user_requests.deny_reason.is_empty());
    let fund_token_balance =
        client.get_token_account_balance(&wallet2, fund_token.name.as_str())?;
    assert_eq!(fund_token_balance, 0.0);
    let user_info = client.get_fund_user_info(&wallet2, &fund_name)?;
    assert_eq!(user_info.virtual_tokens_balance, 0);
    let fund_info = client.get_fund_info(&fund_name)?;
    assert_eq!(fund_info.virtual_tokens_supply, 0);

    Ok(())
}

fn test_custody_withdrawal(
    client: &FarmClient,
    admin_keypair: &Keypair,
    fund_name: &str,
    token_name: &str,
    custody_type: FundCustodyType,
) -> Result<(), FarmClientError> {
    let receiver = client.get_fund_custody_token_account(fund_name, token_name, custody_type)?;
    let custody_fees_address =
        client.get_fund_custody_fees_token_account(fund_name, token_name, custody_type)?;
    let custody_fees_balance =
        client.get_token_account_balance_with_address(&custody_fees_address)?;
    let receiver_balance = client.get_token_account_balance_with_address(&receiver)?;
    info!(
        "{} {} fees balance: {}",
        token_name,
        if custody_type == FundCustodyType::Trading {
            "Trading"
        } else {
            "W/D"
        },
        custody_fees_balance
    );
    info!(
        "{}",
        client.withdraw_fees_fund(
            admin_keypair,
            fund_name,
            token_name,
            custody_type,
            0.0,
            &receiver
        )?
    );
    let custody_fees_balance2 =
        client.get_token_account_balance_with_address(&custody_fees_address)?;
    assert_eq!(custody_fees_balance2, 0.0);
    let receiver_balance2 = client.get_token_account_balance_with_address(&receiver)?;
    assert!(receiver_balance2 - receiver_balance - custody_fees_balance < 0.001);

    Ok(())
}
