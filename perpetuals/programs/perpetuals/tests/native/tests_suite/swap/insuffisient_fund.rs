use {
    crate::{
        instructions,
        utils::{self, fixtures},
    },
    bonfida_test_utils::ProgramTestExt,
    perpetuals::instructions::SwapParams,
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

const USDC_DECIMALS: u8 = 6;
const ETH_DECIMALS: u8 = 9;

pub async fn insuffisient_fund() {
    let mut program_test = ProgramTest::default();

    // Initialize the accounts that will be used during the test suite
    let keypairs =
        utils::create_and_fund_multiple_accounts(&mut program_test, KEYPAIRS_COUNT).await;

    // Initialize mints
    let usdc_mint = program_test
        .add_mint(None, USDC_DECIMALS, &keypairs[ROOT_AUTHORITY].pubkey())
        .0;
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
        // Alice: mint 7.5k USDC and 5 ETH
        {
            utils::initialize_and_fund_token_account(
                &mut program_test_ctx,
                &usdc_mint,
                &keypairs[USER_ALICE].pubkey(),
                &keypairs[ROOT_AUTHORITY],
                utils::scale(7_500, USDC_DECIMALS),
            )
            .await;

            utils::initialize_and_fund_token_account(
                &mut program_test_ctx,
                &eth_mint,
                &keypairs[USER_ALICE].pubkey(),
                &keypairs[ROOT_AUTHORITY],
                utils::scale(5, ETH_DECIMALS),
            )
            .await;
        }

        // Martin: mint 1k USDC, 10 ETH
        {
            utils::initialize_and_fund_token_account(
                &mut program_test_ctx,
                &usdc_mint,
                &keypairs[USER_MARTIN].pubkey(),
                &keypairs[ROOT_AUTHORITY],
                utils::scale(1_000, USDC_DECIMALS),
            )
            .await;

            utils::initialize_and_fund_token_account(
                &mut program_test_ctx,
                &eth_mint,
                &keypairs[USER_MARTIN].pubkey(),
                &keypairs[ROOT_AUTHORITY],
                utils::scale(10, ETH_DECIMALS),
            )
            .await;
        }
    }

    // Set the pool with 50%/50% ETH/USDC liquidity
    let (pool_pda, _, _, _, _) = utils::setup_pool_with_custodies_and_liquidity(
        &mut program_test_ctx,
        &keypairs[MULTISIG_MEMBER_A],
        "FOO",
        &keypairs[PAYER],
        multisig_signers,
        vec![
            utils::SetupCustodyWithLiquidityParams {
                setup_custody_params: utils::SetupCustodyParams {
                    mint: usdc_mint,
                    decimals: USDC_DECIMALS,
                    is_stable: true,
                    target_ratio: utils::ratio_from_percentage(50.0),
                    min_ratio: utils::ratio_from_percentage(0.0),
                    max_ratio: utils::ratio_from_percentage(100.0),
                    initial_price: utils::scale(1, USDC_DECIMALS),
                    initial_conf: utils::scale_f64(0.01, USDC_DECIMALS),
                    pricing_params: None,
                    permissions: None,
                    fees: None,
                    borrow_rate: None,
                },
                liquidity_amount: utils::scale(7_500, USDC_DECIMALS),
                payer: utils::copy_keypair(&keypairs[USER_ALICE]),
            },
            utils::SetupCustodyWithLiquidityParams {
                setup_custody_params: utils::SetupCustodyParams {
                    mint: eth_mint,
                    decimals: ETH_DECIMALS,
                    is_stable: false,
                    target_ratio: utils::ratio_from_percentage(50.0),
                    min_ratio: utils::ratio_from_percentage(0.0),
                    max_ratio: utils::ratio_from_percentage(100.0),
                    initial_price: utils::scale(1_500, ETH_DECIMALS),
                    initial_conf: utils::scale(10, ETH_DECIMALS),
                    pricing_params: None,
                    permissions: None,
                    fees: None,
                    borrow_rate: None,
                },
                liquidity_amount: utils::scale(5, ETH_DECIMALS),
                payer: utils::copy_keypair(&keypairs[USER_ALICE]),
            },
        ],
    )
    .await;

    // Swap with not enough collateral should fail
    {
        // Martin: Swap 5k USDC for ETH
        assert!(instructions::test_swap(
            &mut program_test_ctx,
            &keypairs[USER_MARTIN],
            &keypairs[PAYER],
            &pool_pda,
            &eth_mint,
            // The program receives USDC
            &usdc_mint,
            SwapParams {
                amount_in: utils::scale(5_000, USDC_DECIMALS),
                min_amount_out: 0,
            },
        )
        .await
        .is_err());
    }

    // Swap for more token that the pool own should fail
    {
        // Martin: Swap 10 ETH for (15k) USDC
        assert!(instructions::test_swap(
            &mut program_test_ctx,
            &keypairs[USER_MARTIN],
            &keypairs[PAYER],
            &pool_pda,
            &usdc_mint,
            // The program receives ETH
            &eth_mint,
            SwapParams {
                amount_in: utils::scale(10, ETH_DECIMALS),
                min_amount_out: 0,
            },
        )
        .await
        .is_err());
    }
}
