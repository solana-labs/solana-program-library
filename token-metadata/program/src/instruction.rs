use crate::state::Data;

use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        sysvar,
    },
};

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
/// Args for update call
pub struct UpdateMetadataAccountArgs {
    /// URI pointing to JSON representing the asset
    pub uri: String,
}

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
/// Args for create call
pub struct CreateMetadataAccountArgs {
    pub data: Data,
    // For whatever reason, Borsh throws IO errors when trying to deserialize booleans over the wire.
    // use u8 instead.
    pub allow_duplication: bool,
}

/// Instructions supported by the Metadata program.
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub enum MetadataInstruction {
    /// Create NameSymbolTuple (optional) and  Metadata objects.
    ///   0. `[writable]`  NameSymbolTuple key (pda of ['metadata', program id, name, symbol])
    ///   1. `[writable]`  Metadata key (pda of ['metadata', program id, mint id])
    ///   2. `[]` Mint of token asset
    ///   3. `[signer]` Mint authority
    ///   4. `[signer]` payer
    ///   5. `[signer]` update authority info (Signer is optional - only required if NameSymbolTuple exists)
    ///   6. `[]` System program
    CreateMetadataAccounts(CreateMetadataAccountArgs),

    /// Update an  Metadata (name/symbol are unchangeable)
    ///   0. `[writable]`  Metadata account
    ///   1. `[signer]` Update authority key
    ///   2. `[]`  NameSymbolTuple account key (pda of ['metadata', program id, name, symbol])
    ///            (does not need to exist if Metadata is of the duplicatable type)
    UpdateMetadataAccounts(UpdateMetadataAccountArgs),
}

/// Creates an CreateMetadataAccounts instruction
#[allow(clippy::too_many_arguments)]
pub fn create_metadata_accounts(
    program_id: Pubkey,
    name_symbol_account: Pubkey,
    metadata_account: Pubkey,
    mint: Pubkey,
    mint_authority: Pubkey,
    payer: Pubkey,
    update_authority: Pubkey,
    name: String,
    symbol: String,
    uri: String,
    allow_duplication: bool,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(name_symbol_account, false),
            AccountMeta::new(metadata_account, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(mint_authority, true),
            AccountMeta::new_readonly(payer, true),
            AccountMeta::new_readonly(update_authority, false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: MetadataInstruction::CreateMetadataAccounts(CreateMetadataAccountArgs {
            data: Data { name, symbol, uri },
            allow_duplication,
        })
        .try_to_vec()
        .unwrap(),
    }
}

/// update metadata account instruction
pub fn update_metadata_accounts(
    program_id: Pubkey,
    metadata_account: Pubkey,
    owner_account: Pubkey,
    owner: Pubkey,
    uri: String,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(metadata_account, false),
            AccountMeta::new_readonly(owner, true),
            AccountMeta::new_readonly(owner_account, false),
        ],
        data: MetadataInstruction::UpdateMetadataAccounts(UpdateMetadataAccountArgs { uri })
            .try_to_vec()
            .unwrap(),
    }
}
