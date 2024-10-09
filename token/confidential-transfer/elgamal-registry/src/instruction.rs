use solana_program::{
    program_error::ProgramError,
    pubkey::{Pubkey, PUBKEY_BYTES},
};

#[derive(Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum RegistryInstruction {
    /// Initialize an ElGamal public key registry for an account.
    ///
    /// 0. `[writable]` The account to initialize
    /// 1. `[]` Instructions sysvar if `VerifyPubkeyValidity` is included in the
    ///    same transaction or context state account if `VerifyPubkeyValidity`
    ///    is pre-verified into a context state account.
    /// 2. `[]` (Optional) Record account if the accompanying proof is to be
    ///    read from a record account.
    CreateRegistry {
        /// The owner of the ElGamal registry account
        owner: Pubkey,
        /// Relative location of the `ProofInstruction::PubkeyValidityProof`
        /// instruction to the `CreateElGamalRegistry` instruction in the
        /// transaction. If the offset is `0`, then use a context state account
        /// for the proof.
        proof_instruction_offset: i8,
    },
    /// Update an ElGamal public key registry with a new ElGamal public key.
    ///
    /// 0. `[writable]` The account to initialize
    /// 1. `[signer]` The owner of the ElGamal public key registry
    /// 2. `[]` Instructions sysvar if `VerifyPubkeyValidity` is included in the
    ///    same transaction or context state account if `VerifyPubkeyValidity`
    ///    is pre-verified into a context state account.
    /// 3. `[]` (Optional) Record account if the accompanying proof is to be
    ///    read from a record account.
    UpdateRegistry {
        /// Relative location of the `ProofInstruction::PubkeyValidityProof`
        /// instruction to the `UpdateElGamalRegistry` instruction in the
        /// transaction. If the offset is `0`, then use a context state account
        /// for the proof.
        proof_instruction_offset: i8,
    },
}

impl RegistryInstruction {
    /// Unpacks a byte buffer into a `RegistryInstruction`
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&tag, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;

        Ok(match tag {
            0 => {
                let owner = rest
                    .get(..PUBKEY_BYTES)
                    .and_then(|x| Pubkey::try_from(x).ok())
                    .ok_or(ProgramError::InvalidInstructionData)?;
                let proof_instruction_offset =
                    *rest.first().ok_or(ProgramError::InvalidInstructionData)?;
                Self::CreateRegistry {
                    owner,
                    proof_instruction_offset: proof_instruction_offset as i8,
                }
            }
            2 => {
                let proof_instruction_offset =
                    *rest.first().ok_or(ProgramError::InvalidInstructionData)?;
                Self::UpdateRegistry {
                    proof_instruction_offset: proof_instruction_offset as i8,
                }
            }
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }

    /// Packs a `RegistryInstruction` into a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = vec![];
        match self {
            Self::CreateRegistry {
                owner,
                proof_instruction_offset,
            } => {
                buf.push(0);
                buf.extend_from_slice(owner.as_ref());
                buf.extend_from_slice(&proof_instruction_offset.to_le_bytes());
            }
            Self::UpdateRegistry {
                proof_instruction_offset,
            } => {
                buf.push(1);
                buf.extend_from_slice(&proof_instruction_offset.to_le_bytes());
            }
        };
        buf
    }
}
