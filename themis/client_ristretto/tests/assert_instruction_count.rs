use borsh::BorshSerialize;
use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
use elgamal_ristretto::ciphertext::Ciphertext;
use separator::Separatable;
use solana_bpf_loader_program::{
    create_vm,
    serialization::{deserialize_parameters, serialize_parameters},
};
use solana_rbpf::vm::EbpfVm;
use solana_sdk::{
    account::Account,
    bpf_loader,
    entrypoint::SUCCESS,
    keyed_account::KeyedAccount,
    process_instruction::{BpfComputeBudget, MockInvokeContext},
    pubkey::Pubkey,
};
use spl_themis_ristretto::{
    instruction::ThemisInstruction,
    state::{generate_keys, /*recover_scalar,*/ Policies, User},
};
use std::{fs::File, io::Read};

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
    let mut program_account = Account::default();
    program_account.data = load_program("../../target/deploy/spl_themis_ristretto.so");
    let loader_id = bpf_loader::id();
    let mut invoke_context = MockInvokeContext::default();
    invoke_context.bpf_compute_budget = BpfComputeBudget {
        max_invoke_depth: 10,
        ..BpfComputeBudget::default()
    };

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
    let program_id = Pubkey::new_unique();

    // Create new policies
    let policies_key = Pubkey::new_unique();
    let scalars = vec![1u64.into(), 2u64.into()];
    //let scalars = vec![
    //        1u64.into(),
    //        1u64.into(),
    //        1u64.into(),
    //        1u64.into(),
    //        1u64.into(),
    //        1u64.into(),
    //        1u64.into(),
    //        1u64.into(),
    //        1u64.into(),
    //        1u64.into(), //10
    //        2u64.into(),
    //        2u64.into(),
    //        2u64.into(),
    //        2u64.into(),
    //        2u64.into(),
    //        2u64.into(),
    //        2u64.into(),
    //        2u64.into(),
    //        2u64.into(),
    //        2u64.into(), // 2 * 10
    //    1u64.into(),
    //    1u64.into(),
    //    1u64.into(),
    //    1u64.into(),
    //    1u64.into(),
    //    1u64.into(),
    //    1u64.into(),
    //    1u64.into(),
    //    1u64.into(),
    //    1u64.into(), //10
    //    2u64.into(),
    //    2u64.into(),
    //    2u64.into(),
    //    2u64.into(),
    //    2u64.into(),
    //    2u64.into(),
    //    2u64.into(),
    //    2u64.into(),
    //    2u64.into(),
    //    2u64.into(), // 2 * 10
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //    0u64.into(),
    //];
    let num_scalars = scalars.len();

    let (sk, pk) = generate_keys();
    let encrypted_interactions: Vec<_> = (0..num_scalars)
        .map(|i| (i as u8, pk.encrypt(&RISTRETTO_BASEPOINT_POINT).points))
        .collect();

    let policies_account = Account::new_ref(
        0,
        Policies {
            is_initialized: true,
            num_scalars: num_scalars as u8,
            scalars,
        }
        .try_to_vec()
        .unwrap()
        .len(),
        &program_id,
    );
    let instruction_data = ThemisInstruction::InitializePoliciesAccount {
        num_scalars: num_scalars as u8,
    }
    .serialize()
    .unwrap();
    let parameter_accounts = vec![KeyedAccount::new(&policies_key, false, &policies_account)];
    let initialize_policies_count =
        run_program(&program_id, &parameter_accounts[..], &instruction_data);

    // Create user account
    let user_key = Pubkey::new_unique();
    let user_account =
        Account::new_ref(0, User::default().try_to_vec().unwrap().len(), &program_id);
    let instruction_data = ThemisInstruction::InitializeUserAccount { public_key: pk }
        .serialize()
        .unwrap();
    let parameter_accounts = vec![KeyedAccount::new(&user_key, false, &user_account)];
    let initialize_user_count =
        run_program(&program_id, &parameter_accounts[..], &instruction_data);

    // Calculate Aggregate
    let instruction_data = ThemisInstruction::SubmitInteractions {
        encrypted_interactions,
    }
    .serialize()
    .unwrap();
    let parameter_accounts = vec![
        KeyedAccount::new(&user_key, true, &user_account),
        KeyedAccount::new(&policies_key, false, &policies_account),
    ];
    let calculate_aggregate_count =
        run_program(&program_id, &parameter_accounts[..], &instruction_data);

    // Submit proof decryption
    let user = User::deserialize(&user_account.try_borrow().unwrap().data).unwrap();
    let encrypted_point = user.fetch_encrypted_aggregate();
    let ciphertext = Ciphertext {
        points: encrypted_point,
        pk,
    };

    let decrypted_aggregate = sk.decrypt(&ciphertext);
    //let scalar_aggregate = recover_scalar(decrypted_aggregate, 16);
    //let expected_scalar_aggregate = 3u64.into();
    //assert_eq!(scalar_aggregate, expected_scalar_aggregate);

    let (announcement, response) =
        sk.prove_correct_decryption_no_Merlin(&ciphertext, &decrypted_aggregate);

    let instruction_data = ThemisInstruction::SubmitProofDecryption {
        plaintext: decrypted_aggregate,
        announcement: Box::new(announcement),
        response,
    }
    .serialize()
    .unwrap();
    let parameter_accounts = vec![KeyedAccount::new(&user_key, true, &user_account)];
    let proof_decryption_count =
        run_program(&program_id, &parameter_accounts[..], &instruction_data);

    const BASELINE_NEW_POLICIES_COUNT: u64 = 80_000; // last known 3,354
    const BASELINE_INITIALIZE_USER_COUNT: u64 = 22_000; // last known 19,746
    const BASELINE_CALCULATE_AGGREGATE_COUNT: u64 = 200_000; // last known 87,220
    const BASELINE_PROOF_DECRYPTION_COUNT: u64 = 200_000; // last known 105,368

    println!("BPF instructions executed");
    println!(
        "  InitializePolicies({}): {} ({:?})",
        num_scalars,
        initialize_policies_count.separated_string(),
        BASELINE_NEW_POLICIES_COUNT
    );
    println!(
        "  InitializeUserAccount: {} ({:?})",
        initialize_user_count.separated_string(),
        BASELINE_INITIALIZE_USER_COUNT
    );
    println!(
        "  CalculateAggregate:    {} ({:?})",
        calculate_aggregate_count.separated_string(),
        BASELINE_CALCULATE_AGGREGATE_COUNT
    );
    println!(
        "  SubmitProofDecryption: {} ({:?})",
        proof_decryption_count.separated_string(),
        BASELINE_PROOF_DECRYPTION_COUNT
    );

    assert!(initialize_policies_count <= BASELINE_NEW_POLICIES_COUNT);
    assert!(initialize_user_count <= BASELINE_INITIALIZE_USER_COUNT);
    assert!(calculate_aggregate_count <= BASELINE_CALCULATE_AGGREGATE_COUNT);
    assert!(proof_decryption_count <= BASELINE_PROOF_DECRYPTION_COUNT);
}
