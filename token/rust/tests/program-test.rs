use solana_program_test::{
    tokio::{self, sync::Mutex},
    ProgramTest,
};
use solana_sdk::{
    program_option::COption,
    signer::{keypair::Keypair, Signer},
};
use spl_token_2022::{instruction, state};
use spl_token_client::{
    client::{ProgramBanksClient, ProgramBanksClientProcessTransaction, ProgramClient},
    token::Token,
};
use std::sync::Arc;

struct TestContext {
    pub decimals: u8,
    pub mint_authority: Keypair,
    pub token: Token<ProgramBanksClientProcessTransaction, Keypair>,

    pub alice: Keypair,
    pub bob: Keypair,
}

impl TestContext {
    async fn new() -> Self {
        let program_test = ProgramTest::default();
        let ctx = program_test.start_with_context().await;
        let ctx = Arc::new(Mutex::new(ctx));

        let payer = keypair_clone(&ctx.lock().await.payer);

        let client: Arc<dyn ProgramClient<ProgramBanksClientProcessTransaction>> =
            Arc::new(ProgramBanksClient::new_from_context(
                Arc::clone(&ctx),
                ProgramBanksClientProcessTransaction,
            ));

        let decimals: u8 = 6;

        let mint_account = Keypair::new();
        let mint_authority = Keypair::new();
        let mint_authority_pubkey = mint_authority.pubkey();

        let token = Token::create_mint(
            Arc::clone(&client),
            keypair_clone(&payer),
            &mint_account,
            &mint_authority_pubkey,
            None,
            decimals,
            vec![],
        )
        .await
        .expect("failed to create mint");

        Self {
            decimals,
            mint_authority,
            token,

            alice: Keypair::new(),
            bob: Keypair::new(),
        }
    }
}

fn keypair_clone(kp: &Keypair) -> Keypair {
    Keypair::from_bytes(&kp.to_bytes()).expect("failed to copy keypair")
}

// TODO unignore once spl-token-2022 becomes spl-token, and is included in
// ProgramTest by default
#[ignore]
#[tokio::test]
async fn associated_token_account() {
    let TestContext { token, alice, .. } = TestContext::new().await;

    let alice_vault = token
        .create_associated_token_account(&alice.pubkey())
        .await
        .expect("failed to create associated token account");

    assert_eq!(
        token.get_associated_token_address(&alice.pubkey()),
        alice_vault
    );

    assert_eq!(
        token
            .get_account_info(alice_vault)
            .await
            .expect("failed to get account info"),
        state::Account {
            mint: *token.get_address(),
            owner: alice.pubkey(),
            amount: 0,
            delegate: COption::None,
            state: state::AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        }
    );
}

// TODO unignore once spl-token-2022 becomes spl-token, and is included in
// ProgramTest by default
#[ignore]
#[tokio::test]
async fn get_or_create_associated_token_account() {
    let TestContext { token, alice, .. } = TestContext::new().await;

    assert_eq!(
        token
            .get_or_create_associated_account_info(&alice.pubkey())
            .await
            .expect("failed to get account info"),
        state::Account {
            mint: *token.get_address(),
            owner: alice.pubkey(),
            amount: 0,
            delegate: COption::None,
            state: state::AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        }
    );
}

// TODO unignore once spl-token-2022 becomes spl-token, and is included in
// ProgramTest by default
#[ignore]
#[tokio::test]
async fn set_authority() {
    let TestContext {
        mint_authority,
        token,
        alice,
        bob,
        ..
    } = TestContext::new().await;

    let alice_vault = token
        .create_associated_token_account(&alice.pubkey())
        .await
        .expect("failed to create associated token account");

    token
        .mint_to(&alice_vault, &mint_authority, 1)
        .await
        .expect("failed to mint token");

    token
        .set_authority(
            token.get_address(),
            None,
            instruction::AuthorityType::MintTokens,
            &mint_authority,
        )
        .await
        .expect("failed to set authority");

    let mint = token
        .get_mint_info()
        .await
        .expect("failed to get mint info");
    assert!(mint.base.mint_authority.is_none());

    // TODO: compare
    // Err(Client(TransactionError(InstructionError(0, Custom(5)))))
    assert!(token
        .mint_to(&alice_vault, &mint_authority, 2)
        .await
        .is_err());

    token
        .set_authority(
            &alice_vault,
            Some(&bob.pubkey()),
            instruction::AuthorityType::AccountOwner,
            &alice,
        )
        .await
        .expect("failed to set_authority");

    assert_eq!(
        token
            .get_account_info(alice_vault)
            .await
            .expect("failed to get account info")
            .owner,
        bob.pubkey(),
    );
}

// TODO unignore once spl-token-2022 becomes spl-token, and is included in
// ProgramTest by default
#[ignore]
#[tokio::test]
async fn mint_to() {
    let TestContext {
        decimals,
        mint_authority,
        token,
        alice,
        ..
    } = TestContext::new().await;

    let alice_vault = token
        .create_associated_token_account(&alice.pubkey())
        .await
        .expect("failed to create associated token account");

    let mint_amount = 10 * u64::pow(10, decimals as u32);
    token
        .mint_to(&alice_vault, &mint_authority, mint_amount)
        .await
        .expect("failed to mint token");

    assert_eq!(
        token
            .get_account_info(alice_vault)
            .await
            .expect("failed to get account")
            .amount,
        mint_amount
    );
}

// TODO unignore once spl-token-2022 becomes spl-token, and is included in
// ProgramTest by default
#[ignore]
#[tokio::test]
async fn transfer() {
    let TestContext {
        decimals,
        mint_authority,
        token,
        alice,
        bob,
        ..
    } = TestContext::new().await;

    let alice_vault = token
        .create_associated_token_account(&alice.pubkey())
        .await
        .expect("failed to create associated token account");
    let bob_vault = token
        .create_associated_token_account(&bob.pubkey())
        .await
        .expect("failed to create associated token account");

    let mint_amount = 10 * u64::pow(10, decimals as u32);
    token
        .mint_to(&alice_vault, &mint_authority, mint_amount)
        .await
        .expect("failed to mint token");

    let transfer_amount = mint_amount.overflowing_div(3).0;
    token
        .transfer_checked(&alice_vault, &bob_vault, &alice, transfer_amount, decimals)
        .await
        .expect("failed to transfer");

    assert_eq!(
        token
            .get_account_info(alice_vault)
            .await
            .expect("failed to get account")
            .amount,
        mint_amount - transfer_amount
    );
    assert_eq!(
        token
            .get_account_info(bob_vault)
            .await
            .expect("failed to get account")
            .amount,
        transfer_amount
    );
}
