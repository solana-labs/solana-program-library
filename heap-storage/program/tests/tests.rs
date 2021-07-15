#![cfg(feature = "test-bpf")]

use borsh::de::BorshDeserialize;
use heap_storage::*;
use solana_program::{instruction::InstructionError, pubkey::Pubkey, system_instruction};
use solana_program_test::*;
use solana_sdk::{
    account::Account,
    signature::{Keypair, Signer},
    transaction::{Transaction, TransactionError},
    transport::TransportError,
};

pub fn program_test() -> ProgramTest {
    ProgramTest::new(
        "heap_storage",
        id(),
        processor!(processor::Processor::process_instruction),
    )
}

pub async fn get_account(program_context: &mut ProgramTestContext, pubkey: &Pubkey) -> Account {
    program_context
        .banks_client
        .get_account(*pubkey)
        .await
        .expect("account not found")
        .expect("account empty")
}

pub async fn create_node_account(
    program_context: &mut ProgramTestContext,
    heap: &Pubkey,
    account_to_create: &Pubkey,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[instruction::create_node_account(
            &id(),
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

pub async fn init_heap(
    program_context: &mut ProgramTestContext,
    authority: &Keypair,
) -> Result<Keypair, TransportError> {
    let rent = program_context.banks_client.get_rent().await.unwrap();
    let heap_min_rent = rent.minimum_balance(state::Heap::LEN);

    let heap_account = Keypair::new();

    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &program_context.payer.pubkey(),
                &heap_account.pubkey(),
                heap_min_rent,
                state::Heap::LEN as u64,
                &id(),
            ),
            instruction::init(&id(), &heap_account.pubkey(), &authority.pubkey()).unwrap(),
        ],
        Some(&program_context.payer.pubkey()),
    );

    transaction.sign(
        &[&program_context.payer, &heap_account, authority],
        program_context.last_blockhash,
    );
    program_context
        .banks_client
        .process_transaction(transaction)
        .await?;
    Ok(heap_account)
}

pub async fn add_node(
    program_context: &mut ProgramTestContext,
    heap_account: &Pubkey,
    node_account: &Pubkey,
    authority_account: &Keypair,
    node_data: [u8; 32],
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[instruction::add_node(
            &id(),
            heap_account,
            node_account,
            &authority_account.pubkey(),
            node_data,
        )
        .unwrap()],
        Some(&program_context.payer.pubkey()),
    );

    transaction.sign(
        &[&program_context.payer, authority_account],
        program_context.last_blockhash,
    );
    program_context
        .banks_client
        .process_transaction(transaction)
        .await
}

pub async fn remove_node(
    program_context: &mut ProgramTestContext,
    heap_account: &Pubkey,
    node_account: &Pubkey,
    leaf_account: &Pubkey,
    authority_account: &Keypair,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[instruction::remove_node(
            &id(),
            heap_account,
            node_account,
            leaf_account,
            &authority_account.pubkey(),
        )
        .unwrap()],
        Some(&program_context.payer.pubkey()),
    );

    transaction.sign(
        &[&program_context.payer, authority_account],
        program_context.last_blockhash,
    );
    program_context
        .banks_client
        .process_transaction(transaction)
        .await
}

pub async fn swap_nodes(
    program_context: &mut ProgramTestContext,
    heap_account: &Pubkey,
    parent_node_account: &Pubkey,
    child_node_account: &Pubkey,
    authority_account: &Keypair,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[instruction::swap(
            &id(),
            heap_account,
            parent_node_account,
            child_node_account,
            &authority_account.pubkey(),
        )
        .unwrap()],
        Some(&program_context.payer.pubkey()),
    );

    transaction.sign(
        &[&program_context.payer, authority_account],
        program_context.last_blockhash,
    );
    program_context
        .banks_client
        .process_transaction(transaction)
        .await
}

#[tokio::test]
async fn test_init_heap() {
    let mut program_context = program_test().start_with_context().await;

    let heap_authority = Keypair::new();

    let heap_key: Keypair = init_heap(&mut program_context, &heap_authority)
        .await
        .unwrap();

    let heap_account_data = get_account(&mut program_context, &heap_key.pubkey()).await;
    let heap = state::Heap::try_from_slice(&heap_account_data.data.as_slice()).unwrap();

    assert!(heap.is_initialized());
}

#[tokio::test]
async fn test_create_node_account() {
    let mut program_context = program_test().start_with_context().await;

    let heap_authority = Keypair::new();

    let heap_key: Keypair = init_heap(&mut program_context, &heap_authority)
        .await
        .unwrap();

    let heap_account_data = get_account(&mut program_context, &heap_key.pubkey()).await;
    let heap = state::Heap::try_from_slice(&heap_account_data.data.as_slice()).unwrap();

    let (node_key, _) = Pubkey::find_program_address(
        &[
            &heap_key.pubkey().to_bytes()[..32],
            &heap.size.to_le_bytes(),
        ],
        &id(),
    );

    create_node_account(&mut program_context, &heap_key.pubkey(), &node_key)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_add_node() {
    let mut program_context = program_test().start_with_context().await;

    let heap_authority = Keypair::new();

    let heap_key: Keypair = init_heap(&mut program_context, &heap_authority)
        .await
        .unwrap();

    let heap_account_data = get_account(&mut program_context, &heap_key.pubkey()).await;
    let heap = state::Heap::try_from_slice(&heap_account_data.data.as_slice()).unwrap();

    let (node_key, _) = Pubkey::find_program_address(
        &[
            &heap_key.pubkey().to_bytes()[..32],
            &heap.size.to_le_bytes(),
        ],
        &id(),
    );

    create_node_account(&mut program_context, &heap_key.pubkey(), &node_key)
        .await
        .unwrap();

    let node_data = [1; 32];
    let heap_size_before = heap.size;

    add_node(
        &mut program_context,
        &heap_key.pubkey(),
        &node_key,
        &heap_authority,
        node_data,
    )
    .await
    .unwrap();

    let node_account_data = get_account(&mut program_context, &node_key).await;
    let node = state::Node::try_from_slice(&node_account_data.data.as_slice()).unwrap();

    let heap_account_data = get_account(&mut program_context, &heap_key.pubkey()).await;
    let heap = state::Heap::try_from_slice(&heap_account_data.data.as_slice()).unwrap();

    assert!(node.is_initialized());
    assert_eq!(node.data, node_data);
    assert_eq!(heap.size, heap_size_before + 1);
    assert_eq!(node.index, heap_size_before);
}

#[tokio::test]
async fn test_remove_node() {
    let mut program_context = program_test().start_with_context().await;

    let heap_authority = Keypair::new();

    let heap_key: Keypair = init_heap(&mut program_context, &heap_authority)
        .await
        .unwrap();

    let heap_account_data = get_account(&mut program_context, &heap_key.pubkey()).await;
    let heap = state::Heap::try_from_slice(&heap_account_data.data.as_slice()).unwrap();

    let (node_key, _) = Pubkey::find_program_address(
        &[
            &heap_key.pubkey().to_bytes()[..32],
            &heap.size.to_le_bytes(),
        ],
        &id(),
    );

    create_node_account(&mut program_context, &heap_key.pubkey(), &node_key)
        .await
        .unwrap();

    let node_data = [1; 32];

    add_node(
        &mut program_context,
        &heap_key.pubkey(),
        &node_key,
        &heap_authority,
        node_data,
    )
    .await
    .unwrap();

    let heap_account_data = get_account(&mut program_context, &heap_key.pubkey()).await;
    let heap = state::Heap::try_from_slice(&heap_account_data.data.as_slice()).unwrap();

    let heap_size_before = heap.size;

    remove_node(
        &mut program_context,
        &heap_key.pubkey(),
        &node_key,
        &node_key,
        &heap_authority,
    )
    .await
    .unwrap();

    let heap_account_data = get_account(&mut program_context, &heap_key.pubkey()).await;
    let heap = state::Heap::try_from_slice(&heap_account_data.data.as_slice()).unwrap();

    assert_eq!(heap.size, heap_size_before - 1);

    let node_account_data = get_account(&mut program_context, &node_key).await;
    let node = state::Node::try_from_slice(&node_account_data.data.as_slice()).unwrap();

    assert_eq!(node.is_initialized(), false);
}

#[tokio::test]
async fn test_swap_nodes() {
    let mut program_context = program_test().start_with_context().await;

    let heap_authority = Keypair::new();

    let heap_key: Keypair = init_heap(&mut program_context, &heap_authority)
        .await
        .unwrap();

    let node_count: u8 = 6;
    let mut nodes: Vec<Pubkey> = Vec::new();
    for index in 0..node_count {
        let (node_key, _) = Pubkey::find_program_address(
            &[
                &heap_key.pubkey().to_bytes()[..32],
                &(index as u128).to_le_bytes(),
            ],
            &id(),
        );
        nodes.push(node_key);

        create_node_account(&mut program_context, &heap_key.pubkey(), &node_key)
            .await
            .unwrap();

        // +1 because data can't be empty
        let node_data = [index + 1; 32];

        add_node(
            &mut program_context,
            &heap_key.pubkey(),
            &node_key,
            &heap_authority,
            node_data,
        )
        .await
        .unwrap();
    }

    swap_nodes(
        &mut program_context,
        &heap_key.pubkey(),
        &nodes[0],
        &nodes[1],
        &heap_authority,
    )
    .await
    .unwrap();

    let first_node_account_data = get_account(&mut program_context, &nodes[0]).await;
    let first_node = state::Node::try_from_slice(&first_node_account_data.data.as_slice()).unwrap();

    let second_node_account_data = get_account(&mut program_context, &nodes[1]).await;
    let second_node =
        state::Node::try_from_slice(&second_node_account_data.data.as_slice()).unwrap();

    assert_eq!(first_node.data, [2; 32]);
    assert_eq!(second_node.data, [1; 32]);

    swap_nodes(
        &mut program_context,
        &heap_key.pubkey(),
        &nodes[0],
        &nodes[2],
        &heap_authority,
    )
    .await
    .unwrap();

    let third_node_in_place_of_first = get_account(&mut program_context, &nodes[0]).await;
    let third_node_in_place_of_first =
        state::Node::try_from_slice(&third_node_in_place_of_first.data.as_slice()).unwrap();
    assert_eq!(third_node_in_place_of_first.data, [3; 32]);

    // try to swap not parent-child nodes
    let transaction_error = swap_nodes(
        &mut program_context,
        &heap_key.pubkey(),
        &nodes[0],
        &nodes[5],
        &heap_authority,
    )
    .await
    .err()
    .unwrap();

    match transaction_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => {
            let program_error = error::HeapProgramError::NodesAreNotRelated as u32;
            assert_eq!(error_index, program_error);
        }
        _ => panic!("Wrong error occurs while try to swap unrelated nodes"),
    }
}
