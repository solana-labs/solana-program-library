use {
    crate::utils::{self, pda},
    anchor_lang::{prelude::AccountMeta, ToAccountMetas},
    perpetuals::{
        instructions::AddPoolParams,
        state::{multisig::Multisig, perpetuals::Perpetuals, pool::Pool},
    },
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
    std::str::FromStr,
};

pub async fn test_add_pool(
    program_test_ctx: &mut ProgramTestContext,
    // Admin must be a part of the multisig
    admin: &Keypair,
    payer: &Keypair,
    pool_name: &str,
    multisig_signers: &[&Keypair],
) -> std::result::Result<
    (
        anchor_lang::prelude::Pubkey,
        u8,
        anchor_lang::prelude::Pubkey,
        u8,
    ),
    BanksClientError,
> {
    // ==== WHEN ==============================================================
    let multisig_pda = pda::get_multisig_pda().0;
    let transfer_authority_pda = pda::get_transfer_authority_pda().0;
    let perpetuals_pda = pda::get_perpetuals_pda().0;
    let (pool_pda, pool_bump) = pda::get_pool_pda(String::from_str(pool_name).unwrap());
    let (lp_token_mint_pda, lp_token_mint_bump) = pda::get_lp_token_mint_pda(&pool_pda);

    let multisig_account = utils::get_account::<Multisig>(program_test_ctx, multisig_pda).await;

    // One Tx per multisig signer
    for i in 0..multisig_account.min_signatures {
        let signer: &Keypair = multisig_signers[i as usize];

        let accounts_meta = {
            let accounts = perpetuals::accounts::AddPool {
                admin: admin.pubkey(),
                multisig: multisig_pda,
                transfer_authority: transfer_authority_pda,
                perpetuals: perpetuals_pda,
                pool: pool_pda,
                lp_token_mint: lp_token_mint_pda,
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
            perpetuals::instruction::AddPool {
                params: AddPoolParams {
                    name: String::from_str(pool_name).unwrap(),
                },
            },
            Some(&payer.pubkey()),
            &[admin, payer, signer],
        )
        .await?;
    }

    // ==== THEN ==============================================================
    let pool_account = utils::get_account::<Pool>(program_test_ctx, pool_pda).await;

    assert_eq!(pool_account.name.as_str(), pool_name);
    assert_eq!(pool_account.bump, pool_bump);
    assert_eq!(pool_account.lp_token_bump, lp_token_mint_bump);

    let perpetuals_account =
        utils::get_account::<Perpetuals>(program_test_ctx, perpetuals_pda).await;

    assert_eq!(*perpetuals_account.pools.last().unwrap(), pool_pda);
    assert_eq!(
        perpetuals_account.inception_time,
        pool_account.inception_time
    );

    Ok((pool_pda, pool_bump, lp_token_mint_pda, lp_token_mint_bump))
}
