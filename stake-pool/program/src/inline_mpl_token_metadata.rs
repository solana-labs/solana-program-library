//! Inlined MPL metadata types to avoid a direct dependency on
//! `mpl-token-metadata' NOTE: this file is sym-linked in `spl-single-pool`, so
//! be careful with changes!

solana_program::declare_id!("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");

pub(crate) mod instruction {
    use {
        super::state::DataV2,
        borsh::{BorshDeserialize, BorshSerialize},
        solana_program::{
            instruction::{AccountMeta, Instruction},
            pubkey::Pubkey,
        },
    };

    #[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
    struct CreateMetadataAccountArgsV3 {
        /// Note that unique metadatas are disabled for now.
        pub data: DataV2,
        /// Whether you want your metadata to be updateable in the future.
        pub is_mutable: bool,
        /// UNUSED If this is a collection parent NFT.
        pub collection_details: Option<u8>,
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create_metadata_accounts_v3(
        program_id: Pubkey,
        metadata_account: Pubkey,
        mint: Pubkey,
        mint_authority: Pubkey,
        payer: Pubkey,
        update_authority: Pubkey,
        name: String,
        symbol: String,
        uri: String,
    ) -> Instruction {
        let mut data = vec![33]; // CreateMetadataAccountV3
        data.append(
            &mut borsh::to_vec(&CreateMetadataAccountArgsV3 {
                data: DataV2 {
                    name,
                    symbol,
                    uri,
                    seller_fee_basis_points: 0,
                    creators: None,
                    collection: None,
                    uses: None,
                },
                is_mutable: true,
                collection_details: None,
            })
            .unwrap(),
        );
        Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(metadata_account, false),
                AccountMeta::new_readonly(mint, false),
                AccountMeta::new_readonly(mint_authority, true),
                AccountMeta::new(payer, true),
                AccountMeta::new_readonly(update_authority, true),
                AccountMeta::new_readonly(solana_program::system_program::ID, false),
            ],
            data,
        }
    }

    #[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
    struct UpdateMetadataAccountArgsV2 {
        pub data: Option<DataV2>,
        pub update_authority: Option<Pubkey>,
        pub primary_sale_happened: Option<bool>,
        pub is_mutable: Option<bool>,
    }
    pub(crate) fn update_metadata_accounts_v2(
        program_id: Pubkey,
        metadata_account: Pubkey,
        update_authority: Pubkey,
        new_update_authority: Option<Pubkey>,
        metadata: Option<DataV2>,
        primary_sale_happened: Option<bool>,
        is_mutable: Option<bool>,
    ) -> Instruction {
        let mut data = vec![15]; // UpdateMetadataAccountV2
        data.append(
            &mut borsh::to_vec(&UpdateMetadataAccountArgsV2 {
                data: metadata,
                update_authority: new_update_authority,
                primary_sale_happened,
                is_mutable,
            })
            .unwrap(),
        );
        Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(metadata_account, false),
                AccountMeta::new_readonly(update_authority, true),
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
    #[repr(C)]
    #[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
    pub(crate) struct DataV2 {
        /// The name of the asset
        pub name: String,
        /// The symbol for the asset
        pub symbol: String,
        /// URI pointing to JSON representing the asset
        pub uri: String,
        /// Royalty basis points that goes to creators in secondary sales
        /// (0-10000)
        pub seller_fee_basis_points: u16,
        /// UNUSED Array of creators, optional
        pub creators: Option<u8>,
        /// UNUSED Collection
        pub collection: Option<u8>,
        /// UNUSED Uses
        pub uses: Option<u8>,
    }
}
