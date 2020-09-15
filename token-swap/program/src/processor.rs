//! Program state processor

#![cfg(feature = "program")]

use crate::{
    error::SwapError,
    instruction::SwapInstruction,
    state::{Invariant, SwapInfo},
};
use num_traits::FromPrimitive;
#[cfg(not(target_arch = "bpf"))]
use solana_sdk::instruction::Instruction;
#[cfg(target_arch = "bpf")]
use solana_sdk::program::invoke_signed;
use solana_sdk::{
    account_info::{next_account_info, AccountInfo},
    decode_error::DecodeError,
    entrypoint::ProgramResult,
    info,
    program_error::PrintProgramError,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use spl_token::pack::Pack;

// Test program id for the swap program.
#[cfg(not(target_arch = "bpf"))]
const SWAP_PROGRAM_ID: Pubkey = Pubkey::new_from_array([2u8; 32]);
// Test program id for the token program.
#[cfg(not(target_arch = "bpf"))]
const TOKEN_PROGRAM_ID: Pubkey = Pubkey::new_from_array([1u8; 32]);

/// Program state handler.
pub struct Processor {}
impl Processor {
    /// Unpacks a spl_token `Account`.
    pub fn unpack_token_account(data: &[u8]) -> Result<spl_token::state::Account, SwapError> {
        spl_token::state::Account::unpack(data).map_err(|_| SwapError::ExpectedAccount)
    }

    /// Unpacks a spl_token `Mint`.
    pub fn unpack_mint(data: &[u8]) -> Result<spl_token::state::Mint, SwapError> {
        spl_token::state::Mint::unpack(data).map_err(|_| SwapError::ExpectedMint)
    }

    /// Calculates the authority id by generating a program address.
    pub fn authority_id(
        program_id: &Pubkey,
        my_info: &Pubkey,
        nonce: u8,
    ) -> Result<Pubkey, SwapError> {
        Pubkey::create_program_address(&[&my_info.to_bytes()[..32], &[nonce]], program_id)
            .or(Err(SwapError::InvalidProgramAddress))
    }

    /// Issue a spl_token `Burn` instruction.
    pub fn token_burn<'a>(
        swap: &Pubkey,
        token_program: AccountInfo<'a>,
        burn_account: AccountInfo<'a>,
        mint: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        nonce: u8,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let swap_bytes = swap.to_bytes();
        let authority_signature_seeds = [&swap_bytes[..32], &[nonce]];
        let signers = &[&authority_signature_seeds[..]];

        let ix = spl_token::instruction::burn(
            token_program.key,
            burn_account.key,
            mint.key,
            authority.key,
            &[],
            amount,
        )?;

        invoke_signed(
            &ix,
            &[burn_account, mint, authority, token_program],
            signers,
        )
    }

    /// Issue a spl_token `MintTo` instruction.
    pub fn token_mint_to<'a>(
        swap: &Pubkey,
        token_program: AccountInfo<'a>,
        mint: AccountInfo<'a>,
        destination: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        nonce: u8,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let swap_bytes = swap.to_bytes();
        let authority_signature_seeds = [&swap_bytes[..32], &[nonce]];
        let signers = &[&authority_signature_seeds[..]];
        let ix = spl_token::instruction::mint_to(
            token_program.key,
            mint.key,
            destination.key,
            authority.key,
            &[],
            amount,
        )?;

        invoke_signed(&ix, &[mint, destination, authority, token_program], signers)
    }

    /// Issue a spl_token `Transfer` instruction.
    pub fn token_transfer<'a>(
        swap: &Pubkey,
        token_program: AccountInfo<'a>,
        source: AccountInfo<'a>,
        destination: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        nonce: u8,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let swap_bytes = swap.to_bytes();
        let authority_signature_seeds = [&swap_bytes[..32], &[nonce]];
        let signers = &[&authority_signature_seeds[..]];
        let ix = spl_token::instruction::transfer(
            token_program.key,
            source.key,
            destination.key,
            authority.key,
            &[],
            amount,
        )?;
        invoke_signed(
            &ix,
            &[source, destination, authority, token_program],
            signers,
        )
    }

    /// Processes an [Initialize](enum.Instruction.html).
    pub fn process_initialize(
        program_id: &Pubkey,
        nonce: u8,
        fee_numerator: u64,
        fee_denominator: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let token_a_info = next_account_info(account_info_iter)?;
        let token_b_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let destination_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let token_swap = SwapInfo::unpack_unchecked(&swap_info.data.borrow())?;
        if token_swap.is_initialized {
            return Err(SwapError::AlreadyInUse.into());
        }

        if *authority_info.key != Self::authority_id(program_id, swap_info.key, nonce)? {
            return Err(SwapError::InvalidProgramAddress.into());
        }
        let token_a = Self::unpack_token_account(&token_a_info.data.borrow())?;
        let token_b = Self::unpack_token_account(&token_b_info.data.borrow())?;
        let destination = Self::unpack_token_account(&destination_info.data.borrow())?;
        let pool_mint = Self::unpack_mint(&pool_info.data.borrow())?;
        if *authority_info.key != token_a.owner {
            return Err(SwapError::InvalidOwner.into());
        }
        if *authority_info.key != token_b.owner {
            return Err(SwapError::InvalidOwner.into());
        }
        if *authority_info.key == destination.owner {
            return Err(SwapError::InvalidOutputOwner.into());
        }
        if spl_token::option::COption::Some(*authority_info.key) != pool_mint.mint_authority {
            return Err(SwapError::InvalidOwner.into());
        }
        if token_a.mint == token_b.mint {
            return Err(SwapError::RepeatedMint.into());
        }
        if token_b.amount == 0 {
            return Err(SwapError::EmptySupply.into());
        }
        if token_a.amount == 0 {
            return Err(SwapError::EmptySupply.into());
        }
        if token_a.delegate.is_some() {
            return Err(SwapError::InvalidDelegate.into());
        }
        if token_b.delegate.is_some() {
            return Err(SwapError::InvalidDelegate.into());
        }
        if pool_mint.supply != 0 {
            return Err(SwapError::InvalidSupply.into());
        }
        // double check to be totally sure -- TODO check if this is necessary
        if destination.amount != 0 {
            return Err(SwapError::InvalidSupply.into());
        }

        // liquidity is measured in terms of token_a's value since both sides of
        // the pool are equal
        let amount = token_a.amount;
        Self::token_mint_to(
            swap_info.key,
            token_program_info.clone(),
            pool_info.clone(),
            destination_info.clone(),
            authority_info.clone(),
            nonce,
            amount,
        )?;

        let obj = SwapInfo {
            is_initialized: true,
            nonce,
            token_a: *token_a_info.key,
            token_b: *token_b_info.key,
            pool_mint: *pool_info.key,
            fee_numerator,
            fee_denominator,
        };
        obj.pack(&mut swap_info.data.borrow_mut());
        Ok(())
    }

    /// Processes an [Swap](enum.Instruction.html).
    pub fn process_swap(
        program_id: &Pubkey,
        amount: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let source_info = next_account_info(account_info_iter)?;
        let swap_source_info = next_account_info(account_info_iter)?;
        let swap_destination_info = next_account_info(account_info_iter)?;
        let destination_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let token_swap = SwapInfo::unpack(&swap_info.data.borrow())?;

        if *authority_info.key != Self::authority_id(program_id, swap_info.key, token_swap.nonce)? {
            return Err(SwapError::InvalidProgramAddress.into());
        }
        if !(*swap_source_info.key == token_swap.token_a
            || *swap_source_info.key == token_swap.token_b)
        {
            return Err(SwapError::InvalidInput.into());
        }
        if !(*swap_destination_info.key == token_swap.token_a
            || *swap_destination_info.key == token_swap.token_b)
        {
            return Err(SwapError::InvalidOutput.into());
        }
        if *swap_source_info.key == *swap_destination_info.key {
            return Err(SwapError::InvalidInput.into());
        }
        let source_account = Self::unpack_token_account(&swap_source_info.data.borrow())?;
        let dest_account = Self::unpack_token_account(&swap_destination_info.data.borrow())?;

        let output = if *swap_source_info.key == token_swap.token_a {
            let mut invariant = Invariant {
                token_a: source_account.amount,
                token_b: dest_account.amount,
                fee_numerator: token_swap.fee_numerator,
                fee_denominator: token_swap.fee_denominator,
            };
            invariant
                .swap_a_to_b(amount)
                .ok_or(SwapError::CalculationFailure)?
        } else {
            let mut invariant = Invariant {
                token_a: dest_account.amount,
                token_b: source_account.amount,
                fee_numerator: token_swap.fee_numerator,
                fee_denominator: token_swap.fee_denominator,
            };
            invariant
                .swap_b_to_a(amount)
                .ok_or(SwapError::CalculationFailure)?
        };
        Self::token_transfer(
            swap_info.key,
            token_program_info.clone(),
            source_info.clone(),
            swap_source_info.clone(),
            authority_info.clone(),
            token_swap.nonce,
            amount,
        )?;
        Self::token_transfer(
            swap_info.key,
            token_program_info.clone(),
            swap_destination_info.clone(),
            destination_info.clone(),
            authority_info.clone(),
            token_swap.nonce,
            output,
        )?;
        Ok(())
    }

    /// Processes an [Deposit](enum.Instruction.html).
    pub fn process_deposit(
        program_id: &Pubkey,
        a_amount: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let source_a_info = next_account_info(account_info_iter)?;
        let source_b_info = next_account_info(account_info_iter)?;
        let token_a_info = next_account_info(account_info_iter)?;
        let token_b_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let dest_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let token_swap = SwapInfo::unpack(&swap_info.data.borrow())?;
        if *authority_info.key != Self::authority_id(program_id, swap_info.key, token_swap.nonce)? {
            return Err(SwapError::InvalidProgramAddress.into());
        }
        if *token_a_info.key != token_swap.token_a {
            return Err(SwapError::InvalidInput.into());
        }
        if *token_b_info.key != token_swap.token_b {
            return Err(SwapError::InvalidInput.into());
        }
        if *pool_info.key != token_swap.pool_mint {
            return Err(SwapError::InvalidInput.into());
        }
        let token_a = Self::unpack_token_account(&token_a_info.data.borrow())?;
        let token_b = Self::unpack_token_account(&token_b_info.data.borrow())?;

        let invariant = Invariant {
            token_a: token_a.amount,
            token_b: token_b.amount,
            fee_numerator: token_swap.fee_numerator,
            fee_denominator: token_swap.fee_denominator,
        };
        let b_amount = invariant
            .exchange_rate(a_amount)
            .ok_or(SwapError::CalculationFailure)?;

        // liquidity is measured in terms of token_a's value
        // since both sides of the pool are equal
        let output = a_amount;

        Self::token_transfer(
            swap_info.key,
            token_program_info.clone(),
            source_a_info.clone(),
            token_a_info.clone(),
            authority_info.clone(),
            token_swap.nonce,
            a_amount,
        )?;
        Self::token_transfer(
            swap_info.key,
            token_program_info.clone(),
            source_b_info.clone(),
            token_b_info.clone(),
            authority_info.clone(),
            token_swap.nonce,
            b_amount,
        )?;
        Self::token_mint_to(
            swap_info.key,
            token_program_info.clone(),
            pool_info.clone(),
            dest_info.clone(),
            authority_info.clone(),
            token_swap.nonce,
            output,
        )?;

        Ok(())
    }

    /// Processes an [Withdraw](enum.Instruction.html).
    pub fn process_withdraw(
        program_id: &Pubkey,
        amount: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let source_info = next_account_info(account_info_iter)?;
        let token_a_info = next_account_info(account_info_iter)?;
        let token_b_info = next_account_info(account_info_iter)?;
        let dest_token_a_info = next_account_info(account_info_iter)?;
        let dest_token_b_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let token_swap = SwapInfo::unpack(&swap_info.data.borrow())?;
        if *authority_info.key != Self::authority_id(program_id, swap_info.key, token_swap.nonce)? {
            return Err(SwapError::InvalidProgramAddress.into());
        }
        if *token_a_info.key != token_swap.token_a {
            return Err(SwapError::InvalidInput.into());
        }
        if *token_b_info.key != token_swap.token_b {
            return Err(SwapError::InvalidInput.into());
        }

        let token_a = Self::unpack_token_account(&token_a_info.data.borrow())?;
        let token_b = Self::unpack_token_account(&token_b_info.data.borrow())?;

        let invariant = Invariant {
            token_a: token_a.amount,
            token_b: token_b.amount,
            fee_numerator: token_swap.fee_numerator,
            fee_denominator: token_swap.fee_denominator,
        };

        let a_amount = amount;
        let b_amount = invariant
            .exchange_rate(a_amount)
            .ok_or(SwapError::CalculationFailure)?;

        Self::token_transfer(
            swap_info.key,
            token_program_info.clone(),
            token_a_info.clone(),
            dest_token_a_info.clone(),
            authority_info.clone(),
            token_swap.nonce,
            a_amount,
        )?;
        Self::token_transfer(
            swap_info.key,
            token_program_info.clone(),
            token_b_info.clone(),
            dest_token_b_info.clone(),
            authority_info.clone(),
            token_swap.nonce,
            b_amount,
        )?;
        Self::token_burn(
            swap_info.key,
            token_program_info.clone(),
            source_info.clone(),
            pool_mint_info.clone(),
            authority_info.clone(),
            token_swap.nonce,
            amount,
        )?;
        Ok(())
    }

    /// Processes an [Instruction](enum.Instruction.html).
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        let instruction = SwapInstruction::unpack(input)?;
        match instruction {
            SwapInstruction::Initialize {
                fee_numerator,
                fee_denominator,
                nonce,
            } => {
                info!("Instruction: Init");
                Self::process_initialize(
                    program_id,
                    nonce,
                    fee_numerator,
                    fee_denominator,
                    accounts,
                )
            }
            SwapInstruction::Swap { amount } => {
                info!("Instruction: Swap");
                Self::process_swap(program_id, amount, accounts)
            }
            SwapInstruction::Deposit { amount } => {
                info!("Instruction: Deposit");
                Self::process_deposit(program_id, amount, accounts)
            }
            SwapInstruction::Withdraw { amount } => {
                info!("Instruction: Withdraw");
                Self::process_withdraw(program_id, amount, accounts)
            }
        }
    }
}

/// Routes invokes to the token program, used for testing.
#[cfg(not(target_arch = "bpf"))]
pub fn invoke_signed<'a>(
    instruction: &Instruction,
    account_infos: &[AccountInfo<'a>],
    signers_seeds: &[&[&[u8]]],
) -> ProgramResult {
    let mut new_account_infos = vec![];

    // mimic check for token program in accounts
    if !account_infos.iter().any(|x| *x.key == TOKEN_PROGRAM_ID) {
        return Err(ProgramError::InvalidAccountData);
    }

    for meta in instruction.accounts.iter() {
        for account_info in account_infos.iter() {
            if meta.pubkey == *account_info.key {
                let mut new_account_info = account_info.clone();
                for seeds in signers_seeds.iter() {
                    let signer = Pubkey::create_program_address(&seeds, &SWAP_PROGRAM_ID).unwrap();
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

impl PrintProgramError for SwapError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            SwapError::AlreadyInUse => info!("Error: Swap account already in use"),
            SwapError::InvalidProgramAddress => {
                info!("Error: Invalid program address generated from nonce and key")
            }
            SwapError::InvalidOwner => {
                info!("Error: The input account owner is not the program address")
            }
            SwapError::InvalidOutputOwner => {
                info!("Error: Output pool account owner cannot be the program address")
            }
            SwapError::ExpectedMint => {
                info!("Error: Deserialized account is not an SPL Token mint")
            }
            SwapError::ExpectedAccount => {
                info!("Error: Deserialized account is not an SPL Token account")
            }
            SwapError::EmptySupply => info!("Error: Input token account empty"),
            SwapError::InvalidSupply => info!("Error: Pool token mint has a non-zero supply"),
            SwapError::RepeatedMint => info!("Error: Swap input token accounts have the same mint"),
            SwapError::InvalidDelegate => info!("Error: Token account has a delegate"),
            SwapError::InvalidSwapInfo => info!("Error: Swap info invalid"),
            SwapError::InvalidInput => info!("Error: InvalidInput"),
            SwapError::InvalidOutput => info!("Error: InvalidOutput"),
            SwapError::CalculationFailure => info!("Error: CalculationFailure"),
            SwapError::InvalidInstruction => info!("Error: InvalidInstruction"),
        }
    }
}

// Pull in syscall stubs when building for non-BPF targets
#[cfg(not(target_arch = "bpf"))]
solana_sdk::program_stubs!();

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        instruction::{deposit, initialize, swap, withdraw},
        state::SwapResult,
    };
    use solana_sdk::{
        account::Account, account_info::create_is_signer_account_infos, instruction::Instruction,
        rent::Rent, sysvar::rent,
    };
    use spl_token::{
        instruction::{
            approve, initialize_account, initialize_mint, mint_to, revoke
        },
        pack::Pack,
        processor::Processor as SplProcessor,
        state::{Account as SplAccount, Mint as SplMint},
    };
    use std::mem::size_of;

    struct SwapAccountInfo {
        nonce: u8,
        authority_key: Pubkey,
        fee_numerator: u64,
        fee_denominator: u64,
        swap_key: Pubkey,
        swap_account: Account,
        pool_mint_key: Pubkey,
        pool_mint_account: Account,
        pool_token_key: Pubkey,
        pool_token_account: Account,
        token_a_key: Pubkey,
        token_a_account: Account,
        token_a_mint_key: Pubkey,
        token_a_mint_account: Account,
        token_b_key: Pubkey,
        token_b_account: Account,
        token_b_mint_key: Pubkey,
        token_b_mint_account: Account,
    }

    impl SwapAccountInfo {
        pub fn new(
            user_key: &Pubkey,
            fee_numerator: u64,
            fee_denominator: u64,
            token_a_amount: u64,
            token_b_amount: u64,
        ) -> Self {
            let swap_key = pubkey_rand();
            let swap_account = Account::new(0, size_of::<SwapInfo>(), &SWAP_PROGRAM_ID);
            let (authority_key, nonce) =
                Pubkey::find_program_address(&[&swap_key.to_bytes()[..]], &SWAP_PROGRAM_ID);

            let (pool_mint_key, mut pool_mint_account) = create_mint(&TOKEN_PROGRAM_ID, &authority_key);
            let (pool_token_key, pool_token_account) = mint_token(
                &TOKEN_PROGRAM_ID,
                &pool_mint_key,
                &mut pool_mint_account,
                &authority_key,
                &user_key,
                0,
            );
            let (token_a_mint_key, mut token_a_mint_account) =
                create_mint(&TOKEN_PROGRAM_ID, &user_key);
            let (token_a_key, token_a_account) = mint_token(
                &TOKEN_PROGRAM_ID,
                &token_a_mint_key,
                &mut token_a_mint_account,
                &user_key,
                &authority_key,
                token_a_amount,
            );
            let (token_b_mint_key, mut token_b_mint_account) =
                create_mint(&TOKEN_PROGRAM_ID, &user_key);
            let (token_b_key, token_b_account) = mint_token(
                &TOKEN_PROGRAM_ID,
                &token_b_mint_key,
                &mut token_b_mint_account,
                &user_key,
                &authority_key,
                token_b_amount,
            );

            SwapAccountInfo {
                nonce,
                authority_key,
                fee_numerator,
                fee_denominator,
                swap_key,
                swap_account,
                pool_mint_key,
                pool_mint_account,
                pool_token_key,
                pool_token_account,
                token_a_key,
                token_a_account,
                token_a_mint_key,
                token_a_mint_account,
                token_b_key,
                token_b_account,
                token_b_mint_key,
                token_b_mint_account,
            }
        }

        pub fn initialize_swap(&mut self) -> ProgramResult {
            do_process_instruction(
                initialize(
                    &SWAP_PROGRAM_ID,
                    &TOKEN_PROGRAM_ID,
                    &self.swap_key,
                    &self.authority_key,
                    &self.token_a_key,
                    &self.token_b_key,
                    &self.pool_mint_key,
                    &self.pool_token_key,
                    self.nonce,
                    self.fee_numerator,
                    self.fee_denominator,
                )
                .unwrap(),
                vec![
                    &mut self.swap_account,
                    &mut Account::default(),
                    &mut self.token_a_account,
                    &mut self.token_b_account,
                    &mut self.pool_mint_account,
                    &mut self.pool_token_account,
                    &mut Account::default(),
                ],
            )
        }
    }

    fn mint_minimum_balance() -> u64 {
        Rent::default().minimum_balance(SplMint::get_packed_len())
    }

    fn account_minimum_balance() -> u64 {
        Rent::default().minimum_balance(SplAccount::get_packed_len())
    }

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
            Processor::process(&instruction.program_id, &account_infos, &instruction.data)
        } else {
            SplProcessor::process(&instruction.program_id, &account_infos, &instruction.data)
        }
    }

    fn mint_token(
        program_id: &Pubkey,
        mint_key: &Pubkey,
        mut mint_account: &mut Account,
        mint_authority_key: &Pubkey,
        account_owner_key: &Pubkey,
        amount: u64,
    ) -> (Pubkey, Account) {
        let account_key = pubkey_rand();
        let mut account_account = Account::new(
            account_minimum_balance(),
            SplAccount::get_packed_len(),
            &program_id,
        );
        let mut mint_authority_account = Account::default();
        let mut rent_sysvar_account = rent::create_account(1, &Rent::free());

        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, account_owner_key).unwrap(),
            vec![
                &mut account_account,
                &mut mint_account,
                &mut mint_authority_account,
                &mut rent_sysvar_account,
            ],
        )
        .unwrap();

        if amount > 0 {
            do_process_instruction(
                mint_to(
                    &program_id,
                    &mint_key,
                    &account_key,
                    &mint_authority_key,
                    &[],
                    amount,
                )
                .unwrap(),
                vec![
                    &mut mint_account,
                    &mut account_account,
                    &mut mint_authority_account,
                ],
            )
            .unwrap();
        }

        (account_key, account_account)
    }

    fn create_mint(program_id: &Pubkey, authority_key: &Pubkey) -> (Pubkey, Account) {
        let mint_key = pubkey_rand();
        let mut mint_account = Account::new(
            mint_minimum_balance(),
            SplMint::get_packed_len(),
            &program_id,
        );
        let mut rent_sysvar_account = rent::create_account(1, &Rent::free());

        do_process_instruction(
            initialize_mint(&program_id, &mint_key, authority_key, None, 2).unwrap(),
            vec![&mut mint_account, &mut rent_sysvar_account],
        )
        .unwrap();

        (mint_key, mint_account)
    }

    #[test]
    fn test_token_program_id_error() {
        let swap_key = pubkey_rand();
        let mut mint = (pubkey_rand(), Account::default());
        let mut destination = (pubkey_rand(), Account::default());
        let token_program = (TOKEN_PROGRAM_ID, Account::default());
        let (authority_key, nonce) =
            Pubkey::find_program_address(&[&swap_key.to_bytes()[..]], &SWAP_PROGRAM_ID);
        let mut authority = (authority_key, Account::default());
        let swap_bytes = swap_key.to_bytes();
        let authority_signature_seeds = [&swap_bytes[..32], &[nonce]];
        let signers = &[&authority_signature_seeds[..]];
        let ix = mint_to(
            &token_program.0,
            &mint.0,
            &destination.0,
            &authority.0,
            &[],
            10,
        )
        .unwrap();
        let mint = (&mut mint).into();
        let destination = (&mut destination).into();
        let authority = (&mut authority).into();

        let err = invoke_signed(&ix, &[mint, destination, authority], signers).unwrap_err();
        assert_eq!(err, ProgramError::InvalidAccountData);
    }


    #[test]
    fn test_initialize() {
        let user_key = pubkey_rand();
        let fee_numerator = 1;
        let fee_denominator = 2;
        let token_a_amount = 1000;
        let token_b_amount = 2000;
        let pool_token_amount = 10;

        let mut accounts = SwapAccountInfo::new(
            &user_key,
            fee_numerator,
            fee_denominator,
            token_a_amount,
            token_b_amount,
        );

        // wrong nonce for authority_key
        {
            let old_nonce = accounts.nonce;
            accounts.nonce = old_nonce - 1;
            assert_eq!(
                Err(SwapError::InvalidProgramAddress.into()),
                accounts.initialize_swap()
            );
            accounts.nonce = old_nonce;
        }

        // uninitialized token a account
        {
            let old_account = accounts.token_a_account;
            accounts.token_a_account = Account::default();
            assert_eq!(
                Err(SwapError::ExpectedAccount.into()),
                accounts.initialize_swap()
            );
            accounts.token_a_account = old_account;
        }

        // uninitialized token b account
        {
            let old_account = accounts.token_b_account;
            accounts.token_b_account = Account::default();
            assert_eq!(
                Err(SwapError::ExpectedAccount.into()),
                accounts.initialize_swap()
            );
            accounts.token_b_account = old_account;
        }

        // uninitialized pool mint
        {
            let old_account = accounts.pool_mint_account;
            accounts.pool_mint_account = Account::default();
            assert_eq!(
                Err(SwapError::ExpectedMint.into()),
                accounts.initialize_swap()
            );
            accounts.pool_mint_account = old_account;
        }

        // token A account owner is not swap authority
        {
            let (_token_a_key, token_a_account) = mint_token(
                &TOKEN_PROGRAM_ID,
                &accounts.token_a_mint_key,
                &mut accounts.token_a_mint_account,
                &user_key,
                &user_key,
                0,
            );
            let old_account = accounts.token_a_account;
            accounts.token_a_account = token_a_account;
            assert_eq!(
                Err(SwapError::InvalidOwner.into()),
                accounts.initialize_swap()
            );
            accounts.token_a_account = old_account;
        }

        // token B account owner is not swap authority
        {
            let (_token_b_key, token_b_account) = mint_token(
                &TOKEN_PROGRAM_ID,
                &accounts.token_b_mint_key,
                &mut accounts.token_b_mint_account,
                &user_key,
                &user_key,
                0,
            );
            let old_account = accounts.token_b_account;
            accounts.token_b_account = token_b_account;
            assert_eq!(
                Err(SwapError::InvalidOwner.into()),
                accounts.initialize_swap()
            );
            accounts.token_b_account = old_account;
        }

        // pool token account owner is swap authority
        {
            let (_pool_token_key, pool_token_account) = mint_token(
                &TOKEN_PROGRAM_ID,
                &accounts.pool_mint_key,
                &mut accounts.pool_mint_account,
                &accounts.authority_key,
                &accounts.authority_key,
                0,
            );
            let old_account = accounts.pool_token_account;
            accounts.pool_token_account = pool_token_account;
            assert_eq!(
                Err(SwapError::InvalidOutputOwner.into()),
                accounts.initialize_swap()
            );
            accounts.pool_token_account = old_account;
        }

        // pool mint authority is not swap authority
        {
            let (_pool_mint_key, pool_mint_account) = create_mint(&TOKEN_PROGRAM_ID, &user_key);
            let old_mint = accounts.pool_mint_account;
            accounts.pool_mint_account = pool_mint_account;
            assert_eq!(
                Err(SwapError::InvalidOwner.into()),
                accounts.initialize_swap()
            );
            accounts.pool_mint_account = old_mint;
        }

        // empty token A account
        {
            let (_token_a_key, token_a_account) = mint_token(
                &TOKEN_PROGRAM_ID,
                &accounts.token_a_mint_key,
                &mut accounts.token_a_mint_account,
                &user_key,
                &accounts.authority_key,
                0,
            );
            let old_account = accounts.token_a_account;
            accounts.token_a_account = token_a_account;
            assert_eq!(
                Err(SwapError::EmptySupply.into()),
                accounts.initialize_swap()
            );
            accounts.token_a_account = old_account;
        }

        // empty token B account
        {
            let (_token_b_key, token_b_account) = mint_token(
                &TOKEN_PROGRAM_ID,
                &accounts.token_b_mint_key,
                &mut accounts.token_b_mint_account,
                &user_key,
                &accounts.authority_key,
                0,
            );
            let old_account = accounts.token_b_account;
            accounts.token_b_account = token_b_account;
            assert_eq!(
                Err(SwapError::EmptySupply.into()),
                accounts.initialize_swap()
            );
            accounts.token_b_account = old_account;
        }

        // invalid pool tokens
        {
            let old_mint = accounts.pool_mint_account;
            let old_pool_account = accounts.pool_token_account;

            let (_pool_mint_key, pool_mint_account) = create_mint(&TOKEN_PROGRAM_ID, &accounts.authority_key);
            accounts.pool_mint_account = pool_mint_account;

            let (_empty_pool_token_key, empty_pool_token_account) = mint_token(
                &TOKEN_PROGRAM_ID,
                &accounts.pool_mint_key,
                &mut accounts.pool_mint_account,
                &accounts.authority_key,
                &user_key,
                0,
            );

            let (_pool_token_key, pool_token_account) = mint_token(
                &TOKEN_PROGRAM_ID,
                &accounts.pool_mint_key,
                &mut accounts.pool_mint_account,
                &accounts.authority_key,
                &user_key,
                pool_token_amount,
            );

            // non-empty pool token account
            accounts.pool_token_account = pool_token_account;
            assert_eq!(
                Err(SwapError::InvalidSupply.into()),
                accounts.initialize_swap()
            );

            // pool tokens already in circulation
            accounts.pool_token_account = empty_pool_token_account;
            assert_eq!(
                Err(SwapError::InvalidSupply.into()),
                accounts.initialize_swap()
            );

            accounts.pool_mint_account = old_mint;
            accounts.pool_token_account = old_pool_account;
        }

        // token A account is delegated
        {
            do_process_instruction(
                approve(
                    &TOKEN_PROGRAM_ID,
                    &accounts.token_a_key,
                    &user_key,
                    &accounts.authority_key,
                    &[],
                    1,
                )
                .unwrap(),
                vec![
                    &mut accounts.token_a_account,
                    &mut Account::default(),
                    &mut Account::default(),
                ],
            )
            .unwrap();
            assert_eq!(
                Err(SwapError::InvalidDelegate.into()),
                accounts.initialize_swap()
            );

            do_process_instruction(
                revoke(&TOKEN_PROGRAM_ID, &accounts.token_a_key, &accounts.authority_key, &[]).unwrap(),
                vec![&mut accounts.token_a_account, &mut Account::default()],
            )
            .unwrap();
        }

        // token B account is delegated
        {
            do_process_instruction(
                approve(
                    &TOKEN_PROGRAM_ID,
                    &accounts.token_b_key,
                    &user_key,
                    &accounts.authority_key,
                    &[],
                    1,
                )
                .unwrap(),
                vec![
                    &mut accounts.token_b_account,
                    &mut Account::default(),
                    &mut Account::default(),
                ],
            )
            .unwrap();
            assert_eq!(
                Err(SwapError::InvalidDelegate.into()),
                accounts.initialize_swap()
            );

            do_process_instruction(
                revoke(&TOKEN_PROGRAM_ID, &accounts.token_b_key, &accounts.authority_key, &[]).unwrap(),
                vec![&mut accounts.token_b_account, &mut Account::default()],
            )
            .unwrap();
        }

        // wrong token program id
        {
            let wrong_program_id = pubkey_rand();
            assert_eq!(
                Err(ProgramError::InvalidAccountData),
                do_process_instruction(
                    initialize(
                        &SWAP_PROGRAM_ID,
                        &wrong_program_id,
                        &accounts.swap_key,
                        &accounts.authority_key,
                        &accounts.token_a_key,
                        &accounts.token_b_key,
                        &accounts.pool_mint_key,
                        &accounts.pool_token_key,
                        accounts.nonce,
                        accounts.fee_numerator,
                        accounts.fee_denominator,
                    )
                    .unwrap(),
                    vec![
                        &mut accounts.swap_account,
                        &mut Account::default(),
                        &mut accounts.token_a_account,
                        &mut accounts.token_b_account,
                        &mut accounts.pool_mint_account,
                        &mut accounts.pool_token_account,
                        &mut Account::default(),
                    ],
                )
            );
        }

        // create swap with same token A and B
        {
            let (_token_a_repeat_key, token_a_repeat_account) = mint_token(
                &TOKEN_PROGRAM_ID,
                &accounts.token_a_mint_key,
                &mut accounts.token_a_mint_account,
                &user_key,
                &accounts.authority_key,
                10,
            );
            let old_account = accounts.token_b_account;
            accounts.token_b_account = token_a_repeat_account;
            assert_eq!(
                Err(SwapError::RepeatedMint.into()),
                accounts.initialize_swap()
            );
            accounts.token_b_account = old_account;
        }

        // create valid swap
        accounts.initialize_swap().unwrap();

        // create again
        {
            assert_eq!(
                Err(SwapError::AlreadyInUse.into()),
                accounts.initialize_swap()
            );
        }
        let swap_info = SwapInfo::unpack(&accounts.swap_account.data).unwrap();
        assert_eq!(swap_info.is_initialized, true);
        assert_eq!(swap_info.nonce, accounts.nonce);
        assert_eq!(swap_info.token_a, accounts.token_a_key);
        assert_eq!(swap_info.token_b, accounts.token_b_key);
        assert_eq!(swap_info.pool_mint, accounts.pool_mint_key);
        assert_eq!(swap_info.fee_denominator, fee_denominator);
        assert_eq!(swap_info.fee_numerator, fee_numerator);
        let token_a = Processor::unpack_token_account(&accounts.token_a_account.data).unwrap();
        assert_eq!(token_a.amount, token_a_amount);
        let token_b = Processor::unpack_token_account(&accounts.token_b_account.data).unwrap();
        assert_eq!(token_b.amount, token_b_amount);
        let pool_account =
            Processor::unpack_token_account(&accounts.pool_token_account.data).unwrap();
        let pool_mint = Processor::unpack_mint(&accounts.pool_mint_account.data).unwrap();
        assert_eq!(pool_mint.supply, pool_account.amount);
    }

    #[test]
    fn test_deposit() {
        let user_key = pubkey_rand();
        let depositor_key = pubkey_rand();
        let fee_numerator = 1;
        let fee_denominator = 2;
        let token_a_amount = 1000;
        let token_b_amount = 8000;
        let mut accounts = SwapAccountInfo::new(
            &user_key,
            fee_numerator,
            fee_denominator,
            token_a_amount,
            token_b_amount,
        );
        accounts.initialize_swap().unwrap();
        let seeds = [&accounts.swap_key.to_bytes()[..32], &[accounts.nonce]];
        let authority_key = Pubkey::create_program_address(&seeds, &SWAP_PROGRAM_ID).unwrap();

        // swap not initialized
        // not enough token A
        // not enough token B
        // wrong swap token accounts
        // wrong pool token account
        // wrong mint
        // no approval
        // wrong token program id
        // deposit max into both

        let deposit_a = token_a_amount / 10;
        let (depositor_token_a_key, mut depositor_token_a_account) = mint_token(
            &TOKEN_PROGRAM_ID,
            &accounts.token_a_mint_key,
            &mut accounts.token_a_mint_account,
            &user_key,
            &depositor_key,
            deposit_a,
        );
        let deposit_b = token_b_amount / 10;
        let (depositor_token_b_key, mut depositor_token_b_account) = mint_token(
            &TOKEN_PROGRAM_ID,
            &accounts.token_b_mint_key,
            &mut accounts.token_b_mint_account,
            &user_key,
            &depositor_key,
            deposit_b,
        );
        let initial_pool = 10;
        let (depositor_pool_key, mut depositor_pool_account) = mint_token(
            &TOKEN_PROGRAM_ID,
            &accounts.pool_mint_key,
            &mut accounts.pool_mint_account,
            &authority_key,
            &depositor_key,
            initial_pool,
        );

        // approve swap program to take out a / b tokens
        do_process_instruction(
            approve(
                &TOKEN_PROGRAM_ID,
                &depositor_token_a_key,
                &authority_key,
                &depositor_key,
                &[],
                deposit_a,
            )
            .unwrap(),
            vec![
                &mut depositor_token_a_account,
                &mut Account::default(),
                &mut Account::default(),
            ],
        )
        .unwrap();
        do_process_instruction(
            approve(
                &TOKEN_PROGRAM_ID,
                &depositor_token_b_key,
                &authority_key,
                &depositor_key,
                &[],
                deposit_b,
            )
            .unwrap(),
            vec![
                &mut depositor_token_b_account,
                &mut Account::default(),
                &mut Account::default(),
            ],
        )
        .unwrap();

        // deposit token a and b correctly
        do_process_instruction(
            deposit(
                &SWAP_PROGRAM_ID,
                &TOKEN_PROGRAM_ID,
                &accounts.swap_key,
                &authority_key,
                &depositor_token_a_key,
                &depositor_token_b_key,
                &accounts.token_a_key,
                &accounts.token_b_key,
                &accounts.pool_mint_key,
                &depositor_pool_key,
                deposit_a,
            )
            .unwrap(),
            vec![
                &mut accounts.swap_account,
                &mut Account::default(),
                &mut depositor_token_a_account,
                &mut depositor_token_b_account,
                &mut accounts.token_a_account,
                &mut accounts.token_b_account,
                &mut accounts.pool_mint_account,
                &mut depositor_pool_account,
                &mut Account::default(),
            ],
        )
        .unwrap();
        let token_a = Processor::unpack_token_account(&accounts.token_a_account.data).unwrap();
        assert_eq!(token_a.amount, deposit_a + token_a_amount);
        let token_b = Processor::unpack_token_account(&accounts.token_b_account.data).unwrap();
        assert_eq!(token_b.amount, deposit_b + token_b_amount);
        let depositor_token_a =
            Processor::unpack_token_account(&depositor_token_a_account.data).unwrap();
        assert_eq!(depositor_token_a.amount, 0);
        let depositor_token_b =
            Processor::unpack_token_account(&depositor_token_b_account.data).unwrap();
        assert_eq!(depositor_token_b.amount, 0);
        let depositor_pool_account =
            Processor::unpack_token_account(&depositor_pool_account.data).unwrap();
        let pool_account =
            Processor::unpack_token_account(&accounts.pool_token_account.data).unwrap();
        let pool_mint = Processor::unpack_mint(&accounts.pool_mint_account.data).unwrap();
        assert_eq!(
            pool_mint.supply,
            pool_account.amount + depositor_pool_account.amount
        );
    }

    #[test]
    fn test_withdraw() {
        let user_key = pubkey_rand();
        let fee_numerator = 1;
        let fee_denominator = 2;
        let token_a_amount = 1000;
        let token_b_amount = 2000;
        let mut accounts = SwapAccountInfo::new(
            &user_key,
            fee_numerator,
            fee_denominator,
            token_a_amount,
            token_b_amount,
        );
        accounts.initialize_swap().unwrap();
        let seeds = [&accounts.swap_key.to_bytes()[..32], &[accounts.nonce]];
        let authority_key = Pubkey::create_program_address(&seeds, &SWAP_PROGRAM_ID).unwrap();

        // swap not initialized
        // wrong token a / b accounts
        // not enough pool tokens
        // wrong pool token account
        // wrong mint
        // no approval
        // wrong token program id
        // withdraw everything

        let initial_a = token_a_amount / 10;
        let (withdraw_token_a_key, mut withdraw_token_a_account) = mint_token(
            &TOKEN_PROGRAM_ID,
            &accounts.token_a_mint_key,
            &mut accounts.token_a_mint_account,
            &user_key,
            &user_key,
            initial_a,
        );
        let initial_b = token_b_amount / 10;
        let (withdraw_token_b_key, mut withdraw_token_b_account) = mint_token(
            &TOKEN_PROGRAM_ID,
            &accounts.token_b_mint_key,
            &mut accounts.token_b_mint_account,
            &user_key,
            &user_key,
            initial_b,
        );

        let withdraw_amount = token_a_amount / 4;

        // approve swap program to take out pool tokens
        do_process_instruction(
            approve(
                &TOKEN_PROGRAM_ID,
                &accounts.pool_token_key,
                &authority_key,
                &user_key,
                &[],
                withdraw_amount,
            )
            .unwrap(),
            vec![
                &mut accounts.pool_token_account,
                &mut Account::default(),
                &mut Account::default(),
            ],
        )
        .unwrap();

        // withraw token a and b correctly
        do_process_instruction(
            withdraw(
                &SWAP_PROGRAM_ID,
                &TOKEN_PROGRAM_ID,
                &accounts.swap_key,
                &authority_key,
                &accounts.pool_mint_key,
                &accounts.pool_token_key,
                &accounts.token_a_key,
                &accounts.token_b_key,
                &withdraw_token_a_key,
                &withdraw_token_b_key,
                withdraw_amount,
            )
            .unwrap(),
            vec![
                &mut accounts.swap_account,
                &mut Account::default(),
                &mut accounts.pool_mint_account,
                &mut accounts.pool_token_account,
                &mut accounts.token_a_account,
                &mut accounts.token_b_account,
                &mut withdraw_token_a_account,
                &mut withdraw_token_b_account,
                &mut Account::default(),
            ],
        )
        .unwrap();

        let token_a = Processor::unpack_token_account(&accounts.token_a_account.data).unwrap();
        assert_eq!(token_a.amount, token_a_amount - withdraw_amount);
        let token_b = Processor::unpack_token_account(&accounts.token_b_account.data).unwrap();
        assert_eq!(token_b.amount, token_b_amount - (withdraw_amount * 2));
        let withdraw_token_a =
            Processor::unpack_token_account(&withdraw_token_a_account.data).unwrap();
        assert_eq!(withdraw_token_a.amount, initial_a + withdraw_amount);
        let withdraw_token_b =
            Processor::unpack_token_account(&withdraw_token_b_account.data).unwrap();
        assert_eq!(withdraw_token_b.amount, initial_b + (withdraw_amount * 2));
        let pool_account =
            Processor::unpack_token_account(&accounts.pool_token_account.data).unwrap();
        let pool_mint = Processor::unpack_mint(&accounts.pool_mint_account.data).unwrap();
        assert_eq!(pool_mint.supply, pool_account.amount);
    }

    #[test]
    fn test_swap() {
        let user_key = pubkey_rand();
        let fee_numerator = 1;
        let fee_denominator = 10;
        let token_a_amount = 1000;
        let token_b_amount = 5000;
        let mut accounts = SwapAccountInfo::new(
            &user_key,
            fee_numerator,
            fee_denominator,
            token_a_amount,
            token_b_amount,
        );
        accounts.initialize_swap().unwrap();
        let seeds = [&accounts.swap_key.to_bytes()[..32], &[accounts.nonce]];
        let authority_key = Pubkey::create_program_address(&seeds, &SWAP_PROGRAM_ID).unwrap();

        // swap not initialized
        // not enough token to swap
        // swap all the rest of token A
        // wrong user token A / B accounts
        // wrong swap token A / B accounts
        // no approval
        // wrong token program id

        let initial_a = token_a_amount / 5;
        let (user_token_a_key, mut user_token_a_account) = mint_token(
            &TOKEN_PROGRAM_ID,
            &accounts.token_a_mint_key,
            &mut accounts.token_a_mint_account,
            &user_key,
            &user_key,
            initial_a,
        );
        let initial_b = token_b_amount / 5;
        let (user_token_b_key, mut user_token_b_account) = mint_token(
            &TOKEN_PROGRAM_ID,
            &accounts.token_b_mint_key,
            &mut accounts.token_b_mint_account,
            &user_key,
            &user_key,
            initial_b,
        );

        let a_to_b_amount = initial_a / 10;
        // approve swap program to take out token a
        do_process_instruction(
            approve(
                &TOKEN_PROGRAM_ID,
                &user_token_a_key,
                &authority_key,
                &user_key,
                &[],
                a_to_b_amount,
            )
            .unwrap(),
            vec![
                &mut user_token_a_account,
                &mut Account::default(),
                &mut Account::default(),
            ],
        )
        .unwrap();

        do_process_instruction(
            swap(
                &SWAP_PROGRAM_ID,
                &TOKEN_PROGRAM_ID,
                &accounts.swap_key,
                &authority_key,
                &user_token_a_key,
                &accounts.token_a_key,
                &accounts.token_b_key,
                &user_token_b_key,
                a_to_b_amount,
            )
            .unwrap(),
            vec![
                &mut accounts.swap_account,
                &mut Account::default(),
                &mut user_token_a_account,
                &mut accounts.token_a_account,
                &mut accounts.token_b_account,
                &mut user_token_b_account,
                &mut Account::default(),
            ],
        )
        .unwrap();

        let results = SwapResult::swap_to(
            a_to_b_amount,
            token_a_amount,
            token_b_amount,
            fee_numerator,
            fee_denominator,
        )
        .unwrap();

        let swap_token_a = Processor::unpack_token_account(&accounts.token_a_account.data).unwrap();
        let token_a_amount = swap_token_a.amount;
        assert_eq!(token_a_amount, results.new_source);
        let user_token_a = Processor::unpack_token_account(&user_token_a_account.data).unwrap();
        assert_eq!(user_token_a.amount, initial_a - a_to_b_amount);

        let swap_token_b = Processor::unpack_token_account(&accounts.token_b_account.data).unwrap();
        let token_b_amount = swap_token_b.amount;
        assert_eq!(token_b_amount, results.new_destination);
        let user_token_b = Processor::unpack_token_account(&user_token_b_account.data).unwrap();
        assert_eq!(user_token_b.amount, initial_b + results.amount_swapped);

        let first_swap_amount = results.amount_swapped;

        let b_to_a_amount = initial_b / 10;
        // approve swap program to take out token a
        do_process_instruction(
            approve(
                &TOKEN_PROGRAM_ID,
                &user_token_b_key,
                &authority_key,
                &user_key,
                &[],
                b_to_a_amount,
            )
            .unwrap(),
            vec![
                &mut user_token_b_account,
                &mut Account::default(),
                &mut Account::default(),
            ],
        )
        .unwrap();

        do_process_instruction(
            swap(
                &SWAP_PROGRAM_ID,
                &TOKEN_PROGRAM_ID,
                &accounts.swap_key,
                &authority_key,
                &user_token_b_key,
                &accounts.token_b_key,
                &accounts.token_a_key,
                &user_token_a_key,
                b_to_a_amount,
            )
            .unwrap(),
            vec![
                &mut accounts.swap_account,
                &mut Account::default(),
                &mut user_token_b_account,
                &mut accounts.token_b_account,
                &mut accounts.token_a_account,
                &mut user_token_a_account,
                &mut Account::default(),
            ],
        )
        .unwrap();

        let results = SwapResult::swap_to(
            b_to_a_amount,
            token_b_amount,
            token_a_amount,
            fee_numerator,
            fee_denominator,
        )
        .unwrap();

        let swap_token_a = Processor::unpack_token_account(&accounts.token_a_account.data).unwrap();
        assert_eq!(swap_token_a.amount, results.new_destination);
        let user_token_a = Processor::unpack_token_account(&user_token_a_account.data).unwrap();
        assert_eq!(
            user_token_a.amount,
            initial_a - a_to_b_amount + results.amount_swapped
        );

        let swap_token_b = Processor::unpack_token_account(&accounts.token_b_account.data).unwrap();
        assert_eq!(swap_token_b.amount, results.new_source);
        let user_token_b = Processor::unpack_token_account(&user_token_b_account.data).unwrap();
        assert_eq!(
            user_token_b.amount,
            initial_b + first_swap_amount - b_to_a_amount
        );
    }
}
