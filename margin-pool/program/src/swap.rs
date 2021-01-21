use crate::error::MarginPoolError;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program::invoke_signed,
};

/// Issue a spl_token_swap `Swap` instruction.
#[inline(always)]
pub fn spl_token_swap_swap<'a>(
    swap_program: AccountInfo<'a>,
    token_program: AccountInfo<'a>,
    pool: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    user_transfer: AccountInfo<'a>,
    source: AccountInfo<'a>,
    swap_source: AccountInfo<'a>,
    swap_destination: AccountInfo<'a>,
    destination: AccountInfo<'a>,
    pool_mint: AccountInfo<'a>,
    pool_fee: AccountInfo<'a>,
    host_fee: AccountInfo<'a>,

    amount_in: u64,
    minimum_amount_out: u64,
) -> ProgramResult {
    let result = invoke_signed(
        &spl_token_swap::instruction::swap(
            swap_program.key,
            token_program.key,
            pool.key,
            authority.key,
            // user_transfer.key,
            source.key,
            swap_source.key,
            swap_destination.key,
            destination.key,
            pool_mint.key,
            pool_fee.key,
            Some(host_fee.key),
            spl_token_swap::instruction::Swap {
                amount_in,
                minimum_amount_out,
            },
        )?,
        // TODO: check accounts ...
        &[source, token_program, swap_program],
        &[],
    );
    result.map_err(|_| MarginPoolError::SwapFaild.into())
}

/// Issue a withdraw_single_token_type_exact_amount_out `Swap` instruction.
#[inline(always)]
pub fn spl_token_swap_withdraw_single<'a>(
    swap_program: AccountInfo<'a>,
    token_program: AccountInfo<'a>,
    pool: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    user_transfer: AccountInfo<'a>,
    source: AccountInfo<'a>,
    swap_source: AccountInfo<'a>,
    swap_destination: AccountInfo<'a>,
    destination: AccountInfo<'a>,
    pool_mint: AccountInfo<'a>,
    pool_fee: AccountInfo<'a>,
    host_fee: AccountInfo<'a>,

    destination_token_amount: u64,
    maximum_pool_token_amount: u64,
) -> ProgramResult {
    let result = invoke_signed(
        &spl_token_swap::instruction::withdraw_single_token_type_exact_amount_out(
            swap_program.key,
            token_program.key,
            pool.key,
            authority.key,
            pool_mint.key,
            pool_fee.key,
            // user_transfer.key,
            source.key,
            swap_source.key,
            swap_destination.key,
            destination.key,
            spl_token_swap::instruction::WithdrawSingleTokenTypeExactAmountOut {
                destination_token_amount: destination_token_amount,
                maximum_pool_token_amount: maximum_pool_token_amount,
            },
        )?,
        // TODO: check accounts ...
        &[source, token_program, swap_program],
        &[],
    );
    result.map_err(|_| MarginPoolError::SwapFaild.into())
}
