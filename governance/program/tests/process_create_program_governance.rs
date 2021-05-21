#![cfg(feature = "test-bpf")]
mod program_test;

use solana_program::pubkey::Pubkey;
use solana_program_test::*;

use program_test::{tools::ProgramInstructionError, *};
use solana_sdk::signature::Keypair;
use spl_governance::{
    error::GovernanceError, tools::bpf_loader_upgradeable::get_program_upgrade_authority,
};

#[tokio::test]
async fn test_program_governance_created() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_program_cookie = governance_test.with_governed_program().await;

    // Act
    let program_governance_cookie = governance_test
        .with_program_governance(&realm_cookie, &governed_program_cookie)
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
async fn test_program_governance_with_incorrect_upgrade_authority_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let mut governed_program_cookie = governance_test.with_governed_program().await;

    governed_program_cookie.upgrade_authority = Keypair::new();

    // Act
    let err = governance_test
        .with_program_governance(&realm_cookie, &governed_program_cookie)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, ProgramInstructionError::IncorrectAuthority.into());
}

#[tokio::test]
async fn test_program_governance_with_with_invalid_realm_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let mut realm_cookie = governance_test.with_realm().await;
    let governed_program_cookie = governance_test.with_governed_program().await;

    let program_governance_cookie = governance_test
        .with_program_governance(&realm_cookie, &governed_program_cookie)
        .await
        .unwrap();

    realm_cookie.address = program_governance_cookie.address;

    // Act
    let err = governance_test
        .with_program_governance(&realm_cookie, &governed_program_cookie)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::InvalidAccountType.into());
}
