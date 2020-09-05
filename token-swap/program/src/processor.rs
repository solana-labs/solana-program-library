//! Program state processor

#![cfg(feature = "program")]

use crate::{
    error::Error,
    instruction::{Fee, SwapInstruction},
    state::{Invariant, State, SwapInfo},
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
    pub fn authority_id(program_id: &Pubkey, my_info: &Pubkey, nonce: u8) -> Result<Pubkey, Error> {
        Pubkey::create_program_address(&[&my_info.to_bytes()[..32], &[nonce]], program_id)
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
        nonce: u8,
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

        if *authority_info.key != Self::authority_id(program_id, swap_info.key, nonce)? {
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
            swap_info.key,
            token_program_info.clone(),
            pool_info.clone(),
            user_output_info.clone(),
            authority_info.clone(),
            nonce,
            amount,
        )?;

        let obj = State::Init(SwapInfo {
            nonce,
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

        let token_swap = State::deserialize(&swap_info.data.borrow())?.token_swap()?;

        if *authority_info.key != Self::authority_id(program_id, swap_info.key, token_swap.nonce)? {
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
        let output = invariant.swap(amount).ok_or(Error::CalculationFailure)?;
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

        let token_swap = State::deserialize(&swap_info.data.borrow())?.token_swap()?;
        if *authority_info.key != Self::authority_id(program_id, swap_info.key, token_swap.nonce)? {
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
            .ok_or(Error::CalculationFailure)?;

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
        let source_info = next_account_info(account_info_iter)?;
        let token_a_info = next_account_info(account_info_iter)?;
        let token_b_info = next_account_info(account_info_iter)?;
        let dest_token_a_info = next_account_info(account_info_iter)?;
        let dest_token_b_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let token_swap = State::deserialize(&swap_info.data.borrow())?.token_swap()?;
        if *authority_info.key != Self::authority_id(program_id, swap_info.key, token_swap.nonce)? {
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
            .ok_or(Error::CalculationFailure)?;

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
            SwapInstruction::Initialize(init_data) => {
                info!("Instruction: Init");
                let fee = Fee {
                    numerator: init_data.fee_numerator,
                    denominator: init_data.fee_denominator,
                };
                Self::process_initialize(program_id, init_data.nonce, fee, accounts)
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
        rent::Rent, sysvar::rent,
    };
    use spl_token::{
        instruction::{initialize_account, initialize_mint, mint_to},
        pack::Pack,
        processor::Processor as SplProcessor,
        state::{Account as SplAccount, Mint as SplMint},
    };
    use std::mem::size_of;

    const SWAP_INDEX: usize = 0;
    const TOKEN_A_INDEX: usize = 1;
    const TOKEN_B_INDEX: usize = 2;
    const POOL_MINT_INDEX: usize = 3;
    const POOL_INDEX: usize = 4;
    const AUTHORITY_INDEX: usize = 5;

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
        authority_key: &Pubkey,
        amount: u64,
    ) -> ((Pubkey, Account), (Pubkey, Account)) {
        let token_key = pubkey_rand();
        let mut token_account = Account::new(
            mint_minimum_balance(),
            SplMint::get_packed_len(),
            &program_id,
        );
        let account_key = pubkey_rand();
        let mut account_account = Account::new(
            account_minimum_balance(),
            SplAccount::get_packed_len(),
            &program_id,
        );
        let mut rent_sysvar_account = rent::create_account(1, &Rent::free());

        // create token mint
        do_process_instruction(
            initialize_mint(&program_id, &token_key, authority_key, None, 2).unwrap(),
            vec![&mut token_account, &mut rent_sysvar_account],
        )
        .unwrap();

        let mut authority_account = Account::default();

        // create account
        do_process_instruction(
            initialize_account(&program_id, &account_key, &token_key, authority_key).unwrap(),
            vec![
                &mut account_account,
                &mut token_account,
                &mut authority_account,
                &mut rent_sysvar_account,
            ],
        )
        .unwrap();

        do_process_instruction(
            mint_to(
                &program_id,
                &token_key,
                &account_key,
                &authority_key,
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
        let ix = spl_token::instruction::mint_to(
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

    fn initialize_swap(numerator: u64, denominator: u64) -> (u8, Vec<(Pubkey, Account)>) {
        let mut account_infos = vec![];
        let swap_key = pubkey_rand();
        let mut swap_account = Account::new(0, size_of::<State>(), &SWAP_PROGRAM_ID);
        let (authority_key, nonce) =
            Pubkey::find_program_address(&[&swap_key.to_bytes()[..]], &SWAP_PROGRAM_ID);

        let ((pool_key, mut pool_account), (pool_token_key, mut pool_token_account)) =
            mint_token(&TOKEN_PROGRAM_ID, &authority_key, 0);
        let ((_token_a_mint_key, mut _token_a_mint_account), (token_a_key, mut token_a_account)) =
            mint_token(&TOKEN_PROGRAM_ID, &authority_key, 1000);
        let ((_token_b_mint_key, mut _token_b_mint_account), (token_b_key, mut token_b_account)) =
            mint_token(&TOKEN_PROGRAM_ID, &authority_key, 1000);

        // Swap Init
        let mut authority_account = Account::default();
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
                nonce,
                Fee {
                    denominator,
                    numerator,
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
        account_infos.insert(SWAP_INDEX, (swap_key, swap_account));
        account_infos.insert(TOKEN_A_INDEX, (token_a_key, token_a_account));
        account_infos.insert(TOKEN_B_INDEX, (token_b_key, token_b_account));
        account_infos.insert(POOL_MINT_INDEX, (pool_key, pool_account));
        account_infos.insert(POOL_INDEX, (pool_token_key, pool_token_account));
        account_infos.insert(AUTHORITY_INDEX, (authority_key, authority_account));
        (nonce, account_infos)
    }

    #[test]
    fn test_initialize() {
        let numerator = 1;
        let denominator = 2;
        let (nonce, account_infos) = initialize_swap(numerator, denominator);
        let state = State::deserialize(&account_infos[SWAP_INDEX].1.data).unwrap();
        match state {
            State::Init(swap_info) => {
                assert_eq!(swap_info.nonce, nonce);
                assert_eq!(swap_info.token_a, account_infos[TOKEN_A_INDEX].0);
                assert_eq!(swap_info.token_b, account_infos[TOKEN_B_INDEX].0);
                assert_eq!(swap_info.pool_mint, account_infos[POOL_MINT_INDEX].0);
                assert_eq!(swap_info.fee.denominator, denominator);
                assert_eq!(swap_info.fee.numerator, numerator);
            }
            _ => {
                panic!("Incorrect state");
            }
        }
    }
}
