use {
    crate::instruction::TestInstruction,
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program::invoke,
        pubkey::Pubkey,
    },
    spl_token_2022::{extension::cpi_guard, instruction},
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
            TestInstruction::TransferOneChecked => {
                msg!("Instruction: TransferOneChecked ");

                let account_info_iter = &mut accounts.iter();
                let token_program = next_account_info(account_info_iter)?;
                let source = next_account_info(account_info_iter)?;
                let mint = next_account_info(account_info_iter)?;
                let destination = next_account_info(account_info_iter)?;
                let owner = next_account_info(account_info_iter)?;

                let instruction = instruction::transfer_checked(
                    token_program.key,
                    source.key,
                    mint.key,
                    destination.key,
                    owner.key,
                    &[],
                    1,
                    9,
                )?;

                invoke(
                    &instruction,
                    &[
                        source.clone(),
                        mint.clone(),
                        destination.clone(),
                        owner.clone(),
                    ],
                )
            }
            TestInstruction::TransferOneUnchecked => {
                msg!("Instruction: TransferOneUnchecked ");

                let account_info_iter = &mut accounts.iter();
                let token_program = next_account_info(account_info_iter)?;
                let source = next_account_info(account_info_iter)?;
                let _ = next_account_info(account_info_iter)?;
                let destination = next_account_info(account_info_iter)?;
                let owner = next_account_info(account_info_iter)?;

                #[allow(deprecated)]
                let instruction = instruction::transfer(
                    token_program.key,
                    source.key,
                    destination.key,
                    owner.key,
                    &[],
                    1,
                )?;

                invoke(
                    &instruction,
                    &[source.clone(), destination.clone(), owner.clone()],
                )
            }
        }
    }
}
