//! Raydium router implementation.

use {
    crate::{
        add_liquidity::add_liquidity, remove_liquidity::remove_liquidity, stake::stake, swap::swap,
        unstake::unstake,
    },
    solana_farm_sdk::{instruction::amm::AmmInstruction, log::sol_log_params_short},
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, log::sol_log_compute_units, msg,
        pubkey::Pubkey,
    },
};

/// Program's entrypoint.
///
/// # Arguments
/// * `program_id` - Public key of the router.
/// * `accounts` - Accounts, see particular instruction handler for the list.
/// * `instructions_data` - Packed AmmInstruction.
pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("Raydium router entrypoint");
    if cfg!(feature = "debug") {
        sol_log_params_short(accounts, instruction_data);
    }

    // Read and unpack instruction data
    let instruction = AmmInstruction::unpack(instruction_data)?;

    match instruction {
        AmmInstruction::AddLiquidity {
            max_token_a_amount,
            max_token_b_amount,
        } => add_liquidity(accounts, max_token_a_amount, max_token_b_amount)?,
        AmmInstruction::RemoveLiquidity { amount } => remove_liquidity(accounts, amount)?,
        AmmInstruction::Swap {
            token_a_amount_in,
            token_b_amount_in,
            min_token_amount_out,
        } => swap(
            accounts,
            token_a_amount_in,
            token_b_amount_in,
            min_token_amount_out,
        )?,
        AmmInstruction::Stake { amount } => stake(accounts, amount, false)?,
        AmmInstruction::Unstake { amount } => unstake(accounts, amount)?,
        AmmInstruction::Harvest => stake(accounts, 0, true)?,
        _ => {}
    }

    sol_log_compute_units();
    msg!("Raydium router end of instruction");
    Ok(())
}
