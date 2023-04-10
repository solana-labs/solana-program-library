use {
    crate::{
        instructions,
        utils::{self, fixtures},
    },
    bonfida_test_utils::ProgramTestExt,
    perpetuals::{
        instructions::OpenPositionParams,
        state::{custody::PricingParams, position::Side},
    },
    solana_program_test::ProgramTest,
    solana_sdk::signer::Signer,
};

const ROOT_AUTHORITY: usize = 0;
const PERPETUALS_UPGRADE_AUTHORITY: usize = 1;
const MULTISIG_MEMBER_A: usize = 2;
const MULTISIG_MEMBER_B: usize = 3;
const MULTISIG_MEMBER_C: usize = 4;
const PAYER: usize = 5;
const USER_ALICE: usize = 6;
const USER_MARTIN: usize = 7;

const KEYPAIRS_COUNT: usize = 8;

const ETH_DECIMALS: u8 = 9;

pub async fn min_max_leverage() {
    let mut program_test = ProgramTest::default();

    // Initialize the accounts that will be used during the test suite
    let keypairs =
        utils::create_and_fund_multiple_accounts(&mut program_test, KEYPAIRS_COUNT).await;

    // Initialize mints
    let eth_mint = program_test
        .add_mint(None, ETH_DECIMALS, &keypairs[ROOT_AUTHORITY].pubkey())
        .0;

    // Deploy the perpetuals program onchain as upgradeable program
    utils::add_perpetuals_program(&mut program_test, &keypairs[PERPETUALS_UPGRADE_AUTHORITY]).await;

    // Start the client and connect to localnet validator
    let mut program_test_ctx = program_test.start_with_context().await;

    let upgrade_authority = &keypairs[PERPETUALS_UPGRADE_AUTHORITY];

    let multisig_signers = &[
        &keypairs[MULTISIG_MEMBER_A],
        &keypairs[MULTISIG_MEMBER_B],
        &keypairs[MULTISIG_MEMBER_C],
    ];

    instructions::test_init(
        &mut program_test_ctx,
        upgrade_authority,
        fixtures::init_params_permissions_full(1),
        multisig_signers,
    )
    .await
    .unwrap();

    // Initialize and fund associated token accounts
    {
        // Alice: mint 10k ETH
        {
            utils::initialize_and_fund_token_account(
                &mut program_test_ctx,
                &eth_mint,
                &keypairs[USER_ALICE].pubkey(),
                &keypairs[ROOT_AUTHORITY],
                utils::scale(10_000, ETH_DECIMALS),
            )
            .await;
        }

        // Martin: mint 2 ETH
        {
            utils::initialize_and_fund_token_account(
                &mut program_test_ctx,
                &eth_mint,
                &keypairs[USER_MARTIN].pubkey(),
                &keypairs[ROOT_AUTHORITY],
                utils::scale(2, ETH_DECIMALS),
            )
            .await;
        }
    }

    let (pool_pda, _, _, _, _) = utils::setup_pool_with_custodies_and_liquidity(
        &mut program_test_ctx,
        &keypairs[MULTISIG_MEMBER_A],
        "FOO",
        &keypairs[PAYER],
        multisig_signers,
        vec![utils::SetupCustodyWithLiquidityParams {
            setup_custody_params: utils::SetupCustodyParams {
                mint: eth_mint,
                decimals: ETH_DECIMALS,
                is_stable: false,
                target_ratio: utils::ratio_from_percentage(100.0),
                min_ratio: utils::ratio_from_percentage(0.0),
                max_ratio: utils::ratio_from_percentage(100.0),
                initial_price: utils::scale(1_500, ETH_DECIMALS),
                initial_conf: utils::scale(10, ETH_DECIMALS),
                pricing_params: Some(PricingParams {
                    // Expressed in BPS, with BPS = 10_000
                    // 10_000 = x1, 50_000 = x5
                    max_leverage: 100_000,
                    min_initial_leverage: 10_000,
                    max_initial_leverage: 100_000,
                    ..fixtures::pricing_params_regular(false)
                }),
                permissions: None,
                fees: None,
                borrow_rate: None,
            },
            liquidity_amount: utils::scale(10_000, ETH_DECIMALS),
            payer: utils::copy_keypair(&keypairs[USER_ALICE]),
        }],
    )
    .await;

    // Martin: Open 1 ETH long position x10 should fail
    // Fails because fees increase ETH entry price
    assert!(instructions::test_open_position(
        &mut program_test_ctx,
        &keypairs[USER_MARTIN],
        &keypairs[PAYER],
        &pool_pda,
        &eth_mint,
        OpenPositionParams {
            // max price paid (slippage implied)
            price: utils::scale(1_550, ETH_DECIMALS),
            collateral: utils::scale(1, ETH_DECIMALS),
            size: utils::scale(10, ETH_DECIMALS),
            side: Side::Long,
        },
    )
    .await
    .is_err());

    // Martin: Open 1 ETH long position x0.5 should fail
    assert!(instructions::test_open_position(
        &mut program_test_ctx,
        &keypairs[USER_MARTIN],
        &keypairs[PAYER],
        &pool_pda,
        &eth_mint,
        OpenPositionParams {
            // max price paid (slippage implied)
            price: utils::scale(1_550, ETH_DECIMALS),
            collateral: utils::scale(1, ETH_DECIMALS),
            size: utils::scale_f64(0.5, ETH_DECIMALS),
            side: Side::Long,
        },
    )
    .await
    .is_err());
}
