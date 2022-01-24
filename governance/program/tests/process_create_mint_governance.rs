#![cfg(feature = "test-bpf")]
mod program_test;

use solana_program_test::*;

use program_test::*;
use solana_sdk::{signature::Keypair, signer::Signer};
use spl_governance::error::GovernanceError;
use spl_governance_tools::error::GovernanceToolsError;
use spl_token::error::TokenError;

#[tokio::test]
async fn test_create_mint_governance() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_mint_cookie = governance_test.with_governed_mint().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // Act
    let mint_governance_cookie = governance_test
        .with_mint_governance(
            &realm_cookie,
            &governed_mint_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    // // Assert
    let mint_governance_account = governance_test
        .get_governance_account(&mint_governance_cookie.address)
        .await;

    assert_eq!(mint_governance_cookie.account, mint_governance_account);

    let mint_account = governance_test
        .get_mint_account(&governed_mint_cookie.address)
        .await;

    assert_eq!(
        mint_governance_cookie.address,
        mint_account.mint_authority.unwrap()
    );
}

#[tokio::test]
async fn test_create_mint_governance_without_transferring_mint_authority() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let mut governed_mint_cookie = governance_test.with_governed_mint().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    governed_mint_cookie.transfer_mint_authority = false;
    // Act
    let mint_governance_cookie = governance_test
        .with_mint_governance(
            &realm_cookie,
            &governed_mint_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    // // Assert
    let mint_governance_account = governance_test
        .get_governance_account(&mint_governance_cookie.address)
        .await;

    assert_eq!(mint_governance_cookie.account, mint_governance_account);

    let mint_account = governance_test
        .get_mint_account(&governed_mint_cookie.address)
        .await;

    assert_eq!(
        governed_mint_cookie.mint_authority.pubkey(),
        mint_account.mint_authority.unwrap()
    );
}

#[tokio::test]
async fn test_create_mint_governance_without_transferring_mint_authority_with_invalid_authority_error(
) {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let mut governed_mint_cookie = governance_test.with_governed_mint().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    governed_mint_cookie.transfer_mint_authority = false;
    governed_mint_cookie.mint_authority = Keypair::new();

    // Act
    let err = governance_test
        .with_mint_governance(
            &realm_cookie,
            &governed_mint_cookie,
            &token_owner_record_cookie,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::InvalidMintAuthority.into());
}

#[tokio::test]
async fn test_create_mint_governance_without_transferring_mint_authority_with_authority_not_signed_error(
) {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let mut governed_mint_cookie = governance_test.with_governed_mint().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    governed_mint_cookie.transfer_mint_authority = false;

    // Act
    let err = governance_test
        .with_mint_governance_using_instruction(
            &realm_cookie,
            &governed_mint_cookie,
            &token_owner_record_cookie,
            |i| {
                i.accounts[3].is_signer = false; // governed_mint_authority
            },
            Some(&[&token_owner_record_cookie.token_owner]),
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::MintAuthorityMustSign.into());
}

#[tokio::test]
async fn test_create_mint_governance_with_invalid_mint_authority_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let mut governed_mint_cookie = governance_test.with_governed_mint().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    governed_mint_cookie.mint_authority = Keypair::new();

    // Act
    let err = governance_test
        .with_mint_governance(
            &realm_cookie,
            &governed_mint_cookie,
            &token_owner_record_cookie,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, TokenError::OwnerMismatch.into());
}

#[tokio::test]
async fn test_create_mint_governance_with_invalid_realm_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let mut realm_cookie = governance_test.with_realm().await;
    let governed_mint_cookie = governance_test.with_governed_mint().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mint_governance_cookie = governance_test
        .with_mint_governance(
            &realm_cookie,
            &governed_mint_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    // try to use Governance account other than Realm as realm
    realm_cookie.address = mint_governance_cookie.address;

    // Act
    let err = governance_test
        .with_mint_governance(
            &realm_cookie,
            &governed_mint_cookie,
            &token_owner_record_cookie,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceToolsError::InvalidAccountType.into());
}

#[tokio::test]
async fn test_create_mint_governance_with_freeze_authority_transfer() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_mint_cookie = governance_test.with_freezable_governed_mint().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // Act
    let mint_governance_cookie = governance_test
        .with_mint_governance(
            &realm_cookie,
            &governed_mint_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    // // Assert
    let mint_governance_account = governance_test
        .get_governance_account(&mint_governance_cookie.address)
        .await;

    assert_eq!(mint_governance_cookie.account, mint_governance_account);

    let mint_account = governance_test
        .get_mint_account(&governed_mint_cookie.address)
        .await;

    assert_eq!(
        mint_governance_cookie.address,
        mint_account.mint_authority.unwrap()
    );

    assert_eq!(
        mint_governance_cookie.address,
        mint_account.freeze_authority.unwrap()
    );
}
