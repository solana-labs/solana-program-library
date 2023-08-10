//! Program instructions

use {
    borsh::{BorshDeserialize, BorshSchema, BorshSerialize},
    solana_program::{
        feature::Feature,
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        rent::Rent,
        system_instruction, system_program,
    },
};

/// Feature Gate program instructions
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, BorshSchema, PartialEq)]
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

/// Creates an 'Activate' instruction.
pub fn activate(program_id: &Pubkey, feature: &Pubkey, authority: &Pubkey) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*feature, true),
        AccountMeta::new_readonly(*authority, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Instruction {
        program_id: *program_id,
        accounts,
        data: FeatureGateInstruction::Activate.try_to_vec().unwrap(),
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

    Instruction {
        program_id: *program_id,
        accounts,
        data: FeatureGateInstruction::RevokePendingActivation
            .try_to_vec()
            .unwrap(),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_pack_unpack(instruction: &FeatureGateInstruction) {
        let packed = instruction.try_to_vec().unwrap();
        let unpacked = FeatureGateInstruction::try_from_slice(&packed).unwrap();
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
