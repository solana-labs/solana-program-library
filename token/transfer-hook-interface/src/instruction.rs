//! Instruction types

use {
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
        system_program,
    },
    spl_type_length_value::discriminator::{Discriminator, TlvDiscriminator},
    std::convert::TryInto,
};

/// Instructions supported by the transfer hook interface.
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum TransferHookInstruction {
    /// Runs additional transfer logic.
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
    Execute {
        /// Amount of tokens to transfer
        amount: u64,
    },
    /// Initializes the extra account metas on an account, writing into
    /// the first open TLV space.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[w]` Account with extra account metas
    ///   1. `[]` Mint
    ///   2. `[s]` Mint authority
    ///   3. `[]` System program
    ///   4..4+M `[]` `M` additional accounts, to be written to validation data
    ///
    InitializeExtraAccountMetas,
}
/// TLV instruction type only used to define the discriminator. The actual data
/// is entirely managed by `ExtraAccountMetas`, and it is the only data contained
/// by this type.
pub struct ExecuteInstruction;
impl TlvDiscriminator for ExecuteInstruction {
    /// Please use this discriminator in your program when matching
    const TLV_DISCRIMINATOR: Discriminator = Discriminator::new(EXECUTE_DISCRIMINATOR);
}
/// First 8 bytes of `hash::hashv(&["spl-transfer-hook-interface:execute"])`
const EXECUTE_DISCRIMINATOR: [u8; Discriminator::LENGTH] = [105, 37, 101, 197, 75, 251, 102, 26];
// annoying, but needed to perform a match on the value
const EXECUTE_DISCRIMINATOR_SLICE: &[u8] = &EXECUTE_DISCRIMINATOR;
/// First 8 bytes of `hash::hashv(&["spl-transfer-hook-interface:initialize-extra-account-metas"])`
const INITIALIZE_EXTRA_ACCOUNT_METAS_DISCRIMINATOR: &[u8] = &[43, 34, 13, 49, 167, 88, 235, 235];

impl TransferHookInstruction {
    /// Unpacks a byte buffer into a [TransferHookInstruction](enum.TransferHookInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        if input.len() < Discriminator::LENGTH {
            return Err(ProgramError::InvalidInstructionData);
        }
        let (discriminator, rest) = input.split_at(Discriminator::LENGTH);
        Ok(match discriminator {
            EXECUTE_DISCRIMINATOR_SLICE => {
                let amount = rest
                    .get(..8)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u64::from_le_bytes)
                    .ok_or(ProgramError::InvalidInstructionData)?;
                Self::Execute { amount }
            }
            INITIALIZE_EXTRA_ACCOUNT_METAS_DISCRIMINATOR => Self::InitializeExtraAccountMetas,
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }

    /// Packs a [TokenInstruction](enum.TokenInstruction.html) into a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = vec![];
        match self {
            Self::Execute { amount } => {
                buf.extend_from_slice(EXECUTE_DISCRIMINATOR_SLICE);
                buf.extend_from_slice(&amount.to_le_bytes());
            }
            Self::InitializeExtraAccountMetas => {
                buf.extend_from_slice(INITIALIZE_EXTRA_ACCOUNT_METAS_DISCRIMINATOR);
            }
        };
        buf
    }
}

/// Creates an `Execute` instruction, provided all of the additional required
/// account metas
#[allow(clippy::too_many_arguments)]
pub fn execute_with_extra_account_metas(
    program_id: &Pubkey,
    source_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    validate_state_pubkey: &Pubkey,
    additional_accounts: &[AccountMeta],
    amount: u64,
) -> Instruction {
    let mut instruction = execute(
        program_id,
        source_pubkey,
        mint_pubkey,
        destination_pubkey,
        authority_pubkey,
        validate_state_pubkey,
        amount,
    );
    instruction.accounts.extend_from_slice(additional_accounts);
    instruction
}

/// Creates an `Execute` instruction, without the additional accounts
#[allow(clippy::too_many_arguments)]
pub fn execute(
    program_id: &Pubkey,
    source_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    validate_state_pubkey: &Pubkey,
    amount: u64,
) -> Instruction {
    let data = TransferHookInstruction::Execute { amount }.pack();
    let accounts = vec![
        AccountMeta::new_readonly(*source_pubkey, false),
        AccountMeta::new_readonly(*mint_pubkey, false),
        AccountMeta::new_readonly(*destination_pubkey, false),
        AccountMeta::new_readonly(*authority_pubkey, false),
        AccountMeta::new_readonly(*validate_state_pubkey, false),
    ];
    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

/// Creates a `InitializeExtraAccountMetas` instruction.
pub fn initialize_extra_account_metas(
    program_id: &Pubkey,
    extra_account_metas_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    additional_accounts: &[AccountMeta],
) -> Instruction {
    let data = TransferHookInstruction::InitializeExtraAccountMetas.pack();

    let mut accounts = vec![
        AccountMeta::new(*extra_account_metas_pubkey, false),
        AccountMeta::new_readonly(*mint_pubkey, false),
        AccountMeta::new_readonly(*authority_pubkey, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];
    accounts.extend_from_slice(additional_accounts);

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
        let check = TransferHookInstruction::Execute { amount };
        let packed = check.pack();
        // Please use ExecuteInstruction::TLV_DISCRIMINATOR in your program, the
        // following is just for test purposes
        let preimage = hash::hashv(&[format!("{NAMESPACE}:execute").as_bytes()]);
        let discriminator = &preimage.as_ref()[..Discriminator::LENGTH];
        let mut expect = vec![];
        expect.extend_from_slice(discriminator.as_ref());
        expect.extend_from_slice(&amount.to_le_bytes());
        assert_eq!(packed, expect);
        let unpacked = TransferHookInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);
    }

    #[test]
    fn initialize_validation_pubkeys_packing() {
        let check = TransferHookInstruction::InitializeExtraAccountMetas;
        let packed = check.pack();
        // Please use INITIALIZE_EXTRA_ACCOUNT_METAS_DISCRIMINATOR in your program,
        // the following is just for test purposes
        let preimage =
            hash::hashv(&[format!("{NAMESPACE}:initialize-extra-account-metas").as_bytes()]);
        let discriminator = &preimage.as_ref()[..Discriminator::LENGTH];
        let mut expect = vec![];
        expect.extend_from_slice(discriminator.as_ref());
        assert_eq!(packed, expect);
        let unpacked = TransferHookInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);
    }
}
