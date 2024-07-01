#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{keypair_clone, TestContext, TokenContext},
    solana_program_test::{
        processor,
        tokio::{self, sync::Mutex},
        ProgramTest,
    },
    solana_sdk::{
        instruction::InstructionError, pubkey::Pubkey, signature::Signer, signer::keypair::Keypair,
        transaction::TransactionError, transport::TransportError,
    },
    spl_instruction_padding::instruction::wrap_instruction,
    spl_token_2022::{
        error::TokenError,
        extension::{
            cpi_guard::{self, CpiGuard},
            BaseStateWithExtensions, ExtensionType,
        },
        instruction::{self, AuthorityType},
        processor::Processor as SplToken2022Processor,
    },
    spl_token_client::{
        client::ProgramBanksClientProcessTransaction,
        token::{Token, TokenError as TokenClientError},
    },
    std::sync::Arc,
};

// set up a bank and bank client with spl token 2022 and the instruction padder
// also creates a token with no extensions and inits two token accounts
async fn make_context() -> TestContext {
    // TODO this may be removed when we upgrade to a solana version with a fixed
    // `get_stack_height()` stub
    if std::env::var("BPF_OUT_DIR").is_err() && std::env::var("SBF_OUT_DIR").is_err() {
        panic!("CpiGuard tests MUST be invoked with `cargo test-sbf`, NOT `cargo test --feature test-sbf`. \
                In a non-BPF context, `get_stack_height()` always returns 0, and all tests WILL fail.");
    }

    let mut program_test = ProgramTest::new(
        "spl_token_2022",
        spl_token_2022::id(),
        processor!(SplToken2022Processor::process),
    );

    program_test.add_program(
        "spl_instruction_padding",
        spl_instruction_padding::id(),
        processor!(spl_instruction_padding::processor::process),
    );

    let program_context = program_test.start_with_context().await;
    let program_context = Arc::new(Mutex::new(program_context));

    let mut test_context = TestContext {
        context: program_context,
        token_context: None,
    };

    test_context.init_token_with_mint(vec![]).await.unwrap();
    let token_context = test_context.token_context.as_ref().unwrap();

    token_context
        .token
        .create_auxiliary_token_account_with_extension_space(
            &token_context.alice,
            &token_context.alice.pubkey(),
            vec![ExtensionType::CpiGuard],
        )
        .await
        .unwrap();

    token_context
        .token
        .create_auxiliary_token_account(&token_context.bob, &token_context.bob.pubkey())
        .await
        .unwrap();

    test_context
}

fn client_error(token_error: TokenError) -> TokenClientError {
    TokenClientError::Client(Box::new(TransportError::TransactionError(
        TransactionError::InstructionError(0, InstructionError::Custom(token_error as u32)),
    )))
}

#[tokio::test]
async fn test_cpi_guard_enable_disable() {
    let context = make_context().await;
    let TokenContext {
        token, alice, bob, ..
    } = context.token_context.unwrap();

    // enable guard properly
    token
        .enable_cpi_guard(&alice.pubkey(), &alice.pubkey(), &[&alice])
        .await
        .unwrap();

    // guard is enabled
    let alice_state = token.get_account_info(&alice.pubkey()).await.unwrap();
    let extension = alice_state.get_extension::<CpiGuard>().unwrap();
    assert!(bool::from(extension.lock_cpi));

    // attempt to disable through cpi. this fails
    let error = token
        .process_ixs(
            &[wrap_instruction(
                spl_instruction_padding::id(),
                cpi_guard::instruction::disable_cpi_guard(
                    &spl_token_2022::id(),
                    &alice.pubkey(),
                    &alice.pubkey(),
                    &[],
                )
                .unwrap(),
                vec![],
                0,
            )
            .unwrap()],
            &[&alice],
        )
        .await
        .unwrap_err();
    assert_eq!(error, client_error(TokenError::CpiGuardSettingsLocked));

    // guard remains enabled
    let alice_state = token.get_account_info(&alice.pubkey()).await.unwrap();
    let extension = alice_state.get_extension::<CpiGuard>().unwrap();
    assert!(bool::from(extension.lock_cpi));

    // disable guard properly
    token
        .disable_cpi_guard(&alice.pubkey(), &alice.pubkey(), &[&alice])
        .await
        .unwrap();

    // guard is disabled
    let alice_state = token.get_account_info(&alice.pubkey()).await.unwrap();
    let extension = alice_state.get_extension::<CpiGuard>().unwrap();
    assert!(!bool::from(extension.lock_cpi));

    // attempt to enable through cpi. this fails
    let error = token
        .process_ixs(
            &[wrap_instruction(
                spl_instruction_padding::id(),
                cpi_guard::instruction::enable_cpi_guard(
                    &spl_token_2022::id(),
                    &alice.pubkey(),
                    &alice.pubkey(),
                    &[],
                )
                .unwrap(),
                vec![],
                0,
            )
            .unwrap()],
            &[&alice],
        )
        .await
        .unwrap_err();
    assert_eq!(error, client_error(TokenError::CpiGuardSettingsLocked));

    // guard remains disabled
    let alice_state = token.get_account_info(&alice.pubkey()).await.unwrap();
    let extension = alice_state.get_extension::<CpiGuard>().unwrap();
    assert!(!bool::from(extension.lock_cpi));

    // enable works with realloc
    token
        .reallocate(
            &bob.pubkey(),
            &bob.pubkey(),
            &[ExtensionType::CpiGuard],
            &[&bob],
        )
        .await
        .unwrap();

    token
        .enable_cpi_guard(&bob.pubkey(), &bob.pubkey(), &[&bob])
        .await
        .unwrap();

    let bob_state = token.get_account_info(&bob.pubkey()).await.unwrap();
    let extension = bob_state.get_extension::<CpiGuard>().unwrap();
    assert!(bool::from(extension.lock_cpi));
}

#[tokio::test]
async fn test_cpi_guard_transfer() {
    let context = make_context().await;
    let TokenContext {
        token,
        token_unchecked,
        mint_authority,
        alice,
        bob,
        ..
    } = context.token_context.unwrap();

    let mk_transfer = |authority, do_checked| {
        wrap_instruction(
            spl_instruction_padding::id(),
            if do_checked {
                instruction::transfer_checked(
                    &spl_token_2022::id(),
                    &alice.pubkey(),
                    token.get_address(),
                    &bob.pubkey(),
                    &authority,
                    &[],
                    1,
                    9,
                )
                .unwrap()
            } else {
                #[allow(deprecated)]
                instruction::transfer(
                    &spl_token_2022::id(),
                    &alice.pubkey(),
                    &bob.pubkey(),
                    &authority,
                    &[],
                    1,
                )
                .unwrap()
            },
            vec![],
            0,
        )
        .unwrap()
    };

    let mut amount = 100;
    token
        .mint_to(
            &alice.pubkey(),
            &mint_authority.pubkey(),
            amount,
            &[&mint_authority],
        )
        .await
        .unwrap();

    for do_checked in [true, false] {
        let token_obj = if do_checked { &token } else { &token_unchecked };
        token_obj
            .enable_cpi_guard(&alice.pubkey(), &alice.pubkey(), &[&alice])
            .await
            .unwrap();

        // transfer works normally with cpi guard enabled
        token_obj
            .transfer(
                &alice.pubkey(),
                &bob.pubkey(),
                &alice.pubkey(),
                1,
                &[&alice],
            )
            .await
            .unwrap();
        amount -= 1;

        let alice_state = token_obj.get_account_info(&alice.pubkey()).await.unwrap();
        assert_eq!(alice_state.base.amount, amount);

        // user-auth cpi transfer with cpi guard doesn't work
        let error = token_obj
            .process_ixs(&[mk_transfer(alice.pubkey(), do_checked)], &[&alice])
            .await
            .unwrap_err();
        assert_eq!(error, client_error(TokenError::CpiGuardTransferBlocked));

        let alice_state = token_obj.get_account_info(&alice.pubkey()).await.unwrap();
        assert_eq!(alice_state.base.amount, amount);

        // delegate-auth cpi transfer to self does not work
        token_obj
            .approve(
                &alice.pubkey(),
                &alice.pubkey(),
                &alice.pubkey(),
                1,
                &[&alice],
            )
            .await
            .unwrap();

        let error = token_obj
            .process_ixs(&[mk_transfer(alice.pubkey(), do_checked)], &[&alice])
            .await
            .unwrap_err();
        assert_eq!(error, client_error(TokenError::CpiGuardTransferBlocked));

        let alice_state = token_obj.get_account_info(&alice.pubkey()).await.unwrap();
        assert_eq!(alice_state.base.amount, amount);

        // delegate-auth cpi transfer with cpi guard works
        token_obj
            .approve(
                &alice.pubkey(),
                &bob.pubkey(),
                &alice.pubkey(),
                1,
                &[&alice],
            )
            .await
            .unwrap();

        token_obj
            .process_ixs(&[mk_transfer(bob.pubkey(), do_checked)], &[&bob])
            .await
            .unwrap();
        amount -= 1;

        let alice_state = token_obj.get_account_info(&alice.pubkey()).await.unwrap();
        assert_eq!(alice_state.base.amount, amount);

        // transfer still works through cpi with cpi guard off
        token_obj
            .disable_cpi_guard(&alice.pubkey(), &alice.pubkey(), &[&alice])
            .await
            .unwrap();

        token_obj
            .process_ixs(&[mk_transfer(alice.pubkey(), do_checked)], &[&alice])
            .await
            .unwrap();
        amount -= 1;

        let alice_state = token_obj.get_account_info(&alice.pubkey()).await.unwrap();
        assert_eq!(alice_state.base.amount, amount);
    }
}

#[tokio::test]
async fn test_cpi_guard_burn() {
    let context = make_context().await;
    let TokenContext {
        token,
        token_unchecked,
        mint_authority,
        alice,
        bob,
        ..
    } = context.token_context.unwrap();

    let mk_burn = |authority, do_checked| {
        wrap_instruction(
            spl_instruction_padding::id(),
            if do_checked {
                instruction::burn_checked(
                    &spl_token_2022::id(),
                    &alice.pubkey(),
                    token.get_address(),
                    &authority,
                    &[],
                    1,
                    9,
                )
                .unwrap()
            } else {
                instruction::burn(
                    &spl_token_2022::id(),
                    &alice.pubkey(),
                    token.get_address(),
                    &authority,
                    &[],
                    1,
                )
                .unwrap()
            },
            vec![],
            0,
        )
        .unwrap()
    };

    let mut amount = 100;
    token
        .mint_to(
            &alice.pubkey(),
            &mint_authority.pubkey(),
            amount,
            &[&mint_authority],
        )
        .await
        .unwrap();

    for do_checked in [true, false] {
        let token_obj = if do_checked { &token } else { &token_unchecked };
        token_obj
            .enable_cpi_guard(&alice.pubkey(), &alice.pubkey(), &[&alice])
            .await
            .unwrap();

        // burn works normally with cpi guard enabled
        token_obj
            .burn(&alice.pubkey(), &alice.pubkey(), 1, &[&alice])
            .await
            .unwrap();
        amount -= 1;

        let alice_state = token_obj.get_account_info(&alice.pubkey()).await.unwrap();
        assert_eq!(alice_state.base.amount, amount);

        // user-auth cpi burn with cpi guard doesn't work
        let error = token_obj
            .process_ixs(&[mk_burn(alice.pubkey(), do_checked)], &[&alice])
            .await
            .unwrap_err();
        assert_eq!(error, client_error(TokenError::CpiGuardBurnBlocked));

        let alice_state = token_obj.get_account_info(&alice.pubkey()).await.unwrap();
        assert_eq!(alice_state.base.amount, amount);

        // delegate-auth cpi burn by self does not work
        token_obj
            .approve(
                &alice.pubkey(),
                &alice.pubkey(),
                &alice.pubkey(),
                1,
                &[&alice],
            )
            .await
            .unwrap();

        let error = token_obj
            .process_ixs(&[mk_burn(alice.pubkey(), do_checked)], &[&alice])
            .await
            .unwrap_err();
        assert_eq!(error, client_error(TokenError::CpiGuardBurnBlocked));

        let alice_state = token_obj.get_account_info(&alice.pubkey()).await.unwrap();
        assert_eq!(alice_state.base.amount, amount);

        // delegate-auth cpi burn with cpi guard works
        token_obj
            .approve(
                &alice.pubkey(),
                &bob.pubkey(),
                &alice.pubkey(),
                1,
                &[&alice],
            )
            .await
            .unwrap();

        token_obj
            .process_ixs(&[mk_burn(bob.pubkey(), do_checked)], &[&bob])
            .await
            .unwrap();
        amount -= 1;

        let alice_state = token_obj.get_account_info(&alice.pubkey()).await.unwrap();
        assert_eq!(alice_state.base.amount, amount);

        // burn still works through cpi with cpi guard off
        token_obj
            .disable_cpi_guard(&alice.pubkey(), &alice.pubkey(), &[&alice])
            .await
            .unwrap();

        token_obj
            .process_ixs(&[mk_burn(alice.pubkey(), do_checked)], &[&alice])
            .await
            .unwrap();
        amount -= 1;

        let alice_state = token_obj.get_account_info(&alice.pubkey()).await.unwrap();
        assert_eq!(alice_state.base.amount, amount);
    }
}

#[tokio::test]
async fn test_cpi_guard_approve() {
    let context = make_context().await;
    let TokenContext {
        token,
        token_unchecked,
        alice,
        bob,
        ..
    } = context.token_context.unwrap();

    let mk_approve = |do_checked| {
        wrap_instruction(
            spl_instruction_padding::id(),
            if do_checked {
                instruction::approve_checked(
                    &spl_token_2022::id(),
                    &alice.pubkey(),
                    token.get_address(),
                    &bob.pubkey(),
                    &alice.pubkey(),
                    &[],
                    1,
                    9,
                )
                .unwrap()
            } else {
                instruction::approve(
                    &spl_token_2022::id(),
                    &alice.pubkey(),
                    &bob.pubkey(),
                    &alice.pubkey(),
                    &[],
                    1,
                )
                .unwrap()
            },
            vec![],
            0,
        )
        .unwrap()
    };

    for do_checked in [true, false] {
        let token_obj = if do_checked { &token } else { &token_unchecked };
        token_obj
            .enable_cpi_guard(&alice.pubkey(), &alice.pubkey(), &[&alice])
            .await
            .unwrap();

        // approve works normally with cpi guard enabled
        token_obj
            .approve(
                &alice.pubkey(),
                &bob.pubkey(),
                &alice.pubkey(),
                1,
                &[&alice],
            )
            .await
            .unwrap();

        token_obj
            .revoke(&alice.pubkey(), &alice.pubkey(), &[&alice])
            .await
            .unwrap();

        // approve doesn't work through cpi
        let error = token_obj
            .process_ixs(&[mk_approve(do_checked)], &[&alice])
            .await
            .unwrap_err();
        assert_eq!(error, client_error(TokenError::CpiGuardApproveBlocked));

        // approve still works through cpi with cpi guard off
        token_obj
            .disable_cpi_guard(&alice.pubkey(), &alice.pubkey(), &[&alice])
            .await
            .unwrap();

        // refresh the blockhash
        token_obj.get_new_latest_blockhash().await.unwrap();
        token_obj
            .process_ixs(&[mk_approve(do_checked)], &[&alice])
            .await
            .unwrap();

        token_obj
            .revoke(&alice.pubkey(), &alice.pubkey(), &[&alice])
            .await
            .unwrap();
    }
}

async fn make_close_test_account<S: Signer>(
    token: &Token<ProgramBanksClientProcessTransaction>,
    owner: &S,
    authority: Option<Pubkey>,
) -> Pubkey {
    let account = Keypair::new();

    token
        .create_auxiliary_token_account_with_extension_space(
            &account,
            &owner.pubkey(),
            vec![ExtensionType::CpiGuard],
        )
        .await
        .unwrap();

    if authority.is_some() {
        token
            .set_authority(
                &account.pubkey(),
                &owner.pubkey(),
                authority.as_ref(),
                AuthorityType::CloseAccount,
                &[owner],
            )
            .await
            .unwrap();
    }

    token
        .enable_cpi_guard(&account.pubkey(), &owner.pubkey(), &[owner])
        .await
        .unwrap();

    account.pubkey()
}

#[tokio::test]
async fn test_cpi_guard_close_account() {
    let context = make_context().await;
    let TokenContext {
        token, alice, bob, ..
    } = context.token_context.unwrap();

    let mk_close = |account, destination, authority| {
        wrap_instruction(
            spl_instruction_padding::id(),
            instruction::close_account(
                &spl_token_2022::id(),
                &account,
                &destination,
                &authority,
                &[],
            )
            .unwrap(),
            vec![],
            0,
        )
        .unwrap()
    };

    // test closing through owner and closing through close authority
    // the result should be the same eitehr way
    for maybe_close_authority in [None, Some(bob.pubkey())] {
        let authority = if maybe_close_authority.is_none() {
            &alice
        } else {
            &bob
        };

        // closing normally works
        let account = make_close_test_account(&token, &alice, maybe_close_authority).await;
        token
            .close_account(&account, &bob.pubkey(), &authority.pubkey(), &[authority])
            .await
            .unwrap();

        // cpi close with guard enabled fails if lamports diverted to third party
        let account = make_close_test_account(&token, &alice, maybe_close_authority).await;
        let error = token
            .process_ixs(
                &[mk_close(account, bob.pubkey(), authority.pubkey())],
                &[authority],
            )
            .await
            .unwrap_err();
        assert_eq!(error, client_error(TokenError::CpiGuardCloseAccountBlocked));

        // but close succeeds if lamports are returned to owner
        token
            .process_ixs(
                &[mk_close(account, alice.pubkey(), authority.pubkey())],
                &[authority],
            )
            .await
            .unwrap();

        // close still works through cpi when guard disabled
        let account = make_close_test_account(&token, &alice, maybe_close_authority).await;
        token
            .disable_cpi_guard(&account, &alice.pubkey(), &[&alice])
            .await
            .unwrap();

        token
            .process_ixs(
                &[mk_close(account, bob.pubkey(), authority.pubkey())],
                &[authority],
            )
            .await
            .unwrap();
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum SetAuthTest {
    ChangeOwner,
    AddCloseAuth,
    ChangeCloseAuth,
    RemoveCloseAuth,
}

#[tokio::test]
async fn test_cpi_guard_set_authority() {
    let context = make_context().await;
    let TokenContext {
        token, alice, bob, ..
    } = context.token_context.unwrap();

    // the behavior of cpi guard and close authority is so complicated that its best
    // to test all cases exhaustively
    let mut states = vec![];
    for action in [
        SetAuthTest::ChangeOwner,
        SetAuthTest::AddCloseAuth,
        SetAuthTest::ChangeCloseAuth,
        SetAuthTest::RemoveCloseAuth,
    ] {
        for enable_cpi_guard in [true, false] {
            for do_in_cpi in [true, false] {
                states.push((action, enable_cpi_guard, do_in_cpi));
            }
        }
    }

    for state in states {
        let (action, enable_cpi_guard, do_in_cpi) = state;

        // make a new account
        let account = Keypair::new();
        token
            .create_auxiliary_token_account_with_extension_space(
                &account,
                &alice.pubkey(),
                vec![ExtensionType::CpiGuard],
            )
            .await
            .unwrap();

        // turn on cpi guard if we are testing that case
        // all actions with cpi guard off should succeed unconditionally
        // so half of these tests are backwards compat checks
        if enable_cpi_guard {
            token
                .enable_cpi_guard(&account.pubkey(), &alice.pubkey(), &[&alice])
                .await
                .unwrap();
        }

        // if we are changing or removing close auth, we need to have one to
        // change/remove
        if action == SetAuthTest::ChangeCloseAuth || action == SetAuthTest::RemoveCloseAuth {
            token
                .set_authority(
                    &account.pubkey(),
                    &alice.pubkey(),
                    Some(&bob.pubkey()),
                    AuthorityType::CloseAccount,
                    &[&alice],
                )
                .await
                .unwrap();
        }

        // this produces the token instruction we want to execute
        let (current_authority, new_authority) = match action {
            SetAuthTest::ChangeOwner | SetAuthTest::AddCloseAuth => {
                (keypair_clone(&alice), Some(bob.pubkey()))
            }
            SetAuthTest::ChangeCloseAuth => (keypair_clone(&bob), Some(alice.pubkey())),
            SetAuthTest::RemoveCloseAuth => (keypair_clone(&bob), None),
        };
        let token_instruction = instruction::set_authority(
            &spl_token_2022::id(),
            &account.pubkey(),
            new_authority.as_ref(),
            if action == SetAuthTest::ChangeOwner {
                AuthorityType::AccountOwner
            } else {
                AuthorityType::CloseAccount
            },
            &current_authority.pubkey(),
            &[],
        )
        .unwrap();

        // this wraps it or doesn't based on the test case
        let instruction = if do_in_cpi {
            wrap_instruction(spl_instruction_padding::id(), token_instruction, vec![], 0).unwrap()
        } else {
            token_instruction
        };

        // and here we go
        let result = token
            .process_ixs(&[instruction], &[&current_authority])
            .await;

        // truth table for our cases
        match (action, enable_cpi_guard, do_in_cpi) {
            // all actions succeed with cpi guard off
            (_, false, _) => result.unwrap(),
            // ownership cannot be transferred with guard
            (SetAuthTest::ChangeOwner, true, false) => assert_eq!(
                result.unwrap_err(),
                client_error(TokenError::CpiGuardOwnerChangeBlocked)
            ),
            // all other actions succeed outside cpi with guard
            (_, true, false) => result.unwrap(),
            // removing a close authority succeeds in cpi with guard
            (SetAuthTest::RemoveCloseAuth, true, true) => result.unwrap(),
            // changing owner, adding close, or changing close all fail in cpi with guard
            (_, true, true) => assert_eq!(
                result.unwrap_err(),
                client_error(TokenError::CpiGuardSetAuthorityBlocked)
            ),
        }
    }
}
