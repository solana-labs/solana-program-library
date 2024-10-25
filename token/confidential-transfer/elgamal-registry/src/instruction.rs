use {
    crate::{get_elgamal_registry_address, id},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
        system_program, sysvar,
    },
    solana_zk_sdk::zk_elgamal_proof_program::{
        instruction::ProofInstruction, proof_data::PubkeyValidityProofData,
    },
    spl_token_confidential_transfer_proof_extraction::instruction::{ProofData, ProofLocation},
};

#[derive(Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum RegistryInstruction {
    /// Initialize an ElGamal public key registry.
    ///
    /// 0. `[writable]` The account to be created
    /// 1. `[signer]` The wallet address (will also be the owner address for the
    ///    registry account)
    /// 2. `[]` System program
    /// 3. `[]` Instructions sysvar if `VerifyPubkeyValidity` is included in the
    ///    same transaction or context state account if `VerifyPubkeyValidity`
    ///    is pre-verified into a context state account.
    /// 4. `[]` (Optional) Record account if the accompanying proof is to be
    ///    read from a record account.
    CreateRegistry {
        /// Relative location of the `ProofInstruction::PubkeyValidityProof`
        /// instruction to the `CreateElGamalRegistry` instruction in the
        /// transaction. If the offset is `0`, then use a context state account
        /// for the proof.
        proof_instruction_offset: i8,
    },
    /// Update an ElGamal public key registry with a new ElGamal public key.
    ///
    /// 0. `[writable]` The ElGamal registry account
    /// 1. `[]` Instructions sysvar if `VerifyPubkeyValidity` is included in the
    ///    same transaction or context state account if `VerifyPubkeyValidity`
    ///    is pre-verified into a context state account.
    /// 2. `[]` (Optional) Record account if the accompanying proof is to be
    ///    read from a record account.
    /// 3. `[signer]` The owner of the ElGamal public key registry
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
                let proof_instruction_offset =
                    *rest.first().ok_or(ProgramError::InvalidInstructionData)?;
                Self::CreateRegistry {
                    proof_instruction_offset: proof_instruction_offset as i8,
                }
            }
            1 => {
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
                proof_instruction_offset,
            } => {
                buf.push(0);
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

/// Create a `RegistryInstruction::CreateRegistry` instruction
pub fn create_registry(
    owner_address: &Pubkey,
    proof_location: ProofLocation<PubkeyValidityProofData>,
) -> Result<Vec<Instruction>, ProgramError> {
    let elgamal_registry_address = get_elgamal_registry_address(owner_address, &id());

    let mut accounts = vec![
        AccountMeta::new(elgamal_registry_address, false),
        AccountMeta::new_readonly(*owner_address, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];
    let proof_instruction_offset = proof_instruction_offset(&mut accounts, proof_location);

    let mut instructions = vec![Instruction {
        program_id: id(),
        accounts,
        data: RegistryInstruction::CreateRegistry {
            proof_instruction_offset,
        }
        .pack(),
    }];
    append_zk_elgamal_proof(&mut instructions, proof_location)?;
    Ok(instructions)
}

/// Create a `RegistryInstruction::UpdateRegistry` instruction
pub fn update_registry(
    owner_address: &Pubkey,
    proof_location: ProofLocation<PubkeyValidityProofData>,
) -> Result<Vec<Instruction>, ProgramError> {
    let elgamal_registry_address = get_elgamal_registry_address(owner_address, &id());

    let mut accounts = vec![AccountMeta::new(elgamal_registry_address, false)];
    let proof_instruction_offset = proof_instruction_offset(&mut accounts, proof_location);
    accounts.push(AccountMeta::new_readonly(*owner_address, true));

    let mut instructions = vec![Instruction {
        program_id: id(),
        accounts,
        data: RegistryInstruction::UpdateRegistry {
            proof_instruction_offset,
        }
        .pack(),
    }];
    append_zk_elgamal_proof(&mut instructions, proof_location)?;
    Ok(instructions)
}

/// Takes a `ProofLocation`, updates the list of accounts, and returns a
/// suitable proof location
fn proof_instruction_offset(
    accounts: &mut Vec<AccountMeta>,
    proof_location: ProofLocation<PubkeyValidityProofData>,
) -> i8 {
    match proof_location {
        ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) => {
            accounts.push(AccountMeta::new_readonly(sysvar::instructions::id(), false));
            if let ProofData::RecordAccount(record_address, _) = proof_data {
                accounts.push(AccountMeta::new_readonly(*record_address, false));
            }
            proof_instruction_offset.into()
        }
        ProofLocation::ContextStateAccount(context_state_account) => {
            accounts.push(AccountMeta::new_readonly(*context_state_account, false));
            0
        }
    }
}

/// Takes a `RegistryInstruction` and appends the pubkey validity proof
/// instruction
fn append_zk_elgamal_proof(
    instructions: &mut Vec<Instruction>,
    proof_data_location: ProofLocation<PubkeyValidityProofData>,
) -> Result<(), ProgramError> {
    if let ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) =
        proof_data_location
    {
        let proof_instruction_offset: i8 = proof_instruction_offset.into();
        if proof_instruction_offset != 1 {
            return Err(ProgramError::InvalidArgument);
        }
        match proof_data {
            ProofData::InstructionData(data) => instructions
                .push(ProofInstruction::VerifyPubkeyValidity.encode_verify_proof(None, data)),
            ProofData::RecordAccount(address, offset) => instructions.push(
                ProofInstruction::VerifyPubkeyValidity
                    .encode_verify_proof_from_account(None, address, offset),
            ),
        }
    }
    Ok(())
}
