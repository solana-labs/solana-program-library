use {
    crate::utils::{self, pda},
    anchor_lang::{prelude::AccountMeta, ToAccountMetas},
    perpetuals::{
        instructions::InitParams,
        state::{multisig::Multisig, perpetuals::Perpetuals},
    },
    solana_program_test::{BanksClientError, ProgramTestContext},
    solana_sdk::signer::{keypair::Keypair, Signer},
};

pub async fn test_init(
    program_test_ctx: &mut ProgramTestContext,
    upgrade_authority: &Keypair,
    params: InitParams,
    multisig_signers: &[&Keypair],
) -> std::result::Result<(), BanksClientError> {
    // ==== WHEN ==============================================================
    let perpetuals_program_data_pda = pda::get_program_data_pda().0;
    let (multisig_pda, multisig_bump) = pda::get_multisig_pda();
    let (transfer_authority_pda, transfer_authority_bump) = pda::get_transfer_authority_pda();
    let (perpetuals_pda, perpetuals_bump) = pda::get_perpetuals_pda();

    let accounts_meta = {
        let accounts = perpetuals::accounts::Init {
            upgrade_authority: upgrade_authority.pubkey(),
            multisig: multisig_pda,
            transfer_authority: transfer_authority_pda,
            perpetuals: perpetuals_pda,
            perpetuals_program: perpetuals::ID,
            perpetuals_program_data: perpetuals_program_data_pda,
            system_program: anchor_lang::system_program::ID,
            token_program: anchor_spl::token::ID,
        };

        let mut accounts_meta = accounts.to_account_metas(None);

        for signer in multisig_signers {
            accounts_meta.push(AccountMeta {
                pubkey: signer.pubkey(),
                is_signer: true,
                is_writable: false,
            });
        }

        accounts_meta
    };

    utils::create_and_execute_perpetuals_ix(
        program_test_ctx,
        accounts_meta,
        perpetuals::instruction::Init { params },
        Some(&upgrade_authority.pubkey()),
        &[&[upgrade_authority], multisig_signers].concat(),
    )
    .await?;

    // ==== THEN ==============================================================
    let perpetuals_account =
        utils::get_account::<Perpetuals>(program_test_ctx, perpetuals_pda).await;

    // Assert permissions
    {
        let p = perpetuals_account.permissions;

        assert_eq!(p.allow_swap, params.allow_swap);
        assert_eq!(p.allow_add_liquidity, params.allow_add_liquidity);
        assert_eq!(p.allow_remove_liquidity, params.allow_remove_liquidity);
        assert_eq!(p.allow_open_position, params.allow_open_position);
        assert_eq!(p.allow_close_position, params.allow_close_position);
        assert_eq!(p.allow_pnl_withdrawal, params.allow_pnl_withdrawal);
        assert_eq!(
            p.allow_collateral_withdrawal,
            params.allow_collateral_withdrawal
        );
        assert_eq!(p.allow_size_change, params.allow_size_change);
    }

    assert_eq!(
        perpetuals_account.transfer_authority_bump,
        transfer_authority_bump
    );
    assert_eq!(perpetuals_account.perpetuals_bump, perpetuals_bump);

    let multisig_account = utils::get_account::<Multisig>(program_test_ctx, multisig_pda).await;

    // Assert multisig
    {
        assert_eq!(multisig_account.bump, multisig_bump);
        assert_eq!(multisig_account.min_signatures, params.min_signatures);

        // Check signers
        {
            for (i, signer) in multisig_signers.iter().enumerate() {
                assert_eq!(multisig_account.signers[i], signer.pubkey());
            }
        }
    }

    Ok(())
}
