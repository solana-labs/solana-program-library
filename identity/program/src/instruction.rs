//! Instruction types

use crate::error::IdentityError;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    program_option::COption,
    pubkey::Pubkey,
    sysvar,
};
use std::mem::size_of;

/// Minimum number of multisignature signers (min N)
pub const MIN_SIGNERS: usize = 1;
/// Maximum number of multisignature signers (max N)
pub const MAX_SIGNERS: usize = 11;

/// Instructions supported by the identity program.
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum IdentityInstruction {
    /// Initializes a new account to hold identity information.
    ///
    /// The `InitializeIdentity` instruction requires no signers and MUST be
    /// included within the same Transaction as the system program's
    /// `CreateAccount` instruction that creates the account being initialized.
    /// Otherwise another party can acquire ownership of the uninitialized
    /// account.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]`  The account to initialize.
    ///   1. `[]` The new account's owner/multisignature.
    ///   2. `[]` Rent sysvar
    InitializeIdentity


}
impl IdentityInstruction {
    /// Unpacks a byte buffer into a [IdentityInstruction](enum.IdentityInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        use IdentityError::InvalidInstruction;

        let (&tag, rest) = input.split_first().ok_or(InvalidInstruction)?;
        Ok(match tag {
            0 => Self::InitializeIdentity,
            _ => return Err(IdentityError::InvalidInstruction.into()),
        })
    }

    /// Packs a [IdentityInstruction](enum.IdentityInstruction.html) into a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());
        match self {
            Self::InitializeIdentity => buf.push(0),
        };
        buf
    }

    fn unpack_pubkey(input: &[u8]) -> Result<(Pubkey, &[u8]), ProgramError> {
        if input.len() >= 32 {
            let (key, rest) = input.split_at(32);
            let pk = Pubkey::new(key);
            Ok((pk, rest))
        } else {
            Err(IdentityError::InvalidInstruction.into())
        }
    }

    fn unpack_pubkey_option(input: &[u8]) -> Result<(COption<Pubkey>, &[u8]), ProgramError> {
        match input.split_first() {
            Option::Some((&0, rest)) => Ok((COption::None, rest)),
            Option::Some((&1, rest)) if rest.len() >= 32 => {
                let (key, rest) = rest.split_at(32);
                let pk = Pubkey::new(key);
                Ok((COption::Some(pk), rest))
            }
            _ => Err(IdentityError::InvalidInstruction.into()),
        }
    }

    fn pack_pubkey_option(value: &COption<Pubkey>, buf: &mut Vec<u8>) {
        match *value {
            COption::Some(ref key) => {
                buf.push(1);
                buf.extend_from_slice(&key.to_bytes());
            }
            COption::None => buf.push(0),
        }
    }
}

/// Specifies the authority type for SetAuthority instructions
#[repr(u8)]
#[derive(Clone, Debug, PartialEq)]
pub enum AuthorityType {
    /// Owner of a given identity account
    AccountOwner
}

impl AuthorityType {
    fn into(&self) -> u8 {
        match self {
            AuthorityType::AccountOwner => 0,
        }
    }

    fn from(index: u8) -> Result<Self, ProgramError> {
        match index {
            0 => Ok(AuthorityType::AccountOwner),
            _ => Err(IdentityError::InvalidInstruction.into()),
        }
    }
}

/// Creates a `InitializeIdentity` instruction.
pub fn initialize_identity(
    identity_program_id: &Pubkey,
    account_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = IdentityInstruction::InitializeIdentity.pack(); // TODO do we need to return result?

    let accounts = vec![
        AccountMeta::new(*account_pubkey, false),
        AccountMeta::new_readonly(*owner_pubkey, false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    Ok(Instruction {
        program_id: *identity_program_id,
        accounts,
        data,
    })
}

/// Utility function that checks index is between MIN_SIGNERS and MAX_SIGNERS
pub fn is_valid_signer_index(index: usize) -> bool {
    !(index < MIN_SIGNERS || index > MAX_SIGNERS)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_instruction_packing() {
        let check = IdentityInstruction::InitializeIdentity;
        let packed = check.pack();
        let expect = Vec::from([1u8]);
        assert_eq!(packed, expect);
        let unpacked = IdentityInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);
    }
}
