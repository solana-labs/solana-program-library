use std::str::FromStr;

use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    sysvar,
};

use borsh::{BorshDeserialize, BorshSerialize};

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
/// Args for Create call
pub struct CreateMetadataAccountArgs {
    /// The name of the asset
    pub name: String,
    /// The symbol for the asset, ie, AAPL or SHOES
    pub symbol: String,
    /// URI pointing to JSON representing the asset
    pub uri: String,
}

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
/// Args for init call
pub struct InitMetadataAccountArgs {
    /// The name of the asset
    pub name: String,
    /// The symbol for the asset, ie, AAPL or SHOES
    pub symbol: String,
    /// URI pointing to JSON representing the asset
    pub uri: String,
}

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
/// Args for update call
pub struct UpdateMetadataAccountArgs {
    /// URI pointing to JSON representing the asset
    pub uri: String,
}

/// Instructions supported by the Metadata program.
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub enum MetadataInstruction {
    /// Create an  Owner and  Metadata objects.
    ///   0. `[writable]`  Owner key (pda of ['metadata', program id, name, symbol])
    ///   1. `[writable]`  metadata key (pda of ['metadata', program id, mint id])
    ///   2. `[]` Mint of
    ///   3. `[signer]` Mint authority
    ///   4. `[signer]` payer
    ///   5. `[]`  metadata program
    ///   6. `[]` System program
    CreateMetadataAccounts(CreateMetadataAccountArgs),

    /// Instantiate an  Owner and  Metadata object.
    ///   0. `[writable]` Uninitialized  Owner account
    ///   1. `[writable]` Uninitialized  Metadata account
    ///   2. `[]` Mint of
    ///   3. `[signer]` Mint authority of
    ///   4. `[]` Owner key
    ///   5. `[]` Rent sysvar
    InitMetadataAccounts(InitMetadataAccountArgs),

    /// Update an  Metadata (name/symbol are unchangeable)
    ///   0. `[writable]`  Metadata account
    ///   1. `[signer]` Owner key
    ///   2. `[]`  Owner account
    UpdateMetadataAccounts(UpdateMetadataAccountArgs),
}

/// Creates an CreateMetadataAccounts instruction
#[allow(clippy::too_many_arguments)]
pub fn create_metadata_accounts(
    program_id: Pubkey,
    owner_account: Pubkey,
    metadata_account: Pubkey,
    mint: Pubkey,
    mint_authority: Pubkey,
    payer: Pubkey,
    owner: Pubkey,
    name: String,
    symbol: String,
    uri: String,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new_readonly(owner_account, false),
            AccountMeta::new_readonly(metadata_account, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(mint_authority, true),
            AccountMeta::new_readonly(payer, true),
            AccountMeta::new_readonly(owner, false),
            AccountMeta::new_readonly(program_id, false),
            AccountMeta::new_readonly(
                Pubkey::from_str("11111111111111111111111111111111").unwrap(),
                false,
            ),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: MetadataInstruction::CreateMetadataAccounts(CreateMetadataAccountArgs {
            name,
            symbol,
            uri,
        })
        .try_to_vec()
        .unwrap(),
    }
}

/// Creates an 'InitMetadataAccounts' instruction.
#[allow(clippy::too_many_arguments)]
pub fn init_metadata_accounts(
    program_id: Pubkey,
    owner_account: Pubkey,
    metadata_account: Pubkey,
    mint: Pubkey,
    mint_authority: Pubkey,
    owner: Pubkey,
    name: String,
    symbol: String,
    uri: String,
) -> Instruction {
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
        data: MetadataInstruction::InitMetadataAccounts(InitMetadataAccountArgs {
            name,
            symbol,
            uri,
        })
        .try_to_vec()
        .unwrap(),
    }
}

/// update  metadata account instruction
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
