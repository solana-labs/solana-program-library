// Mark this test as SBF-only due to current `ProgramTest` limitations when CPIing into the system program
#![cfg(feature = "test-sbf")]

use {
    solana_program_test::{
        processor,
        tokio::{self, sync::Mutex},
        ProgramTest, ProgramTestContext,
    },
    solana_sdk::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        instruction::{AccountMeta, InstructionError},
        pubkey::Pubkey,
        signature::Signer,
        signer::keypair::Keypair,
        system_instruction, sysvar,
        transaction::{Transaction, TransactionError},
    },
    spl_tlv_account_resolution::state::ExtraAccountMetas,
    spl_token_client::{
        client::{
            ProgramBanksClient, ProgramBanksClientProcessTransaction, ProgramClient,
            SendTransaction,
        },
        token::Token,
    },
    spl_transfer_hook_interface::{
        error::TransferHookError,
        get_extra_account_metas_address,
        instruction::{execute_with_extra_account_metas, initialize_extra_account_metas},
        invoke,
    },
    std::sync::Arc,
};

fn keypair_clone(kp: &Keypair) -> Keypair {
    Keypair::from_bytes(&kp.to_bytes()).expect("failed to copy keypair")
}

async fn setup(
    program_id: &Pubkey,
) -> (
    Arc<Mutex<ProgramTestContext>>,
    Arc<dyn ProgramClient<ProgramBanksClientProcessTransaction>>,
    Arc<Keypair>,
) {
    let mut program_test = ProgramTest::new(
        "spl_transfer_hook_example",
        *program_id,
        processor!(spl_transfer_hook_example::processor::process),
    );

    program_test.prefer_bpf(false); // simplicity in the build

    program_test.add_program(
        "spl_token_2022",
        spl_token_2022::id(),
        processor!(spl_token_2022::processor::Processor::process),
    );

    let context = program_test.start_with_context().await;
    let payer = Arc::new(keypair_clone(&context.payer));
    let context = Arc::new(Mutex::new(context));

    let client: Arc<dyn ProgramClient<ProgramBanksClientProcessTransaction>> =
        Arc::new(ProgramBanksClient::new_from_context(
            Arc::clone(&context),
            ProgramBanksClientProcessTransaction,
        ));
    (context, client, payer)
}

async fn setup_mint<T: SendTransaction>(
    program_id: &Pubkey,
    mint_authority: &Pubkey,
    decimals: u8,
    payer: Arc<Keypair>,
    client: Arc<dyn ProgramClient<T>>,
) -> Token<T> {
    let mint_account = Keypair::new();
    let token = Token::new(
        client,
        program_id,
        &mint_account.pubkey(),
        Some(decimals),
        payer,
    );
    token
        .create_mint(mint_authority, None, vec![], &[&mint_account])
        .await
        .unwrap();
    token
}

#[tokio::test]
async fn success() {
    let program_id = Pubkey::new_unique();
    let (context, client, payer) = setup(&program_id).await;

    let token_program_id = spl_token_2022::id();
    let wallet = Keypair::new();
    let mint_authority = Keypair::new();
    let mint_authority_pubkey = mint_authority.pubkey();

    let decimals = 2;
    let token = setup_mint(
        &token_program_id,
        &mint_authority_pubkey,
        decimals,
        payer.clone(),
        client.clone(),
    )
    .await;

    let extra_account_metas = get_extra_account_metas_address(token.get_address(), &program_id);

    token
        .create_associated_token_account(&wallet.pubkey())
        .await
        .unwrap();
    let source = token.get_associated_token_address(&wallet.pubkey());
    let token_amount = 1_000_000_000_000;
    token
        .mint_to(
            &source,
            &mint_authority_pubkey,
            token_amount,
            &[&mint_authority],
        )
        .await
        .unwrap();

    let destination = Keypair::new();
    token
        .create_auxiliary_token_account(&destination, &wallet.pubkey())
        .await
        .unwrap();
    let destination = destination.pubkey();

    let extra_account_pubkeys = [
        AccountMeta::new_readonly(sysvar::instructions::id(), false),
        AccountMeta::new_readonly(mint_authority_pubkey, true),
        AccountMeta::new(extra_account_metas, false),
    ];
    let mut context = context.lock().await;
    let rent = context.banks_client.get_rent().await.unwrap();
    let rent_lamports =
        rent.minimum_balance(ExtraAccountMetas::size_of(extra_account_pubkeys.len()).unwrap());
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::transfer(
                &context.payer.pubkey(),
                &extra_account_metas,
                rent_lamports,
            ),
            initialize_extra_account_metas(
                &program_id,
                &extra_account_metas,
                token.get_address(),
                &mint_authority_pubkey,
                &extra_account_pubkeys,
            ),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &mint_authority],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // fail with missing account
    {
        let transaction = Transaction::new_signed_with_payer(
            &[execute_with_extra_account_metas(
                &program_id,
                &source,
                token.get_address(),
                &destination,
                &wallet.pubkey(),
                &extra_account_metas,
                &extra_account_pubkeys[..2],
                0,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, &mint_authority],
            context.last_blockhash,
        );
        let error = context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap();
        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TransferHookError::IncorrectAccount as u32),
            )
        );
    }

    // fail with wrong account
    {
        let extra_account_pubkeys = [
            AccountMeta::new_readonly(sysvar::instructions::id(), false),
            AccountMeta::new_readonly(mint_authority_pubkey, true),
            AccountMeta::new(wallet.pubkey(), false),
        ];
        let transaction = Transaction::new_signed_with_payer(
            &[execute_with_extra_account_metas(
                &program_id,
                &source,
                token.get_address(),
                &destination,
                &wallet.pubkey(),
                &extra_account_metas,
                &extra_account_pubkeys,
                0,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, &mint_authority],
            context.last_blockhash,
        );
        let error = context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap();
        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TransferHookError::IncorrectAccount as u32),
            )
        );
    }

    // fail with not signer
    {
        let extra_account_pubkeys = [
            AccountMeta::new_readonly(sysvar::instructions::id(), false),
            AccountMeta::new_readonly(mint_authority_pubkey, false),
            AccountMeta::new(extra_account_metas, false),
        ];
        let transaction = Transaction::new_signed_with_payer(
            &[execute_with_extra_account_metas(
                &program_id,
                &source,
                token.get_address(),
                &destination,
                &wallet.pubkey(),
                &extra_account_metas,
                &extra_account_pubkeys,
                0,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );
        let error = context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap();
        assert_eq!(
            error,
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TransferHookError::IncorrectAccount as u32),
            )
        );
    }

    // success with correct params
    {
        let transaction = Transaction::new_signed_with_payer(
            &[execute_with_extra_account_metas(
                &program_id,
                &source,
                token.get_address(),
                &destination,
                &wallet.pubkey(),
                &extra_account_metas,
                &extra_account_pubkeys,
                0,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, &mint_authority],
            context.last_blockhash,
        );
        context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn fail_incorrect_derivation() {
    let program_id = Pubkey::new_unique();
    let (context, client, payer) = setup(&program_id).await;

    let token_program_id = spl_token_2022::id();
    let mint_authority = Keypair::new();
    let mint_authority_pubkey = mint_authority.pubkey();

    let decimals = 2;
    let token = setup_mint(
        &token_program_id,
        &mint_authority_pubkey,
        decimals,
        payer.clone(),
        client.clone(),
    )
    .await;

    // wrong derivation
    let extra_account_metas = get_extra_account_metas_address(&program_id, token.get_address());

    let mut context = context.lock().await;
    let rent = context.banks_client.get_rent().await.unwrap();
    let rent_lamports = rent.minimum_balance(ExtraAccountMetas::size_of(0).unwrap());

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::transfer(
                &context.payer.pubkey(),
                &extra_account_metas,
                rent_lamports,
            ),
            initialize_extra_account_metas(
                &program_id,
                &extra_account_metas,
                token.get_address(),
                &mint_authority_pubkey,
                &[],
            ),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &mint_authority],
        context.last_blockhash,
    );
    let error = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err()
        .unwrap();
    assert_eq!(
        error,
        TransactionError::InstructionError(1, InstructionError::InvalidSeeds)
    );
}

/// Test program to CPI into default transfer-hook-interface program
pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    _input: &[u8],
) -> ProgramResult {
    invoke::execute(
        accounts[0].key,
        accounts[1].clone(),
        accounts[2].clone(),
        accounts[3].clone(),
        accounts[4].clone(),
        &accounts[4..],
        0,
    )
}

#[tokio::test]
async fn success_on_chain_invoke() {
    let hook_program_id = Pubkey::new_unique();
    let mut program_test = ProgramTest::new(
        "spl_transfer_hook_example",
        hook_program_id,
        processor!(spl_transfer_hook_example::processor::process),
    );
    program_test.prefer_bpf(false);
    program_test.add_program(
        "spl_token_2022",
        spl_token_2022::id(),
        processor!(spl_token_2022::processor::Processor::process),
    );

    let program_id = Pubkey::new_unique();
    program_test.add_program(
        "test_cpi_program",
        program_id,
        processor!(process_instruction),
    );

    let context = program_test.start_with_context().await;
    let payer = Arc::new(keypair_clone(&context.payer));
    let context = Arc::new(Mutex::new(context));

    let client: Arc<dyn ProgramClient<ProgramBanksClientProcessTransaction>> =
        Arc::new(ProgramBanksClient::new_from_context(
            Arc::clone(&context),
            ProgramBanksClientProcessTransaction,
        ));

    let token_program_id = spl_token_2022::id();
    let wallet = Keypair::new();
    let mint_authority = Keypair::new();
    let mint_authority_pubkey = mint_authority.pubkey();

    let decimals = 2;
    let token = setup_mint(
        &token_program_id,
        &mint_authority_pubkey,
        decimals,
        payer.clone(),
        client.clone(),
    )
    .await;

    let extra_account_metas =
        get_extra_account_metas_address(token.get_address(), &hook_program_id);

    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let writable_pubkey = Pubkey::new_unique();

    let extra_account_pubkeys = [
        AccountMeta::new_readonly(sysvar::instructions::id(), false),
        AccountMeta::new_readonly(mint_authority_pubkey, true),
        AccountMeta::new(writable_pubkey, false),
    ];
    let mut context = context.lock().await;
    let rent = context.banks_client.get_rent().await.unwrap();
    let rent_lamports =
        rent.minimum_balance(ExtraAccountMetas::size_of(extra_account_pubkeys.len()).unwrap());
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::transfer(
                &context.payer.pubkey(),
                &extra_account_metas,
                rent_lamports,
            ),
            initialize_extra_account_metas(
                &hook_program_id,
                &extra_account_metas,
                token.get_address(),
                &mint_authority_pubkey,
                &extra_account_pubkeys,
            ),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &mint_authority],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // easier to hack this up!
    let mut test_instruction = execute_with_extra_account_metas(
        &program_id,
        &source,
        token.get_address(),
        &destination,
        &wallet.pubkey(),
        &extra_account_metas,
        &extra_account_pubkeys,
        0,
    );
    test_instruction
        .accounts
        .insert(0, AccountMeta::new_readonly(hook_program_id, false));
    let transaction = Transaction::new_signed_with_payer(
        &[test_instruction],
        Some(&context.payer.pubkey()),
        &[&context.payer, &mint_authority],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}
