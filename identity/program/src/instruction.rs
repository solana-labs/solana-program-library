//! Instruction types

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar,
    info,
};

/// Minimum number of multisignature signers (min N)
pub const MIN_SIGNERS: usize = 1;
/// Maximum number of multisignature signers (max N)
pub const MAX_SIGNERS: usize = 11;

/// Instructions supported by the identity program.
#[repr(C)]
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
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
    InitializeIdentity,

    /// Registers an attestation against an identity
    ///
    /// The 'Attest' instruction allows an identity validator (IdV) to
    /// register claims against an identity account.
    ///
    /// An attestation is typically merely a string,
    /// representing a hash of an off-chain credential.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The subject identity account to attest claims against.
    ///   1. `[signer]` The IDV.
    Attest {
        /// The string to be attested, as a byte array
        attestation_data: [u8; 32]
    }

}
impl IdentityInstruction {
    /// Serializes an [IdentityInstruction](enum.IdentityInstruction.html) into a byte buffer.
    pub fn serialize(&self) -> Result<Vec<u8>, ProgramError> {
        info!("insrtuction serialize");
        self.try_to_vec()
            .map_err(|_| ProgramError::AccountDataTooSmall)
    }

    /// Deserializes a byte buffer into a [IdentityInstruction](enum.IdentityInstruction.html).
    pub(crate) fn deserialize(data: &[u8]) -> Result<Self, ProgramError> {
        Self::try_from_slice(&data).map_err(|_| ProgramError::InvalidInstructionData)
    }
}

/// Creates a `InitializeIdentity` instruction.
pub fn initialize_identity(
    identity_program_id: &Pubkey,
    account_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
) -> Result<Instruction, ProgramError> {
    info!("initialize_identity: start");
    let data = IdentityInstruction::InitializeIdentity; // TODO do we need to return result?

    let accounts = vec![
        AccountMeta::new(*account_pubkey, false),
        AccountMeta::new_readonly(*owner_pubkey, false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];
    info!("initialize_identity: accounts created");

    Ok(Instruction {
        program_id: *identity_program_id,
        accounts,
        data: data.serialize().unwrap(),
    })
}

/// Return an `Attest` instruction.
pub fn attest(
    program_id: &Pubkey,
    identity_pubkey: &Pubkey,
    idv_pubkey: &Pubkey,
    attestation_data: &[u8; 32],
) -> Instruction {
    let data = IdentityInstruction::Attest {
        attestation_data: *attestation_data,
    };
    let accounts = vec![
        AccountMeta::new(*identity_pubkey, false),
        AccountMeta::new_readonly(*idv_pubkey, true),
    ];
    Instruction {
        program_id: *program_id,
        accounts,
        data: data.serialize().unwrap(),
    }
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
