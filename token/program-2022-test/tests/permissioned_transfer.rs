#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{TestContext, TokenContext},
    solana_program_test::{processor, tokio, ProgramTest},
    solana_sdk::{
        account::Account,
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        instruction::InstructionError,
        program_error::ProgramError,
        pubkey::Pubkey,
        signature::Signer,
        signer::keypair::Keypair,
        transaction::TransactionError,
        transport::TransportError,
    },
    spl_permissioned_transfer::{
        get_extra_account_metas_address,
        instruction::PermissionedTransferInstruction,
        pod::PodAccountMeta,
        state::ExtraAccountMetas,
        tlv::{TlvState, TlvStateBorrowed, TlvType},
    },
    spl_token_2022::{
        error::TokenError,
        extension::{permissioned_transfer::PermissionedTransfer, BaseStateWithExtensions},
        instruction,
        processor::Processor,
    },
    spl_token_client::token::{ExtensionInitializationParams, TokenError as TokenClientError},
    std::{convert::TryInto, sync::Arc},
};

/// Test program to validate a transfer, conforms to permssioned-transfer
/// `validate`
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = PermissionedTransferInstruction::unpack(input)?;
    let _amount = match instruction {
        PermissionedTransferInstruction::Validate { amount } => amount,
        _ => return Err(ProgramError::InvalidInstructionData),
    };
    let account_info_iter = &mut accounts.iter();

    let _source_account_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let _destination_account_info = next_account_info(account_info_iter)?;
    let _authority_info = next_account_info(account_info_iter)?;
    let extra_account_metas_info = next_account_info(account_info_iter)?;

    // Only check that the correct pda and account are provided
    let expected_validation_address = get_extra_account_metas_address(mint_info.key, program_id);
    if expected_validation_address != *extra_account_metas_info.key {
        return Err(ProgramError::InvalidSeeds);
    }

    let data = extra_account_metas_info.try_borrow_data()?;
    let state = TlvStateBorrowed::unpack(&data).unwrap();
    let bytes = state.get_bytes::<ExtraAccountMetas>()?;
    let extra_account_metas = ExtraAccountMetas::unpack(bytes)?;

    // if incorrect number of accounts is provided, error
    let extra_account_infos = account_info_iter.as_slice();
    let account_metas = extra_account_metas.data();
    if extra_account_infos.len() != account_metas.len() {
        return Err(ProgramError::InvalidInstructionData);
    }

    // Let's assume that they're provided in the correct order
    for (i, account_info) in extra_account_infos.iter().enumerate() {
        if &account_metas[i] != account_info {
            return Err(ProgramError::InvalidInstructionData);
        }
    }

    Ok(())
}

/// Test program to fail transfer validation, conforms to permssioned-transfer
/// `validate`
pub fn process_instruction_fail(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _input: &[u8],
) -> ProgramResult {
    Err(ProgramError::InvalidInstructionData)
}

async fn setup_accounts(token_context: &TokenContext, amount: u64) -> (Pubkey, Pubkey) {
    let alice_account = Keypair::new();
    token_context
        .token
        .create_auxiliary_token_account(&alice_account, &token_context.alice.pubkey())
        .await
        .unwrap();
    let alice_account = alice_account.pubkey();
    let bob_account = Keypair::new();
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
        "my_permissioned_transfer",
        *program_id,
        processor!(process_instruction),
    );
    let account_metas = vec![
        PodAccountMeta {
            pubkey: Pubkey::new_unique(),
            is_signer: false.into(),
            is_writable: false.into(),
        },
        PodAccountMeta {
            pubkey: Pubkey::new_unique(),
            is_signer: false.into(),
            is_writable: false.into(),
        },
    ];
    let mut tlv_data = vec![];
    tlv_data.extend_from_slice(&(account_metas.len() as u16).to_le_bytes());
    account_metas
        .iter()
        .for_each(|m| tlv_data.extend_from_slice(bytemuck::bytes_of(m)));
    let mut data = vec![];
    data.extend_from_slice(ExtraAccountMetas::TYPE.as_ref());
    data.extend_from_slice(&(tlv_data.len() as u32).to_le_bytes());
    data.extend_from_slice(&tlv_data);
    let validation_address = get_extra_account_metas_address(&mint.pubkey(), program_id);
    program_test.add_account(
        validation_address,
        Account {
            lamports: 1_000_000_000, // a lot, just to be safe
            data,
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
            vec![ExtensionInitializationParams::PermissionedTransfer {
                authority: Some(*authority),
                permissioned_transfer_program_id: Some(*program_id),
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
    let extension = state.get_extension::<PermissionedTransfer>().unwrap();
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
            instruction::AuthorityType::PermissionedTransfer,
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
            instruction::AuthorityType::PermissionedTransfer,
            &[&authority],
        )
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<PermissionedTransfer>().unwrap();
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
            instruction::AuthorityType::PermissionedTransfer,
            &[&new_authority],
        )
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<PermissionedTransfer>().unwrap();
    assert_eq!(extension.authority, None.try_into().unwrap(),);

    // fail set again
    let err = token
        .set_authority(
            token.get_address(),
            &new_authority.pubkey(),
            Some(&authority.pubkey()),
            instruction::AuthorityType::PermissionedTransfer,
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
async fn update_permissioned_transfer_program_id() {
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
        .update_permissioned_transfer_program_id(&wrong.pubkey(), Some(new_program_id), &[&wrong])
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
        .update_permissioned_transfer_program_id(
            &authority.pubkey(),
            Some(new_program_id),
            &[&authority],
        )
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<PermissionedTransfer>().unwrap();
    assert_eq!(
        extension.program_id,
        Some(new_program_id).try_into().unwrap(),
    );

    // set to none
    token
        .update_permissioned_transfer_program_id(&authority.pubkey(), None, &[&authority])
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<PermissionedTransfer>().unwrap();
    assert_eq!(extension.program_id, None.try_into().unwrap(),);
}

#[tokio::test]
async fn success_transfer() {
    let authority = Keypair::new();
    let program_id = Pubkey::new_unique();
    let mint_keypair = Keypair::new();
    let token_context = setup(mint_keypair, &program_id, &authority.pubkey()).await;
    let amount = 10;
    let (alice_account, bob_account) = setup_accounts(&token_context, amount).await;

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
async fn fail_permissioned_transfer_program() {
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
        "my_permissioned_transfer",
        program_id,
        processor!(process_instruction_fail),
    );
    let mut tlv_data = vec![];
    tlv_data.extend_from_slice(&0u16.to_le_bytes());
    let mut data = vec![];
    data.extend_from_slice(ExtraAccountMetas::TYPE.as_ref());
    data.extend_from_slice(&(tlv_data.len() as u32).to_le_bytes());
    data.extend_from_slice(&tlv_data);
    let validation_address = get_extra_account_metas_address(&mint.pubkey(), &program_id);
    program_test.add_account(
        validation_address,
        Account {
            lamports: 1_000_000_000, // a lot, just to be safe
            data,
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
            vec![ExtensionInitializationParams::PermissionedTransfer {
                authority: Some(authority),
                permissioned_transfer_program_id: Some(program_id),
            }],
            None,
        )
        .await
        .unwrap();
    let token_context = context.token_context.take().unwrap();

    let amount = 10;
    let (alice_account, bob_account) = setup_accounts(&token_context, amount).await;

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
