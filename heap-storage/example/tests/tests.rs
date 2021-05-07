#![cfg(feature = "test-bpf")]

use borsh::BorshDeserialize;
use solana_program::{instruction::InstructionError, pubkey::Pubkey, system_instruction};
use solana_program_template::*;
use solana_program_test::*;
use solana_sdk::{
    account::Account,
    signature::{Keypair, Signer},
    transaction::Transaction,
    transport::TransportError,
};

pub fn program_test() -> ProgramTest {
    let mut program = ProgramTest::new(
        "solana_program_template",
        id(),
        processor!(processor::Processor::process_instruction),
    );
    program.add_program("heap_storage", heap_storage::id(), None);
    program
}

pub async fn get_account(program_context: &mut ProgramTestContext, pubkey: &Pubkey) -> Account {
    program_context
        .banks_client
        .get_account(*pubkey)
        .await
        .expect("account not found")
        .expect("account empty")
}
pub async fn create_account(
    program_context: &mut ProgramTestContext,
    account: &Keypair,
    rent: u64,
    space: u64,
    owner: &Pubkey,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[system_instruction::create_account(
            &program_context.payer.pubkey(),
            &account.pubkey(),
            rent,
            space,
            owner,
        )],
        Some(&program_context.payer.pubkey()),
    );

    transaction.sign(
        &[&program_context.payer, account],
        program_context.last_blockhash,
    );
    program_context
        .banks_client
        .process_transaction(transaction)
        .await?;
    Ok(())
}

pub async fn init_heap(
    program_context: &mut ProgramTestContext,
) -> Result<Keypair, TransportError> {
    let rent = program_context.banks_client.get_rent().await.unwrap();
    let heap_min_rent = rent.minimum_balance(heap_storage::state::Heap::LEN);

    let heap_account = Keypair::new();

    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &program_context.payer.pubkey(),
                &heap_account.pubkey(),
                heap_min_rent,
                heap_storage::state::Heap::LEN as u64,
                &heap_storage::id(),
            ),
            instruction::init(&id(), &heap_account.pubkey()).unwrap(),
        ],
        Some(&program_context.payer.pubkey()),
    );

    transaction.sign(
        &[&program_context.payer, &heap_account],
        program_context.last_blockhash,
    );
    program_context
        .banks_client
        .process_transaction(transaction)
        .await?;
    Ok(heap_account)
}

pub async fn create_node_account(
    program_context: &mut ProgramTestContext,
    heap: &Pubkey,
    account_to_create: &Pubkey,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[heap_storage::instruction::create_node_account(
            &heap_storage::id(),
            &program_context.payer.pubkey(),
            heap,
            account_to_create,
        )
        .unwrap()],
        Some(&program_context.payer.pubkey()),
    );
    transaction.sign(&[&program_context.payer], program_context.last_blockhash);
    program_context
        .banks_client
        .process_transaction(transaction)
        .await
}

pub async fn add_node(
    program_context: &mut ProgramTestContext,
    data: &Pubkey,
    node: &Pubkey,
    heap: &Pubkey,
    input: instruction::Add,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[instruction::add(&id(), data, node, heap, input).unwrap()],
        Some(&program_context.payer.pubkey()),
    );
    transaction.sign(&[&program_context.payer], program_context.last_blockhash);
    program_context
        .banks_client
        .process_transaction(transaction)
        .await
}

pub async fn remove_node(
    program_context: &mut ProgramTestContext,
    data: &Pubkey,
    heap: &Pubkey,
    node: &Pubkey,
    leaf: &Pubkey,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[instruction::remove(&id(), data, heap, node, leaf).unwrap()],
        Some(&program_context.payer.pubkey()),
    );
    transaction.sign(&[&program_context.payer], program_context.last_blockhash);
    program_context
        .banks_client
        .process_transaction(transaction)
        .await
}

#[tokio::test]
async fn test_call_example_instruction() {
    let mut program_context = program_test().start_with_context().await;

    let heap_acc = init_heap(&mut program_context).await.unwrap();

    let heap_info = get_account(&mut program_context, &heap_acc.pubkey()).await;
    let heap_info = heap_storage::state::Heap::try_from_slice(&heap_info.data.as_slice()).unwrap();

    assert!(heap_info.is_initialized());
}

#[tokio::test]
async fn test_add_node() {
    let mut program_context = program_test().start_with_context().await;
    let rent = program_context.banks_client.get_rent().await.unwrap();

    let heap_acc = init_heap(&mut program_context).await.unwrap();

    let heap_account_data = get_account(&mut program_context, &heap_acc.pubkey()).await;
    let heap =
        heap_storage::state::Heap::try_from_slice(&heap_account_data.data.as_slice()).unwrap();

    let (node_key, _) = Pubkey::find_program_address(
        &[
            &heap_acc.pubkey().to_bytes()[..32],
            &heap.size.to_le_bytes(),
        ],
        &heap_storage::id(),
    );

    create_node_account(&mut program_context, &heap_acc.pubkey(), &node_key)
        .await
        .unwrap();

    let data_acc = Keypair::new();
    let data_acc_min_rent = rent.minimum_balance(state::DataAccount::LEN);
    create_account(
        &mut program_context,
        &data_acc,
        data_acc_min_rent,
        state::DataAccount::LEN as u64,
        &id(),
    )
    .await
    .unwrap();

    let node_data = instruction::Add { amount: 1 };
    add_node(
        &mut program_context,
        &data_acc.pubkey(),
        &node_key,
        &heap_acc.pubkey(),
        node_data.clone(),
    )
    .await
    .unwrap();

    let data_acc_info = get_account(&mut program_context, &data_acc.pubkey()).await;
    let data_acc_info = state::DataAccount::try_from_slice(&data_acc_info.data.as_slice()).unwrap();

    assert_eq!(data_acc_info.value, node_data.amount);

    let node_acc_info = get_account(&mut program_context, &node_key).await;
    let node_acc =
        heap_storage::state::Node::try_from_slice(&node_acc_info.data.as_slice()).unwrap();

    assert_eq!(node_acc.data, data_acc.pubkey().to_bytes());
}

#[tokio::test]
async fn test_remove_node() {
    let mut program_context = program_test().start_with_context().await;
    let rent = program_context.banks_client.get_rent().await.unwrap();

    let heap_acc = init_heap(&mut program_context).await.unwrap();

    let heap_account_data = get_account(&mut program_context, &heap_acc.pubkey()).await;
    let heap =
        heap_storage::state::Heap::try_from_slice(&heap_account_data.data.as_slice()).unwrap();

    let (node_key, _) = Pubkey::find_program_address(
        &[
            &heap_acc.pubkey().to_bytes()[..32],
            &heap.size.to_le_bytes(),
        ],
        &heap_storage::id(),
    );

    create_node_account(&mut program_context, &heap_acc.pubkey(), &node_key)
        .await
        .unwrap();

    let data_acc = Keypair::new();
    let data_acc_min_rent = rent.minimum_balance(state::DataAccount::LEN);
    create_account(
        &mut program_context,
        &data_acc,
        data_acc_min_rent,
        state::DataAccount::LEN as u64,
        &id(),
    )
    .await
    .unwrap();

    let node_data = instruction::Add { amount: 1 };
    add_node(
        &mut program_context,
        &data_acc.pubkey(),
        &node_key,
        &heap_acc.pubkey(),
        node_data.clone(),
    )
    .await
    .unwrap();

    remove_node(
        &mut program_context,
        &data_acc.pubkey(),
        &heap_acc.pubkey(),
        &node_key,
        &node_key,
    )
    .await
    .unwrap();

    let data_acc_info = get_account(&mut program_context, &data_acc.pubkey()).await;
    let data_acc_info = state::DataAccount::try_from_slice(&data_acc_info.data.as_slice()).unwrap();

    assert_eq!(data_acc_info.value, state::UNINITIALIZED_VALUE);

    let node_acc_info = get_account(&mut program_context, &node_key).await;
    let node_acc =
        heap_storage::state::Node::try_from_slice(&node_acc_info.data.as_slice()).unwrap();

    assert_eq!(node_acc.is_initialized(), false);
}
