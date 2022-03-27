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
    spl_token_client::{
        client::ProgramBanksClientProcessTransaction,
        token::{ExtensionInitializationParams, Token, TokenError as TokenClientError},
    },
    std::convert::TryInto,
};

const TEST_MAXIMUM_FEE: u64 = 10_000_000;
const TEST_FEE_BASIS_POINTS: u16 = 250;

fn test_transfer_fee() -> TransferFee {
    TransferFee {
        epoch: 0.into(),
        transfer_fee_basis_points: TEST_FEE_BASIS_POINTS.into(),
        maximum_fee: TEST_MAXIMUM_FEE.into(),
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

struct TokenWithAccounts {
    context: TestContext,
    token: Token<ProgramBanksClientProcessTransaction, Keypair>,
    transfer_fee_config: TransferFeeConfig,
    withdraw_withheld_authority: Keypair,
    freeze_authority: Keypair,
    alice: Keypair,
    alice_account: Pubkey,
    bob_account: Pubkey,
    decimals: u8,
}

async fn create_mint_with_accounts(alice_amount: u64) -> TokenWithAccounts {
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
        .init_token_with_freezing_mint(vec![ExtensionInitializationParams::TransferFeeConfig {
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
        freeze_authority,
        token,
        alice,
        bob,
        ..
    } = context.token_context.take().unwrap();

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

    // mint tokens
    token
        .mint_to(&alice_account, &mint_authority, alice_amount)
        .await
        .unwrap();
    TokenWithAccounts {
        context,
        token,
        transfer_fee_config,
        withdraw_withheld_authority,
        freeze_authority: freeze_authority.unwrap(),
        alice,
        alice_account,
        bob_account,
        decimals,
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
async fn fail_unsupported_mint() {
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
    let error = token
        .harvest_withheld_tokens_to_mint(&[])
        .await
        .err()
        .unwrap();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(0, InstructionError::InvalidAccountData)
        )))
    );
    let error = token
        .withdraw_withheld_tokens_from_mint(&Pubkey::new_unique(), &mint_authority)
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

    // new authority can withdraw tokens
    let account = token
        .create_auxiliary_token_account(&Keypair::new(), &new_authority.pubkey())
        .await
        .unwrap();
    token
        .withdraw_withheld_tokens_from_accounts(&account, &new_authority, &[&account])
        .await
        .unwrap();
    // old one cannot
    let error = token
        .withdraw_withheld_tokens_from_accounts(&account, &withdraw_withheld_authority, &[&account])
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::OwnerMismatch as u32)
            )
        )))
    );

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

    // assert no authority can withdraw withheld fees
    let account = token
        .create_auxiliary_token_account(&Keypair::new(), &new_authority.pubkey())
        .await
        .unwrap();
    let error = token
        .withdraw_withheld_tokens_from_accounts(&account, &withdraw_withheld_authority, &[&account])
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::NoAuthorityExists as u32)
            )
        )))
    );
    let error = token
        .withdraw_withheld_tokens_from_accounts(&account, &new_authority, &[&account])
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::NoAuthorityExists as u32)
            )
        )))
    );
}

#[tokio::test]
async fn transfer_checked() {
    let maximum_fee = TEST_MAXIMUM_FEE;
    let mut alice_amount = maximum_fee * 100;
    let TokenWithAccounts {
        token,
        transfer_fee_config,
        alice,
        alice_account,
        bob_account,
        decimals,
        ..
    } = create_mint_with_accounts(alice_amount).await;

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
    let transfer_amount = 1 + maximum_fee * (MAX_FEE_BASIS_POINTS as u64)
        / (u16::from(
            transfer_fee_config
                .newer_transfer_fee
                .transfer_fee_basis_points,
        ) as u64);
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
    let maximum_fee = TEST_MAXIMUM_FEE;
    let alice_amount = maximum_fee * 100;
    let TokenWithAccounts {
        token,
        transfer_fee_config,
        alice,
        alice_account,
        bob_account,
        decimals,
        ..
    } = create_mint_with_accounts(alice_amount).await;

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
    let amount = TEST_MAXIMUM_FEE;
    let alice_amount = amount * 100;
    let TokenWithAccounts {
        token,
        transfer_fee_config,
        alice,
        alice_account,
        decimals,
        ..
    } = create_mint_with_accounts(alice_amount).await;

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
    assert_eq!(alice_state.base.amount, alice_amount);
    let extension = alice_state.get_extension::<TransferFeeAmount>().unwrap();
    assert_eq!(extension.withheld_amount, 0.into());
}

async fn create_and_transfer_to_account(
    token: &Token<ProgramBanksClientProcessTransaction, Keypair>,
    source: &Pubkey,
    authority: &Keypair,
    owner: &Pubkey,
    amount: u64,
    decimals: u8,
) -> Pubkey {
    let account = token
        .create_auxiliary_token_account(&Keypair::new(), owner)
        .await
        .unwrap();
    token
        .transfer_checked(source, &account, authority, amount, decimals)
        .await
        .unwrap();
    account
}

#[tokio::test]
async fn harvest_withheld_tokens_to_mint() {
    let amount = TEST_MAXIMUM_FEE;
    let alice_amount = amount * 100;
    let TokenWithAccounts {
        mut context,
        token,
        transfer_fee_config,
        alice,
        alice_account,
        decimals,
        ..
    } = create_mint_with_accounts(alice_amount).await;

    // harvest from zero accounts
    token.harvest_withheld_tokens_to_mint(&[]).await.unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<TransferFeeConfig>().unwrap();
    assert_eq!(extension.withheld_amount, 0.into());

    // harvest from one account
    let accumulated_fees = transfer_fee_config.calculate_epoch_fee(0, amount).unwrap();
    let account = create_and_transfer_to_account(
        &token,
        &alice_account,
        &alice,
        &alice.pubkey(),
        amount,
        decimals,
    )
    .await;
    token
        .harvest_withheld_tokens_to_mint(&[&account])
        .await
        .unwrap();
    let state = token.get_account_info(&account).await.unwrap();
    let extension = state.get_extension::<TransferFeeAmount>().unwrap();
    assert_eq!(extension.withheld_amount, 0.into());
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<TransferFeeConfig>().unwrap();
    assert_eq!(extension.withheld_amount, accumulated_fees.into());

    // no fail harvesting from account belonging to different mint, but nothing
    // happens
    let account = create_and_transfer_to_account(
        &token,
        &alice_account,
        &alice,
        &alice.pubkey(),
        amount,
        decimals,
    )
    .await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::TransferFeeConfig {
            transfer_fee_config_authority: Some(Pubkey::new_unique()),
            withdraw_withheld_authority: Some(Pubkey::new_unique()),
            transfer_fee_basis_points: TEST_FEE_BASIS_POINTS,
            maximum_fee: TEST_MAXIMUM_FEE,
        }])
        .await
        .unwrap();
    let TokenContext { token, .. } = context.token_context.take().unwrap();
    token
        .harvest_withheld_tokens_to_mint(&[&account])
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<TransferFeeConfig>().unwrap();
    assert_eq!(extension.withheld_amount, 0.into());
}

#[tokio::test]
async fn max_harvest_withheld_tokens_to_mint() {
    let amount = TEST_MAXIMUM_FEE;
    let alice_amount = amount * 100;
    let TokenWithAccounts {
        token,
        transfer_fee_config,
        alice,
        alice_account,
        decimals,
        ..
    } = create_mint_with_accounts(alice_amount).await;

    // harvest from max accounts, which is around 35, AKA 34 accounts + 1 mint
    // see https://docs.solana.com/proposals/transactions-v2#problem
    let mut accounts = vec![];
    let max_accounts = 34;
    for _ in 0..max_accounts {
        let account = create_and_transfer_to_account(
            &token,
            &alice_account,
            &alice,
            &alice.pubkey(),
            amount,
            decimals,
        )
        .await;
        accounts.push(account);
    }
    let accounts: Vec<_> = accounts.iter().collect();
    let accumulated_fees =
        max_accounts * transfer_fee_config.calculate_epoch_fee(0, amount).unwrap();
    token
        .harvest_withheld_tokens_to_mint(&accounts)
        .await
        .unwrap();
    for account in accounts {
        let state = token.get_account_info(account).await.unwrap();
        let extension = state.get_extension::<TransferFeeAmount>().unwrap();
        assert_eq!(extension.withheld_amount, 0.into());
    }
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<TransferFeeConfig>().unwrap();
    assert_eq!(extension.withheld_amount, accumulated_fees.into());
}

#[tokio::test]
async fn max_withdraw_withheld_tokens_from_accounts() {
    let amount = TEST_MAXIMUM_FEE;
    let alice_amount = amount * 100;
    let TokenWithAccounts {
        token,
        withdraw_withheld_authority,
        transfer_fee_config,
        alice,
        alice_account,
        decimals,
        ..
    } = create_mint_with_accounts(alice_amount).await;

    // withdraw from max accounts, which is around 35: 1 mint, 1 destination, 1 authority,
    // 32 accounts
    // see https://docs.solana.com/proposals/transactions-v2#problem
    let destination = token
        .create_auxiliary_token_account(&Keypair::new(), &alice.pubkey())
        .await
        .unwrap();
    let mut accounts = vec![];
    let max_accounts = 32;
    for _ in 0..max_accounts {
        let account = create_and_transfer_to_account(
            &token,
            &alice_account,
            &alice,
            &alice.pubkey(),
            amount,
            decimals,
        )
        .await;
        accounts.push(account);
    }
    let accounts: Vec<_> = accounts.iter().collect();
    let accumulated_fees =
        max_accounts * transfer_fee_config.calculate_epoch_fee(0, amount).unwrap();
    token
        .withdraw_withheld_tokens_from_accounts(
            &destination,
            &withdraw_withheld_authority,
            &accounts,
        )
        .await
        .unwrap();
    for account in accounts {
        let state = token.get_account_info(account).await.unwrap();
        let extension = state.get_extension::<TransferFeeAmount>().unwrap();
        assert_eq!(extension.withheld_amount, 0.into());
    }
    let state = token.get_account_info(&destination).await.unwrap();
    assert_eq!(state.base.amount, accumulated_fees);
}

#[tokio::test]
async fn withdraw_withheld_tokens_from_mint() {
    let amount = TEST_MAXIMUM_FEE;
    let alice_amount = amount * 100;
    let TokenWithAccounts {
        mut context,
        token,
        transfer_fee_config,
        withdraw_withheld_authority,
        freeze_authority,
        alice,
        alice_account,
        decimals,
        bob_account,
        ..
    } = create_mint_with_accounts(alice_amount).await;

    // no tokens withheld on mint
    token
        .withdraw_withheld_tokens_from_mint(&alice_account, &withdraw_withheld_authority)
        .await
        .unwrap();
    let state = token.get_account_info(&alice_account).await.unwrap();
    assert_eq!(state.base.amount, alice_amount);
    let extension = state.get_extension::<TransferFeeAmount>().unwrap();
    assert_eq!(extension.withheld_amount, 0.into());
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<TransferFeeConfig>().unwrap();
    assert_eq!(extension.withheld_amount, 0.into());

    // transfer + harvest to mint
    let fee = transfer_fee_config.calculate_epoch_fee(0, amount).unwrap();
    let account = create_and_transfer_to_account(
        &token,
        &alice_account,
        &alice,
        &alice.pubkey(),
        amount,
        decimals,
    )
    .await;

    let state = token.get_account_info(&account).await.unwrap();
    let extension = state.get_extension::<TransferFeeAmount>().unwrap();
    assert_eq!(extension.withheld_amount, fee.into());

    token
        .harvest_withheld_tokens_to_mint(&[&account])
        .await
        .unwrap();

    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<TransferFeeConfig>().unwrap();
    assert_eq!(extension.withheld_amount, fee.into());

    // success
    token
        .withdraw_withheld_tokens_from_mint(&bob_account, &withdraw_withheld_authority)
        .await
        .unwrap();
    let state = token.get_account_info(&bob_account).await.unwrap();
    assert_eq!(state.base.amount, fee);
    let state = token.get_account_info(&account).await.unwrap();
    let extension = state.get_extension::<TransferFeeAmount>().unwrap();
    assert_eq!(extension.withheld_amount, 0.into());
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<TransferFeeConfig>().unwrap();
    assert_eq!(extension.withheld_amount, 0.into());

    // fail wrong signer
    let error = token
        .withdraw_withheld_tokens_from_mint(&alice_account, &alice)
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::OwnerMismatch as u32)
            )
        )))
    );

    // fail frozen account
    let account = create_and_transfer_to_account(
        &token,
        &alice_account,
        &alice,
        &alice.pubkey(),
        amount,
        decimals,
    )
    .await;
    token
        .freeze_account(&account, &freeze_authority)
        .await
        .unwrap();
    let error = token
        .withdraw_withheld_tokens_from_mint(&account, &withdraw_withheld_authority)
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::AccountFrozen as u32)
            )
        )))
    );

    // set to none, fail
    let account = create_and_transfer_to_account(
        &token,
        &alice_account,
        &alice,
        &alice.pubkey(),
        amount,
        decimals,
    )
    .await;
    token
        .set_authority(
            token.get_address(),
            None,
            instruction::AuthorityType::WithheldWithdraw,
            &withdraw_withheld_authority,
        )
        .await
        .unwrap();
    let error = token
        .withdraw_withheld_tokens_from_mint(&account, &withdraw_withheld_authority)
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::NoAuthorityExists as u32)
            )
        )))
    );

    // fail on new mint with mint mismatch
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::TransferFeeConfig {
            transfer_fee_config_authority: Some(Pubkey::new_unique()),
            withdraw_withheld_authority: Some(withdraw_withheld_authority.pubkey()),
            transfer_fee_basis_points: TEST_FEE_BASIS_POINTS,
            maximum_fee: TEST_MAXIMUM_FEE,
        }])
        .await
        .unwrap();
    let TokenContext { token, .. } = context.token_context.take().unwrap();
    let error = token
        .withdraw_withheld_tokens_from_mint(&account, &withdraw_withheld_authority)
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::MintMismatch as u32)
            )
        )))
    );
}

#[tokio::test]
async fn withdraw_withheld_tokens_from_accounts() {
    let amount = TEST_MAXIMUM_FEE;
    let alice_amount = amount * 100;
    let TokenWithAccounts {
        mut context,
        token,
        withdraw_withheld_authority,
        alice,
        alice_account,
        decimals,
        ..
    } = create_mint_with_accounts(alice_amount).await;

    // wrong signer
    let error = token
        .withdraw_withheld_tokens_from_accounts(&alice_account, &Keypair::new(), &[])
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::OwnerMismatch as u32)
            )
        )))
    );

    // withdraw from zero accounts
    token
        .withdraw_withheld_tokens_from_accounts(&alice_account, &withdraw_withheld_authority, &[])
        .await
        .unwrap();
    let state = token.get_account_info(&alice_account).await.unwrap();
    assert_eq!(state.base.amount, alice_amount);

    // self-harvest from one account
    let account = create_and_transfer_to_account(
        &token,
        &alice_account,
        &alice,
        &alice.pubkey(),
        amount,
        decimals,
    )
    .await;
    token
        .withdraw_withheld_tokens_from_accounts(&account, &withdraw_withheld_authority, &[&account])
        .await
        .unwrap();
    let state = token.get_account_info(&account).await.unwrap();
    let extension = state.get_extension::<TransferFeeAmount>().unwrap();
    // we transferred to this account, and then withdrew the fee to it, so it's
    // like doing a fee-less transfer!
    assert_eq!(extension.withheld_amount, 0.into());
    assert_eq!(state.base.amount, amount);

    // harvest again from the same account
    token
        .withdraw_withheld_tokens_from_accounts(
            &alice_account,
            &withdraw_withheld_authority,
            &[&account],
        )
        .await
        .unwrap();
    let state = token.get_account_info(&account).await.unwrap();
    let extension = state.get_extension::<TransferFeeAmount>().unwrap();
    assert_eq!(extension.withheld_amount, 0.into());
    assert_eq!(state.base.amount, amount);
    let state = token.get_account_info(&alice_account).await.unwrap();
    assert_eq!(state.base.amount, alice_amount - amount);

    // no fail harvesting from account belonging to different mint, but nothing
    // happens
    let account = create_and_transfer_to_account(
        &token,
        &alice_account,
        &alice,
        &alice.pubkey(),
        amount,
        decimals,
    )
    .await;
    context
        .init_token_with_mint(vec![ExtensionInitializationParams::TransferFeeConfig {
            transfer_fee_config_authority: Some(Pubkey::new_unique()),
            withdraw_withheld_authority: Some(withdraw_withheld_authority.pubkey()),
            transfer_fee_basis_points: TEST_FEE_BASIS_POINTS,
            maximum_fee: TEST_MAXIMUM_FEE,
        }])
        .await
        .unwrap();
    let TokenContext { token, .. } = context.token_context.take().unwrap();
    let withdraw_account = token
        .create_auxiliary_token_account(&Keypair::new(), &alice.pubkey())
        .await
        .unwrap();
    token
        .withdraw_withheld_tokens_from_accounts(
            &withdraw_account,
            &withdraw_withheld_authority,
            &[&account],
        )
        .await
        .unwrap();
    let state = token.get_mint_info().await.unwrap();
    let extension = state.get_extension::<TransferFeeConfig>().unwrap();
    assert_eq!(extension.withheld_amount, 0.into());

    // fail withdrawing into account on different mint
    let error = token
        .withdraw_withheld_tokens_from_accounts(
            &account,
            &withdraw_withheld_authority,
            &[&withdraw_account],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::MintMismatch as u32)
            )
        )))
    );
}

#[tokio::test]
async fn fail_close_with_withheld() {
    let amount = TEST_MAXIMUM_FEE;
    let alice_amount = amount * 100;
    let TokenWithAccounts {
        token,
        transfer_fee_config,
        alice,
        alice_account,
        decimals,
        ..
    } = create_mint_with_accounts(alice_amount).await;

    // accrue withheld fees on new account
    let account = create_and_transfer_to_account(
        &token,
        &alice_account,
        &alice,
        &alice.pubkey(),
        amount,
        decimals,
    )
    .await;

    // empty the account
    let fee = transfer_fee_config.calculate_epoch_fee(0, amount).unwrap();
    token
        .transfer_checked(&account, &alice_account, &alice, amount - fee, decimals)
        .await
        .unwrap();

    // fail to close
    let error = token
        .close_account(&account, &Pubkey::new_unique(), &alice)
        .await
        .unwrap_err();
    assert_eq!(
        error,
        TokenClientError::Client(Box::new(TransportError::TransactionError(
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(TokenError::AccountHasWithheldTransferFees as u32)
            )
        )))
    );

    // harvest the fees to the mint
    token
        .harvest_withheld_tokens_to_mint(&[&account])
        .await
        .unwrap();

    // successfully close
    token
        .close_account(&account, &Pubkey::new_unique(), &alice)
        .await
        .unwrap();
}
