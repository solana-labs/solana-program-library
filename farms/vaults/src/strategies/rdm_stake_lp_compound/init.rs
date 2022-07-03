//! Vault Init instruction handler

use {
    crate::{traits::Init, vault_info::VaultInfo},
    solana_farm_sdk::{
        id::zero,
        instruction::vault::VaultInstruction,
        program::{
            pda,
            protocol::raydium::{RaydiumUserStakeInfo, RaydiumUserStakeInfoV4},
        },
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
            rent_program,
            farm_program,
            vault_token_mint,
            vault_token_ref,
            vault_stake_info,
            vault_stake_info_v4,
            fees_account_a,
            fees_account_b,
            token_a_custody,
            token_b_custody,
            lp_token_custody,
            token_a_mint,
            token_b_mint,
            lp_token_mint,
            token_a_reward_custody,
            token_b_reward_custody,
            token_a_reward_mint,
            token_b_reward_mint
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
                msg!("Init stake info");
                if vault_stake_info.key != &zero::id() {
                    pda::init_system_account(
                        admin_account,
                        vault_stake_info,
                        farm_program.key,
                        &vault.vault_program_id,
                        &[b"vault_stake_info", vault.name.as_bytes()],
                        RaydiumUserStakeInfo::LEN,
                    )?;
                } else {
                    pda::init_system_account(
                        admin_account,
                        vault_stake_info_v4,
                        farm_program.key,
                        &vault.vault_program_id,
                        &[b"vault_stake_info_v4", vault.name.as_bytes()],
                        RaydiumUserStakeInfoV4::LEN,
                    )?;
                }
            }

            if step == 0 || step == 2 {
                // init token accounts
                msg!("Init fees account a");
                pda::init_token_account(
                    admin_account,
                    fees_account_a,
                    token_a_reward_mint,
                    vault_authority,
                    rent_program,
                    &vault.vault_program_id,
                    &[b"fees_account_a", vault.name.as_bytes()],
                )?;

                if *fees_account_b.key != zero::id() {
                    msg!("Init fees account b");
                    pda::init_token_account(
                        admin_account,
                        fees_account_b,
                        token_b_reward_mint,
                        vault_authority,
                        rent_program,
                        &vault.vault_program_id,
                        &[b"fees_account_b", vault.name.as_bytes()],
                    )?;
                }

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

                msg!("Init token a custody account");
                pda::init_token_account(
                    admin_account,
                    token_a_custody,
                    token_a_mint,
                    vault_authority,
                    rent_program,
                    &vault.vault_program_id,
                    &[b"token_a_custody", vault.name.as_bytes()],
                )?;

                msg!("Init token b custody account");
                pda::init_token_account(
                    admin_account,
                    token_b_custody,
                    token_b_mint,
                    vault_authority,
                    rent_program,
                    &vault.vault_program_id,
                    &[b"token_b_custody", vault.name.as_bytes()],
                )?;

                msg!("Init token a reward custody account");
                pda::init_token_account(
                    admin_account,
                    token_a_reward_custody,
                    token_a_reward_mint,
                    vault_authority,
                    rent_program,
                    &vault.vault_program_id,
                    &[b"token_a_reward_custody", vault.name.as_bytes()],
                )?;

                if *token_b_reward_custody.key != zero::id() {
                    msg!("Init token b reward custody account");
                    pda::init_token_account(
                        admin_account,
                        token_b_reward_custody,
                        token_b_reward_mint,
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
