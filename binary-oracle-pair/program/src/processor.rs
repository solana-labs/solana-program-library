//! Program state processor

use crate::{error::PoolError, instruction::PoolInstruction, state::Pool, state::POOL_VERSION};
use borsh::BorshDeserialize;
use solana_program::{
    account_info::next_account_info,
    account_info::AccountInfo,
    clock::Slot,
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};
use spl_token::state::Mint;

/// Program state handler.
pub struct Processor {}
impl Processor {
    /// Calculates the authority id by generating a program address.
    pub fn authority_id(
        program_id: &Pubkey,
        my_info: &Pubkey,
        bump_seed: u8,
    ) -> Result<Pubkey, ProgramError> {
        Pubkey::create_program_address(&[&my_info.to_bytes()[..32], &[bump_seed]], program_id)
            .or(Err(PoolError::InvalidAuthorityData.into()))
    }

    /// Initialize the pool
    pub fn process_init_pool(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        mint_end_slot: Slot,
        decide_end_slot: Slot,
        bump_seed: u8,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_account_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let funding_account_info = next_account_info(account_info_iter)?;
        let decider_info = next_account_info(account_info_iter)?;
        let deposit_token_mint_info = next_account_info(account_info_iter)?;
        // uninitialized
        let deposit_account_info = next_account_info(account_info_iter)?;
        // uninitialized
        let token_pass_mint_info = next_account_info(account_info_iter)?;
        // uninitialized
        let token_fail_mint_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let mut pool = Pool::unpack(&pool_account_info.data.borrow())?;
        // Pool account should not be already initialized
        if pool.is_initialized() {
            return Err(PoolError::AlreadyInUse.into());
        }

        // Check if pool account is rent-exempt
        if !rent.is_exempt(pool_account_info.lamports(), pool_account_info.data_len()) {
            return Err(PoolError::NotRentExempt.into());
        }

        // Check if deposit token's mint owner is token program
        if deposit_token_mint_info.owner != token_program_info.key {
            return Err(PoolError::InvalidTokenMint.into());
        }

        // Check if deposit token mint is initialized
        let deposit_token_mint = Mint::unpack(&deposit_token_mint_info.data.borrow())?;
        if !deposit_token_mint.is_initialized() {
            return Err(PoolError::UninitializedTokenMint.into());
        }

        let authority = Self::authority_id(program_id, pool_account_info.key, bump_seed)?;
        if &authority != authority_info.key {
            return Err(PoolError::InvalidAuthorityAccount.into());
        }

        let account_rent = rent.minimum_balance(spl_token::state::Account::LEN);
        // Create and initialize deposit token account
        invoke(
            &system_instruction::create_account(
                funding_account_info.key,
                deposit_account_info.key,
                account_rent,
                spl_token::state::Account::LEN as u64,
                token_program_info.key,
            ),
            &[funding_account_info.clone(), deposit_account_info.clone()],
        )?;

        invoke(
            &spl_token::instruction::initialize_account(
                token_program_info.key,
                deposit_account_info.key,
                deposit_token_mint_info.key,
                authority_info.key,
            )
            .unwrap(),
            &[],
        )?;

        let mint_rent = rent.minimum_balance(spl_token::state::Mint::LEN);
        // Create and initialize token pass mint
        invoke(
            &system_instruction::create_account(
                funding_account_info.key,
                token_pass_mint_info.key,
                mint_rent,
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            &[funding_account_info.clone(), token_pass_mint_info.clone()],
        )?;

        invoke(
            &spl_token::instruction::initialize_mint(
                &spl_token::id(),
                token_pass_mint_info.key,
                authority_info.key,
                None,
                0, // TODO: create constant
            )
            .unwrap(),
            &[],
        )?;

        // Create and initialize token fail mint
        invoke(
            &system_instruction::create_account(
                funding_account_info.key,
                token_fail_mint_info.key,
                mint_rent,
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            &[funding_account_info.clone(), token_fail_mint_info.clone()],
        )?;

        invoke(
            &spl_token::instruction::initialize_mint(
                &spl_token::id(),
                token_fail_mint_info.key,
                authority_info.key,
                None,
                0, // TODO: create constant
            )
            .unwrap(),
            &[],
        )?;

        pool.version = POOL_VERSION;
        pool.bump_seed = bump_seed;
        pool.token_program_id = *token_program_info.key;
        pool.deposit_account = *deposit_account_info.key;
        pool.token_pass_mint = *token_pass_mint_info.key;
        pool.token_fail_mint = *token_fail_mint_info.key;
        pool.decider = *decider_info.key;
        pool.mint_end_slot = mint_end_slot;
        pool.decide_end_slot = decide_end_slot;
        pool.decision = None;

        Pool::pack(pool, &mut pool_account_info.data.borrow_mut())
    }
    /// Processes an instruction
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: &[u8],
    ) -> ProgramResult {
        let instruction =
            PoolInstruction::try_from_slice(input).or(Err(PoolError::InstructionUnpackError))?;
        match instruction {
            PoolInstruction::InitPool(init_args) => {
                msg!("Instruction: InitPool");
                Self::process_init_pool(
                    program_id,
                    accounts,
                    init_args.mint_end_slot,
                    init_args.decide_end_slot,
                    init_args.bump_seed,
                )
            }
            _ => unimplemented!(),
        }
    }
}
