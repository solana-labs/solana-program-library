//! Program state processor

#![cfg(feature = "program")]

use crate::{
    error::Error,
    instruction::{unpack, Fee, SwapInstruction},
    state::{Invariant, State, SwapInfo},
};
use num_traits::FromPrimitive;
#[cfg(not(target_arch = "bpf"))]
use solana_sdk::instruction::Instruction;
#[cfg(target_arch = "bpf")]
use solana_sdk::program::invoke_signed;
use solana_sdk::{
    account_info::next_account_info, account_info::AccountInfo, decode_error::DecodeError,
    entrypoint::ProgramResult, info, program_error::PrintProgramError, program_error::ProgramError,
    pubkey::Pubkey,
};
use spl_token::pack::Pack;
use std::mem::size_of;

impl State {
    /// Deserializes a byte buffer into a [State](struct.State.html).
    pub fn deserialize(input: &[u8]) -> Result<Self, ProgramError> {
        if input.len() < size_of::<u8>() {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(match input[0] {
            0 => Self::Unallocated,
            1 => {
                let swap: &SwapInfo = unpack(input)?;
                Self::Init(*swap)
            }
            _ => return Err(ProgramError::InvalidAccountData),
        })
    }

    /// Serializes [State](struct.State.html) into a byte buffer.
    pub fn serialize(&self, output: &mut [u8]) -> ProgramResult {
        if output.len() < size_of::<u8>() {
            return Err(ProgramError::InvalidAccountData);
        }
        match self {
            Self::Unallocated => output[0] = 0,
            Self::Init(swap) => {
                if output.len() < size_of::<u8>() + size_of::<SwapInfo>() {
                    return Err(ProgramError::InvalidAccountData);
                }
                output[0] = 1;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut SwapInfo) };
                *value = *swap;
            }
        }
        Ok(())
    }

    /// Gets the `SwapInfo` from `State`
    fn token_swap(&self) -> Result<SwapInfo, ProgramError> {
        if let State::Init(swap) = &self {
            Ok(*swap)
        } else {
            Err(Error::InvalidState.into())
        }
    }

    /// Deserializes a spl_token `Account`.
    pub fn token_account_deserialize(
        info: &AccountInfo,
    ) -> Result<spl_token::state::Account, Error> {
        spl_token::state::Account::unpack(&info.data.borrow_mut())
            .map_err(|_| Error::ExpectedAccount)
    }

    /// Deserializes a spl_token `Mint`.
    pub fn mint_deserialize(info: &AccountInfo) -> Result<spl_token::state::Mint, Error> {
        spl_token::state::Mint::unpack(&info.data.borrow_mut()).map_err(|_| Error::ExpectedAccount)
    }

    /// Calculates the authority id by generating a program address.
    pub fn authority_id(program_id: &Pubkey, my_info: &Pubkey) -> Result<Pubkey, Error> {
        Pubkey::create_program_address(&[&my_info.to_bytes()[..32]], program_id)
            .or(Err(Error::InvalidProgramAddress))
    }
    /// Issue a spl_token `Burn` instruction.
    pub fn token_burn(
        accounts: &[AccountInfo],
        token_program_id: &Pubkey,
        swap: &Pubkey,
        burn_account: &Pubkey,
        mint: &Pubkey,
        authority: &Pubkey,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let swap_bytes = swap.to_bytes();
        let signers = &[&[&swap_bytes[..32]][..]];
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
        let swap_bytes = swap.to_bytes();
        let signers = &[&[&swap_bytes[..32]][..]];
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

    /// Issue a spl_token `Transfer` instruction.
    pub fn token_transfer(
        accounts: &[AccountInfo],
        token_program_id: &Pubkey,
        swap: &Pubkey,
        source: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let swap_bytes = swap.to_bytes();
        let signers = &[&[&swap_bytes[..32]][..]];
        let ix = spl_token::instruction::transfer(
            token_program_id,
            source,
            destination,
            authority,
            &[],
            amount,
        )?;
        invoke_signed(&ix, accounts, signers)
    }

    /// Processes an [Initialize](enum.Instruction.html).
    pub fn process_initialize(
        program_id: &Pubkey,
        fee: Fee,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let token_a_info = next_account_info(account_info_iter)?;
        let token_b_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let user_output_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        if State::Unallocated != State::deserialize(&swap_info.data.borrow())? {
            return Err(Error::AlreadyInUse.into());
        }

        if *authority_info.key != Self::authority_id(program_id, swap_info.key)? {
            return Err(Error::InvalidProgramAddress.into());
        }
        let token_a = Self::token_account_deserialize(token_a_info)?;
        let token_b = Self::token_account_deserialize(token_b_info)?;
        let pool_mint = Self::mint_deserialize(pool_info)?;
        if *authority_info.key != token_a.owner {
            return Err(Error::InvalidOwner.into());
        }
        if *authority_info.key != token_b.owner {
            return Err(Error::InvalidOwner.into());
        }
        if spl_token::option::COption::Some(*authority_info.key) != pool_mint.mint_authority {
            return Err(Error::InvalidOwner.into());
        }
        if token_b.amount == 0 {
            return Err(Error::InvalidSupply.into());
        }
        if token_a.amount == 0 {
            return Err(Error::InvalidSupply.into());
        }
        if token_a.delegate.is_some() {
            return Err(Error::InvalidDelegate.into());
        }
        if token_b.delegate.is_some() {
            return Err(Error::InvalidDelegate.into());
        }

        // liquidity is measured in terms of token_a's value since both sides of
        // the pool are equal
        let amount = token_a.amount;
        Self::token_mint_to(
            accounts,
            token_program_info.key,
            swap_info.key,
            pool_info.key,
            user_output_info.key,
            authority_info.key,
            amount,
        )?;

        let obj = State::Init(SwapInfo {
            token_a: *token_a_info.key,
            token_b: *token_b_info.key,
            pool_mint: *pool_info.key,
            fee,
        });
        obj.serialize(&mut swap_info.data.borrow_mut())
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
        let into_info = next_account_info(account_info_iter)?;
        let from_info = next_account_info(account_info_iter)?;
        let dest_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let token_swap = Self::deserialize(&swap_info.data.borrow())?.token_swap()?;

        if *authority_info.key != Self::authority_id(program_id, swap_info.key)? {
            return Err(Error::InvalidProgramAddress.into());
        }
        if !(*into_info.key == token_swap.token_a || *into_info.key == token_swap.token_b) {
            return Err(Error::InvalidInput.into());
        }
        if !(*from_info.key == token_swap.token_a || *from_info.key == token_swap.token_b) {
            return Err(Error::InvalidOutput.into());
        }
        if *into_info.key == *from_info.key {
            return Err(Error::InvalidInput.into());
        }
        let into_token = Self::token_account_deserialize(into_info)?;
        let from_token = Self::token_account_deserialize(from_info)?;
        let mut invariant = Invariant {
            token_a: into_token.amount,
            token_b: from_token.amount,
            fee: token_swap.fee,
        };
        let output = invariant
            .swap(amount)
            .ok_or_else(|| Error::CalculationFailure)?;
        Self::token_transfer(
            accounts,
            token_program_info.key,
            swap_info.key,
            source_info.key,
            into_info.key,
            authority_info.key,
            amount,
        )?;
        Self::token_transfer(
            accounts,
            token_program_info.key,
            swap_info.key,
            from_info.key,
            dest_info.key,
            authority_info.key,
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

        let token_swap = Self::deserialize(&swap_info.data.borrow())?.token_swap()?;
        if *authority_info.key != Self::authority_id(program_id, swap_info.key)? {
            return Err(Error::InvalidProgramAddress.into());
        }
        if *token_a_info.key != token_swap.token_a {
            return Err(Error::InvalidInput.into());
        }
        if *token_b_info.key != token_swap.token_b {
            return Err(Error::InvalidInput.into());
        }
        if *pool_info.key != token_swap.pool_mint {
            return Err(Error::InvalidInput.into());
        }
        let token_a = Self::token_account_deserialize(token_a_info)?;
        let token_b = Self::token_account_deserialize(token_b_info)?;

        let invariant = Invariant {
            token_a: token_a.amount,
            token_b: token_b.amount,
            fee: token_swap.fee,
        };
        let b_amount = invariant
            .exchange_rate(a_amount)
            .ok_or_else(|| Error::CalculationFailure)?;

        // liquidity is measured in terms of token_a's value
        // since both sides of the pool are equal
        let output = a_amount;

        Self::token_transfer(
            accounts,
            token_program_info.key,
            swap_info.key,
            source_a_info.key,
            token_a_info.key,
            authority_info.key,
            a_amount,
        )?;
        Self::token_transfer(
            accounts,
            token_program_info.key,
            swap_info.key,
            source_b_info.key,
            token_b_info.key,
            authority_info.key,
            b_amount,
        )?;
        Self::token_mint_to(
            accounts,
            token_program_info.key,
            swap_info.key,
            pool_info.key,
            dest_info.key,
            authority_info.key,
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
        let source_info = next_account_info(account_info_iter)?;
        let token_a_info = next_account_info(account_info_iter)?;
        let token_b_info = next_account_info(account_info_iter)?;
        let dest_token_a_info = next_account_info(account_info_iter)?;
        let dest_token_b_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let token_swap = Self::deserialize(&swap_info.data.borrow())?.token_swap()?;
        if *authority_info.key != Self::authority_id(program_id, swap_info.key)? {
            return Err(Error::InvalidProgramAddress.into());
        }
        if *token_a_info.key != token_swap.token_a {
            return Err(Error::InvalidInput.into());
        }
        if *token_b_info.key != token_swap.token_b {
            return Err(Error::InvalidInput.into());
        }

        let token_a = Self::token_account_deserialize(token_a_info)?;
        let token_b = Self::token_account_deserialize(token_b_info)?;

        let invariant = Invariant {
            token_a: token_a.amount,
            token_b: token_b.amount,
            fee: token_swap.fee,
        };

        let a_amount = amount;
        let b_amount = invariant
            .exchange_rate(a_amount)
            .ok_or_else(|| Error::CalculationFailure)?;

        Self::token_transfer(
            accounts,
            token_program_info.key,
            swap_info.key,
            token_a_info.key,
            dest_token_a_info.key,
            authority_info.key,
            a_amount,
        )?;
        Self::token_transfer(
            accounts,
            token_program_info.key,
            swap_info.key,
            token_b_info.key,
            dest_token_b_info.key,
            authority_info.key,
            b_amount,
        )?;
        Self::token_burn(
            accounts,
            token_program_info.key,
            swap_info.key,
            source_info.key,
            &token_swap.pool_mint,
            authority_info.key,
            amount,
        )?;
        Ok(())
    }
    /// Processes an [Instruction](enum.Instruction.html).
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        let instruction = SwapInstruction::deserialize(input)?;
        match instruction {
            SwapInstruction::Initialize(fee) => {
                info!("Instruction: Init");
                Self::process_initialize(program_id, fee, accounts)
            }
            SwapInstruction::Swap(amount) => {
                info!("Instruction: Swap");
                Self::process_swap(program_id, amount, accounts)
            }
            SwapInstruction::Deposit(amount) => {
                info!("Instruction: Deposit");
                Self::process_deposit(program_id, amount, accounts)
            }
            SwapInstruction::Withdraw(amount) => {
                info!("Instruction: Withdraw");
                Self::process_withdraw(program_id, amount, accounts)
            }
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
                    let signer = Pubkey::create_program_address(seeds, &SWAP_PROGRAM_ID).unwrap();
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
        instruction::{initialize_account, initialize_mint, mint_to},
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
            initialize_mint(&program_id, &token_key, authority_key, None, 2).unwrap(),
            vec![&mut token_account, &mut authority_account],
        )
        .unwrap();

        do_process_instruction(
            mint_to(
                &program_id,
                &token_key,
                &account_key,
                authority_key,
                &[],
                amount,
            )
            .unwrap(),
            vec![
                &mut token_account,
                &mut account_account,
                &mut authority_account,
            ],
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

        // Swap Init
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
