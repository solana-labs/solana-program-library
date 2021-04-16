//! Instructin types

use crate::error::OwnableError;
use solana_program::program_error::ProgramError;
use std::mem::size_of;

#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum OwnableInstruction {
    /// Initialize the owner.
    /// Accounts expected by this instruciton:
    ///   0. `[writable]` The account to initialize
    ///   1. `[]` account of the initial owner
    InitializeOwnership,
    /// Transfer ownership to another.
    /// Accounts expected by this instruction:
    ///  0. `[writable]` The account to transfer ownership
    ///  1. `[]` The account of the current owner
    ///  2. `[]` account to transfer ownership to
    TransferOwnership,
    /// Renounce ownership.  Once ownership has been renounced
    /// all ownership restricted functionality will be lost.
    /// Accounts expected by this instruction:
    ///  0. `[writable]` The account to transfer ownership
    ///  1. `[]` The account of the current owner
    RenounceOwnership,
}
impl OwnableInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&tag, _) = input.split_first().ok_or(OwnableError::InvalidInstruction)?;
        Ok(match tag {
            0 => Self::InitializeOwnership,
            1 => Self::TransferOwnership,
            2 => Self::RenounceOwnership,
            _ => return Err(OwnableError::InvalidInstruction.into()),
        })
    }

    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());
        match self {
            Self::InitializeOwnership => buf.push(0),
            Self::TransferOwnership => buf.push(1),
            Self::RenounceOwnership => buf.push(2),
        };
        buf
    }
}

#[cfg(test)]
mod test {
    use super::OwnableInstruction;

    #[test]
    fn test_packing() {
        assert_eq!(OwnableInstruction::InitializeOwnership.pack(), [0]);
        assert_eq!(OwnableInstruction::unpack(&[1]).unwrap(), OwnableInstruction::TransferOwnership);
        assert_eq!(OwnableInstruction::RenounceOwnership.pack(), [2]);
    }
}
