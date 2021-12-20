//! Remove Liquidity from the Vault instruction handler

use {
    crate::{traits::RemoveLiquidity, user_info::UserInfo, vault_info::VaultInfo},
    solana_farm_sdk::{
        instruction::vault::VaultInstruction,
        program::{account, protocol::saber},
        vault::{Vault, VaultStrategy},
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
        pubkey::Pubkey,
    },
};

impl RemoveLiquidity for VaultInstruction {
    fn remove_liquidity(vault: &Vault, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
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
            user_token_a_account,
            user_token_b_account,
            user_vt_token_account,
            lp_token_custody,
            pool_program_id,
            pool_token_a_account,
            pool_token_b_account,
            lp_token_mint,
            swap_account,
            swap_authority,
            fees_account_a,
            fees_account_b,
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
            if &account::get_token_account_owner(user_token_a_account)? != user_account.key
                || &account::get_token_account_owner(user_token_b_account)? != user_account.key
                || &account::get_token_account_owner(user_vt_token_account)? != user_account.key
            {
                msg!("Error: Invalid token account owner");
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

            let mut vault_info = VaultInfo::new(vault_info_account);
            if !vault_info.is_withdrawal_allowed()? {
                msg!("Error: Withdrawals are not allowed for this Vault");
                return Err(ProgramError::Custom(230));
            }

            // calculate amounts to unstake
            let vt_remove_amount = if amount > 0 {
                amount
            } else {
                account::get_token_balance(user_vt_token_account)?
            };
            let vt_supply_amount = account::get_token_supply(vault_token_mint)?;
            let stake_balance = saber::get_stake_account_balance(vault_stake_info)?;

            msg!(
                "Read balances. vt_remove_amount: {}, vt_supply_amount: {}, stake_balance: {}",
                vt_remove_amount,
                vt_supply_amount,
                stake_balance
            );
            if vt_remove_amount == 0 || vt_supply_amount == 0 || stake_balance == 0 {
                msg!("Error: Zero balance");
                return Err(ProgramError::InsufficientFunds);
            }
            let lp_remove_amount = account::to_token_amount(
                stake_balance as f64 * (vt_remove_amount as f64 / vt_supply_amount as f64),
                0,
            )?;

            // unstake
            let seeds: &[&[&[u8]]] = &[&[
                b"vault_authority",
                vault.name.as_bytes(),
                &[vault.authority_bump],
            ]];

            let initial_lp_tokens_balance = account::get_token_balance(lp_token_custody)?;

            msg!(
                "Unstake user's lp tokens. amount: {}, lp_remove_amount: {}",
                amount,
                lp_remove_amount
            );
            saber::unstake_with_seeds(
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
                lp_remove_amount,
            )?;
            let _ = account::check_tokens_received(
                lp_token_custody,
                initial_lp_tokens_balance,
                lp_remove_amount,
            )?;

            // brun vault tokens
            msg!(
                "Burn Vault tokens from the user. vt_remove_amount: {}",
                vt_remove_amount
            );
            let key = Pubkey::create_program_address(
                &[
                    b"vault_token_mint",
                    vault.name.as_bytes(),
                    &[vault.vault_token_bump],
                ],
                &vault.vault_program_id,
            )?;
            if vault_token_mint.key != &key {
                msg!("Error: Invalid Vault token mint");
                return Err(ProgramError::InvalidSeeds);
            }
            account::burn_tokens(
                user_vt_token_account,
                vault_token_mint,
                user_account,
                vt_remove_amount,
            )?;

            // remove liquidity from the pool
            let initial_token_a_user_balance = account::get_token_balance(user_token_a_account)?;
            let initial_token_b_user_balance = account::get_token_balance(user_token_b_account)?;

            msg!(
                "Remove liquidity from the pool. lp_remove_amount: {}",
                lp_remove_amount
            );
            saber::remove_liquidity_with_seeds(
                &[
                    vault_authority.clone(),
                    user_token_a_account.clone(),
                    user_token_b_account.clone(),
                    lp_token_custody.clone(),
                    pool_program_id.clone(),
                    pool_token_a_account.clone(),
                    pool_token_b_account.clone(),
                    lp_token_mint.clone(),
                    spl_token_program.clone(),
                    swap_account.clone(),
                    swap_authority.clone(),
                    fees_account_a.clone(),
                    fees_account_b.clone(),
                ],
                seeds,
                lp_remove_amount,
            )?;

            // check tokens received
            let tokens_a_received =
                account::get_balance_increase(user_token_a_account, initial_token_a_user_balance)?;
            let tokens_b_received =
                account::get_balance_increase(user_token_b_account, initial_token_b_user_balance)?;
            if tokens_a_received == 0 && tokens_b_received == 0 {
                msg!("Error: Remove liquidity instruction didn't result in any of the tokens received");
                return Err(ProgramError::Custom(190));
            }
            if initial_lp_tokens_balance != account::get_token_balance(lp_token_custody)? {
                msg!(
                    "Error: Remove liquidity instruction didn't result in expected amount of LP tokens spent"
                );
                return Err(ProgramError::Custom(165));
            }

            // send tokens to the user
            msg!(
                "Update stats. tokens_a_received: {}, tokens_b_received: {}",
                tokens_a_received,
                tokens_b_received
            );

            // update user stats
            msg!("Update user stats");
            let mut user_info = UserInfo::new(user_info_account);
            user_info.remove_liquidity(tokens_a_received, tokens_b_received)?;

            // update vault stats
            msg!("Update Vault stats");
            vault_info.remove_liquidity(tokens_a_received, tokens_b_received)?;

            Ok(())
        } else {
            Err(ProgramError::NotEnoughAccountKeys)
        }
    }
}
