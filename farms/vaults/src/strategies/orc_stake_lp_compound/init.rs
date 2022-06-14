//! Vault Init instruction handler

use {
    crate::{traits::Init, vault_info::VaultInfo},
    solana_farm_sdk::{
        instruction::{orca::OrcaUserInit, vault::VaultInstruction},
        program::{account, pda, protocol::orca::OrcaUserStakeInfo},
        token::Token,
        traits::Packed,
        vault::Vault,
    },
    solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        instruction::{AccountMeta, Instruction},
        msg,
        program::invoke_signed,
        program_error::ProgramError,
        system_program, sysvar,
        sysvar::Sysvar,
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
            _multisig_account,
            vault_authority,
            vault_program,
            _system_program,
            _spl_token_program,
            rent_program,
            farm_program,
            vault_token_mint,
            vault_token_ref,
            vault_stake_info,
            vault_stake_custody,
            fees_account,
            token_a_custody,
            token_b_custody,
            lp_token_custody,
            token_a_mint,
            token_b_mint,
            lp_token_mint,
            farm_lp_token_mint,
            reward_token_custody,
            reward_token_mint,
            farm_id
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
                if account::is_empty(vault_stake_info)? {
                    msg!("Init stake info");
                    let min_balance = sysvar::rent::Rent::get().unwrap().minimum_balance(OrcaUserStakeInfo::LEN);
                    account::transfer_sol(admin_account, vault_authority, min_balance)?;

                    let seeds: &[&[&[u8]]] = &[&[
                        b"vault_authority",
                        vault.name.as_bytes(),
                        &[vault.authority_bump],
                    ]];
                    let orca_accounts = vec![
                        AccountMeta::new_readonly(*farm_id.key, false),
                        AccountMeta::new(*vault_stake_info.key, false),
                        AccountMeta::new(*vault_authority.key, true),
                        AccountMeta::new_readonly(system_program::id(), false),
                    ];

                    let instruction = Instruction {
                        program_id: *farm_program.key,
                        accounts: orca_accounts,
                        data: OrcaUserInit {}.to_vec()?,
                    };

                    invoke_signed(&instruction, accounts, seeds)?;
                }
            }

            if step == 0 || step == 2 {
                // init token accounts
                msg!("Init fees account");
                pda::init_token_account(
                    admin_account,
                    fees_account,
                    reward_token_mint,
                    vault_authority,
                    rent_program,
                    &vault.vault_program_id,
                    &[b"fees_account", vault.name.as_bytes()],
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

                msg!("Init reward token custody account");
                pda::init_token_account(
                    admin_account,
                    reward_token_custody,
                    reward_token_mint,
                    vault_authority,
                    rent_program,
                    &vault.vault_program_id,
                    &[b"reward_token_custody", vault.name.as_bytes()],
                )?;

                msg!("Init vault stake custody");
                pda::init_token_account(
                    admin_account,
                    vault_stake_custody,
                    farm_lp_token_mint,
                    vault_authority,
                    rent_program,
                    &vault.vault_program_id,
                    &[b"vault_stake_custody", vault.name.as_bytes()],
                )?;
            }

            Ok(())
        } else {
            Err(ProgramError::NotEnoughAccountKeys)
        }
    }
}
