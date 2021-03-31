use std::{mem::size_of, str::FromStr};

use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar,
};

use crate::{
    error::MetadataError,
    state::metadata::{NAME_LENGTH, SYMBOL_LENGTH, URI_LENGTH},
};

/// Instructions supported by the Metadata program.
#[derive(Clone)]
pub enum MetadataInstruction {
    /// Create an  Owner and  Metadata objects.
    ///   0. `[writable]`  Owner key (pda of ['metadata', program id, name, symbol])
    ///   1. `[writable]`  metadata key (pda of ['metadata', program id, mint id])
    ///   2. `[]` Mint of
    ///   3. `[signer]` Mint authority
    ///   4. `[signer]` payer
    ///   5. `[]`  metadata program
    ///   6. `[]` System program
    CreateMetadataAccounts {
        /// name
        name: [u8; NAME_LENGTH],
        /// symbol
        symbol: [u8; SYMBOL_LENGTH],
    },

    /// Instantiate an  Owner and  Metadata object.
    ///   0. `[writable]` Uninitialized  Owner account
    ///   1. `[writable]` Uninitialized  Metadata account
    ///   2. `[]` Mint of
    ///   3. `[signer]` Mint authority of
    ///   4. `[]` Owner key
    ///   5. `[]` Rent sysvar
    InitMetadataAccounts {
        /// name
        name: [u8; NAME_LENGTH],
        /// symbol
        symbol: [u8; SYMBOL_LENGTH],
        /// uri
        uri: [u8; URI_LENGTH],
    },

    /// Update an  Metadata (name/symbol are unchangeable)
    ///   0. `[writable]`  Metadata account
    ///   1. `[signer]` Owner key
    ///   2. `[]`  Owner account
    UpdateMetadataAccounts {
        /// uri
        uri: [u8; URI_LENGTH],
    },
}

impl MetadataInstruction {
    /// Unpacks a byte buffer into a [MetadataInstruction](enum.MetadataInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&tag, rest) = input
            .split_first()
            .ok_or(MetadataError::InstructionUnpackError)?;
        Ok(match tag {
            0 => {
                let (input_name, rest) = rest.split_at(NAME_LENGTH);
                let (input_symbol, _rest) = rest.split_at(SYMBOL_LENGTH);
                let mut name: [u8; NAME_LENGTH] = [0; NAME_LENGTH];
                let mut symbol: [u8; SYMBOL_LENGTH] = [0; SYMBOL_LENGTH];

                for n in 0..(NAME_LENGTH - 1) {
                    name[n] = input_name[n];
                }
                for n in 0..(SYMBOL_LENGTH - 1) {
                    symbol[n] = input_symbol[n];
                }
                Self::CreateMetadataAccounts { name, symbol }
            }
            1 => {
                let (input_name, rest) = rest.split_at(NAME_LENGTH);
                let (input_symbol, rest) = rest.split_at(SYMBOL_LENGTH);
                let (input_uri, _rest) = rest.split_at(URI_LENGTH);
                let mut name: [u8; NAME_LENGTH] = [0; NAME_LENGTH];
                let mut symbol: [u8; SYMBOL_LENGTH] = [0; SYMBOL_LENGTH];
                let mut uri: [u8; URI_LENGTH] = [0; URI_LENGTH];

                for n in 0..(NAME_LENGTH - 1) {
                    name[n] = input_name[n];
                }
                for n in 0..(SYMBOL_LENGTH - 1) {
                    symbol[n] = input_symbol[n];
                }
                for n in 0..(URI_LENGTH - 1) {
                    uri[n] = input_uri[n];
                }

                Self::InitMetadataAccounts { name, symbol, uri }
            }
            2 => {
                let (input_uri, _rest) = rest.split_at(URI_LENGTH);
                let mut uri: [u8; URI_LENGTH] = [0; URI_LENGTH];
                for n in 0..(URI_LENGTH - 1) {
                    uri[n] = input_uri[n];
                }

                Self::UpdateMetadataAccounts { uri }
            }
            _ => return Err(MetadataError::InstructionUnpackError.into()),
        })
    }

    /// Packs a [MetadataInstruction](enum.MetadataInstruction.html) into a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());

        match self {
            Self::CreateMetadataAccounts { name, symbol } => {
                buf.push(0);
                buf.extend_from_slice(name);
                buf.extend_from_slice(symbol);
            }
            Self::InitMetadataAccounts { name, symbol, uri } => {
                buf.push(1);
                buf.extend_from_slice(name);
                buf.extend_from_slice(symbol);
                buf.extend_from_slice(uri);
            }
            Self::UpdateMetadataAccounts { uri } => {
                buf.push(2);
                buf.extend_from_slice(uri);
            }
        }
        buf
    }
}

/// Creates an CreateMetadataAccounts instruction
pub fn create_metadata_accounts(
    program_id: Pubkey,
    owner_account: Pubkey,
    metadata_account: Pubkey,
    mint: Pubkey,
    mint_authority: Pubkey,
    payer: Pubkey,
    name_str: &str,
    symbol_str: &str,
) -> Instruction {
    let mut name: [u8; NAME_LENGTH] = [0; NAME_LENGTH];
    let mut symbol: [u8; SYMBOL_LENGTH] = [0; SYMBOL_LENGTH];

    let name_bytes = name_str.as_bytes();
    for n in 0..(NAME_LENGTH - 1) {
        if n < name_bytes.len() {
            name[n] = name_bytes[n];
        }
    }

    let symbol_bytes = symbol_str.as_bytes();
    for n in 0..(SYMBOL_LENGTH - 1) {
        if n < symbol_bytes.len() {
            symbol[n] = symbol_bytes[n];
        }
    }
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new_readonly(owner_account, false),
            AccountMeta::new_readonly(metadata_account, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(mint_authority, true),
            AccountMeta::new_readonly(payer, true),
            AccountMeta::new_readonly(program_id, false),
            AccountMeta::new_readonly(
                Pubkey::from_str("11111111111111111111111111111111").unwrap(),
                false,
            ),
        ],
        data: MetadataInstruction::CreateMetadataAccounts { name, symbol }.pack(),
    }
}

/// Creates an 'InitMetadataAccounts' instruction.
pub fn init_metadata_accounts(
    program_id: Pubkey,
    owner_account: Pubkey,
    metadata_account: Pubkey,
    mint: Pubkey,
    mint_authority: Pubkey,
    owner: Pubkey,
    name_str: &str,
    symbol_str: &str,
    uri_str: &str,
) -> Instruction {
    let mut name: [u8; NAME_LENGTH] = [0; NAME_LENGTH];
    let mut symbol: [u8; SYMBOL_LENGTH] = [0; SYMBOL_LENGTH];
    let mut uri: [u8; URI_LENGTH] = [0; URI_LENGTH];

    let name_bytes = name_str.as_bytes();
    for n in 0..(NAME_LENGTH - 1) {
        if n < name_bytes.len() {
            name[n] = name_bytes[n];
        }
    }

    let symbol_bytes = symbol_str.as_bytes();
    for n in 0..(SYMBOL_LENGTH - 1) {
        if n < symbol_bytes.len() {
            symbol[n] = symbol_bytes[n];
        }
    }

    let uri_bytes = uri_str.as_bytes();
    for n in 0..(URI_LENGTH - 1) {
        if n < uri_bytes.len() {
            uri[n] = uri_bytes[n];
        }
    }

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(owner_account, false),
            AccountMeta::new(metadata_account, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(mint_authority, true),
            AccountMeta::new_readonly(owner, false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: MetadataInstruction::InitMetadataAccounts { name, symbol, uri }.pack(),
    }
}

/// update  metadata account instruction
pub fn update_metadata_accounts(
    program_id: Pubkey,
    metadata_account: Pubkey,
    owner_account: Pubkey,
    owner: Pubkey,
    uri_str: &str,
) -> Instruction {
    let mut uri: [u8; URI_LENGTH] = [0; URI_LENGTH];

    let uri_bytes = uri_str.as_bytes();
    for n in 0..(URI_LENGTH - 1) {
        if n < uri_bytes.len() {
            uri[n] = uri_bytes[n];
        }
    }

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(metadata_account, false),
            AccountMeta::new_readonly(owner, true),
            AccountMeta::new_readonly(owner_account, false),
        ],
        data: MetadataInstruction::UpdateMetadataAccounts { uri }.pack(),
    }
}
