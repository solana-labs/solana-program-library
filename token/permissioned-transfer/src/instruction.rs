//! Instruction types

use {
    crate::DISCRIMINATOR_LENGTH,
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    std::convert::TryInto,
};

/// Instructions supported by the permissioned transfer program.
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum PermissionedTransferInstruction {
    /// Validates transfer accounts and additional required accounts.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[]` Source account
    ///   1. `[]` Token mint
    ///   2. `[]` Destination account
    ///   3. `[]` Source account's owner/delegate
    ///   4. `[]` Validation account
    ///   5..5+M `[]` `M` additional accounts, written in validation account data
    ///
    Validate {
        /// Amount of tokens to transfer
        amount: u64,
    },
    /// Initializes the validate pubkeys struct on an account, writing into
    /// the first open TLV space.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[w]` Validate transfer account
    ///   1. `[]` Mint
    ///   2. `[s]` Mint authority
    ///   3..3+M `[]` `M` additional accounts, to be written to validation data
    ///
    InitializeValidationPubkeys,
}
/// First 8 bytes of `hash::hashv(&["permissioned-transfer:validate"])`
const VALIDATE_DISCRIMINATOR: &[u8] = &[242, 240, 55, 155, 72, 84, 63, 231];
/// First 8 bytes of `hash::hashv(&["permissioned-transfer:initialize-validation-pubkeys"])`
const INITIALIZE_VALIDATION_PUBKEYS_DISCRIMINATOR: &[u8] = &[248, 8, 136, 21, 37, 96, 4, 61];

impl PermissionedTransferInstruction {
    /// Unpacks a byte buffer into a [PermissionedTransferInstruction](enum.PermissionedTransferInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (discriminator, rest) = input.split_at(DISCRIMINATOR_LENGTH);
        Ok(match discriminator {
            VALIDATE_DISCRIMINATOR => {
                let amount = rest
                    .get(..8)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u64::from_le_bytes)
                    .ok_or(ProgramError::InvalidInstructionData)?;
                Self::Validate { amount }
            }
            INITIALIZE_VALIDATION_PUBKEYS_DISCRIMINATOR => Self::InitializeValidationPubkeys,
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }

    /// Packs a [TokenInstruction](enum.TokenInstruction.html) into a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = vec![];
        match self {
            Self::Validate { amount } => {
                buf.extend_from_slice(VALIDATE_DISCRIMINATOR);
                buf.extend_from_slice(&amount.to_le_bytes());
            }
            Self::InitializeValidationPubkeys => {
                buf.extend_from_slice(INITIALIZE_VALIDATION_PUBKEYS_DISCRIMINATOR);
            }
        };
        buf
    }
}

/// Creates a `Validate` instruction.
#[allow(clippy::too_many_arguments)]
pub fn validate(
    program_id: &Pubkey,
    source_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    validate_state_pubkey: &Pubkey,
    additional_pubkeys: &[&Pubkey],
    amount: u64,
) -> Instruction {
    let data = PermissionedTransferInstruction::Validate { amount }.pack();

    let mut accounts = vec![
        AccountMeta::new_readonly(*source_pubkey, false),
        AccountMeta::new_readonly(*mint_pubkey, false),
        AccountMeta::new_readonly(*destination_pubkey, false),
        AccountMeta::new_readonly(*authority_pubkey, false),
        AccountMeta::new_readonly(*validate_state_pubkey, false),
    ];
    accounts.extend(
        additional_pubkeys
            .iter()
            .map(|pk| AccountMeta::new_readonly(**pk, false)),
    );

    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

/// Creates a `InitializeValidationPubkeys` instruction.
pub fn initialize_validation_pubkeys(
    program_id: &Pubkey,
    validate_state_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    additional_pubkeys: &[&Pubkey],
) -> Instruction {
    let data = PermissionedTransferInstruction::InitializeValidationPubkeys.pack();

    let mut accounts = vec![
        AccountMeta::new(*validate_state_pubkey, false),
        AccountMeta::new_readonly(*mint_pubkey, false),
        AccountMeta::new_readonly(*authority_pubkey, false),
    ];
    accounts.extend(
        additional_pubkeys
            .iter()
            .map(|pk| AccountMeta::new_readonly(**pk, false)),
    );

    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

#[cfg(test)]
mod test {
    use {super::*, crate::NAMESPACE, solana_program::hash};

    #[test]
    fn validate_packing() {
        let amount = 111_111_111;
        let check = PermissionedTransferInstruction::Validate { amount };
        let packed = check.pack();
        let preimage = hash::hashv(&[format!("{NAMESPACE}:validate").as_bytes()]);
        let discriminator = &preimage.as_ref()[..DISCRIMINATOR_LENGTH];
        let mut expect = vec![];
        expect.extend_from_slice(discriminator.as_ref());
        expect.extend_from_slice(&amount.to_le_bytes());
        assert_eq!(packed, expect);
        let unpacked = PermissionedTransferInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);
    }

    #[test]
    fn initialize_validation_pubkeys_packing() {
        let check = PermissionedTransferInstruction::InitializeValidationPubkeys;
        let packed = check.pack();
        let preimage =
            hash::hashv(&[format!("{NAMESPACE}:initialize-validation-pubkeys").as_bytes()]);
        let discriminator = &preimage.as_ref()[..DISCRIMINATOR_LENGTH];
        let mut expect = vec![];
        expect.extend_from_slice(discriminator.as_ref());
        assert_eq!(packed, expect);
        let unpacked = PermissionedTransferInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);
    }
}
