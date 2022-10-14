use {
    crate::instruction::TestInstruction,
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program::invoke,
        pubkey::Pubkey,
    },
    spl_token_2022::extension::cpi_guard,
    std::convert::TryFrom,
};

pub struct Processor {}
impl Processor {
    pub fn process(_program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        match TestInstruction::try_from(input[0])? {
            TestInstruction::EnableCpiGuard => {
                msg!("Instruction: EnableCpiGuard");

                let account_info_iter = &mut accounts.iter();
                let token_program = next_account_info(account_info_iter)?;
                let account = next_account_info(account_info_iter)?;
                let owner = next_account_info(account_info_iter)?;

                let instruction = cpi_guard::instruction::enable_cpi_guard(
                    token_program.key,
                    account.key,
                    owner.key,
                    &[],
                )?;

                invoke(&instruction, &[account.clone(), owner.clone()])
            }
            TestInstruction::DisableCpiGuard => {
                msg!("Instruction: DisableCpiGuard");

                let account_info_iter = &mut accounts.iter();
                let token_program = next_account_info(account_info_iter)?;
                let account = next_account_info(account_info_iter)?;
                let owner = next_account_info(account_info_iter)?;

                let instruction = cpi_guard::instruction::disable_cpi_guard(
                    token_program.key,
                    account.key,
                    owner.key,
                    &[],
                )?;

                invoke(&instruction, &[account.clone(), owner.clone()])
            }
        }
    }
}
