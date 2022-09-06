use lightweight_solana_client::{LightweightClientResult, LightweightSolanaClient};
use solana_program_test::*;
use solana_sdk::{
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::Signature,
    signature::{Keypair, Signer},
    system_instruction,
};

use permissioned_token::instruction::*;
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account,
};

pub fn sol(amount: f64) -> u64 {
    (amount * LAMPORTS_PER_SOL as f64) as u64
}

pub async fn airdrop(
    context: &LightweightSolanaClient,
    receiver: &Pubkey,
    amount: u64,
) -> LightweightClientResult<Signature> {
    let ixs = vec![system_instruction::transfer(
        &context.payer.pubkey(),
        receiver,
        amount,
    )];
    context.sign_send_instructions(ixs, vec![]).await
}

pub fn permissioned_token_test() -> ProgramTest {
    ProgramTest::new("permissioned_token", permissioned_token::id(), None)
}

#[tokio::test]
async fn test_permissioned_token_basic() {
    let context = permissioned_token_test().start_with_context().await;
    let lwc = LightweightSolanaClient::from_banks(&context.banks_client, &context.payer)
        .await
        .unwrap();
    let authority = Keypair::new();
    airdrop(&lwc, &authority.pubkey(), sol(10.0)).await.unwrap();
    let mint = Keypair::new();
    let mint_key = mint.pubkey();
    let create_ix =
        create_initialize_mint_instruction(&mint_key, &authority.pubkey(), &authority.pubkey(), 0)
            .unwrap();
    lwc.sign_send_instructions(vec![create_ix], vec![&authority, &mint])
        .await
        .unwrap();

    let alice = Keypair::new();
    let alice_key = alice.pubkey();
    let bob = Keypair::new();
    let bob_key = bob.pubkey();
    let eve = Keypair::new();
    let eve_key = eve.pubkey();

    for k in [&alice_key, &bob_key] {
        airdrop(&lwc, k, sol(1.0)).await.unwrap();
        let create_ata = create_initialize_account_instruction(
            &mint_key,
            k,
            &authority.pubkey(),
            &authority.pubkey(),
        )
        .unwrap();
        let mint_to_ix =
            create_mint_to_instruction(&mint_key, k, &authority.pubkey(), 1000).unwrap();
        lwc.sign_send_instructions(vec![create_ata, mint_to_ix], vec![&authority])
            .await
            .unwrap();
    }

    let create_eve =
        create_associated_token_account(&authority.pubkey(), &eve_key, &mint_key, &spl_token::id());
    lwc.sign_send_instructions(vec![create_eve], vec![&authority])
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

    match lwc
        .sign_send_instructions(vec![failed_transfer_ix], vec![&alice])
        .await
    {
        Ok(_) => panic!("transfer should fail"),
        Err(_) => {}
    };

    let eve_ix =
        create_transfer_instruction(&alice_key, &eve_key, &mint_key, &authority.pubkey(), 100)
            .unwrap();

    match lwc
        .sign_send_instructions(vec![eve_ix], vec![&alice, &authority])
        .await
    {
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

    lwc.sign_send_instructions(
        vec![successful_transfer_ix, burn_ix, close_ix],
        vec![&alice, &authority],
    )
    .await
    .unwrap();
}
