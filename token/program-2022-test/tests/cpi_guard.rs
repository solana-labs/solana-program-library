#![cfg(feature = "test-sbf")]

mod program_test;
use {
    program_test::{TestContext, TokenContext},
    solana_program_test::{
        processor,
        tokio::{self, sync::Mutex},
        ProgramTest,
    },
    solana_sdk::{pubkey::Pubkey, signature::Signer, signer::keypair::Keypair},
    spl_instruction_padding::instruction::wrap_instruction,
    spl_token_2022::{
        extension::{
            cpi_guard::{self, CpiGuard},
            ExtensionType,
        },
        instruction,
        processor::Processor as SplToken2022Processor,
    },
    std::sync::Arc,
};

// set up a bank and bank client with spl token 2022 and the instruction padder
// also creates a token with no extensions and inits two token accounts
async fn make_context() -> (TestContext, Pubkey) {
    if std::env::var("BPF_OUT_DIR").is_err() && std::env::var("SBF_OUT_DIR").is_err() {
        panic!("CpiGuard tests MUST be invoked with `cargo test-sbf`, NOT `cargo test --feature test-sbf`. \
                In a non-BPF context, `get_stack_height()` always returns 0, and all tests WILL fail.");
    }

    let instruction_padding_id = Keypair::new().pubkey();

    let mut program_test = ProgramTest::new(
        "spl_token_2022",
        spl_token_2022::id(),
        processor!(SplToken2022Processor::process),
    );

    program_test.add_program(
        "spl_instruction_padding",
        instruction_padding_id,
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

    (test_context, instruction_padding_id)
}

#[tokio::test]
async fn test_cpi_guard_enable_disable() {
    let (context, instruction_pad_id) = make_context().await;
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
            &[wrap_instruction(
                instruction_pad_id,
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
            &[wrap_instruction(
                instruction_pad_id,
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

    // guard remains disabled
    let alice_state = token.get_account_info(&alice.pubkey()).await.unwrap();
    let extension = alice_state.get_extension::<CpiGuard>().unwrap();
    assert!(!bool::from(extension.lock_cpi));
}

#[tokio::test]
async fn test_cpi_guard_transfer() {
    let (context, instruction_pad_id) = make_context().await;
    let TokenContext {
        token,
        mint_authority,
        alice,
        bob,
        ..
    } = context.token_context.unwrap();

    let mk_transfer = |authority, do_checked| {
        wrap_instruction(
            instruction_pad_id,
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
            .process_ixs(&[mk_transfer(alice.pubkey(), do_checked)], &[&alice])
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
            .process_ixs(&[mk_transfer(bob.pubkey(), do_checked)], &[&bob])
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
            .process_ixs(&[mk_transfer(alice.pubkey(), do_checked)], &[&alice])
            .await
            .unwrap();
        amount -= 1;

        let alice_state = token.get_account_info(&alice.pubkey()).await.unwrap();
        assert_eq!(alice_state.base.amount, amount);
    }
}

#[tokio::test]
async fn test_cpi_guard_burn() {
    let (context, instruction_pad_id) = make_context().await;
    let TokenContext {
        token,
        mint_authority,
        alice,
        bob,
        ..
    } = context.token_context.unwrap();

    let mk_burn = |authority, do_checked| {
        wrap_instruction(
            instruction_pad_id,
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

    // burn works normally
    token
        .burn(
            &alice.pubkey(),
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

        // user-auth cpi burn with cpi guard doesnt work
        token
            .process_ixs(&[mk_burn(alice.pubkey(), do_checked)], &[&alice])
            .await
            .unwrap_err();

        let alice_state = token.get_account_info(&alice.pubkey()).await.unwrap();
        assert_eq!(alice_state.base.amount, amount);

        // delegate-auth cpi burn with cpi guard works
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
            .process_ixs(&[mk_burn(bob.pubkey(), do_checked)], &[&bob])
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
            .process_ixs(&[mk_burn(alice.pubkey(), do_checked)], &[&alice])
            .await
            .unwrap();
        amount -= 1;

        let alice_state = token.get_account_info(&alice.pubkey()).await.unwrap();
        assert_eq!(alice_state.base.amount, amount);
    }
}
