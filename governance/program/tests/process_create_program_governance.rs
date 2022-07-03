#![cfg(feature = "test-bpf")]
mod program_test;

use solana_program_test::*;

use program_test::*;
use solana_sdk::signature::{Keypair, Signer};
use spl_governance::{
    error::GovernanceError, tools::bpf_loader_upgradeable::get_program_upgrade_authority,
};
use spl_governance_test_sdk::tools::ProgramInstructionError;
use spl_governance_tools::error::GovernanceToolsError;

#[tokio::test]
async fn test_create_program_governance() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_program_cookie = governance_test.with_governed_program().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // Act
    let program_governance_cookie = governance_test
        .with_program_governance(
            &realm_cookie,
            &governed_program_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    // Assert
    let program_governance_account = governance_test
        .get_governance_account(&program_governance_cookie.address)
        .await;

    assert_eq!(
        program_governance_cookie.account,
        program_governance_account
    );

    let program_data = governance_test
        .get_upgradable_loader_account(&governed_program_cookie.data_address)
        .await;

    let upgrade_authority = get_program_upgrade_authority(&program_data).unwrap();

    assert_eq!(Some(program_governance_cookie.address), upgrade_authority);
}

#[tokio::test]
async fn test_create_program_governance_without_transferring_upgrade_authority() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let mut governed_program_cookie = governance_test.with_governed_program().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    governed_program_cookie.transfer_upgrade_authority = false;

    // Act
    let program_governance_cookie = governance_test
        .with_program_governance(
            &realm_cookie,
            &governed_program_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    // Assert
    let program_governance_account = governance_test
        .get_governance_account(&program_governance_cookie.address)
        .await;

    assert_eq!(
        program_governance_cookie.account,
        program_governance_account
    );

    let program_data = governance_test
        .get_upgradable_loader_account(&governed_program_cookie.data_address)
        .await;

    let upgrade_authority = get_program_upgrade_authority(&program_data).unwrap();

    assert_eq!(
        Some(governed_program_cookie.upgrade_authority.pubkey()),
        upgrade_authority
    );
}

#[tokio::test]
async fn test_create_program_governance_without_transferring_upgrade_authority_with_invalid_authority_error(
) {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let mut governed_program_cookie = governance_test.with_governed_program().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    governed_program_cookie.transfer_upgrade_authority = false;
    governed_program_cookie.upgrade_authority = Keypair::new();

    // Act
    let err = governance_test
        .with_program_governance(
            &realm_cookie,
            &governed_program_cookie,
            &token_owner_record_cookie,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::InvalidUpgradeAuthority.into());
}

#[tokio::test]
async fn test_create_program_governance_without_transferring_upgrade_authority_with_authority_not_signed_error(
) {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let mut governed_program_cookie = governance_test.with_governed_program().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    governed_program_cookie.transfer_upgrade_authority = false;

    // Act
    let err = governance_test
        .with_program_governance_using_instruction(
            &realm_cookie,
            &governed_program_cookie,
            &token_owner_record_cookie,
            |i| {
                i.accounts[4].is_signer = false; // governed_program_upgrade_authority
            },
            Some(&[&token_owner_record_cookie.token_owner]),
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::UpgradeAuthorityMustSign.into());
}

#[tokio::test]
async fn test_create_program_governance_with_incorrect_upgrade_authority_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let mut governed_program_cookie = governance_test.with_governed_program().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    governed_program_cookie.upgrade_authority = Keypair::new();

    // Act
    let err = governance_test
        .with_program_governance(
            &realm_cookie,
            &governed_program_cookie,
            &token_owner_record_cookie,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, ProgramInstructionError::IncorrectAuthority.into());
}

#[tokio::test]
async fn test_create_program_governance_with_invalid_realm_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let mut realm_cookie = governance_test.with_realm().await;
    let governed_program_cookie = governance_test.with_governed_program().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let program_governance_cookie = governance_test
        .with_program_governance(
            &realm_cookie,
            &governed_program_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    realm_cookie.address = program_governance_cookie.address;

    // Act
    let err = governance_test
        .with_program_governance(
            &realm_cookie,
            &governed_program_cookie,
            &token_owner_record_cookie,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceToolsError::InvalidAccountType.into());
}
