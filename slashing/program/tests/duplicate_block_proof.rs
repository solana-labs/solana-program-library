#![cfg(feature = "test-sbf")]

use {
    rand::Rng,
    solana_entry::entry::Entry,
    solana_ledger::{
        blockstore_meta::ErasureMeta,
        shred::{ProcessShredsStats, ReedSolomonCache, Shred, Shredder},
    },
    solana_program::pubkey::Pubkey,
    solana_program_test::*,
    solana_sdk::{
        clock::{Clock, Slot},
        decode_error::DecodeError,
        hash::Hash,
        instruction::InstructionError,
        rent::Rent,
        signature::{Keypair, Signer},
        system_instruction, system_transaction,
        transaction::{Transaction, TransactionError},
    },
    spl_pod::bytemuck::pod_get_packed_len,
    spl_record::{instruction as record, state::RecordData},
    spl_slashing::{
        duplicate_block_proof::DuplicateBlockProofData, error::SlashingError, id, instruction,
        processor::process_instruction, state::ProofType,
    },
    std::sync::Arc,
};

const SLOT: Slot = 53084024;

fn program_test() -> ProgramTest {
    let mut program_test = ProgramTest::new("spl_slashing", id(), processor!(process_instruction));
    program_test.add_program(
        "spl_record",
        spl_record::id(),
        processor!(spl_record::processor::process_instruction),
    );
    program_test
}

async fn setup_clock(context: &mut ProgramTestContext) {
    let clock: Clock = context.banks_client.get_sysvar().await.unwrap();
    let mut new_clock = clock.clone();
    new_clock.slot = SLOT;
    context.set_sysvar(&new_clock);
}

async fn initialize_duplicate_proof_account(
    context: &mut ProgramTestContext,
    authority: &Keypair,
    account: &Keypair,
) {
    let account_length = ProofType::DuplicateBlockProof
        .proof_account_length()
        .saturating_add(pod_get_packed_len::<RecordData>());
    println!("Creating account of size {account_length}");
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &account.pubkey(),
                1.max(Rent::default().minimum_balance(account_length)),
                account_length as u64,
                &spl_record::id(),
            ),
            record::initialize(&account.pubkey(), &authority.pubkey()),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, account],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}

async fn write_proof(
    context: &mut ProgramTestContext,
    authority: &Keypair,
    account: &Keypair,
    proof: &[u8],
) {
    let mut offset = 0;
    let proof_len = proof.len();
    let chunk_size = 800;
    println!("Writing a proof of size {proof_len}");
    while offset < proof_len {
        let end = std::cmp::min(offset.checked_add(chunk_size).unwrap(), proof_len);
        let transaction = Transaction::new_signed_with_payer(
            &[record::write(
                &account.pubkey(),
                &authority.pubkey(),
                offset as u64,
                &proof[offset..end],
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, authority],
            context.last_blockhash,
        );
        context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        offset = offset.checked_add(chunk_size).unwrap();
    }
}

pub fn new_rand_data_shred<R: Rng>(
    rng: &mut R,
    next_shred_index: u32,
    shredder: &Shredder,
    keypair: &Keypair,
    is_last_in_slot: bool,
) -> Shred {
    let (mut data_shreds, _) = new_rand_shreds(
        rng,
        next_shred_index,
        next_shred_index,
        5,
        shredder,
        keypair,
        is_last_in_slot,
    );
    data_shreds.pop().unwrap()
}

pub(crate) fn new_rand_coding_shreds<R: Rng>(
    rng: &mut R,
    next_shred_index: u32,
    num_entries: usize,
    shredder: &Shredder,
    keypair: &Keypair,
) -> Vec<Shred> {
    let (_, coding_shreds) = new_rand_shreds(
        rng,
        next_shred_index,
        next_shred_index,
        num_entries,
        shredder,
        keypair,
        true,
    );
    coding_shreds
}

pub(crate) fn new_rand_shreds<R: Rng>(
    rng: &mut R,
    next_shred_index: u32,
    next_code_index: u32,
    num_entries: usize,
    shredder: &Shredder,
    keypair: &Keypair,
    is_last_in_slot: bool,
) -> (Vec<Shred>, Vec<Shred>) {
    let entries: Vec<_> = std::iter::repeat_with(|| {
        let tx = system_transaction::transfer(
            &Keypair::new(),       // from
            &Pubkey::new_unique(), // to
            rng.gen(),             // lamports
            Hash::new_unique(),    // recent blockhash
        );
        Entry::new(
            &Hash::new_unique(), // prev_hash
            1,                   // num_hashes,
            vec![tx],            // transactions
        )
    })
    .take(num_entries)
    .collect();
    shredder.entries_to_shreds(
        keypair,
        &entries,
        is_last_in_slot,
        // chained_merkle_root
        Some(Hash::new_from_array(rng.gen())),
        next_shred_index,
        next_code_index, // next_code_index
        true,            // merkle_variant
        &ReedSolomonCache::default(),
        &mut ProcessShredsStats::default(),
    )
}

#[tokio::test]
async fn valid_proof_data() {
    let mut context = program_test().start_with_context().await;
    setup_clock(&mut context).await;

    let authority = Keypair::new();
    let account = Keypair::new();

    let mut rng = rand::thread_rng();
    let leader = Arc::new(Keypair::new());
    let (slot, parent_slot, reference_tick, version) = (SLOT, 53084023, 0, 0);
    let shredder = Shredder::new(slot, parent_slot, reference_tick, version).unwrap();
    let next_shred_index = rng.gen_range(0..32_000);
    let shred1 = new_rand_data_shred(&mut rng, next_shred_index, &shredder, &leader, true);
    let shred2 = new_rand_data_shred(&mut rng, next_shred_index, &shredder, &leader, true);

    assert_ne!(
        shred1.merkle_root().unwrap(),
        shred2.merkle_root().unwrap(),
        "Expecting merkle root conflict",
    );

    let duplicate_proof = DuplicateBlockProofData {
        shred1: shred1.payload().as_slice(),
        shred2: shred2.payload().as_slice(),
    };
    let data = duplicate_proof.pack();

    initialize_duplicate_proof_account(&mut context, &authority, &account).await;
    write_proof(&mut context, &authority, &account, &data).await;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::duplicate_block_proof(
            &account.pubkey(),
            RecordData::WRITABLE_START_INDEX as u64,
            slot,
            leader.pubkey(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}

#[tokio::test]
async fn valid_proof_coding() {
    let mut context = program_test().start_with_context().await;
    setup_clock(&mut context).await;

    let authority = Keypair::new();
    let account = Keypair::new();

    let mut rng = rand::thread_rng();
    let leader = Arc::new(Keypair::new());
    let (slot, parent_slot, reference_tick, version) = (SLOT, 53084023, 0, 0);
    let shredder = Shredder::new(slot, parent_slot, reference_tick, version).unwrap();
    let next_shred_index = rng.gen_range(0..32_000);
    let shred1 =
        new_rand_coding_shreds(&mut rng, next_shred_index, 10, &shredder, &leader)[0].clone();
    let shred2 =
        new_rand_coding_shreds(&mut rng, next_shred_index, 10, &shredder, &leader)[1].clone();

    assert!(
        ErasureMeta::check_erasure_consistency(&shred1, &shred2),
        "Expected erasure consistency failure",
    );

    let duplicate_proof = DuplicateBlockProofData {
        shred1: shred1.payload().as_slice(),
        shred2: shred2.payload().as_slice(),
    };
    let data = duplicate_proof.pack();

    initialize_duplicate_proof_account(&mut context, &authority, &account).await;
    write_proof(&mut context, &authority, &account, &data).await;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::duplicate_block_proof(
            &account.pubkey(),
            RecordData::WRITABLE_START_INDEX as u64,
            slot,
            leader.pubkey(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}

#[tokio::test]
async fn invalid_proof_data() {
    let mut context = program_test().start_with_context().await;
    setup_clock(&mut context).await;

    let authority = Keypair::new();
    let account = Keypair::new();

    let mut rng = rand::thread_rng();
    let leader = Arc::new(Keypair::new());
    let (slot, parent_slot, reference_tick, version) = (SLOT, 53084023, 0, 0);
    let shredder = Shredder::new(slot, parent_slot, reference_tick, version).unwrap();
    let next_shred_index = rng.gen_range(0..32_000);
    let shred1 = new_rand_data_shred(&mut rng, next_shred_index, &shredder, &leader, true);
    let shred2 = shred1.clone();

    let duplicate_proof = DuplicateBlockProofData {
        shred1: shred1.payload().as_slice(),
        shred2: shred2.payload().as_slice(),
    };
    let data = duplicate_proof.pack();

    initialize_duplicate_proof_account(&mut context, &authority, &account).await;
    write_proof(&mut context, &authority, &account, &data).await;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::duplicate_block_proof(
            &account.pubkey(),
            RecordData::WRITABLE_START_INDEX as u64,
            slot,
            leader.pubkey(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
    let err = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err()
        .unwrap();
    let TransactionError::InstructionError(0, InstructionError::Custom(code)) = err else {
        panic!("Invalid error {err:?}");
    };
    let err: SlashingError = SlashingError::decode_custom_error_to_enum(code).unwrap();
    assert_eq!(err, SlashingError::InvalidPayloadProof);
}

#[tokio::test]
async fn invalid_proof_coding() {
    let mut context = program_test().start_with_context().await;
    setup_clock(&mut context).await;

    let authority = Keypair::new();
    let account = Keypair::new();

    let mut rng = rand::thread_rng();
    let leader = Arc::new(Keypair::new());
    let (slot, parent_slot, reference_tick, version) = (SLOT, 53084023, 0, 0);
    let shredder = Shredder::new(slot, parent_slot, reference_tick, version).unwrap();
    let next_shred_index = rng.gen_range(0..32_000);
    let coding_shreds = new_rand_coding_shreds(&mut rng, next_shred_index, 10, &shredder, &leader);
    let shred1 = coding_shreds[0].clone();
    let shred2 = coding_shreds[1].clone();

    assert!(
        ErasureMeta::check_erasure_consistency(&shred1, &shred2),
        "Expecting no erasure conflict"
    );
    let duplicate_proof = DuplicateBlockProofData {
        shred1: shred1.payload().as_slice(),
        shred2: shred2.payload().as_slice(),
    };
    let data = duplicate_proof.pack();

    initialize_duplicate_proof_account(&mut context, &authority, &account).await;
    write_proof(&mut context, &authority, &account, &data).await;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::duplicate_block_proof(
            &account.pubkey(),
            RecordData::WRITABLE_START_INDEX as u64,
            slot,
            leader.pubkey(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
    let err = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err()
        .unwrap();
    let TransactionError::InstructionError(0, InstructionError::Custom(code)) = err else {
        panic!("Invalid error {err:?}");
    };
    let err: SlashingError = SlashingError::decode_custom_error_to_enum(code).unwrap();
    assert_eq!(err, SlashingError::InvalidErasureMetaConflict);
}
