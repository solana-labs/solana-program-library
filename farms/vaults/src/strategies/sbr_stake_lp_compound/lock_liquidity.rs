//! Lock Liquidity in the Vault instruction handler

use {
    crate::{traits::LockLiquidity, user_info::UserInfo, vault_info::VaultInfo},
    solana_farm_sdk::{
        instruction::vault::VaultInstruction,
        program::{account, pda, protocol::saber},
        vault::{Vault, VaultStrategy},
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

impl LockLiquidity for VaultInstruction {
    fn lock_liquidity(vault: &Vault, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
        #[allow(clippy::deprecated_cfg_attr)]
        #[cfg_attr(rustfmt, rustfmt_skip)]
        if let [
            user_account,
            _vault_metadata,
            vault_info_account,
            vault_authority,
            spl_token_program,
            vault_token_mint,
            user_info_account,
            user_vt_token_account,
            lp_token_custody,
            farm_program,
            vault_stake_info,
            vault_miner_account,
            quarry,
            rewarder
            ] = accounts
        {
            // validate accounts
            if vault_authority.key != &vault.vault_authority
                || &account::get_token_account_owner(vault_miner_account)? != vault_stake_info.key
            {
                msg!("Error: Invalid Vault accounts");
                return Err(ProgramError::InvalidArgument);
            }
            if !user_account.is_signer {
                return Err(ProgramError::MissingRequiredSignature);
            }
            if &account::get_token_account_owner(user_vt_token_account)? != user_account.key {
                msg!("Error: Invalid VT token account owner");
                return Err(ProgramError::IllegalOwner);
            }

            if let VaultStrategy::StakeLpCompoundRewards {
                lp_token_custody: lp_token_custody_key,
                vault_stake_info: vault_stake_info_key,
                ..
            } = vault.strategy
            {
                if &vault_stake_info_key != vault_stake_info.key {
                    msg!("Error: Invalid Vault Stake Info account");
                    return Err(ProgramError::InvalidArgument);
                }
                if &lp_token_custody_key != lp_token_custody.key {
                    msg!("Error: Invalid custody accounts");
                    return Err(ProgramError::InvalidArgument);
                }
            } else {
                msg!("Error: Vault strategy mismatch");
                return Err(ProgramError::InvalidArgument);
            }

            if !UserInfo::validate_account(vault, user_info_account, user_account.key) {
                msg!("Error: Invalid user info account");
                return Err(ProgramError::Custom(140));
            }

            let vault_info = VaultInfo::new(vault_info_account);
            if !vault_info.is_deposit_allowed()? {
                msg!("Error: Deposits are not allowed for this Vault");
                return Err(ProgramError::Custom(220));
            }

            // check lp balance
            let mut user_info = UserInfo::new(user_info_account);
            let lp_tokens_debt = user_info.get_lp_tokens_debt()?;
            msg!("Read balances. lp_tokens_debt: {}", lp_tokens_debt);

            let lp_stake_amount = if amount > 0 {
                if lp_tokens_debt < amount {
                    msg!("Error: Insufficient funds");
                    return Err(ProgramError::InsufficientFunds);
                }
                amount
            } else {
                lp_tokens_debt
            };
            if lp_stake_amount == 0 {
                msg!("Error: Zero balance. Forgot to deposit funds?");
                return Err(ProgramError::InsufficientFunds);
            }

            let initial_lp_custody_balance = account::get_token_balance(lp_token_custody)?;

            // Stake LP tokens
            let seeds: &[&[&[u8]]] = &[&[
                b"vault_authority",
                vault.name.as_bytes(),
                &[vault.authority_bump],
            ]];

            msg!("Stake LP tokens. lp_stake_amount: {}", lp_stake_amount);
            let stake_balance = saber::get_stake_account_balance(vault_stake_info)?;

            saber::stake_with_seeds(
                &[
                    vault_authority.clone(),
                    lp_token_custody.clone(),
                    farm_program.clone(),
                    spl_token_program.clone(),
                    vault_stake_info.clone(),
                    vault_miner_account.clone(),
                    quarry.clone(),
                    rewarder.clone(),
                ],
                seeds,
                lp_stake_amount,
            )?;
            let _ = account::check_tokens_spent(
                lp_token_custody,
                initial_lp_custody_balance,
                lp_stake_amount,
            )?;

            // update user stats
            msg!("Update user stats");
            user_info.remove_lp_tokens_debt(lp_stake_amount)?;

            // compute Vault tokens to mint
            let vt_supply_amount = account::get_token_supply(vault_token_mint)?;
            let vt_to_mint = if vt_supply_amount == 0 || stake_balance == 0 {
                lp_stake_amount
            } else {
                account::to_token_amount(
                    lp_stake_amount as f64 / stake_balance as f64 * vt_supply_amount as f64,
                    0,
                )?
            };

            // mint vault tokens to user
            msg!(
                "Mint Vault tokens to the user. vt_to_mint: {}, vt_supply_amount: {}, stake_balance: {}",
                vt_to_mint, vt_supply_amount,
                stake_balance
            );
            if vt_to_mint == 0 {
                msg!("Error: Add liquidity instruction didn't result in Vault tokens mint");
                return Err(ProgramError::Custom(170));
            }
            pda::mint_to_with_seeds(
                user_vt_token_account,
                vault_token_mint,
                vault_authority,
                seeds,
                vt_to_mint,
            )?;

            Ok(())
        } else {
            Err(ProgramError::NotEnoughAccountKeys)
        }
    }
}
