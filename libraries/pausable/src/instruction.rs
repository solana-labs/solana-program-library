//! Instructin types

use solana_program::program_error::ProgramError;
use std::mem::size_of;

#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum PausableInstruction {
    /// Pause the Program.  Can only be performed by the program Owner.
    ///   0. `[writable]` The program account to pause
    ///   1. `[]` account of the program owner
    Pause,
    /// Resume the Program.  Can only b eperformed by the program Owner.
    ///   0. `[writable]` The program account to resume
    ///   1. `[]` account of the program owner
    Resume,
}
impl PausableInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&tag, _) = input.split_first().ok_or(ProgramError::InvalidInstructionData)?;
        Ok(match tag {
            0 => Self::Pause,
            1 => Self::Resume,
            _ => return Err(ProgramError::InvalidInstructionData.into()),
        })
    }

    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());
        match self {
            Self::Pause => { buf.push(0); }
            Self::Resume => { buf.push(1); }
        };
        buf
    }
}

#[cfg(test)]
mod test {
    use super::PausableInstruction;
    use solana_program::program_error::ProgramError;

    #[test]
    fn test_packing() {
        assert_eq!(PausableInstruction::Pause .pack(), [0]);
        assert_eq!(PausableInstruction::unpack(&[1]).unwrap(), PausableInstruction::Resume);
        assert_eq!(PausableInstruction::unpack(&[2]), Err(ProgramError::InvalidInstructionData.into()));
    }
}
