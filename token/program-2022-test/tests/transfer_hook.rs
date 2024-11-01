#![cfg(feature = "test-sbf")]

mod program_test;
use {
    futures_util::TryFutureExt,
    program_test::{
        ConfidentialTokenAccountBalances, ConfidentialTokenAccountMeta, TestContext, TokenContext,
    },
    solana_program_test::{tokio, ProgramTest},
    solana_sdk::{
        account::Account,
        instruction::{AccountMeta, Instruction, InstructionError},
        program_option::COption,
        pubkey::Pubkey,
        signature::Signer,
        signer::keypair::Keypair,
        transaction::TransactionError,
        transport::TransportError,
    },
    spl_tlv_account_resolution::{account::ExtraAccountMeta, seeds::Seed},
    spl_token_2022::{
        error::TokenError,
        extension::{
            transfer_fee::{TransferFee, TransferFeeAmount, TransferFeeConfig},
            transfer_hook::{TransferHook, TransferHookAccount},
            BaseStateWithExtensions,
        },
        instruction, offchain,
    },
    spl_token_client::token::{ExtensionInitializationParams, TokenError as TokenClientError},
    spl_transfer_hook_interface::{
        get_extra_account_metas_address, offchain::add_extra_account_metas_for_execute,
    },
    std::{convert::TryInto, sync::Arc},
};

const TEST_MAXIMUM_FEE: u64 = 10_000_000;
const TEST_FEE_BASIS_POINTS: u16 = 100;

async fn setup_accounts(
    token_context: &TokenContext,
    alice_account: Keypair,
    bob_account: Keypair,
    amount: u64,
) -> (Pubkey, Pubkey) {
    token_context
        .token
        .create_auxiliary_token_account(&alice_account, &token_context.alice.pubkey())
        .await
        .unwrap();
    let alice_account = alice_account.pubkey();
    token_context
        .token
        .create_auxiliary_token_account(&bob_account, &token_context.bob.pubkey())
        .await
        .unwrap();
    let bob_account = bob_account.pubkey();

    // mint tokens
    token_context
        .token
        .mint_to(
            &alice_account,
            &token_context.mint_authority.pubkey(),
            amount,
            &[&token_context.mint_authority],
        )
        .await
        .unwrap();
    (alice_account, bob_account)
}

fn setup_program_test(program_id: &Pubkey) -> ProgramTest {
    let mut program_test = ProgramTest::default();
    program_test.add_program("spl_token_2022", spl_token_2022::id(), None);
    program_test.add_program("spl_transfer_hook_example", *program_id, None);
    program_test
}

fn add_validation_account(program_test: &mut ProgramTest, mint: &Pubkey, program_id: &Pubkey) {
    let validation_address = get_extra_account_metas_address(mint, program_id);
    let extra_account_metas = vec![
        AccountMeta {
            pubkey: Pubkey::new_unique(),
            is_signer: false,
            is_writable: false,
        }
        .into(),
        AccountMeta {
            pubkey: Pubkey::new_unique(),
            is_signer: false,
            is_writable: false,
        }
        .into(),
        ExtraAccountMeta::new_with_seeds(
            &[
                Seed::AccountKey { index: 0 }, // source
                Seed::AccountKey { index: 2 }, // destination
                Seed::AccountKey { index: 4 }, // validation state
            ],
            false,
            true,
        )
        .unwrap(),
        ExtraAccountMeta::new_with_seeds(
            &[
                Seed::Literal {
                    bytes: vec![1, 2, 3, 4, 5, 6],
                },
                Seed::AccountKey { index: 2 }, // destination
                Seed::AccountKey { index: 5 }, // extra meta 1
            ],
            false,
            true,
        )
        .unwrap(),
    ];
    program_test.add_account(
        validation_address,
        Account {
            lamports: 1_000_000_000, // a lot, just to be safe
            data: spl_transfer_hook_example::state::example_data(&extra_account_metas).unwrap(),
            owner: *program_id,
            ..Account::default()
        },
    );
}

async fn setup(mint: Keypair, program_id: &Pubkey, authority: &Pubkey) -> TestContext {
    let mut program_test = setup_program_test(program_id);
    add_validation_account(&mut program_test, &mint.pubkey(), program_id);

    let context = program_test.start_with_context().await;
    let context = Arc::new(tokio::sync::Mutex::new(context));
    let mut context = TestContext {
        context,
        token_context: None,
    };
    context
        .init_token_with_mint_keypair_and_freeze_authority(
            mint,
            vec![ExtensionInitializationParams::TransferHook {
                authority: Some(*authority),
                program_id: Some(*program_id),
            }],
            None,
        )
        .await
        .unwrap();
    context
}

async fn setup_with_fee(mint: Keypair, program_id: &Pubkey, authority: &Pubkey) -> TestContext {
    let mut program_test = setup_program_test(program_id);

    let transfer_fee_config_authority = Keypair::new();
    let withdraw_withheld_authority = Keypair::new();
    let transfer_fee_basis_points = TEST_FEE_BASIS_POINTS;
    let maximum_fee = TEST_MAXIMUM_FEE;
    add_validation_account(&mut program_test, &mint.pubkey(), program_id);

    let context = program_test.start_with_context().await;
    let context = Arc::new(tokio::sync::Mutex::new(context));

    let mut context = TestContext {
        context,
        token_context: None,
    };
    context
        .init_token_with_mint_keypair_and_freeze_authority(
            mint,
            vec![
                ExtensionInitializationParams::TransferHook {
                    authority: Some(*authority),
                    program_id: Some(*program_id),
                },
                ExtensionInitializationParams::TransferFeeConfig {
                    transfer_fee_config_authority: transfer_fee_config_authority.pubkey().into(),
                    withdraw_withheld_authority: withdraw_withheld_authority.pubkey().into(),
                    transfer_fee_basis_points,
                    maximum_fee,
                },
            ],
            None,
        )
        .await
        .unwrap();
    context
}

async fn setup_with_confidential_transfers(
    mint: Keypair,
    program_id: &Pubkey,
    authority: &Pubkey,
) -> TestContext {
    let mut program_test = setup_program_test(program_id);
    add_validation_account(&mut program_test, &mint.pubkey(), program_id);

    let context = program_test.start_with_context().await;
    let context = Arc::new(tokio::sync::Mutex::new(context));
    let mut context = TestContext {
        context,
        token_context: None,
    };
    context
        .init_token_with_mint_keypair_and_freeze_authority(
            mint,
            vec![
                ExtensionInitializationParams::TransferHook {
                    authority: Some(*authority),
                    program_id: Some(*program_id),
                },
                ExtensionInitializationParams::ConfidentialTransferMint {
                    authority: Some(*authority),
                    auto_approve_new_accounts: true,
                    auditor_elgamal_pubkey: None,
                },
            ],
            None,
        )
        .await
        .unwrap();
    context
}

#[tokio::test]
async fn success_init() {
    let authority = Pubkey::new_unique();
    let program_id = Pubkey::new_unique();
    let mint_keypair = Keypair::new();
    let token = setup(mint_keypair, &program_id, &authority)
        .await
        .token_context
        .take()
        .unwrap()
        .token;

    let state = token.get_mint_info().await.unwrap();
    assert!(state.base.is_initialized);
    let extension = state.get_extension::<TransferHook>().unwrap();
    assert_eq!(extension.authority, Some(authority).try_into().unwrap());
    assert_eq!(extension.program_id, Some(program_id).try_into().unwrap());
}

#[tokio::test]
async fn fail_init_all_none() {
    let mut program_test = ProgramTest::default();
    program_test.add_program("spl_token_2022", spl_token_2022::id(), None);
    let context = program_test.start_with_context().await;
    let context = Arc::new(tokio::sync::Mutex::new(context));
    let mut context = TestContext {
        context,
        token_context: None,
    };
    let err = context
        .init_token_with_mint_keypair_and_freeze_authority(
            Keypair::new(),
            vec![ExtensionInitializationParams::TransferHook {
                authority: None,
                program_id: None,
            }],
            None,
        )
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                1,
                InstructionError::Custom(TokenError::InvalidInstruction as u32)
            )
        )))
    );
}

#[tokio::test]
async fn set_authority() {
    let authority = Keypair::new();
    let program_id = Pubkey::new_unique();
    let mint_keypair = Keypair::new();
    let token = setup(mint_keypair, &program_id, &authority.pubkey())
        .await
        .token_context
        .take()
        .unwrap()
        .token;
    let new_authority = Keypair::new();

    // fail, wrong signature
    let wrong = Keypair::new();
    let err = token
        .set_authority(
            token.get_address(),
            &wrong.pubkey(),
            Some(&new_authority.pubkey()),
            instruction::AuthorityType::TransferHookProgramId,
            &[&wrong],
        )
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::OwnerMismatch as u32)
            )
        )))
    );

    // success
    token
        .set_authority(
            token.get_address(),
            &authority.pubkey(),
            Some(&new_authority.pubkey()),
            instruction::AuthorityType::TransferHookProgramId,
            &[&authority],
        )
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<TransferHook>().unwrap();
    assert_eq!(
        extension.authority,
        Some(new_authority.pubkey()).try_into().unwrap(),
    );

    // set to none
    token
        .set_authority(
            token.get_address(),
            &new_authority.pubkey(),
            None,
            instruction::AuthorityType::TransferHookProgramId,
            &[&new_authority],
        )
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<TransferHook>().unwrap();
    assert_eq!(extension.authority, None.try_into().unwrap(),);

    // fail set again
    let err = token
        .set_authority(
            token.get_address(),
            &new_authority.pubkey(),
            Some(&authority.pubkey()),
            instruction::AuthorityType::TransferHookProgramId,
            &[&new_authority],
        )
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::AuthorityTypeNotSupported as u32)
            )
        )))
    );
}

#[tokio::test]
async fn update_transfer_hook_program_id() {
    let authority = Keypair::new();
    let program_id = Pubkey::new_unique();
    let mint_keypair = Keypair::new();
    let token = setup(mint_keypair, &program_id, &authority.pubkey())
        .await
        .token_context
        .take()
        .unwrap()
        .token;
    let new_program_id = Pubkey::new_unique();

    // fail, wrong signature
    let wrong = Keypair::new();
    let err = token
        .update_transfer_hook_program_id(&wrong.pubkey(), Some(new_program_id), &[&wrong])
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::OwnerMismatch as u32)
            )
        )))
    );

    // success
    token
        .update_transfer_hook_program_id(&authority.pubkey(), Some(new_program_id), &[&authority])
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<TransferHook>().unwrap();
    assert_eq!(
        extension.program_id,
        Some(new_program_id).try_into().unwrap(),
    );

    // set to none
    token
        .update_transfer_hook_program_id(&authority.pubkey(), None, &[&authority])
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<TransferHook>().unwrap();
    assert_eq!(extension.program_id, None.try_into().unwrap(),);
}

#[tokio::test]
async fn success_transfer() {
    let authority = Keypair::new();
    let program_id = Pubkey::new_unique();
    let mint_keypair = Keypair::new();
    let token_context = setup(mint_keypair, &program_id, &authority.pubkey())
        .await
        .token_context
        .take()
        .unwrap();
    let amount = 10;
    let (alice_account, bob_account) =
        setup_accounts(&token_context, Keypair::new(), Keypair::new(), amount).await;

    token_context
        .token
        .transfer(
            &alice_account,
            &bob_account,
            &token_context.alice.pubkey(),
            amount,
            &[&token_context.alice],
        )
        .await
        .unwrap();

    let destination = token_context
        .token
        .get_account_info(&bob_account)
        .await
        .unwrap();
    assert_eq!(destination.base.amount, amount);

    // the example program checks that the transferring flag was set to true,
    // so make sure that it was correctly unset by the token program
    assert_eq!(
        destination
            .get_extension::<TransferHookAccount>()
            .unwrap()
            .transferring,
        false.into()
    );
    let source = token_context
        .token
        .get_account_info(&alice_account)
        .await
        .unwrap();
    assert_eq!(
        source
            .get_extension::<TransferHookAccount>()
            .unwrap()
            .transferring,
        false.into()
    );
}

#[tokio::test]
async fn success_transfer_with_fee() {
    let authority = Keypair::new();
    let program_id = Pubkey::new_unique();
    let mint_keypair = Keypair::new();

    let maximum_fee = TEST_MAXIMUM_FEE;
    let alice_amount = maximum_fee * 100;
    let transfer_amount = maximum_fee;

    let token_context = setup_with_fee(mint_keypair, &program_id, &authority.pubkey())
        .await
        .token_context
        .take()
        .unwrap();

    let (alice_account, bob_account) =
        setup_accounts(&token_context, Keypair::new(), Keypair::new(), alice_amount).await;

    let transfer_fee = TransferFee {
        epoch: 0.into(),
        transfer_fee_basis_points: TEST_FEE_BASIS_POINTS.into(),
        maximum_fee: TEST_MAXIMUM_FEE.into(),
    };
    let transfer_fee_config = TransferFeeConfig {
        transfer_fee_config_authority: COption::Some(Pubkey::new_unique()).try_into().unwrap(),
        withdraw_withheld_authority: COption::Some(Pubkey::new_unique()).try_into().unwrap(),
        withheld_amount: 0.into(),
        older_transfer_fee: transfer_fee,
        newer_transfer_fee: transfer_fee,
    };
    let fee = transfer_fee_config
        .calculate_epoch_fee(0, transfer_amount)
        .unwrap();

    token_context
        .token
        .transfer_with_fee(
            &alice_account,
            &bob_account,
            &token_context.alice.pubkey(),
            transfer_amount,
            fee,
            &[&token_context.alice],
        )
        .await
        .unwrap();

    // Get the accounts' state after the transfer
    let alice_state = token_context
        .token
        .get_account_info(&alice_account)
        .await
        .unwrap();
    let bob_state = token_context
        .token
        .get_account_info(&bob_account)
        .await
        .unwrap();

    // Check that the correct amount was deducted from Alice's account
    assert_eq!(alice_state.base.amount, alice_amount - transfer_amount);

    // Check the there are no tokens withheld in Alice's account
    let extension = alice_state.get_extension::<TransferFeeAmount>().unwrap();
    assert_eq!(extension.withheld_amount, 0.into());

    // Check the fee tokens are withheld in Bobs's account
    let extension = bob_state.get_extension::<TransferFeeAmount>().unwrap();
    assert_eq!(extension.withheld_amount, fee.into());

    // Check that the correct amount was added to Bobs's account
    assert_eq!(bob_state.base.amount, transfer_amount - fee);

    // the example program checks that the transferring flag was set to true,
    // so make sure that it was correctly unset by the token program
    assert_eq!(
        bob_state
            .get_extension::<TransferHookAccount>()
            .unwrap()
            .transferring,
        false.into()
    );

    assert_eq!(
        alice_state
            .get_extension::<TransferHookAccount>()
            .unwrap()
            .transferring,
        false.into()
    );
}

#[tokio::test]
async fn fail_transfer_hook_program() {
    let authority = Pubkey::new_unique();
    let program_id = Pubkey::new_unique();
    let mint = Keypair::new();
    let mut program_test = ProgramTest::default();
    program_test.add_program("spl_token_2022", spl_token_2022::id(), None);
    program_test.add_program("spl_transfer_hook_example_fail", program_id, None);
    let validation_address = get_extra_account_metas_address(&mint.pubkey(), &program_id);
    program_test.add_account(
        validation_address,
        Account {
            lamports: 1_000_000_000, // a lot, just to be safe
            data: spl_transfer_hook_example::state::example_data(&[]).unwrap(),
            owner: program_id,
            ..Account::default()
        },
    );
    let context = program_test.start_with_context().await;
    let context = Arc::new(tokio::sync::Mutex::new(context));
    let mut context = TestContext {
        context,
        token_context: None,
    };
    context
        .init_token_with_mint_keypair_and_freeze_authority(
            mint,
            vec![ExtensionInitializationParams::TransferHook {
                authority: Some(authority),
                program_id: Some(program_id),
            }],
            None,
        )
        .await
        .unwrap();
    let token_context = context.token_context.take().unwrap();

    let amount = 10;
    let (alice_account, bob_account) =
        setup_accounts(&token_context, Keypair::new(), Keypair::new(), amount).await;

    let err = token_context
        .token
        .transfer(
            &alice_account,
            &bob_account,
            &token_context.alice.pubkey(),
            amount,
            &[&token_context.alice],
        )
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(0, InstructionError::InvalidInstructionData)
        )))
    );
}

#[tokio::test]
async fn success_downgrade_writable_and_signer_accounts() {
    let authority = Pubkey::new_unique();
    let program_id = Pubkey::new_unique();
    let mint = Keypair::new();
    let mut program_test = ProgramTest::default();
    program_test.add_program("spl_token_2022", spl_token_2022::id(), None);
    program_test.add_program("spl_transfer_hook_example_downgrade", program_id, None);
    let alice = Keypair::new();
    let alice_account = Keypair::new();
    let validation_address = get_extra_account_metas_address(&mint.pubkey(), &program_id);
    let extra_account_metas = vec![
        AccountMeta {
            pubkey: alice_account.pubkey(),
            is_signer: false,
            is_writable: true,
        }
        .into(),
        AccountMeta {
            pubkey: alice.pubkey(),
            is_signer: true,
            is_writable: false,
        }
        .into(),
    ];
    program_test.add_account(
        validation_address,
        Account {
            lamports: 1_000_000_000, // a lot, just to be safe
            data: spl_transfer_hook_example::state::example_data(&extra_account_metas).unwrap(),
            owner: program_id,
            ..Account::default()
        },
    );
    let context = program_test.start_with_context().await;
    let context = Arc::new(tokio::sync::Mutex::new(context));
    let mut context = TestContext {
        context,
        token_context: None,
    };
    context
        .init_token_with_mint_keypair_and_freeze_authority(
            mint,
            vec![ExtensionInitializationParams::TransferHook {
                authority: Some(authority),
                program_id: Some(program_id),
            }],
            None,
        )
        .await
        .unwrap();
    let mut token_context = context.token_context.take().unwrap();
    token_context.alice = alice;

    let amount = 10;
    let (alice_account, bob_account) =
        setup_accounts(&token_context, alice_account, Keypair::new(), amount).await;

    token_context
        .token
        .transfer(
            &alice_account,
            &bob_account,
            &token_context.alice.pubkey(),
            amount,
            &[&token_context.alice],
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn success_transfers_using_onchain_helper() {
    let authority = Pubkey::new_unique();
    let program_id = Pubkey::new_unique();
    let mint_a_keypair = Keypair::new();
    let mint_a = mint_a_keypair.pubkey();
    let mint_b_keypair = Keypair::new();
    let mint_b = mint_b_keypair.pubkey();
    let amount = 10;

    let swap_program_id = Pubkey::new_unique();
    let mut program_test = setup_program_test(&program_id);
    program_test.add_program("spl_transfer_hook_example_swap", swap_program_id, None);
    add_validation_account(&mut program_test, &mint_a, &program_id);
    add_validation_account(&mut program_test, &mint_b, &program_id);

    let context = program_test.start_with_context().await;
    let context = Arc::new(tokio::sync::Mutex::new(context));
    let mut context_a = TestContext {
        context: context.clone(),
        token_context: None,
    };
    context_a
        .init_token_with_mint_keypair_and_freeze_authority(
            mint_a_keypair,
            vec![ExtensionInitializationParams::TransferHook {
                authority: Some(authority),
                program_id: Some(program_id),
            }],
            None,
        )
        .await
        .unwrap();
    let token_a_context = context_a.token_context.unwrap();
    let (source_a_account, destination_a_account) =
        setup_accounts(&token_a_context, Keypair::new(), Keypair::new(), amount).await;
    let authority_a = token_a_context.alice;
    let token_a = token_a_context.token;
    let mut context_b = TestContext {
        context,
        token_context: None,
    };
    context_b
        .init_token_with_mint_keypair_and_freeze_authority(
            mint_b_keypair,
            vec![ExtensionInitializationParams::TransferHook {
                authority: Some(authority),
                program_id: Some(program_id),
            }],
            None,
        )
        .await
        .unwrap();
    let token_b_context = context_b.token_context.unwrap();
    let (source_b_account, destination_b_account) =
        setup_accounts(&token_b_context, Keypair::new(), Keypair::new(), amount).await;
    let authority_b = token_b_context.alice;
    let account_metas = vec![
        AccountMeta::new(source_a_account, false),
        AccountMeta::new_readonly(mint_a, false),
        AccountMeta::new(destination_a_account, false),
        AccountMeta::new_readonly(authority_a.pubkey(), true),
        AccountMeta::new_readonly(spl_token_2022::id(), false),
        AccountMeta::new(source_b_account, false),
        AccountMeta::new_readonly(mint_b, false),
        AccountMeta::new(destination_b_account, false),
        AccountMeta::new_readonly(authority_b.pubkey(), true),
        AccountMeta::new_readonly(spl_token_2022::id(), false),
    ];

    let mut instruction = Instruction::new_with_bytes(swap_program_id, &[], account_metas);

    add_extra_account_metas_for_execute(
        &mut instruction,
        &program_id,
        &source_a_account,
        &mint_a,
        &destination_a_account,
        &authority_a.pubkey(),
        amount,
        |address| {
            token_a.get_account(address).map_ok_or_else(
                |e| match e {
                    TokenClientError::AccountNotFound => Ok(None),
                    _ => Err(offchain::AccountFetchError::from(e)),
                },
                |acc| Ok(Some(acc.data)),
            )
        },
    )
    .await
    .unwrap();
    add_extra_account_metas_for_execute(
        &mut instruction,
        &program_id,
        &source_b_account,
        &mint_b,
        &destination_b_account,
        &authority_b.pubkey(),
        amount,
        |address| {
            token_a.get_account(address).map_ok_or_else(
                |e| match e {
                    TokenClientError::AccountNotFound => Ok(None),
                    _ => Err(offchain::AccountFetchError::from(e)),
                },
                |acc| Ok(Some(acc.data)),
            )
        },
    )
    .await
    .unwrap();

    token_a
        .process_ixs(&[instruction], &[&authority_a, &authority_b])
        .await
        .unwrap();
}

#[tokio::test]
async fn success_transfers_with_fee_using_onchain_helper() {
    let authority = Pubkey::new_unique();
    let program_id = Pubkey::new_unique();
    let mint_a_keypair = Keypair::new();
    let mint_a = mint_a_keypair.pubkey();
    let mint_b_keypair = Keypair::new();
    let mint_b = mint_b_keypair.pubkey();
    let amount = 10_000_000_000;

    let transfer_fee_config_authority = Keypair::new();
    let withdraw_withheld_authority = Keypair::new();
    let transfer_fee_basis_points = TEST_FEE_BASIS_POINTS;
    let maximum_fee = TEST_MAXIMUM_FEE;

    let swap_program_id = Pubkey::new_unique();
    let mut program_test = setup_program_test(&program_id);
    program_test.add_program(
        "spl_transfer_hook_example_swap_with_fee",
        swap_program_id,
        None,
    );
    add_validation_account(&mut program_test, &mint_a, &program_id);
    add_validation_account(&mut program_test, &mint_b, &program_id);

    let context = program_test.start_with_context().await;
    let context = Arc::new(tokio::sync::Mutex::new(context));
    let mut context_a = TestContext {
        context: context.clone(),
        token_context: None,
    };
    context_a
        .init_token_with_mint_keypair_and_freeze_authority(
            mint_a_keypair,
            vec![
                ExtensionInitializationParams::TransferHook {
                    authority: Some(authority),
                    program_id: Some(program_id),
                },
                ExtensionInitializationParams::TransferFeeConfig {
                    transfer_fee_config_authority: transfer_fee_config_authority.pubkey().into(),
                    withdraw_withheld_authority: withdraw_withheld_authority.pubkey().into(),
                    transfer_fee_basis_points,
                    maximum_fee,
                },
            ],
            None,
        )
        .await
        .unwrap();
    let token_a_context = context_a.token_context.unwrap();
    let (source_a_account, destination_a_account) =
        setup_accounts(&token_a_context, Keypair::new(), Keypair::new(), amount).await;
    let authority_a = token_a_context.alice;
    let token_a = token_a_context.token;
    let mut context_b = TestContext {
        context,
        token_context: None,
    };
    context_b
        .init_token_with_mint_keypair_and_freeze_authority(
            mint_b_keypair,
            vec![
                ExtensionInitializationParams::TransferHook {
                    authority: Some(authority),
                    program_id: Some(program_id),
                },
                ExtensionInitializationParams::TransferFeeConfig {
                    transfer_fee_config_authority: transfer_fee_config_authority.pubkey().into(),
                    withdraw_withheld_authority: withdraw_withheld_authority.pubkey().into(),
                    transfer_fee_basis_points,
                    maximum_fee,
                },
            ],
            None,
        )
        .await
        .unwrap();
    let token_b_context = context_b.token_context.unwrap();
    let (source_b_account, destination_b_account) =
        setup_accounts(&token_b_context, Keypair::new(), Keypair::new(), amount).await;
    let authority_b = token_b_context.alice;
    let account_metas = vec![
        AccountMeta::new(source_a_account, false),
        AccountMeta::new_readonly(mint_a, false),
        AccountMeta::new(destination_a_account, false),
        AccountMeta::new_readonly(authority_a.pubkey(), true),
        AccountMeta::new_readonly(spl_token_2022::id(), false),
        AccountMeta::new(source_b_account, false),
        AccountMeta::new_readonly(mint_b, false),
        AccountMeta::new(destination_b_account, false),
        AccountMeta::new_readonly(authority_b.pubkey(), true),
        AccountMeta::new_readonly(spl_token_2022::id(), false),
    ];

    let mut instruction = Instruction::new_with_bytes(swap_program_id, &[], account_metas);

    add_extra_account_metas_for_execute(
        &mut instruction,
        &program_id,
        &source_a_account,
        &mint_a,
        &destination_a_account,
        &authority_a.pubkey(),
        amount,
        |address| {
            token_a.get_account(address).map_ok_or_else(
                |e| match e {
                    TokenClientError::AccountNotFound => Ok(None),
                    _ => Err(offchain::AccountFetchError::from(e)),
                },
                |acc| Ok(Some(acc.data)),
            )
        },
    )
    .await
    .unwrap();
    add_extra_account_metas_for_execute(
        &mut instruction,
        &program_id,
        &source_b_account,
        &mint_b,
        &destination_b_account,
        &authority_b.pubkey(),
        amount,
        |address| {
            token_a.get_account(address).map_ok_or_else(
                |e| match e {
                    TokenClientError::AccountNotFound => Ok(None),
                    _ => Err(offchain::AccountFetchError::from(e)),
                },
                |acc| Ok(Some(acc.data)),
            )
        },
    )
    .await
    .unwrap();

    token_a
        .process_ixs(&[instruction], &[&authority_a, &authority_b])
        .await
        .unwrap();
}

#[tokio::test]
async fn success_confidential_transfer() {
    let authority = Keypair::new();
    let program_id = Pubkey::new_unique();
    let mint_keypair = Keypair::new();
    let token_context =
        setup_with_confidential_transfers(mint_keypair, &program_id, &authority.pubkey())
            .await
            .token_context
            .take()
            .unwrap();
    let amount = 10;

    let TokenContext {
        token,
        alice,
        bob,
        mint_authority,
        decimals,
        ..
    } = token_context;

    let alice_meta = ConfidentialTokenAccountMeta::new_with_tokens(
        &token,
        &alice,
        None,
        false,
        false,
        &mint_authority,
        amount,
        decimals,
    )
    .await;

    let bob_meta = ConfidentialTokenAccountMeta::new(&token, &bob, Some(2), false, false).await;

    token
        .confidential_transfer_transfer(
            &alice_meta.token_account,
            &bob_meta.token_account,
            &alice.pubkey(),
            None,
            None,
            None,
            amount,
            None,
            &alice_meta.elgamal_keypair,
            &alice_meta.aes_key,
            bob_meta.elgamal_keypair.pubkey(),
            None, // auditor
            &[&alice],
        )
        .await
        .unwrap();

    let destination = token
        .get_account_info(&bob_meta.token_account)
        .await
        .unwrap();
    alice_meta
        .check_balances(
            &token,
            ConfidentialTokenAccountBalances {
                pending_balance_lo: 0,
                pending_balance_hi: 0,
                available_balance: 0,
                decryptable_available_balance: 0,
            },
        )
        .await;
    bob_meta
        .check_balances(
            &token,
            ConfidentialTokenAccountBalances {
                pending_balance_lo: amount,
                pending_balance_hi: 0,
                available_balance: 0,
                decryptable_available_balance: 0,
            },
        )
        .await;

    // the example program checks that the transferring flag was set to true,
    // so make sure that it was correctly unset by the token program
    assert_eq!(
        destination
            .get_extension::<TransferHookAccount>()
            .unwrap()
            .transferring,
        false.into()
    );
    let source = token
        .get_account_info(&alice_meta.token_account)
        .await
        .unwrap();
    assert_eq!(
        source
            .get_extension::<TransferHookAccount>()
            .unwrap()
            .transferring,
        false.into()
    );
}

#[tokio::test]
async fn success_without_validation_account() {
    let authority = Pubkey::new_unique();
    let program_id = Pubkey::new_unique();
    let mint = Keypair::new();
    let mut program_test = ProgramTest::default();
    program_test.add_program("spl_token_2022", spl_token_2022::id(), None);
    program_test.add_program("spl_transfer_hook_example_success", program_id, None);
    let context = program_test.start_with_context().await;
    let context = Arc::new(tokio::sync::Mutex::new(context));
    let mut context = TestContext {
        context,
        token_context: None,
    };
    context
        .init_token_with_mint_keypair_and_freeze_authority(
            mint,
            vec![ExtensionInitializationParams::TransferHook {
                authority: Some(authority),
                program_id: Some(program_id),
            }],
            None,
        )
        .await
        .unwrap();
    let token_context = context.token_context.take().unwrap();

    let amount = 10;
    let (alice_account, bob_account) =
        setup_accounts(&token_context, Keypair::new(), Keypair::new(), amount).await;

    // only add the transfer hook program id, nothing else
    let token = token_context
        .token
        .with_transfer_hook_accounts(vec![AccountMeta::new_readonly(program_id, false)]);
    token
        .transfer(
            &alice_account,
            &bob_account,
            &token_context.alice.pubkey(),
            amount,
            &[&token_context.alice],
        )
        .await
        .unwrap();
    let destination = token.get_account_info(&bob_account).await.unwrap();
    assert_eq!(destination.base.amount, amount);
}
