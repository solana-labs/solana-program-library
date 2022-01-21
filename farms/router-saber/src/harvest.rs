//! Harvest rewards from a Saber farm instruction

use {
    solana_farm_sdk::{id::zero, program::account},
    solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        hash::Hasher,
        instruction::{AccountMeta, Instruction},
        msg,
        program::invoke,
        program_error::ProgramError,
    },
};

pub fn harvest(accounts: &[AccountInfo]) -> ProgramResult {
    msg!("Processing AmmInstruction::Harvest");

    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        user_account,
        user_iou_token_account,
        user_sbr_token_account,
        farm_program_id,
        _spl_token_id,
        _zero_id,
        miner,
        rewarder,
        redeemer,
        redeemer_program,
        minter,
        mint_wrapper,
        mint_wrapper_program,
        sbr_token_mint,
        iou_token_mint,
        iou_fees_account,
        quarry,
        saber_vault,
        saber_mint_proxy_program,
        mint_proxy_authority,
        mint_proxy_state,
        minter_info
        ] = accounts
    {
        if &quarry_mine::id() != farm_program_id.key
            || &quarry_mint_wrapper::id() != mint_wrapper_program.key
        {
            return Err(ProgramError::IncorrectProgramId);
        }

        let initial_iou_token_user_balance = account::get_token_balance(user_iou_token_account)?;
        let initial_sbr_token_user_balance = account::get_token_balance(user_sbr_token_account)?;

        // harvest IOU rewards
        let mut hasher = Hasher::default();
        hasher.hash(b"global:claim_rewards");

        let data = hasher.result().as_ref()[..8].to_vec();

        let saber_accounts = vec![
            AccountMeta::new(*mint_wrapper.key, false),
            AccountMeta::new_readonly(*mint_wrapper_program.key, false),
            AccountMeta::new(*minter.key, false),
            AccountMeta::new(*iou_token_mint.key, false),
            AccountMeta::new(*user_iou_token_account.key, false),
            AccountMeta::new(*iou_fees_account.key, false),
            AccountMeta::new_readonly(*user_account.key, true),
            AccountMeta::new(*miner.key, false),
            AccountMeta::new(*quarry.key, false),
            AccountMeta::new(zero::id(), false),
            AccountMeta::new(zero::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(*rewarder.key, false),
        ];

        let instruction = Instruction {
            program_id: quarry_mine::id(),
            accounts: saber_accounts,
            data,
        };

        invoke(&instruction, accounts)?;

        let iou_rewards =
            account::get_balance_increase(user_iou_token_account, initial_iou_token_user_balance)?;

        if iou_rewards == 0 {
            return Ok(());
        }

        // convert IOU to Saber
        let mut hasher = Hasher::default();
        hasher.hash(b"global:redeem_all_tokens_from_mint_proxy");

        let data = hasher.result().as_ref()[..8].to_vec();

        let saber_accounts = vec![
            AccountMeta::new_readonly(*redeemer.key, false),
            AccountMeta::new(*iou_token_mint.key, false),
            AccountMeta::new(*sbr_token_mint.key, false),
            AccountMeta::new(*saber_vault.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(*user_account.key, true),
            AccountMeta::new(*user_iou_token_account.key, false),
            AccountMeta::new(*user_sbr_token_account.key, false),
            AccountMeta::new_readonly(*mint_proxy_authority.key, false),
            AccountMeta::new_readonly(*mint_proxy_state.key, false),
            AccountMeta::new_readonly(*saber_mint_proxy_program.key, false),
            AccountMeta::new(*minter_info.key, false),
        ];

        let instruction = Instruction {
            program_id: *redeemer_program.key,
            accounts: saber_accounts,
            data,
        };

        invoke(&instruction, accounts)?;

        account::check_tokens_received(
            user_sbr_token_account,
            initial_sbr_token_user_balance,
            iou_rewards,
        )?;
    } else {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    msg!("AmmInstruction::Harvest complete");
    Ok(())
}
