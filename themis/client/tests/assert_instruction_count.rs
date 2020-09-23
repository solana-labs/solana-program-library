use bincode::{serialize, serialized_size};
use solana_bpf_loader_program::{
    create_vm,
    serialization::{deserialize_parameters, serialize_parameters},
};
use solana_rbpf::vm::{EbpfVm, InstructionMeter};
use solana_sdk::{
    account::{Account as SolanaAccount, KeyedAccount},
    bpf_loader,
    entrypoint::SUCCESS,
    entrypoint_native::{
        ComputeBudget, ComputeMeter, Executor, InvokeContext, Logger, ProcessInstruction,
    },
    instruction::{CompiledInstruction, InstructionError},
    message::Message,
    pubkey::Pubkey,
};
use spl_themis::{
    instruction::ThemisInstruction,
    state::Policies,
    //state::{Policies, User},
};
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
    let mut program_account = SolanaAccount::default();
    program_account.data = load_program("spl_themis");
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
    let program_id = Pubkey::new_rand();

    // Create new policies
    let policies_key = Pubkey::new_rand();
    let scalars = vec![];
    //let scalars = vec![0u8.into()]; // TODO: Only works if MAX_CALL_DEPTH doubled in the BPF VM
    let policies_account = SolanaAccount::new_ref(0, serialized_size(&Policies {is_initialized: true, scalars: scalars.clone() }).unwrap() as usize, &program_id);
    let instruction_data = serialize(&ThemisInstruction::InitializePoliciesAccount {
        scalars
    }).unwrap();
    let parameter_accounts = vec![
        KeyedAccount::new(&policies_key, false, &policies_account),
    ];
    let initialize_policies_count =
        run_program(&program_id, &parameter_accounts[..], &instruction_data).unwrap();

    // Create user account
    //let user_key = Pubkey::new_rand();
    //let user_account = SolanaAccount::new_ref(0, serialized_size(&User::default()).unwrap() as usize, &program_id);
    //let instruction_data = serialize(&ThemisInstruction::InitializeUserAccount).unwrap();
    //let parameter_accounts = vec![
    //    KeyedAccount::new(&user_key, false, &user_account),
    //];
    //let initialize_user_account =
    //    run_program(&program_id, &parameter_accounts[..], &instruction_data).unwrap();

    const BASELINE_NEW_POLICIES_COUNT: u64 = 100_000; // last known 3107
    //const BASELINE_INITIALIZE_ACCOUNT_COUNT: u64 = 6500; // last known 6445

    println!("BPF instructions executed");
    println!(
        "  InitializePolicies   : {:?} ({:?})",
        initialize_policies_count, BASELINE_NEW_POLICIES_COUNT
    );
    //println!(
    //    "  InitializeUserAccount: {:?} ({:?})",
    //    initialize_user_account, BASELINE_INITIALIZE_ACCOUNT_COUNT
    //);

    assert!(initialize_policies_count <= BASELINE_NEW_POLICIES_COUNT);
    //assert!(initialize_user_account <= BASELINE_INITIALIZE_ACCOUNT_COUNT);
}

// Mock InvokeContext

#[derive(Debug, Default)]
struct MockInvokeContext {
    pub key: Pubkey,
    pub logger: MockLogger,
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
        _accounts: &[Rc<RefCell<SolanaAccount>>],
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
    fn is_cross_program_supported(&self) -> bool {
        true
    }
    fn get_compute_budget(&self) -> ComputeBudget {
        ComputeBudget { max_invoke_depth: 10, .. ComputeBudget::default() }
    }
    fn get_compute_meter(&self) -> Rc<RefCell<dyn ComputeMeter>> {
        Rc::new(RefCell::new(self.compute_meter.clone()))
    }
    fn add_executor(&mut self, _pubkey: &Pubkey, _executor: Arc<dyn Executor>) {}
    fn get_executor(&mut self, _pubkey: &Pubkey) -> Option<Arc<dyn Executor>> {
        None
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
