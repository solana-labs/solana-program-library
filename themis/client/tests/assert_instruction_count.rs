use bn::{Fr, Group, G1};
use borsh::BorshSerialize;
use elgamal_bn::ciphertext::Ciphertext;
use solana_bpf_loader_program::{
    create_vm,
    serialization::{deserialize_parameters, serialize_parameters},
};
use solana_rbpf::vm::{EbpfVm, InstructionMeter};
use solana_runtime::process_instruction::{
    ComputeBudget, ComputeMeter, Executor, InvokeContext, Logger, ProcessInstruction,
};
use solana_sdk::{
    account::{Account as SolanaAccount, KeyedAccount},
    bpf_loader,
    entrypoint::SUCCESS,
    instruction::{CompiledInstruction, InstructionError},
    message::Message,
    pubkey::Pubkey,
};
use spl_themis::{
    instruction::ThemisInstruction,
    state::{generate_keys, /*recover_scalar,*/ Policies, User},
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
    let scalars = vec![Fr::new(1u64.into()).unwrap(), Fr::new(2u64.into()).unwrap()];
    //let scalars = vec![
    //        Fr::new(1u64.into()).unwrap(),
    //        Fr::new(1u64.into()).unwrap(),
    //        Fr::new(1u64.into()).unwrap(),
    //        Fr::new(1u64.into()).unwrap(),
    //        Fr::new(1u64.into()).unwrap(),
    //        Fr::new(1u64.into()).unwrap(),
    //        Fr::new(1u64.into()).unwrap(),
    //        Fr::new(1u64.into()).unwrap(),
    //        Fr::new(1u64.into()).unwrap(),
    //        Fr::new(1u64.into()).unwrap(), //10
    //        Fr::new(2u64.into()).unwrap(),
    //        Fr::new(2u64.into()).unwrap(),
    //        Fr::new(2u64.into()).unwrap(),
    //        Fr::new(2u64.into()).unwrap(),
    //        Fr::new(2u64.into()).unwrap(),
    //        Fr::new(2u64.into()).unwrap(),
    //        Fr::new(2u64.into()).unwrap(),
    //        Fr::new(2u64.into()).unwrap(),
    //        Fr::new(2u64.into()).unwrap(),
    //        Fr::new(2u64.into()).unwrap(), // 2 * 10
    //    Fr::new(1u64.into()).unwrap(),
    //    Fr::new(1u64.into()).unwrap(),
    //    Fr::new(1u64.into()).unwrap(),
    //    Fr::new(1u64.into()).unwrap(),
    //    Fr::new(1u64.into()).unwrap(),
    //    Fr::new(1u64.into()).unwrap(),
    //    Fr::new(1u64.into()).unwrap(),
    //    Fr::new(1u64.into()).unwrap(),
    //    Fr::new(1u64.into()).unwrap(),
    //    Fr::new(1u64.into()).unwrap(), //10
    //    Fr::new(2u64.into()).unwrap(),
    //    Fr::new(2u64.into()).unwrap(),
    //    Fr::new(2u64.into()).unwrap(),
    //    Fr::new(2u64.into()).unwrap(),
    //    Fr::new(2u64.into()).unwrap(),
    //    Fr::new(2u64.into()).unwrap(),
    //    Fr::new(2u64.into()).unwrap(),
    //    Fr::new(2u64.into()).unwrap(),
    //    Fr::new(2u64.into()).unwrap(),
    //    Fr::new(2u64.into()).unwrap(), // 2 * 10
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //    Fr::new(0u64.into()).unwrap(),
    //];
    let num_scalars = scalars.len();

    let (sk, pk) = generate_keys();
    let encrypted_interactions: Vec<_> = scalars
        .iter()
        .map(|_| pk.encrypt(&G1::one()).points)
        .collect();

    let policies_account = SolanaAccount::new_ref(
        0,
        Policies {
            is_initialized: true,
            scalars: scalars.clone(),
        }
        .try_to_vec()
        .unwrap()
        .len(),
        &program_id,
    );
    let instruction_data = ThemisInstruction::InitializePoliciesAccount { scalars }
        .serialize()
        .unwrap();
    let parameter_accounts = vec![KeyedAccount::new(&policies_key, false, &policies_account)];
    let initialize_policies_count =
        run_program(&program_id, &parameter_accounts[..], &instruction_data).unwrap();

    // Create user account
    let user_key = Pubkey::new_rand();
    let user_account =
        SolanaAccount::new_ref(0, User::default().try_to_vec().unwrap().len(), &program_id);
    let instruction_data = ThemisInstruction::InitializeUserAccount
        .serialize()
        .unwrap();
    let parameter_accounts = vec![KeyedAccount::new(&user_key, false, &user_account)];
    let initialize_user_count =
        run_program(&program_id, &parameter_accounts[..], &instruction_data).unwrap();

    // Calculate Aggregate
    let instruction_data = ThemisInstruction::CalculateAggregate {
        encrypted_interactions,
        public_key: pk,
    }
    .serialize()
    .unwrap();
    let parameter_accounts = vec![
        KeyedAccount::new(&user_key, true, &user_account),
        KeyedAccount::new(&policies_key, false, &policies_account),
    ];
    let calculate_aggregate_count =
        run_program(&program_id, &parameter_accounts[..], &instruction_data).unwrap();

    // Submit proof decryption
    let user = User::deserialize(&user_account.try_borrow().unwrap().data).unwrap();
    let encrypted_point = user.fetch_encrypted_aggregate();
    let ciphertext = Ciphertext {
        points: encrypted_point,
        pk,
    };

    let decrypted_aggregate = sk.decrypt(&ciphertext);
    //let scalar_aggregate = recover_scalar(decrypted_aggregate, 16);
    //let expected_scalar_aggregate = Fr::new(3u64.into()).unwrap();
    //assert_eq!(scalar_aggregate, expected_scalar_aggregate);

    let (announcement, response) = sk
        .prove_correct_decryption_no_Merlin(&ciphertext, &decrypted_aggregate)
        .unwrap();

    let instruction_data = ThemisInstruction::SubmitProofDecryption {
        plaintext: decrypted_aggregate,
        announcement,
        response,
    }
    .serialize()
    .unwrap();
    let parameter_accounts = vec![KeyedAccount::new(&user_key, true, &user_account)];
    let proof_decryption_count =
        run_program(&program_id, &parameter_accounts[..], &instruction_data).unwrap();

    const BASELINE_NEW_POLICIES_COUNT: u64 = 80_000; // last known 75,796 @ 128, 4,675 @ 2
    const BASELINE_INITIALIZE_USER_COUNT: u64 = 22_000; // last known 19,868
    const BASELINE_CALCULATE_AGGREGATE_COUNT: u64 = 15_000_000; // last known 13,061,884
    const BASELINE_PROOF_DECRYPTION_COUNT: u64 = 60_000_000; // last known 13,167,140

    println!("BPF instructions executed");
    println!(
        "  InitializePolicies({}): {:?} ({:?})",
        num_scalars, initialize_policies_count, BASELINE_NEW_POLICIES_COUNT
    );
    println!(
        "  InitializeUserAccount: {:?} ({:?})",
        initialize_user_count, BASELINE_INITIALIZE_USER_COUNT
    );
    println!(
        "  CalculateAggregate:    {:?} ({:?})",
        calculate_aggregate_count, BASELINE_CALCULATE_AGGREGATE_COUNT
    );
    println!(
        "  SubmitProofDecryption: {:?} ({:?})",
        proof_decryption_count, BASELINE_PROOF_DECRYPTION_COUNT
    );

    assert!(initialize_policies_count <= BASELINE_NEW_POLICIES_COUNT);
    assert!(initialize_user_count <= BASELINE_INITIALIZE_USER_COUNT);
    assert!(calculate_aggregate_count <= BASELINE_CALCULATE_AGGREGATE_COUNT);
    assert!(proof_decryption_count <= BASELINE_PROOF_DECRYPTION_COUNT);
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
        ComputeBudget {
            max_invoke_depth: 10,
            ..ComputeBudget::default()
        }
    }
    fn get_compute_meter(&self) -> Rc<RefCell<dyn ComputeMeter>> {
        Rc::new(RefCell::new(self.compute_meter.clone()))
    }
    fn add_executor(&mut self, _pubkey: &Pubkey, _executor: Arc<dyn Executor>) {}
    fn get_executor(&mut self, _pubkey: &Pubkey) -> Option<Arc<dyn Executor>> {
        None
    }
    fn record_instruction(&self, _: &solana_sdk::instruction::Instruction) {
        todo!()
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
