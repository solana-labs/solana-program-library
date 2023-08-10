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
    /// Accounts expected by this instruction:
    ///
    ///   0. `[ws]` Feature account (must be a system account)
    ///   1. `[s]` Authority
    ///   2. `[]` System program
    Activate,
    /// Revoke a pending feature activation.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[w]` Feature account
    ///   1. `[w]` Destination (for rent lamports)
    ///   2. `[s]` Authority
    RevokePendingActivation,
}
impl FeatureGateInstruction {
    /// Unpacks a byte buffer into a
    /// [FeatureGateInstruction](enum.FeatureGateInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        if input.is_empty() {
            return Err(ProgramError::InvalidInstructionData);
        }
        match input[0] {
            0 => Ok(Self::Activate),
            1 => Ok(Self::RevokePendingActivation),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }

    /// Packs a [FeatureGateInstruction](enum.FeatureGateInstruction.html) into
    /// a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        match self {
            Self::Activate => vec![0],
            Self::RevokePendingActivation => vec![1],
        }
    }
}

/// Creates an 'Activate' instruction.
pub fn activate(program_id: &Pubkey, feature: &Pubkey, authority: &Pubkey) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*feature, true),
        AccountMeta::new_readonly(*authority, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    let data = FeatureGateInstruction::Activate.pack();

    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

/// Creates a set of two instructions:
///   * One to fund the feature account with rent-exempt lamports
///   * Another is the Feature Gate Program's 'Activate' instruction
pub fn activate_with_rent_transfer(
    program_id: &Pubkey,
    feature: &Pubkey,
    authority: &Pubkey,
    payer: &Pubkey,
) -> [Instruction; 2] {
    let lamports = Rent::default().minimum_balance(Feature::size_of());
    [
        system_instruction::transfer(payer, feature, lamports),
        activate(program_id, feature, authority),
    ]
}

/// Creates a 'RevokePendingActivation' instruction.
pub fn revoke(
    program_id: &Pubkey,
    feature: &Pubkey,
    destination: &Pubkey,
    authority: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*feature, false),
        AccountMeta::new(*destination, false),
        AccountMeta::new_readonly(*authority, false),
    ];

    let data = FeatureGateInstruction::RevokePendingActivation.pack();

    Instruction {
        program_id: *program_id,
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
    fn test_pack_unpack_activate() {
        test_pack_unpack(&FeatureGateInstruction::Activate);
    }

    #[test]
    fn test_pack_unpack_revoke() {
        test_pack_unpack(&FeatureGateInstruction::RevokePendingActivation);
    }
}
