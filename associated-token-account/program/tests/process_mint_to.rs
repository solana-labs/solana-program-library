#![cfg(feature = "test-bpf")]

mod program_test;

use program_test::AssociatedTokenAccountProgramTest;
use solana_program_test::tokio;

#[tokio::test]
async fn test_mint_to() {
    // Arrange
    let mut ata_test = AssociatedTokenAccountProgramTest::start_new().await;

    let mint_cookie = ata_test.bench.with_mint().await;
    let wallet_cookie = ata_test.bench.with_wallet().await;

    // Act
    let ata_cookie = ata_test
        .mint_to(&wallet_cookie, &mint_cookie, 10)
        .await
        .unwrap();

    // Assert
    //  let ata_account = ata_test.bench.get_token_account(&ata_cookie.address).await;
}
