//! Program instructions

use solana_program::{
    feature::Feature,
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction, system_program,
};

/// Feature Gate program instructions
#[derive(Clone, Debug, PartialEq)]
pub enum FeatureGateInstruction {
    /// Submit a feature for activation.
    ///
    /// Note: This instruction expects the account to exist and be owned by the
    /// system program. The account should also have enough rent-exempt lamports
    /// to cover the cost of the account creation for a
    /// `solana_program::feature::Feature` state prior to invoking this
    /// instruction.
    ///
    /// For this instruction, one must sign the transaction with the feature
    /// keypair. No additional authority is required.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[w+s]`    Feature account (must be a system account)
    ///   1. `[]`       System program
    ActivateFeature,
    /// Submit a feature for activation using an authority signer.
    ///
    /// Note: This instruction expects the account to exist and be owned by the
    /// system program. The account should also have enough rent-exempt lamports
    /// to cover the cost of the account creation for a
    /// `solana_program::feature::Feature` state prior to invoking this
    /// instruction.
    ///
    /// For this instruction, some authority - which can be a multisig - must
    /// sign the transaction. The feature account is a PDA and is not required
    /// to sign the transaction.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[w]`      Feature account (must be a system account)
    ///   1. `[s]`      Feature activation authority (can be multisig)
    ///   2. `[]`       System program
    ActivateFeatureWithAuthority {
        /// The nonce used to derive the feature ID.
        nonce: u16,
    },
    /// Revoke a pending feature activation.
    ///
    /// A "pending" feature activation is a feature account that has been
    /// allocated and assigned, but hasn't yet been updated by the runtime
    /// with an `activation_slot`.
    ///
    /// Features that _have_ been activated by the runtime cannot be revoked.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[w+s]`    Feature account
    ///   1. `[w]`      Destination (for rent lamports)
    RevokePendingActivation,
}
impl FeatureGateInstruction {
    /// Unpacks a byte buffer into a
    /// [FeatureGateInstruction](enum.FeatureGateInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        if input.is_empty() {
            return Err(ProgramError::InvalidInstructionData);
        }
        let (instruction, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;
        match instruction {
            0 => Ok(Self::ActivateFeature),
            1 => {
                if rest.len() != 2 {
                    return Err(ProgramError::InvalidInstructionData);
                }
                let nonce = u16::from_le_bytes([rest[0], rest[1]]);
                Ok(Self::ActivateFeatureWithAuthority { nonce })
            }
            2 => Ok(Self::RevokePendingActivation),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }

    /// Packs a [FeatureGateInstruction](enum.FeatureGateInstruction.html) into
    /// a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(3);
        match self {
            Self::ActivateFeature => buf.push(0),
            Self::ActivateFeatureWithAuthority { nonce } => {
                buf.push(1);
                buf.extend_from_slice(&nonce.to_le_bytes());
            }
            Self::RevokePendingActivation => buf.push(2),
        }
        buf
    }
}

/// Creates an 'ActivateFeature' instruction.
pub fn activate_feature(feature_id: &Pubkey) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*feature_id, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    let data = FeatureGateInstruction::ActivateFeature.pack();

    Instruction {
        program_id: crate::id(),
        accounts,
        data,
    }
}

/// Creates a set of two instructions:
///   * One to fund the feature account with rent-exempt lamports
///   * Another is the Feature Gate Program's 'ActivateFeature' instruction
pub fn activate_feature_with_rent_transfer(
    feature_id: &Pubkey,
    payer: &Pubkey,
) -> [Instruction; 2] {
    let lamports = Rent::default().minimum_balance(Feature::size_of());
    [
        system_instruction::transfer(payer, feature_id, lamports),
        activate_feature(feature_id),
    ]
}

/// Creates an 'ActivateFeatureWithAuthority' instruction.
pub fn activate_feature_with_authority(
    feature_id: &Pubkey,
    authority: &Pubkey,
    nonce: u16,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*feature_id, false),
        AccountMeta::new_readonly(*authority, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    let data = FeatureGateInstruction::ActivateFeatureWithAuthority { nonce }.pack();

    Instruction {
        program_id: crate::id(),
        accounts,
        data,
    }
}

/// Creates a set of two instructions:
///   * One to fund the feature account with rent-exempt lamports
///   * Another is the Feature Gate Program's 'ActivateFeatureWithAuthority'
///     instruction
pub fn activate_feature_with_authority_with_rent_transfer(
    feature_id: &Pubkey,
    authority: &Pubkey,
    nonce: u16,
) -> [Instruction; 2] {
    let lamports = Rent::default().minimum_balance(Feature::size_of());
    [
        system_instruction::transfer(authority, feature_id, lamports),
        activate_feature_with_authority(feature_id, authority, nonce),
    ]
}

/// Creates a 'RevokePendingActivation' instruction.
pub fn revoke_pending_activation(feature_id: &Pubkey, destination: &Pubkey) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*feature_id, true),
        AccountMeta::new(*destination, false),
    ];

    let data = FeatureGateInstruction::RevokePendingActivation.pack();

    Instruction {
        program_id: crate::id(),
        accounts,
        data,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_pack_unpack(instruction: &FeatureGateInstruction) {
        let packed = instruction.pack();
        let unpacked = FeatureGateInstruction::unpack(&packed).unwrap();
        assert_eq!(instruction, &unpacked);
    }

    #[test]
    fn test_pack_unpack_activate_feature() {
        test_pack_unpack(&FeatureGateInstruction::ActivateFeature);
    }

    #[test]
    fn test_pack_unpack_activate_feature_with_authority() {
        test_pack_unpack(&FeatureGateInstruction::ActivateFeatureWithAuthority { nonce: 8u16 });
    }

    #[test]
    fn test_pack_unpack_revoke_pending_activation() {
        test_pack_unpack(&FeatureGateInstruction::RevokePendingActivation);
    }
}
