//! Instruction types

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    clock::Slot,
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar,
};

/// Initialize arguments for pool
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct InitArgs {
    /// mint end slot
    pub mint_end_slot: Slot,
    /// decide end slot
    pub decide_end_slot: Slot,
    /// authority nonce
    pub bump_seed: u8,
}

/// Instruction definition
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub enum PoolInstruction {
    /// Initializes a new binary oracle pair pool.
    ///
    ///   0. `[w]` Pool account.
    ///   1. `[]` Authority
    ///   2. `[]` Decider authority
    ///   3. `[]` Deposit currency SPL Token mint. Must be initialized.
    ///   4. `[w]` Deposit token account. Should not be initialized
    ///   5. `[w]` Token Pass mint. Should not be initialized
    ///   6. `[w]` Token Fail mint. Should not be initialized
    ///   7. `[]` Rent sysvar
    ///   8. `[]` Token program id
    InitPool(InitArgs),

    ///   Deposit into the pool.
    ///
    ///   0. `[]` Pool
    ///   1. `[]` Authority
    ///   2. `[s]` User transfer authority
    ///   3. `[w]` Token SOURCE Account, amount is transferable by pool authority with allowances.
    ///   4. `[w]` Deposit token account
    ///   5. `[w]` token_P PASS mint
    ///   6. `[w]` token_F FAIL mint
    ///   7. `[w]` token_P DESTINATION Account
    ///   8. `[w]` token_F DESTINATION Account
    ///   9. `[]` Sysvar Clock
    ///   10. `[]` Token program id
    Deposit(u64),

    ///   Withdraw from the pool.
    ///   If current slot is < mint_end slot, 1 Pass AND 1 Fail token convert to 1 deposit
    ///   If current slot is > decide_end_slot slot && decide == Some(true), 1 Pass convert to 1 deposit
    ///   otherwise 1 Fail converts to 1 deposit
    ///
    ///   Pass tokens convert 1:1 to the deposit token iff decision is set to Some(true)
    ///   AND current slot is > decide_end_slot.
    ///
    ///   0. `[]` Pool
    ///   1. `[]` Authority
    ///   2. `[s]` User transfer authority
    ///   3. `[w]` Pool deposit token account
    ///   4. `[w]` token_P PASS SOURCE Account
    ///   5. `[w]` token_F FAIL SOURCE Account
    ///   6. `[w]` token_P PASS mint
    ///   7. `[w]` token_F FAIL mint
    ///   8. `[w]` Deposit DESTINATION Account
    ///   9. `[]` Sysvar Clock
    ///   10. `[]` Token program id
    Withdraw(u64),

    ///  Trigger the decision.
    ///  Call only succeeds once and if current slot > mint_end slot AND < decide_end slot
    ///   0. `[]` Pool
    ///   1. `[s]` Decider pubkey
    ///   2. `[]` Sysvar Clock
    Decide(bool),
}

/// Create `InitPool` instruction
#[allow(clippy::too_many_arguments)]
pub fn init_pool(
    program_id: &Pubkey,
    pool: &Pubkey,
    authority: &Pubkey,
    decider: &Pubkey,
    deposit_token_mint: &Pubkey,
    deposit_account: &Pubkey,
    token_pass_mint: &Pubkey,
    token_fail_mint: &Pubkey,
    token_program_id: &Pubkey,
    init_args: InitArgs,
) -> Result<Instruction, ProgramError> {
    let init_data = PoolInstruction::InitPool(init_args);
    let data = init_data.try_to_vec()?;
    let accounts = vec![
        AccountMeta::new(*pool, false),
        AccountMeta::new_readonly(*authority, false),
        AccountMeta::new_readonly(*decider, false),
        AccountMeta::new_readonly(*deposit_token_mint, false),
        AccountMeta::new(*deposit_account, false),
        AccountMeta::new(*token_pass_mint, false),
        AccountMeta::new(*token_fail_mint, false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(*token_program_id, false),
    ];
    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Create `Deposit` instruction
#[allow(clippy::too_many_arguments)]
pub fn deposit(
    program_id: &Pubkey,
    pool: &Pubkey,
    authority: &Pubkey,
    user_transfer_authority: &Pubkey,
    user_token_account: &Pubkey,
    pool_deposit_token_account: &Pubkey,
    token_pass_mint: &Pubkey,
    token_fail_mint: &Pubkey,
    token_pass_destination_account: &Pubkey,
    token_fail_destination_account: &Pubkey,
    token_program_id: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let init_data = PoolInstruction::Deposit(amount);
    let data = init_data.try_to_vec()?;

    let accounts = vec![
        AccountMeta::new_readonly(*pool, false),
        AccountMeta::new_readonly(*authority, false),
        AccountMeta::new_readonly(
            *user_transfer_authority,
            authority != user_transfer_authority,
        ),
        AccountMeta::new(*user_token_account, false),
        AccountMeta::new(*pool_deposit_token_account, false),
        AccountMeta::new(*token_pass_mint, false),
        AccountMeta::new(*token_fail_mint, false),
        AccountMeta::new(*token_pass_destination_account, false),
        AccountMeta::new(*token_fail_destination_account, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(*token_program_id, false),
    ];
    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Create `Withdraw` instruction
#[allow(clippy::too_many_arguments)]
pub fn withdraw(
    program_id: &Pubkey,
    pool: &Pubkey,
    authority: &Pubkey,
    user_transfer_authority: &Pubkey,
    pool_deposit_token_account: &Pubkey,
    token_pass_user_account: &Pubkey,
    token_fail_user_account: &Pubkey,
    token_pass_mint: &Pubkey,
    token_fail_mint: &Pubkey,
    user_token_destination_account: &Pubkey,
    token_program_id: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let init_data = PoolInstruction::Withdraw(amount);
    let data = init_data.try_to_vec()?;
    let accounts = vec![
        AccountMeta::new_readonly(*pool, false),
        AccountMeta::new_readonly(*authority, false),
        AccountMeta::new_readonly(
            *user_transfer_authority,
            authority != user_transfer_authority,
        ),
        AccountMeta::new(*pool_deposit_token_account, false),
        AccountMeta::new(*token_pass_user_account, false),
        AccountMeta::new(*token_fail_user_account, false),
        AccountMeta::new(*token_pass_mint, false),
        AccountMeta::new(*token_fail_mint, false),
        AccountMeta::new(*user_token_destination_account, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(*token_program_id, false),
    ];
    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Create `Decide` instruction
pub fn decide(
    program_id: &Pubkey,
    pool: &Pubkey,
    decider: &Pubkey,
    decision: bool,
) -> Result<Instruction, ProgramError> {
    let init_data = PoolInstruction::Decide(decision);
    let data = init_data.try_to_vec()?;
    let accounts = vec![
        AccountMeta::new(*pool, false),
        AccountMeta::new_readonly(*decider, true),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
    ];
    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}
