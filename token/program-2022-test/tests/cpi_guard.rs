#![cfg(feature = "test-sbf")]

mod program_test;
use {
    cpi_caller::processor::Processor as CpiCallerProcessor,
    program_test::{TestContext, TokenContext},
    solana_program_test::{
        processor,
        tokio::{self, sync::Mutex},
        ProgramTest,
    },
    solana_sdk::{pubkey::Pubkey, signature::Signer, signer::keypair::Keypair},
    spl_token_2022::{
        extension::{cpi_guard::CpiGuard, ExtensionType},
        processor::Processor as SplToken2022Processor,
    },
    std::sync::Arc,
};

// set up a bank and bank client with spl token 2022 and the test cpi caller
// also creates a token with no extensions and inits two token accounts
async fn make_context() -> (TestContext, Pubkey) {
    if std::env::var("BPF_OUT_DIR").is_err() && std::env::var("SBF_OUT_DIR").is_err() {
        panic!("CpiGuard tests MUST be invoked with `cargo test-sbf`, NOT `cargo test --feature test-sbf`. \
                In a non-BPF context, `get_stack_height()` always returns 0, and all tests WILL fail.");
    }

    let cpi_caller_id = Keypair::new().pubkey();

    let mut program_test = ProgramTest::new(
        "spl_token_2022",
        spl_token_2022::id(),
        processor!(SplToken2022Processor::process),
    );

    program_test.add_program(
        "cpi_caller",
        cpi_caller_id,
        processor!(CpiCallerProcessor::process),
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

    (test_context, cpi_caller_id)
}

#[tokio::test]
async fn test_cpi_guard_enable_disable() {
    let (context, cpi_caller_id) = make_context().await;
    let TokenContext { token, alice, .. } = context.token_context.unwrap();

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
    token
        .process_ixs(
            &[cpi_caller::instruction::disable_cpi_guard(
                &cpi_caller_id,
                &spl_token_2022::id(),
                &alice.pubkey(),
                &alice.pubkey(),
            )
            .unwrap()],
            &[&alice],
        )
        .await
        .unwrap_err();

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
    token
        .process_ixs(
            &[cpi_caller::instruction::enable_cpi_guard(
                &cpi_caller_id,
                &spl_token_2022::id(),
                &alice.pubkey(),
                &alice.pubkey(),
            )
            .unwrap()],
            &[&alice],
        )
        .await
        .unwrap_err();

    // guard remains disabled
    let alice_state = token.get_account_info(&alice.pubkey()).await.unwrap();
    let extension = alice_state.get_extension::<CpiGuard>().unwrap();
    assert!(!bool::from(extension.lock_cpi));
}

#[tokio::test]
async fn test_cpi_guard_transfer() {
    let (context, cpi_caller_id) = make_context().await;
    let TokenContext {
        token,
        mint_authority,
        alice,
        bob,
        ..
    } = context.token_context.unwrap();

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

    // transfer works normally
    token
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

    let alice_state = token.get_account_info(&alice.pubkey()).await.unwrap();
    assert_eq!(alice_state.base.amount, amount);

    for do_checked in [true, false] {
        token
            .enable_cpi_guard(&alice.pubkey(), &alice.pubkey(), &[&alice])
            .await
            .unwrap();

        // user-auth cpi transfer with cpi guard doesnt work
        token
            .process_ixs(
                &[cpi_caller::instruction::transfer_one_token(
                    &cpi_caller_id,
                    &spl_token_2022::id(),
                    &alice.pubkey(),
                    token.get_address(),
                    &bob.pubkey(),
                    &alice.pubkey(),
                    do_checked,
                )
                .unwrap()],
                &[&alice],
            )
            .await
            .unwrap_err();

        let alice_state = token.get_account_info(&alice.pubkey()).await.unwrap();
        assert_eq!(alice_state.base.amount, amount);

        // delegate-auth cpi transfer with cpi guard works
        token
            .approve(
                &alice.pubkey(),
                &bob.pubkey(),
                &alice.pubkey(),
                1,
                &[&alice],
            )
            .await
            .unwrap();

        token
            .process_ixs(
                &[cpi_caller::instruction::transfer_one_token(
                    &cpi_caller_id,
                    &spl_token_2022::id(),
                    &alice.pubkey(),
                    token.get_address(),
                    &bob.pubkey(),
                    &bob.pubkey(),
                    do_checked,
                )
                .unwrap()],
                &[&bob],
            )
            .await
            .unwrap();
        amount -= 1;

        let alice_state = token.get_account_info(&alice.pubkey()).await.unwrap();
        assert_eq!(alice_state.base.amount, amount);

        // make sure we didnt break backwards compat somehow
        token
            .disable_cpi_guard(&alice.pubkey(), &alice.pubkey(), &[&alice])
            .await
            .unwrap();

        token
            .process_ixs(
                &[cpi_caller::instruction::transfer_one_token(
                    &cpi_caller_id,
                    &spl_token_2022::id(),
                    &alice.pubkey(),
                    token.get_address(),
                    &bob.pubkey(),
                    &alice.pubkey(),
                    do_checked,
                )
                .unwrap()],
                &[&alice],
            )
            .await
            .unwrap();
        amount -= 1;

        let alice_state = token.get_account_info(&alice.pubkey()).await.unwrap();
        assert_eq!(alice_state.base.amount, amount);
    }
}
