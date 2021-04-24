use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, msg, pubkey::Pubkey,
};

use crate::helpers::flash_loan_receiver::FlashLoanReceiverError::InvalidInstruction;
use spl_token::solana_program::account_info::next_account_info;
use spl_token::solana_program::program::invoke_signed;
use spl_token::solana_program::program_error::ProgramError;
use spl_token::solana_program::program_pack::Pack;
use spl_token::state::Account as TokenAccount;
use std::convert::TryInto;
use thiserror::Error;

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
            FlashLoanReceiverInstruction::ExecuteOperation { amount } => {
                msg!("Execute operation");
                Self::process_execute_operation(accounts, amount, program_id)
            }
        }
    }

    fn process_execute_operation(
        accounts: &[AccountInfo],
        amount: u64,
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let destination_liquidity_account_info = next_account_info(account_info_iter)?;
        let pda_account_info = next_account_info(account_info_iter)?;
        let repay_token_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let token_account = TokenAccount::unpack_from_slice(
            &destination_liquidity_account_info.try_borrow_mut_data()?,
        )?;
        let (pda, nonce) = Pubkey::find_program_address(&[b"flashloan"], program_id);

        if token_account.owner != pda {
            msg!("Token account is not owned by the program.");
            return Err(ProgramError::IncorrectProgramId);
        }

        let transfer_ix = spl_token::instruction::transfer(
            token_program_info.key,
            destination_liquidity_account_info.key,
            repay_token_account_info.key,
            &pda,
            &[],
            amount,
        )?;

        invoke_signed(
            &transfer_ix,
            &[
                destination_liquidity_account_info.clone(),
                repay_token_account_info.clone(),
                pda_account_info.clone(),
                token_program_info.clone(),
            ],
            &[&[&b"flashloan"[..], &[nonce]]],
        )?;

        Ok(())
    }
}

pub enum FlashLoanReceiverInstruction {
    /// Execute the operation that is needed after flash loan
    ///
    /// Accounts expected:
    ///
    /// 0. `[writable]` The destination liquidity token account.
    /// 1. `[]` program derived account.
    /// 2. `[writable]` The repay token account.
    /// 3. `[]` The token program
    /// 4. `[writable]` the account that the FlashLoanReceiver needs to write to.
    ExecuteOperation {
        /// The amount that is loaned
        amount: u64,
    },
}

impl FlashLoanReceiverInstruction {
    /// Unpacks a byte buffer into a [EscrowInstruction](enum.EscrowInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (tag, rest) = input.split_first().ok_or(InvalidInstruction)?;

        Ok(match tag {
            0 => Self::ExecuteOperation {
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
