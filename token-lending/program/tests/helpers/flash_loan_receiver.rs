use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, msg, pubkey::Pubkey,
};

use crate::helpers::flash_loan_receiver::FlashLoanReceiverError::InvalidInstruction;
use spl_token::{
    solana_program::{
        account_info::next_account_info, program::invoke_signed, program_error::ProgramError,
        program_pack::Pack,
    },
    state::Account,
};
use std::cmp::min;
use std::convert::TryInto;
use thiserror::Error;

pub enum FlashLoanReceiverInstruction {
    /// Receive a flash loan and perform user-defined operation and finally return the fund back.
    ///
    /// Accounts expected:
    ///
    ///   0. `[writable]` Source liquidity (matching the destination from above).
    ///   1. `[writable]` Destination liquidity (matching the source from above).
    ///   2. `[]` Token program id
    ///   .. `[any]` Additional accounts provided to the lending program's `FlashLoan` instruction above.
    ReceiveFlashLoan {
        /// The amount that is loaned
        amount: u64,
    },
}

entrypoint!(process_instruction);
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    Processor::process(program_id, accounts, instruction_data)
}

pub struct Processor;
impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = FlashLoanReceiverInstruction::unpack(instruction_data)?;

        match instruction {
            FlashLoanReceiverInstruction::ReceiveFlashLoan { amount } => {
                msg!("Instruction: Receive Flash Loan");
                Self::process_receive_flash_loan(accounts, amount, program_id)
            }
        }
    }

    fn process_receive_flash_loan(
        accounts: &[AccountInfo],
        amount: u64,
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_liquidity_token_account_info = next_account_info(account_info_iter)?;
        let destination_liquidity_token_account_info = next_account_info(account_info_iter)?;
        let token_program_id = next_account_info(account_info_iter)?;
        let program_derived_account_info = next_account_info(account_info_iter)?;

        let destination_liquidity_token_account = Account::unpack_from_slice(
            &source_liquidity_token_account_info.try_borrow_mut_data()?,
        )?;
        let (expected_program_derived_account_pubkey, bump_seed) =
            Pubkey::find_program_address(&[b"flashloan"], program_id);

        if &expected_program_derived_account_pubkey != program_derived_account_info.key {
            msg!("Supplied program derived account doesn't match with expectation.")
        }

        if destination_liquidity_token_account.owner != expected_program_derived_account_pubkey {
            msg!("Destination liquidity token account is not owned by the program");
            return Err(ProgramError::IncorrectProgramId);
        }

        let balance_in_token_account =
            Account::unpack_from_slice(&source_liquidity_token_account_info.try_borrow_data()?)?
                .amount;
        let transfer_ix = spl_token::instruction::transfer(
            token_program_id.key,
            source_liquidity_token_account_info.key,
            destination_liquidity_token_account_info.key,
            &expected_program_derived_account_pubkey,
            &[],
            min(balance_in_token_account, amount),
        )?;

        invoke_signed(
            &transfer_ix,
            &[
                source_liquidity_token_account_info.clone(),
                destination_liquidity_token_account_info.clone(),
                program_derived_account_info.clone(),
                token_program_id.clone(),
            ],
            &[&[&b"flashloan"[..], &[bump_seed]]],
        )?;

        Ok(())
    }
}

impl FlashLoanReceiverInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (tag, rest) = input.split_first().ok_or(InvalidInstruction)?;

        Ok(match tag {
            0 => Self::ReceiveFlashLoan {
                amount: Self::unpack_amount(rest)?,
            },
            _ => return Err(InvalidInstruction.into()),
        })
    }

    fn unpack_amount(input: &[u8]) -> Result<u64, ProgramError> {
        let amount = input
            .get(..8)
            .and_then(|slice| slice.try_into().ok())
            .map(u64::from_le_bytes)
            .ok_or(InvalidInstruction)?;
        Ok(amount)
    }
}

#[derive(Error, Debug, Copy, Clone)]
pub enum FlashLoanReceiverError {
    /// Invalid instruction
    #[error("Invalid Instruction")]
    InvalidInstruction,
    #[error("The account is not currently owned by the program")]
    IncorrectProgramId,
}

impl From<FlashLoanReceiverError> for ProgramError {
    fn from(e: FlashLoanReceiverError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
