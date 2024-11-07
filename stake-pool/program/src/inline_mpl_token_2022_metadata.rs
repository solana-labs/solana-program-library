//! Inlined MPL metadata types to avoid a direct dependency on
//! `mpl-token-metadata' NOTE: this file is sym-linked in `spl-single-pool`, so
//! be careful with changes!

solana_program::declare_id!("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");

pub(crate) mod instruction {
    use {
        super::state::Data,
        borsh::{BorshDeserialize, BorshSerialize},
        solana_program::{
            instruction::{AccountMeta, Instruction},
            pubkey::Pubkey,
        },
    };

    #[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
    pub enum CreateArgs {
        V1 {
            name: String,
            symbol: String,
            uri: String,
            seller_fee_basis_points: u16,
            creators: Option<Vec<u8>>,
            primary_sale_happened: bool,
            is_mutable: bool,
            token_standard: u8,
            collection: Option<u8>,
            uses: Option<u8>,
            collection_details: Option<u8>,
            rule_set: Option<u8>,
            decimals: Option<u8>,
            print_supply: Option<u8>,
        },
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create(
        program_id: Pubkey,
        metadata_account: Pubkey,
        mint: Pubkey,
        mint_authority: Pubkey,
        payer: Pubkey,
        update_authority: Pubkey,
        token_program_id: Pubkey,
        name: String,
        symbol: String,
        uri: String,
    ) -> Instruction {
        let mut data = vec![42]; // create
        data.append(
            &mut borsh::to_vec(&CreateArgs::V1 {
                name,
                symbol,
                uri,
                seller_fee_basis_points: 0,
                creators: None,
                primary_sale_happened: false,
                is_mutable: true,
                token_standard: 2,
                collection: None,
                uses: None,
                collection_details: None,
                rule_set: None,
                decimals: None,
                print_supply: None,
            })
            .unwrap(),
        );
        Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(metadata_account, false),
                AccountMeta::new(metadata_account, false),
                AccountMeta::new_readonly(mint, false),
                AccountMeta::new_readonly(mint_authority, true),
                AccountMeta::new(payer, true),
                AccountMeta::new_readonly(update_authority, true),
                AccountMeta::new_readonly(solana_program::system_program::ID, false),
                AccountMeta::new_readonly(solana_program::sysvar::instructions::ID, false),
                AccountMeta::new_readonly(token_program_id, false),
            ],
            data,
        }
    }

    #[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
    pub enum UpdateArgs {
        V1 {
            new_update_authority: Option<Pubkey>,
            data: Option<Data>,
            primary_sale_happened: Option<bool>,
            is_mutable: Option<bool>,
            collection: u8,
            collection_details: u8,
            uses: u8,
            rule_set: u8,
            authorization_data: Option<u8>,
        },
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn update(
        program_id: Pubkey,
        metadata_account: Pubkey,
        update_authority: Pubkey,
        mint: Pubkey,
        payer: Pubkey,
        metadata_data: Option<Data>,
        primary_sale_happened: Option<bool>,
        is_mutable: Option<bool>,
    ) -> Instruction {
        let mut data = vec![50]; // update
        data.append(
            &mut borsh::to_vec(&UpdateArgs::V1 {
                new_update_authority: Some(update_authority),
                data: metadata_data,
                primary_sale_happened,
                is_mutable,
                collection: 0,
                collection_details: 0,
                uses: 0,
                rule_set: 0,
                authorization_data: None,
            })
            .unwrap(),
        );
        Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new_readonly(update_authority, true), // authority
                AccountMeta::new_readonly(super::ID, false),       // delegate_record
                AccountMeta::new_readonly(super::ID, false),       // token
                AccountMeta::new_readonly(mint, false),            // mint
                AccountMeta::new(metadata_account, false),         // metadata
                AccountMeta::new_readonly(super::ID, false),       // edition
                AccountMeta::new(payer, true),                     // payer
                AccountMeta::new_readonly(solana_program::system_program::ID, false), // system_program
                AccountMeta::new_readonly(solana_program::sysvar::instructions::ID, false), // sysvar_instructions
                AccountMeta::new_readonly(super::ID, false), // authorization_rules_program
                AccountMeta::new_readonly(super::ID, false), // authorization_rules
            ],
            data,
        }
    }
}

/// PDA creation helpers
pub mod pda {
    use {super::ID, solana_program::pubkey::Pubkey};
    const PREFIX: &str = "metadata";
    /// Helper to find a metadata account address
    pub fn find_metadata_account(mint: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[PREFIX.as_bytes(), ID.as_ref(), mint.as_ref()], &ID)
    }
}

pub(crate) mod state {
    use borsh::{BorshDeserialize, BorshSerialize};
    #[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
    pub struct Data {
        pub name: String,
        pub symbol: String,
        pub uri: String,
        pub seller_fee_basis_points: u16,
        pub creators: Option<Vec<u8>>,
    }
}
