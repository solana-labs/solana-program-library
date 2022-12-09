use solana_program::program_option::COption;
use solana_program_test::*;
use solana_sdk::{
    commitment_config::CommitmentLevel,
    instruction::Instruction,
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::Signature,
    signature::{signers::Signers, Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};

use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account,
};
use spl_managed_token::{get_unified_transfer_address, instruction::*};
use spl_token::state::Account as TokenAccount;
use spl_token_manager_registry::{
    bytes_to_account_meta, create_get_transfer_accounts_instruction, create_register_instruction,
    create_unified_transfer_instruction, find_manager_registration_address, ACCOUNT_META_BYTES,
};

pub fn sol(amount: f64) -> u64 {
    (amount * LAMPORTS_PER_SOL as f64) as u64
}

async fn process_transaction<S: Signers>(
    client: &mut BanksClient,
    instructions: &[Instruction],
    signers: &S,
) -> anyhow::Result<Signature> {
    let tx = Transaction::new_signed_with_payer(
        instructions,
        Some(&signers.pubkeys()[0]),
        signers,
        client.get_latest_blockhash().await?,
    );
    let sig = tx.signatures[0];
    client
        .process_transaction_with_commitment(tx, CommitmentLevel::Confirmed)
        .await?;
    Ok(sig)
}

async fn transfer(
    context: &mut BanksClient,
    payer: &Keypair,
    receiver: &Pubkey,
    amount: u64,
) -> anyhow::Result<Signature> {
    let ix = system_instruction::transfer(&payer.pubkey(), receiver, amount);
    process_transaction(context, &[ix], &[payer]).await
}

fn spl_managed_token_test() -> ProgramTest {
    ProgramTest::new(
        "spl_managed_token",
        spl_managed_token::id(),
        processor!(spl_managed_token::process_instruction),
    )
}

#[tokio::test]
async fn test_spl_managed_token_basic() {
    let mut context = spl_managed_token_test().start_with_context().await;
    let lwc = &mut context.banks_client;
    let authority = Keypair::new();
    transfer(lwc, &context.payer, &authority.pubkey(), sol(10.0))
        .await
        .unwrap();
    let mint = Keypair::new();
    let mint_key = mint.pubkey();
    let unified_transfer = get_unified_transfer_address(&spl_managed_token::id(), &mint_key);
    let create_ix = create_initialize_mint_instruction(
        &mint_key,
        &authority.pubkey(),
        &authority.pubkey(),
        &unified_transfer,
        0,
    )
    .unwrap();
    process_transaction(lwc, &[create_ix], &[&authority, &mint])
        .await
        .unwrap();

    let alice = Keypair::new();
    let alice_key = alice.pubkey();
    let bob = Keypair::new();
    let bob_key = bob.pubkey();
    let eve = Keypair::new();
    let eve_key = eve.pubkey();

    for k in [&alice_key, &bob_key] {
        transfer(lwc, &context.payer, k, sol(1.0)).await.unwrap();
        let create_ata = create_initialize_account_instruction(
            &mint_key,
            k,
            &authority.pubkey(),
            &authority.pubkey(),
        )
        .unwrap();
        let mint_to_ix =
            create_mint_to_instruction(&mint_key, k, &authority.pubkey(), 1000).unwrap();
        process_transaction(lwc, &[create_ata, mint_to_ix], &[&authority])
            .await
            .unwrap();
    }

    let create_eve =
        create_associated_token_account(&authority.pubkey(), &eve_key, &mint_key, &spl_token::id());
    process_transaction(lwc, &[create_eve], &[&authority])
        .await
        .unwrap();

    // Try transfer the normal way
    let failed_transfer_ix = spl_token::instruction::transfer(
        &spl_token::id(),
        &get_associated_token_address(&alice_key, &mint_key),
        &get_associated_token_address(&bob_key, &mint_key),
        &alice_key,
        &[],
        100,
    )
    .unwrap();

    assert!(process_transaction(lwc, &[failed_transfer_ix], &[&alice])
        .await
        .is_err());

    let eve_ix =
        create_transfer_instruction(&alice_key, &eve_key, &mint_key, &authority.pubkey(), 100)
            .unwrap();

    assert!(process_transaction(lwc, &[eve_ix], &[&alice, &authority])
        .await
        .is_err());

    let successful_transfer_ix =
        create_transfer_instruction(&alice_key, &bob_key, &mint_key, &authority.pubkey(), 100)
            .unwrap();
    let burn_ix = create_burn_instruction(&mint_key, &alice_key, &authority.pubkey(), 900).unwrap();

    process_transaction(
        lwc,
        &[successful_transfer_ix, burn_ix],
        &[&alice, &authority],
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn test_spl_managed_token_with_approve_and_revoke() {
    let mut context = spl_managed_token_test().start_with_context().await;
    let lwc = &mut context.banks_client;
    let authority = Keypair::new();
    transfer(lwc, &context.payer, &authority.pubkey(), sol(10.0))
        .await
        .unwrap();
    let mint = Keypair::new();
    let mint_key = mint.pubkey();
    let unified_transfer = get_unified_transfer_address(&spl_managed_token::id(), &mint_key);
    let create_ix = create_initialize_mint_instruction(
        &mint_key,
        &authority.pubkey(),
        &authority.pubkey(),
        &unified_transfer,
        0,
    )
    .unwrap();
    process_transaction(lwc, &[create_ix], &[&authority, &mint])
        .await
        .unwrap();

    let alice = Keypair::new();
    let alice_key = alice.pubkey();
    let bob = Keypair::new();
    let bob_key = bob.pubkey();

    transfer(lwc, &context.payer, &alice_key, sol(1.0))
        .await
        .unwrap();
    transfer(lwc, &context.payer, &bob_key, sol(1.0))
        .await
        .unwrap();

    let create_alice_ata_ix = create_initialize_account_instruction(
        &mint_key,
        &alice_key,
        &authority.pubkey(),
        &authority.pubkey(),
    )
    .unwrap();
    let mint_to_ix =
        create_mint_to_instruction(&mint_key, &alice_key, &authority.pubkey(), 1).unwrap();
    let delegate_ix =
        create_approve_instruction(&mint_key, &alice_key, &bob_key, &authority.pubkey(), 1)
            .unwrap();
    process_transaction(
        lwc,
        &[create_alice_ata_ix, mint_to_ix, delegate_ix],
        &[&alice, &authority],
    )
    .await
    .unwrap();

    assert!(lwc
        .get_packed_account_data::<TokenAccount>(get_associated_token_address(
            &alice_key, &mint_key
        ))
        .await
        .unwrap()
        .delegate
        .eq(&COption::Some(bob_key)));

    let revoke_ix = create_revoke_instruction(&mint_key, &alice_key, &authority.pubkey()).unwrap();
    process_transaction(lwc, &[revoke_ix], &[&alice, &authority])
        .await
        .unwrap();

    assert!(lwc
        .get_packed_account_data::<TokenAccount>(get_associated_token_address(
            &alice_key, &mint_key
        ))
        .await
        .unwrap()
        .delegate
        .is_none());
}

#[tokio::test]
async fn test_spl_managed_token_with_delegate_transfer() {
    let mut context = spl_managed_token_test().start_with_context().await;
    let lwc = &mut context.banks_client;
    let authority = Keypair::new();
    transfer(lwc, &context.payer, &authority.pubkey(), sol(10.0))
        .await
        .unwrap();
    let mint = Keypair::new();
    let mint_key = mint.pubkey();
    let unified_transfer = get_unified_transfer_address(&spl_managed_token::id(), &mint_key);
    let create_ix = create_initialize_mint_instruction(
        &mint_key,
        &authority.pubkey(),
        &authority.pubkey(),
        &unified_transfer,
        0,
    )
    .unwrap();
    process_transaction(lwc, &[create_ix], &[&authority, &mint])
        .await
        .unwrap();

    let alice = Keypair::new();
    let alice_key = alice.pubkey();
    let bob = Keypair::new();
    let bob_key = bob.pubkey();
    let eve = Keypair::new();
    let eve_key = eve.pubkey();

    transfer(lwc, &context.payer, &alice_key, sol(1.0))
        .await
        .unwrap();
    transfer(lwc, &context.payer, &bob_key, sol(1.0))
        .await
        .unwrap();

    let create_alice_ata_ix = create_initialize_account_instruction(
        &mint_key,
        &alice_key,
        &authority.pubkey(),
        &authority.pubkey(),
    )
    .unwrap();
    let mint_to_ix =
        create_mint_to_instruction(&mint_key, &alice_key, &authority.pubkey(), 1).unwrap();
    let delegate_ix =
        create_approve_instruction(&mint_key, &alice_key, &bob_key, &authority.pubkey(), 1)
            .unwrap();
    process_transaction(
        lwc,
        &[create_alice_ata_ix, mint_to_ix, delegate_ix],
        &[&alice, &authority],
    )
    .await
    .unwrap();

    assert!(lwc
        .get_packed_account_data::<TokenAccount>(get_associated_token_address(
            &alice_key, &mint_key
        ))
        .await
        .unwrap()
        .delegate
        .eq(&COption::Some(bob_key)));

    let create_eve_ata_ix = create_initialize_account_instruction(
        &mint_key,
        &eve_key,
        &authority.pubkey(),
        &authority.pubkey(),
    )
    .unwrap();
    let successful_transfer_ix = create_transfer_with_delegate_instruction(
        &alice_key,
        &eve_key,
        &bob_key,
        &mint_key,
        &authority.pubkey(),
        1,
    )
    .unwrap();
    process_transaction(
        lwc,
        &[create_eve_ata_ix, successful_transfer_ix],
        &[&bob, &authority],
    )
    .await
    .unwrap();

    assert!(
        lwc.get_packed_account_data::<TokenAccount>(get_associated_token_address(
            &eve_key, &mint_key
        ))
        .await
        .unwrap()
        .amount
            == 1
    );
}

#[tokio::test]
async fn success_with_unified_transfer() {
    let mut program_test = spl_managed_token_test();
    program_test.prefer_bpf(false); // simplicity in the build
    program_test.add_program(
        "spl_token_manager_registry",
        spl_token_manager_registry::id(),
        processor!(spl_token_manager_registry::processor::process_instruction),
    );
    let mut context = program_test.start_with_context().await;

    let authority = Keypair::new();
    transfer(
        &mut context.banks_client,
        &context.payer,
        &authority.pubkey(),
        sol(10.0),
    )
    .await
    .unwrap();
    let mint = Keypair::new();
    let mint_pubkey = mint.pubkey();
    let unified_transfer = get_unified_transfer_address(&spl_managed_token::id(), &mint_pubkey);
    let create_ix = create_initialize_mint_instruction(
        &mint_pubkey,
        &authority.pubkey(),
        &authority.pubkey(),
        &unified_transfer,
        0,
    )
    .unwrap();
    process_transaction(
        &mut context.banks_client,
        &[create_ix],
        &[&authority, &mint],
    )
    .await
    .unwrap();

    // setup accounts
    let alice = Keypair::new();
    let alice_key = alice.pubkey();
    let bob = Keypair::new();
    let bob_key = bob.pubkey();

    for k in [&alice_key, &bob_key] {
        transfer(&mut context.banks_client, &context.payer, k, sol(1.0))
            .await
            .unwrap();
        let create_ata = create_initialize_account_instruction(
            &mint_pubkey,
            k,
            &authority.pubkey(),
            &authority.pubkey(),
        )
        .unwrap();
        let mint_to_ix =
            create_mint_to_instruction(&mint_pubkey, k, &authority.pubkey(), 1000).unwrap();
        process_transaction(
            &mut context.banks_client,
            &[create_ata, mint_to_ix],
            &[&authority],
        )
        .await
        .unwrap();
    }

    println!("Do a unified transfer!");
    println!("Zeroth, setup the manager program registration");
    let manager_registration_pubkey =
        find_manager_registration_address(&spl_token_manager_registry::id(), &mint_pubkey);
    let ix = create_register_instruction(
        &spl_token_manager_registry::id(),
        &authority.pubkey(),
        &mint_pubkey,
        &authority.pubkey(),
        &manager_registration_pubkey,
        &spl_managed_token::id(),
    );
    process_transaction(&mut context.banks_client, &[ix], &[&authority])
        .await
        .unwrap();

    println!(
        "First, figure out the manager program. Note: this is pedantic since we know already."
    );
    let registration = context
        .banks_client
        .get_account(manager_registration_pubkey)
        .await
        .unwrap()
        .unwrap();
    let manager_program_id = Pubkey::new(&registration.data);

    println!("Second, get the required accounts");
    let ix = create_get_transfer_accounts_instruction(
        &manager_program_id,
        &mint_pubkey,
        &unified_transfer,
    );
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&authority.pubkey()),
        &[&authority],
        context.last_blockhash,
    );
    let simulation_result = context
        .banks_client
        .simulate_transaction_with_commitment(tx, CommitmentLevel::Confirmed)
        .await
        .unwrap();
    let data = simulation_result
        .simulation_details
        .unwrap()
        .return_data
        .unwrap()
        .data;
    let num_accounts = data[0] as usize;
    let metas = data[1..]
        .chunks(ACCOUNT_META_BYTES)
        .map(bytes_to_account_meta)
        .collect::<Vec<_>>();
    assert_eq!(metas.len(), num_accounts);

    println!("Finally, execute the transfer");
    let source = get_associated_token_address(&alice_key, &mint_pubkey);
    let destination = get_associated_token_address(&bob_key, &mint_pubkey);
    let ix = create_unified_transfer_instruction(
        &manager_program_id,
        &source,
        &mint_pubkey,
        &destination,
        &alice_key,
        100,
        &metas,
    );
    process_transaction(&mut context.banks_client, &[ix], &[&alice, &authority])
        .await
        .unwrap();
}
