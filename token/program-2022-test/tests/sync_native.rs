#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{TestContext, TokenContext},
    solana_program_test::{
        tokio::{self, sync::Mutex},
        ProgramTestContext,
    },
    solana_sdk::{
        pubkey::Pubkey, signature::Signer, signer::keypair::Keypair, system_instruction,
        transaction::Transaction,
    },
    spl_token_2022::extension::ExtensionType,
    spl_token_client::{client::ProgramBanksClientProcessTransaction, token::Token},
    std::sync::Arc,
};

async fn run_basic(
    token: Token<ProgramBanksClientProcessTransaction>,
    context: Arc<Mutex<ProgramTestContext>>,
    account: Pubkey,
) {
    let account_info = token.get_account_info(&account).await.unwrap();
    assert_eq!(account_info.base.amount, 0);

    // system transfer to account
    let amount = 1_000;
    {
        let context = context.lock().await;
        let instructions = vec![system_instruction::transfer(
            &context.payer.pubkey(),
            &account,
            amount,
        )];
        let tx = Transaction::new_signed_with_payer(
            &instructions,
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );
        context.banks_client.process_transaction(tx).await.unwrap();
    }
    let account_info = token.get_account_info(&account).await.unwrap();
    assert_eq!(account_info.base.amount, 0);

    token.sync_native(&account).await.unwrap();
    let account_info = token.get_account_info(&account).await.unwrap();
    assert_eq!(account_info.base.amount, amount);
}

#[tokio::test]
async fn basic() {
    let mut context = TestContext::new().await;
    context.init_token_with_native_mint().await.unwrap();
    let TokenContext { token, alice, .. } = context.token_context.unwrap();
    let context = context.context.clone();

    let account = Keypair::new();
    token
        .create_auxiliary_token_account(&account, &alice.pubkey())
        .await
        .unwrap();
    let account = account.pubkey();
    run_basic(token, context, account).await;
}

#[tokio::test]
async fn basic_with_extension() {
    let mut context = TestContext::new().await;
    context.init_token_with_native_mint().await.unwrap();
    let TokenContext { token, alice, .. } = context.token_context.unwrap();
    let context = context.context.clone();

    let account = Keypair::new();
    token
        .create_auxiliary_token_account_with_extension_space(
            &account,
            &alice.pubkey(),
            vec![ExtensionType::ImmutableOwner],
        )
        .await
        .unwrap();
    let account = account.pubkey();
    run_basic(token, context, account).await;
}
