//! Vault Init instruction handler

use {
    crate::{traits::Init, vault_info::VaultInfo},
    solana_farm_sdk::{
        id::zero,
        instruction::vault::VaultInstruction,
        program::{pda, protocol::saber},
        token::Token,
        vault::Vault,
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

impl Init for VaultInstruction {
    fn init(vault: &Vault, accounts: &[AccountInfo], step: u64) -> ProgramResult {
        #[allow(clippy::deprecated_cfg_attr)]
        #[cfg_attr(rustfmt, rustfmt_skip)]
        if let [
            admin_account,
            _vault_metadata,
            vault_info_account,
            vault_authority,
            vault_program,
            _system_program,
            _spl_token_program,
            _associated_token_program,
            rent_program,
            farm_program,
            vault_token_mint,
            vault_token_ref,
            vault_stake_info,
            vault_miner_account,
            fees_account_a,
            fees_account_b,
            usdc_token_custody,
            wrapped_token_custody,
            lp_token_custody,
            usdc_token_mint,
            non_usdc_token_mint,
            wrapped_token_mint,
            lp_token_mint,
            sbr_token_reward_custody,
            iou_token_reward_custody,
            sbr_token_mint,
            iou_token_mint,
            quarry,
            rewarder
            ] = accounts
        {
            // validate accounts
            if vault_authority.key != &vault.vault_authority
                || vault_token_ref.key != &vault.vault_token_ref
                || vault_program.key != &vault.vault_program_id
            {
                msg!("Error: Invalid Vault accounts");
                return Err(ProgramError::InvalidArgument);
            }

            if step <= 1 {
                // init vault authority account
                msg!("Init vault authority");
                pda::init_system_account(
                    admin_account,
                    vault_authority,
                    &vault.vault_program_id,
                    &vault.vault_program_id,
                    &[b"vault_authority", vault.name.as_bytes()],
                    0,
                )?;

                // init vault info account
                msg!("Init vault info");
                pda::init_system_account(
                    admin_account,
                    vault_info_account,
                    &vault.vault_program_id,
                    &vault.vault_program_id,
                    &[b"info_account", vault.name.as_bytes()],
                    VaultInfo::LEN,
                )?;
                let mut vault_info = VaultInfo::new(vault_info_account);
                vault_info.init(&vault.name)?;

                // init vault token mint
                msg!("Init vault token mint");
                let vault_token = Token::unpack(&vault_token_ref.try_borrow_data()?)?;
                if vault_token_mint.key != &vault_token.mint {
                    msg!("Error: Invalid Vault token mint");
                    return Err(ProgramError::InvalidArgument);
                }
                pda::init_mint(
                    admin_account,
                    vault_token_mint,
                    vault_authority,
                    rent_program,
                    &vault.vault_program_id,
                    &[b"vault_token_mint", vault.name.as_bytes()],
                    vault_token.decimals,
                )?;

                // init stake info
                msg!("Init vault miner");
                pda::init_associated_token_account(
                    admin_account,
                    vault_stake_info,
                    vault_miner_account,
                    lp_token_mint,
                    rent_program,
                )?;

                if vault_stake_info.data_is_empty() {
                    msg!("Init stake info");
                    let seeds: &[&[&[u8]]] = &[&[
                        b"vault_authority",
                        vault.name.as_bytes(),
                        &[vault.authority_bump],
                    ]];
                    saber::user_init_with_seeds(
                        &[
                            vault_authority.clone(),
                            admin_account.clone(),
                            farm_program.clone(),
                            lp_token_mint.clone(),
                            vault_stake_info.clone(),
                            vault_miner_account.clone(),
                            quarry.clone(),
                            rewarder.clone(),
                        ],
                        seeds,
                    )?;
                }
            }

            if step == 0 || step == 2 {
                // init token accounts
                msg!("Init fees account a");
                pda::init_token_account(
                    admin_account,
                    fees_account_a,
                    sbr_token_mint,
                    vault_authority,
                    rent_program,
                    &vault.vault_program_id,
                    &[b"fees_account_a", vault.name.as_bytes()],
                )?;

                msg!("Init fees account b");
                pda::init_token_account(
                    admin_account,
                    fees_account_b,
                    non_usdc_token_mint,
                    vault_authority,
                    rent_program,
                    &vault.vault_program_id,
                    &[b"fees_account_b", vault.name.as_bytes()],
                )?;

                msg!("Init lp token custody account");
                pda::init_token_account(
                    admin_account,
                    lp_token_custody,
                    lp_token_mint,
                    vault_authority,
                    rent_program,
                    &vault.vault_program_id,
                    &[b"lp_token_custody", vault.name.as_bytes()],
                )?;

                msg!("Init USDC token custody account");
                pda::init_token_account(
                    admin_account,
                    usdc_token_custody,
                    usdc_token_mint,
                    vault_authority,
                    rent_program,
                    &vault.vault_program_id,
                    &[b"token_a_custody", vault.name.as_bytes()],
                )?;

                if wrapped_token_custody.key != &zero::id() {
                    msg!("Init wrapped USDC token custody account");
                    pda::init_token_account(
                        admin_account,
                        wrapped_token_custody,
                        wrapped_token_mint,
                        vault_authority,
                        rent_program,
                        &vault.vault_program_id,
                        &[b"token_b_custody", vault.name.as_bytes()],
                    )?;
                }

                msg!("Init SBR token reward custody account");
                pda::init_token_account(
                    admin_account,
                    sbr_token_reward_custody,
                    sbr_token_mint,
                    vault_authority,
                    rent_program,
                    &vault.vault_program_id,
                    &[b"token_a_reward_custody", vault.name.as_bytes()],
                )?;

                if *iou_token_reward_custody.key != zero::id() {
                    msg!("Init IOU token reward custody account");
                    pda::init_token_account(
                        admin_account,
                        iou_token_reward_custody,
                        iou_token_mint,
                        vault_authority,
                        rent_program,
                        &vault.vault_program_id,
                        &[b"token_b_reward_custody", vault.name.as_bytes()],
                    )?;
                }
            }

            Ok(())
        } else {
            Err(ProgramError::NotEnoughAccountKeys)
        }
    }
}
