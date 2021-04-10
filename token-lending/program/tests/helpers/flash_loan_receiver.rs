use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, pubkey::Pubkey,
};

use crate::processor::Processor;
use spl_token::solana_program::account_info::next_account_info;
use spl_token::solana_program::program_error::ProgramError;
use spl_token::solana_program::program::invoke_signed;
use core::mem;

solana_program::declare_id!("FlashLoan1111111111111111111111111111111111");


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
        let value_account_info = next_account_info(account_info_iter)?;

        // I don't understand why we need the & here...
        let token_account = TokenAccount::unpack_from_slice(&destination_liquidity_account_info.try_borrow_mut_data()?)?;
        let (pda, nonce) = Pubkey::find_program_address(&[b"flashloan"], program_id);


        if token_account.owner != pda {
            msg!(&pda.to_string());
            msg!("Token account is not owned by the program.");
            return Err(ProgramError::IncorrectProgramId);
        }


        if value_account_info.owner != program_id {
            msg!("Value account is not owned by the program.");
            return Err(ProgramError::IncorrectProgramId);
        }

        // The data must be large enough to hold a u64 count
        if value_account_info.try_data_len()? < mem::size_of::<u64>() {
            msg!("Greeted account data length too small for u64");
            return Err(ProgramError::InvalidAccountData);
        }


        let mut data = value_account_info.try_borrow_mut_data()?;
        LittleEndian::write_u64(&mut data[0..], token_account.amount);

        let transfer_ix = spl_token::instruction::transfer(
            token_program_info.key,
            destination_liquidity_account_info.key,
            repay_token_account_info.key,
            &pda,
            &[&pda],
            amount)?;


        msg!("Calling the token program to transfer the token back...");
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

