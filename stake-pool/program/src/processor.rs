//! Program state processor

#![cfg(feature = "program")]

use crate::{
    error::Error,
    instruction::{InitArgs, StakePoolInstruction},
    state::{StakePool, State},
};
use num_traits::FromPrimitive;
#[cfg(not(target_arch = "bpf"))]
use solana_sdk::instruction::Instruction;
#[cfg(target_arch = "bpf")]
use solana_sdk::program::{create_program_address, invoke_signed};
use solana_sdk::{
    account_info::next_account_info, account_info::AccountInfo, decode_error::DecodeError,
    entrypoint::ProgramResult, info, program_error::PrintProgramError, program_error::ProgramError,
    pubkey::Pubkey,
};
use std::convert::TryFrom;

/// Program state handler.
pub struct Processor {}
impl Processor {
    /// Calculates the authority id by generating a program address.
    pub fn authority_id(program_id: &Pubkey, my_info: &Pubkey, nonce: u8) -> Result<Pubkey, Error> {
        create_program_address(&[&my_info.to_bytes()[..32], &[nonce]], program_id)
            .or(Err(Error::InvalidProgramAddress))
    }

    /// Issue a stake_split instruction.
    pub fn stake_split(
        accounts: &[AccountInfo],
        my_info: &Pubkey,
        authority: &Pubkey,
        nonce: u8,
        stake: &Pubkey,
        amount: u64,
    ) -> Result<(), ProgramError> {
        unimplemented!();
    }

    /// Issue a stake_set_owner instruction.
    pub fn stake_set_owner(
        accounts: &[AccountInfo],
        my_info: &Pubkey,
        authority: &Pubkey,
        nonce: u8,
        stake: &Pubkey,
        owner: &Pubkey,
    ) -> Result<(), ProgramError> {
        unimplemented!();
    }

    /// Issue a spl_token `Burn` instruction.
    pub fn token_burn(
        accounts: &[AccountInfo],
        my_info: &Pubkey,
        token_program_id: &Pubkey,
        burn_account: &Pubkey,
        mint: &Pubkey,
        authority: &Pubkey,
        nonce: u8,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let my_info = my_info.to_bytes();
        let authority_signature_seeds = [&my_info[..32], &[nonce]];
        let signers = &[&authority_signature_seeds[..]];
        let ix = spl_token::instruction::burn(
            token_program_id,
            burn_account,
            mint,
            authority,
            &[],
            amount,
        )?;
        invoke_signed(&ix, accounts, signers)
    }

    /// Issue a spl_token `MintTo` instruction.
    pub fn token_mint_to(
        accounts: &[AccountInfo],
        token_program_id: &Pubkey,
        swap: &Pubkey,
        mint: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let my_info = my_info.to_bytes();
        let authority_signature_seeds = [&my_info[..32], &[nonce]];
        let signers = &[&authority_signature_seeds[..]];
        let ix = spl_token::instruction::mint_to(
            token_program_id,
            mint,
            destination,
            authority,
            &[],
            amount,
        )?;
        invoke_signed(&ix, accounts, signers)
    }

    /// Processes an [Initialize](enum.Instruction.html).
    pub fn process_initialize(
        _program_id: &Pubkey,
        init: InitArgs,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let stake_pool_info = next_account_info(account_info_iter)?;
        let owner_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let owner_fee_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        if State::Unallocated != State::deserialize(&stake_pool_info.data.borrow())? {
            return Err(Error::AlreadyInUse.into());
        }

        let stake_pool = State::Init(StakePool {
            owner: *owner_info.key,
            deposit_nonce: init.deposit_nonce,
            withdraw_nonce: init.deposit_nonce,
            pool_mint: *pool_mint_info.key,
            owner_fee_account: *owner_fee_info.key,
            token_program_id: *token_program_info.key,
            stake_total: 0,
            pool_total: 0,
            fee: init.fee,
        });
        stake_pool.serialize(&mut stake_pool_info.data.borrow_mut())
    }

    /// Processes an [Withdraw](enum.Instruction.html).
    pub fn process_deposit(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let stake_pool_info = next_account_info(account_info_iter)?;
        let deposit_info = next_account_info(account_info_iter)?;
        let withdraw_info = next_account_info(account_info_iter)?;
        let source_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let stake_info = next_account_info(account_info_iter)?;
        let stake_dest_user_info = next_account_info(account_info_iter)?;
        let dest_user_info = next_account_info(account_info_iter)?;
        let owner_fee_info = next_account_info(account_info_iter)?;

        let stake_pool = State::deserialize(&stake_pool_info.data.borrow())?.stake_pool()?;

        if *withdraw_info.key
            != Self::authority_id(program_id, stake_pool_info.key, stake_pool.withdraw_nonce)?
        {
            return Err(Error::InvalidProgramAddress.into());
        }

        if *deposit_info.key
            != Self::authority_id(program_id, stake_pool_info.key, stake_pool.deposit_nonce)?
        {
            return Err(Error::InvalidProgramAddress.into());
        }

        if stake_pool.owner_fee_account != *owner_fee_info.key {
            return Err(Error::InvalidInput.into());
        }

        let stake_lamports = **stake_info.lamports.borrow();
        let pool_amount = stake_pool
            .calc_pool_amount(stake_lamports)
            .ok_or(Error::CalculationFailure)?;

        let fee_amount = stake_pool
            .calc_fee_amount(pool_amount)
            .ok_or(Error::CalculationFailure)?;

        let user_amount = pool_amount
            .checked_sub(fee_amount)
            .ok_or(Error::CalculationFailure)?;

        Self::stake_set_owner(
            accounts,
            stake_pool_info.key,
            deposit_info.key,
            stake_pool.deposit_nonce,
            stake_info.key,
            withdraw_info.key,
        )?;

        let user_amount = <u64>::try_from(user_amount).or(Err(Error::CalculationFailure))?;
        Self::token_mint_to(
            accounts,
            &stake_pool.token_program_id,
            stake_pool_info.key,
            pool_mint_info.key,
            dest_user_info.key,
            withdraw_info.key,
            user_amount,
        )?;
        let fee_amount = <u64>::try_from(fee_amount).or(Err(Error::CalculationFailure))?;
        Self::token_mint_to(
            accounts,
            &stake_pool.token_program_id,
            stake_pool_info.key,
            pool_mint_info.key,
            owner_fee_info.key,
            withdraw_info.key,
            fee_amount as u64,
        )?;
        let pool_amount = <u64>::try_from(pool_amount).or(Err(Error::CalculationFailure))?;
        stake_pool.pool_total += pool_amount;
        stake_pool.stake_total += stake_lamports;
        State::Init(stake_pool).serialize(&mut stake_pool_info.data.borrow_mut());
        Ok(())
    }

    /// Processes an [Withdraw](enum.Instruction.html).
    pub fn process_withdraw(
        program_id: &Pubkey,
        stake_amount: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let stake_pool_info = next_account_info(account_info_iter)?;
        let withdraw_info = next_account_info(account_info_iter)?;
        let source_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let stake_info = next_account_info(account_info_iter)?;
        let stake_dest_user_info = next_account_info(account_info_iter)?;
        let dest_user_info = next_account_info(account_info_iter)?;

        let stake_pool = State::deserialize(&stake_pool_info.data.borrow())?.stake_pool()?;

        if *withdraw_info.key
            != Self::authority_id(program_id, stake_pool_info.key, stake_pool.withdraw_nonce)?
        {
            return Err(Error::InvalidProgramAddress.into());
        }

        let pool_amount = stake_pool
            .calc_pool_amount(stake_amount)
            .ok_or(Error::CalculationFailure)?;
        let pool_amount = <u64>::try_from(pool_amount).or(Err(Error::CalculationFailure))?;

        Self::stake_split(
            accounts,
            stake_pool_info.key,
            withdraw_info.key,
            stake_pool.withdraw_nonce,
            stake_info.key,
            stake_amount,
        )?;

        Self::stake_set_owner(
            accounts,
            stake_pool_info.key,
            withdraw_info.key,
            stake_pool.withdraw_nonce,
            stake_info.key,
            dest_user_info.key,
        )?;

        Self::token_burn(
            accounts,
            stake_info.key,
            &stake_pool.token_program_id,
            dest_user_info.key,
            stake_pool_info.key,
            withdraw_info.key,
            stake_pool.withdraw_nonce,
            pool_amount,
        )?;

        stake_pool.pool_total -= pool_amount;
        stake_pool.stake_total -= stake_amount;
        State::Init(stake_pool).serialize(&mut stake_pool_info.data.borrow_mut());
        Ok(())
    }
    //    /// Processes an [UpdateStakeAuthority](enum.Instruction.html).
    //    pub fn process_update_stake_auth(
    //        program_id: &Pubkey,
    //        accounts: &[AccountInfo],
    //    ) -> ProgramResult {
    //        let account_info_iter = &mut accounts.iter();
    //        let stake_pool_info = next_account_info(account_info_iter)?;
    //        let owner_info = next_account_info(account_info_iter)?;
    //        let withdraw_info = next_account_info(account_info_iter)?;
    //        let staking_info = next_account_info(account_info_iter)?;
    //        let stake_info = next_account_info(account_info_iter)?;
    //
    //        let stake_pool = Self::deserialize(&swap_info.data.borrow())?.token_swap()?;
    //
    //        if *owner_info.key != stake_pool.owner {
    //            return Err(Error::InvalidInput.into());
    //        }
    //        if !*owner_info.is_signer {
    //            return Err(Error::InvalidInput.into());
    //        }
    //
    //        if *withdraw_info.key != Self::authority_id(program_id, stake_pool_info.key)? {
    //            return Err(Error::InvalidProgramAddress.into());
    //        }
    //        Self::update_stake_auth(
    //            accounts,
    //            stake_info.key,
    //            withdraw_info.key,
    //            staking_info.key,
    //        )?;
    //        Ok(())
    //    }
    //
    //    /// Processes an [UpdateOwner](enum.Instruction.html).
    //    pub fn process_update_owner(
    //        program_id: &Pubkey,
    //        accounts: &[AccountInfo],
    //    ) -> ProgramResult {
    //        let account_info_iter = &mut accounts.iter();
    //        let stake_pool_info = next_account_info(account_info_iter)?;
    //        let owner_info = next_account_info(account_info_iter)?;
    //        let new_owner_info = next_account_info(account_info_iter)?;
    //        let new_owner_fee_info = next_account_info(account_info_iter)?;
    //
    //        let mut stake_pool = Self::deserialize(&stake_pool_info.data.borrow())?.stake_pool()?;
    //
    //        if *owner_info.key != stake_pool.owner {
    //            return Err(Error::InvalidInput.into());
    //        }
    //        if !*owner_info.is_signer {
    //            return Err(Error::InvalidInput.into());
    //        }
    //
    //        )?;
    //        stake_pool.owner = new_owner_info.key;
    //        stake_pool.owner_fee_account = new_owner_fee_info.key;
    //        stake_pool.serialize(&mut stake_pool_info.data.borrow_mut())
    //        Ok(())
    //    }
    //    /// Processes an [Instruction](enum.Instruction.html).
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        let instruction = StakePoolInstruction::deserialize(input)?;
        match instruction {
            StakePoolInstruction::Initialize(init) => {
                info!("Instruction: Init");
                Self::process_initialize(program_id, init, accounts)
            }
            StakePoolInstruction::Deposit => {
                info!("Instruction: Deposit");
                Self::process_deposit(program_id, accounts)
            }
            StakePoolInstruction::Withdraw(amount) => {
                info!("Instruction: Withdraw");
                Self::process_withdraw(program_id, amount, accounts)
            } //StakePoolInstruction::UpdateStakingAuthority => {
              //    info!("Instruction: UpdateStakingAuthority");
              //    Self::process_update_staking_auth(program_id, accounts)
              //}
              //StakePoolInstruction::UpdateOwner => {
              //    info!("Instruction: UpdateOwner");
              //    Self::process_update_owner(program_id, accounts)
              //}
        }
    }
}

// Test program id for the swap program.
#[cfg(not(target_arch = "bpf"))]
const SWAP_PROGRAM_ID: Pubkey = Pubkey::new_from_array([2u8; 32]);

/// Routes invokes to the token program, used for testing.
#[cfg(not(target_arch = "bpf"))]
pub fn invoke_signed<'a>(
    instruction: &Instruction,
    account_infos: &[AccountInfo<'a>],
    signers_seeds: &[&[&[u8]]],
) -> ProgramResult {
    let mut new_account_infos = vec![];
    for meta in instruction.accounts.iter() {
        for account_info in account_infos.iter() {
            if meta.pubkey == *account_info.key {
                let mut new_account_info = account_info.clone();
                for seeds in signers_seeds.iter() {
                    let signer = create_program_address(seeds, &SWAP_PROGRAM_ID).unwrap();
                    if *account_info.key == signer {
                        new_account_info.is_signer = true;
                    }
                }
                new_account_infos.push(new_account_info);
            }
        }
    }
    spl_token::processor::Processor::process(
        &instruction.program_id,
        &new_account_infos,
        &instruction.data,
    )
}

/// TODO: Remove this stub function once solana-sdk exports it
#[cfg(not(target_arch = "bpf"))]
pub fn create_program_address(
    seeds: &[&[u8]],
    program_id: &Pubkey,
) -> Result<Pubkey, solana_sdk::pubkey::PubkeyError> {
    let mut hasher = solana_sdk::hash::Hasher::default();
    for seed in seeds.iter() {
        if seed.len() > solana_sdk::pubkey::MAX_SEED_LEN {
            return Err(solana_sdk::pubkey::PubkeyError::MaxSeedLengthExceeded);
        }
        hasher.hash(seed);
    }
    hasher.hashv(&[program_id.as_ref(), "ProgramDerivedAddress".as_ref()]);

    Ok(Pubkey::new(
        solana_sdk::hash::hash(hasher.result().as_ref()).as_ref(),
    ))
}

impl PrintProgramError for Error {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            Error::AlreadyInUse => info!("Error: AlreadyInUse"),
            Error::InvalidProgramAddress => info!("Error: InvalidProgramAddress"),
            Error::InvalidOwner => info!("Error: InvalidOwner"),
            Error::ExpectedToken => info!("Error: ExpectedToken"),
            Error::ExpectedAccount => info!("Error: ExpectedAccount"),
            Error::InvalidSupply => info!("Error: InvalidSupply"),
            Error::InvalidDelegate => info!("Error: InvalidDelegate"),
            Error::InvalidState => info!("Error: InvalidState"),
            Error::InvalidInput => info!("Error: InvalidInput"),
            Error::InvalidOutput => info!("Error: InvalidOutput"),
            Error::CalculationFailure => info!("Error: CalculationFailure"),
        }
    }
}

// Pull in syscall stubs when building for non-BPF targets
#[cfg(not(target_arch = "bpf"))]
solana_sdk::program_stubs!();

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::initialize;
    use solana_sdk::{
        account::Account, account_info::create_is_signer_account_infos, instruction::Instruction,
    };
    use spl_token::{
        instruction::{initialize_account, initialize_mint},
        processor::Processor as SplProcessor,
        state::{Account as SplAccount, Mint as SplMint},
    };

    const TOKEN_PROGRAM_ID: Pubkey = Pubkey::new_from_array([1u8; 32]);

    fn pubkey_rand() -> Pubkey {
        Pubkey::new(&rand::random::<[u8; 32]>())
    }

    fn do_process_instruction(
        instruction: Instruction,
        accounts: Vec<&mut Account>,
    ) -> ProgramResult {
        let mut meta = instruction
            .accounts
            .iter()
            .zip(accounts)
            .map(|(account_meta, account)| (&account_meta.pubkey, account_meta.is_signer, account))
            .collect::<Vec<_>>();

        let account_infos = create_is_signer_account_infos(&mut meta);
        if instruction.program_id == SWAP_PROGRAM_ID {
            State::process(&instruction.program_id, &account_infos, &instruction.data)
        } else {
            SplProcessor::process(&instruction.program_id, &account_infos, &instruction.data)
        }
    }

    fn mint_token(
        program_id: &Pubkey,
        authority_key: &Pubkey,
        amount: u64,
    ) -> ((Pubkey, Account), (Pubkey, Account)) {
        let token_key = pubkey_rand();
        let mut token_account = Account::new(0, size_of::<SplMint>(), &program_id);
        let account_key = pubkey_rand();
        let mut account_account = Account::new(0, size_of::<SplAccount>(), &program_id);

        // create pool and pool account
        do_process_instruction(
            initialize_account(&program_id, &account_key, &token_key, &authority_key).unwrap(),
            vec![
                &mut account_account,
                &mut Account::default(),
                &mut token_account,
            ],
        )
        .unwrap();
        let mut authority_account = Account::default();
        do_process_instruction(
            initialize_mint(
                &program_id,
                &token_key,
                Some(&account_key),
                Some(&authority_key),
                amount,
                2,
            )
            .unwrap(),
            if amount == 0 {
                vec![&mut token_account, &mut authority_account]
            } else {
                vec![
                    &mut token_account,
                    &mut account_account,
                    &mut authority_account,
                ]
            },
        )
        .unwrap();

        return ((token_key, token_account), (account_key, account_account));
    }

    #[test]
    fn test_initialize() {
        let swap_key = pubkey_rand();
        let mut swap_account = Account::new(0, size_of::<State>(), &SWAP_PROGRAM_ID);
        let authority_key = State::authority_id(&SWAP_PROGRAM_ID, &swap_key).unwrap();
        let mut authority_account = Account::default();

        let ((pool_key, mut pool_account), (pool_token_key, mut pool_token_account)) =
            mint_token(&TOKEN_PROGRAM_ID, &authority_key, 0);
        let ((_token_a_mint_key, mut _token_a_mint_account), (token_a_key, mut token_a_account)) =
            mint_token(&TOKEN_PROGRAM_ID, &authority_key, 1000);
        let ((_token_b_mint_key, mut _token_b_mint_account), (token_b_key, mut token_b_account)) =
            mint_token(&TOKEN_PROGRAM_ID, &authority_key, 1000);

        // StakePool Init
        do_process_instruction(
            initialize(
                &SWAP_PROGRAM_ID,
                &TOKEN_PROGRAM_ID,
                &swap_key,
                &authority_key,
                &token_a_key,
                &token_b_key,
                &pool_key,
                &pool_token_key,
                Fee {
                    denominator: 1,
                    numerator: 2,
                },
            )
            .unwrap(),
            vec![
                &mut swap_account,
                &mut authority_account,
                &mut token_a_account,
                &mut token_b_account,
                &mut pool_account,
                &mut pool_token_account,
                &mut Account::default(),
            ],
        )
        .unwrap();
    }
}
