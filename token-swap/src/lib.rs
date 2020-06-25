extern crate spl_token;

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
#[cfg(target_arch = "bpf")]
use solana_sdk::program::invoke_signed;
use solana_sdk::{
    account_info::AccountInfo,
    entrypoint,
    entrypoint::ProgramResult,
    info,
    instruction::{AccountMeta, Instruction},
    program_error::{PrintProgramError, ProgramError},
    program_utils::{next_account_info, DecodeError},
    pubkey::Pubkey,
};
use std::mem::size_of;
use thiserror::Error;

// TODO update instruction documentation
/// Instructions supported by the TokenSwap program.
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum SwapInstruction {
    ///   Initializes a new TokenSwap.
    ///
    ///   0. `[writable, signer]` New Token-swap to create.
    ///   1. `[]` $authority derived from `create_program_address(&[Token-swap account])`
    ///   2. `[]` token_a Account. Must be non zero, owned by $authority.
    ///   3. `[]` token_b Account. Must be non zero, owned by $authority.
    ///   4. `[writable]` pool Token. Must be empty, owned by $authority.
    ///   5. `[writable]` Pool Account to deposit the generated tokens, user is the owner.
    ///   6. '[]` Token program id
    ///   userdata: fee rate as a ratio
    Init((u64, u64)),

    ///   Swap the tokens in the pool.
    ///
    ///   0. `[]` Token-swap
    ///   1. `[]` $authority
    ///   2. `[writable]` token_(A|B) SOURCE delegate Account, amount is transferable by $authority,
    ///   3. `[writable]` token_(A|B) SOURCE Account associated with the delegate
    ///   4. `[writable]` token_(A|B) Base Account to swap INTO.  Must be the SOURCE token.
    ///   5. `[writable]` token_(A|B) Base Account to swap FROM.  Must be the DEST token.
    ///   6. `[writable]` token_(A|B) DEST Account assigned to USER as the owner.
    ///   7. '[]` Token program id
    ///   userdata: SOURCE amount to transfer, output to DEST is based on the exchange rate
    Swap(u64),

    ///   Deposit some tokens into the pool.  The output is a "pool" token representing ownership
    ///   into the pool. Inputs are converted to the current ratio.
    ///
    ///   0. `[]` Token-swap
    ///   1. `[]` $authority
    ///   2. `[writable]` token_a delegate $authority can transfer amount,
    ///   3. `[writable]` token_a account associated with delegate
    ///   4. `[writable]` token_b delegate $authority can transfer amount,
    ///   5. `[writable]` token_b account associated with delegate
    ///   6. `[writable]` token_a Base Account to deposit into.
    ///   7. `[writable]` token_b Base Account to deposit into.
    ///   8. `[writable]` Pool MINT account, $authority is the owner.
    ///   9. `[writable]` Pool Account to deposit the generated tokens, user is the owner.
    ///   10. '[]` Token program id
    ///   userdata: token_a amount to transfer.  token_b amount is set by the current exchange rate.
    Deposit(u64),

    ///   Withdraw the token from the pool at the current ratio.
    ///   
    ///   0. `[]` Token-swap
    ///   1. `[]` $authority
    ///   2. `[writable]` SOURCE Pool delegate, amount is transferable by $authority.
    ///   3. `[writable]` SOURCE Pool account associated with the delegate
    ///   4. `[writable]` Pool MINT account, $authority is the owner.
    ///   5. `[writable]` token_a Account to withdraw FROM.
    ///   6. `[writable]` token_b Account to withdraw FROM.
    ///   7. `[writable]` token_a user Account.
    ///   8. `[writable]` token_b user Account.
    ///   9. '[]` Token program id
    ///   userdata: SOURCE amount of pool tokens to transfer. User receives an output based on the
    ///   percentage of the pool tokens that are returned.
    Withdraw(u64),
}

/// Creates an 'Init' instruction
pub fn init(
    program_id: &Pubkey,
    token_program_id: &Pubkey,
    swap_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    token_a_pubkey: &Pubkey,
    token_b_pubkey: &Pubkey,
    pool_pubkey: &Pubkey,
    user_output_pubkey: &Pubkey,
    fees: (u64, u64),
) -> Result<Instruction, ProgramError> {
    let data = SwapInstruction::Init(fees).serialize()?;

    let accounts = vec![
        AccountMeta::new(*swap_pubkey, true),
        AccountMeta::new(*authority_pubkey, false),
        AccountMeta::new(*token_a_pubkey, false),
        AccountMeta::new(*token_b_pubkey, false),
        AccountMeta::new(*pool_pubkey, false),
        AccountMeta::new(*user_output_pubkey, false),
        AccountMeta::new(*token_program_id, false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

pub fn unpack<T>(input: &[u8]) -> Result<&T, ProgramError> {
    if input.len() < size_of::<u8>() + size_of::<T>() {
        return Err(ProgramError::InvalidAccountData);
    }
    #[allow(clippy::cast_ptr_alignment)]
    let val: &T = unsafe { &*(&input[1] as *const u8 as *const T) };
    Ok(val)
}

impl SwapInstruction {
    /// Deserializes a byte buffer into an [SwapInstruction](enum.SwapInstruction.html)
    pub fn deserialize(input: &[u8]) -> Result<Self, ProgramError> {
        if input.len() < size_of::<u8>() {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(match input[0] {
            0 => {
                let fee: &(u64, u64) = unpack(input)?;
                Self::Init(*fee)
            }
            1 => {
                let fee: &u64 = unpack(input)?;
                Self::Swap(*fee)
            }
            2 => {
                let fee: &u64 = unpack(input)?;
                Self::Deposit(*fee)
            }
            3 => {
                let fee: &u64 = unpack(input)?;
                Self::Withdraw(*fee)
            }
            _ => return Err(ProgramError::InvalidAccountData),
        })
    }

    /// Serializes an [SwapInstruction](enum.SwapInstruction.html) into a byte buffer
    pub fn serialize(self: &Self) -> Result<Vec<u8>, ProgramError> {
        let mut output = vec![0u8; size_of::<SwapInstruction>()];
        match self {
            Self::Init(fees) => {
                output[0] = 0;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut (u64, u64)) };
                *value = *fees;
            }
            Self::Swap(amount) => {
                output[0] = 1;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut u64) };
                *value = *amount;
            }
            Self::Deposit(amount) => {
                output[0] = 2;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut u64) };
                *value = *amount;
            }
            Self::Withdraw(amount) => {
                output[0] = 3;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut u64) };
                *value = *amount;
            }
        }
        Ok(output)
    }
}

#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum Error {
    /// The account cannot be initialized because it is already being used.
    #[error("AlreadyInUse")]
    AlreadyInUse,
    /// The program address provided doesn't match the value generated by the program.
    #[error("InvalidProgramAddress")]
    InvalidProgramAddress,
    /// The owner of the input isn't set to the program address generated by the program.
    #[error("InvalidOwner")]
    InvalidOwner,
    /// The deserialization of the Token state returned something besides State::Token
    #[error("ExpectedToken")]
    ExpectedToken,
    /// The deserialization of the Token state returned something besides State::Account
    #[error("ExpectedAccount")]
    ExpectedAccount,
    /// The initialized pool had a non zero supply
    #[error("InvalidSupply")]
    InvalidSupply,
    /// The intiailized token has a delegate
    #[error("InvalidDelegate")]
    InvalidDelegate,
    /// The token swap state is invalid
    #[error("InvalidState")]
    InvalidState,
    /// The input token is invalid for swap
    #[error("InvalidInput")]
    InvalidInput,
    /// The output token is invalid for swap
    #[error("InvalidOutput")]
    InvalidOutput,
    /// The calculation failed
    #[error("CalculationFailure")]
    CalculationFailure,
}
impl From<Error> for ProgramError {
    fn from(e: Error) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl<T> DecodeError<T> for Error {
    fn type_of() -> &'static str {
        "Swap Error"
    }
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

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TokenSwap {
    /// token A
    /// The Liquidity token is issued against this value.
    token_a: Pubkey,
    /// token B
    token_b: Pubkey,
    /// pool tokens are issued when A or B tokens are deposited
    /// pool tokens can be withdrawn back to the original A or B token
    pool_mint: Pubkey,
    /// fee applied to the input token amount prior to output calculation
    fee: (u64, u64),
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// Unallocated state, may be initialized into another state.
    Unallocated,
    Init(TokenSwap),
}

struct Invariant {
    token_a: u64,
    token_b: u64,
    pool: Option<u64>,
    fee: (u64, u64),
}

impl Invariant {
    fn swap(&mut self, token_a: u64) -> Option<u64> {
        let invariant = self.token_a.checked_mul(self.token_b)?;
        let new_a = self.token_a.checked_add(token_a)?;
        let new_b = invariant.checked_div(new_a)?;
        let remove = self.token_b.checked_sub(new_b)?;
        let fee = remove.checked_mul(self.fee.1)?.checked_div(self.fee.0)?;
        let new_b_with_fee = new_b.checked_add(fee)?;
        let remove_less_fee = remove.checked_sub(fee)?;
        self.token_a = new_a;
        self.token_b = new_b_with_fee;
        Some(remove_less_fee)
    }
    fn exchange_rate(&self, token_a: u64) -> Option<u64> {
        token_a.checked_mul(self.token_b)?.checked_div(self.token_a)
    }
    fn redeem(&self, user_pool: u64) -> Option<(u64, u64)> {
        let token_a = self
            .token_a
            .checked_mul(user_pool)?
            .checked_div(self.pool?)?;
        let token_b = self
            .token_b
            .checked_mul(user_pool)?
            .checked_div(self.pool?)?;
        Some((token_a, token_b))
    }
}

impl State {
    pub fn deserialize(input: &[u8]) -> Result<Self, ProgramError> {
        if input.len() < size_of::<u8>() {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(match input[0] {
            0 => Self::Unallocated,
            1 => {
                let swap: &TokenSwap = unpack(input)?;
                Self::Init(*swap)
            }
            _ => return Err(ProgramError::InvalidAccountData),
        })
    }

    pub fn serialize(self: &Self, output: &mut [u8]) -> ProgramResult {
        if output.len() < size_of::<u8>() {
            return Err(ProgramError::InvalidAccountData);
        }
        match self {
            Self::Unallocated => output[0] = 0,
            Self::Init(swap) => {
                if output.len() < size_of::<u8>() + size_of::<TokenSwap>() {
                    return Err(ProgramError::InvalidAccountData);
                }
                output[0] = 1;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut TokenSwap) };
                *value = *swap;
            }
        }
        Ok(())
    }

    fn token_swap(&self) -> Result<TokenSwap, ProgramError> {
        if let State::Init(swap) = &self {
            Ok(*swap)
        } else {
            Err(Error::InvalidState.into())
        }
    }

    pub fn token_account_deserialize(
        info: &AccountInfo,
    ) -> Result<spl_token::state::Account, Error> {
        if let Some(spl_token::state::State::Account(account)) =
            spl_token::state::State::deserialize(&info.data.borrow()).ok()
        {
            Ok(account)
        } else {
            Err(Error::ExpectedAccount)
        }
    }

    pub fn token_deserialize(info: &AccountInfo) -> Result<spl_token::state::Token, Error> {
        if let Some(spl_token::state::State::Token(token)) =
            spl_token::state::State::deserialize(&info.data.borrow()).ok()
        {
            Ok(token)
        } else {
            Err(Error::ExpectedToken)
        }
    }

    pub fn authority_id(program_id: &Pubkey, my_info: &Pubkey) -> Result<Pubkey, Error> {
        Pubkey::create_program_address(&[&my_info.to_string()[..32]], program_id)
            .or(Err(Error::InvalidProgramAddress))
    }
    pub fn token_burn(
        accounts: &[AccountInfo],
        token_program_id: &Pubkey,
        swap: &Pubkey,
        authority: &Pubkey,
        token: &Pubkey,
        source: Option<&Pubkey>,
        burn_account: &Pubkey,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let swap_string = swap.to_string();
        let signers = &[&[&swap_string[..32]][..]];
        let ix = spl_token::instruction::burn(
            token_program_id,
            authority,
            burn_account,
            token,
            source,
            amount,
        )?;
        invoke_signed(&ix, accounts, signers)
    }

    pub fn token_mint_to(
        accounts: &[AccountInfo],
        token_program_id: &Pubkey,
        swap: &Pubkey,
        authority: &Pubkey,
        token: &Pubkey,
        destination: &Pubkey,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let swap_string = swap.to_string();
        let signers = &[&[&swap_string[..32]][..]];
        let ix = spl_token::instruction::mint_to(
            token_program_id,
            authority,
            token,
            destination,
            amount,
        )?;
        invoke_signed(&ix, accounts, signers)
    }

    pub fn token_transfer(
        accounts: &[AccountInfo],
        token_program_id: &Pubkey,
        swap: &Pubkey,
        authority: &Pubkey,
        token: &Pubkey,
        source: Option<&Pubkey>,
        destination: &Pubkey,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let swap_string = swap.to_string();
        let signers = &[&[&swap_string[..32]][..]];
        let ix = spl_token::instruction::transfer(
            token_program_id,
            authority,
            token,
            destination,
            source,
            amount,
        )?;
        invoke_signed(&ix, accounts, signers)
    }

    pub fn process_init(
        program_id: &Pubkey,
        fee: (u64, u64),
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
        let pool_mint = Self::token_deserialize(pool_info)?;
        if *authority_info.key != token_a.owner {
            return Err(Error::InvalidOwner.into());
        }
        if *authority_info.key != token_b.owner {
            return Err(Error::InvalidOwner.into());
        }
        if Some(*authority_info.key) != pool_mint.owner {
            return Err(Error::InvalidOwner.into());
        }
        if 0 != pool_mint.info.supply {
            return Err(Error::InvalidSupply.into());
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

        // liqudity is measured in terms of token_a's value
        // since both sides of the pool are equal
        let amount = token_a.amount;
        Self::token_mint_to(
            accounts,
            token_program_info.key,
            swap_info.key,
            authority_info.key,
            pool_info.key,
            user_output_info.key,
            amount,
        )?;

        let obj = State::Init(TokenSwap {
            token_a: *token_a_info.key,
            token_b: *token_b_info.key,
            pool_mint: *pool_info.key,
            fee,
        });
        obj.serialize(&mut swap_info.data.borrow_mut())
    }

    pub fn process_swap(
        program_id: &Pubkey,
        amount: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let source_delegate_info = next_account_info(account_info_iter)?;
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
            pool: None,
        };
        let output = invariant
            .swap(amount)
            .ok_or_else(|| Error::CalculationFailure)?;
        Self::token_transfer(
            accounts,
            token_program_info.key,
            swap_info.key,
            authority_info.key,
            source_delegate_info.key,
            Some(source_info.key),
            into_info.key,
            amount,
        )?;
        Self::token_transfer(
            accounts,
            token_program_info.key,
            swap_info.key,
            authority_info.key,
            from_info.key,
            None,
            dest_info.key,
            output,
        )?;
        Ok(())
    }
    pub fn process_deposit(
        program_id: &Pubkey,
        a_amount: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let delegate_a_info = next_account_info(account_info_iter)?;
        let source_a_info = next_account_info(account_info_iter)?;
        let delegate_b_info = next_account_info(account_info_iter)?;
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
            pool: None,
        };
        let b_amount = invariant
            .exchange_rate(a_amount)
            .ok_or_else(|| Error::CalculationFailure)?;

        // liqudity is measured in terms of token_a's value
        // since both sides of the pool are equal
        let output = a_amount;

        Self::token_transfer(
            accounts,
            token_program_info.key,
            swap_info.key,
            authority_info.key,
            delegate_a_info.key,
            Some(source_a_info.key),
            token_a_info.key,
            a_amount,
        )?;
        Self::token_transfer(
            accounts,
            token_program_info.key,
            swap_info.key,
            authority_info.key,
            delegate_b_info.key,
            Some(source_b_info.key),
            token_b_info.key,
            b_amount,
        )?;
        Self::token_mint_to(
            accounts,
            token_program_info.key,
            swap_info.key,
            authority_info.key,
            pool_info.key,
            dest_info.key,
            output,
        )?;

        Ok(())
    }

    pub fn process_withdraw(
        program_id: &Pubkey,
        amount: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let delegate_info = next_account_info(account_info_iter)?;
        let source_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
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
        if *pool_info.key != token_swap.pool_mint {
            return Err(Error::InvalidInput.into());
        }
        let token_a = Self::token_account_deserialize(token_a_info)?;
        let token_b = Self::token_account_deserialize(token_b_info)?;
        let pool_token = Self::token_deserialize(pool_info)?;

        let invariant = Invariant {
            token_a: token_a.amount,
            token_b: token_b.amount,
            fee: token_swap.fee,
            pool: Some(pool_token.info.supply),
        };

        let (a_amount, b_amount) = invariant
            .redeem(amount)
            .ok_or_else(|| Error::CalculationFailure)?;
        Self::token_transfer(
            accounts,
            token_program_info.key,
            swap_info.key,
            authority_info.key,
            token_a_info.key,
            None,
            dest_token_a_info.key,
            a_amount,
        )?;
        Self::token_transfer(
            accounts,
            token_program_info.key,
            swap_info.key,
            authority_info.key,
            token_b_info.key,
            None,
            dest_token_b_info.key,
            b_amount,
        )?;
        Self::token_burn(
            accounts,
            token_program_info.key,
            swap_info.key,
            authority_info.key,
            pool_info.key,
            Some(source_info.key),
            delegate_info.key,
            amount,
        )?;
        Ok(())
    }
    /// Processes an [SwapInstruction](enum.Instruction.html).
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        let instruction = SwapInstruction::deserialize(input)?;
        match instruction {
            SwapInstruction::Init(fee) => {
                info!("Instruction: Init");
                Self::process_init(program_id, fee, accounts)
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

entrypoint!(process_instruction);
fn process_instruction<'a>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'a>],
    instruction_data: &[u8],
) -> ProgramResult {
    if let Err(error) = State::process(program_id, accounts, instruction_data) {
        // catch the error so we can print it
        error.print::<Error>();
        return Err(error);
    }
    Ok(())
}

// Test program id for the swap program
#[cfg(not(target_arch = "bpf"))]
const SWAP_PROGRAM_ID: Pubkey = Pubkey::new_from_array([2u8; 32]);

/// Routes invokes to the token program, used for testing
#[cfg(not(target_arch = "bpf"))]
pub fn invoke_signed<'a>(
    instruction: &Instruction,
    account_infos: &[AccountInfo<'a>],
    signers_seeds: &[&[&str]],
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
    spl_token::state::State::process(
        &instruction.program_id,
        &new_account_infos,
        &instruction.data,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{
        account::Account, account_info::create_is_signer_account_infos, instruction::Instruction,
    };
    use spl_token::{
        instruction::{new_account, new_token, TokenInfo},
        state::State as SplState,
    };

    const TOKEN_PROGRAM_ID: Pubkey = Pubkey::new_from_array([1u8; 32]);

    // Pulls in the stubs required for `info!()`
    #[cfg(not(target_arch = "bpf"))]
    solana_sdk_bpf_test::stubs!();

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
            SplState::process(&instruction.program_id, &account_infos, &instruction.data)
        }
    }

    fn mint_token(
        program_id: &Pubkey,
        authority_key: &Pubkey,
        supply: u64,
    ) -> ((Pubkey, Account), (Pubkey, Account)) {
        let token_key = pubkey_rand();
        let mut token_account = Account::new(0, size_of::<SplState>(), &program_id);
        let account_key = pubkey_rand();
        let mut account_account = Account::new(0, size_of::<SplState>(), &program_id);

        // create pool and pool account
        do_process_instruction(
            new_account(&program_id, &account_key, &authority_key, &token_key, None).unwrap(),
            vec![
                &mut account_account,
                &mut Account::default(),
                &mut token_account,
            ],
        )
        .unwrap();
        let mut authority_account = Account::default();
        do_process_instruction(
            new_token(
                &program_id,
                &token_key,
                Some(&account_key),
                Some(&authority_key),
                TokenInfo {
                    supply,
                    decimals: 2,
                },
            )
            .unwrap(),
            if supply == 0 {
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
    fn test_init() {
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
            init(
                &SWAP_PROGRAM_ID,
                &TOKEN_PROGRAM_ID,
                &swap_key,
                &authority_key,
                &token_a_key,
                &token_b_key,
                &pool_key,
                &pool_token_key,
                (1, 2),
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
