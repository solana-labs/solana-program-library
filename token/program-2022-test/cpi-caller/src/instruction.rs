use {
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    std::convert::TryFrom,
};

#[derive(Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum TestInstruction {
    // try to enable cpi guard from cpi. this should fail unconditionally
    EnableCpiGuard,
    // try to disable cpi guard from cpi. this should fail unconditionally
    DisableCpiGuard,
}
impl TryFrom<u8> for TestInstruction {
    type Error = ProgramError;

    fn try_from(index: u8) -> Result<Self, Self::Error> {
        match index {
            0 => Ok(TestInstruction::EnableCpiGuard),
            1 => Ok(TestInstruction::DisableCpiGuard),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
impl From<TestInstruction> for u8 {
    fn from(ixn: TestInstruction) -> u8 {
        match ixn {
            TestInstruction::EnableCpiGuard => 0,
            TestInstruction::DisableCpiGuard => 1,
        }
    }
}

pub fn enable_cpi_guard(
    program_id: &Pubkey,
    token_program_id: &Pubkey,
    account: &Pubkey,
    owner: &Pubkey,
) -> Result<Instruction, ProgramError> {
    Ok(Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*token_program_id, false),
            AccountMeta::new(*account, false),
            AccountMeta::new_readonly(*owner, true),
        ],
        data: vec![TestInstruction::EnableCpiGuard.into()],
    })
}

pub fn disable_cpi_guard(
    program_id: &Pubkey,
    token_program_id: &Pubkey,
    account: &Pubkey,
    owner: &Pubkey,
) -> Result<Instruction, ProgramError> {
    Ok(Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*token_program_id, false),
            AccountMeta::new(*account, false),
            AccountMeta::new_readonly(*owner, true),
        ],
        data: vec![TestInstruction::DisableCpiGuard.into()],
    })
}
