//! Program state processor

use crate::{
    error::PoolError,
    instruction::PoolInstruction,
    state::{Pool, POOL_VERSION},
};
use borsh::BorshDeserialize;
use solana_program::{
    account_info::next_account_info,
    account_info::AccountInfo,
    clock::Slot,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};
use spl_token::state::{Account, Mint};

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

    /// Transfer tokens
    pub fn transfer<'a>(
        pool_pub_key: &Pubkey,
        bump_seed: u8,
        destination_account: AccountInfo<'a>,
        authority_account: AccountInfo<'a>,
        token_program_id: AccountInfo<'a>,
        source_account: AccountInfo<'a>,
        amount: u64,
    ) -> ProgramResult {
        let me_bytes = pool_pub_key.to_bytes();
        let authority_signature_seeds = [&me_bytes[..32], &[bump_seed]];
        let signers = &[&authority_signature_seeds[..]];

        invoke_signed(
            &spl_token::instruction::transfer(
                token_program_id.key,
                source_account.key,
                destination_account.key,
                authority_account.key,
                &[authority_account.key],
                amount,
            )
            .unwrap(),
            &[
                token_program_id,
                authority_account,
                source_account,
                destination_account,
            ],
            signers,
        )
    }

    /// Mint tokens
    pub fn mint<'a>(
        pool_pub_key: &Pubkey,
        bump_seed: u8,
        token_program_id: AccountInfo<'a>,
        mint_account: AccountInfo<'a>,
        destination_account: AccountInfo<'a>,
        authority_account: AccountInfo<'a>,
        amount: u64,
    ) -> ProgramResult {
        let me_bytes = pool_pub_key.to_bytes();
        let authority_signature_seeds = [&me_bytes[..32], &[bump_seed]];
        let signers = &[&authority_signature_seeds[..]];

        invoke_signed(
            &spl_token::instruction::mint_to(
                token_program_id.key,
                mint_account.key,
                destination_account.key,
                authority_account.key,
                &[authority_account.key],
                amount,
            )
            .unwrap(),
            &[
                token_program_id,
                mint_account,
                destination_account,
                authority_account,
            ],
            signers,
        )
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
        let decider_info = next_account_info(account_info_iter)?;
        let deposit_token_mint_info = next_account_info(account_info_iter)?;
        let deposit_account_info = next_account_info(account_info_iter)?;
        let token_pass_mint_info = next_account_info(account_info_iter)?;
        let token_fail_mint_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let mut pool = Pool::unpack_unchecked(&pool_account_info.data.borrow())?;
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

        // Check if bump seed is correct
        let authority = Self::authority_id(program_id, pool_account_info.key, bump_seed)?;
        if &authority != authority_info.key {
            return Err(PoolError::InvalidAuthorityAccount.into());
        }

        let deposit_account = Account::unpack_unchecked(&deposit_account_info.data.borrow())?;
        if deposit_account.is_initialized() {
            return Err(PoolError::DepositAccountInUse.into());
        }

        let token_pass = Mint::unpack_unchecked(&token_pass_mint_info.data.borrow())?;
        if token_pass.is_initialized() {
            return Err(PoolError::TokenMintInUse.into());
        }

        let token_fail = Mint::unpack_unchecked(&token_fail_mint_info.data.borrow())?;
        if token_fail.is_initialized() {
            return Err(PoolError::TokenMintInUse.into());
        }

        invoke(
            &spl_token::instruction::initialize_account(
                token_program_info.key,
                deposit_account_info.key,
                deposit_token_mint_info.key,
                authority_info.key,
            )
            .unwrap(),
            &[
                token_program_info.clone(),
                deposit_account_info.clone(),
                deposit_token_mint_info.clone(),
                authority_info.clone(),
                rent_info.clone(),
            ],
        )?;

        invoke(
            &spl_token::instruction::initialize_mint(
                &spl_token::id(),
                token_pass_mint_info.key,
                authority_info.key,
                None,
                deposit_token_mint.decimals,
            )
            .unwrap(),
            &[
                token_program_info.clone(),
                token_pass_mint_info.clone(),
                rent_info.clone(),
            ],
        )?;

        invoke(
            &spl_token::instruction::initialize_mint(
                &spl_token::id(),
                token_fail_mint_info.key,
                authority_info.key,
                None,
                deposit_token_mint.decimals,
            )
            .unwrap(),
            &[
                token_program_info.clone(),
                token_fail_mint_info.clone(),
                rent_info.clone(),
            ],
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

    /// Process Deposit instruction
    pub fn process_deposit(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_account_info = next_account_info(account_info_iter)?;
        let authority_account_info = next_account_info(account_info_iter)?;
        let user_token_account_info = next_account_info(account_info_iter)?;
        let deposit_token_account_info = next_account_info(account_info_iter)?;
        let token_pass_mint_info = next_account_info(account_info_iter)?;
        let token_fail_mint_info = next_account_info(account_info_iter)?;
        let token_pass_destination_account_info = next_account_info(account_info_iter)?;
        let token_fail_destination_account_info = next_account_info(account_info_iter)?;
        let token_program_id_info = next_account_info(account_info_iter)?;

        if amount == 0 {
            return Err(PoolError::InvalidAmount.into());
        }

        let pool = Pool::unpack(&pool_account_info.data.borrow())?;

        let authority_pub_key =
            Self::authority_id(program_id, pool_account_info.key, pool.bump_seed)?;
        if *authority_account_info.key != authority_pub_key {
            return Err(PoolError::InvalidAuthorityAccount.into());
        }

        // Transfer deposit tokens from user's account to our deposit account
        Self::transfer(
            pool_account_info.key,
            pool.bump_seed,
            deposit_token_account_info.clone(),
            authority_account_info.clone(),
            token_program_id_info.clone(),
            user_token_account_info.clone(),
            amount,
        )?;
        // Mint PASS tokens to user account
        Self::mint(
            pool_account_info.key,
            pool.bump_seed,
            token_program_id_info.clone(),
            token_pass_mint_info.clone(),
            token_pass_destination_account_info.clone(),
            authority_account_info.clone(),
            amount,
        )?;
        // Mint FAIL tokens to user account
        Self::mint(
            pool_account_info.key,
            pool.bump_seed,
            token_program_id_info.clone(),
            token_fail_mint_info.clone(),
            token_fail_destination_account_info.clone(),
            authority_account_info.clone(),
            amount,
        )?;
        Ok(())
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
            PoolInstruction::Deposit(amount) => {
                msg!("Instruction: Deposit");
                Self::process_deposit(program_id, accounts, amount)
            }
            _ => unimplemented!(),
        }
    }
}
