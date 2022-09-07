use solana_program_test::*;
use solana_sdk::{
    commitment_config::CommitmentLevel,
    instruction::Instruction,
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::Signature,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};

use permissioned_token::instruction::*;
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account,
};

pub fn sol(amount: f64) -> u64 {
    (amount * LAMPORTS_PER_SOL as f64) as u64
}

async fn process_transaction(
    client: &mut BanksClient,
    instructions: Vec<Instruction>,
    signers: Vec<&Keypair>,
) -> anyhow::Result<Signature> {
    let mut tx = Transaction::new_with_payer(&instructions, Some(&signers[0].pubkey()));
    tx.partial_sign(&signers, client.get_latest_blockhash().await?);
    let sig = tx.signatures[0];
    client
        .process_transaction_with_commitment(tx, CommitmentLevel::Confirmed)
        .await?;
    Ok(sig)
}

pub async fn airdrop(
    context: &mut BanksClient,
    payer: &Keypair,
    receiver: &Pubkey,
    amount: u64,
) -> anyhow::Result<Signature> {
    let ixs = vec![system_instruction::transfer(
        &payer.pubkey(),
        receiver,
        amount,
    )];
    process_transaction(context, ixs, vec![payer]).await
}

pub fn permissioned_token_test() -> ProgramTest {
    ProgramTest::new("permissioned_token", permissioned_token::id(), None)
}

#[tokio::test]
async fn test_permissioned_token_basic() {
    let mut context = permissioned_token_test().start_with_context().await;
    let lwc = &mut context.banks_client;
    let authority = Keypair::new();
    airdrop(lwc, &context.payer, &authority.pubkey(), sol(10.0))
        .await
        .unwrap();
    let mint = Keypair::new();
    let mint_key = mint.pubkey();
    let create_ix =
        create_initialize_mint_instruction(&mint_key, &authority.pubkey(), &authority.pubkey(), 0)
            .unwrap();
    process_transaction(lwc, vec![create_ix], vec![&authority, &mint])
        .await
        .unwrap();

    let alice = Keypair::new();
    let alice_key = alice.pubkey();
    let bob = Keypair::new();
    let bob_key = bob.pubkey();
    let eve = Keypair::new();
    let eve_key = eve.pubkey();

    for k in [&alice_key, &bob_key] {
        airdrop(lwc, &authority, k, sol(1.0)).await.unwrap();
        let create_ata = create_initialize_account_instruction(
            &mint_key,
            k,
            &authority.pubkey(),
            &authority.pubkey(),
        )
        .unwrap();
        let mint_to_ix =
            create_mint_to_instruction(&mint_key, k, &authority.pubkey(), 1000).unwrap();
        process_transaction(lwc, vec![create_ata, mint_to_ix], vec![&authority])
            .await
            .unwrap();
    }

    let create_eve =
        create_associated_token_account(&authority.pubkey(), &eve_key, &mint_key, &spl_token::id());
    process_transaction(lwc, vec![create_eve], vec![&authority])
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

    match process_transaction(lwc, vec![failed_transfer_ix], vec![&alice]).await {
        Ok(_) => panic!("transfer should fail"),
        Err(_) => {}
    };

    let eve_ix =
        create_transfer_instruction(&alice_key, &eve_key, &mint_key, &authority.pubkey(), 100)
            .unwrap();

    match process_transaction(lwc, vec![eve_ix], vec![&alice, &authority]).await {
        Ok(_) => panic!("transfer should fail"),
        Err(e) => {
            println!("{:?}", e)
        }
    };

    let successful_transfer_ix =
        create_transfer_instruction(&alice_key, &bob_key, &mint_key, &authority.pubkey(), 100)
            .unwrap();
    let burn_ix = create_burn_instruction(&mint_key, &alice_key, &authority.pubkey(), 900).unwrap();
    let close_ix =
        create_close_account_instruction(&mint_key, &alice_key, &authority.pubkey()).unwrap();

    process_transaction(
        lwc,
        vec![successful_transfer_ix, burn_ix, close_ix],
        vec![&alice, &authority],
    )
    .await
    .unwrap();
}
