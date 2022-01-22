#![cfg(feature = "test-bpf")]

mod program_test;
use {
    program_test::TestContext,
    solana_program_test::tokio,
    solana_sdk::{
        instruction::InstructionError, program_option::COption, pubkey::Pubkey, signature::Signer,
        transaction::TransactionError, transport::TransportError,
    },
    spl_token_2022::extension::transfer_fee::{TransferFee, TransferFeeConfig},
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
        older_transfer_fee: transfer_fee.clone(),
        newer_transfer_fee: transfer_fee,
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
        transfer_fee_config_authority.try_into().unwrap(),
    );
    assert_eq!(
        extension.withdraw_withheld_authority,
        withdraw_withheld_authority.try_into().unwrap(),
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
    .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(1, InstructionError::InvalidArgument)
        )))
    );
}
