use crate::state::SerializationMethod;
use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        sysvar,
    },
};

/// Instructions supported by the Metadata program.
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub enum MetadataInstruction {
    ///   0. `[writable]` Metadata account (seed: ['metadata', target_program_id, name])
    ///   1. `[]` Target program
    ///   2. `[]` Target program ProgramData
    ///   3. `[signer]` Target program update authority
    ///   4. `[signer]` Payer
    ///   5. `[signer]` Metadata update authority
    ///   6. `[]` System program
    ///   7. `[]` Rent info
    CreateMetadataEntry { name: String, value: String },

    ///   0. `[writeable]` Metadata account
    ///   1. `[signer]` Update authority
    UpdateMetadataEntry { value: String },

    ///   0. `[writable]` Idl account (seed: ['idl', target_program_id, effective_slot])
    ///   1. `[]` Target program
    ///   2. `[]` Target program ProgramData
    ///   3. `[signer]` Program authority
    ///   4. `[signer]` Payer
    ///   5. `[signer]` IDL update authority
    ///   6. `[]` System program
    ///   7. `[]` Rent info
    CreateVersionedIdl {
        effective_slot: u64,
        idl_url: String,
        source_url: String,
        serialization: SerializationMethod,
        custom_layout_url: Option<String>,
    },

    ///   0. `[writeable]` Idl account
    ///   1. `[signer]` Update authority
    UpdateVersionedIdl {
        idl_url: String,
        source_url: String,
        serialization: SerializationMethod,
        custom_layout_url: Option<String>,
    },

    /// Transfer Update Authority
    ///   0. `[writable]`  Metadata account
    ///   1. `[signer]` Current Update authority key
    ///   2. `[]`  New Update authority account key
    TransferUpdateAuthority,
}

pub fn create_metadata_entry(
    program_id: Pubkey,
    metadata_account: Pubkey,
    target_program: Pubkey,
    target_program_authority: Pubkey,
    payer: Pubkey,
    update_authority: Pubkey,
    name: String,
    value: String,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(metadata_account, false),
            AccountMeta::new_readonly(target_program, false),
            AccountMeta::new_readonly(target_program_authority, false),
            AccountMeta::new_readonly(payer, false),
            AccountMeta::new_readonly(update_authority, false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: MetadataInstruction::CreateMetadataEntry { name, value }
            .try_to_vec()
            .unwrap(),
    }
}

pub fn update_metadata_entry(
    program_id: Pubkey,
    metadata_account: Pubkey,
    update_authority: Pubkey,
    value: String,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(metadata_account, false),
            AccountMeta::new_readonly(update_authority, false),
        ],
        data: MetadataInstruction::UpdateMetadataEntry { value }
            .try_to_vec()
            .unwrap(),
    }
}

pub fn create_versioned_idl(
    program_id: Pubkey,
    metadata_account: Pubkey,
    target_program: Pubkey,
    target_program_authority: Pubkey,
    payer: Pubkey,
    update_authority: Pubkey,
    effective_slot: u64,
    idl_url: String,
    source_url: String,
    serialization: SerializationMethod,
    custom_layout_url: Option<String>,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(metadata_account, false),
            AccountMeta::new_readonly(target_program, false),
            AccountMeta::new_readonly(target_program_authority, false),
            AccountMeta::new_readonly(payer, false),
            AccountMeta::new_readonly(update_authority, false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: MetadataInstruction::CreateVersionedIdl {
            effective_slot,
            idl_url,
            source_url,
            serialization,
            custom_layout_url,
        }
        .try_to_vec()
        .unwrap(),
    }
}

pub fn update_versioned_idl(
    program_id: Pubkey,
    metadata_account: Pubkey,
    update_authority: Pubkey,
    idl_url: String,
    source_url: String,
    serialization: SerializationMethod,
    custom_layout_url: Option<String>,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(metadata_account, false),
            AccountMeta::new_readonly(update_authority, false),
        ],
        data: MetadataInstruction::UpdateVersionedIdl {
            idl_url,
            source_url,
            serialization,
            custom_layout_url,
        }
        .try_to_vec()
        .unwrap(),
    }
}

pub fn transfer_update_authority(
    program_id: Pubkey,
    object: Pubkey,
    update_authority: Pubkey,
    new_update_authority: Pubkey,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(object, false),
            AccountMeta::new_readonly(update_authority, true),
            AccountMeta::new_readonly(new_update_authority, false),
        ],
        data: MetadataInstruction::TransferUpdateAuthority
            .try_to_vec()
            .unwrap(),
    }
}
