use crate::utils::error_msg;
use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};
use std::mem::size_of;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub enum EncodeMethod {
    UTF8,
    Base58Encode,
}

impl From<u8> for EncodeMethod {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::UTF8,
            1 => Self::Base58Encode,
            _ => panic!("Unsupported value for EncodeMethod"),
        }
    }
}

impl EncodeMethod {
    pub fn to_u8(&self) -> u8 {
        match self {
            Self::UTF8 => 0,
            Self::Base58Encode => 1,
        }
    }
}

pub const NUM_CREATORS: usize = 5;

// Adapter Creator class that implements POD
#[repr(C)]
#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Copy, Clone, Zeroable, Pod)]
pub struct GumballCreatorAdapter {
    pub address: Pubkey,
    // Bool does not work with the POD trait which is desired for GumballMachineHeader.
    // See `adapt` below for the compatability with bubblegum::state::metaplex_adapter::Creator
    pub verified: u8,
    // In percentages, NOT basis points ;) Watch out!
    pub share: u8,
}

impl Default for GumballCreatorAdapter {
    fn default() -> Self {
        Self {
            address: Default::default(),
            verified: 0,
            share: 0,
        }
    }
}

impl GumballCreatorAdapter {
    pub fn adapt(&self) -> bubblegum::state::metaplex_adapter::Creator {
        bubblegum::state::metaplex_adapter::Creator {
            address: self.address,
            verified: self.verified == 1,
            share: self.share,
        }
    }
    pub fn is_valid(&self) -> bool {
        return self.address != Default::default() && self.share > 0;
    }
}

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
    // 0 for whitespace trimming, 1 for base58 encode
    pub config_line_encode_method: u8,
    // Secondary sale royalty recipients
    pub creators: [GumballCreatorAdapter; NUM_CREATORS],
    // Used for 8-byte aligning zero copy structs
    pub _padding: [u8; 1],
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
