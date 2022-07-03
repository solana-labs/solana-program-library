//! RefDB management instructions.

use {
    crate::{
        pack::{
            as64_deserialize, as64_serialize, check_data_len, pack_array_string64, pack_option_u32,
            unpack_array_string64, unpack_bool, unpack_option_u32,
        },
        refdb::{Record, Reference, ReferenceType},
        string::ArrayString64,
    },
    arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs},
    num_enum::TryFromPrimitive,
    serde::{Deserialize, Serialize},
    solana_program::program_error::ProgramError,
};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum RefDbInstruction {
    /// Initialize on-chain RefDB storage
    ///
    /// # Account references
    ///   0. [SIGNER] Funding account, must be main router admin
    ///   1. [WRITE] RefDB storage PDA
    ///   3. [] Sytem program
    Init {
        #[serde(
            serialize_with = "as64_serialize",
            deserialize_with = "as64_deserialize"
        )]
        name: ArrayString64,
        reference_type: ReferenceType,
        max_records: u32,
        init_account: bool,
    },

    /// Delete on-chain RefDB storage
    ///
    /// # Account references
    ///   0. [SIGNER] Funding account, must be main router admin
    ///   1. [WRITE] RefDB storage PDA
    ///   3. [] Sytem program
    Drop { close_account: bool },

    /// Write the record into on-chain RefDB storage
    ///
    /// # Account references
    ///   0. [SIGNER] Funding account, must be main router admin
    ///   1. [WRITE] RefDB storage PDA
    ///   3. [] Sytem program
    Write { record: Record },

    /// Delete the record from on-chain RefDB storage
    ///
    /// # Account references
    ///   0. [SIGNER] Funding account, must be main router admin
    ///   1. [WRITE] RefDB storage PDA
    ///   3. [] Sytem program
    Delete { record: Record },
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum RefDbInstructionType {
    Init,
    Drop,
    Write,
    Delete,
}

impl RefDbInstruction {
    pub const MAX_LEN: usize = Record::MAX_LEN + 7;
    pub const INIT_LEN: usize = 71;
    pub const DROP_LEN: usize = 3;
    pub const WRITE_MAX_LEN: usize = Record::MAX_LEN + 7;
    pub const DELETE_MAX_LEN: usize = Record::MAX_LEN + 7;

    pub fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        match self {
            Self::Init {
                name,
                reference_type,
                max_records,
                init_account,
            } => {
                check_data_len(output, RefDbInstruction::INIT_LEN)?;

                output[0] = RefDbInstructionType::Init as u8;
                output[1] = *reference_type as u8;

                let output = array_mut_ref![output, 2, RefDbInstruction::INIT_LEN - 2];

                let (name_out, max_records_out, init_account_out) =
                    mut_array_refs![output, 64, 4, 1];
                pack_array_string64(name, name_out);
                *max_records_out = max_records.to_le_bytes();
                init_account_out[0] = *init_account as u8;

                Ok(RefDbInstruction::INIT_LEN)
            }
            Self::Drop { close_account } => {
                check_data_len(output, RefDbInstruction::DROP_LEN)?;
                output[0] = RefDbInstructionType::Drop as u8;
                output[1] = ReferenceType::Empty as u8;
                output[2] = *close_account as u8;
                Ok(RefDbInstruction::DROP_LEN)
            }
            Self::Write { record } => {
                check_data_len(output, 7)?;

                let header = array_mut_ref![output, 0, 7];
                let (instruction_out, reference_type_out, index_out) =
                    mut_array_refs![header, 1, 1, 5];

                instruction_out[0] = RefDbInstructionType::Write as u8;
                reference_type_out[0] = match record.reference {
                    Reference::Pubkey { .. } => ReferenceType::Pubkey as u8,
                    Reference::U8 { .. } => ReferenceType::U8 as u8,
                    Reference::U16 { .. } => ReferenceType::U16 as u8,
                    Reference::U32 { .. } => ReferenceType::U32 as u8,
                    Reference::U64 { .. } => ReferenceType::U64 as u8,
                    Reference::F64 { .. } => ReferenceType::F64 as u8,
                    Reference::Empty => ReferenceType::Empty as u8,
                };
                pack_option_u32(record.index, index_out);
                record.pack(&mut output[7..])?;

                Ok(7 + record.get_size())
            }
            Self::Delete { record } => {
                check_data_len(output, 7)?;

                let header = array_mut_ref![output, 0, 7];
                let (instruction_out, reference_type_out, index_out) =
                    mut_array_refs![header, 1, 1, 5];

                instruction_out[0] = RefDbInstructionType::Delete as u8;
                reference_type_out[0] = match record.reference {
                    Reference::Pubkey { .. } => ReferenceType::Pubkey as u8,
                    Reference::U8 { .. } => ReferenceType::U8 as u8,
                    Reference::U16 { .. } => ReferenceType::U16 as u8,
                    Reference::U32 { .. } => ReferenceType::U32 as u8,
                    Reference::U64 { .. } => ReferenceType::U64 as u8,
                    Reference::F64 { .. } => ReferenceType::F64 as u8,
                    Reference::Empty => ReferenceType::Empty as u8,
                };
                pack_option_u32(record.index, index_out);
                record.pack(&mut output[7..])?;

                Ok(7 + record.get_size())
            }
        }
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; RefDbInstruction::MAX_LEN] = [0; RefDbInstruction::MAX_LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    pub fn unpack(input: &[u8]) -> Result<RefDbInstruction, ProgramError> {
        check_data_len(input, 3)?;
        let instruction_type = RefDbInstructionType::try_from_primitive(input[0])
            .or(Err(ProgramError::InvalidInstructionData))?;
        let reference_type = ReferenceType::try_from_primitive(input[1])
            .or(Err(ProgramError::InvalidInstructionData))?;
        match instruction_type {
            RefDbInstructionType::Init => {
                check_data_len(input, RefDbInstruction::INIT_LEN)?;

                let input = array_ref![input, 2, RefDbInstruction::INIT_LEN - 2];
                #[allow(clippy::ptr_offset_with_cast)]
                let (name, max_records, init_account) = array_refs![input, 64, 4, 1];

                Ok(RefDbInstruction::Init {
                    name: unpack_array_string64(name)?,
                    reference_type,
                    max_records: u32::from_le_bytes(*max_records),
                    init_account: unpack_bool(init_account)?,
                })
            }
            RefDbInstructionType::Drop => Ok(RefDbInstruction::Drop {
                close_account: unpack_bool(&[input[2]])?,
            }),
            RefDbInstructionType::Write => {
                check_data_len(input, 7)?;
                let index = array_ref![input, 2, 5];
                Ok(RefDbInstruction::Write {
                    record: Record::unpack(&input[7..], reference_type, unpack_option_u32(index)?)?,
                })
            }
            RefDbInstructionType::Delete => {
                check_data_len(input, 7)?;
                let index = array_ref![input, 2, 5];
                Ok(RefDbInstruction::Delete {
                    record: Record::unpack(&input[7..], reference_type, unpack_option_u32(index)?)?,
                })
            }
        }
    }
}

impl std::fmt::Display for RefDbInstructionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            RefDbInstructionType::Init => write!(f, "Init"),
            RefDbInstructionType::Drop => write!(f, "Drop"),
            RefDbInstructionType::Write => write!(f, "Write"),
            RefDbInstructionType::Delete => write!(f, "Delete"),
        }
    }
}
