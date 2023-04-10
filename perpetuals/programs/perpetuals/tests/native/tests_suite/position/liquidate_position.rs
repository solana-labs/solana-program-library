use {
    crate::{
        instructions,
        utils::{self, fixtures},
    },
    bonfida_test_utils::ProgramTestExt,
    perpetuals::{
        instructions::{OpenPositionParams, SetTestOraclePriceParams},
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
const USER_EXECUTIONER: usize = 8;

const KEYPAIRS_COUNT: usize = 9;

const ETH_DECIMALS: u8 = 9;

pub async fn liquidate_position() {
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
        // Alice: mint 100 ETH
        {
            utils::initialize_and_fund_token_account(
                &mut program_test_ctx,
                &eth_mint,
                &keypairs[USER_ALICE].pubkey(),
                &keypairs[ROOT_AUTHORITY],
                utils::scale(100, ETH_DECIMALS),
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

        // Executioner: init ETH token account
        {
            utils::initialize_token_account(
                &mut program_test_ctx,
                &eth_mint,
                &keypairs[USER_EXECUTIONER].pubkey(),
            )
            .await;
        }
    }

    let (pool_pda, _, _, _, custodies_infos) = utils::setup_pool_with_custodies_and_liquidity(
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
                    // 50_000 = x5, 100_000 = x10
                    max_leverage: 100_000,
                    ..fixtures::pricing_params_regular(false)
                }),
                permissions: None,
                fees: None,
                borrow_rate: None,
            },
            liquidity_amount: utils::scale(100, ETH_DECIMALS),
            payer: utils::copy_keypair(&keypairs[USER_ALICE]),
        }],
    )
    .await;

    // Martin: Open 1 ETH long position x5
    let position_pda = instructions::test_open_position(
        &mut program_test_ctx,
        &keypairs[USER_MARTIN],
        &keypairs[PAYER],
        &pool_pda,
        &eth_mint,
        OpenPositionParams {
            // max price paid (slippage implied)
            price: utils::scale(1_550, ETH_DECIMALS),
            collateral: utils::scale(1, ETH_DECIMALS),
            size: utils::scale(5, ETH_DECIMALS),
            side: Side::Long,
        },
    )
    .await
    .unwrap()
    .0;

    // Alice: Try and fail to liquidate Martin ETH position
    assert!(instructions::test_liquidate(
        &mut program_test_ctx,
        &keypairs[USER_ALICE],
        &keypairs[PAYER],
        &pool_pda,
        &eth_mint,
        &position_pda,
    )
    .await
    .is_err());

    // Makes ETH price to drop 10%
    {
        let eth_test_oracle_pda = custodies_infos[0].test_oracle_pda;
        let eth_custody_pda = custodies_infos[0].custody_pda;

        let publish_time = utils::get_current_unix_timestamp(&mut program_test_ctx).await;

        instructions::test_set_test_oracle_price(
            &mut program_test_ctx,
            &keypairs[MULTISIG_MEMBER_A],
            &keypairs[PAYER],
            &pool_pda,
            &eth_custody_pda,
            &eth_test_oracle_pda,
            SetTestOraclePriceParams {
                price: utils::scale(1_350, ETH_DECIMALS),
                expo: -(ETH_DECIMALS as i32),
                conf: utils::scale(10, ETH_DECIMALS),
                publish_time,
            },
            multisig_signers,
        )
        .await
        .unwrap();
    }

    // Price drop makes the position to go over authorized leverage

    // Executioner: Liquidate Martin ETH position
    instructions::test_liquidate(
        &mut program_test_ctx,
        &keypairs[USER_EXECUTIONER],
        &keypairs[PAYER],
        &pool_pda,
        &eth_mint,
        &position_pda,
    )
    .await
    .unwrap();

    // Check user final balance
    {
        let martin_eth_pda =
            utils::find_associated_token_account(&keypairs[USER_MARTIN].pubkey(), &eth_mint).0;

        let martin_eth_balance =
            utils::get_token_account_balance(&mut program_test_ctx, martin_eth_pda).await;

        assert_eq!(
            martin_eth_balance,
            utils::scale_f64(1.348985147, ETH_DECIMALS)
        );
    }
}
