use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        sysvar,
    },
    crate::state::{
        SerializationMethod
    }
};

/// Instructions supported by the Metadata program.
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub enum MetadataInstruction {
    ///   0. `[]` Class account (seed: ['program_metadata', target_program_id])
    ///   1. `[writable]` Name record account (seed: [SHA256(HASH_PREFIX, 'Create::name')])
    ///   2. `[]` Target program
    ///   3. `[]` Target program ProgramData
    ///   4. `[signer]` Target program update authority
    ///   5. `[signer]` Payer
    ///   6. `[]` System program
    ///   7. `[]` Rent info
    ///   8. `[]` Name service
    CreateMetadataEntry {
        name: String,
        value: String,
        hashed_name: Vec<u8>,
    },

    ///   0. `[writable]` Class account (seed: ['program_metadata', target_program_id])
    ///   1. `[writable]` Name record account (seed: [SHA256(HASH_PREFIX, 'Create::name')])
    ///   2. `[]` Target program
    ///   3. `[]` Target program ProgramData
    ///   4. `[signer]` Target program update authority
    ///   5. `[]` Name service
    UpdateMetadataEntry { value: String },

    ///   0. `[writable]` Class account (seed: ['program_metadata', target_program_id])
    ///   1. `[writable]` Name record PDA (seed: [SHA256(HASH_PREFIX, 'Create::name')])
    ///   2. `[]` Target program
    ///   3. `[]` Target program ProgramData
    ///   4. `[signer]` Target program update authority
    ///   5. `[]` Refund account
    ///   6. `[]` Name service
    DeleteMetadataEntry,

    ///   0. `[]` Class account (seed: ['program_metadata', target_program_id])
    ///   1. `[writable]` Name record account (seed: [SHA256(HASH_PREFIX, 'Create::name')])
    ///   2. `[]` Target program
    ///   3. `[]` Target program ProgramData
    ///   4. `[signer]` Target program update authority
    ///   5. `[signer]` Payer
    ///   6. `[]` System program
    ///   7. `[]` Rent info
    ///   8. `[]` Name service
    CreateVersionedIdl {
        effective_slot: u64,
        idl_url: String,
        idl_hash: [u8; 32],
        source_url: String,
        serialization: SerializationMethod,
        custom_layout_url: Option<String>,
        hashed_name: [u8; 32],
    },

    ///   0. `[writable]` Class account (seed: ['program_metadata', target_program_id])
    ///   1. `[writable]` Name record account (seed: [SHA256(HASH_PREFIX, 'Create::name')])
    ///   2. `[]` Target program
    ///   3. `[]` Target program ProgramData
    ///   4. `[signer]` Target program update authority
    ///   5. `[]` Name service
    UpdateVersionedIdl {
        idl_url: String,
        idl_hash: [u8; 32],
        source_url: String,
        serialization: SerializationMethod,
        custom_layout_url: Option<String>,
    },
}

pub fn create_metadata_entry(
    program_id: Pubkey,
    class_account: Pubkey,
    metadata_account: Pubkey,
    target_program: Pubkey,
    target_program_authority: Pubkey,
    payer: Pubkey,
    update_authority: Pubkey,
    name: String,
    value: String,
    hashed_name: Vec<u8>,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(class_account, false),
            AccountMeta::new(metadata_account, false),
            AccountMeta::new_readonly(target_program, false),
            AccountMeta::new_readonly(target_program_authority, false),
            AccountMeta::new_readonly(payer, false),
            AccountMeta::new_readonly(update_authority, false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: MetadataInstruction::CreateMetadataEntry {
            name,
            value,
            hashed_name,
        }
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

// pub fn create_versioned_idl(
//     program_id: Pubkey,
//     metadata_account: Pubkey,
//     target_program: Pubkey,
//     target_program_authority: Pubkey,
//     payer: Pubkey,
//     update_authority: Pubkey,
//     effective_slot: u64,
//     idl_url: String,
//     source_url: String,
//     serialization: SerializationMethod,
//     custom_layout_url: Option<String>,
// ) -> Instruction {
//     Instruction {
//         program_id,
//         accounts: vec![
//             AccountMeta::new(metadata_account, false),
//             AccountMeta::new_readonly(target_program, false),
//             AccountMeta::new_readonly(target_program_authority, false),
//             AccountMeta::new_readonly(payer, false),
//             AccountMeta::new_readonly(update_authority, false),
//             AccountMeta::new_readonly(solana_program::system_program::id(), false),
//             AccountMeta::new_readonly(sysvar::rent::id(), false),
//         ],
//         data: MetadataInstruction::CreateVersionedIdl {
//             effective_slot,
//             idl_url,
//             source_url,
//             serialization,
//             custom_layout_url,
//         }
//         .try_to_vec()
//         .unwrap(),
//     }
// }

// pub fn update_versioned_idl(
//     program_id: Pubkey,
//     metadata_account: Pubkey,
//     update_authority: Pubkey,
//     idl_url: String,
//     source_url: String,
//     serialization: SerializationMethod,
//     custom_layout_url: Option<String>,
// ) -> Instruction {
//     Instruction {
//         program_id,
//         accounts: vec![
//             AccountMeta::new(metadata_account, false),
//             AccountMeta::new_readonly(update_authority, false),
//         ],
//         data: MetadataInstruction::UpdateVersionedIdl {
//             idl_url,
//             source_url,
//             serialization,
//             custom_layout_url,
//         }
//         .try_to_vec()
//         .unwrap(),
//     }
// }

// pub fn transfer_update_authority(
//     program_id: Pubkey,
//     object: Pubkey,
//     update_authority: Pubkey,
//     new_update_authority: Pubkey,
// ) -> Instruction {
//     Instruction {
//         program_id,
//         accounts: vec![
//             AccountMeta::new(object, false),
//             AccountMeta::new_readonly(update_authority, true),
//             AccountMeta::new_readonly(new_update_authority, false),
//         ],
//         data: MetadataInstruction::TransferUpdateAuthority
//             .try_to_vec()
//             .unwrap(),
//     }
// }
