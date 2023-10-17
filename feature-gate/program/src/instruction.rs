//! Program instructions

use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_program,
};

/// Feature Gate program instructions
#[derive(Clone, Debug, PartialEq)]
pub enum FeatureGateInstruction {
    /// Queue a feature for activation by allocating and assigning a feature
    /// account.
    ///
    /// Note: This instruction expects the account to be owned by the system
    /// program.
    ///
    /// If an optional feature activation authority is provided, then the
    /// feature account will be a PDA derived from the authority and the
    /// provided nonce.
    /// Therefore, the feature account is not required to sign the transaction
    /// if the authority is provided.
    /// A nonce is required if the authority is provided.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[w+s]`    Feature account (must be a system account)
    ///   1. `[w+s]`    Payer (for rent lamports)
    ///   2. `[]`       System program
    ///
    /// -- or --
    ///
    ///   0. `[w]`      Feature account (must be a system account)
    ///   1. `[w+s]`    Payer (for rent lamports)
    ///   2. `[]`       System program
    ///   3. `[s]`      Feature activation authority
    ActivateFeature {
        /// The nonce used to derive the feature ID.
        nonce: Option<u16>,
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
            0 => {
                if rest.is_empty() {
                    return Err(ProgramError::InvalidInstructionData);
                }
                if rest[0] == 0 {
                    Ok(Self::ActivateFeature { nonce: None })
                } else if rest[0] == 1 {
                    if rest.len() != 3 {
                        return Err(ProgramError::InvalidInstructionData);
                    }
                    let nonce = u16::from_le_bytes([rest[1], rest[2]]);
                    Ok(Self::ActivateFeature { nonce: Some(nonce) })
                } else {
                    Err(ProgramError::InvalidInstructionData)
                }
            }
            1 => Ok(Self::RevokePendingActivation),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }

    /// Packs a [FeatureGateInstruction](enum.FeatureGateInstruction.html) into
    /// a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(3);
        match self {
            Self::ActivateFeature { nonce } => {
                buf.push(0);
                if let Some(nonce) = nonce {
                    buf.push(1);
                    buf.extend_from_slice(&nonce.to_le_bytes());
                } else {
                    buf.push(0);
                }
            }
            Self::RevokePendingActivation => buf.push(1),
        }
        buf
    }
}

/// Creates an 'ActivateFeature' instruction.
pub fn activate_feature(
    feature_id: &Pubkey,
    payer: &Pubkey,
    authority_with_nonce: Option<(&Pubkey, u16)>,
) -> Instruction {
    let (accounts, data) = if let Some((authority, nonce)) = authority_with_nonce {
        (
            vec![
                AccountMeta::new(*feature_id, false),
                AccountMeta::new(*payer, true),
                AccountMeta::new_readonly(system_program::id(), false),
                AccountMeta::new_readonly(*authority, true),
            ],
            FeatureGateInstruction::ActivateFeature { nonce: Some(nonce) }.pack(),
        )
    } else {
        (
            vec![
                AccountMeta::new(*feature_id, true),
                AccountMeta::new(*payer, true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            FeatureGateInstruction::ActivateFeature { nonce: None }.pack(),
        )
    };

    Instruction {
        program_id: crate::id(),
        accounts,
        data,
    }
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
        test_pack_unpack(&FeatureGateInstruction::ActivateFeature { nonce: None });
        test_pack_unpack(&FeatureGateInstruction::ActivateFeature { nonce: Some(16) });
    }

    #[test]
    fn test_pack_unpack_revoke_pending_activation() {
        test_pack_unpack(&FeatureGateInstruction::RevokePendingActivation);
    }
}
