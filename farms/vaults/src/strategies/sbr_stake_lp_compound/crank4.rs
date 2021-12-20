//! Crank step 4 instruction handler

use {
    crate::{clock::check_min_crank_interval, vault_info::VaultInfo},
    solana_farm_sdk::{
        program::{account, protocol::saber},
        vault::{Vault, VaultStrategy},
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

pub fn crank4(vault: &Vault, accounts: &[AccountInfo]) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        _funding_account,
        _vault_metadata,
        vault_info_account,
        vault_authority,
        spl_token_program,
        token_a_custody,
        token_b_custody,
        lp_token_custody,
        pool_program_id,
        pool_token_a_account,
        pool_token_b_account,
        lp_token_mint,
        clock_program,
        swap_account,
        swap_authority
        ] = accounts
    {
        // validate accounts
        if vault_authority.key != &vault.vault_authority {
            msg!("Error: Invalid Vault accounts");
            return Err(ProgramError::InvalidArgument);
        }
        if let VaultStrategy::StakeLpCompoundRewards {
            token_a_custody: token_a_custody_key,
            token_b_custody: token_b_custody_key,
            lp_token_custody: lp_token_custody_key,
            ..
        } = vault.strategy
        {
            if vault.fees_account_b.is_none()
                || (token_a_custody.key != &token_a_custody_key
                    && (token_b_custody_key.is_none()
                        || token_a_custody.key != &token_b_custody_key.unwrap())
                    && token_a_custody.key != &vault.fees_account_b.unwrap())
                || (token_b_custody.key != &token_a_custody_key
                    && (token_b_custody_key.is_none()
                        || token_b_custody.key != &token_b_custody_key.unwrap())
                    && token_b_custody.key != &vault.fees_account_b.unwrap())
                || &lp_token_custody_key != lp_token_custody.key
            {
                msg!("Error: Invalid custody accounts");
                return Err(ProgramError::InvalidArgument);
            }
        } else {
            msg!("Error: Vault strategy mismatch");
            return Err(ProgramError::InvalidArgument);
        }

        let mut vault_info = VaultInfo::new(vault_info_account);
        check_min_crank_interval(&vault_info)?;

        // read balances
        let token_a_balance = account::get_token_balance(token_a_custody)?;
        let token_b_balance = account::get_token_balance(token_b_custody)?;
        let lp_token_balance = account::get_token_balance(lp_token_custody)?;
        msg!(
            "Read balances. token_a_balance: {}, token_b_balance: {}",
            token_a_balance,
            token_b_balance
        );
        if token_a_balance < 10 && token_b_balance < 10 {
            msg!("Nothing to do: Not enough tokens to deposit");
            return Ok(());
        }

        let seeds: &[&[&[u8]]] = &[&[
            b"vault_authority",
            vault.name.as_bytes(),
            &[vault.authority_bump],
        ]];

        msg!("Deposit tokens into the pool");
        saber::add_liquidity_with_seeds(
            &[
                vault_authority.clone(),
                token_a_custody.clone(),
                token_b_custody.clone(),
                lp_token_custody.clone(),
                pool_program_id.clone(),
                pool_token_a_account.clone(),
                pool_token_b_account.clone(),
                lp_token_mint.clone(),
                spl_token_program.clone(),
                clock_program.clone(),
                swap_account.clone(),
                swap_authority.clone(),
            ],
            seeds,
            token_a_balance,
            token_b_balance,
        )?;

        // check amounts spent and received
        let tokens_a_spent =
            account::check_tokens_spent(token_a_custody, token_a_balance, token_a_balance)?;
        let tokens_b_spent =
            account::check_tokens_spent(token_b_custody, token_b_balance, token_b_balance)?;
        let lp_tokens_received =
            account::check_tokens_received(lp_token_custody, lp_token_balance, 1)?;

        // update Vault stats
        msg!(
            "Update Vault stats. tokens_a_spent {}, tokens_b_spent {}, lp_tokens_received {}",
            tokens_a_spent,
            tokens_b_spent,
            lp_tokens_received
        );
        vault_info.add_liquidity(tokens_a_spent, tokens_b_spent)?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
