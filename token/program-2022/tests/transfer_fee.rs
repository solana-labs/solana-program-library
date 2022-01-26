#![cfg(feature = "test-bpf")]

mod program_test;
use {
    program_test::{TestContext, TokenContext},
    solana_program_test::tokio,
    solana_sdk::{
        instruction::InstructionError, program_option::COption, pubkey::Pubkey, signature::Signer,
        signer::keypair::Keypair, transaction::TransactionError, transport::TransportError,
    },
    spl_token_2022::{
        error::TokenError,
        extension::transfer_fee::{
            TransferFee, TransferFeeAmount, TransferFeeConfig, MAX_FEE_BASIS_POINTS,
        },
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
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::TransferFeeConfig {
            transfer_fee_config_authority: transfer_fee_config_authority.into(),
            withdraw_withheld_authority: withdraw_withheld_authority.into(),
            transfer_fee_basis_points: newer_transfer_fee.transfer_fee_basis_points.into(),
            maximum_fee: newer_transfer_fee.maximum_fee.into(),
        }])
        .await
        .unwrap();
    let TokenContext {
        decimals,
        mint_authority,
        token,
        ..
    } = context.token_context.unwrap();

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
    let mut context = TestContext::new().await;
    let err = context
        .init_token_with_mint(vec![ExtensionInitializationParams::TransferFeeConfig {
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

#[tokio::test]
async fn fail_init_fee_too_high() {
    let TransferFeeConfig {
        transfer_fee_config_authority,
        withdraw_withheld_authority,
        newer_transfer_fee,
        ..
    } = test_transfer_fee_config();
    let mut context = TestContext::new().await;
    let err = context
        .init_token_with_mint(vec![ExtensionInitializationParams::TransferFeeConfig {
            transfer_fee_config_authority: transfer_fee_config_authority.into(),
            withdraw_withheld_authority: withdraw_withheld_authority.into(),
            transfer_fee_basis_points: MAX_FEE_BASIS_POINTS + 1,
            maximum_fee: newer_transfer_fee.maximum_fee.into(),
        }])
        .await
        .unwrap_err();
    assert_eq!(
        err,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                1,
                InstructionError::Custom(TokenError::TransferFeeExceedsMaximum as u32)
            )
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
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::TransferFeeConfig {
            transfer_fee_config_authority: transfer_fee_config_authority.pubkey().into(),
            withdraw_withheld_authority: withdraw_withheld_authority.pubkey().into(),
            transfer_fee_basis_points: newer_transfer_fee.transfer_fee_basis_points.into(),
            maximum_fee: newer_transfer_fee.maximum_fee.into(),
        }])
        .await
        .unwrap();
    let token = context.token_context.unwrap().token;

    // set to something new, old fee not touched
    let new_transfer_fee_basis_points = MAX_FEE_BASIS_POINTS;
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
    context.context.lock().await.warp_to_slot(10_000).unwrap();
    let new_transfer_fee_basis_points = MAX_FEE_BASIS_POINTS;
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

    // fail, set too high
    let error = token
        .set_transfer_fee(
            &transfer_fee_config_authority,
            MAX_FEE_BASIS_POINTS + 1,
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
                InstructionError::Custom(TokenError::TransferFeeExceedsMaximum as u32)
            )
        )))
    );
}

#[tokio::test]
async fn fail_set_fee_unsupported_mint() {
    let mut context = TestContext::new().await;
    context.init_token_with_mint(vec![]).await.unwrap();
    let TokenContext {
        mint_authority,
        token,
        ..
    } = context.token_context.unwrap();
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
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::TransferFeeConfig {
            transfer_fee_config_authority: transfer_fee_config_authority.pubkey().into(),
            withdraw_withheld_authority: withdraw_withheld_authority.pubkey().into(),
            transfer_fee_basis_points: newer_transfer_fee.transfer_fee_basis_points.into(),
            maximum_fee: newer_transfer_fee.maximum_fee.into(),
        }])
        .await
        .unwrap();
    let token = context.token_context.unwrap().token;

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
    let transfer_fee_basis_points = MAX_FEE_BASIS_POINTS;
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
    let mut context = TestContext::new().await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::TransferFeeConfig {
            transfer_fee_config_authority: transfer_fee_config_authority.pubkey().into(),
            withdraw_withheld_authority: withdraw_withheld_authority.pubkey().into(),
            transfer_fee_basis_points: newer_transfer_fee.transfer_fee_basis_points.into(),
            maximum_fee: newer_transfer_fee.maximum_fee.into(),
        }])
        .await
        .unwrap();
    let token = context.token_context.unwrap().token;

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

#[tokio::test]
async fn transfer_checked() {
    let TransferFeeConfigWithKeypairs {
        transfer_fee_config_authority,
        withdraw_withheld_authority,
        transfer_fee_config,
        ..
    } = test_transfer_fee_config_with_keypairs();
    let mut context = TestContext::new().await;
    let transfer_fee_basis_points = u16::from(
        transfer_fee_config
            .newer_transfer_fee
            .transfer_fee_basis_points,
    );
    let maximum_fee = u64::from(transfer_fee_config.newer_transfer_fee.maximum_fee);
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::TransferFeeConfig {
            transfer_fee_config_authority: transfer_fee_config_authority.pubkey().into(),
            withdraw_withheld_authority: withdraw_withheld_authority.pubkey().into(),
            transfer_fee_basis_points,
            maximum_fee,
        }])
        .await
        .unwrap();
    let TokenContext {
        decimals,
        mint_authority,
        token,
        alice,
        bob,
        ..
    } = context.token_context.unwrap();

    // token account is self-owned just to test another case
    let alice_account = token
        .create_auxiliary_token_account(&alice, &alice.pubkey())
        .await
        .unwrap();
    let bob_account = Keypair::new();
    let bob_account = token
        .create_auxiliary_token_account(&bob_account, &bob.pubkey())
        .await
        .unwrap();

    // mint a lot of tokens, 100x max fee
    let mut alice_amount = maximum_fee * 100;
    token
        .mint_to(&alice_account, &mint_authority, alice_amount)
        .await
        .unwrap();

    // fail unchecked always
    let error = token
        .transfer_unchecked(&alice_account, &bob_account, &alice, maximum_fee)
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::MintRequiredForTransfer as u32)
            )
        )))
    );

    // fail because amount too high
    let error = token
        .transfer_checked(
            &alice_account,
            &bob_account,
            &alice,
            alice_amount + 1,
            decimals,
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::InsufficientFunds as u32)
            )
        )))
    );

    let mut withheld_amount = 0;
    let mut transferred_amount = 0;

    // success, clean calculation for transfer fee
    let fee = transfer_fee_config
        .calculate_epoch_fee(0, maximum_fee)
        .unwrap();
    token
        .transfer_checked(&alice_account, &bob_account, &alice, maximum_fee, decimals)
        .await
        .unwrap();
    alice_amount -= maximum_fee;
    withheld_amount += fee;
    transferred_amount += maximum_fee - fee;

    let alice_state = token.get_account_info(&alice_account).await.unwrap();
    assert_eq!(alice_state.base.amount, alice_amount);
    let extension = alice_state.get_extension::<TransferFeeAmount>().unwrap();
    assert_eq!(extension.withheld_amount, 0.into());
    let bob_state = token.get_account_info(&bob_account).await.unwrap();
    assert_eq!(bob_state.base.amount, transferred_amount);
    let extension = bob_state.get_extension::<TransferFeeAmount>().unwrap();
    assert_eq!(extension.withheld_amount, withheld_amount.into());

    // success, rounded up transfer fee
    let transfer_amount = maximum_fee - 1;
    let fee = transfer_fee_config
        .calculate_epoch_fee(0, transfer_amount)
        .unwrap();
    token
        .transfer_checked(
            &alice_account,
            &bob_account,
            &alice,
            transfer_amount,
            decimals,
        )
        .await
        .unwrap();
    alice_amount -= transfer_amount;
    withheld_amount += fee;
    transferred_amount += transfer_amount - fee;
    let alice_state = token.get_account_info(&alice_account).await.unwrap();
    assert_eq!(alice_state.base.amount, alice_amount);
    let extension = alice_state.get_extension::<TransferFeeAmount>().unwrap();
    assert_eq!(extension.withheld_amount, 0.into());
    let bob_state = token.get_account_info(&bob_account).await.unwrap();
    assert_eq!(bob_state.base.amount, transferred_amount);
    let extension = bob_state.get_extension::<TransferFeeAmount>().unwrap();
    assert_eq!(extension.withheld_amount, withheld_amount.into());

    // success, maximum fee kicks in
    let transfer_amount =
        1 + maximum_fee * (MAX_FEE_BASIS_POINTS as u64) / (transfer_fee_basis_points as u64);
    let fee = transfer_fee_config
        .calculate_epoch_fee(0, transfer_amount)
        .unwrap();
    assert_eq!(fee, maximum_fee); // sanity
    token
        .transfer_checked(
            &alice_account,
            &bob_account,
            &alice,
            transfer_amount,
            decimals,
        )
        .await
        .unwrap();
    alice_amount -= transfer_amount;
    withheld_amount += fee;
    transferred_amount += transfer_amount - fee;
    let alice_state = token.get_account_info(&alice_account).await.unwrap();
    assert_eq!(alice_state.base.amount, alice_amount);
    let extension = alice_state.get_extension::<TransferFeeAmount>().unwrap();
    assert_eq!(extension.withheld_amount, 0.into());
    let bob_state = token.get_account_info(&bob_account).await.unwrap();
    assert_eq!(bob_state.base.amount, transferred_amount);
    let extension = bob_state.get_extension::<TransferFeeAmount>().unwrap();
    assert_eq!(extension.withheld_amount, withheld_amount.into());

    // transfer down to 1 token
    token
        .transfer_checked(
            &alice_account,
            &bob_account,
            &alice,
            alice_amount - 1,
            decimals,
        )
        .await
        .unwrap();
    transferred_amount += alice_amount - 1 - maximum_fee;
    alice_amount = 1;
    withheld_amount += maximum_fee;
    let alice_state = token.get_account_info(&alice_account).await.unwrap();
    assert_eq!(alice_state.base.amount, alice_amount);
    let extension = alice_state.get_extension::<TransferFeeAmount>().unwrap();
    assert_eq!(extension.withheld_amount, 0.into());
    let bob_state = token.get_account_info(&bob_account).await.unwrap();
    assert_eq!(bob_state.base.amount, transferred_amount);
    let extension = bob_state.get_extension::<TransferFeeAmount>().unwrap();
    assert_eq!(extension.withheld_amount, withheld_amount.into());

    // final transfer, only move tokens to withheld amount, nothing received
    token
        .transfer_checked(&alice_account, &bob_account, &alice, 1, decimals)
        .await
        .unwrap();
    withheld_amount += 1;
    let alice_state = token.get_account_info(&alice_account).await.unwrap();
    assert_eq!(alice_state.base.amount, 0);
    let extension = alice_state.get_extension::<TransferFeeAmount>().unwrap();
    assert_eq!(extension.withheld_amount, 0.into());
    let bob_state = token.get_account_info(&bob_account).await.unwrap();
    assert_eq!(bob_state.base.amount, transferred_amount);
    let extension = bob_state.get_extension::<TransferFeeAmount>().unwrap();
    assert_eq!(extension.withheld_amount, withheld_amount.into());
}

#[tokio::test]
async fn transfer_checked_with_fee() {
    let TransferFeeConfigWithKeypairs {
        transfer_fee_config_authority,
        withdraw_withheld_authority,
        transfer_fee_config,
        ..
    } = test_transfer_fee_config_with_keypairs();
    let mut context = TestContext::new().await;
    let transfer_fee_basis_points = u16::from(
        transfer_fee_config
            .newer_transfer_fee
            .transfer_fee_basis_points,
    );
    let maximum_fee = u64::from(transfer_fee_config.newer_transfer_fee.maximum_fee);
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::TransferFeeConfig {
            transfer_fee_config_authority: transfer_fee_config_authority.pubkey().into(),
            withdraw_withheld_authority: withdraw_withheld_authority.pubkey().into(),
            transfer_fee_basis_points,
            maximum_fee,
        }])
        .await
        .unwrap();
    let TokenContext {
        decimals,
        mint_authority,
        token,
        alice,
        bob,
        ..
    } = context.token_context.unwrap();

    let alice_account = Keypair::new();
    let alice_account = token
        .create_auxiliary_token_account(&alice_account, &alice.pubkey())
        .await
        .unwrap();
    let bob_account = Keypair::new();
    let bob_account = token
        .create_auxiliary_token_account(&bob_account, &bob.pubkey())
        .await
        .unwrap();

    // mint a lot of tokens, 100x max fee
    let alice_amount = maximum_fee * 100;
    token
        .mint_to(&alice_account, &mint_authority, alice_amount)
        .await
        .unwrap();

    // incorrect fee, too high
    let transfer_amount = maximum_fee;
    let fee = transfer_fee_config
        .calculate_epoch_fee(0, transfer_amount)
        .unwrap()
        + 1;
    let error = token
        .transfer_checked_with_fee(
            &alice_account,
            &bob_account,
            &alice,
            transfer_amount,
            decimals,
            fee,
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::FeeMismatch as u32)
            )
        )))
    );

    // incorrect fee, too low
    let fee = transfer_fee_config
        .calculate_epoch_fee(0, transfer_amount)
        .unwrap()
        - 1;
    let error = token
        .transfer_checked_with_fee(
            &alice_account,
            &bob_account,
            &alice,
            transfer_amount,
            decimals,
            fee,
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::FeeMismatch as u32)
            )
        )))
    );

    // correct fee, not enough tokens
    let fee = transfer_fee_config
        .calculate_epoch_fee(0, alice_amount + 1)
        .unwrap()
        - 1;
    let error = token
        .transfer_checked_with_fee(
            &alice_account,
            &bob_account,
            &alice,
            alice_amount + 1,
            decimals,
            fee,
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::InsufficientFunds as u32)
            )
        )))
    );

    // correct fee
    let fee = transfer_fee_config
        .calculate_epoch_fee(0, transfer_amount)
        .unwrap();
    token
        .transfer_checked_with_fee(
            &alice_account,
            &bob_account,
            &alice,
            transfer_amount,
            decimals,
            fee,
        )
        .await
        .unwrap();
    let alice_state = token.get_account_info(&alice_account).await.unwrap();
    assert_eq!(alice_state.base.amount, alice_amount - transfer_amount);
    let extension = alice_state.get_extension::<TransferFeeAmount>().unwrap();
    assert_eq!(extension.withheld_amount, 0.into());
    let bob_state = token.get_account_info(&bob_account).await.unwrap();
    assert_eq!(bob_state.base.amount, transfer_amount - fee);
    let extension = bob_state.get_extension::<TransferFeeAmount>().unwrap();
    assert_eq!(extension.withheld_amount, fee.into());
}

#[tokio::test]
async fn no_fees_from_self_transfer() {
    let TransferFeeConfigWithKeypairs {
        transfer_fee_config_authority,
        withdraw_withheld_authority,
        transfer_fee_config,
        ..
    } = test_transfer_fee_config_with_keypairs();
    let mut context = TestContext::new().await;
    let transfer_fee_basis_points = u16::from(
        transfer_fee_config
            .newer_transfer_fee
            .transfer_fee_basis_points,
    );
    let maximum_fee = u64::from(transfer_fee_config.newer_transfer_fee.maximum_fee);
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::TransferFeeConfig {
            transfer_fee_config_authority: transfer_fee_config_authority.pubkey().into(),
            withdraw_withheld_authority: withdraw_withheld_authority.pubkey().into(),
            transfer_fee_basis_points,
            maximum_fee,
        }])
        .await
        .unwrap();
    let TokenContext {
        decimals,
        mint_authority,
        token,
        alice,
        ..
    } = context.token_context.unwrap();

    let alice_account = Keypair::new();
    let alice_account = token
        .create_auxiliary_token_account(&alice_account, &alice.pubkey())
        .await
        .unwrap();

    // mint some tokens
    let amount = maximum_fee;
    token
        .mint_to(&alice_account, &mint_authority, amount)
        .await
        .unwrap();

    // self transfer, no fee assessed
    let fee = transfer_fee_config.calculate_epoch_fee(0, amount).unwrap();
    token
        .transfer_checked_with_fee(
            &alice_account,
            &alice_account,
            &alice,
            amount,
            decimals,
            fee,
        )
        .await
        .unwrap();
    let alice_state = token.get_account_info(&alice_account).await.unwrap();
    assert_eq!(alice_state.base.amount, amount);
    let extension = alice_state.get_extension::<TransferFeeAmount>().unwrap();
    assert_eq!(extension.withheld_amount, 0.into());
}
