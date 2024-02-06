#![cfg(feature = "test-sbf")]

mod program_test;

use {
    program_test::*,
    solana_program_test::tokio,
    solana_sdk::signer::Signer,
    spl_governance::{
        state::realm::SetRealmConfigItemArgs, tools::structs::SetConfigItemActionType,
    },
};

#[tokio::test]
async fn test_add_community_token_owner_record_lock_authority() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    // Act
    let token_owner_record_lock_authority_cookie = governance_test
        .with_community_token_owner_record_lock_authority(&realm_cookie)
        .await
        .unwrap();

    // Assert
    let realm_config_account = governance_test
        .get_realm_config_account(&realm_cookie.realm_config.address)
        .await;

    assert_eq!(
        1,
        realm_config_account
            .community_token_config
            .lock_authorities
            .len()
    );

    assert_eq!(
        &token_owner_record_lock_authority_cookie.authority.pubkey(),
        realm_config_account
            .community_token_config
            .lock_authorities
            .first()
            .unwrap()
    );
}

#[tokio::test]
async fn test_remove_community_token_owner_record_lock_authority() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_lock_authority_cookie = governance_test
        .with_community_token_owner_record_lock_authority(&realm_cookie)
        .await
        .unwrap();

    // Act

    let args = SetRealmConfigItemArgs::TokenOwnerRecordLockAuthority {
        action: SetConfigItemActionType::Remove,
        governing_token_mint: realm_cookie.account.community_mint,
        authority: token_owner_record_lock_authority_cookie.authority.pubkey(),
    };

    governance_test
        .set_realm_config_item(&realm_cookie, args)
        .await
        .unwrap();

    // Assert
    let realm_config_account = governance_test
        .get_realm_config_account(&realm_cookie.realm_config.address)
        .await;

    assert_eq!(
        0,
        realm_config_account
            .community_token_config
            .lock_authorities
            .len()
    );
}

#[tokio::test]
async fn test_add_council_token_owner_record_lock_authority() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    // Act
    let token_owner_record_lock_authority_cookie = governance_test
        .with_council_token_owner_record_lock_authority(&realm_cookie)
        .await
        .unwrap();

    // Assert
    let realm_config_account = governance_test
        .get_realm_config_account(&realm_cookie.realm_config.address)
        .await;

    assert_eq!(
        1,
        realm_config_account
            .council_token_config
            .lock_authorities
            .len()
    );

    assert_eq!(
        &token_owner_record_lock_authority_cookie.authority.pubkey(),
        realm_config_account
            .council_token_config
            .lock_authorities
            .first()
            .unwrap()
    );
}
