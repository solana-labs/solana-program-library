use solana_bpf_loader_program::{
    create_vm,
    serialization::{deserialize_parameters, serialize_parameters},
};
use solana_rbpf::vm::{EbpfVm, InstructionMeter};
use solana_runtime::process_instruction::{
    ComputeBudget, ComputeMeter, Executor, InvokeContext, Logger, ProcessInstruction,
};
use solana_sdk::{
    account::{Account, KeyedAccount},
    bpf_loader,
    entrypoint::SUCCESS,
    instruction::{CompiledInstruction, Instruction, InstructionError},
    message::Message,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use spl_shared_memory::entrypoint;

use std::{cell::RefCell, fs::File, io::Read, path::PathBuf, rc::Rc, sync::Arc};

fn load_program(name: &str) -> Vec<u8> {
    let mut path = PathBuf::new();
    path.push("../../target/bpfel-unknown-unknown/release");
    path.push(name);
    path.set_extension("so");
    let mut file = File::open(path).unwrap();

    let mut program = Vec::new();
    file.read_to_end(&mut program).unwrap();
    program
}

fn run_program(
    program_id: &Pubkey,
    parameter_accounts: &[KeyedAccount],
    instruction_data: &[u8],
) -> Result<u64, InstructionError> {
    let mut program_account = Account::default();
    program_account.data = load_program("spl_shared_memory");
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
    Ok(vm.get_total_instruction_count())
}

#[test]
fn assert_instruction_count() {
    const OFFSET: usize = 51;
    const NUM_TO_SHARE: usize = 500;
    let program_id = Pubkey::new_rand();
    let shared_key = Pubkey::new_rand();
    let shared_account = Account::new_ref(u64::MAX, OFFSET + NUM_TO_SHARE * 2, &program_id);

    // Send some data to share
    let parameter_accounts = vec![KeyedAccount::new(&shared_key, true, &shared_account)];
    let content = vec![42; NUM_TO_SHARE];
    let mut instruction_data = OFFSET.to_le_bytes().to_vec();
    instruction_data.extend_from_slice(&content);
    let share_count = run_program(&program_id, &parameter_accounts[..], &instruction_data).unwrap();
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

#[test]
fn test_share_data() {
    const OFFSET: usize = 51;
    const NUM_TO_SHARE: usize = 500;
    let program_id = Pubkey::new(&[0; 32]);
    let shared_key = Pubkey::new_rand();
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

// Mock InvokeContext

#[derive(Debug, Default)]
struct MockInvokeContext {
    pub key: Pubkey,
    pub logger: MockLogger,
    pub compute_budget: ComputeBudget,
    pub compute_meter: MockComputeMeter,
}
impl InvokeContext for MockInvokeContext {
    fn push(&mut self, _key: &Pubkey) -> Result<(), InstructionError> {
        Ok(())
    }
    fn pop(&mut self) {}
    fn verify_and_update(
        &mut self,
        _message: &Message,
        _instruction: &CompiledInstruction,
        _accounts: &[Rc<RefCell<Account>>],
    ) -> Result<(), InstructionError> {
        Ok(())
    }
    fn get_caller(&self) -> Result<&Pubkey, InstructionError> {
        Ok(&self.key)
    }
    fn get_programs(&self) -> &[(Pubkey, ProcessInstruction)] {
        &[]
    }
    fn get_logger(&self) -> Rc<RefCell<dyn Logger>> {
        Rc::new(RefCell::new(self.logger.clone()))
    }
    fn get_compute_budget(&self) -> &ComputeBudget {
        &self.compute_budget
    }
    fn get_compute_meter(&self) -> Rc<RefCell<dyn ComputeMeter>> {
        Rc::new(RefCell::new(self.compute_meter.clone()))
    }
    fn add_executor(&mut self, _pubkey: &Pubkey, _executor: Arc<dyn Executor>) {}
    fn get_executor(&mut self, _pubkey: &Pubkey) -> Option<Arc<dyn Executor>> {
        None
    }
    fn record_instruction(&self, _instruction: &Instruction) {}
    fn is_feature_active(&self, _feature_id: &Pubkey) -> bool {
        true
    }
}

#[derive(Debug, Default, Clone)]
struct MockComputeMeter {}
impl ComputeMeter for MockComputeMeter {
    fn consume(&mut self, _amount: u64) -> Result<(), InstructionError> {
        Ok(())
    }
    fn get_remaining(&self) -> u64 {
        u64::MAX
    }
}
#[derive(Debug, Default, Clone)]
struct MockLogger {}
impl Logger for MockLogger {
    fn log_enabled(&self) -> bool {
        true
    }
    fn log(&mut self, message: &str) {
        println!("{}", message);
    }
}

struct TestInstructionMeter {}
impl InstructionMeter for TestInstructionMeter {
    fn consume(&mut self, _amount: u64) {}
    fn get_remaining(&self) -> u64 {
        u64::MAX
    }
}
