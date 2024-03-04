#![cfg(feature = "test-sbf")]

mod program_test;

use {
    program_test::*,
    solana_program::pubkey::Pubkey,
    solana_program_test::tokio,
    solana_sdk::{signature::Keypair, signer::Signer},
    spl_governance::{
        error::GovernanceError,
        state::{
            enums::GovernanceAccountType,
            realm::SetRealmConfigItemArgs,
            realm_config::{GoverningTokenConfig, RealmConfigAccount},
        },
        tools::structs::{Reserved110, SetConfigItemActionType},
    },
    spl_governance_tools::account::AccountMaxSize,
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

#[tokio::test]
async fn test_set_realm_config_item_with_realm_authority_must_sign_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let args = SetRealmConfigItemArgs::TokenOwnerRecordLockAuthority {
        action: SetConfigItemActionType::Add,
        governing_token_mint: realm_cookie.account.community_mint,
        authority: Keypair::new().pubkey(),
    };

    // Act
    let err = governance_test
        .set_realm_config_item_using_ix(
            &realm_cookie,
            args,
            |i| i.accounts[2].is_signer = false,
            Some(&[]),
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::RealmAuthorityMustSign.into());
}

#[tokio::test]
async fn test_set_realm_config_item_with_invalid_realm_authority_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let args = SetRealmConfigItemArgs::TokenOwnerRecordLockAuthority {
        action: SetConfigItemActionType::Add,
        governing_token_mint: realm_cookie.account.community_mint,
        authority: Keypair::new().pubkey(),
    };

    let realm_authority = Keypair::new();

    // Act
    let err = governance_test
        .set_realm_config_item_using_ix(
            &realm_cookie,
            args,
            |i| i.accounts[2].pubkey = realm_authority.pubkey(),
            Some(&[&realm_authority]),
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::InvalidAuthorityForRealm.into());
}

#[tokio::test]
async fn test_set_realm_config_item_with_invalid_realm_config_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let args = SetRealmConfigItemArgs::TokenOwnerRecordLockAuthority {
        action: SetConfigItemActionType::Add,
        governing_token_mint: realm_cookie.account.community_mint,
        authority: Keypair::new().pubkey(),
    };

    let realm_cookie2 = governance_test.with_realm().await;

    // Act
    let err = governance_test
        .set_realm_config_item_using_ix(
            &realm_cookie,
            args,
            |i| i.accounts[1].pubkey = realm_cookie2.realm_config.address,
            None,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::InvalidRealmConfigForRealm.into());
}

#[tokio::test]
async fn test_add_token_owner_record_lock_authority_with_invalid_governing_token_mint() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let args = SetRealmConfigItemArgs::TokenOwnerRecordLockAuthority {
        action: SetConfigItemActionType::Add,
        governing_token_mint: Pubkey::new_unique(), // Use invalid mint
        authority: Keypair::new().pubkey(),
    };

    // Act
    let err = governance_test
        .set_realm_config_item(&realm_cookie, args)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::InvalidGoverningTokenMint.into());
}

#[tokio::test]
async fn test_add_token_owner_record_lock_authority_with_authority_already_exists_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let args = SetRealmConfigItemArgs::TokenOwnerRecordLockAuthority {
        action: SetConfigItemActionType::Add,
        governing_token_mint: realm_cookie.account.config.council_mint.unwrap(),
        // Set the same authority
        authority: Pubkey::new_unique(),
    };

    governance_test
        .set_realm_config_item(&realm_cookie, args.clone())
        .await
        .unwrap();

    // Advance the clock to accept the same transaction
    governance_test.advance_clock().await;

    // Act
    let err = governance_test
        .set_realm_config_item(&realm_cookie, args)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        err,
        GovernanceError::TokenOwnerRecordLockAuthorityAlreadyExists.into()
    );
}

#[tokio::test]
async fn test_set_realm_config_item_without_existing_realm_config() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    governance_test.remove_realm_config_account(&realm_cookie.realm_config.address);

    // Act
    governance_test
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
}

#[tokio::test]
async fn test_set_realm_config_item_with_extended_account_size() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    // Act
    governance_test
        .with_community_token_owner_record_lock_authority(&realm_cookie)
        .await
        .unwrap();

    // Assert
    let realm_config_account = governance_test
        .bench
        .get_account(&realm_cookie.realm_config.address)
        .await
        .unwrap();

    // RealmConfig without any lock authorities
    let realm_config = RealmConfigAccount {
        account_type: GovernanceAccountType::RealmConfig,
        realm: realm_cookie.address,
        community_token_config: GoverningTokenConfig::default(),
        council_token_config: GoverningTokenConfig::default(),
        reserved: Reserved110::default(),
    };

    assert_eq!(
        realm_config.get_max_size().unwrap() + 32,
        realm_config_account.data.len()
    );
}

#[tokio::test]
async fn test_remove_token_owner_record_lock_authority_with_authority_not_found_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_lock_authority_cookie = governance_test
        .with_community_token_owner_record_lock_authority(&realm_cookie)
        .await
        .unwrap();

    let args = SetRealmConfigItemArgs::TokenOwnerRecordLockAuthority {
        action: SetConfigItemActionType::Remove,
        governing_token_mint: realm_cookie.account.community_mint,
        authority: token_owner_record_lock_authority_cookie.authority.pubkey(),
    };

    governance_test
        .set_realm_config_item(&realm_cookie, args.clone())
        .await
        .unwrap();

    // Advance the clock to accept the same transaction
    governance_test.advance_clock().await;

    // Act
    let err = governance_test
        .set_realm_config_item(&realm_cookie, args)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        err,
        GovernanceError::TokenOwnerRecordLockAuthorityNotFound.into()
    );
}
