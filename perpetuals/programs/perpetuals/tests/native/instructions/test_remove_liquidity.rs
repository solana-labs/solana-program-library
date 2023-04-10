use {
    crate::utils::{self, pda},
    anchor_lang::{
        prelude::{AccountMeta, Pubkey},
        ToAccountMetas,
    },
    bonfida_test_utils::ProgramTestContextExt,
    perpetuals::{
        instructions::RemoveLiquidityParams,
        state::{custody::Custody, pool::Pool},
    },
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
};

pub async fn test_remove_liquidity(
    program_test_ctx: &mut ProgramTestContext,
    owner: &Keypair,
    payer: &Keypair,
    pool_pda: &Pubkey,
    custody_token_mint: &Pubkey,
    params: RemoveLiquidityParams,
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================

    // Prepare PDA and addresses
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let perpetuals_pda = pda::get_perpetuals_pda().0;
    let custody_pda = pda::get_custody_pda(pool_pda, custody_token_mint).0;
    let custody_token_account_pda =
        pda::get_custody_token_account_pda(pool_pda, custody_token_mint).0;
    let lp_token_mint_pda = pda::get_lp_token_mint_pda(pool_pda).0;

    let receiving_account_address =
        utils::find_associated_token_account(&owner.pubkey(), custody_token_mint).0;
    let lp_token_account_address =
        utils::find_associated_token_account(&owner.pubkey(), &lp_token_mint_pda).0;

    let custody_account = utils::get_account::<Custody>(program_test_ctx, custody_pda).await;
    let custody_oracle_account_address = custody_account.oracle.oracle_account;

    // Save account state before tx execution
    let owner_receiving_account_before = program_test_ctx
        .get_token_account(receiving_account_address)
        .await
        .unwrap();
    let owner_lp_token_account_before = program_test_ctx
        .get_token_account(lp_token_account_address)
        .await
        .unwrap();
    let custody_token_account_before = program_test_ctx
        .get_token_account(custody_token_account_pda)
        .await
        .unwrap();

    let accounts_meta = {
        let accounts = perpetuals::accounts::RemoveLiquidity {
            owner: owner.pubkey(),
            receiving_account: receiving_account_address,
            lp_token_account: lp_token_account_address,
            transfer_authority: transfer_authority_pda,
            perpetuals: perpetuals_pda,
            pool: *pool_pda,
            custody: custody_pda,
            custody_oracle_account: custody_oracle_account_address,
            custody_token_account: custody_token_account_pda,
            lp_token_mint: lp_token_mint_pda,
            token_program: anchor_spl::token::ID,
        };

        let mut accounts_meta = accounts.to_account_metas(None);

        let pool_account = utils::get_account::<Pool>(program_test_ctx, *pool_pda).await;

        // For each token, add custody account as remaining_account
        for custody in &pool_account.custodies {
            accounts_meta.push(AccountMeta {
                pubkey: *custody,
                is_signer: false,
                is_writable: false,
            });
        }

        // For each token, add custody oracle account as remaining_account
        for custody in &pool_account.custodies {
            let custody_account = utils::get_account::<Custody>(program_test_ctx, *custody).await;

            accounts_meta.push(AccountMeta {
                pubkey: custody_account.oracle.oracle_account,
                is_signer: false,
                is_writable: false,
            });
        }

        accounts_meta
    };

    utils::create_and_execute_perpetuals_ix(
        program_test_ctx,
        accounts_meta,
        perpetuals::instruction::RemoveLiquidity { params },
        Some(&payer.pubkey()),
        &[owner, payer],
    )
    .await?;

    // ==== THEN ==============================================================
    let owner_receiving_account_after = program_test_ctx
        .get_token_account(receiving_account_address)
        .await
        .unwrap();
    let owner_lp_token_account_after = program_test_ctx
        .get_token_account(lp_token_account_address)
        .await
        .unwrap();
    let custody_token_account_after = program_test_ctx
        .get_token_account(custody_token_account_pda)
        .await
        .unwrap();

    assert!(owner_receiving_account_after.amount > owner_receiving_account_before.amount);
    assert!(owner_lp_token_account_after.amount < owner_lp_token_account_before.amount);
    assert!(custody_token_account_after.amount < custody_token_account_before.amount);

    Ok(())
}
