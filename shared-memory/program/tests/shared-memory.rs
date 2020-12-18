use solana_bpf_loader_program::serialization::serialize_parameters;
use solana_program::{
    bpf_loader, entrypoint::SUCCESS, program_error::ProgramError, pubkey::Pubkey,
};
use solana_sdk::{account::Account, keyed_account::KeyedAccount};
use spl_shared_memory::entrypoint;

// TODO: Rework `assert_instruction_count` test to use solana-program-test, avoiding the need to
// link directly with the BPF VM
/*
fn load_program(name: &str) -> Vec<u8> {
    let mut file =
        File::open(&name).unwrap_or_else(|err| panic!("Unable to open {}: {}", name, err));

    let mut program = Vec::new();
    file.read_to_end(&mut program).unwrap();
    program
}

fn run_program(
    program_id: &Pubkey,
    parameter_accounts: &[KeyedAccount],
    instruction_data: &[u8],
) -> u64 {
    let program_account = Account {
        data: load_program("../../target/deploy/spl_shared_memory.so"),
        ..Account::default()
    };
    let loader_id = bpf_loader::id();
    let mut invoke_context = MockInvokeContext::default();
    let executable = EbpfVm::<solana_bpf_loader_program::BPFError>::create_executable_from_elf(
        &&program_account.data,
        None,
    )
    .unwrap();
    let (mut vm, heap_region) = create_vm(
        &loader_id,
        executable.as_ref(),
        parameter_accounts,
        &mut invoke_context,
    )
    .unwrap();
    let mut parameter_bytes = serialize_parameters(
        &loader_id,
        program_id,
        parameter_accounts,
        &instruction_data,
    )
    .unwrap();
    assert_eq!(
        SUCCESS,
        vm.execute_program(parameter_bytes.as_mut_slice(), &[], &[heap_region])
            .unwrap()
    );
    deserialize_parameters(&loader_id, parameter_accounts, &parameter_bytes).unwrap();
    vm.get_total_instruction_count()
}

#[test]
fn assert_instruction_count() {
    const OFFSET: usize = 51;
    const NUM_TO_SHARE: usize = 500;
    let program_id = Pubkey::new_unique();
    let shared_key = Pubkey::new_unique();
    let shared_account = Account::new_ref(u64::MAX, OFFSET + NUM_TO_SHARE * 2, &program_id);

    // Send some data to share
    let parameter_accounts = vec![KeyedAccount::new(&shared_key, true, &shared_account)];
    let content = vec![42; NUM_TO_SHARE];
    let mut instruction_data = OFFSET.to_le_bytes().to_vec();
    instruction_data.extend_from_slice(&content);
    let share_count = run_program(&program_id, &parameter_accounts[..], &instruction_data);
    const BASELINE_COUNT: u64 = 1474; // 113 if NUM_TO_SHARE is 8
    println!(
        "BPF instructions executed {:?} (expected {:?})",
        share_count, BASELINE_COUNT
    );
    assert_eq!(
        &shared_account.borrow().data[OFFSET..OFFSET + NUM_TO_SHARE],
        content
    );
    assert!(share_count <= BASELINE_COUNT);
}
*/

#[test]
fn test_share_data() {
    const OFFSET: usize = 51;
    const NUM_TO_SHARE: usize = 500;
    let program_id = Pubkey::new(&[0; 32]);
    let shared_key = Pubkey::new_unique();
    let shared_account = Account::new_ref(u64::MAX, NUM_TO_SHARE * 2, &program_id);

    // success
    let content = vec![42; NUM_TO_SHARE];
    let mut instruction_data = OFFSET.to_le_bytes().to_vec();
    instruction_data.extend_from_slice(&content);
    let keyed_accounts = vec![KeyedAccount::new(&shared_key, true, &shared_account)];
    let mut input = serialize_parameters(
        &bpf_loader::id(),
        &program_id,
        &keyed_accounts,
        &instruction_data,
    )
    .unwrap();
    assert_eq!(unsafe { entrypoint(input.as_mut_ptr()) }, SUCCESS);

    // success zero offset
    let content = vec![42; NUM_TO_SHARE];
    let mut instruction_data = 0_usize.to_le_bytes().to_vec();
    instruction_data.extend_from_slice(&content);
    let keyed_accounts = vec![KeyedAccount::new(&shared_key, true, &shared_account)];
    let mut input = serialize_parameters(
        &bpf_loader::id(),
        &program_id,
        &keyed_accounts,
        &instruction_data,
    )
    .unwrap();
    assert_eq!(unsafe { entrypoint(input.as_mut_ptr()) }, SUCCESS);

    // too few accounts
    let mut input =
        serialize_parameters(&bpf_loader::id(), &program_id, &[], &instruction_data).unwrap();
    assert_eq!(
        unsafe { entrypoint(input.as_mut_ptr()) },
        u64::from(ProgramError::NotEnoughAccountKeys)
    );

    // too many accounts
    let keyed_accounts = vec![
        KeyedAccount::new(&shared_key, true, &shared_account),
        KeyedAccount::new(&shared_key, true, &shared_account),
    ];
    let mut input = serialize_parameters(
        &bpf_loader::id(),
        &program_id,
        &keyed_accounts,
        &instruction_data,
    )
    .unwrap();
    assert_eq!(
        unsafe { entrypoint(input.as_mut_ptr()) },
        u64::from(ProgramError::InvalidArgument)
    );

    // account data too small
    let keyed_accounts = vec![KeyedAccount::new(&shared_key, true, &shared_account)];
    let content = vec![42; NUM_TO_SHARE * 10];
    let mut instruction_data = OFFSET.to_le_bytes().to_vec();
    instruction_data.extend_from_slice(&content);
    let mut input = serialize_parameters(
        &bpf_loader::id(),
        &program_id,
        &keyed_accounts,
        &instruction_data,
    )
    .unwrap();
    assert_eq!(
        unsafe { entrypoint(input.as_mut_ptr()) },
        u64::from(ProgramError::AccountDataTooSmall)
    );

    // offset too large
    let keyed_accounts = vec![KeyedAccount::new(&shared_key, true, &shared_account)];
    let content = vec![42; NUM_TO_SHARE];
    let mut instruction_data = (OFFSET * 10).to_le_bytes().to_vec();
    instruction_data.extend_from_slice(&content);
    let mut input = serialize_parameters(
        &bpf_loader::id(),
        &program_id,
        &keyed_accounts,
        &instruction_data,
    )
    .unwrap();
    assert_eq!(
        unsafe { entrypoint(input.as_mut_ptr()) },
        u64::from(ProgramError::AccountDataTooSmall)
    );
}
