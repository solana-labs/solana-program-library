// Mark this test as SBF-only due to current `ProgramTest` limitations when CPIing into the system program
#![cfg(feature = "test-sbf")]

use {
    solana_program_test::{processor, tokio, ProgramTest},
    solana_sdk::{
        account::Account as SolanaAccount,
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        instruction::{AccountMeta, InstructionError},
        program_option::COption,
        pubkey::Pubkey,
        signature::Signer,
        signer::keypair::Keypair,
        system_instruction, sysvar,
        transaction::{Transaction, TransactionError},
    },
    spl_tlv_account_resolution::state::ExtraAccountMetas,
    spl_token_2022::{
        extension::{transfer_hook::TransferHookAccount, ExtensionType, StateWithExtensionsMut},
        state::{Account, AccountState, Mint},
    },
    spl_transfer_hook_interface::{
        error::TransferHookError,
        get_extra_account_metas_address,
        instruction::{execute_with_extra_account_metas, initialize_extra_account_metas},
        onchain,
    },
};

fn setup(program_id: &Pubkey) -> ProgramTest {
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

    program_test
}

#[allow(clippy::too_many_arguments)]
fn setup_token_accounts(
    program_test: &mut ProgramTest,
    program_id: &Pubkey,
    mint_address: &Pubkey,
    mint_authority: &Pubkey,
    source: &Pubkey,
    destination: &Pubkey,
    owner: &Pubkey,
    decimals: u8,
    transferring: bool,
) {
    // add mint, source, and destination accounts by hand to always force
    // the "transferring" flag to true
    let mint_size = ExtensionType::try_get_account_len::<Mint>(&[]).unwrap();
    let mut mint_data = vec![0; mint_size];
    let mut state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut mint_data).unwrap();
    let token_amount = 1_000_000_000_000;
    state.base = Mint {
        mint_authority: COption::Some(*mint_authority),
        supply: token_amount,
        decimals,
        is_initialized: true,
        freeze_authority: COption::None,
    };
    state.pack_base();
    program_test.add_account(
        *mint_address,
        SolanaAccount {
            lamports: 1_000_000_000,
            data: mint_data,
            owner: *program_id,
            ..SolanaAccount::default()
        },
    );

    let account_size =
        ExtensionType::try_get_account_len::<Account>(&[ExtensionType::TransferHookAccount])
            .unwrap();
    let mut account_data = vec![0; account_size];
    let mut state =
        StateWithExtensionsMut::<Account>::unpack_uninitialized(&mut account_data).unwrap();
    let extension = state.init_extension::<TransferHookAccount>(true).unwrap();
    extension.transferring = transferring.into();
    let token_amount = 1_000_000_000_000;
    state.base = Account {
        mint: *mint_address,
        owner: *owner,
        amount: token_amount,
        delegate: COption::None,
        state: AccountState::Initialized,
        is_native: COption::None,
        delegated_amount: 0,
        close_authority: COption::None,
    };
    state.pack_base();
    state.init_account_type().unwrap();

    program_test.add_account(
        *source,
        SolanaAccount {
            lamports: 1_000_000_000,
            data: account_data.clone(),
            owner: *program_id,
            ..SolanaAccount::default()
        },
    );
    program_test.add_account(
        *destination,
        SolanaAccount {
            lamports: 1_000_000_000,
            data: account_data,
            owner: *program_id,
            ..SolanaAccount::default()
        },
    );
}

#[tokio::test]
async fn success_execute() {
    let program_id = Pubkey::new_unique();
    let mut program_test = setup(&program_id);

    let token_program_id = spl_token_2022::id();
    let wallet = Keypair::new();
    let mint_address = Pubkey::new_unique();
    let mint_authority = Keypair::new();
    let mint_authority_pubkey = mint_authority.pubkey();
    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let decimals = 2;

    setup_token_accounts(
        &mut program_test,
        &token_program_id,
        &mint_address,
        &mint_authority_pubkey,
        &source,
        &destination,
        &wallet.pubkey(),
        decimals,
        true,
    );

    let extra_account_metas = get_extra_account_metas_address(&mint_address, &program_id);

    let extra_account_pubkeys = [
        AccountMeta::new_readonly(sysvar::instructions::id(), false),
        AccountMeta::new_readonly(mint_authority_pubkey, true),
        AccountMeta::new(extra_account_metas, false),
    ];
    let mut context = program_test.start_with_context().await;
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
                &mint_address,
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
                &mint_address,
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
                &mint_address,
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
                &mint_address,
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
                &mint_address,
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
    let mut program_test = setup(&program_id);

    let token_program_id = spl_token_2022::id();
    let wallet = Keypair::new();
    let mint_address = Pubkey::new_unique();
    let mint_authority = Keypair::new();
    let mint_authority_pubkey = mint_authority.pubkey();
    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let decimals = 2;
    setup_token_accounts(
        &mut program_test,
        &token_program_id,
        &mint_address,
        &mint_authority_pubkey,
        &source,
        &destination,
        &wallet.pubkey(),
        decimals,
        true,
    );

    // wrong derivation
    let extra_account_metas = get_extra_account_metas_address(&program_id, &mint_address);

    let mut context = program_test.start_with_context().await;
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
                &mint_address,
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
    onchain::invoke_execute(
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
    let mut program_test = setup(&hook_program_id);
    let program_id = Pubkey::new_unique();
    program_test.add_program(
        "test_cpi_program",
        program_id,
        processor!(process_instruction),
    );

    let token_program_id = spl_token_2022::id();
    let wallet = Keypair::new();
    let mint_address = Pubkey::new_unique();
    let mint_authority = Keypair::new();
    let mint_authority_pubkey = mint_authority.pubkey();
    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let decimals = 2;

    setup_token_accounts(
        &mut program_test,
        &token_program_id,
        &mint_address,
        &mint_authority_pubkey,
        &source,
        &destination,
        &wallet.pubkey(),
        decimals,
        true,
    );

    let extra_account_metas = get_extra_account_metas_address(&mint_address, &hook_program_id);

    let writable_pubkey = Pubkey::new_unique();
    let extra_account_pubkeys = [
        AccountMeta::new_readonly(sysvar::instructions::id(), false),
        AccountMeta::new_readonly(mint_authority_pubkey, true),
        AccountMeta::new(writable_pubkey, false),
    ];
    let mut context = program_test.start_with_context().await;
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
                &mint_address,
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
        &mint_address,
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

#[tokio::test]
async fn fail_without_transferring_flag() {
    let program_id = Pubkey::new_unique();
    let mut program_test = setup(&program_id);

    let token_program_id = spl_token_2022::id();
    let wallet = Keypair::new();
    let mint_address = Pubkey::new_unique();
    let mint_authority = Keypair::new();
    let mint_authority_pubkey = mint_authority.pubkey();
    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let decimals = 2;
    setup_token_accounts(
        &mut program_test,
        &token_program_id,
        &mint_address,
        &mint_authority_pubkey,
        &source,
        &destination,
        &wallet.pubkey(),
        decimals,
        false,
    );

    let extra_account_metas = get_extra_account_metas_address(&mint_address, &program_id);
    let extra_account_pubkeys = [];
    let mut context = program_test.start_with_context().await;
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
                &mint_address,
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
    let transaction = Transaction::new_signed_with_payer(
        &[execute_with_extra_account_metas(
            &program_id,
            &source,
            &mint_address,
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
            InstructionError::Custom(TransferHookError::ProgramCalledOutsideOfTransfer as u32)
        )
    );
}
