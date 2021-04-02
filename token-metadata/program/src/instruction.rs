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
    pub uri: String,
    // Ignored when NameSymbolTuple present
    pub non_unique_specific_update_authority: Option<Pubkey>,
}

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
/// Args for create call
pub struct CreateMetadataAccountArgs {
    pub allow_duplication: bool,
    pub data: Data,
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
    ///   0. `[writable]` Metadata account
    ///   1. `[signer]` Update authority key
    ///   2. `[]`  NameSymbolTuple account key (pda of ['metadata', program id, name, symbol])
    ///            (does not need to exist if Metadata is of the duplicatable type)
    UpdateMetadataAccounts(UpdateMetadataAccountArgs),

    /// Transfer Update Authority
    ///   0. `[writable]`  NameSymbolTuple account
    ///   1. `[signer]` Current Update authority key
    ///   2. `[]`  New Update authority account key
    TransferUpdateAuthority,
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
    update_authority_is_signer: bool,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(name_symbol_account, false),
            AccountMeta::new(metadata_account, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(mint_authority, true),
            AccountMeta::new_readonly(payer, true),
            AccountMeta::new_readonly(update_authority, update_authority_is_signer),
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
    name_symbol_account: Pubkey,
    update_authority: Pubkey,
    non_unique_specific_update_authority: Option<Pubkey>,
    uri: String,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(metadata_account, false),
            AccountMeta::new_readonly(update_authority, true),
            AccountMeta::new_readonly(name_symbol_account, false),
        ],
        data: MetadataInstruction::UpdateMetadataAccounts(UpdateMetadataAccountArgs {
            uri,
            non_unique_specific_update_authority,
        })
        .try_to_vec()
        .unwrap(),
    }
}

/// transfer update authority instruction
pub fn transfer_update_authority(
    program_id: Pubkey,
    name_symbol_account: Pubkey,
    update_authority: Pubkey,
    new_update_authority: Pubkey,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(name_symbol_account, false),
            AccountMeta::new_readonly(update_authority, true),
            AccountMeta::new_readonly(new_update_authority, false),
        ],
        data: MetadataInstruction::TransferUpdateAuthority
            .try_to_vec()
            .unwrap(),
    }
}
