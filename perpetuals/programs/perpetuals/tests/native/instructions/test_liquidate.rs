use {
    crate::utils::{self, pda},
    anchor_lang::{prelude::Pubkey, ToAccountMetas},
    bonfida_test_utils::ProgramTestContextExt,
    perpetuals::{
        instructions::LiquidateParams,
        state::{custody::Custody, position::Position},
    },
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
};

pub async fn test_liquidate(
    program_test_ctx: &mut ProgramTestContext,
    liquidator: &Keypair,
    payer: &Keypair,
    pool_pda: &Pubkey,
    custody_token_mint: &Pubkey,
    position_pda: &Pubkey,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================
    let owner = {
        let position_account =
            utils::get_account::<Position>(program_test_ctx, *position_pda).await;
        position_account.owner
    };

    // Prepare PDA and addresses
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let perpetuals_pda = pda::get_perpetuals_pda().0;
    let custody_pda = pda::get_custody_pda(pool_pda, custody_token_mint).0;
    let custody_token_account_pda =
        pda::get_custody_token_account_pda(pool_pda, custody_token_mint).0;

    let receiving_account_address =
        utils::find_associated_token_account(&owner, custody_token_mint).0;

    let rewards_receiving_account_address =
        utils::find_associated_token_account(&liquidator.pubkey(), custody_token_mint).0;

    let custody_account = utils::get_account::<Custody>(program_test_ctx, custody_pda).await;
    let custody_oracle_account_address = custody_account.oracle.oracle_account;

    // Save account state before tx execution
    let receiving_account_before = program_test_ctx
        .get_token_account(receiving_account_address)
        .await
        .unwrap();
    let custody_token_account_before = program_test_ctx
        .get_token_account(custody_token_account_pda)
        .await
        .unwrap();
    let rewards_receiving_account_before = program_test_ctx
        .get_token_account(rewards_receiving_account_address)
        .await
        .unwrap();

    utils::create_and_execute_perpetuals_ix(
        program_test_ctx,
        perpetuals::accounts::Liquidate {
            signer: liquidator.pubkey(),
            rewards_receiving_account: rewards_receiving_account_address,
            receiving_account: receiving_account_address,
            transfer_authority: transfer_authority_pda,
            perpetuals: perpetuals_pda,
            pool: *pool_pda,
            position: *position_pda,
            custody: custody_pda,
            custody_oracle_account: custody_oracle_account_address,
            custody_token_account: custody_token_account_pda,
            token_program: anchor_spl::token::ID,
        }
        .to_account_metas(None),
        perpetuals::instruction::Liquidate {
            params: LiquidateParams {},
        },
        Some(&payer.pubkey()),
        &[liquidator, payer],
    )
    .await?;

    // ==== THEN ==============================================================
    // Check the balance change
    {
        let receiving_account_after = program_test_ctx
            .get_token_account(receiving_account_address)
            .await
            .unwrap();
        let custody_token_account_after = program_test_ctx
            .get_token_account(custody_token_account_pda)
            .await
            .unwrap();
        let rewards_receiving_account_after = program_test_ctx
            .get_token_account(rewards_receiving_account_address)
            .await
            .unwrap();

        assert!(receiving_account_after.amount >= receiving_account_before.amount);
        assert!(custody_token_account_after.amount <= custody_token_account_before.amount);
        assert!(rewards_receiving_account_after.amount > rewards_receiving_account_before.amount);
    }

    Ok(())
}
