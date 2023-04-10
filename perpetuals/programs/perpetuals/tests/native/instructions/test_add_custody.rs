use {
    crate::utils::{self, pda},
    anchor_lang::{
        prelude::{AccountMeta, Pubkey},
        ToAccountMetas,
    },
    perpetuals::{
        instructions::AddCustodyParams,
        state::{custody::Custody, multisig::Multisig, pool::Pool},
    },
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
};

#[allow(clippy::too_many_arguments)]
pub async fn test_add_custody(
    program_test_ctx: &mut ProgramTestContext,
    admin: &Keypair,
    payer: &Keypair,
    pool_pda: &Pubkey,
    custody_token_mint: &Pubkey,
    custody_token_decimals: u8,
    params: AddCustodyParams,
    multisig_signers: &[&Keypair],
) -> std::result::Result<(anchor_lang::prelude::Pubkey, u8), BanksClientError> {
    // ==== WHEN ==============================================================
    let multisig_pda = pda::get_multisig_pda().0;
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let perpetuals_pda = pda::get_perpetuals_pda().0;
    let (custody_pda, custody_bump) = pda::get_custody_pda(pool_pda, custody_token_mint);
    let (custody_token_account_pda, custody_token_account_bump) =
        pda::get_custody_token_account_pda(pool_pda, custody_token_mint);

    let multisig_account = utils::get_account::<Multisig>(program_test_ctx, multisig_pda).await;

    // One Tx per multisig signer
    for i in 0..multisig_account.min_signatures {
        let signer: &Keypair = multisig_signers[i as usize];

        let accounts_meta = {
            let accounts = perpetuals::accounts::AddCustody {
                admin: admin.pubkey(),
                multisig: multisig_pda,
                transfer_authority: transfer_authority_pda,
                perpetuals: perpetuals_pda,
                pool: *pool_pda,
                custody: custody_pda,
                custody_token_account: custody_token_account_pda,
                custody_token_mint: *custody_token_mint,
                system_program: anchor_lang::system_program::ID,
                token_program: anchor_spl::token::ID,
                rent: solana_program::sysvar::rent::ID,
            };

            let mut accounts_meta = accounts.to_account_metas(None);

            accounts_meta.push(AccountMeta {
                pubkey: signer.pubkey(),
                is_signer: true,
                is_writable: false,
            });

            accounts_meta
        };

        utils::create_and_execute_perpetuals_ix(
            program_test_ctx,
            accounts_meta,
            perpetuals::instruction::AddCustody {
                params: params.clone(),
            },
            Some(&payer.pubkey()),
            &[admin, payer, signer],
        )
        .await?;
    }

    // ==== THEN ==============================================================
    let custody_account = utils::get_account::<Custody>(program_test_ctx, custody_pda).await;

    // Check custody account
    {
        assert_eq!(custody_account.pool, *pool_pda);
        assert_eq!(custody_account.mint, *custody_token_mint);
        assert_eq!(custody_account.token_account, custody_token_account_pda);
        assert_eq!(custody_account.decimals, custody_token_decimals);
        assert_eq!(custody_account.is_stable, params.is_stable);
        assert_eq!(custody_account.oracle, params.oracle);
        assert_eq!(custody_account.pricing, params.pricing);
        assert_eq!(custody_account.permissions, params.permissions);
        assert_eq!(custody_account.fees, params.fees);
        assert_eq!(custody_account.borrow_rate, params.borrow_rate,);
        assert_eq!(custody_account.bump, custody_bump);
        assert_eq!(
            custody_account.token_account_bump,
            custody_token_account_bump
        );
    }

    let pool_account = utils::get_account::<Pool>(program_test_ctx, *pool_pda).await;

    // Check pool token
    {
        let idx = pool_account.get_token_id(&custody_pda).unwrap();
        let custody = pool_account.custodies[idx];
        let ratios = pool_account.ratios[idx];

        assert_eq!(custody, custody_pda);
        assert_eq!(ratios.target, params.ratios[idx].target);
        assert_eq!(ratios.min, params.ratios[idx].min);
        assert_eq!(ratios.max, params.ratios[idx].max);
    }

    Ok((custody_pda, custody_bump))
}
