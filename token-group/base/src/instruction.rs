//! Interface base instruction types

use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    spl_discriminator::{ArrayDiscriminator, SplDiscriminate},
    spl_tlv_account_resolution::account::ExtraAccountMeta,
    spl_type_length_value::pod::{pod_slice_to_bytes, PodSlice},
};

/// Instruction data for `Emit`
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize, SplDiscriminate)]
#[discriminator_hash_input("spl_interface_base:emitter")]
pub struct Emit {
    /// Start of range of data to emit
    pub start: Option<u64>,
    /// End of range of data to emit
    pub end: Option<u64>,
}

/// Instruction data for `InitializeExtraAccountMetaList`
#[derive(Clone, Debug, PartialEq, SplDiscriminate)]
#[discriminator_hash_input("spl_interface_base:initialize_extra_account_meta_list")]
pub struct InitializeExtraAccountMetaList {
    /// The instruction discriminator these extra account metas are for
    pub instruction_discriminator: ArrayDiscriminator,
    /// List of `ExtraAccountMeta`s to write into the account
    pub extra_account_metas: Vec<ExtraAccountMeta>,
}
impl InitializeExtraAccountMetaList {
    /// Unpacks a byte buffer into a `InitializeExtraAccountMetaList`
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        if input.len() < ArrayDiscriminator::LENGTH {
            return Err(ProgramError::InvalidInstructionData);
        }
        let (discriminator, rest) = input.split_at(ArrayDiscriminator::LENGTH);
        let instruction_discriminator =
            ArrayDiscriminator::new(discriminator[..8].try_into().unwrap());
        let pod_slice = PodSlice::<ExtraAccountMeta>::unpack(rest)?;
        let extra_account_metas = pod_slice.data().to_vec();
        Ok(Self {
            instruction_discriminator,
            extra_account_metas,
        })
    }

    /// Packs a `InitializeExtraAccountMetaList` into a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = vec![];
        buf.extend_from_slice(self.instruction_discriminator.as_ref());
        buf.extend_from_slice(&(self.extra_account_metas.len() as u32).to_le_bytes());
        buf.extend_from_slice(pod_slice_to_bytes(&self.extra_account_metas));
        buf
    }
}

/// Base instructions that may be used by any on-chain program implementing one
/// or more interfaces.
///
/// Note: Any instruction can be extended using additional required accounts by
/// using the `InitializeExtraAccountMetaList` instruction to write
/// configurations for extra required accounts into validation data
/// corresponding to an instruction's unique discriminator.
#[derive(Clone, Debug, PartialEq)]
pub enum InterfaceBaseInstruction {
    /// Emits the group or member as return data
    ///
    /// The format of the data emitted follows either the `Group` or
    /// `Member` struct,  but it's possible that the account data is stored in
    /// another format by the program.
    ///
    /// With this instruction, a program that implements the token-groups
    /// interface can return `Group` or `Member` without adhering to the
    /// specific byte layout of the structs in any accounts.
    ///
    /// The dictation of which data to emit is determined by the `ItemType`
    /// enum argument to the instruction data.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[]`   Group _or_ Member account
    Emit(Emit),

    /// Initializes the extra account metas on an account, writing into
    /// the first open TLV space.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[w]` Account with extra account metas
    ///   1. `[]` Mint
    ///   2. `[s]` Mint authority
    ///   3. `[]` System program
    InitializeExtraAccountMetaList(InitializeExtraAccountMetaList),
}

impl InterfaceBaseInstruction {
    /// Unpacks a byte buffer into a `InterfaceBaseInstruction`
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        // Should have at least _two_ leading discriminators
        if input.len() < ArrayDiscriminator::LENGTH * 2 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let (discriminator, rest) = input.split_at(ArrayDiscriminator::LENGTH);
        Ok(match discriminator {
            Emit::SPL_DISCRIMINATOR_SLICE => {
                let data = Emit::try_from_slice(rest)?;
                Self::Emit(data)
            }
            InitializeExtraAccountMetaList::SPL_DISCRIMINATOR_SLICE => {
                let data = InitializeExtraAccountMetaList::unpack(rest)?;
                Self::InitializeExtraAccountMetaList(data)
            }
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }

    /// Packs a `InterfaceBaseInstruction` into a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = vec![];
        match self {
            Self::Emit(data) => {
                buf.extend_from_slice(Emit::SPL_DISCRIMINATOR_SLICE);
                buf.append(&mut data.try_to_vec().unwrap());
            }
            Self::InitializeExtraAccountMetaList(data) => {
                buf.extend_from_slice(InitializeExtraAccountMetaList::SPL_DISCRIMINATOR_SLICE);
                buf.append(&mut data.pack());
            }
        };
        buf
    }
}

/// Creates an `Emit` instruction
pub fn emit(
    program_id: &Pubkey,
    item: &Pubkey,
    start: Option<u64>,
    end: Option<u64>,
) -> Instruction {
    let data = InterfaceBaseInstruction::Emit(Emit { start, end }).pack();
    Instruction {
        program_id: *program_id,
        accounts: vec![AccountMeta::new_readonly(*item, false)],
        data,
    }
}

#[cfg(test)]
mod test {
    use {super::*, crate::NAMESPACE, solana_program::hash};

    #[test]
    fn emit_pack() {
        let preimage = hash::hashv(&[format!("{NAMESPACE}:emitter").as_bytes()]);
        let discriminator = &preimage.as_ref()[..ArrayDiscriminator::LENGTH];

        let data = Emit {
            start: None,
            end: Some(10),
        };
        let instruction = InterfaceBaseInstruction::Emit(data.clone());

        let mut expect = vec![];
        expect.extend_from_slice(discriminator.as_ref());
        expect.append(&mut data.try_to_vec().unwrap());
        let packed = instruction.pack();
        assert_eq!(packed, expect);
        let unpacked = InterfaceBaseInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, instruction);
    }

    #[test]
    fn initialize_extra_account_meta_list_pack() {
        let data = InitializeExtraAccountMetaList {
            instruction_discriminator: ArrayDiscriminator::new([0; 8]),
            extra_account_metas: vec![ExtraAccountMeta::new_with_pubkey(
                &Pubkey::new_unique(),
                false,
                false,
            )
            .unwrap()],
        };
        let instruction = InterfaceBaseInstruction::InitializeExtraAccountMetaList(data.clone());
        let preimage =
            hash::hashv(&[format!("{NAMESPACE}:initialize_extra_account_meta_list").as_bytes()]);
        let discriminator = &preimage.as_ref()[..ArrayDiscriminator::LENGTH];
        let mut expect = vec![];
        expect.extend_from_slice(discriminator.as_ref());
        expect.append(&mut data.pack());
        let packed = instruction.pack();
        assert_eq!(packed, expect);
        let unpacked = InterfaceBaseInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, instruction);
    }
}
