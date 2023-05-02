#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{TestContext, TokenContext},
    solana_program_test::{processor, tokio, ProgramTest},
    solana_sdk::{
        account::Account,
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        instruction::{AccountMeta, InstructionError},
        program_error::ProgramError,
        pubkey::Pubkey,
        signature::Signer,
        signer::keypair::Keypair,
        transaction::TransactionError,
        transport::TransportError,
    },
    spl_token_2022::{
        error::TokenError,
        extension::{transfer_hook::TransferHook, BaseStateWithExtensions},
        instruction,
        processor::Processor,
    },
    spl_token_client::token::{ExtensionInitializationParams, TokenError as TokenClientError},
    spl_transfer_hook_interface::get_extra_account_metas_address,
    std::{convert::TryInto, sync::Arc},
};

/// Test program to fail transfer hook, conforms to transfer-hook-interface
pub fn process_instruction_fail(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _input: &[u8],
) -> ProgramResult {
    Err(ProgramError::InvalidInstructionData)
}

/// Test program to check signer / write downgrade for repeated accounts, conforms
/// to transfer-hook-interface
pub fn process_instruction_downgrade(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    _input: &[u8],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let source_account_info = next_account_info(account_info_iter)?;
    let _mint_info = next_account_info(account_info_iter)?;
    let _destination_account_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let _extra_account_metas_info = next_account_info(account_info_iter)?;

    let source_account_info_again = next_account_info(account_info_iter)?;
    let authority_info_again = next_account_info(account_info_iter)?;

    if source_account_info.key != source_account_info_again.key {
        return Err(ProgramError::InvalidAccountData);
    }

    if source_account_info_again.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }

    if authority_info.key != authority_info_again.key {
        return Err(ProgramError::InvalidAccountData);
    }

    if authority_info.is_signer {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

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

async fn setup(mint: Keypair, program_id: &Pubkey, authority: &Pubkey) -> TokenContext {
    let mut program_test = ProgramTest::default();
    program_test.prefer_bpf(false);
    program_test.add_program(
        "spl_token_2022",
        spl_token_2022::id(),
        processor!(Processor::process),
    );
    program_test.add_program(
        "my_transfer_hook",
        *program_id,
        processor!(spl_transfer_hook_example::processor::process),
    );
    let validation_address = get_extra_account_metas_address(&mint.pubkey(), program_id);
    let account_metas = vec![
        AccountMeta {
            pubkey: Pubkey::new_unique(),
            is_signer: false,
            is_writable: false,
        },
        AccountMeta {
            pubkey: Pubkey::new_unique(),
            is_signer: false,
            is_writable: false,
        },
    ];
    program_test.add_account(
        validation_address,
        Account {
            lamports: 1_000_000_000, // a lot, just to be safe
            data: spl_transfer_hook_example::state::example_data(&account_metas).unwrap(),
            owner: *program_id,
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
                authority: Some(*authority),
                program_id: Some(*program_id),
            }],
            None,
        )
        .await
        .unwrap();
    context.token_context.take().unwrap()
}

#[tokio::test]
async fn success_init() {
    let authority = Pubkey::new_unique();
    let program_id = Pubkey::new_unique();
    let mint_keypair = Keypair::new();
    let token = setup(mint_keypair, &program_id, &authority).await.token;

    let state = token.get_mint_info().await.unwrap();
    assert!(state.base.is_initialized);
    let extension = state.get_extension::<TransferHook>().unwrap();
    assert_eq!(extension.authority, Some(authority).try_into().unwrap());
    assert_eq!(extension.program_id, Some(program_id).try_into().unwrap());
}

#[tokio::test]
async fn set_authority() {
    let authority = Keypair::new();
    let program_id = Pubkey::new_unique();
    let mint_keypair = Keypair::new();
    let token = setup(mint_keypair, &program_id, &authority.pubkey())
        .await
        .token;
    let new_authority = Keypair::new();

    // fail, wrong signature
    let wrong = Keypair::new();
    let err = token
        .set_authority(
            token.get_address(),
            &wrong.pubkey(),
            Some(&new_authority.pubkey()),
            instruction::AuthorityType::TransferHook,
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
            instruction::AuthorityType::TransferHook,
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
            instruction::AuthorityType::TransferHook,
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
            instruction::AuthorityType::TransferHook,
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
    let token_context = setup(mint_keypair, &program_id, &authority.pubkey()).await;
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
}

#[tokio::test]
async fn fail_transfer_hook_program() {
    let authority = Pubkey::new_unique();
    let program_id = Pubkey::new_unique();
    let mint = Keypair::new();
    let mut program_test = ProgramTest::default();
    program_test.prefer_bpf(false);
    program_test.add_program(
        "spl_token_2022",
        spl_token_2022::id(),
        processor!(Processor::process),
    );
    program_test.add_program(
        "my_transfer_hook",
        program_id,
        processor!(process_instruction_fail),
    );
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
    program_test.prefer_bpf(false);
    program_test.add_program(
        "spl_token_2022",
        spl_token_2022::id(),
        processor!(Processor::process),
    );
    program_test.add_program(
        "my_transfer_hook",
        program_id,
        processor!(process_instruction_downgrade),
    );
    let alice = Keypair::new();
    let alice_account = Keypair::new();
    let validation_address = get_extra_account_metas_address(&mint.pubkey(), &program_id);
    let account_metas = vec![
        AccountMeta {
            pubkey: alice_account.pubkey(),
            is_signer: false,
            is_writable: true,
        },
        AccountMeta {
            pubkey: alice.pubkey(),
            is_signer: true,
            is_writable: false,
        },
    ];
    program_test.add_account(
        validation_address,
        Account {
            lamports: 1_000_000_000, // a lot, just to be safe
            data: spl_transfer_hook_example::state::example_data(&account_metas).unwrap(),
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
