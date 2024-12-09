#![allow(clippy::arithmetic_side_effects)]
use {
    libtest_mimic::{Arguments, Trial},
    solana_cli_output::OutputFormat,
    solana_client::{nonblocking::rpc_client::RpcClient, rpc_request::TokenAccountsFilter},
    solana_sdk::{
        bpf_loader_upgradeable,
        hash::Hash,
        program_option::COption,
        program_pack::Pack,
        pubkey::Pubkey,
        signature::{write_keypair_file, Keypair, Signer},
        system_instruction, system_program,
        transaction::Transaction,
    },
    solana_test_validator::{TestValidator, TestValidatorGenesis, UpgradeableProgramInfo},
    spl_associated_token_account_client::address::get_associated_token_address_with_program_id,
    spl_token_2022::{
        extension::{
            confidential_transfer::{ConfidentialTransferAccount, ConfidentialTransferMint},
            confidential_transfer_fee::ConfidentialTransferFeeConfig,
            cpi_guard::CpiGuard,
            default_account_state::DefaultAccountState,
            group_member_pointer::GroupMemberPointer,
            group_pointer::GroupPointer,
            interest_bearing_mint::InterestBearingConfig,
            memo_transfer::MemoTransfer,
            metadata_pointer::MetadataPointer,
            non_transferable::NonTransferable,
            transfer_fee::{TransferFeeAmount, TransferFeeConfig},
            transfer_hook::TransferHook,
            BaseStateWithExtensions, StateWithExtensionsOwned,
        },
        instruction::create_native_mint,
        solana_zk_sdk::encryption::pod::elgamal::PodElGamalPubkey,
        state::{Account, AccountState, Mint, Multisig},
    },
    spl_token_cli::{
        clap_app::*,
        command::{process_command, CommandResult},
        config::Config,
    },
    spl_token_client::{
        client::{
            ProgramClient, ProgramOfflineClient, ProgramRpcClient, ProgramRpcClientSendTransaction,
        },
        token::{ComputeUnitLimit, Token},
    },
    spl_token_group_interface::state::{TokenGroup, TokenGroupMember},
    spl_token_metadata_interface::state::TokenMetadata,
    std::{
        ffi::{OsStr, OsString},
        path::PathBuf,
        str::FromStr,
        sync::Arc,
    },
    tempfile::NamedTempFile,
};

macro_rules! async_trial {
    ($test_func:ident, $test_validator:ident, $payer:ident) => {{
        let local_test_validator = $test_validator.clone();
        let local_payer = $payer.clone();
        Trial::test(stringify!($test_func), move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    $test_func(local_test_validator.as_ref(), local_payer.as_ref()).await
                });
            Ok(())
        })
    }};
}

#[tokio::main]
async fn main() {
    let args = Arguments::from_args();
    let (test_validator, payer) = new_validator_for_test().await;
    let test_validator = Arc::new(test_validator);
    let payer = Arc::new(payer);

    // setup the native mint to be used by other tests
    do_create_native_mint(&test_validator.get_async_rpc_client(), payer.as_ref()).await;

    // the GC test requires its own whole environment
    let (gc_test_validator, gc_payer) = new_validator_for_test().await;
    let gc_test_validator = Arc::new(gc_test_validator);
    let gc_payer = Arc::new(gc_payer);

    // maybe come up with a way to do this through a some macro tag on the function?
    let tests = vec![
        async_trial!(create_token_default, test_validator, payer),
        async_trial!(create_token_2022, test_validator, payer),
        async_trial!(create_token_interest_bearing, test_validator, payer),
        async_trial!(set_interest_rate, test_validator, payer),
        async_trial!(supply, test_validator, payer),
        async_trial!(create_account_default, test_validator, payer),
        async_trial!(account_info, test_validator, payer),
        async_trial!(balance, test_validator, payer),
        async_trial!(mint, test_validator, payer),
        async_trial!(balance_after_mint, test_validator, payer),
        async_trial!(balance_after_mint_with_owner, test_validator, payer),
        async_trial!(accounts, test_validator, payer),
        async_trial!(accounts_with_owner, test_validator, payer),
        async_trial!(wrapped_sol, test_validator, payer),
        async_trial!(transfer, test_validator, payer),
        async_trial!(transfer_fund_recipient, test_validator, payer),
        async_trial!(transfer_non_standard_recipient, test_validator, payer),
        async_trial!(allow_non_system_account_recipient, test_validator, payer),
        async_trial!(close_account, test_validator, payer),
        async_trial!(disable_mint_authority, test_validator, payer),
        async_trial!(set_owner, test_validator, payer),
        async_trial!(transfer_with_account_delegate, test_validator, payer),
        async_trial!(burn, test_validator, payer),
        async_trial!(burn_with_account_delegate, test_validator, payer),
        async_trial!(burn_with_permanent_delegate, test_validator, payer),
        async_trial!(transfer_with_permanent_delegate, test_validator, payer),
        async_trial!(close_mint, test_validator, payer),
        async_trial!(required_transfer_memos, test_validator, payer),
        async_trial!(cpi_guard, test_validator, payer),
        async_trial!(immutable_accounts, test_validator, payer),
        async_trial!(non_transferable, test_validator, payer),
        async_trial!(default_account_state, test_validator, payer),
        async_trial!(transfer_fee, test_validator, payer),
        async_trial!(transfer_fee_basis_point, test_validator, payer),
        async_trial!(confidential_transfer, test_validator, payer),
        async_trial!(multisig_transfer, test_validator, payer),
        async_trial!(offline_multisig_transfer_with_nonce, test_validator, payer),
        async_trial!(
            withdraw_excess_lamports_from_multisig,
            test_validator,
            payer
        ),
        async_trial!(withdraw_excess_lamports_from_mint, test_validator, payer),
        async_trial!(withdraw_excess_lamports_from_account, test_validator, payer),
        async_trial!(metadata_pointer, test_validator, payer),
        async_trial!(group_pointer, test_validator, payer),
        async_trial!(group_member_pointer, test_validator, payer),
        async_trial!(transfer_hook, test_validator, payer),
        async_trial!(transfer_hook_with_transfer_fee, test_validator, payer),
        async_trial!(metadata, test_validator, payer),
        async_trial!(group, test_validator, payer),
        async_trial!(confidential_transfer_with_fee, test_validator, payer),
        async_trial!(compute_budget, test_validator, payer),
        // GC messes with every other test, so have it on its own test validator
        async_trial!(gc, gc_test_validator, gc_payer),
    ];

    libtest_mimic::run(&args, tests).exit();
}

fn clone_keypair(keypair: &Keypair) -> Keypair {
    Keypair::from_bytes(&keypair.to_bytes()).unwrap()
}

const TEST_DECIMALS: u8 = 9;

async fn new_validator_for_test() -> (TestValidator, Keypair) {
    solana_logger::setup();
    let mut test_validator_genesis = TestValidatorGenesis::default();
    test_validator_genesis.add_upgradeable_programs_with_path(&[
        UpgradeableProgramInfo {
            program_id: spl_token::id(),
            loader: bpf_loader_upgradeable::id(),
            program_path: PathBuf::from("../../target/deploy/spl_token.so"),
            upgrade_authority: Pubkey::new_unique(),
        },
        UpgradeableProgramInfo {
            program_id: spl_associated_token_account_client::program::id(),
            loader: bpf_loader_upgradeable::id(),
            program_path: PathBuf::from("../../target/deploy/spl_associated_token_account.so"),
            upgrade_authority: Pubkey::new_unique(),
        },
        UpgradeableProgramInfo {
            program_id: spl_token_2022::id(),
            loader: bpf_loader_upgradeable::id(),
            program_path: PathBuf::from("../../target/deploy/spl_token_2022.so"),
            upgrade_authority: Pubkey::new_unique(),
        },
    ]);
    test_validator_genesis.start_async().await
}

fn test_config_with_default_signer<'a>(
    test_validator: &TestValidator,
    payer: &Keypair,
    program_id: &Pubkey,
) -> Config<'a> {
    let websocket_url = test_validator.rpc_pubsub_url();
    let rpc_client = Arc::new(test_validator.get_async_rpc_client());
    let program_client: Arc<dyn ProgramClient<ProgramRpcClientSendTransaction>> = Arc::new(
        ProgramRpcClient::new(rpc_client.clone(), ProgramRpcClientSendTransaction),
    );
    Config {
        rpc_client,
        program_client,
        websocket_url,
        output_format: OutputFormat::JsonCompact,
        fee_payer: Some(Arc::new(clone_keypair(payer))),
        default_signer: Some(Arc::new(clone_keypair(payer))),
        nonce_account: None,
        nonce_authority: None,
        nonce_blockhash: None,
        sign_only: false,
        dump_transaction_message: false,
        multisigner_pubkeys: vec![],
        program_id: *program_id,
        restrict_to_program_id: true,
        compute_unit_price: None,
        compute_unit_limit: ComputeUnitLimit::Simulated,
    }
}

fn test_config_without_default_signer<'a>(
    test_validator: &TestValidator,
    program_id: &Pubkey,
) -> Config<'a> {
    let websocket_url = test_validator.rpc_pubsub_url();
    let rpc_client = Arc::new(test_validator.get_async_rpc_client());
    let program_client: Arc<dyn ProgramClient<ProgramRpcClientSendTransaction>> = Arc::new(
        ProgramRpcClient::new(rpc_client.clone(), ProgramRpcClientSendTransaction),
    );
    Config {
        rpc_client,
        program_client,
        websocket_url,
        output_format: OutputFormat::JsonCompact,
        fee_payer: None,
        default_signer: None,
        nonce_account: None,
        nonce_authority: None,
        nonce_blockhash: None,
        sign_only: false,
        dump_transaction_message: false,
        multisigner_pubkeys: vec![],
        program_id: *program_id,
        restrict_to_program_id: true,
        compute_unit_price: None,
        compute_unit_limit: ComputeUnitLimit::Simulated,
    }
}

async fn create_nonce(config: &Config<'_>, authority: &Keypair) -> Pubkey {
    let nonce = Keypair::new();

    let nonce_rent = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(solana_sdk::nonce::State::size())
        .await
        .unwrap();
    let instr = system_instruction::create_nonce_account(
        &authority.pubkey(),
        &nonce.pubkey(),
        &authority.pubkey(), // Make the fee payer the nonce account authority
        nonce_rent,
    );

    let blockhash = config.rpc_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &instr,
        Some(&authority.pubkey()),
        &[&nonce, authority],
        blockhash,
    );

    config
        .rpc_client
        .send_and_confirm_transaction(&tx)
        .await
        .unwrap();
    nonce.pubkey()
}

async fn do_create_native_mint(rpc_client: &RpcClient, payer: &Keypair) {
    let native_mint = spl_token_2022::native_mint::id();
    if rpc_client.get_account(&native_mint).await.is_err() {
        let transaction = Transaction::new_signed_with_payer(
            &[create_native_mint(&spl_token_2022::id(), &payer.pubkey()).unwrap()],
            Some(&payer.pubkey()),
            &[payer],
            rpc_client.get_latest_blockhash().await.unwrap(),
        );
        rpc_client
            .send_and_confirm_transaction(&transaction)
            .await
            .unwrap();
    }
}

async fn create_token(config: &Config<'_>, payer: &Keypair) -> Pubkey {
    let token = Keypair::new();
    let token_keypair_file = NamedTempFile::new().unwrap();
    write_keypair_file(&token, &token_keypair_file).unwrap();
    process_test_command(
        config,
        payer,
        &[
            "spl-token",
            CommandName::CreateToken.into(),
            token_keypair_file.path().to_str().unwrap(),
        ],
    )
    .await
    .unwrap();
    token.pubkey()
}

async fn create_interest_bearing_token(
    config: &Config<'_>,
    payer: &Keypair,
    rate_bps: i16,
) -> Pubkey {
    let token = Keypair::new();
    let token_keypair_file = NamedTempFile::new().unwrap();
    write_keypair_file(&token, &token_keypair_file).unwrap();
    process_test_command(
        config,
        payer,
        &[
            "spl-token",
            CommandName::CreateToken.into(),
            token_keypair_file.path().to_str().unwrap(),
            "--interest-rate",
            &rate_bps.to_string(),
        ],
    )
    .await
    .unwrap();
    token.pubkey()
}

async fn create_auxiliary_account(config: &Config<'_>, payer: &Keypair, mint: Pubkey) -> Pubkey {
    let auxiliary = Keypair::new();
    let auxiliary_keypair_file = NamedTempFile::new().unwrap();
    write_keypair_file(&auxiliary, &auxiliary_keypair_file).unwrap();
    process_test_command(
        config,
        payer,
        &[
            "spl-token",
            CommandName::CreateAccount.into(),
            &mint.to_string(),
            auxiliary_keypair_file.path().to_str().unwrap(),
        ],
    )
    .await
    .unwrap();
    auxiliary.pubkey()
}

async fn create_associated_account(
    config: &Config<'_>,
    payer: &Keypair,
    mint: &Pubkey,
    owner: &Pubkey,
) -> Pubkey {
    process_test_command(
        config,
        payer,
        &[
            "spl-token",
            CommandName::CreateAccount.into(),
            &mint.to_string(),
            "--owner",
            &owner.to_string(),
        ],
    )
    .await
    .unwrap();
    get_associated_token_address_with_program_id(owner, mint, &config.program_id)
}

async fn mint_tokens(
    config: &Config<'_>,
    payer: &Keypair,
    mint: Pubkey,
    ui_amount: f64,
    recipient: Pubkey,
) -> CommandResult {
    process_test_command(
        config,
        payer,
        &[
            "spl-token",
            CommandName::Mint.into(),
            &mint.to_string(),
            &ui_amount.to_string(),
            &recipient.to_string(),
        ],
    )
    .await
}

async fn run_transfer_test(config: &Config<'_>, payer: &Keypair) {
    let token = create_token(config, payer).await;
    let source = create_associated_account(config, payer, &token, &payer.pubkey()).await;
    let destination = create_auxiliary_account(config, payer, token).await;
    let ui_amount = 100.0;
    mint_tokens(config, payer, token, ui_amount, source)
        .await
        .unwrap();
    let result = process_test_command(
        config,
        payer,
        &[
            "spl-token",
            CommandName::Transfer.into(),
            &token.to_string(),
            "10",
            &destination.to_string(),
        ],
    )
    .await;
    result.unwrap();

    let account = config.rpc_client.get_account(&source).await.unwrap();
    let token_account = StateWithExtensionsOwned::<Account>::unpack(account.data).unwrap();
    let amount = spl_token::ui_amount_to_amount(90.0, TEST_DECIMALS);
    assert_eq!(token_account.base.amount, amount);
    let account = config.rpc_client.get_account(&destination).await.unwrap();
    let token_account = StateWithExtensionsOwned::<Account>::unpack(account.data).unwrap();
    let amount = spl_token::ui_amount_to_amount(10.0, TEST_DECIMALS);
    assert_eq!(token_account.base.amount, amount);
}

async fn process_test_command<I, T>(config: &Config<'_>, payer: &Keypair, args: I) -> CommandResult
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let default_decimals = format!("{}", spl_token_2022::native_mint::DECIMALS);
    let minimum_signers_help = minimum_signers_help_string();
    let multisig_member_help = multisig_member_help_string();

    let app_matches = app(
        &default_decimals,
        &minimum_signers_help,
        &multisig_member_help,
    )
    .get_matches_from(args);
    let (sub_command, matches) = app_matches.subcommand().unwrap();
    let sub_command = CommandName::from_str(sub_command).unwrap();

    let wallet_manager = None;
    let bulk_signers: Vec<Arc<dyn Signer>> = vec![Arc::new(clone_keypair(payer))];
    process_command(&sub_command, matches, config, wallet_manager, bulk_signers).await
}

async fn exec_test_cmd<T: AsRef<OsStr>>(config: &Config<'_>, args: &[T]) -> CommandResult {
    let default_decimals = format!("{}", spl_token_2022::native_mint::DECIMALS);
    let minimum_signers_help = minimum_signers_help_string();
    let multisig_member_help = multisig_member_help_string();

    let app_matches = app(
        &default_decimals,
        &minimum_signers_help,
        &multisig_member_help,
    )
    .get_matches_from(args);
    let (sub_command, matches) = app_matches.subcommand().unwrap();
    let sub_command = CommandName::from_str(sub_command).unwrap();

    let mut wallet_manager = None;
    let mut bulk_signers: Vec<Arc<dyn Signer>> = Vec::new();
    let mut multisigner_ids = Vec::new();

    let config = Config::new_with_clients_and_ws_url(
        matches,
        &mut wallet_manager,
        &mut bulk_signers,
        &mut multisigner_ids,
        config.rpc_client.clone(),
        config.program_client.clone(),
        config.websocket_url.clone(),
    )
    .await;

    process_command(&sub_command, matches, &config, wallet_manager, bulk_signers).await
}

async fn create_token_default(test_validator: &TestValidator, payer: &Keypair) {
    for program_id in VALID_TOKEN_PROGRAM_IDS.iter() {
        let config = test_config_with_default_signer(test_validator, payer, program_id);
        let result = process_test_command(
            &config,
            payer,
            &["spl-token", CommandName::CreateToken.into()],
        )
        .await;
        let value: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        let mint = Pubkey::from_str(value["commandOutput"]["address"].as_str().unwrap()).unwrap();
        let account = config.rpc_client.get_account(&mint).await.unwrap();
        assert_eq!(account.owner, *program_id);
    }
}

async fn create_token_2022(test_validator: &TestValidator, payer: &Keypair) {
    let config = test_config_with_default_signer(test_validator, payer, &spl_token_2022::id());
    let mut wallet_manager = None;
    let mut bulk_signers: Vec<Arc<dyn Signer>> = Vec::new();
    let mut multisigner_ids = Vec::new();

    let args = &[
        "spl-token",
        CommandName::CreateToken.into(),
        "--program-2022",
    ];

    let default_decimals = format!("{}", spl_token_2022::native_mint::DECIMALS);
    let minimum_signers_help = minimum_signers_help_string();
    let multisig_member_help = multisig_member_help_string();

    let app_matches = app(
        &default_decimals,
        &minimum_signers_help,
        &multisig_member_help,
    )
    .get_matches_from(args);
    let (_, matches) = app_matches.subcommand().unwrap();

    let config = Config::new_with_clients_and_ws_url(
        matches,
        &mut wallet_manager,
        &mut bulk_signers,
        &mut multisigner_ids,
        config.rpc_client.clone(),
        config.program_client.clone(),
        config.websocket_url.clone(),
    )
    .await;

    assert_eq!(config.program_id, spl_token_2022::ID);
}

async fn create_token_interest_bearing(test_validator: &TestValidator, payer: &Keypair) {
    let config = test_config_with_default_signer(test_validator, payer, &spl_token_2022::id());
    let rate_bps: i16 = 100;
    let result = process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::CreateToken.into(),
            "--interest-rate",
            &rate_bps.to_string(),
        ],
    )
    .await;
    let value: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    let mint = Pubkey::from_str(value["commandOutput"]["address"].as_str().unwrap()).unwrap();
    let account = config.rpc_client.get_account(&mint).await.unwrap();
    let mint_account = StateWithExtensionsOwned::<Mint>::unpack(account.data).unwrap();
    let extension = mint_account
        .get_extension::<InterestBearingConfig>()
        .unwrap();
    assert_eq!(account.owner, spl_token_2022::id());
    assert_eq!(i16::from(extension.current_rate), rate_bps);
    assert_eq!(
        Option::<Pubkey>::from(extension.rate_authority),
        Some(payer.pubkey())
    );
}

async fn set_interest_rate(test_validator: &TestValidator, payer: &Keypair) {
    let config = test_config_with_default_signer(test_validator, payer, &spl_token_2022::id());
    let initial_rate: i16 = 100;
    let new_rate: i16 = 300;
    let token = create_interest_bearing_token(&config, payer, initial_rate).await;
    let account = config.rpc_client.get_account(&token).await.unwrap();
    let mint_account = StateWithExtensionsOwned::<Mint>::unpack(account.data).unwrap();
    let extension = mint_account
        .get_extension::<InterestBearingConfig>()
        .unwrap();
    assert_eq!(account.owner, spl_token_2022::id());
    assert_eq!(i16::from(extension.current_rate), initial_rate);

    let result = process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::SetInterestRate.into(),
            &token.to_string(),
            &new_rate.to_string(),
        ],
    )
    .await;
    let _value: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    let account = config.rpc_client.get_account(&token).await.unwrap();
    let mint_account = StateWithExtensionsOwned::<Mint>::unpack(account.data).unwrap();
    let extension = mint_account
        .get_extension::<InterestBearingConfig>()
        .unwrap();
    assert_eq!(i16::from(extension.current_rate), new_rate);
}

async fn supply(test_validator: &TestValidator, payer: &Keypair) {
    for program_id in VALID_TOKEN_PROGRAM_IDS.iter() {
        let config = test_config_with_default_signer(test_validator, payer, program_id);
        let token = create_token(&config, payer).await;
        let result = process_test_command(
            &config,
            payer,
            &["spl-token", CommandName::Supply.into(), &token.to_string()],
        )
        .await;
        let value: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(value["amount"], "0");
        assert_eq!(value["uiAmountString"], "0");
    }
}

async fn create_account_default(test_validator: &TestValidator, payer: &Keypair) {
    for program_id in VALID_TOKEN_PROGRAM_IDS.iter() {
        let config = test_config_with_default_signer(test_validator, payer, program_id);
        let token = create_token(&config, payer).await;
        let result = process_test_command(
            &config,
            payer,
            &[
                "spl-token",
                CommandName::CreateAccount.into(),
                &token.to_string(),
            ],
        )
        .await;
        result.unwrap();
    }
}

async fn account_info(test_validator: &TestValidator, payer: &Keypair) {
    for program_id in VALID_TOKEN_PROGRAM_IDS.iter() {
        let config = test_config_with_default_signer(test_validator, payer, program_id);
        let token = create_token(&config, payer).await;
        let _account = create_associated_account(&config, payer, &token, &payer.pubkey()).await;
        let result = process_test_command(
            &config,
            payer,
            &[
                "spl-token",
                CommandName::AccountInfo.into(),
                &token.to_string(),
            ],
        )
        .await;
        let value: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        let account = get_associated_token_address_with_program_id(
            &payer.pubkey(),
            &token,
            &config.program_id,
        );
        assert_eq!(value["address"], account.to_string());
        assert_eq!(value["mint"], token.to_string());
        assert_eq!(value["isAssociated"], true);
        assert_eq!(value["isNative"], false);
        assert_eq!(value["owner"], payer.pubkey().to_string());
        assert_eq!(value["state"], "initialized");
    }
}

async fn balance(test_validator: &TestValidator, payer: &Keypair) {
    for program_id in VALID_TOKEN_PROGRAM_IDS.iter() {
        let config = test_config_with_default_signer(test_validator, payer, program_id);
        let token = create_token(&config, payer).await;
        let _account = create_associated_account(&config, payer, &token, &payer.pubkey()).await;
        let result = process_test_command(
            &config,
            payer,
            &["spl-token", CommandName::Balance.into(), &token.to_string()],
        )
        .await;
        let value: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(value["amount"], "0");
        assert_eq!(value["uiAmountString"], "0");
    }
}

async fn mint(test_validator: &TestValidator, payer: &Keypair) {
    for program_id in VALID_TOKEN_PROGRAM_IDS.iter() {
        let config = test_config_with_default_signer(test_validator, payer, program_id);
        let token = create_token(&config, payer).await;
        let account = create_associated_account(&config, payer, &token, &payer.pubkey()).await;
        let mut amount = 0;

        // mint via implicit owner
        process_test_command(
            &config,
            payer,
            &[
                "spl-token",
                CommandName::Mint.into(),
                &token.to_string(),
                "1",
            ],
        )
        .await
        .unwrap();
        amount += spl_token::ui_amount_to_amount(1.0, TEST_DECIMALS);

        let account_data = config.rpc_client.get_account(&account).await.unwrap();
        let token_account = StateWithExtensionsOwned::<Account>::unpack(account_data.data).unwrap();
        assert_eq!(token_account.base.amount, amount);
        assert_eq!(token_account.base.mint, token);
        assert_eq!(token_account.base.owner, payer.pubkey());

        // mint via explicit recipient
        process_test_command(
            &config,
            payer,
            &[
                "spl-token",
                CommandName::Mint.into(),
                &token.to_string(),
                "1",
                &account.to_string(),
            ],
        )
        .await
        .unwrap();
        amount += spl_token::ui_amount_to_amount(1.0, TEST_DECIMALS);

        let account_data = config.rpc_client.get_account(&account).await.unwrap();
        let token_account = StateWithExtensionsOwned::<Account>::unpack(account_data.data).unwrap();
        assert_eq!(token_account.base.amount, amount);
        assert_eq!(token_account.base.mint, token);
        assert_eq!(token_account.base.owner, payer.pubkey());

        // mint via explicit owner
        process_test_command(
            &config,
            payer,
            &[
                "spl-token",
                CommandName::Mint.into(),
                &token.to_string(),
                "1",
                "--recipient-owner",
                &payer.pubkey().to_string(),
            ],
        )
        .await
        .unwrap();
        amount += spl_token::ui_amount_to_amount(1.0, TEST_DECIMALS);

        let account_data = config.rpc_client.get_account(&account).await.unwrap();
        let token_account = StateWithExtensionsOwned::<Account>::unpack(account_data.data).unwrap();
        assert_eq!(token_account.base.amount, amount);
        assert_eq!(token_account.base.mint, token);
        assert_eq!(token_account.base.owner, payer.pubkey());
    }
}

async fn balance_after_mint(test_validator: &TestValidator, payer: &Keypair) {
    for program_id in VALID_TOKEN_PROGRAM_IDS.iter() {
        let config = test_config_with_default_signer(test_validator, payer, program_id);
        let token = create_token(&config, payer).await;
        let account = create_associated_account(&config, payer, &token, &payer.pubkey()).await;
        let ui_amount = 100.0;
        mint_tokens(&config, payer, token, ui_amount, account)
            .await
            .unwrap();
        let result = process_test_command(
            &config,
            payer,
            &["spl-token", CommandName::Balance.into(), &token.to_string()],
        )
        .await;
        let value: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        let amount = spl_token::ui_amount_to_amount(ui_amount, TEST_DECIMALS);
        assert_eq!(value["amount"], format!("{}", amount));
        assert_eq!(value["uiAmountString"], format!("{}", ui_amount));
    }
}

async fn balance_after_mint_with_owner(test_validator: &TestValidator, payer: &Keypair) {
    for program_id in VALID_TOKEN_PROGRAM_IDS.iter() {
        let config = test_config_with_default_signer(test_validator, payer, program_id);
        let token = create_token(&config, payer).await;
        let account = create_associated_account(&config, payer, &token, &payer.pubkey()).await;
        let ui_amount = 100.0;
        mint_tokens(&config, payer, token, ui_amount, account)
            .await
            .unwrap();
        let config = test_config_without_default_signer(test_validator, program_id);
        let result = process_test_command(
            &config,
            payer,
            &[
                "spl-token",
                CommandName::Balance.into(),
                &token.to_string(),
                "--owner",
                &payer.pubkey().to_string(),
            ],
        )
        .await;
        let value: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        let amount = spl_token::ui_amount_to_amount(ui_amount, TEST_DECIMALS);
        assert_eq!(value["amount"], format!("{}", amount));
        assert_eq!(value["uiAmountString"], format!("{}", ui_amount));
    }
}

async fn accounts(test_validator: &TestValidator, payer: &Keypair) {
    for program_id in VALID_TOKEN_PROGRAM_IDS.iter() {
        let config = test_config_with_default_signer(test_validator, payer, program_id);
        let token1 = create_token(&config, payer).await;
        let _account1 = create_associated_account(&config, payer, &token1, &payer.pubkey()).await;
        let token2 = create_token(&config, payer).await;
        let _account2 = create_associated_account(&config, payer, &token2, &payer.pubkey()).await;
        let token3 = create_token(&config, payer).await;
        let result =
            process_test_command(&config, payer, &["spl-token", CommandName::Accounts.into()])
                .await
                .unwrap();
        assert!(result.contains(&token1.to_string()));
        assert!(result.contains(&token2.to_string()));
        assert!(!result.contains(&token3.to_string()));
    }
}

async fn accounts_with_owner(test_validator: &TestValidator, payer: &Keypair) {
    for program_id in VALID_TOKEN_PROGRAM_IDS.iter() {
        let config = test_config_with_default_signer(test_validator, payer, program_id);
        let token1 = create_token(&config, payer).await;
        let _account1 = create_associated_account(&config, payer, &token1, &payer.pubkey()).await;
        let token2 = create_token(&config, payer).await;
        let _account2 = create_associated_account(&config, payer, &token2, &payer.pubkey()).await;
        let token3 = create_token(&config, payer).await;
        let config = test_config_without_default_signer(test_validator, program_id);
        let result = process_test_command(
            &config,
            payer,
            &[
                "spl-token",
                CommandName::Accounts.into(),
                "--owner",
                &payer.pubkey().to_string(),
            ],
        )
        .await
        .unwrap();
        assert!(result.contains(&token1.to_string()));
        assert!(result.contains(&token2.to_string()));
        assert!(!result.contains(&token3.to_string()));
    }
}

async fn wrapped_sol(test_validator: &TestValidator, payer: &Keypair) {
    for program_id in VALID_TOKEN_PROGRAM_IDS.iter() {
        let config = test_config_with_default_signer(test_validator, payer, program_id);
        let native_mint = *Token::new_native(
            config.program_client.clone(),
            program_id,
            config.fee_payer().unwrap().clone(),
        )
        .get_address();
        let _result = process_test_command(
            &config,
            payer,
            &["spl-token", CommandName::Wrap.into(), "0.5"],
        )
        .await
        .unwrap();
        let wrapped_address = get_associated_token_address_with_program_id(
            &payer.pubkey(),
            &native_mint,
            &config.program_id,
        );
        let account = config
            .rpc_client
            .get_account(&wrapped_address)
            .await
            .unwrap();
        let token_account = StateWithExtensionsOwned::<Account>::unpack(account.data).unwrap();
        assert_eq!(token_account.base.mint, native_mint);
        assert_eq!(token_account.base.owner, payer.pubkey());
        assert!(token_account.base.is_native());
        let result = process_test_command(
            &config,
            payer,
            &[
                "spl-token",
                CommandName::Unwrap.into(),
                &wrapped_address.to_string(),
            ],
        )
        .await;
        result.unwrap();
        config
            .rpc_client
            .get_account(&wrapped_address)
            .await
            .unwrap_err();

        // now use `close` to close it
        let token = create_token(&config, payer).await;
        let source = create_associated_account(&config, payer, &token, &payer.pubkey()).await;
        let _result = process_test_command(
            &config,
            payer,
            &["spl-token", CommandName::Wrap.into(), "10.0"],
        )
        .await
        .unwrap();

        let recipient =
            get_associated_token_address_with_program_id(&payer.pubkey(), &native_mint, program_id);
        let result = process_test_command(
            &config,
            payer,
            &[
                "spl-token",
                CommandName::Close.into(),
                "--address",
                &source.to_string(),
                "--recipient",
                &recipient.to_string(),
            ],
        )
        .await;
        result.unwrap();

        let ui_account = config
            .rpc_client
            .get_token_account(&recipient)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ui_account.token_amount.amount, "10000000000");
    }
}

async fn transfer(test_validator: &TestValidator, payer: &Keypair) {
    for program_id in VALID_TOKEN_PROGRAM_IDS.iter() {
        let config = test_config_with_default_signer(test_validator, payer, program_id);
        run_transfer_test(&config, payer).await;
    }
}

async fn transfer_fund_recipient(test_validator: &TestValidator, payer: &Keypair) {
    for program_id in VALID_TOKEN_PROGRAM_IDS.iter() {
        let config = test_config_with_default_signer(test_validator, payer, program_id);
        let token = create_token(&config, payer).await;
        let source = create_associated_account(&config, payer, &token, &payer.pubkey()).await;
        let recipient = Keypair::new().pubkey().to_string();
        let ui_amount = 100.0;
        mint_tokens(&config, payer, token, ui_amount, source)
            .await
            .unwrap();
        let result = process_test_command(
            &config,
            payer,
            &[
                "spl-token",
                CommandName::Transfer.into(),
                "--fund-recipient",
                "--allow-unfunded-recipient",
                &token.to_string(),
                "10",
                &recipient,
            ],
        )
        .await;
        result.unwrap();

        let account = config.rpc_client.get_account(&source).await.unwrap();
        let token_account = StateWithExtensionsOwned::<Account>::unpack(account.data).unwrap();
        assert_eq!(
            token_account.base.amount,
            spl_token::ui_amount_to_amount(90.0, TEST_DECIMALS)
        );
    }
}

async fn transfer_non_standard_recipient(test_validator: &TestValidator, payer: &Keypair) {
    for program_id in VALID_TOKEN_PROGRAM_IDS.iter() {
        for other_program_id in VALID_TOKEN_PROGRAM_IDS
            .iter()
            .filter(|id| *id != program_id)
        {
            let mut config =
                test_config_with_default_signer(test_validator, payer, other_program_id);
            let wrong_program_token = create_token(&config, payer).await;
            let wrong_program_account =
                create_associated_account(&config, payer, &wrong_program_token, &payer.pubkey())
                    .await;
            config.program_id = *program_id;
            let config = config;

            let token = create_token(&config, payer).await;
            let source = create_associated_account(&config, payer, &token, &payer.pubkey()).await;
            let recipient = Keypair::new().pubkey();
            let recipient_token_account = get_associated_token_address_with_program_id(
                &recipient,
                &token,
                &config.program_id,
            );
            let system_token_account = get_associated_token_address_with_program_id(
                &system_program::id(),
                &token,
                &config.program_id,
            );
            let amount = 100;
            mint_tokens(&config, payer, token, amount as f64, source)
                .await
                .unwrap();

            // transfer fails to unfunded recipient without flag
            process_test_command(
                &config,
                payer,
                &[
                    "spl-token",
                    CommandName::Transfer.into(),
                    "--fund-recipient",
                    &token.to_string(),
                    "1",
                    &recipient.to_string(),
                ],
            )
            .await
            .unwrap_err();

            // with unfunded flag, transfer goes through
            process_test_command(
                &config,
                payer,
                &[
                    "spl-token",
                    CommandName::Transfer.into(),
                    "--fund-recipient",
                    "--allow-unfunded-recipient",
                    &token.to_string(),
                    "1",
                    &recipient.to_string(),
                ],
            )
            .await
            .unwrap();
            let account = config
                .rpc_client
                .get_account(&recipient_token_account)
                .await
                .unwrap();
            let token_account = StateWithExtensionsOwned::<Account>::unpack(account.data).unwrap();
            assert_eq!(
                token_account.base.amount,
                spl_token::ui_amount_to_amount(1.0, TEST_DECIMALS)
            );

            // transfer fails to non-system recipient without flag
            process_test_command(
                &config,
                payer,
                &[
                    "spl-token",
                    CommandName::Transfer.into(),
                    "--fund-recipient",
                    &token.to_string(),
                    "1",
                    &system_program::id().to_string(),
                ],
            )
            .await
            .unwrap_err();

            // with non-system flag, transfer goes through
            process_test_command(
                &config,
                payer,
                &[
                    "spl-token",
                    CommandName::Transfer.into(),
                    "--fund-recipient",
                    "--allow-non-system-account-recipient",
                    &token.to_string(),
                    "1",
                    &system_program::id().to_string(),
                ],
            )
            .await
            .unwrap();
            let account = config
                .rpc_client
                .get_account(&system_token_account)
                .await
                .unwrap();
            let token_account = StateWithExtensionsOwned::<Account>::unpack(account.data).unwrap();
            assert_eq!(
                token_account.base.amount,
                spl_token::ui_amount_to_amount(1.0, TEST_DECIMALS)
            );

            // transfer to same-program non-account fails
            process_test_command(
                &config,
                payer,
                &[
                    "spl-token",
                    CommandName::Transfer.into(),
                    "--fund-recipient",
                    "--allow-non-system-account-recipient",
                    "--allow-unfunded-recipient",
                    &token.to_string(),
                    "1",
                    &token.to_string(),
                ],
            )
            .await
            .unwrap_err();

            // transfer to other-program account fails
            process_test_command(
                &config,
                payer,
                &[
                    "spl-token",
                    CommandName::Transfer.into(),
                    "--fund-recipient",
                    "--allow-non-system-account-recipient",
                    "--allow-unfunded-recipient",
                    &token.to_string(),
                    "1",
                    &wrong_program_account.to_string(),
                ],
            )
            .await
            .unwrap_err();
        }
    }
}

async fn allow_non_system_account_recipient(test_validator: &TestValidator, payer: &Keypair) {
    let config = test_config_with_default_signer(test_validator, payer, &spl_token::id());

    let token = create_token(&config, payer).await;
    let source = create_associated_account(&config, payer, &token, &payer.pubkey()).await;
    let recipient = Keypair::new().pubkey().to_string();
    let ui_amount = 100.0;
    mint_tokens(&config, payer, token, ui_amount, source)
        .await
        .unwrap();
    let result = process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::Transfer.into(),
            "--fund-recipient",
            "--allow-non-system-account-recipient",
            "--allow-unfunded-recipient",
            &token.to_string(),
            "10",
            &recipient,
        ],
    )
    .await;
    result.unwrap();

    let ui_account = config
        .rpc_client
        .get_token_account(&source)
        .await
        .unwrap()
        .unwrap();
    let amount = spl_token::ui_amount_to_amount(90.0, TEST_DECIMALS);
    assert_eq!(ui_account.token_amount.amount, format!("{amount}"));
}

async fn close_account(test_validator: &TestValidator, payer: &Keypair) {
    for program_id in VALID_TOKEN_PROGRAM_IDS.iter() {
        let config = test_config_with_default_signer(test_validator, payer, program_id);

        let native_mint = Token::new_native(
            config.program_client.clone(),
            program_id,
            config.fee_payer().unwrap().clone(),
        );
        let recipient_owner = Pubkey::new_unique();
        native_mint
            .get_or_create_associated_account_info(&recipient_owner)
            .await
            .unwrap();

        let token = create_token(&config, payer).await;

        let system_recipient = Keypair::new().pubkey();
        let wsol_recipient = native_mint.get_associated_token_address(&recipient_owner);

        for recipient in [system_recipient, wsol_recipient] {
            let base_balance = config
                .rpc_client
                .get_account(&recipient)
                .await
                .map(|account| account.lamports)
                .unwrap_or(0);

            let source = create_auxiliary_account(&config, payer, token).await;
            let token_rent_amount = config
                .rpc_client
                .get_account(&source)
                .await
                .unwrap()
                .lamports;

            process_test_command(
                &config,
                payer,
                &[
                    "spl-token",
                    CommandName::Close.into(),
                    "--address",
                    &source.to_string(),
                    "--recipient",
                    &recipient.to_string(),
                ],
            )
            .await
            .unwrap();

            let recipient_data = config.rpc_client.get_account(&recipient).await.unwrap();

            assert_eq!(recipient_data.lamports, base_balance + token_rent_amount);
            if recipient == wsol_recipient {
                let recipient_account =
                    StateWithExtensionsOwned::<Account>::unpack(recipient_data.data).unwrap();
                assert_eq!(recipient_account.base.amount, token_rent_amount);
            }
        }
    }
}

async fn disable_mint_authority(test_validator: &TestValidator, payer: &Keypair) {
    for program_id in VALID_TOKEN_PROGRAM_IDS.iter() {
        let config = test_config_with_default_signer(test_validator, payer, program_id);
        let token = create_token(&config, payer).await;
        let result = process_test_command(
            &config,
            payer,
            &[
                "spl-token",
                CommandName::Authorize.into(),
                &token.to_string(),
                "mint",
                "--disable",
            ],
        )
        .await;
        result.unwrap();

        let account = config.rpc_client.get_account(&token).await.unwrap();
        let mint = StateWithExtensionsOwned::<Mint>::unpack(account.data).unwrap();
        assert_eq!(mint.base.mint_authority, COption::None);
    }
}

async fn gc(test_validator: &TestValidator, payer: &Keypair) {
    for program_id in VALID_TOKEN_PROGRAM_IDS.iter() {
        let mut config = test_config_with_default_signer(test_validator, payer, program_id);
        let token = create_token(&config, payer).await;
        let _account = create_associated_account(&config, payer, &token, &payer.pubkey()).await;
        let _aux1 = create_auxiliary_account(&config, payer, token).await;
        let _aux2 = create_auxiliary_account(&config, payer, token).await;
        let _aux3 = create_auxiliary_account(&config, payer, token).await;
        let result = process_test_command(
            &config,
            payer,
            &[
                "spl-token",
                CommandName::Accounts.into(),
                &token.to_string(),
            ],
        )
        .await
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(
            value["accounts"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|x| x["mint"] == token.to_string())
                .count(),
            4
        );
        config.output_format = OutputFormat::Display; // fixup eventually?
        let _result = process_test_command(&config, payer, &["spl-token", CommandName::Gc.into()])
            .await
            .unwrap();
        config.output_format = OutputFormat::JsonCompact;
        let result = process_test_command(
            &config,
            payer,
            &[
                "spl-token",
                CommandName::Accounts.into(),
                &token.to_string(),
            ],
        )
        .await
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(
            value["accounts"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|x| x["mint"] == token.to_string())
                .count(),
            1
        );

        config.output_format = OutputFormat::Display;

        // test implicit transfer
        let token = create_token(&config, payer).await;
        let ata = create_associated_account(&config, payer, &token, &payer.pubkey()).await;
        let aux = create_auxiliary_account(&config, payer, token).await;
        mint_tokens(&config, payer, token, 1.0, ata).await.unwrap();
        mint_tokens(&config, payer, token, 1.0, aux).await.unwrap();

        process_test_command(&config, payer, &["spl-token", CommandName::Gc.into()])
            .await
            .unwrap();

        let ui_ata = config
            .rpc_client
            .get_token_account(&ata)
            .await
            .unwrap()
            .unwrap();

        // aux is gone and its tokens are in ata
        let amount = spl_token::ui_amount_to_amount(2.0, TEST_DECIMALS);
        assert_eq!(ui_ata.token_amount.amount, format!("{amount}"));
        config.rpc_client.get_account(&aux).await.unwrap_err();

        // test ata closure
        let token = create_token(&config, payer).await;
        let ata = create_associated_account(&config, payer, &token, &payer.pubkey()).await;

        process_test_command(
            &config,
            payer,
            &[
                "spl-token",
                CommandName::Gc.into(),
                "--close-empty-associated-accounts",
            ],
        )
        .await
        .unwrap();

        // ata is gone
        config.rpc_client.get_account(&ata).await.unwrap_err();

        // test a tricky corner case of both
        let token = create_token(&config, payer).await;
        let ata = create_associated_account(&config, payer, &token, &payer.pubkey()).await;
        let aux = create_auxiliary_account(&config, payer, token).await;
        mint_tokens(&config, payer, token, 1.0, aux).await.unwrap();

        process_test_command(
            &config,
            payer,
            &[
                "spl-token",
                CommandName::Gc.into(),
                "--close-empty-associated-accounts",
            ],
        )
        .await
        .unwrap();

        let ui_ata = config
            .rpc_client
            .get_token_account(&ata)
            .await
            .unwrap()
            .unwrap();

        // aux is gone and its tokens are in ata, and ata has not been closed
        let amount = spl_token::ui_amount_to_amount(1.0, TEST_DECIMALS);
        assert_eq!(ui_ata.token_amount.amount, format!("{amount}"));
        config.rpc_client.get_account(&aux).await.unwrap_err();

        // test that balance moves off an uncloseable account
        let token = create_token(&config, payer).await;
        let ata = create_associated_account(&config, payer, &token, &payer.pubkey()).await;
        let aux = create_auxiliary_account(&config, payer, token).await;
        let close_authority = Keypair::new().pubkey();
        mint_tokens(&config, payer, token, 1.0, aux).await.unwrap();

        process_test_command(
            &config,
            payer,
            &[
                "spl-token",
                CommandName::Authorize.into(),
                &aux.to_string(),
                "close",
                &close_authority.to_string(),
            ],
        )
        .await
        .unwrap();

        process_test_command(&config, payer, &["spl-token", CommandName::Gc.into()])
            .await
            .unwrap();

        let ui_ata = config
            .rpc_client
            .get_token_account(&ata)
            .await
            .unwrap()
            .unwrap();

        // aux tokens are now in ata
        let amount = spl_token::ui_amount_to_amount(1.0, TEST_DECIMALS);
        assert_eq!(ui_ata.token_amount.amount, format!("{amount}"));
    }
}

async fn set_owner(test_validator: &TestValidator, payer: &Keypair) {
    for program_id in VALID_TOKEN_PROGRAM_IDS.iter() {
        let config = test_config_with_default_signer(test_validator, payer, program_id);
        let token = create_token(&config, payer).await;
        let aux = create_auxiliary_account(&config, payer, token).await;
        let aux_string = aux.to_string();
        let _result = process_test_command(
            &config,
            payer,
            &[
                "spl-token",
                CommandName::Authorize.into(),
                &aux_string,
                "owner",
                &aux_string,
            ],
        )
        .await
        .unwrap();
        let account = config.rpc_client.get_account(&aux).await.unwrap();
        let token_account = StateWithExtensionsOwned::<Account>::unpack(account.data).unwrap();
        assert_eq!(token_account.base.mint, token);
        assert_eq!(token_account.base.owner, aux);
    }
}

async fn transfer_with_account_delegate(test_validator: &TestValidator, payer: &Keypair) {
    for program_id in VALID_TOKEN_PROGRAM_IDS.iter() {
        let config = test_config_with_default_signer(test_validator, payer, program_id);

        let token = create_token(&config, payer).await;
        let source = create_associated_account(&config, payer, &token, &payer.pubkey()).await;
        let destination = create_auxiliary_account(&config, payer, token).await;
        let delegate = Keypair::new();

        let delegate_keypair_file = NamedTempFile::new().unwrap();
        write_keypair_file(&delegate, &delegate_keypair_file).unwrap();
        let fee_payer_keypair_file = NamedTempFile::new().unwrap();
        write_keypair_file(payer, &fee_payer_keypair_file).unwrap();

        let ui_amount = 100.0;
        mint_tokens(&config, payer, token, ui_amount, source)
            .await
            .unwrap();

        let ui_account = config
            .rpc_client
            .get_token_account(&source)
            .await
            .unwrap()
            .unwrap();
        let amount = spl_token::ui_amount_to_amount(100.0, TEST_DECIMALS);
        assert_eq!(ui_account.token_amount.amount, format!("{amount}"));
        assert_eq!(ui_account.delegate, None);
        assert_eq!(ui_account.delegated_amount, None);
        let ui_account = config
            .rpc_client
            .get_token_account(&destination)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ui_account.token_amount.amount, "0");

        exec_test_cmd(
            &config,
            &[
                "spl-token",
                CommandName::Approve.into(),
                &source.to_string(),
                "10",
                &delegate.pubkey().to_string(),
                "--owner",
                fee_payer_keypair_file.path().to_str().unwrap(),
                "--fee-payer",
                fee_payer_keypair_file.path().to_str().unwrap(),
                "--program-id",
                &program_id.to_string(),
            ],
        )
        .await
        .unwrap();

        let ui_account = config
            .rpc_client
            .get_token_account(&source)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ui_account.delegate.unwrap(), delegate.pubkey().to_string());
        let amount = spl_token::ui_amount_to_amount(10.0, TEST_DECIMALS);
        assert_eq!(
            ui_account.delegated_amount.unwrap().amount,
            format!("{amount}")
        );

        let result = exec_test_cmd(
            &config,
            &[
                "spl-token",
                CommandName::Transfer.into(),
                &token.to_string(),
                "10",
                &destination.to_string(),
                "--from",
                &source.to_string(),
                "--owner",
                delegate_keypair_file.path().to_str().unwrap(),
                "--fee-payer",
                fee_payer_keypair_file.path().to_str().unwrap(),
                "--program-id",
                &program_id.to_string(),
            ],
        )
        .await;
        result.unwrap();

        let ui_account = config
            .rpc_client
            .get_token_account(&source)
            .await
            .unwrap()
            .unwrap();
        let amount = spl_token::ui_amount_to_amount(90.0, TEST_DECIMALS);
        assert_eq!(ui_account.token_amount.amount, format!("{amount}"));
        assert_eq!(ui_account.delegate, None);
        assert_eq!(ui_account.delegated_amount, None);
        let ui_account = config
            .rpc_client
            .get_token_account(&destination)
            .await
            .unwrap()
            .unwrap();
        let amount = spl_token::ui_amount_to_amount(10.0, TEST_DECIMALS);
        assert_eq!(ui_account.token_amount.amount, format!("{amount}"));
    }
}

async fn burn(test_validator: &TestValidator, payer: &Keypair) {
    for program_id in VALID_TOKEN_PROGRAM_IDS.iter() {
        let mut config = test_config_with_default_signer(test_validator, payer, program_id);
        let token = create_token(&config, payer).await;
        let source = create_associated_account(&config, payer, &token, &payer.pubkey()).await;
        let ui_amount = 100.0;
        mint_tokens(&config, payer, token, ui_amount, source)
            .await
            .unwrap();

        process_test_command(
            &config,
            payer,
            &[
                "spl-token",
                CommandName::Burn.into(),
                &source.to_string(),
                "10",
            ],
        )
        .await
        .unwrap();

        let account = config.rpc_client.get_account(&source).await.unwrap();
        let token_account = StateWithExtensionsOwned::<Account>::unpack(account.data).unwrap();
        let amount = spl_token::ui_amount_to_amount(90.0, TEST_DECIMALS);
        assert_eq!(token_account.base.amount, amount);

        process_test_command(
            &config,
            payer,
            &[
                "spl-token",
                CommandName::Burn.into(),
                &source.to_string(),
                "ALL",
            ],
        )
        .await
        .unwrap();

        let account = config.rpc_client.get_account(&source).await.unwrap();
        let token_account = StateWithExtensionsOwned::<Account>::unpack(account.data).unwrap();
        let amount = spl_token::ui_amount_to_amount(0.0, TEST_DECIMALS);
        assert_eq!(token_account.base.amount, amount);

        let result = process_test_command(
            &config,
            payer,
            &[
                "spl-token",
                CommandName::Burn.into(),
                &source.to_string(),
                "10",
            ],
        )
        .await;
        assert!(result.is_err());

        // Use of the ALL keyword not supported with offline signing
        config.sign_only = true;
        let result = process_test_command(
            &config,
            payer,
            &[
                "spl-token",
                CommandName::Burn.into(),
                &source.to_string(),
                "ALL",
            ],
        )
        .await;
        assert!(result.is_err_and(|err| err.to_string().contains("ALL")));
    }
}

async fn burn_with_account_delegate(test_validator: &TestValidator, payer: &Keypair) {
    for program_id in VALID_TOKEN_PROGRAM_IDS.iter() {
        let config = test_config_with_default_signer(test_validator, payer, program_id);

        let token = create_token(&config, payer).await;
        let source = create_associated_account(&config, payer, &token, &payer.pubkey()).await;
        let delegate = Keypair::new();

        let delegate_keypair_file = NamedTempFile::new().unwrap();
        write_keypair_file(&delegate, &delegate_keypair_file).unwrap();
        let fee_payer_keypair_file = NamedTempFile::new().unwrap();
        write_keypair_file(payer, &fee_payer_keypair_file).unwrap();

        let ui_amount = 100.0;
        mint_tokens(&config, payer, token, ui_amount, source)
            .await
            .unwrap();

        let ui_account = config
            .rpc_client
            .get_token_account(&source)
            .await
            .unwrap()
            .unwrap();
        let amount = spl_token::ui_amount_to_amount(100.0, TEST_DECIMALS);
        assert_eq!(ui_account.token_amount.amount, format!("{amount}"));
        assert_eq!(ui_account.delegate, None);
        assert_eq!(ui_account.delegated_amount, None);

        exec_test_cmd(
            &config,
            &[
                "spl-token",
                CommandName::Approve.into(),
                &source.to_string(),
                "10",
                &delegate.pubkey().to_string(),
                "--owner",
                fee_payer_keypair_file.path().to_str().unwrap(),
                "--fee-payer",
                fee_payer_keypair_file.path().to_str().unwrap(),
                "--program-id",
                &program_id.to_string(),
            ],
        )
        .await
        .unwrap();

        let ui_account = config
            .rpc_client
            .get_token_account(&source)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ui_account.delegate.unwrap(), delegate.pubkey().to_string());
        let amount = spl_token::ui_amount_to_amount(10.0, TEST_DECIMALS);
        assert_eq!(
            ui_account.delegated_amount.unwrap().amount,
            format!("{amount}")
        );

        let result = exec_test_cmd(
            &config,
            &[
                "spl-token",
                CommandName::Burn.into(),
                &source.to_string(),
                "10",
                "--owner",
                delegate_keypair_file.path().to_str().unwrap(),
                "--fee-payer",
                fee_payer_keypair_file.path().to_str().unwrap(),
                "--program-id",
                &program_id.to_string(),
            ],
        )
        .await;
        result.unwrap();

        let ui_account = config
            .rpc_client
            .get_token_account(&source)
            .await
            .unwrap()
            .unwrap();
        let amount = spl_token::ui_amount_to_amount(90.0, TEST_DECIMALS);
        assert_eq!(ui_account.token_amount.amount, format!("{amount}"));
        assert_eq!(ui_account.delegate, None);
        assert_eq!(ui_account.delegated_amount, None);
    }
}

async fn burn_with_permanent_delegate(test_validator: &TestValidator, payer: &Keypair) {
    let config = test_config_with_default_signer(test_validator, payer, &spl_token_2022::id());

    let token = Keypair::new();
    let token_keypair_file = NamedTempFile::new().unwrap();
    write_keypair_file(&token, &token_keypair_file).unwrap();
    let token = token.pubkey();
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::CreateToken.into(),
            token_keypair_file.path().to_str().unwrap(),
            "--enable-permanent-delegate",
        ],
    )
    .await
    .unwrap();

    let permanent_delegate_keypair_file = NamedTempFile::new().unwrap();
    write_keypair_file(payer, &permanent_delegate_keypair_file).unwrap();

    let unknown_owner = Keypair::new();
    let source =
        create_associated_account(&config, &unknown_owner, &token, &unknown_owner.pubkey()).await;
    let ui_amount = 100.0;

    mint_tokens(&config, payer, token, ui_amount, source)
        .await
        .unwrap();

    let ui_account = config
        .rpc_client
        .get_token_account(&source)
        .await
        .unwrap()
        .unwrap();

    let amount = spl_token::ui_amount_to_amount(100.0, TEST_DECIMALS);
    assert_eq!(ui_account.token_amount.amount, format!("{amount}"));

    exec_test_cmd(
        &config,
        &[
            "spl-token",
            CommandName::Burn.into(),
            &source.to_string(),
            "10",
            "--owner",
            permanent_delegate_keypair_file.path().to_str().unwrap(),
        ],
    )
    .await
    .unwrap();

    let ui_account = config
        .rpc_client
        .get_token_account(&source)
        .await
        .unwrap()
        .unwrap();

    let amount = spl_token::ui_amount_to_amount(90.0, TEST_DECIMALS);
    assert_eq!(ui_account.token_amount.amount, format!("{amount}"));
}

async fn transfer_with_permanent_delegate(test_validator: &TestValidator, payer: &Keypair) {
    let config = test_config_with_default_signer(test_validator, payer, &spl_token_2022::id());

    let token = Keypair::new();
    let token_keypair_file = NamedTempFile::new().unwrap();
    write_keypair_file(&token, &token_keypair_file).unwrap();
    let token = token.pubkey();
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::CreateToken.into(),
            token_keypair_file.path().to_str().unwrap(),
            "--enable-permanent-delegate",
        ],
    )
    .await
    .unwrap();

    let unknown_owner = Keypair::new();
    let source =
        create_associated_account(&config, &unknown_owner, &token, &unknown_owner.pubkey()).await;
    let destination = create_associated_account(&config, payer, &token, &payer.pubkey()).await;

    let permanent_delegate_keypair_file = NamedTempFile::new().unwrap();
    write_keypair_file(payer, &permanent_delegate_keypair_file).unwrap();

    let ui_amount = 100.0;
    mint_tokens(&config, payer, token, ui_amount, source)
        .await
        .unwrap();

    let ui_account = config
        .rpc_client
        .get_token_account(&source)
        .await
        .unwrap()
        .unwrap();

    let amount = spl_token::ui_amount_to_amount(100.0, TEST_DECIMALS);
    assert_eq!(ui_account.token_amount.amount, format!("{amount}"));

    let ui_account = config
        .rpc_client
        .get_token_account(&destination)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(ui_account.token_amount.amount, "0");

    exec_test_cmd(
        &config,
        &[
            "spl-token",
            CommandName::Transfer.into(),
            &token.to_string(),
            "50",
            &destination.to_string(),
            "--from",
            &source.to_string(),
            "--owner",
            permanent_delegate_keypair_file.path().to_str().unwrap(),
        ],
    )
    .await
    .unwrap();

    let ui_account = config
        .rpc_client
        .get_token_account(&destination)
        .await
        .unwrap()
        .unwrap();

    let amount = spl_token::ui_amount_to_amount(50.0, TEST_DECIMALS);
    assert_eq!(ui_account.token_amount.amount, format!("{amount}"));

    let ui_account = config
        .rpc_client
        .get_token_account(&source)
        .await
        .unwrap()
        .unwrap();

    let amount = spl_token::ui_amount_to_amount(50.0, TEST_DECIMALS);
    assert_eq!(ui_account.token_amount.amount, format!("{amount}"));
}

async fn close_mint(test_validator: &TestValidator, payer: &Keypair) {
    let config = test_config_with_default_signer(test_validator, payer, &spl_token_2022::id());

    let token = Keypair::new();
    let token_pubkey = token.pubkey();
    let token_keypair_file = NamedTempFile::new().unwrap();
    write_keypair_file(&token, &token_keypair_file).unwrap();
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::CreateToken.into(),
            token_keypair_file.path().to_str().unwrap(),
            "--enable-close",
        ],
    )
    .await
    .unwrap();

    let account = config.rpc_client.get_account(&token_pubkey).await.unwrap();
    let test_mint = StateWithExtensionsOwned::<Mint>::unpack(account.data);
    assert!(test_mint.is_ok());

    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::CloseMint.into(),
            &token_pubkey.to_string(),
        ],
    )
    .await
    .unwrap();

    let account = config.rpc_client.get_account(&token_pubkey).await;
    assert!(account.is_err());
}

async fn required_transfer_memos(test_validator: &TestValidator, payer: &Keypair) {
    let program_id = spl_token_2022::id();
    let config = test_config_with_default_signer(test_validator, payer, &program_id);
    let token = create_token(&config, payer).await;
    let destination_account = create_auxiliary_account(&config, payer, token).await;
    let token_account = create_associated_account(&config, payer, &token, &payer.pubkey()).await;

    mint_tokens(&config, payer, token, 100.0, token_account)
        .await
        .unwrap();

    // enable works
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::EnableRequiredTransferMemos.into(),
            &destination_account.to_string(),
        ],
    )
    .await
    .unwrap();

    let extensions = StateWithExtensionsOwned::<Account>::unpack(
        config
            .rpc_client
            .get_account(&destination_account)
            .await
            .unwrap()
            .data,
    )
    .unwrap();
    let memo_transfer = extensions.get_extension::<MemoTransfer>().unwrap();
    let enabled: bool = memo_transfer.require_incoming_transfer_memos.into();
    assert!(enabled);

    // transfer requires a memo
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::Transfer.into(),
            "--from",
            &token_account.to_string(),
            &token.to_string(),
            "1",
            &destination_account.to_string(),
        ],
    )
    .await
    .unwrap_err();
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::Transfer.into(),
            "--from",
            &token_account.to_string(),
            // malicious compliance
            "--with-memo",
            "memo",
            &token.to_string(),
            "1",
            &destination_account.to_string(),
        ],
    )
    .await
    .unwrap();
    let account_data = config
        .rpc_client
        .get_account(&destination_account)
        .await
        .unwrap();
    let account_state = StateWithExtensionsOwned::<Account>::unpack(account_data.data).unwrap();
    assert_eq!(
        account_state.base.amount,
        spl_token::ui_amount_to_amount(1.0, TEST_DECIMALS)
    );

    // disable works
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::DisableRequiredTransferMemos.into(),
            &destination_account.to_string(),
        ],
    )
    .await
    .unwrap();
    let extensions = StateWithExtensionsOwned::<Account>::unpack(
        config
            .rpc_client
            .get_account(&destination_account)
            .await
            .unwrap()
            .data,
    )
    .unwrap();
    let memo_transfer = extensions.get_extension::<MemoTransfer>().unwrap();
    let enabled: bool = memo_transfer.require_incoming_transfer_memos.into();
    assert!(!enabled);
}

async fn cpi_guard(test_validator: &TestValidator, payer: &Keypair) {
    let program_id = spl_token_2022::id();
    let config = test_config_with_default_signer(test_validator, payer, &program_id);
    let token = create_token(&config, payer).await;
    let token_account = create_associated_account(&config, payer, &token, &payer.pubkey()).await;

    // enable works
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::EnableCpiGuard.into(),
            &token_account.to_string(),
        ],
    )
    .await
    .unwrap();
    let extensions = StateWithExtensionsOwned::<Account>::unpack(
        config
            .rpc_client
            .get_account(&token_account)
            .await
            .unwrap()
            .data,
    )
    .unwrap();
    let cpi_guard = extensions.get_extension::<CpiGuard>().unwrap();
    let enabled: bool = cpi_guard.lock_cpi.into();
    assert!(enabled);

    // disable works
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::DisableCpiGuard.into(),
            &token_account.to_string(),
        ],
    )
    .await
    .unwrap();
    let extensions = StateWithExtensionsOwned::<Account>::unpack(
        config
            .rpc_client
            .get_account(&token_account)
            .await
            .unwrap()
            .data,
    )
    .unwrap();
    let cpi_guard = extensions.get_extension::<CpiGuard>().unwrap();
    let enabled: bool = cpi_guard.lock_cpi.into();
    assert!(!enabled);
}

async fn immutable_accounts(test_validator: &TestValidator, payer: &Keypair) {
    let program_id = spl_token_2022::id();
    let config = test_config_with_default_signer(test_validator, payer, &program_id);
    let token = create_token(&config, payer).await;
    let new_owner = Keypair::new().pubkey();
    let native_mint = *Token::new_native(
        config.program_client.clone(),
        &program_id,
        config.fee_payer().unwrap().clone(),
    )
    .get_address();

    // cannot reassign an ata
    let account = create_associated_account(&config, payer, &token, &payer.pubkey()).await;
    let result = process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::Authorize.into(),
            &account.to_string(),
            "owner",
            &new_owner.to_string(),
        ],
    )
    .await;
    result.unwrap_err();

    // immutable works for create-account
    let aux_account = Keypair::new();
    let aux_pubkey = aux_account.pubkey();
    let aux_keypair_file = NamedTempFile::new().unwrap();
    write_keypair_file(&aux_account, &aux_keypair_file).unwrap();

    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::CreateAccount.into(),
            &token.to_string(),
            aux_keypair_file.path().to_str().unwrap(),
            "--immutable",
        ],
    )
    .await
    .unwrap();

    let result = process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::Authorize.into(),
            &aux_pubkey.to_string(),
            "owner",
            &new_owner.to_string(),
        ],
    )
    .await;
    result.unwrap_err();

    // immutable works for wrap
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::Wrap.into(),
            "--create-aux-account",
            "--immutable",
            "0.5",
        ],
    )
    .await
    .unwrap();

    let accounts = config
        .rpc_client
        .get_token_accounts_by_owner(&payer.pubkey(), TokenAccountsFilter::Mint(native_mint))
        .await
        .unwrap();

    let result = process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::Authorize.into(),
            &accounts[0].pubkey,
            "owner",
            &new_owner.to_string(),
        ],
    )
    .await;
    result.unwrap_err();
}

async fn non_transferable(test_validator: &TestValidator, payer: &Keypair) {
    let config = test_config_with_default_signer(test_validator, payer, &spl_token_2022::id());

    let token = Keypair::new();
    let token_keypair_file = NamedTempFile::new().unwrap();
    write_keypair_file(&token, &token_keypair_file).unwrap();
    let token_pubkey = token.pubkey();
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::CreateToken.into(),
            token_keypair_file.path().to_str().unwrap(),
            "--enable-non-transferable",
        ],
    )
    .await
    .unwrap();

    let account = config.rpc_client.get_account(&token_pubkey).await.unwrap();
    let test_mint = StateWithExtensionsOwned::<Mint>::unpack(account.data).unwrap();
    assert!(test_mint.get_extension::<NonTransferable>().is_ok());

    let associated_account =
        create_associated_account(&config, payer, &token_pubkey, &payer.pubkey()).await;
    let aux_account = create_auxiliary_account(&config, payer, token_pubkey).await;
    mint_tokens(&config, payer, token_pubkey, 100.0, associated_account)
        .await
        .unwrap();

    // transfer not allowed
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::Transfer.into(),
            "--from",
            &associated_account.to_string(),
            &token_pubkey.to_string(),
            "1",
            &aux_account.to_string(),
        ],
    )
    .await
    .unwrap_err();
}

async fn default_account_state(test_validator: &TestValidator, payer: &Keypair) {
    let program_id = spl_token_2022::id();
    let config = test_config_with_default_signer(test_validator, payer, &program_id);

    let token = Keypair::new();
    let token_keypair_file = NamedTempFile::new().unwrap();
    write_keypair_file(&token, &token_keypair_file).unwrap();
    let token_pubkey = token.pubkey();
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::CreateToken.into(),
            token_keypair_file.path().to_str().unwrap(),
            "--enable-freeze",
            "--default-account-state",
            "frozen",
        ],
    )
    .await
    .unwrap();

    let mint_account = config.rpc_client.get_account(&token_pubkey).await.unwrap();
    let mint = StateWithExtensionsOwned::<Mint>::unpack(mint_account.data).unwrap();
    let extension = mint.get_extension::<DefaultAccountState>().unwrap();
    assert_eq!(extension.state, u8::from(AccountState::Frozen));

    let frozen_account =
        create_associated_account(&config, payer, &token_pubkey, &payer.pubkey()).await;
    let token_account = config
        .rpc_client
        .get_account(&frozen_account)
        .await
        .unwrap();
    let account = StateWithExtensionsOwned::<Account>::unpack(token_account.data).unwrap();
    assert_eq!(account.base.state, AccountState::Frozen);

    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::UpdateDefaultAccountState.into(),
            &token_pubkey.to_string(),
            "initialized",
        ],
    )
    .await
    .unwrap();
    let unfrozen_account = create_auxiliary_account(&config, payer, token_pubkey).await;
    let token_account = config
        .rpc_client
        .get_account(&unfrozen_account)
        .await
        .unwrap();
    let account = StateWithExtensionsOwned::<Account>::unpack(token_account.data).unwrap();
    assert_eq!(account.base.state, AccountState::Initialized);
}

async fn transfer_fee(test_validator: &TestValidator, payer: &Keypair) {
    let config = test_config_with_default_signer(test_validator, payer, &spl_token_2022::id());

    let transfer_fee_basis_points = 100;
    let maximum_fee = 10_000_000_000;

    let token = Keypair::new();
    let token_keypair_file = NamedTempFile::new().unwrap();
    write_keypair_file(&token, &token_keypair_file).unwrap();
    let token_pubkey = token.pubkey();
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::CreateToken.into(),
            token_keypair_file.path().to_str().unwrap(),
            "--transfer-fee",
            &transfer_fee_basis_points.to_string(),
            &maximum_fee.to_string(),
        ],
    )
    .await
    .unwrap();

    let account = config.rpc_client.get_account(&token_pubkey).await.unwrap();
    let test_mint = StateWithExtensionsOwned::<Mint>::unpack(account.data).unwrap();
    let extension = test_mint.get_extension::<TransferFeeConfig>().unwrap();
    assert_eq!(
        u16::from(extension.older_transfer_fee.transfer_fee_basis_points),
        transfer_fee_basis_points
    );
    assert_eq!(
        u64::from(extension.older_transfer_fee.maximum_fee),
        maximum_fee
    );
    assert_eq!(
        u16::from(extension.newer_transfer_fee.transfer_fee_basis_points),
        transfer_fee_basis_points
    );
    assert_eq!(
        u64::from(extension.newer_transfer_fee.maximum_fee),
        maximum_fee
    );

    let total_amount = 1000.0;
    let transfer_amount = 100.0;
    let token_account =
        create_associated_account(&config, payer, &token_pubkey, &payer.pubkey()).await;
    let source_account = create_auxiliary_account(&config, payer, token_pubkey).await;
    mint_tokens(&config, payer, token_pubkey, total_amount, source_account)
        .await
        .unwrap();

    // withdraw from account directly
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::Transfer.into(),
            "--from",
            &source_account.to_string(),
            &token_pubkey.to_string(),
            &transfer_amount.to_string(),
            &token_account.to_string(),
            "--expected-fee",
            "1",
        ],
    )
    .await
    .unwrap();

    let account = config.rpc_client.get_account(&token_account).await.unwrap();
    let account_state = StateWithExtensionsOwned::<Account>::unpack(account.data).unwrap();
    let extension = account_state.get_extension::<TransferFeeAmount>().unwrap();
    let withheld_amount =
        spl_token::amount_to_ui_amount(u64::from(extension.withheld_amount), TEST_DECIMALS);
    assert_eq!(withheld_amount, 1.0);

    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::WithdrawWithheldTokens.into(),
            &token_account.to_string(),
            &token_account.to_string(),
        ],
    )
    .await
    .unwrap();

    let account = config.rpc_client.get_account(&token_account).await.unwrap();
    let account_state = StateWithExtensionsOwned::<Account>::unpack(account.data).unwrap();
    let extension = account_state.get_extension::<TransferFeeAmount>().unwrap();
    assert_eq!(u64::from(extension.withheld_amount), 0);

    // withdraw from mint after account closure
    // gather fees
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::Transfer.into(),
            "--from",
            &source_account.to_string(),
            &token_pubkey.to_string(),
            &(total_amount - transfer_amount).to_string(),
            &token_account.to_string(),
            "--expected-fee",
            "9",
        ],
    )
    .await
    .unwrap();

    // burn tokens
    let account = config.rpc_client.get_account(&token_account).await.unwrap();
    let account_state = StateWithExtensionsOwned::<Account>::unpack(account.data).unwrap();
    let burn_amount = spl_token::amount_to_ui_amount(account_state.base.amount, TEST_DECIMALS);
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::Burn.into(),
            &token_account.to_string(),
            &burn_amount.to_string(),
        ],
    )
    .await
    .unwrap();

    let account = config.rpc_client.get_account(&token_account).await.unwrap();
    let account_state = StateWithExtensionsOwned::<Account>::unpack(account.data).unwrap();
    let extension = account_state.get_extension::<TransferFeeAmount>().unwrap();
    let withheld_amount =
        spl_token::amount_to_ui_amount(u64::from(extension.withheld_amount), TEST_DECIMALS);
    assert_eq!(withheld_amount, 9.0);

    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::Close.into(),
            "--address",
            &token_account.to_string(),
            "--recipient",
            &payer.pubkey().to_string(),
        ],
    )
    .await
    .unwrap();

    let mint = config.rpc_client.get_account(&token_pubkey).await.unwrap();
    let mint_state = StateWithExtensionsOwned::<Mint>::unpack(mint.data).unwrap();
    let extension = mint_state.get_extension::<TransferFeeConfig>().unwrap();
    let withheld_amount =
        spl_token::amount_to_ui_amount(u64::from(extension.withheld_amount), TEST_DECIMALS);
    assert_eq!(withheld_amount, 9.0);

    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::WithdrawWithheldTokens.into(),
            &source_account.to_string(),
            "--include-mint",
        ],
    )
    .await
    .unwrap();

    let mint = config.rpc_client.get_account(&token_pubkey).await.unwrap();
    let mint_state = StateWithExtensionsOwned::<Mint>::unpack(mint.data).unwrap();
    let extension = mint_state.get_extension::<TransferFeeConfig>().unwrap();
    assert_eq!(u64::from(extension.withheld_amount), 0);

    // set the transfer fee
    let new_transfer_fee_basis_points = 800;
    let new_maximum_fee = 5_000_000.0;
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::SetTransferFee.into(),
            &token_pubkey.to_string(),
            &new_transfer_fee_basis_points.to_string(),
            &new_maximum_fee.to_string(),
        ],
    )
    .await
    .unwrap();

    let mint = config.rpc_client.get_account(&token_pubkey).await.unwrap();
    let mint_state = StateWithExtensionsOwned::<Mint>::unpack(mint.data).unwrap();
    let extension = mint_state.get_extension::<TransferFeeConfig>().unwrap();
    assert_eq!(
        u16::from(extension.newer_transfer_fee.transfer_fee_basis_points),
        new_transfer_fee_basis_points
    );
    let new_maximum_fee = spl_token::ui_amount_to_amount(new_maximum_fee, TEST_DECIMALS);
    assert_eq!(
        u64::from(extension.newer_transfer_fee.maximum_fee),
        new_maximum_fee
    );

    // disable transfer fee authority
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::Authorize.into(),
            "--disable",
            &token_pubkey.to_string(),
            "transfer-fee-config",
        ],
    )
    .await
    .unwrap();

    let mint = config.rpc_client.get_account(&token_pubkey).await.unwrap();
    let mint_state = StateWithExtensionsOwned::<Mint>::unpack(mint.data).unwrap();
    let extension = mint_state.get_extension::<TransferFeeConfig>().unwrap();

    assert_eq!(
        Option::<Pubkey>::from(extension.transfer_fee_config_authority),
        None,
    );

    // disable withdraw withheld authority
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::Authorize.into(),
            "--disable",
            &token_pubkey.to_string(),
            "withheld-withdraw",
        ],
    )
    .await
    .unwrap();

    let mint = config.rpc_client.get_account(&token_pubkey).await.unwrap();
    let mint_state = StateWithExtensionsOwned::<Mint>::unpack(mint.data).unwrap();
    let extension = mint_state.get_extension::<TransferFeeConfig>().unwrap();

    assert_eq!(
        Option::<Pubkey>::from(extension.withdraw_withheld_authority),
        None,
    );
}

async fn transfer_fee_basis_point(test_validator: &TestValidator, payer: &Keypair) {
    let config = test_config_with_default_signer(test_validator, payer, &spl_token_2022::id());

    let transfer_fee_basis_points = 100;
    let maximum_fee = 1.2;
    let decimal = 9;

    let token = Keypair::new();
    let token_keypair_file = NamedTempFile::new().unwrap();
    write_keypair_file(&token, &token_keypair_file).unwrap();
    let token_pubkey = token.pubkey();
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::CreateToken.into(),
            token_keypair_file.path().to_str().unwrap(),
            "--transfer-fee-basis-points",
            &transfer_fee_basis_points.to_string(),
            "--transfer-fee-maximum-fee",
            &maximum_fee.to_string(),
        ],
    )
    .await
    .unwrap();

    let account = config.rpc_client.get_account(&token_pubkey).await.unwrap();
    let test_mint = StateWithExtensionsOwned::<Mint>::unpack(account.data).unwrap();
    let extension = test_mint.get_extension::<TransferFeeConfig>().unwrap();
    assert_eq!(
        u16::from(extension.older_transfer_fee.transfer_fee_basis_points),
        transfer_fee_basis_points
    );
    assert_eq!(
        u64::from(extension.older_transfer_fee.maximum_fee),
        (maximum_fee * i32::pow(10, decimal) as f64) as u64
    );
    assert_eq!(
        u16::from(extension.newer_transfer_fee.transfer_fee_basis_points),
        transfer_fee_basis_points
    );
    assert_eq!(
        u64::from(extension.newer_transfer_fee.maximum_fee),
        (maximum_fee * i32::pow(10, decimal) as f64) as u64
    );
}

async fn confidential_transfer(test_validator: &TestValidator, payer: &Keypair) {
    use spl_token_2022::solana_zk_sdk::encryption::elgamal::ElGamalKeypair;

    let config = test_config_with_default_signer(test_validator, payer, &spl_token_2022::id());

    // create token with confidential transfers enabled
    let auto_approve = false;
    let confidential_transfer_mint_authority = payer.pubkey();

    let token = Keypair::new();
    let token_keypair_file = NamedTempFile::new().unwrap();
    write_keypair_file(&token, &token_keypair_file).unwrap();
    let token_pubkey = token.pubkey();
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::CreateToken.into(),
            token_keypair_file.path().to_str().unwrap(),
            "--enable-confidential-transfers",
            "manual",
        ],
    )
    .await
    .unwrap();

    let account = config.rpc_client.get_account(&token_pubkey).await.unwrap();
    let test_mint = StateWithExtensionsOwned::<Mint>::unpack(account.data).unwrap();
    let extension = test_mint
        .get_extension::<ConfidentialTransferMint>()
        .unwrap();

    assert_eq!(
        Option::<Pubkey>::from(extension.authority),
        Some(confidential_transfer_mint_authority),
    );
    assert_eq!(
        bool::from(extension.auto_approve_new_accounts),
        auto_approve,
    );
    assert_eq!(
        Option::<PodElGamalPubkey>::from(extension.auditor_elgamal_pubkey),
        None,
    );

    // update confidential transfer mint settings
    let auditor_keypair = ElGamalKeypair::new_rand();
    let auditor_pubkey: PodElGamalPubkey = (*auditor_keypair.pubkey()).into();
    let new_auto_approve = true;

    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::UpdateConfidentialTransferSettings.into(),
            &token_pubkey.to_string(),
            "--auditor-pubkey",
            &auditor_pubkey.to_string(),
            "--approve-policy",
            "auto",
        ],
    )
    .await
    .unwrap();

    let account = config.rpc_client.get_account(&token_pubkey).await.unwrap();
    let test_mint = StateWithExtensionsOwned::<Mint>::unpack(account.data).unwrap();
    let extension = test_mint
        .get_extension::<ConfidentialTransferMint>()
        .unwrap();

    assert_eq!(
        bool::from(extension.auto_approve_new_accounts),
        new_auto_approve,
    );
    assert_eq!(
        Option::<PodElGamalPubkey>::from(extension.auditor_elgamal_pubkey),
        Some(auditor_pubkey),
    );

    // create a confidential transfer account
    let token_account =
        create_associated_account(&config, payer, &token_pubkey, &payer.pubkey()).await;

    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::ConfigureConfidentialTransferAccount.into(),
            &token_pubkey.to_string(),
        ],
    )
    .await
    .unwrap();

    let account = config.rpc_client.get_account(&token_account).await.unwrap();
    let account_state = StateWithExtensionsOwned::<Account>::unpack(account.data).unwrap();
    let extension = account_state
        .get_extension::<ConfidentialTransferAccount>()
        .unwrap();
    assert!(bool::from(extension.approved));
    assert!(bool::from(extension.allow_confidential_credits));
    assert!(bool::from(extension.allow_non_confidential_credits));

    // disable and enable confidential transfers for an account
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::DisableConfidentialCredits.into(),
            &token_pubkey.to_string(),
        ],
    )
    .await
    .unwrap();

    let account = config.rpc_client.get_account(&token_account).await.unwrap();
    let account_state = StateWithExtensionsOwned::<Account>::unpack(account.data).unwrap();
    let extension = account_state
        .get_extension::<ConfidentialTransferAccount>()
        .unwrap();
    assert!(!bool::from(extension.allow_confidential_credits));

    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::EnableConfidentialCredits.into(),
            &token_pubkey.to_string(),
        ],
    )
    .await
    .unwrap();

    let account = config.rpc_client.get_account(&token_account).await.unwrap();
    let account_state = StateWithExtensionsOwned::<Account>::unpack(account.data).unwrap();
    let extension = account_state
        .get_extension::<ConfidentialTransferAccount>()
        .unwrap();
    assert!(bool::from(extension.allow_confidential_credits));

    // disable and enable non-confidential transfers for an account
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::DisableNonConfidentialCredits.into(),
            &token_pubkey.to_string(),
        ],
    )
    .await
    .unwrap();

    let account = config.rpc_client.get_account(&token_account).await.unwrap();
    let account_state = StateWithExtensionsOwned::<Account>::unpack(account.data).unwrap();
    let extension = account_state
        .get_extension::<ConfidentialTransferAccount>()
        .unwrap();
    assert!(!bool::from(extension.allow_non_confidential_credits));

    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::EnableNonConfidentialCredits.into(),
            &token_pubkey.to_string(),
        ],
    )
    .await
    .unwrap();

    let account = config.rpc_client.get_account(&token_account).await.unwrap();
    let account_state = StateWithExtensionsOwned::<Account>::unpack(account.data).unwrap();
    let extension = account_state
        .get_extension::<ConfidentialTransferAccount>()
        .unwrap();
    assert!(bool::from(extension.allow_non_confidential_credits));

    // deposit confidential tokens
    let deposit_amount = 100.0;
    mint_tokens(&config, payer, token_pubkey, deposit_amount, token_account)
        .await
        .unwrap();

    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::DepositConfidentialTokens.into(),
            &token_pubkey.to_string(),
            &deposit_amount.to_string(),
        ],
    )
    .await
    .unwrap();

    // apply pending balance
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::ApplyPendingBalance.into(),
            &token_pubkey.to_string(),
        ],
    )
    .await
    .unwrap();

    // confidential transfer
    let destination_account = create_auxiliary_account(&config, payer, token_pubkey).await;
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::ConfigureConfidentialTransferAccount.into(),
            "--address",
            &destination_account.to_string(),
        ],
    )
    .await
    .unwrap(); // configure destination account for confidential transfers first

    let transfer_amount = 100.0;
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::Transfer.into(),
            &token_pubkey.to_string(),
            &transfer_amount.to_string(),
            &destination_account.to_string(),
            "--confidential",
        ],
    )
    .await
    .unwrap();

    // withdraw confidential tokens
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::ApplyPendingBalance.into(),
            "--address",
            &destination_account.to_string(),
        ],
    )
    .await
    .unwrap(); // apply pending balance first

    let withdraw_amount = 100.0;

    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::WithdrawConfidentialTokens.into(),
            &token_pubkey.to_string(),
            &withdraw_amount.to_string(),
            "--address",
            &destination_account.to_string(),
        ],
    )
    .await
    .unwrap();

    // disable confidential transfers for mint
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::Authorize.into(),
            &token_pubkey.to_string(),
            "confidential-transfer-mint",
            "--disable",
        ],
    )
    .await
    .unwrap();
}

async fn confidential_transfer_with_fee(test_validator: &TestValidator, payer: &Keypair) {
    let config = test_config_with_default_signer(test_validator, payer, &spl_token_2022::id());

    // create token with confidential transfers enabled
    let auto_approve = true;
    let confidential_transfer_mint_authority = payer.pubkey();

    let token = Keypair::new();
    let token_keypair_file = NamedTempFile::new().unwrap();
    write_keypair_file(&token, &token_keypair_file).unwrap();
    let token_pubkey = token.pubkey();
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::CreateToken.into(),
            token_keypair_file.path().to_str().unwrap(),
            "--enable-confidential-transfers",
            "auto",
            "--transfer-fee",
            "100",
            "1000000000",
        ],
    )
    .await
    .unwrap();

    let account = config.rpc_client.get_account(&token_pubkey).await.unwrap();
    let test_mint = StateWithExtensionsOwned::<Mint>::unpack(account.data).unwrap();
    let extension = test_mint
        .get_extension::<ConfidentialTransferMint>()
        .unwrap();

    assert_eq!(
        Option::<Pubkey>::from(extension.authority),
        Some(confidential_transfer_mint_authority),
    );
    assert_eq!(
        bool::from(extension.auto_approve_new_accounts),
        auto_approve,
    );
    assert_eq!(
        Option::<PodElGamalPubkey>::from(extension.auditor_elgamal_pubkey),
        None,
    );

    let extension = test_mint
        .get_extension::<ConfidentialTransferFeeConfig>()
        .unwrap();
    assert_eq!(
        Option::<Pubkey>::from(extension.authority),
        Some(confidential_transfer_mint_authority),
    );
}

async fn multisig_transfer(test_validator: &TestValidator, payer: &Keypair) {
    let m = 3;
    let n = 5u8;
    // need to add "payer" to make the config provide the right signer
    let (multisig_members, multisig_paths): (Vec<_>, Vec<_>) =
        std::iter::once(clone_keypair(payer))
            .chain(std::iter::repeat_with(Keypair::new).take((n - 2) as usize))
            .map(|s| {
                let keypair_file = NamedTempFile::new().unwrap();
                write_keypair_file(&s, &keypair_file).unwrap();
                (s.pubkey(), keypair_file)
            })
            .unzip();
    for program_id in VALID_TOKEN_PROGRAM_IDS.iter() {
        let config = test_config_with_default_signer(test_validator, payer, program_id);
        let token = create_token(&config, payer).await;
        let multisig = Arc::new(Keypair::new());
        let multisig_pubkey = multisig.pubkey();

        // add the multisig as a member to itself, make it self-owned
        let multisig_members = std::iter::once(multisig_pubkey)
            .chain(multisig_members.iter().cloned())
            .collect::<Vec<_>>();
        let multisig_path = NamedTempFile::new().unwrap();
        write_keypair_file(&multisig, &multisig_path).unwrap();
        let multisig_paths = std::iter::once(&multisig_path)
            .chain(multisig_paths.iter())
            .collect::<Vec<_>>();

        let multisig_strings = multisig_members
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>();
        process_test_command(
            &config,
            payer,
            [
                "spl-token",
                CommandName::CreateMultisig.into(),
                "--address-keypair",
                multisig_path.path().to_str().unwrap(),
                "--program-id",
                &program_id.to_string(),
                &m.to_string(),
            ]
            .into_iter()
            .chain(multisig_strings.iter().map(|p| p.as_str())),
        )
        .await
        .unwrap();

        let account = config
            .rpc_client
            .get_account(&multisig_pubkey)
            .await
            .unwrap();
        let multisig = Multisig::unpack(&account.data).unwrap();
        assert_eq!(multisig.m, m);
        assert_eq!(multisig.n, n);

        let source = create_associated_account(&config, payer, &token, &multisig_pubkey).await;
        let destination = create_auxiliary_account(&config, payer, token).await;
        let ui_amount = 100.0;
        mint_tokens(&config, payer, token, ui_amount, source)
            .await
            .unwrap();

        exec_test_cmd(
            &config,
            &[
                "spl-token",
                CommandName::Transfer.into(),
                &token.to_string(),
                "10",
                &destination.to_string(),
                "--multisig-signer",
                multisig_paths[0].path().to_str().unwrap(),
                "--multisig-signer",
                multisig_paths[1].path().to_str().unwrap(),
                "--multisig-signer",
                multisig_paths[2].path().to_str().unwrap(),
                "--from",
                &source.to_string(),
                "--owner",
                &multisig_pubkey.to_string(),
                "--fee-payer",
                multisig_paths[1].path().to_str().unwrap(),
            ],
        )
        .await
        .unwrap();

        let account = config.rpc_client.get_account(&source).await.unwrap();
        let token_account = StateWithExtensionsOwned::<Account>::unpack(account.data).unwrap();
        assert_eq!(
            token_account.base.amount,
            spl_token::ui_amount_to_amount(90.0, TEST_DECIMALS)
        );
        let account = config.rpc_client.get_account(&destination).await.unwrap();
        let token_account = StateWithExtensionsOwned::<Account>::unpack(account.data).unwrap();
        assert_eq!(
            token_account.base.amount,
            spl_token::ui_amount_to_amount(10.0, TEST_DECIMALS)
        );
    }
}

async fn do_offline_multisig_transfer(
    test_validator: &TestValidator,
    payer: &Keypair,
    compute_unit_price: Option<u64>,
) {
    let m = 2;
    let n = 3u8;

    let fee_payer_keypair_file = NamedTempFile::new().unwrap();
    write_keypair_file(payer, &fee_payer_keypair_file).unwrap();

    let (multisig_members, multisig_paths): (Vec<_>, Vec<_>) = std::iter::repeat_with(Keypair::new)
        .take(n as usize)
        .map(|s| {
            let keypair_file = NamedTempFile::new().unwrap();
            write_keypair_file(&s, &keypair_file).unwrap();
            (s.pubkey(), keypair_file)
        })
        .unzip();
    for program_id in VALID_TOKEN_PROGRAM_IDS.iter() {
        let mut config = test_config_with_default_signer(test_validator, payer, program_id);
        config.compute_unit_limit = ComputeUnitLimit::Default;
        let token = create_token(&config, payer).await;
        let nonce = create_nonce(&config, payer).await;

        let nonce_account = config.rpc_client.get_account(&nonce).await.unwrap();
        let start_hash_index = 4 + 4 + 32;
        let blockhash = Hash::new(&nonce_account.data[start_hash_index..start_hash_index + 32]);

        let multisig = Arc::new(Keypair::new());
        let multisig_pubkey = multisig.pubkey();
        let multisig_path = NamedTempFile::new().unwrap();
        write_keypair_file(&multisig, &multisig_path).unwrap();

        let multisig_strings = multisig_members
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>();
        process_test_command(
            &config,
            payer,
            [
                "spl-token",
                CommandName::CreateMultisig.into(),
                "--address-keypair",
                multisig_path.path().to_str().unwrap(),
                "--program-id",
                &program_id.to_string(),
                &m.to_string(),
            ]
            .into_iter()
            .chain(multisig_strings.iter().map(|p| p.as_str())),
        )
        .await
        .unwrap();

        let source = create_associated_account(&config, payer, &token, &multisig_pubkey).await;
        let destination = create_auxiliary_account(&config, payer, token).await;
        let ui_amount = 100.0;
        mint_tokens(&config, payer, token, ui_amount, source)
            .await
            .unwrap();

        let offline_program_client: Arc<dyn ProgramClient<ProgramRpcClientSendTransaction>> =
            Arc::new(ProgramOfflineClient::new(
                blockhash,
                ProgramRpcClientSendTransaction,
            ));
        let mut args = vec![
            "spl-token".to_string(),
            CommandName::Transfer.as_ref().to_string(),
            token.to_string(),
            "100".to_string(),
            destination.to_string(),
            "--blockhash".to_string(),
            blockhash.to_string(),
            "--nonce".to_string(),
            nonce.to_string(),
            "--nonce-authority".to_string(),
            payer.pubkey().to_string(),
            "--sign-only".to_string(),
            "--mint-decimals".to_string(),
            format!("{}", TEST_DECIMALS),
            "--multisig-signer".to_string(),
            multisig_paths[1].path().to_str().unwrap().to_string(),
            "--multisig-signer".to_string(),
            multisig_members[2].to_string(),
            "--from".to_string(),
            source.to_string(),
            "--owner".to_string(),
            multisig_pubkey.to_string(),
            "--fee-payer".to_string(),
            payer.pubkey().to_string(),
            "--program-id".to_string(),
            program_id.to_string(),
        ];
        if let Some(compute_unit_price) = compute_unit_price {
            args.push("--with-compute-unit-price".to_string());
            args.push(compute_unit_price.to_string());
            args.push("--with-compute-unit-limit".to_string());
            args.push(10_000.to_string());
        }
        config.program_client = offline_program_client;
        let result = exec_test_cmd(&config, &args).await.unwrap();
        // the provided signer has a signature, denoted by the pubkey followed
        // by "=" and the signature
        let member_prefix = format!("{}=", multisig_members[1]);
        let signature_position = result.find(&member_prefix).unwrap();
        let end_position = result[signature_position..].find('\n').unwrap();
        let signer = result[signature_position..].get(..end_position).unwrap();

        // other three expected signers are absent
        let absent_signers_position = result.find("Absent Signers").unwrap();
        let absent_signers = result.get(absent_signers_position..).unwrap();
        assert!(absent_signers.contains(&multisig_members[2].to_string()));
        assert!(absent_signers.contains(&payer.pubkey().to_string()));

        // and nothing else is marked a signer
        assert!(!absent_signers.contains(&multisig_members[0].to_string()));
        assert!(!absent_signers.contains(&multisig_pubkey.to_string()));
        assert!(!absent_signers.contains(&nonce.to_string()));
        assert!(!absent_signers.contains(&source.to_string()));
        assert!(!absent_signers.contains(&destination.to_string()));
        assert!(!absent_signers.contains(&token.to_string()));

        // now send the transaction
        let rpc_program_client: Arc<dyn ProgramClient<ProgramRpcClientSendTransaction>> = Arc::new(
            ProgramRpcClient::new(config.rpc_client.clone(), ProgramRpcClientSendTransaction),
        );
        config.program_client = rpc_program_client;
        let mut args = vec![
            "spl-token".to_string(),
            CommandName::Transfer.as_ref().to_string(),
            token.to_string(),
            "100".to_string(),
            destination.to_string(),
            "--blockhash".to_string(),
            blockhash.to_string(),
            "--nonce".to_string(),
            nonce.to_string(),
            "--nonce-authority".to_string(),
            fee_payer_keypair_file.path().to_str().unwrap().to_string(),
            "--mint-decimals".to_string(),
            format!("{}", TEST_DECIMALS),
            "--multisig-signer".to_string(),
            multisig_members[1].to_string(),
            "--multisig-signer".to_string(),
            multisig_paths[2].path().to_str().unwrap().to_string(),
            "--from".to_string(),
            source.to_string(),
            "--owner".to_string(),
            multisig_pubkey.to_string(),
            "--fee-payer".to_string(),
            fee_payer_keypair_file.path().to_str().unwrap().to_string(),
            "--program-id".to_string(),
            program_id.to_string(),
            "--signer".to_string(),
            signer.to_string(),
        ];
        if let Some(compute_unit_price) = compute_unit_price {
            args.push("--with-compute-unit-price".to_string());
            args.push(compute_unit_price.to_string());
            args.push("--with-compute-unit-limit".to_string());
            args.push(10_000.to_string());
        }
        exec_test_cmd(&config, &args).await.unwrap();

        let account = config.rpc_client.get_account(&source).await.unwrap();
        let token_account = StateWithExtensionsOwned::<Account>::unpack(account.data).unwrap();
        let amount = spl_token::ui_amount_to_amount(0.0, TEST_DECIMALS);
        assert_eq!(token_account.base.amount, amount);
        let account = config.rpc_client.get_account(&destination).await.unwrap();
        let token_account = StateWithExtensionsOwned::<Account>::unpack(account.data).unwrap();
        let amount = spl_token::ui_amount_to_amount(100.0, TEST_DECIMALS);
        assert_eq!(token_account.base.amount, amount);

        // get new nonce
        let nonce_account = config.rpc_client.get_account(&nonce).await.unwrap();
        let blockhash = Hash::new(&nonce_account.data[start_hash_index..start_hash_index + 32]);
        let mut args = vec![
            "spl-token".to_string(),
            CommandName::Close.as_ref().to_string(),
            "--address".to_string(),
            source.to_string(),
            "--blockhash".to_string(),
            blockhash.to_string(),
            "--nonce".to_string(),
            nonce.to_string(),
            "--nonce-authority".to_string(),
            fee_payer_keypair_file.path().to_str().unwrap().to_string(),
            "--multisig-signer".to_string(),
            multisig_paths[1].path().to_str().unwrap().to_string(),
            "--multisig-signer".to_string(),
            multisig_paths[2].path().to_str().unwrap().to_string(),
            "--owner".to_string(),
            multisig_pubkey.to_string(),
            "--fee-payer".to_string(),
            fee_payer_keypair_file.path().to_str().unwrap().to_string(),
            "--program-id".to_string(),
            program_id.to_string(),
        ];
        if let Some(compute_unit_price) = compute_unit_price {
            args.push("--with-compute-unit-price".to_string());
            args.push(compute_unit_price.to_string());
            args.push("--with-compute-unit-limit".to_string());
            args.push(10_000.to_string());
        }
        exec_test_cmd(&config, &args).await.unwrap();
        let _ = config.rpc_client.get_account(&source).await.unwrap_err();
    }
}

async fn offline_multisig_transfer_with_nonce(test_validator: &TestValidator, payer: &Keypair) {
    do_offline_multisig_transfer(test_validator, payer, None).await;
    do_offline_multisig_transfer(test_validator, payer, Some(10)).await;
}

async fn withdraw_excess_lamports_from_multisig(test_validator: &TestValidator, payer: &Keypair) {
    let m = 3;
    let n = 5u8;
    // need to add "payer" to make the config provide the right signer
    let (multisig_members, multisig_paths): (Vec<_>, Vec<_>) =
        std::iter::once(clone_keypair(payer))
            .chain(std::iter::repeat_with(Keypair::new).take((n - 2) as usize))
            .map(|s| {
                let keypair_file = NamedTempFile::new().unwrap();
                write_keypair_file(&s, &keypair_file).unwrap();
                (s.pubkey(), keypair_file)
            })
            .unzip();

    let fee_payer_keypair_file = NamedTempFile::new().unwrap();
    write_keypair_file(payer, &fee_payer_keypair_file).unwrap();

    let owner_keypair_file = NamedTempFile::new().unwrap();
    write_keypair_file(payer, &owner_keypair_file).unwrap();

    let program_id = &spl_token_2022::id();
    let config = test_config_with_default_signer(test_validator, payer, program_id);

    let multisig = Arc::new(Keypair::new());
    let multisig_pubkey = multisig.pubkey();

    // add the multisig as a member to itself, make it self-owned
    let multisig_members = std::iter::once(multisig_pubkey)
        .chain(multisig_members.iter().cloned())
        .collect::<Vec<_>>();
    let multisig_path = NamedTempFile::new().unwrap();
    write_keypair_file(&multisig, &multisig_path).unwrap();
    let multisig_paths = std::iter::once(&multisig_path)
        .chain(multisig_paths.iter())
        .collect::<Vec<_>>();

    let multisig_strings = multisig_members
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>();
    process_test_command(
        &config,
        payer,
        [
            "spl-token",
            CommandName::CreateMultisig.into(),
            "--address-keypair",
            multisig_path.path().to_str().unwrap(),
            "--program-id",
            &program_id.to_string(),
            &m.to_string(),
        ]
        .into_iter()
        .chain(multisig_strings.iter().map(|p| p.as_str())),
    )
    .await
    .unwrap();

    let account = config
        .rpc_client
        .get_account(&multisig_pubkey)
        .await
        .unwrap();
    let multisig = Multisig::unpack(&account.data).unwrap();
    assert_eq!(multisig.m, m);
    assert_eq!(multisig.n, n);

    let receiver = Keypair::new();
    let excess_lamports = 4000 * 1_000_000_000;

    config
        .rpc_client
        .send_and_confirm_transaction(&Transaction::new_signed_with_payer(
            &[system_instruction::transfer(
                &payer.pubkey(),
                &multisig_pubkey,
                excess_lamports,
            )],
            Some(&payer.pubkey()),
            &[&payer],
            config.rpc_client.get_latest_blockhash().await.unwrap(),
        ))
        .await
        .unwrap();

    exec_test_cmd(
        &config,
        &[
            "spl-token",
            CommandName::WithdrawExcessLamports.into(),
            &multisig_pubkey.to_string(),
            &receiver.pubkey().to_string(),
            "--owner",
            &multisig_pubkey.to_string(),
            "--multisig-signer",
            multisig_paths[0].path().to_str().unwrap(),
            "--multisig-signer",
            multisig_paths[1].path().to_str().unwrap(),
            "--multisig-signer",
            multisig_paths[2].path().to_str().unwrap(),
            "--fee-payer",
            fee_payer_keypair_file.path().to_str().unwrap(),
            "--program-id",
            &program_id.to_string(),
        ],
    )
    .await
    .unwrap();

    assert_eq!(
        excess_lamports,
        config
            .rpc_client
            .get_balance(&receiver.pubkey())
            .await
            .unwrap()
    );
}

async fn withdraw_excess_lamports_from_mint(test_validator: &TestValidator, payer: &Keypair) {
    let program_id = &spl_token_2022::id();
    let config = test_config_with_default_signer(test_validator, payer, program_id);
    let owner_keypair_file = NamedTempFile::new().unwrap();
    write_keypair_file(payer, &owner_keypair_file).unwrap();

    let receiver = Keypair::new();

    let token_keypair = Keypair::new();
    let token_path = NamedTempFile::new().unwrap();
    write_keypair_file(&token_keypair, &token_path).unwrap();
    let token_pubkey = token_keypair.pubkey();

    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::CreateToken.into(),
            token_path.path().to_str().unwrap(),
            "--program-id",
            &program_id.to_string(),
        ],
    )
    .await
    .unwrap();

    let excess_lamports = 4000 * 1_000_000_000;
    config
        .rpc_client
        .send_and_confirm_transaction(&Transaction::new_signed_with_payer(
            &[system_instruction::transfer(
                &payer.pubkey(),
                &token_pubkey,
                excess_lamports,
            )],
            Some(&payer.pubkey()),
            &[&payer],
            config.rpc_client.get_latest_blockhash().await.unwrap(),
        ))
        .await
        .unwrap();

    exec_test_cmd(
        &config,
        &[
            "spl-token",
            CommandName::WithdrawExcessLamports.into(),
            &token_pubkey.to_string(),
            &receiver.pubkey().to_string(),
            "--owner",
            owner_keypair_file.path().to_str().unwrap(),
            "--program-id",
            &program_id.to_string(),
        ],
    )
    .await
    .unwrap();

    assert_eq!(
        excess_lamports,
        config
            .rpc_client
            .get_balance(&receiver.pubkey())
            .await
            .unwrap()
    );
}

async fn withdraw_excess_lamports_from_account(test_validator: &TestValidator, payer: &Keypair) {
    let program_id = &spl_token_2022::id();
    let config = test_config_with_default_signer(test_validator, payer, program_id);
    let owner_keypair_file = NamedTempFile::new().unwrap();
    write_keypair_file(payer, &owner_keypair_file).unwrap();

    let receiver = Keypair::new();

    let token_keypair = Keypair::new();
    let token_path = NamedTempFile::new().unwrap();
    write_keypair_file(&token_keypair, &token_path).unwrap();
    let token_pubkey = token_keypair.pubkey();

    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::CreateToken.into(),
            token_path.path().to_str().unwrap(),
            "--program-id",
            &program_id.to_string(),
        ],
    )
    .await
    .unwrap();

    let excess_lamports = 4000 * 1_000_000_000;
    let token_account =
        create_associated_account(&config, payer, &token_pubkey, &payer.pubkey()).await;

    config
        .rpc_client
        .send_and_confirm_transaction(&Transaction::new_signed_with_payer(
            &[system_instruction::transfer(
                &payer.pubkey(),
                &token_account,
                excess_lamports,
            )],
            Some(&payer.pubkey()),
            &[&payer],
            config.rpc_client.get_latest_blockhash().await.unwrap(),
        ))
        .await
        .unwrap();

    exec_test_cmd(
        &config,
        &[
            "spl-token",
            CommandName::WithdrawExcessLamports.into(),
            &token_account.to_string(),
            &receiver.pubkey().to_string(),
            "--owner",
            owner_keypair_file.path().to_str().unwrap(),
            "--program-id",
            &program_id.to_string(),
        ],
    )
    .await
    .unwrap();

    assert_eq!(
        excess_lamports,
        config
            .rpc_client
            .get_balance(&receiver.pubkey())
            .await
            .unwrap()
    );
}

async fn metadata_pointer(test_validator: &TestValidator, payer: &Keypair) {
    let program_id = spl_token_2022::id();
    let config = test_config_with_default_signer(test_validator, payer, &program_id);
    let metadata_address = Pubkey::new_unique();

    let result = process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::CreateToken.into(),
            "--program-id",
            &program_id.to_string(),
            "--metadata-address",
            &metadata_address.to_string(),
        ],
    )
    .await;

    let value: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    let mint = Pubkey::from_str(value["commandOutput"]["address"].as_str().unwrap()).unwrap();
    let account = config.rpc_client.get_account(&mint).await.unwrap();
    let mint_state = StateWithExtensionsOwned::<Mint>::unpack(account.data).unwrap();

    let extension = mint_state.get_extension::<MetadataPointer>().unwrap();

    assert_eq!(
        extension.metadata_address,
        Some(metadata_address).try_into().unwrap()
    );

    let new_metadata_address = Pubkey::new_unique();

    let _new_result = process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::UpdateMetadataAddress.into(),
            &mint.to_string(),
            &new_metadata_address.to_string(),
        ],
    )
    .await;

    let new_account = config.rpc_client.get_account(&mint).await.unwrap();
    let new_mint_state = StateWithExtensionsOwned::<Mint>::unpack(new_account.data).unwrap();

    let new_extension = new_mint_state.get_extension::<MetadataPointer>().unwrap();

    assert_eq!(
        new_extension.metadata_address,
        Some(new_metadata_address).try_into().unwrap()
    );

    let _result_with_disable = process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::UpdateMetadataAddress.into(),
            &mint.to_string(),
            "--disable",
        ],
    )
    .await;

    let new_account_disable = config.rpc_client.get_account(&mint).await.unwrap();
    let new_mint_state_disable =
        StateWithExtensionsOwned::<Mint>::unpack(new_account_disable.data).unwrap();

    let new_extension_disable = new_mint_state_disable
        .get_extension::<MetadataPointer>()
        .unwrap();

    assert_eq!(
        new_extension_disable.metadata_address,
        None.try_into().unwrap()
    );
}

async fn group_pointer(test_validator: &TestValidator, payer: &Keypair) {
    let program_id = spl_token_2022::id();
    let config = test_config_with_default_signer(test_validator, payer, &program_id);
    let group_address = Pubkey::new_unique();

    let result = process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::CreateToken.into(),
            "--program-id",
            &program_id.to_string(),
            "--group-address",
            &group_address.to_string(),
        ],
    )
    .await
    .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    let mint = Pubkey::from_str(value["commandOutput"]["address"].as_str().unwrap()).unwrap();
    let account = config.rpc_client.get_account(&mint).await.unwrap();
    let mint_state = StateWithExtensionsOwned::<Mint>::unpack(account.data).unwrap();

    let extension = mint_state.get_extension::<GroupPointer>().unwrap();

    assert_eq!(
        extension.group_address,
        Some(group_address).try_into().unwrap()
    );

    let new_group_address = Pubkey::new_unique();

    let _new_result = process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::UpdateGroupAddress.into(),
            &mint.to_string(),
            &new_group_address.to_string(),
        ],
    )
    .await;

    let new_account = config.rpc_client.get_account(&mint).await.unwrap();
    let new_mint_state = StateWithExtensionsOwned::<Mint>::unpack(new_account.data).unwrap();

    let new_extension = new_mint_state.get_extension::<GroupPointer>().unwrap();

    assert_eq!(
        new_extension.group_address,
        Some(new_group_address).try_into().unwrap()
    );

    let _result_with_disable = process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::UpdateGroupAddress.into(),
            &mint.to_string(),
            "--disable",
        ],
    )
    .await
    .unwrap();

    let new_account_disable = config.rpc_client.get_account(&mint).await.unwrap();
    let new_mint_state_disable =
        StateWithExtensionsOwned::<Mint>::unpack(new_account_disable.data).unwrap();

    let new_extension_disable = new_mint_state_disable
        .get_extension::<GroupPointer>()
        .unwrap();

    assert_eq!(
        new_extension_disable.group_address,
        None.try_into().unwrap()
    );
}

async fn group_member_pointer(test_validator: &TestValidator, payer: &Keypair) {
    let program_id = spl_token_2022::id();
    let config = test_config_with_default_signer(test_validator, payer, &program_id);
    let member_address = Pubkey::new_unique();

    let result = process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::CreateToken.into(),
            "--program-id",
            &program_id.to_string(),
            "--member-address",
            &member_address.to_string(),
        ],
    )
    .await
    .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    let mint = Pubkey::from_str(value["commandOutput"]["address"].as_str().unwrap()).unwrap();
    let account = config.rpc_client.get_account(&mint).await.unwrap();
    let mint_state = StateWithExtensionsOwned::<Mint>::unpack(account.data).unwrap();

    let extension = mint_state.get_extension::<GroupMemberPointer>().unwrap();

    assert_eq!(
        extension.member_address,
        Some(member_address).try_into().unwrap()
    );

    let new_member_address = Pubkey::new_unique();

    let _new_result = process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::UpdateMemberAddress.into(),
            &mint.to_string(),
            &new_member_address.to_string(),
        ],
    )
    .await
    .unwrap();

    let new_account = config.rpc_client.get_account(&mint).await.unwrap();
    let new_mint_state = StateWithExtensionsOwned::<Mint>::unpack(new_account.data).unwrap();

    let new_extension = new_mint_state
        .get_extension::<GroupMemberPointer>()
        .unwrap();

    assert_eq!(
        new_extension.member_address,
        Some(new_member_address).try_into().unwrap()
    );

    let _result_with_disable = process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::UpdateMemberAddress.into(),
            &mint.to_string(),
            "--disable",
        ],
    )
    .await;

    let new_account_disable = config.rpc_client.get_account(&mint).await.unwrap();
    let new_mint_state_disable =
        StateWithExtensionsOwned::<Mint>::unpack(new_account_disable.data).unwrap();

    let new_extension_disable = new_mint_state_disable
        .get_extension::<GroupMemberPointer>()
        .unwrap();

    assert_eq!(
        new_extension_disable.member_address,
        None.try_into().unwrap()
    );
}

async fn transfer_hook(test_validator: &TestValidator, payer: &Keypair) {
    let program_id = spl_token_2022::id();
    let mut config = test_config_with_default_signer(test_validator, payer, &program_id);
    let transfer_hook_program_id = Pubkey::new_unique();

    let result = process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::CreateToken.into(),
            "--program-id",
            &program_id.to_string(),
            "--transfer-hook",
            &transfer_hook_program_id.to_string(),
        ],
    )
    .await;

    let value: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    let mint = Pubkey::from_str(value["commandOutput"]["address"].as_str().unwrap()).unwrap();
    let account = config.rpc_client.get_account(&mint).await.unwrap();
    let mint_state = StateWithExtensionsOwned::<Mint>::unpack(account.data).unwrap();
    let extension = mint_state.get_extension::<TransferHook>().unwrap();

    assert_eq!(
        extension.program_id,
        Some(transfer_hook_program_id).try_into().unwrap()
    );

    let new_transfer_hook_program_id = Pubkey::new_unique();

    let _new_result = process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::SetTransferHook.into(),
            &mint.to_string(),
            &new_transfer_hook_program_id.to_string(),
        ],
    )
    .await
    .unwrap();

    let account = config.rpc_client.get_account(&mint).await.unwrap();
    let mint_state = StateWithExtensionsOwned::<Mint>::unpack(account.data).unwrap();
    let extension = mint_state.get_extension::<TransferHook>().unwrap();

    assert_eq!(
        extension.program_id,
        Some(new_transfer_hook_program_id).try_into().unwrap()
    );

    // Make sure that parsing transfer hook accounts works
    let real_program_client = config.program_client;
    let blockhash = Hash::default();
    let program_client: Arc<dyn ProgramClient<ProgramRpcClientSendTransaction>> = Arc::new(
        ProgramOfflineClient::new(blockhash, ProgramRpcClientSendTransaction),
    );
    config.program_client = program_client;
    let _result = exec_test_cmd(
        &config,
        &[
            "spl-token",
            CommandName::Transfer.into(),
            &mint.to_string(),
            "10",
            &Pubkey::new_unique().to_string(),
            "--blockhash",
            &blockhash.to_string(),
            "--nonce",
            &Pubkey::new_unique().to_string(),
            "--nonce-authority",
            &Pubkey::new_unique().to_string(),
            "--sign-only",
            "--mint-decimals",
            &format!("{}", TEST_DECIMALS),
            "--from",
            &Pubkey::new_unique().to_string(),
            "--owner",
            &Pubkey::new_unique().to_string(),
            "--transfer-hook-account",
            &format!("{}:readonly", Pubkey::new_unique()),
            "--transfer-hook-account",
            &format!("{}:writable", Pubkey::new_unique()),
            "--transfer-hook-account",
            &format!("{}:readonly-signer", Pubkey::new_unique()),
            "--transfer-hook-account",
            &format!("{}:writable-signer", Pubkey::new_unique()),
        ],
    )
    .await
    .unwrap();

    config.program_client = real_program_client;
    let _result_with_disable = process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::SetTransferHook.into(),
            &mint.to_string(),
            "--disable",
        ],
    )
    .await
    .unwrap();

    let account = config.rpc_client.get_account(&mint).await.unwrap();
    let mint_state = StateWithExtensionsOwned::<Mint>::unpack(account.data).unwrap();
    let extension = mint_state.get_extension::<TransferHook>().unwrap();

    assert_eq!(extension.program_id, None.try_into().unwrap());
}

async fn transfer_hook_with_transfer_fee(test_validator: &TestValidator, payer: &Keypair) {
    let program_id = spl_token_2022::id();
    let mut config = test_config_with_default_signer(test_validator, payer, &program_id);
    let transfer_hook_program_id = Pubkey::new_unique();

    let transfer_fee_basis_points = 100;
    let maximum_fee: u64 = 10_000_000_000;

    let result = process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::CreateToken.into(),
            "--program-id",
            &program_id.to_string(),
            "--transfer-hook",
            &transfer_hook_program_id.to_string(),
            "--transfer-fee",
            &transfer_fee_basis_points.to_string(),
            &maximum_fee.to_string(),
        ],
    )
    .await;

    // Check that the transfer-hook extension is correctly configured
    let value: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    let mint = Pubkey::from_str(value["commandOutput"]["address"].as_str().unwrap()).unwrap();
    let account = config.rpc_client.get_account(&mint).await.unwrap();
    let mint_state = StateWithExtensionsOwned::<Mint>::unpack(account.data).unwrap();
    let extension = mint_state.get_extension::<TransferHook>().unwrap();
    assert_eq!(
        extension.program_id,
        Some(transfer_hook_program_id).try_into().unwrap()
    );

    // Check that the transfer-fee extension is correctly configured
    let extension = mint_state.get_extension::<TransferFeeConfig>().unwrap();
    assert_eq!(
        u16::from(extension.older_transfer_fee.transfer_fee_basis_points),
        transfer_fee_basis_points
    );
    assert_eq!(
        u64::from(extension.older_transfer_fee.maximum_fee),
        maximum_fee
    );
    assert_eq!(
        u16::from(extension.newer_transfer_fee.transfer_fee_basis_points),
        transfer_fee_basis_points
    );
    assert_eq!(
        u64::from(extension.newer_transfer_fee.maximum_fee),
        maximum_fee
    );

    // Make sure that parsing transfer hook accounts and expected-fee works
    let blockhash = Hash::default();
    let program_client: Arc<dyn ProgramClient<ProgramRpcClientSendTransaction>> = Arc::new(
        ProgramOfflineClient::new(blockhash, ProgramRpcClientSendTransaction),
    );
    config.program_client = program_client;

    let _result = exec_test_cmd(
        &config,
        &[
            "spl-token",
            CommandName::Transfer.into(),
            &mint.to_string(),
            "100",
            &Pubkey::new_unique().to_string(),
            "--blockhash",
            &blockhash.to_string(),
            "--nonce",
            &Pubkey::new_unique().to_string(),
            "--nonce-authority",
            &Pubkey::new_unique().to_string(),
            "--sign-only",
            "--mint-decimals",
            &format!("{}", TEST_DECIMALS),
            "--from",
            &Pubkey::new_unique().to_string(),
            "--owner",
            &Pubkey::new_unique().to_string(),
            "--transfer-hook-account",
            &format!("{}:readonly", Pubkey::new_unique()),
            "--transfer-hook-account",
            &format!("{}:writable", Pubkey::new_unique()),
            "--transfer-hook-account",
            &format!("{}:readonly-signer", Pubkey::new_unique()),
            "--transfer-hook-account",
            &format!("{}:writable-signer", Pubkey::new_unique()),
            "--expected-fee",
            "1",
            "--program-id",
            &program_id.to_string(),
        ],
    )
    .await
    .unwrap();
}

async fn metadata(test_validator: &TestValidator, payer: &Keypair) {
    let program_id = spl_token_2022::id();
    let config = test_config_with_default_signer(test_validator, payer, &program_id);
    let name = "this";
    let symbol = "is";
    let uri = "METADATA!";

    let result = process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::CreateToken.into(),
            "--program-id",
            &program_id.to_string(),
            "--enable-metadata",
        ],
    )
    .await;

    let value: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    let mint = Pubkey::from_str(value["commandOutput"]["address"].as_str().unwrap()).unwrap();
    let account = config.rpc_client.get_account(&mint).await.unwrap();
    let mint_state = StateWithExtensionsOwned::<Mint>::unpack(account.data).unwrap();

    let extension = mint_state.get_extension::<MetadataPointer>().unwrap();
    assert_eq!(extension.metadata_address, Some(mint).try_into().unwrap());

    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::InitializeMetadata.into(),
            &mint.to_string(),
            name,
            symbol,
            uri,
        ],
    )
    .await
    .unwrap();

    let account = config.rpc_client.get_account(&mint).await.unwrap();
    let mint_state = StateWithExtensionsOwned::<Mint>::unpack(account.data).unwrap();
    let fetched_metadata = mint_state
        .get_variable_len_extension::<TokenMetadata>()
        .unwrap();
    assert_eq!(fetched_metadata.name, name);
    assert_eq!(fetched_metadata.symbol, symbol);
    assert_eq!(fetched_metadata.uri, uri);
    assert_eq!(fetched_metadata.mint, mint);
    assert_eq!(
        fetched_metadata.update_authority,
        Some(payer.pubkey()).try_into().unwrap()
    );
    assert_eq!(fetched_metadata.additional_metadata, []);

    // update canonical field
    let new_value = "THIS!";
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::UpdateMetadata.into(),
            &mint.to_string(),
            "NAME",
            new_value,
        ],
    )
    .await
    .unwrap();
    let account = config.rpc_client.get_account(&mint).await.unwrap();
    let mint_state = StateWithExtensionsOwned::<Mint>::unpack(account.data).unwrap();
    let fetched_metadata = mint_state
        .get_variable_len_extension::<TokenMetadata>()
        .unwrap();
    assert_eq!(fetched_metadata.name, new_value);

    // add new field
    let field = "My field!";
    let value = "Try and stop me";
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::UpdateMetadata.into(),
            &mint.to_string(),
            field,
            value,
        ],
    )
    .await
    .unwrap();
    let account = config.rpc_client.get_account(&mint).await.unwrap();
    let mint_state = StateWithExtensionsOwned::<Mint>::unpack(account.data).unwrap();
    let fetched_metadata = mint_state
        .get_variable_len_extension::<TokenMetadata>()
        .unwrap();
    assert_eq!(
        fetched_metadata.additional_metadata,
        [(field.to_string(), value.to_string())]
    );

    // remove it
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::UpdateMetadata.into(),
            &mint.to_string(),
            field,
            "--remove",
        ],
    )
    .await
    .unwrap();
    let account = config.rpc_client.get_account(&mint).await.unwrap();
    let mint_state = StateWithExtensionsOwned::<Mint>::unpack(account.data).unwrap();
    let fetched_metadata = mint_state
        .get_variable_len_extension::<TokenMetadata>()
        .unwrap();
    assert_eq!(fetched_metadata.additional_metadata, []);

    // fail to remove name
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::UpdateMetadata.into(),
            &mint.to_string(),
            "name",
            "--remove",
        ],
    )
    .await
    .unwrap_err();

    // update authority
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::Authorize.into(),
            &mint.to_string(),
            "metadata",
            &mint.to_string(),
        ],
    )
    .await
    .unwrap();

    let account = config.rpc_client.get_account(&mint).await.unwrap();
    let mint_state = StateWithExtensionsOwned::<Mint>::unpack(account.data).unwrap();
    let fetched_metadata = mint_state
        .get_variable_len_extension::<TokenMetadata>()
        .unwrap();
    assert_eq!(
        fetched_metadata.update_authority,
        Some(mint).try_into().unwrap()
    );
}

async fn group(test_validator: &TestValidator, payer: &Keypair) {
    let program_id = spl_token_2022::id();
    let config = test_config_with_default_signer(test_validator, payer, &program_id);
    let max_size = 10;

    // Create token
    let result = process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::CreateToken.into(),
            "--program-id",
            &program_id.to_string(),
            "--enable-group",
        ],
    )
    .await;

    let value: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    let mint = Pubkey::from_str(value["commandOutput"]["address"].as_str().unwrap()).unwrap();

    // Initialize the group
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::InitializeGroup.into(),
            &mint.to_string(),
            &max_size.to_string(),
        ],
    )
    .await
    .unwrap();

    let account = config.rpc_client.get_account(&mint).await.unwrap();
    let mint_state = StateWithExtensionsOwned::<Mint>::unpack(account.data).unwrap();

    let extension = mint_state.get_extension::<TokenGroup>().unwrap();
    assert_eq!(
        extension.update_authority,
        Some(payer.pubkey()).try_into().unwrap()
    );
    assert_eq!(extension.max_size, max_size.into());

    let extension_pointer = mint_state.get_extension::<GroupPointer>().unwrap();
    assert_eq!(
        extension_pointer.group_address,
        Some(mint).try_into().unwrap()
    );

    let new_max_size = 12;

    // Update token-group max-size
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::UpdateGroupMaxSize.into(),
            &mint.to_string(),
            &new_max_size.to_string(),
        ],
    )
    .await
    .unwrap();

    let account = config.rpc_client.get_account(&mint).await.unwrap();

    let updated_mint_state = StateWithExtensionsOwned::<Mint>::unpack(account.data).unwrap();

    let updated_extension = updated_mint_state.get_extension::<TokenGroup>().unwrap();
    assert_eq!(updated_extension.max_size, new_max_size.into());

    // Create member token
    let result = process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::CreateToken.into(),
            "--program-id",
            &program_id.to_string(),
            "--enable-member",
        ],
    )
    .await
    .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    let member_mint =
        Pubkey::from_str(value["commandOutput"]["address"].as_str().unwrap()).unwrap();

    // Initialize it as a member of the group
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::InitializeMember.into(),
            &member_mint.to_string(),
            &mint.to_string(),
        ],
    )
    .await
    .unwrap();

    let account = config.rpc_client.get_account(&mint).await.unwrap();
    let group_mint_state = StateWithExtensionsOwned::<Mint>::unpack(account.data).unwrap();
    let extension = group_mint_state.get_extension::<TokenGroup>().unwrap();
    assert_eq!(u64::from(extension.size), 1);

    let account = config.rpc_client.get_account(&member_mint).await.unwrap();
    let member_mint_state = StateWithExtensionsOwned::<Mint>::unpack(account.data).unwrap();
    let extension = member_mint_state
        .get_extension::<TokenGroupMember>()
        .unwrap();
    assert_eq!(extension.group, mint);
    assert_eq!(extension.mint, member_mint);
    assert_eq!(u64::from(extension.member_number), 1);

    // update authority
    process_test_command(
        &config,
        payer,
        &[
            "spl-token",
            CommandName::Authorize.into(),
            &mint.to_string(),
            "group",
            &mint.to_string(),
        ],
    )
    .await
    .unwrap();

    let account = config.rpc_client.get_account(&mint).await.unwrap();
    let mint_state = StateWithExtensionsOwned::<Mint>::unpack(account.data).unwrap();
    let extension = mint_state.get_extension::<TokenGroup>().unwrap();
    assert_eq!(extension.update_authority, Some(mint).try_into().unwrap());
}

async fn compute_budget(test_validator: &TestValidator, payer: &Keypair) {
    for program_id in VALID_TOKEN_PROGRAM_IDS.iter() {
        let mut config = test_config_with_default_signer(test_validator, payer, program_id);
        config.compute_unit_price = Some(42);
        config.compute_unit_limit = ComputeUnitLimit::Static(40_000);
        run_transfer_test(&config, payer).await;
    }
}
