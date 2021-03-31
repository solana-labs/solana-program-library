use std::{mem::size_of, str::FromStr};

use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar,
};

use crate::{
    error::NFTMetadataError,
    state::nft_metadata::{
        CATEGORY_LENGTH, CREATOR_LENGTH, NAME_LENGTH, SYMBOL_LENGTH, URI_LENGTH,
    },
};

/// Used for telling caller what type of format you want back
#[derive(Clone, PartialEq)]
pub enum Format {
    /// JSON format
    JSON,
    /// MsgPack format
    MsgPack,
}
impl Default for Format {
    fn default() -> Self {
        Format::JSON
    }
}

/// Instructions supported by the NFTMetadata program.
#[derive(Clone)]
pub enum NFTMetadataInstruction {
    /// Create an NFT Owner and NFT Metadata objects.
    ///   0. `[writable]` NFT Owner key (pda of ['metadata', program id, name, symbol])
    ///   1. `[writable]` NFT metadata key (pda of ['metadata', program id, mint id])
    ///   2. `[]` Mint of NFT
    ///   3. `[signer]` Mint authority
    ///   4. `[signer]` payer
    ///   5. `[]` NFT metadata program
    ///   6. `[]` System program
    CreateNFTMetadataAccounts {
        /// name
        name: [u8; NAME_LENGTH],
        /// symbol
        symbol: [u8; SYMBOL_LENGTH],
    },

    /// Instantiate an NFT Owner and NFT Metadata object.
    ///   0. `[writable]` Uninitialized NFT Owner account
    ///   1. `[writable]` Uninitialized NFT Metadata account
    ///   2. `[]` Mint of NFT
    ///   3. `[signer]` Mint authority of NFT
    ///   4. `[]` Owner key
    ///   5. `[]` Rent sysvar
    InitNFTMetadataAccounts {
        /// name
        name: [u8; NAME_LENGTH],
        /// symbol
        symbol: [u8; SYMBOL_LENGTH],
        /// uri
        uri: [u8; URI_LENGTH],
        /// category
        category: [u8; CATEGORY_LENGTH],
        /// creator (optional)
        creator: [u8; CREATOR_LENGTH],
    },

    /// Update an NFT Metadata (name/symbol are unchangeable)
    ///   0. `[writable]` NFT Metadata account
    ///   1. `[signer]` Owner key
    ///   2. `[]` NFT Owner account
    UpdateNFTMetadataAccounts {
        /// uri
        uri: [u8; URI_LENGTH],
        /// category
        category: [u8; CATEGORY_LENGTH],
        /// creator (optional)
        creator: [u8; CREATOR_LENGTH],
    },
}

impl NFTMetadataInstruction {
    /// Unpacks a byte buffer into a [NFTMetadataInstruction](enum.NFTMetadataInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&tag, rest) = input
            .split_first()
            .ok_or(NFTMetadataError::InstructionUnpackError)?;
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
                Self::CreateNFTMetadataAccounts { name, symbol }
            }
            1 => {
                let (input_name, rest) = rest.split_at(NAME_LENGTH);
                let (input_symbol, rest) = rest.split_at(SYMBOL_LENGTH);
                let (input_uri, rest) = rest.split_at(URI_LENGTH);
                let (input_category, rest) = rest.split_at(CATEGORY_LENGTH);
                let (input_creator, _rest) = rest.split_at(CREATOR_LENGTH);
                let mut name: [u8; NAME_LENGTH] = [0; NAME_LENGTH];
                let mut symbol: [u8; SYMBOL_LENGTH] = [0; SYMBOL_LENGTH];
                let mut uri: [u8; URI_LENGTH] = [0; URI_LENGTH];
                let mut category: [u8; CATEGORY_LENGTH] = [0; CATEGORY_LENGTH];
                let mut creator: [u8; CREATOR_LENGTH] = [0; CREATOR_LENGTH];

                for n in 0..(NAME_LENGTH - 1) {
                    name[n] = input_name[n];
                }
                for n in 0..(SYMBOL_LENGTH - 1) {
                    symbol[n] = input_symbol[n];
                }
                for n in 0..(URI_LENGTH - 1) {
                    uri[n] = input_uri[n];
                }
                for n in 0..(CATEGORY_LENGTH - 1) {
                    category[n] = input_category[n];
                }
                for n in 0..(CREATOR_LENGTH - 1) {
                    creator[n] = input_creator[n];
                }

                Self::InitNFTMetadataAccounts {
                    name,
                    symbol,
                    uri,
                    category,
                    creator,
                }
            }
            2 => {
                let (input_uri, rest) = rest.split_at(URI_LENGTH);
                let (input_category, rest) = rest.split_at(CATEGORY_LENGTH);
                let (input_creator, _rest) = rest.split_at(CREATOR_LENGTH);
                let mut uri: [u8; URI_LENGTH] = [0; URI_LENGTH];
                let mut category: [u8; CATEGORY_LENGTH] = [0; CATEGORY_LENGTH];
                let mut creator: [u8; CREATOR_LENGTH] = [0; CREATOR_LENGTH];

                for n in 0..(URI_LENGTH - 1) {
                    uri[n] = input_uri[n];
                }
                for n in 0..(CATEGORY_LENGTH - 1) {
                    category[n] = input_category[n];
                }
                for n in 0..(CREATOR_LENGTH - 1) {
                    creator[n] = input_creator[n];
                }

                Self::UpdateNFTMetadataAccounts {
                    uri,
                    category,
                    creator,
                }
            }
            _ => return Err(NFTMetadataError::InstructionUnpackError.into()),
        })
    }

    /// Packs a [NFTMetadataInstruction](enum.NFTMetadataInstruction.html) into a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());

        match self {
            Self::CreateNFTMetadataAccounts { name, symbol } => {
                buf.push(0);
                buf.extend_from_slice(name);
                buf.extend_from_slice(symbol);
            }
            Self::InitNFTMetadataAccounts {
                name,
                symbol,
                uri,
                category,
                creator,
            } => {
                buf.push(1);
                buf.extend_from_slice(name);
                buf.extend_from_slice(symbol);
                buf.extend_from_slice(uri);
                buf.extend_from_slice(category);
                buf.extend_from_slice(creator);
            }
            Self::UpdateNFTMetadataAccounts {
                uri,
                category,
                creator,
            } => {
                buf.push(2);
                buf.extend_from_slice(uri);
                buf.extend_from_slice(category);
                buf.extend_from_slice(creator);
            }
        }
        buf
    }
}

/// Creates an CreateNFTMetadataAccounts instruction
pub fn create_nft_metadata_accounts(
    program_id: Pubkey,
    nft_owner: Pubkey,
    nft_metadata: Pubkey,
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
            AccountMeta::new_readonly(nft_owner, false),
            AccountMeta::new_readonly(nft_metadata, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(mint_authority, true),
            AccountMeta::new_readonly(payer, true),
            AccountMeta::new_readonly(program_id, false),
            AccountMeta::new_readonly(
                Pubkey::from_str("11111111111111111111111111111111").unwrap(),
                false,
            ),
        ],
        data: NFTMetadataInstruction::CreateNFTMetadataAccounts { name, symbol }.pack(),
    }
}

/// Creates an 'InitNFTMetadataAccounts' instruction.
pub fn init_nft_metadata_accounts(
    program_id: Pubkey,
    nft_owner: Pubkey,
    nft_metadata: Pubkey,
    mint: Pubkey,
    mint_authority: Pubkey,
    owner: Pubkey,
    name_str: &str,
    symbol_str: &str,
    uri_str: &str,
    category_str: &str,
    creator_str: &str,
) -> Instruction {
    let mut name: [u8; NAME_LENGTH] = [0; NAME_LENGTH];
    let mut symbol: [u8; SYMBOL_LENGTH] = [0; SYMBOL_LENGTH];
    let mut uri: [u8; URI_LENGTH] = [0; URI_LENGTH];
    let mut category: [u8; CATEGORY_LENGTH] = [0; CATEGORY_LENGTH];
    let mut creator: [u8; CREATOR_LENGTH] = [0; CREATOR_LENGTH];

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

    let category_bytes = category_str.as_bytes();
    for n in 0..(CATEGORY_LENGTH - 1) {
        if n < category_bytes.len() {
            category[n] = category_bytes[n];
        }
    }

    let creator_bytes = creator_str.as_bytes();
    for n in 0..(CREATOR_LENGTH - 1) {
        if n < creator_bytes.len() {
            creator[n] = creator_bytes[n];
        }
    }
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(nft_owner, false),
            AccountMeta::new(nft_metadata, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(mint_authority, true),
            AccountMeta::new_readonly(owner, false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: NFTMetadataInstruction::InitNFTMetadataAccounts {
            name,
            symbol,
            uri,
            category,
            creator,
        }
        .pack(),
    }
}

/// update nft metadata account instruction
pub fn update_nft_metadata_accounts(
    program_id: Pubkey,
    nft_metadata: Pubkey,
    nft_owner: Pubkey,
    owner: Pubkey,
    uri_str: &str,
    category_str: &str,
    creator_str: &str,
) -> Instruction {
    let mut uri: [u8; URI_LENGTH] = [0; URI_LENGTH];
    let mut category: [u8; CATEGORY_LENGTH] = [0; CATEGORY_LENGTH];
    let mut creator: [u8; CREATOR_LENGTH] = [0; CREATOR_LENGTH];

    let uri_bytes = uri_str.as_bytes();
    for n in 0..(URI_LENGTH - 1) {
        if n < uri_bytes.len() {
            uri[n] = uri_bytes[n];
        }
    }

    let category_bytes = category_str.as_bytes();
    for n in 0..(CATEGORY_LENGTH - 1) {
        if n < category_bytes.len() {
            category[n] = category_bytes[n];
        }
    }

    let creator_bytes = creator_str.as_bytes();
    for n in 0..(CREATOR_LENGTH - 1) {
        if n < creator_bytes.len() {
            creator[n] = creator_bytes[n];
        }
    }
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(nft_metadata, false),
            AccountMeta::new_readonly(owner, true),
            AccountMeta::new_readonly(nft_owner, false),
        ],
        data: NFTMetadataInstruction::UpdateNFTMetadataAccounts {
            uri,
            category,
            creator,
        }
        .pack(),
    }
}
