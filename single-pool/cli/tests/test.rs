#![allow(clippy::arithmetic_side_effects)]

use {
    serial_test::serial,
    solana_cli_config::Config as SolanaConfig,
    solana_client::nonblocking::rpc_client::RpcClient,
    solana_sdk::{
        bpf_loader_upgradeable,
        clock::Epoch,
        epoch_schedule::{EpochSchedule, MINIMUM_SLOTS_PER_EPOCH},
        native_token::LAMPORTS_PER_SOL,
        pubkey::Pubkey,
        signature::{write_keypair_file, Keypair, Signer},
        stake::{
            self,
            state::{Authorized, Lockup, StakeStateV2},
        },
        system_instruction, system_program,
        transaction::Transaction,
    },
    solana_test_validator::{TestValidator, TestValidatorGenesis, UpgradeableProgramInfo},
    solana_vote_program::{
        vote_instruction::{self, CreateVoteAccountConfig},
        vote_state::{VoteInit, VoteState},
    },
    spl_token_client::client::{ProgramClient, ProgramRpcClient, ProgramRpcClientSendTransaction},
    std::{path::PathBuf, process::Command, str::FromStr, sync::Arc, time::Duration},
    tempfile::NamedTempFile,
    test_case::test_case,
    tokio::time::sleep,
};

type PClient = Arc<dyn ProgramClient<ProgramRpcClientSendTransaction>>;
const SVSP_CLI: &str = "../../target/debug/spl-single-pool";

#[allow(dead_code)]
pub struct Env {
    pub rpc_client: Arc<RpcClient>,
    pub program_client: PClient,
    pub payer: Keypair,
    pub keypair_file_path: String,
    pub config_file_path: String,
    pub vote_account: Pubkey,

    // persist in struct so they dont scope out but callers dont need to make them
    validator: TestValidator,
    keypair_file: NamedTempFile,
    config_file: NamedTempFile,
}

async fn setup(initialize: bool) -> Env {
    // start test validator
    let (validator, payer) = start_validator().await;

    // make clients
    let rpc_client = Arc::new(validator.get_async_rpc_client());
    let program_client: PClient = Arc::new(ProgramRpcClient::new(
        rpc_client.clone(),
        ProgramRpcClientSendTransaction,
    ));

    // write the payer to disk
    let keypair_file = NamedTempFile::new().unwrap();
    write_keypair_file(&payer, &keypair_file).unwrap();

    // write a full config file with our rpc and payer to disk
    let config_file = NamedTempFile::new().unwrap();
    let config_file_path = config_file.path().to_str().unwrap();
    let solana_config = SolanaConfig {
        json_rpc_url: validator.rpc_url(),
        websocket_url: validator.rpc_pubsub_url(),
        keypair_path: keypair_file.path().to_str().unwrap().to_string(),
        ..SolanaConfig::default()
    };
    solana_config.save(config_file_path).unwrap();

    // make vote and stake accounts
    let vote_account = create_vote_account(&program_client, &payer, &payer.pubkey()).await;
    if initialize {
        let status = Command::new(SVSP_CLI)
            .args([
                "manage",
                "initialize",
                "-C",
                config_file_path,
                &vote_account.to_string(),
            ])
            .status()
            .unwrap();
        assert!(status.success());
    }

    Env {
        rpc_client,
        program_client,
        payer,
        keypair_file_path: keypair_file.path().to_str().unwrap().to_string(),
        config_file_path: config_file_path.to_string(),
        vote_account,
        validator,
        keypair_file,
        config_file,
    }
}

async fn start_validator() -> (TestValidator, Keypair) {
    solana_logger::setup();
    let mut test_validator_genesis = TestValidatorGenesis::default();

    test_validator_genesis.epoch_schedule(EpochSchedule::custom(
        MINIMUM_SLOTS_PER_EPOCH,
        MINIMUM_SLOTS_PER_EPOCH,
        false,
    ));

    test_validator_genesis.add_upgradeable_programs_with_path(&[
        UpgradeableProgramInfo {
            program_id: Pubkey::from_str("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s").unwrap(),
            loader: bpf_loader_upgradeable::id(),
            program_path: PathBuf::from("../program/tests/fixtures/mpl_token_metadata.so"),
            upgrade_authority: Pubkey::default(),
        },
        UpgradeableProgramInfo {
            program_id: spl_single_pool::id(),
            loader: bpf_loader_upgradeable::id(),
            program_path: PathBuf::from("../../target/deploy/spl_single_pool.so"),
            upgrade_authority: Pubkey::default(),
        },
    ]);
    test_validator_genesis.start_async().await
}

async fn wait_for_next_epoch(rpc_client: &RpcClient) -> Epoch {
    let current_epoch = rpc_client.get_epoch_info().await.unwrap().epoch;
    println!("current epoch {}, advancing to next...", current_epoch);
    loop {
        let epoch_info = rpc_client.get_epoch_info().await.unwrap();
        if epoch_info.epoch > current_epoch {
            return epoch_info.epoch;
        }

        sleep(Duration::from_millis(200)).await;
    }
}

async fn create_vote_account(
    program_client: &PClient,
    payer: &Keypair,
    withdrawer: &Pubkey,
) -> Pubkey {
    let validator = Keypair::new();
    let vote_account = Keypair::new();
    let voter = Keypair::new();

    let zero_rent = program_client
        .get_minimum_balance_for_rent_exemption(0)
        .await
        .unwrap();

    let vote_rent = program_client
        .get_minimum_balance_for_rent_exemption(VoteState::size_of() * 2)
        .await
        .unwrap();

    let blockhash = program_client.get_latest_blockhash().await.unwrap();

    let mut instructions = vec![system_instruction::create_account(
        &payer.pubkey(),
        &validator.pubkey(),
        zero_rent,
        0,
        &system_program::id(),
    )];
    instructions.append(&mut vote_instruction::create_account_with_config(
        &payer.pubkey(),
        &vote_account.pubkey(),
        &VoteInit {
            node_pubkey: validator.pubkey(),
            authorized_voter: voter.pubkey(),
            authorized_withdrawer: *withdrawer,
            ..VoteInit::default()
        },
        vote_rent,
        CreateVoteAccountConfig {
            space: VoteState::size_of() as u64,
            ..Default::default()
        },
    ));

    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));

    transaction
        .try_partial_sign(&vec![payer], blockhash)
        .unwrap();
    transaction
        .try_partial_sign(&vec![&validator, &vote_account], blockhash)
        .unwrap();

    program_client.send_transaction(&transaction).await.unwrap();

    vote_account.pubkey()
}

async fn create_and_delegate_stake_account(
    program_client: &PClient,
    payer: &Keypair,
    vote_account: &Pubkey,
) -> Pubkey {
    let stake_account = Keypair::new();

    let stake_rent = program_client
        .get_minimum_balance_for_rent_exemption(StakeStateV2::size_of())
        .await
        .unwrap();
    let blockhash = program_client.get_latest_blockhash().await.unwrap();

    let mut transaction = Transaction::new_with_payer(
        &stake::instruction::create_account(
            &payer.pubkey(),
            &stake_account.pubkey(),
            &Authorized::auto(&payer.pubkey()),
            &Lockup::default(),
            stake_rent + LAMPORTS_PER_SOL,
        ),
        Some(&payer.pubkey()),
    );

    transaction
        .try_partial_sign(&vec![payer], blockhash)
        .unwrap();
    transaction
        .try_partial_sign(&vec![&stake_account], blockhash)
        .unwrap();

    program_client.send_transaction(&transaction).await.unwrap();

    let mut transaction = Transaction::new_with_payer(
        &[stake::instruction::delegate_stake(
            &stake_account.pubkey(),
            &payer.pubkey(),
            vote_account,
        )],
        Some(&payer.pubkey()),
    );

    transaction.sign(&vec![payer], blockhash);

    program_client.send_transaction(&transaction).await.unwrap();

    stake_account.pubkey()
}

#[tokio::test]
#[serial]
async fn reactivate_pool_stake() {
    let env = setup(true).await;

    // setting up a test validator for this to succeed is hell, and success is
    // tested in program tests so we just make sure the cli can send a
    // well-formed instruction
    let output = Command::new(SVSP_CLI)
        .args([
            "manage",
            "reactivate-pool-stake",
            "-C",
            &env.config_file_path,
            "--vote-account",
            &env.vote_account.to_string(),
            "--skip-deactivation-check",
        ])
        .output()
        .unwrap();
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("custom program error: 0xc"));
}

#[test_case(true; "default_stake")]
#[test_case(false; "normal_stake")]
#[tokio::test]
#[serial]
async fn deposit(use_default: bool) {
    let env = setup(true).await;

    let stake_account = if use_default {
        let status = Command::new(SVSP_CLI)
            .args([
                "create-default-stake",
                "-C",
                &env.config_file_path,
                "--vote-account",
                &env.vote_account.to_string(),
                &LAMPORTS_PER_SOL.to_string(),
            ])
            .status()
            .unwrap();
        assert!(status.success());

        Pubkey::default()
    } else {
        create_and_delegate_stake_account(&env.program_client, &env.payer, &env.vote_account).await
    };

    wait_for_next_epoch(&env.rpc_client).await;

    let mut args = vec![
        "deposit".to_string(),
        "-C".to_string(),
        env.config_file_path,
    ];

    if use_default {
        args.extend([
            "--vote-account".to_string(),
            env.vote_account.to_string(),
            "--default-stake-account".to_string(),
        ]);
    } else {
        args.push(stake_account.to_string());
    };

    let status = Command::new(SVSP_CLI).args(&args).status().unwrap();
    assert!(status.success());
}

#[tokio::test]
#[serial]
async fn withdraw() {
    let env = setup(true).await;
    let stake_account =
        create_and_delegate_stake_account(&env.program_client, &env.payer, &env.vote_account).await;

    wait_for_next_epoch(&env.rpc_client).await;

    let status = Command::new(SVSP_CLI)
        .args([
            "deposit",
            "-C",
            &env.config_file_path,
            &stake_account.to_string(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let status = Command::new(SVSP_CLI)
        .args([
            "withdraw",
            "-C",
            &env.config_file_path,
            "--vote-account",
            &env.vote_account.to_string(),
            "ALL",
        ])
        .status()
        .unwrap();
    assert!(status.success());
}

#[tokio::test]
#[serial]
async fn create_metadata() {
    let env = setup(false).await;

    let status = Command::new(SVSP_CLI)
        .args([
            "manage",
            "initialize",
            "-C",
            &env.config_file_path,
            "--skip-metadata",
            &env.vote_account.to_string(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let status = Command::new(SVSP_CLI)
        .args([
            "manage",
            "create-token-metadata",
            "-C",
            &env.config_file_path,
            "--vote-account",
            &env.vote_account.to_string(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
}

#[tokio::test]
#[serial]
async fn update_metadata() {
    let env = setup(true).await;

    let status = Command::new(SVSP_CLI)
        .args([
            "manage",
            "update-token-metadata",
            "-C",
            &env.config_file_path,
            "--vote-account",
            &env.vote_account.to_string(),
            "whatever",
            "idk",
        ])
        .status()
        .unwrap();
    assert!(status.success());

    // testing this flag because the match is rather torturous
    let status = Command::new(SVSP_CLI)
        .args([
            "manage",
            "update-token-metadata",
            "-C",
            &env.config_file_path,
            "--vote-account",
            &env.vote_account.to_string(),
            "--authorized-withdrawer",
            &env.keypair_file_path,
            "something",
            "new",
        ])
        .status()
        .unwrap();
    assert!(status.success());
}
