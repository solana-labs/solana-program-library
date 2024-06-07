// Mark this test as SBF-only due to current `ProgramTest` limitations when
// CPIing into the system program
#![cfg(feature = "test-sbf")]

use {
    solana_program_test::{processor, tokio, ProgramTest},
    solana_sdk::{
        account::Account as SolanaAccount,
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        instruction::{AccountMeta, InstructionError},
        program_error::ProgramError,
        program_option::COption,
        pubkey::Pubkey,
        signature::Signer,
        signer::keypair::Keypair,
        system_instruction, sysvar,
        transaction::{Transaction, TransactionError},
    },
    spl_tlv_account_resolution::{
        account::ExtraAccountMeta, error::AccountResolutionError, seeds::Seed,
        state::ExtraAccountMetaList,
    },
    spl_token_2022::{
        extension::{
            transfer_hook::TransferHookAccount, BaseStateWithExtensionsMut, ExtensionType,
            StateWithExtensionsMut,
        },
        state::{Account, AccountState, Mint},
    },
    spl_transfer_hook_interface::{
        error::TransferHookError,
        get_extra_account_metas_address,
        instruction::{
            execute_with_extra_account_metas, initialize_extra_account_meta_list,
            update_extra_account_meta_list,
        },
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
    let mint_size = ExtensionType::try_calculate_account_len::<Mint>(&[]).unwrap();
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
        ExtensionType::try_calculate_account_len::<Account>(&[ExtensionType::TransferHookAccount])
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
    let mint_address = spl_transfer_hook_example::mint::id();
    let mint_authority = Keypair::new();
    let mint_authority_pubkey = mint_authority.pubkey();
    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let decimals = 2;
    let amount = 0u64;

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

    let extra_account_metas_address = get_extra_account_metas_address(&mint_address, &program_id);

    let writable_pubkey = Pubkey::new_unique();

    let init_extra_account_metas = [
        ExtraAccountMeta::new_with_pubkey(&sysvar::instructions::id(), false, false).unwrap(),
        ExtraAccountMeta::new_with_pubkey(&mint_authority_pubkey, true, false).unwrap(),
        ExtraAccountMeta::new_with_seeds(
            &[
                Seed::Literal {
                    bytes: b"seed-prefix".to_vec(),
                },
                Seed::AccountKey { index: 0 },
            ],
            false,
            true,
        )
        .unwrap(),
        ExtraAccountMeta::new_with_seeds(
            &[
                Seed::InstructionData {
                    index: 8,  // After instruction discriminator
                    length: 8, // `u64` (amount)
                },
                Seed::AccountKey { index: 2 },
            ],
            false,
            true,
        )
        .unwrap(),
        ExtraAccountMeta::new_with_pubkey(&writable_pubkey, false, true).unwrap(),
    ];

    let extra_pda_1 = Pubkey::find_program_address(
        &[
            b"seed-prefix",  // Literal prefix
            source.as_ref(), // Account at index 0
        ],
        &program_id,
    )
    .0;
    let extra_pda_2 = Pubkey::find_program_address(
        &[
            &amount.to_le_bytes(), // Instruction data bytes 8 to 16
            destination.as_ref(),  // Account at index 2
        ],
        &program_id,
    )
    .0;

    let extra_account_metas = [
        AccountMeta::new_readonly(sysvar::instructions::id(), false),
        AccountMeta::new_readonly(mint_authority_pubkey, true),
        AccountMeta::new(extra_pda_1, false),
        AccountMeta::new(extra_pda_2, false),
        AccountMeta::new(writable_pubkey, false),
    ];

    let mut context = program_test.start_with_context().await;
    let rent = context.banks_client.get_rent().await.unwrap();
    let rent_lamports = rent
        .minimum_balance(ExtraAccountMetaList::size_of(init_extra_account_metas.len()).unwrap());
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::transfer(
                &context.payer.pubkey(),
                &extra_account_metas_address,
                rent_lamports,
            ),
            initialize_extra_account_meta_list(
                &program_id,
                &extra_account_metas_address,
                &mint_address,
                &mint_authority_pubkey,
                &init_extra_account_metas,
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
                &extra_account_metas_address,
                &extra_account_metas[..2],
                amount,
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
                InstructionError::Custom(AccountResolutionError::IncorrectAccount as u32),
            )
        );
    }

    // fail with wrong account
    {
        let extra_account_metas = [
            AccountMeta::new_readonly(sysvar::instructions::id(), false),
            AccountMeta::new_readonly(mint_authority_pubkey, true),
            AccountMeta::new(extra_pda_1, false),
            AccountMeta::new(extra_pda_2, false),
            AccountMeta::new(Pubkey::new_unique(), false),
        ];
        let transaction = Transaction::new_signed_with_payer(
            &[execute_with_extra_account_metas(
                &program_id,
                &source,
                &mint_address,
                &destination,
                &wallet.pubkey(),
                &extra_account_metas_address,
                &extra_account_metas,
                amount,
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
                InstructionError::Custom(AccountResolutionError::IncorrectAccount as u32),
            )
        );
    }

    // fail with wrong PDA
    let wrong_pda_2 = Pubkey::find_program_address(
        &[
            &99u64.to_le_bytes(), // Wrong data
            destination.as_ref(),
        ],
        &program_id,
    )
    .0;
    {
        let extra_account_metas = [
            AccountMeta::new_readonly(sysvar::instructions::id(), false),
            AccountMeta::new_readonly(mint_authority_pubkey, true),
            AccountMeta::new(extra_pda_1, false),
            AccountMeta::new(wrong_pda_2, false),
            AccountMeta::new(writable_pubkey, false),
        ];
        let transaction = Transaction::new_signed_with_payer(
            &[execute_with_extra_account_metas(
                &program_id,
                &source,
                &mint_address,
                &destination,
                &wallet.pubkey(),
                &extra_account_metas_address,
                &extra_account_metas,
                amount,
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
                InstructionError::Custom(AccountResolutionError::IncorrectAccount as u32),
            )
        );
    }

    // fail with not signer
    {
        let extra_account_metas = [
            AccountMeta::new_readonly(sysvar::instructions::id(), false),
            AccountMeta::new_readonly(mint_authority_pubkey, false),
            AccountMeta::new(extra_pda_1, false),
            AccountMeta::new(extra_pda_2, false),
            AccountMeta::new(writable_pubkey, false),
        ];
        let transaction = Transaction::new_signed_with_payer(
            &[execute_with_extra_account_metas(
                &program_id,
                &source,
                &mint_address,
                &destination,
                &wallet.pubkey(),
                &extra_account_metas_address,
                &extra_account_metas,
                amount,
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
                InstructionError::Custom(AccountResolutionError::IncorrectAccount as u32),
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
                &extra_account_metas_address,
                &extra_account_metas,
                amount,
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
    let mint_address = spl_transfer_hook_example::mint::id();
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
    let rent_lamports = rent.minimum_balance(ExtraAccountMetaList::size_of(0).unwrap());

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::transfer(
                &context.payer.pubkey(),
                &extra_account_metas,
                rent_lamports,
            ),
            initialize_extra_account_meta_list(
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

#[tokio::test]
async fn fail_incorrect_mint() {
    let program_id = Pubkey::new_unique();
    let mut program_test = setup(&program_id);

    let token_program_id = spl_token_2022::id();
    let wallet = Keypair::new();
    // wrong mint, only `spl_transfer_hook_example::mint::id()` allowed
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

    let mut context = program_test.start_with_context().await;
    let rent = context.banks_client.get_rent().await.unwrap();
    let rent_lamports = rent.minimum_balance(ExtraAccountMetaList::size_of(0).unwrap());

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::transfer(
                &context.payer.pubkey(),
                &extra_account_metas,
                rent_lamports,
            ),
            initialize_extra_account_meta_list(
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
        TransactionError::InstructionError(1, InstructionError::InvalidArgument)
    );
}

/// Test program to CPI into default transfer-hook-interface program
pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let amount = input
        .get(8..16)
        .and_then(|slice| slice.try_into().ok())
        .map(u64::from_le_bytes)
        .ok_or(ProgramError::InvalidInstructionData)?;
    onchain::invoke_execute(
        accounts[0].key,
        accounts[1].clone(),
        accounts[2].clone(),
        accounts[3].clone(),
        accounts[4].clone(),
        &accounts[5..],
        amount,
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
    let mint_address = spl_transfer_hook_example::mint::id();
    let mint_authority = Keypair::new();
    let mint_authority_pubkey = mint_authority.pubkey();
    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let decimals = 2;
    let amount = 0u64;

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

    let extra_account_metas_address =
        get_extra_account_metas_address(&mint_address, &hook_program_id);
    let writable_pubkey = Pubkey::new_unique();

    let init_extra_account_metas = [
        ExtraAccountMeta::new_with_pubkey(&sysvar::instructions::id(), false, false).unwrap(),
        ExtraAccountMeta::new_with_pubkey(&mint_authority_pubkey, true, false).unwrap(),
        ExtraAccountMeta::new_with_seeds(
            &[
                Seed::Literal {
                    bytes: b"seed-prefix".to_vec(),
                },
                Seed::AccountKey { index: 0 },
            ],
            false,
            true,
        )
        .unwrap(),
        ExtraAccountMeta::new_with_seeds(
            &[
                Seed::InstructionData {
                    index: 8,  // After instruction discriminator
                    length: 8, // `u64` (amount)
                },
                Seed::AccountKey { index: 2 },
            ],
            false,
            true,
        )
        .unwrap(),
        ExtraAccountMeta::new_with_pubkey(&writable_pubkey, false, true).unwrap(),
    ];

    let extra_pda_1 = Pubkey::find_program_address(
        &[
            b"seed-prefix",  // Literal prefix
            source.as_ref(), // Account at index 0
        ],
        &hook_program_id,
    )
    .0;
    let extra_pda_2 = Pubkey::find_program_address(
        &[
            &amount.to_le_bytes(), // Instruction data bytes 8 to 16
            destination.as_ref(),  // Account at index 2
        ],
        &hook_program_id,
    )
    .0;

    let extra_account_metas = [
        AccountMeta::new_readonly(sysvar::instructions::id(), false),
        AccountMeta::new_readonly(mint_authority_pubkey, true),
        AccountMeta::new(extra_pda_1, false),
        AccountMeta::new(extra_pda_2, false),
        AccountMeta::new(writable_pubkey, false),
    ];

    let mut context = program_test.start_with_context().await;
    let rent = context.banks_client.get_rent().await.unwrap();
    let rent_lamports = rent
        .minimum_balance(ExtraAccountMetaList::size_of(init_extra_account_metas.len()).unwrap());
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::transfer(
                &context.payer.pubkey(),
                &extra_account_metas_address,
                rent_lamports,
            ),
            initialize_extra_account_meta_list(
                &hook_program_id,
                &extra_account_metas_address,
                &mint_address,
                &mint_authority_pubkey,
                &init_extra_account_metas,
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
        &extra_account_metas_address,
        &extra_account_metas,
        amount,
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
    let mint_address = spl_transfer_hook_example::mint::id();
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

    let extra_account_metas_address = get_extra_account_metas_address(&mint_address, &program_id);
    let extra_account_metas = [];
    let init_extra_account_metas = [];
    let mut context = program_test.start_with_context().await;
    let rent = context.banks_client.get_rent().await.unwrap();
    let rent_lamports = rent
        .minimum_balance(ExtraAccountMetaList::size_of(init_extra_account_metas.len()).unwrap());
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::transfer(
                &context.payer.pubkey(),
                &extra_account_metas_address,
                rent_lamports,
            ),
            initialize_extra_account_meta_list(
                &program_id,
                &extra_account_metas_address,
                &mint_address,
                &mint_authority_pubkey,
                &init_extra_account_metas,
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
            &extra_account_metas_address,
            &extra_account_metas,
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

#[tokio::test]
async fn success_on_chain_invoke_with_updated_extra_account_metas() {
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
    let mint_address = spl_transfer_hook_example::mint::id();
    let mint_authority = Keypair::new();
    let mint_authority_pubkey = mint_authority.pubkey();
    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let decimals = 2;
    let amount = 0u64;

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

    let extra_account_metas_address =
        get_extra_account_metas_address(&mint_address, &hook_program_id);
    let writable_pubkey = Pubkey::new_unique();

    // Create an initial account metas list
    let init_extra_account_metas = [
        ExtraAccountMeta::new_with_pubkey(&sysvar::instructions::id(), false, false).unwrap(),
        ExtraAccountMeta::new_with_pubkey(&mint_authority_pubkey, true, false).unwrap(),
        ExtraAccountMeta::new_with_seeds(
            &[
                Seed::Literal {
                    bytes: b"init-seed-prefix".to_vec(),
                },
                Seed::AccountKey { index: 0 },
            ],
            false,
            true,
        )
        .unwrap(),
        ExtraAccountMeta::new_with_seeds(
            &[
                Seed::InstructionData {
                    index: 8,  // After instruction discriminator
                    length: 8, // `u64` (amount)
                },
                Seed::AccountKey { index: 2 },
            ],
            false,
            true,
        )
        .unwrap(),
        ExtraAccountMeta::new_with_pubkey(&writable_pubkey, false, true).unwrap(),
    ];

    let mut context = program_test.start_with_context().await;
    let rent = context.banks_client.get_rent().await.unwrap();
    let rent_lamports = rent
        .minimum_balance(ExtraAccountMetaList::size_of(init_extra_account_metas.len()).unwrap());
    let init_transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::transfer(
                &context.payer.pubkey(),
                &extra_account_metas_address,
                rent_lamports,
            ),
            initialize_extra_account_meta_list(
                &hook_program_id,
                &extra_account_metas_address,
                &mint_address,
                &mint_authority_pubkey,
                &init_extra_account_metas,
            ),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &mint_authority],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(init_transaction)
        .await
        .unwrap();

    // Create an updated account metas list
    let updated_extra_account_metas = [
        ExtraAccountMeta::new_with_pubkey(&sysvar::instructions::id(), false, false).unwrap(),
        ExtraAccountMeta::new_with_pubkey(&mint_authority_pubkey, true, false).unwrap(),
        ExtraAccountMeta::new_with_seeds(
            &[
                Seed::Literal {
                    bytes: b"updated-seed-prefix".to_vec(),
                },
                Seed::AccountKey { index: 0 },
            ],
            false,
            true,
        )
        .unwrap(),
        ExtraAccountMeta::new_with_seeds(
            &[
                Seed::InstructionData {
                    index: 8,  // After instruction discriminator
                    length: 8, // `u64` (amount)
                },
                Seed::AccountKey { index: 2 },
            ],
            false,
            true,
        )
        .unwrap(),
        ExtraAccountMeta::new_with_pubkey(&writable_pubkey, false, true).unwrap(),
    ];

    let rent = context.banks_client.get_rent().await.unwrap();
    let rent_lamports = rent
        .minimum_balance(ExtraAccountMetaList::size_of(updated_extra_account_metas.len()).unwrap());
    let update_transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::transfer(
                &context.payer.pubkey(),
                &extra_account_metas_address,
                rent_lamports,
            ),
            update_extra_account_meta_list(
                &hook_program_id,
                &extra_account_metas_address,
                &mint_address,
                &mint_authority_pubkey,
                &updated_extra_account_metas,
            ),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &mint_authority],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(update_transaction)
        .await
        .unwrap();

    let updated_extra_pda_1 = Pubkey::find_program_address(
        &[
            b"updated-seed-prefix", // Literal prefix
            source.as_ref(),        // Account at index 0
        ],
        &hook_program_id,
    )
    .0;
    let extra_pda_2 = Pubkey::find_program_address(
        &[
            &amount.to_le_bytes(), // Instruction data bytes 8 to 16
            destination.as_ref(),  // Account at index 2
        ],
        &hook_program_id,
    )
    .0;

    let test_updated_extra_account_metas = [
        AccountMeta::new_readonly(sysvar::instructions::id(), false),
        AccountMeta::new_readonly(mint_authority_pubkey, true),
        AccountMeta::new(updated_extra_pda_1, false),
        AccountMeta::new(extra_pda_2, false),
        AccountMeta::new(writable_pubkey, false),
    ];

    // Use updated account metas list
    let mut test_instruction = execute_with_extra_account_metas(
        &program_id,
        &source,
        &mint_address,
        &destination,
        &wallet.pubkey(),
        &extra_account_metas_address,
        &test_updated_extra_account_metas,
        amount,
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
async fn success_execute_with_updated_extra_account_metas() {
    let program_id = Pubkey::new_unique();
    let mut program_test = setup(&program_id);

    let token_program_id = spl_token_2022::id();
    let wallet = Keypair::new();
    let mint_address = spl_transfer_hook_example::mint::id();
    let mint_authority = Keypair::new();
    let mint_authority_pubkey = mint_authority.pubkey();
    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let decimals = 2;
    let amount = 0u64;

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

    let extra_account_metas_address = get_extra_account_metas_address(&mint_address, &program_id);

    let writable_pubkey = Pubkey::new_unique();

    let init_extra_account_metas = [
        ExtraAccountMeta::new_with_pubkey(&sysvar::instructions::id(), false, false).unwrap(),
        ExtraAccountMeta::new_with_pubkey(&mint_authority_pubkey, true, false).unwrap(),
        ExtraAccountMeta::new_with_seeds(
            &[
                Seed::Literal {
                    bytes: b"seed-prefix".to_vec(),
                },
                Seed::AccountKey { index: 0 },
            ],
            false,
            true,
        )
        .unwrap(),
        ExtraAccountMeta::new_with_seeds(
            &[
                Seed::InstructionData {
                    index: 8,  // After instruction discriminator
                    length: 8, // `u64` (amount)
                },
                Seed::AccountKey { index: 2 },
            ],
            false,
            true,
        )
        .unwrap(),
        ExtraAccountMeta::new_with_pubkey(&writable_pubkey, false, true).unwrap(),
    ];

    let extra_pda_1 = Pubkey::find_program_address(
        &[
            b"seed-prefix",  // Literal prefix
            source.as_ref(), // Account at index 0
        ],
        &program_id,
    )
    .0;

    let extra_pda_2 = Pubkey::find_program_address(
        &[
            &amount.to_le_bytes(), // Instruction data bytes 8 to 16
            destination.as_ref(),  // Account at index 2
        ],
        &program_id,
    )
    .0;

    let init_account_metas = [
        AccountMeta::new_readonly(sysvar::instructions::id(), false),
        AccountMeta::new_readonly(mint_authority_pubkey, true),
        AccountMeta::new(extra_pda_1, false),
        AccountMeta::new(extra_pda_2, false),
        AccountMeta::new(writable_pubkey, false),
    ];

    let mut context = program_test.start_with_context().await;
    let rent = context.banks_client.get_rent().await.unwrap();
    let rent_lamports = rent
        .minimum_balance(ExtraAccountMetaList::size_of(init_extra_account_metas.len()).unwrap());
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::transfer(
                &context.payer.pubkey(),
                &extra_account_metas_address,
                rent_lamports,
            ),
            initialize_extra_account_meta_list(
                &program_id,
                &extra_account_metas_address,
                &mint_address,
                &mint_authority_pubkey,
                &init_extra_account_metas,
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

    let updated_amount = 1u64;
    let updated_writable_pubkey = Pubkey::new_unique();

    // Create updated extra account metas
    let updated_extra_account_metas = [
        ExtraAccountMeta::new_with_pubkey(&sysvar::instructions::id(), false, false).unwrap(),
        ExtraAccountMeta::new_with_pubkey(&mint_authority_pubkey, true, false).unwrap(),
        ExtraAccountMeta::new_with_seeds(
            &[
                Seed::Literal {
                    bytes: b"updated-seed-prefix".to_vec(),
                },
                Seed::AccountKey { index: 0 },
            ],
            false,
            true,
        )
        .unwrap(),
        ExtraAccountMeta::new_with_seeds(
            &[
                Seed::InstructionData {
                    index: 8,  // After instruction discriminator
                    length: 8, // `u64` (amount)
                },
                Seed::AccountKey { index: 2 },
            ],
            false,
            true,
        )
        .unwrap(),
        ExtraAccountMeta::new_with_pubkey(&updated_writable_pubkey, false, true).unwrap(),
        ExtraAccountMeta::new_with_seeds(
            &[
                Seed::Literal {
                    bytes: b"new-seed-prefix".to_vec(),
                },
                Seed::AccountKey { index: 0 },
            ],
            false,
            true,
        )
        .unwrap(),
    ];

    let updated_extra_pda_1 = Pubkey::find_program_address(
        &[
            b"updated-seed-prefix", // Literal prefix
            source.as_ref(),        // Account at index 0
        ],
        &program_id,
    )
    .0;

    let updated_extra_pda_2 = Pubkey::find_program_address(
        &[
            &updated_amount.to_le_bytes(), // Instruction data bytes 8 to 16
            destination.as_ref(),          // Account at index 2
        ],
        &program_id,
    )
    .0;

    // add another PDA
    let new_extra_pda = Pubkey::find_program_address(
        &[
            b"new-seed-prefix", // Literal prefix
            source.as_ref(),    // Account at index 0
        ],
        &program_id,
    )
    .0;

    let updated_account_metas = [
        AccountMeta::new_readonly(sysvar::instructions::id(), false),
        AccountMeta::new_readonly(mint_authority_pubkey, true),
        AccountMeta::new(updated_extra_pda_1, false),
        AccountMeta::new(updated_extra_pda_2, false),
        AccountMeta::new(updated_writable_pubkey, false),
        AccountMeta::new(new_extra_pda, false),
    ];

    let update_transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::transfer(
                &context.payer.pubkey(),
                &extra_account_metas_address,
                rent_lamports,
            ),
            update_extra_account_meta_list(
                &program_id,
                &extra_account_metas_address,
                &mint_address,
                &mint_authority_pubkey,
                &updated_extra_account_metas,
            ),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &mint_authority],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(update_transaction)
        .await
        .unwrap();

    // fail with initial account metas list
    {
        let transaction = Transaction::new_signed_with_payer(
            &[execute_with_extra_account_metas(
                &program_id,
                &source,
                &mint_address,
                &destination,
                &wallet.pubkey(),
                &extra_account_metas_address,
                &init_account_metas,
                updated_amount,
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
                InstructionError::Custom(AccountResolutionError::IncorrectAccount as u32),
            )
        );
    }

    // fail with missing account
    {
        let transaction = Transaction::new_signed_with_payer(
            &[execute_with_extra_account_metas(
                &program_id,
                &source,
                &mint_address,
                &destination,
                &wallet.pubkey(),
                &extra_account_metas_address,
                &updated_account_metas[..2],
                updated_amount,
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
                InstructionError::Custom(AccountResolutionError::IncorrectAccount as u32),
            )
        );
    }

    // fail with wrong account
    {
        let extra_account_metas = [
            AccountMeta::new_readonly(sysvar::instructions::id(), false),
            AccountMeta::new_readonly(mint_authority_pubkey, true),
            AccountMeta::new(updated_extra_pda_1, false),
            AccountMeta::new(updated_extra_pda_2, false),
            AccountMeta::new(Pubkey::new_unique(), false),
        ];
        let transaction = Transaction::new_signed_with_payer(
            &[execute_with_extra_account_metas(
                &program_id,
                &source,
                &mint_address,
                &destination,
                &wallet.pubkey(),
                &extra_account_metas_address,
                &extra_account_metas,
                updated_amount,
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
                InstructionError::Custom(AccountResolutionError::IncorrectAccount as u32),
            )
        );
    }

    // fail with wrong PDA
    let wrong_pda_2 = Pubkey::find_program_address(
        &[
            &99u64.to_le_bytes(), // Wrong data
            destination.as_ref(),
        ],
        &program_id,
    )
    .0;
    {
        let extra_account_metas = [
            AccountMeta::new_readonly(sysvar::instructions::id(), false),
            AccountMeta::new_readonly(mint_authority_pubkey, true),
            AccountMeta::new(updated_extra_pda_1, false),
            AccountMeta::new(wrong_pda_2, false),
            AccountMeta::new(writable_pubkey, false),
        ];
        let transaction = Transaction::new_signed_with_payer(
            &[execute_with_extra_account_metas(
                &program_id,
                &source,
                &mint_address,
                &destination,
                &wallet.pubkey(),
                &extra_account_metas_address,
                &extra_account_metas,
                updated_amount,
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
                InstructionError::Custom(AccountResolutionError::IncorrectAccount as u32),
            )
        );
    }

    // fail with not signer
    {
        let extra_account_metas = [
            AccountMeta::new_readonly(sysvar::instructions::id(), false),
            AccountMeta::new_readonly(mint_authority_pubkey, false),
            AccountMeta::new(updated_extra_pda_1, false),
            AccountMeta::new(updated_extra_pda_2, false),
            AccountMeta::new(writable_pubkey, false),
        ];
        let transaction = Transaction::new_signed_with_payer(
            &[execute_with_extra_account_metas(
                &program_id,
                &source,
                &mint_address,
                &destination,
                &wallet.pubkey(),
                &extra_account_metas_address,
                &extra_account_metas,
                updated_amount,
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
                InstructionError::Custom(AccountResolutionError::IncorrectAccount as u32),
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
                &extra_account_metas_address,
                &updated_account_metas,
                updated_amount,
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
