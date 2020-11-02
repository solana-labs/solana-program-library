//! DefaultTokenAccount program instructions

use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar,
};

/// Instructions supported by the DefaultTokenAccount program.
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum DefaultTokenAccountInstruction {
    /// Create the default token account for a user wallet address.
    /// Fails if the address already exists.
    ///
    ///   0. `[writeable]` Default token account address derived from `get_default_token_account_address()`
    ///   1. `[]` User wallet address (system account)
    ///   2. `[]` The mint this account will be associated with.
    ///   3. `[writeable,signer]` The system account that will fund the default token account
    ///   4. `[]` Rent sysvar
    ///   5. `[]` System program
    ///   6. `[]` SPL Token program
    ///
    Create,

    /// Asserts that the default token account exists and the provided user wallet address is the
    /// owner of it.
    ///
    ///   0. `[]` Default token account address derived from `get_default_token_account_address()`
    ///   1. `[]` User wallet address (system account)
    ///   2. `[]` The mint this account should be associated with.
    ///   3. `[]` SPL Token program
    ///
    Exists,
}

impl DefaultTokenAccountInstruction {
    /// Unpacks a byte buffer into a [DefaultTokenAccountInstruction](enum.DefaultTokenAccountInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        if input.len() != 1 {
            return Err(ProgramError::InvalidInstructionData);
        }

        Ok(match input[0] {
            0 => Self::Create,
            1 => Self::Exists,
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }

    /// Packs a [DefaultTokenAccountInstruction](enum.DefaultTokenAccountInstruction.html) into a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        match *self {
            Self::Create => vec![0],
            Self::Exists => vec![1],
        }
    }
}

/// Construct a DefaultTokenAccountInstruction::Create instruction
pub fn create(
    default_token_account_address: &Pubkey,
    user_wallet_address: &Pubkey,
    mint_address: &Pubkey,
    funding_address: &Pubkey,
) -> Instruction {
    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(*default_token_account_address, false),
            AccountMeta::new_readonly(*user_wallet_address, false),
            AccountMeta::new_readonly(*mint_address, false),
            AccountMeta::new(*funding_address, true),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: DefaultTokenAccountInstruction::Create.pack(),
    }
}

/// Construct a DefaultTokenAccountInstruction::Exists instruction
pub fn exists(
    default_token_account_address: &Pubkey,
    user_wallet_address: &Pubkey,
    mint_address: &Pubkey,
) -> Instruction {
    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new_readonly(*default_token_account_address, false),
            AccountMeta::new_readonly(*user_wallet_address, false),
            AccountMeta::new_readonly(*mint_address, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: DefaultTokenAccountInstruction::Exists.pack(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack() {
        assert_eq!(DefaultTokenAccountInstruction::Create.pack(), [0]);
        assert_eq!(DefaultTokenAccountInstruction::Exists.pack(), [1]);
    }

    #[test]
    fn test_unpack() {
        assert_eq!(
            DefaultTokenAccountInstruction::unpack(&[0]),
            Ok(DefaultTokenAccountInstruction::Create)
        );
        assert_eq!(
            DefaultTokenAccountInstruction::unpack(&[1]),
            Ok(DefaultTokenAccountInstruction::Exists)
        );

        assert_eq!(
            DefaultTokenAccountInstruction::unpack(&[]),
            Err(ProgramError::InvalidInstructionData)
        );
        assert_eq!(
            DefaultTokenAccountInstruction::unpack(&[2]),
            Err(ProgramError::InvalidInstructionData)
        );
        assert_eq!(
            DefaultTokenAccountInstruction::unpack(&[0, 0]),
            Err(ProgramError::InvalidInstructionData)
        );
    }
}
