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
    // try to transfer one token of a decimals: 9 mint
    TransferOneChecked,
    // try to transfer one token of a mint
    TransferOneUnchecked,
}
impl TryFrom<u8> for TestInstruction {
    type Error = ProgramError;

    fn try_from(index: u8) -> Result<Self, Self::Error> {
        match index {
            0 => Ok(TestInstruction::EnableCpiGuard),
            1 => Ok(TestInstruction::DisableCpiGuard),
            2 => Ok(TestInstruction::TransferOneChecked),
            3 => Ok(TestInstruction::TransferOneUnchecked),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
impl From<TestInstruction> for u8 {
    fn from(ixn: TestInstruction) -> u8 {
        match ixn {
            TestInstruction::EnableCpiGuard => 0,
            TestInstruction::DisableCpiGuard => 1,
            TestInstruction::TransferOneChecked => 2,
            TestInstruction::TransferOneUnchecked => 3,
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

pub fn transfer_one_token(
    program_id: &Pubkey,
    token_program_id: &Pubkey,
    source: &Pubkey,
    mint: &Pubkey,
    destination: &Pubkey,
    owner: &Pubkey,
    checked: bool,
) -> Result<Instruction, ProgramError> {
    Ok(Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*token_program_id, false),
            AccountMeta::new(*source, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new(*destination, false),
            AccountMeta::new_readonly(*owner, true),
        ],
        data: vec![if checked {
            TestInstruction::TransferOneChecked.into()
        } else {
            TestInstruction::TransferOneUnchecked.into()
        }],
    })
}
