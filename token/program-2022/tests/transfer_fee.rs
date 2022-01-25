#![cfg(feature = "test-bpf")]

mod program_test;
use {
    program_test::TestContext,
    solana_program_test::tokio,
    solana_sdk::{
        instruction::InstructionError, program_option::COption, pubkey::Pubkey, signature::Signer,
        signer::keypair::Keypair, transaction::TransactionError, transport::TransportError,
    },
    spl_token_2022::{
        error::TokenError,
        extension::transfer_fee::{TransferFee, TransferFeeConfig},
        instruction,
    },
    spl_token_client::token::{ExtensionInitializationParams, TokenError as TokenClientError},
    std::convert::TryInto,
};

fn test_transfer_fee() -> TransferFee {
    TransferFee {
        epoch: 0.into(),
        transfer_fee_basis_points: 250.into(),
        maximum_fee: 10_000_000.into(),
    }
}

fn test_transfer_fee_config() -> TransferFeeConfig {
    let transfer_fee = test_transfer_fee();
    TransferFeeConfig {
        transfer_fee_config_authority: COption::Some(Pubkey::new_unique()).try_into().unwrap(),
        withdraw_withheld_authority: COption::Some(Pubkey::new_unique()).try_into().unwrap(),
        withheld_amount: 0.into(),
        older_transfer_fee: transfer_fee,
        newer_transfer_fee: transfer_fee,
    }
}

struct TransferFeeConfigWithKeypairs {
    transfer_fee_config: TransferFeeConfig,
    transfer_fee_config_authority: Keypair,
    withdraw_withheld_authority: Keypair,
}

fn test_transfer_fee_config_with_keypairs() -> TransferFeeConfigWithKeypairs {
    let transfer_fee = test_transfer_fee();
    let transfer_fee_config_authority = Keypair::new();
    let withdraw_withheld_authority = Keypair::new();
    let transfer_fee_config = TransferFeeConfig {
        transfer_fee_config_authority: COption::Some(transfer_fee_config_authority.pubkey())
            .try_into()
            .unwrap(),
        withdraw_withheld_authority: COption::Some(withdraw_withheld_authority.pubkey())
            .try_into()
            .unwrap(),
        withheld_amount: 0.into(),
        older_transfer_fee: transfer_fee,
        newer_transfer_fee: transfer_fee,
    };
    TransferFeeConfigWithKeypairs {
        transfer_fee_config,
        transfer_fee_config_authority,
        withdraw_withheld_authority,
    }
}

#[tokio::test]
async fn success_init() {
    let TransferFeeConfig {
        transfer_fee_config_authority,
        withdraw_withheld_authority,
        newer_transfer_fee,
        ..
    } = test_transfer_fee_config();
    let TestContext {
        decimals,
        mint_authority,
        token,
        ..
    } = TestContext::new(vec![ExtensionInitializationParams::TransferFeeConfig {
        transfer_fee_config_authority: transfer_fee_config_authority.into(),
        withdraw_withheld_authority: withdraw_withheld_authority.into(),
        transfer_fee_basis_points: newer_transfer_fee.transfer_fee_basis_points.into(),
        maximum_fee: newer_transfer_fee.maximum_fee.into(),
    }])
    .await
    .unwrap();

    let state = token.get_mint_info().await.unwrap();
    assert_eq!(state.base.decimals, decimals);
    assert_eq!(
        state.base.mint_authority,
        COption::Some(mint_authority.pubkey())
    );
    assert_eq!(state.base.supply, 0);
    assert!(state.base.is_initialized);
    assert_eq!(state.base.freeze_authority, COption::None);
    let extension = state.get_extension::<TransferFeeConfig>().unwrap();
    assert_eq!(
        extension.transfer_fee_config_authority,
        transfer_fee_config_authority,
    );
    assert_eq!(
        extension.withdraw_withheld_authority,
        withdraw_withheld_authority,
    );
    assert_eq!(extension.newer_transfer_fee, newer_transfer_fee);
    assert_eq!(extension.older_transfer_fee, newer_transfer_fee);
}

#[tokio::test]
async fn fail_init_default_pubkey_as_authority() {
    let TransferFeeConfig {
        transfer_fee_config_authority,
        newer_transfer_fee,
        ..
    } = test_transfer_fee_config();
    let err = TestContext::new(vec![ExtensionInitializationParams::TransferFeeConfig {
        transfer_fee_config_authority: transfer_fee_config_authority.into(),
        withdraw_withheld_authority: Some(Pubkey::default()),
        transfer_fee_basis_points: newer_transfer_fee.transfer_fee_basis_points.into(),
        maximum_fee: newer_transfer_fee.maximum_fee.into(),
    }])
    .await
    .err()
    .unwrap();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(1, InstructionError::InvalidArgument)
        )))
    );
}

#[tokio::test]
async fn set_fee() {
    let TransferFeeConfigWithKeypairs {
        transfer_fee_config_authority,
        withdraw_withheld_authority,
        transfer_fee_config: TransferFeeConfig {
            newer_transfer_fee, ..
        },
        ..
    } = test_transfer_fee_config_with_keypairs();
    let TestContext { context, token, .. } =
        TestContext::new(vec![ExtensionInitializationParams::TransferFeeConfig {
            transfer_fee_config_authority: transfer_fee_config_authority.pubkey().into(),
            withdraw_withheld_authority: withdraw_withheld_authority.pubkey().into(),
            transfer_fee_basis_points: newer_transfer_fee.transfer_fee_basis_points.into(),
            maximum_fee: newer_transfer_fee.maximum_fee.into(),
        }])
        .await
        .unwrap();

    // set to something new, old fee not touched
    let new_transfer_fee_basis_points = u16::MAX;
    let new_maximum_fee = u64::MAX;
    token
        .set_transfer_fee(
            &transfer_fee_config_authority,
            new_transfer_fee_basis_points,
            new_maximum_fee,
        )
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<TransferFeeConfig>().unwrap();
    assert_eq!(
        extension.newer_transfer_fee.transfer_fee_basis_points,
        new_transfer_fee_basis_points.into()
    );
    assert_eq!(
        extension.newer_transfer_fee.maximum_fee,
        new_maximum_fee.into()
    );
    assert_eq!(extension.older_transfer_fee, newer_transfer_fee);

    // set again, old fee still not touched
    let new_transfer_fee_basis_points = 0;
    let new_maximum_fee = 0;
    token
        .set_transfer_fee(
            &transfer_fee_config_authority,
            new_transfer_fee_basis_points,
            new_maximum_fee,
        )
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<TransferFeeConfig>().unwrap();
    assert_eq!(
        extension.newer_transfer_fee.transfer_fee_basis_points,
        new_transfer_fee_basis_points.into()
    );
    assert_eq!(
        extension.newer_transfer_fee.maximum_fee,
        new_maximum_fee.into()
    );
    assert_eq!(extension.older_transfer_fee, newer_transfer_fee);

    // warp forward one epoch, new fee becomes old fee during set
    let newer_transfer_fee = extension.newer_transfer_fee;
    context.lock().await.warp_to_slot(10_000).unwrap();
    let new_transfer_fee_basis_points = u16::MAX;
    let new_maximum_fee = u64::MAX;
    token
        .set_transfer_fee(
            &transfer_fee_config_authority,
            new_transfer_fee_basis_points,
            new_maximum_fee,
        )
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<TransferFeeConfig>().unwrap();
    assert_eq!(
        extension.newer_transfer_fee.transfer_fee_basis_points,
        new_transfer_fee_basis_points.into()
    );
    assert_eq!(
        extension.newer_transfer_fee.maximum_fee,
        new_maximum_fee.into()
    );
    assert_eq!(extension.older_transfer_fee, newer_transfer_fee);

    // fail, wrong signer
    let error = token
        .set_transfer_fee(
            &withdraw_withheld_authority,
            new_transfer_fee_basis_points,
            new_maximum_fee,
        )
        .await
        .err()
        .unwrap();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::OwnerMismatch as u32)
            )
        )))
    );
}

#[tokio::test]
async fn fail_set_fee_unsupported_mint() {
    let TestContext {
        token,
        mint_authority,
        ..
    } = TestContext::new(vec![]).await.unwrap();
    let transfer_fee_basis_points = u16::MAX;
    let maximum_fee = u64::MAX;
    let error = token
        .set_transfer_fee(&mint_authority, transfer_fee_basis_points, maximum_fee)
        .await
        .err()
        .unwrap();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(0, InstructionError::InvalidAccountData)
        )))
    );
}

#[tokio::test]
async fn set_transfer_fee_config_authority() {
    let TransferFeeConfigWithKeypairs {
        transfer_fee_config_authority,
        withdraw_withheld_authority,
        transfer_fee_config: TransferFeeConfig {
            newer_transfer_fee, ..
        },
        ..
    } = test_transfer_fee_config_with_keypairs();
    let TestContext { token, .. } =
        TestContext::new(vec![ExtensionInitializationParams::TransferFeeConfig {
            transfer_fee_config_authority: transfer_fee_config_authority.pubkey().into(),
            withdraw_withheld_authority: withdraw_withheld_authority.pubkey().into(),
            transfer_fee_basis_points: newer_transfer_fee.transfer_fee_basis_points.into(),
            maximum_fee: newer_transfer_fee.maximum_fee.into(),
        }])
        .await
        .unwrap();

    let new_authority = Keypair::new();
    let wrong = Keypair::new();

    // fail, wrong signer
    let err = token
        .set_authority(
            token.get_address(),
            Some(&new_authority.pubkey()),
            instruction::AuthorityType::TransferFeeConfig,
            &wrong,
        )
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::OwnerMismatch as u32)
            )
        )))
    );

    // success
    token
        .set_authority(
            token.get_address(),
            Some(&new_authority.pubkey()),
            instruction::AuthorityType::TransferFeeConfig,
            &transfer_fee_config_authority,
        )
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<TransferFeeConfig>().unwrap();
    assert_eq!(
        extension.transfer_fee_config_authority,
        Some(new_authority.pubkey()).try_into().unwrap(),
    );

    // assert new_authority can update transfer fee config, and old cannot
    let transfer_fee_basis_points = u16::MAX;
    let maximum_fee = u64::MAX;
    let err = token
        .set_transfer_fee(
            &transfer_fee_config_authority,
            transfer_fee_basis_points,
            maximum_fee,
        )
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::OwnerMismatch as u32)
            )
        )))
    );
    token
        .set_transfer_fee(&new_authority, transfer_fee_basis_points, maximum_fee)
        .await
        .unwrap();

    // set to none
    token
        .set_authority(
            token.get_address(),
            None,
            instruction::AuthorityType::TransferFeeConfig,
            &new_authority,
        )
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<TransferFeeConfig>().unwrap();
    assert_eq!(
        extension.transfer_fee_config_authority,
        None.try_into().unwrap(),
    );

    // fail set again
    let err = token
        .set_authority(
            token.get_address(),
            Some(&transfer_fee_config_authority.pubkey()),
            instruction::AuthorityType::TransferFeeConfig,
            &new_authority,
        )
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::AuthorityTypeNotSupported as u32)
            )
        )))
    );

    // fail update transfer fee config
    let err = token
        .set_transfer_fee(&transfer_fee_config_authority, 0, 0)
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::NoAuthorityExists as u32)
            )
        )))
    );
}

#[tokio::test]
async fn set_withdraw_withheld_authority() {
    let TransferFeeConfigWithKeypairs {
        transfer_fee_config_authority,
        withdraw_withheld_authority,
        transfer_fee_config: TransferFeeConfig {
            newer_transfer_fee, ..
        },
        ..
    } = test_transfer_fee_config_with_keypairs();
    let TestContext { token, .. } =
        TestContext::new(vec![ExtensionInitializationParams::TransferFeeConfig {
            transfer_fee_config_authority: transfer_fee_config_authority.pubkey().into(),
            withdraw_withheld_authority: withdraw_withheld_authority.pubkey().into(),
            transfer_fee_basis_points: newer_transfer_fee.transfer_fee_basis_points.into(),
            maximum_fee: newer_transfer_fee.maximum_fee.into(),
        }])
        .await
        .unwrap();

    let new_authority = Keypair::new();
    let wrong = Keypair::new();

    // fail, wrong signer
    let err = token
        .set_authority(
            token.get_address(),
            Some(&new_authority.pubkey()),
            instruction::AuthorityType::WithheldWithdraw,
            &wrong,
        )
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::OwnerMismatch as u32)
            )
        )))
    );

    // success
    token
        .set_authority(
            token.get_address(),
            Some(&new_authority.pubkey()),
            instruction::AuthorityType::WithheldWithdraw,
            &withdraw_withheld_authority,
        )
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<TransferFeeConfig>().unwrap();
    assert_eq!(
        extension.withdraw_withheld_authority,
        Some(new_authority.pubkey()).try_into().unwrap(),
    );

    // TODO: assert new authority can withdraw withheld fees and old cannot

    // set to none
    token
        .set_authority(
            token.get_address(),
            None,
            instruction::AuthorityType::WithheldWithdraw,
            &new_authority,
        )
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<TransferFeeConfig>().unwrap();
    assert_eq!(
        extension.withdraw_withheld_authority,
        None.try_into().unwrap(),
    );

    // fail set again
    let err = token
        .set_authority(
            token.get_address(),
            Some(&withdraw_withheld_authority.pubkey()),
            instruction::AuthorityType::WithheldWithdraw,
            &new_authority,
        )
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::AuthorityTypeNotSupported as u32)
            )
        )))
    );

    // TODO: assert no authority can withdraw withheld fees
}
