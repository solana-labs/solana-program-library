use solana_bpf_loader_program::{
    create_vm,
    serialization::{deserialize_parameters, serialize_parameters},
    syscalls, BpfError, ThisInstructionMeter,
};
use solana_rbpf::{
    elf::EBpfElf,
    vm::{Config, Executable},
};
use solana_sdk::{
    account::AccountSharedData,
    bpf_loader,
    entrypoint::SUCCESS,
    keyed_account::KeyedAccount,
    process_instruction::{MockComputeMeter, MockInvokeContext},
    program_option::COption,
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::{
        self,
        rent::{self, Rent},
    },
};
use spl_token::{
    instruction::TokenInstruction,
    state::{Account, Mint},
};
use std::{cell::RefCell, fs::File, io::Read, rc::Rc};

fn load_program(name: &str) -> Vec<u8> {
    let mut file = File::open(name).unwrap();

    let mut program = Vec::new();
    file.read_to_end(&mut program).unwrap();
    program
}

fn run_program(
    program_id: &Pubkey,
    parameter_accounts: &[KeyedAccount],
    instruction_data: &[u8],
) -> u64 {
    let loader_id = bpf_loader::id();
    let mut invoke_context = MockInvokeContext::new(parameter_accounts.into());

    let mut executable = EBpfElf::<BpfError, ThisInstructionMeter>::load(
        Config::default(),
        &load_program("../../target/deploy/spl_token.so"),
    )
    .expect("failed to load spl_token.so");
    executable.set_syscall_registry(
        syscalls::register_syscalls(&mut invoke_context)
            .expect("failed to create syscalls register"),
    );

    let mut parameter_bytes =
        serialize_parameters(&loader_id, program_id, parameter_accounts, instruction_data)
            .expect("failed to serialize");

    let mut vm = create_vm(
        &loader_id,
        &executable,
        parameter_bytes.as_slice_mut(),
        &mut invoke_context,
    )
    .expect("failed to create vm");

    let compute_meter = Rc::new(RefCell::new(MockComputeMeter {
        remaining: u64::MAX,
    }));
    let mut instruction_meter = ThisInstructionMeter { compute_meter };
    assert_eq!(
        vm.execute_program_interpreted(&mut instruction_meter)
            .expect("failed to execute"),
        SUCCESS
    );

    deserialize_parameters(&loader_id, parameter_accounts, parameter_bytes.as_slice())
        .expect("failed to deserialize");

    vm.get_total_instruction_count()
}

#[test]
fn assert_instruction_count() {
    let program_id = Pubkey::new_unique();
    let source_key = Pubkey::new_unique();
    let source_account =
        AccountSharedData::new_ref(u64::MAX, Account::get_packed_len(), &program_id);
    let destination_key = Pubkey::new_unique();
    let destination_account =
        AccountSharedData::new_ref(u64::MAX, Account::get_packed_len(), &program_id);
    let owner_key = Pubkey::new_unique();
    let owner_account = RefCell::new(AccountSharedData::default());
    let mint_key = Pubkey::new_unique();
    let mint_account = AccountSharedData::new_ref(0, Mint::get_packed_len(), &program_id);
    let rent_key = rent::id();
    let rent_account =
        AccountSharedData::new_ref_data(42, &Rent::free(), &sysvar::id()).expect("invalid rent");

    // Create new mint
    let instruction_data = TokenInstruction::InitializeMint {
        decimals: 9,
        mint_authority: owner_key,
        freeze_authority: COption::None,
    }
    .pack();
    let parameter_accounts = vec![
        KeyedAccount::new(&mint_key, false, &mint_account),
        KeyedAccount::new(&rent_key, false, &rent_account),
    ];
    let initialize_mint_count =
        run_program(&program_id, &parameter_accounts[..], &instruction_data);

    // Create source account
    let instruction_data = TokenInstruction::InitializeAccount.pack();
    let parameter_accounts = vec![
        KeyedAccount::new(&source_key, false, &source_account),
        KeyedAccount::new(&mint_key, false, &mint_account),
        KeyedAccount::new(&owner_key, false, &owner_account),
        KeyedAccount::new(&rent_key, false, &rent_account),
    ];
    let mintto_count = run_program(&program_id, &parameter_accounts[..], &instruction_data);

    // Create destination account
    let instruction_data = TokenInstruction::InitializeAccount.pack();
    let parameter_accounts = vec![
        KeyedAccount::new(&destination_key, false, &destination_account),
        KeyedAccount::new(&mint_key, false, &mint_account),
        KeyedAccount::new(&owner_key, false, &owner_account),
        KeyedAccount::new(&rent_key, false, &rent_account),
    ];
    let _ = run_program(&program_id, &parameter_accounts[..], &instruction_data);

    // MintTo source account
    let instruction_data = TokenInstruction::MintTo { amount: 100 }.pack();
    let parameter_accounts = vec![
        KeyedAccount::new(&mint_key, false, &mint_account),
        KeyedAccount::new(&source_key, false, &source_account),
        KeyedAccount::new(&owner_key, true, &owner_account),
    ];
    let initialize_account_count =
        run_program(&program_id, &parameter_accounts[..], &instruction_data);

    // Transfer from source to destination
    let instruction = TokenInstruction::Transfer { amount: 100 };
    let instruction_data = instruction.pack();
    let parameter_accounts = vec![
        KeyedAccount::new(&source_key, false, &source_account),
        KeyedAccount::new(&destination_key, false, &destination_account),
        KeyedAccount::new(&owner_key, true, &owner_account),
    ];
    let transfer_count = run_program(&program_id, &parameter_accounts[..], &instruction_data);

    const BASELINE_NEW_MINT_COUNT: u64 = 4000; // last known 2112
    const BASELINE_INITIALIZE_ACCOUNT_COUNT: u64 = 6500; // last known 2758
    const BASELINE_MINTTO_COUNT: u64 = 6500; // last known 3239
    const BASELINE_TRANSFER_COUNT: u64 = 8000; // last known 3098

    println!("BPF instructions executed");
    println!(
        "  InitializeMint   : {:?} ({:?})",
        initialize_mint_count, BASELINE_NEW_MINT_COUNT
    );
    println!(
        "  InitializeAccount: {:?} ({:?})",
        initialize_account_count, BASELINE_INITIALIZE_ACCOUNT_COUNT
    );
    println!(
        "  MintTo           : {:?} ({:?})",
        mintto_count, BASELINE_MINTTO_COUNT
    );
    println!(
        "  Transfer         : {:?} ({:?})",
        transfer_count, BASELINE_TRANSFER_COUNT,
    );

    assert!(initialize_account_count <= BASELINE_INITIALIZE_ACCOUNT_COUNT);
    assert!(initialize_mint_count <= BASELINE_NEW_MINT_COUNT);
    assert!(transfer_count <= BASELINE_TRANSFER_COUNT);
}
