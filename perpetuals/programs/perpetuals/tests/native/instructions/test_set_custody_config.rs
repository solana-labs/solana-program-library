use {
    crate::utils::{self, pda},
    anchor_lang::{
        prelude::{AccountMeta, Pubkey},
        ToAccountMetas,
    },
    perpetuals::{
        instructions::SetCustodyConfigParams,
        state::{custody::Custody, multisig::Multisig},
    },
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
};

pub async fn test_set_custody_config(
    program_test_ctx: &mut ProgramTestContext,
    admin: &Keypair,
    payer: &Keypair,
    pool_pda: &Pubkey,
    custody_pda: &Pubkey,
    params: SetCustodyConfigParams,
    multisig_signers: &[&Keypair],
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================
    let multisig_pda = pda::get_multisig_pda().0;
    let multisig_account = utils::get_account::<Multisig>(program_test_ctx, multisig_pda).await;

    // One Tx per multisig signer
    for i in 0..multisig_account.min_signatures {
        let signer: &Keypair = multisig_signers[i as usize];

        let accounts_meta = {
            let accounts = perpetuals::accounts::SetCustodyConfig {
                admin: admin.pubkey(),
                multisig: multisig_pda,
                pool: *pool_pda,
                custody: *custody_pda,
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
            perpetuals::instruction::SetCustodyConfig {
                params: params.clone(),
            },
            Some(&payer.pubkey()),
            &[admin, payer, signer],
        )
        .await?;
    }

    // ==== THEN ==============================================================
    let custody_account = utils::get_account::<Custody>(program_test_ctx, *custody_pda).await;

    // Check custody account
    {
        assert_eq!(custody_account.pool, *pool_pda);
        assert_eq!(custody_account.is_stable, params.is_stable);
        assert_eq!(custody_account.oracle, params.oracle);
        assert_eq!(custody_account.pricing, params.pricing);
        assert_eq!(custody_account.permissions, params.permissions);
        assert_eq!(custody_account.fees, params.fees);
    }

    Ok(())
}
