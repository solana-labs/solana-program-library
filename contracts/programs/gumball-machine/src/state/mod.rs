use crate::utils::error_msg;
use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};
use std::mem::size_of;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct GumballMachineHeader {
    // TODO: Add more fields
    // Used to programmatically create the url and name for each field.
    // Unlike candy machine, each NFT minted has its url programmatically generated 
    // from the config line index as format!("{} #{}", url_base, index)
    pub url_base: [u8; 64],
    // Unlike candy machine, each NFT minted has its name programmatically generated 
    // from the config line index as format!("{} #{}", name_base, index)
    pub name_base: [u8; 32],
    pub symbol: [u8; 8],
    pub seller_fee_basis_points: u16,
    pub is_mutable: u8,
    pub retain_authority: u8,
    // Used for 8-byte aligning zero copy structs
    pub _padding: [u8; 4],
    pub price: u64,
    pub go_live_date: i64,
    // Mint of the Token used to purchase NFTs
    pub mint: Pubkey,
    // Used to collect bot fees
    pub bot_wallet: Pubkey,
    pub receiver: Pubkey,
    pub authority: Pubkey,
    // TokenMetadata collection pointer
    pub collection_key: Pubkey,
    // Force a single creator (use Hydra)
    pub creator_address: Pubkey,
    pub extension_len: u64,
    pub max_mint_size: u64,
    pub remaining: u64,
    pub max_items: u64,
    pub total_items_added: u64,
}

impl ZeroCopy for GumballMachineHeader {}
pub trait ZeroCopy: Pod {
    fn load_mut_bytes<'a>(data: &'a mut [u8]) -> Result<&'a mut Self> {
        let size = size_of::<Self>();
        let data_len = data.len();

        Ok(bytemuck::try_from_bytes_mut(&mut data[..size])
            .map_err(error_msg::<Self>(data_len))
            .unwrap())
    }
}
