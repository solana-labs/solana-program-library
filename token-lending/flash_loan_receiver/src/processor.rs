use {
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program::invoke,
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    std::convert::TryInto,
};

pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    msg!("Flash Loan Receiver invoked.");
    let account_info_iter = &mut accounts.iter();
    let destination_liq_info = next_account_info(account_info_iter)?;
    let source_liq_info = next_account_info(account_info_iter)?;
    let spl_token_program_info = next_account_info(account_info_iter)?;
    let user_transfer_authority_info = next_account_info(account_info_iter)?;

    let (tag, rest) = input
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    if *tag != 0 {
        msg!(
            "Expecting the 0th instruction to be called. Instead {}th was called.",
            tag
        );
        return Err(ProgramError::InvalidInstructionData);
    }

    let amount = unpack_amount(rest)?;

    invoke(
        &spl_token::instruction::transfer(
            spl_token_program_info.key,
            destination_liq_info.key,
            source_liq_info.key,
            user_transfer_authority_info.key,
            &[],
            amount,
        )?,
        &[
            source_liq_info.clone(),
            destination_liq_info.clone(),
            user_transfer_authority_info.clone(),
            spl_token_program_info.clone(),
        ],
    )?;
    Ok(())
}

fn unpack_amount(input: &[u8]) -> Result<u64, ProgramError> {
    let amount = input
        .get(..8)
        .and_then(|slice| slice.try_into().ok())
        .map(u64::from_le_bytes)
        .ok_or(ProgramError::InvalidInstructionData)
        .inspect_err(|_| {
            msg!("Failed to unpack amount.");
        })?;
    Ok(amount)
}
