#![allow(dead_code)]

use {
    borsh::BorshDeserialize,
    solana_program_test::BanksClient,
    solana_sdk::{
        borsh1::try_from_slice_unchecked,
        hash::Hash,
        program_pack::Pack,
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        transaction::Transaction,
    },
    spl_associated_token_account as atoken,
    spl_single_pool::inline_mpl_token_metadata::pda::find_metadata_account,
    spl_token::state::{Account, Mint},
};

pub async fn create_ata(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    owner: &Pubkey,
    recent_blockhash: &Hash,
    pool_mint: &Pubkey,
) {
    let instruction = atoken::instruction::create_associated_token_account(
        &payer.pubkey(),
        owner,
        pool_mint,
        &spl_token::id(),
    );
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[payer],
        *recent_blockhash,
    );

    banks_client.process_transaction(transaction).await.unwrap();
}

pub async fn get_token_balance(banks_client: &mut BanksClient, token: &Pubkey) -> u64 {
    let token_account = banks_client.get_account(*token).await.unwrap().unwrap();
    let account_info = Account::unpack_from_slice(&token_account.data).unwrap();
    account_info.amount
}

pub async fn get_token_supply(banks_client: &mut BanksClient, mint: &Pubkey) -> u64 {
    let mint_account = banks_client.get_account(*mint).await.unwrap().unwrap();
    let account_info = Mint::unpack_from_slice(&mint_account.data).unwrap();
    account_info.supply
}

#[derive(Clone, BorshDeserialize, Debug, PartialEq, Eq)]
pub struct Metadata {
    pub key: u8,
    pub update_authority: Pubkey,
    pub mint: Pubkey,
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub seller_fee_basis_points: u16,
    pub creators: Option<Vec<u8>>,
    pub primary_sale_happened: bool,
    pub is_mutable: bool,
}

pub async fn get_metadata_account(banks_client: &mut BanksClient, token_mint: &Pubkey) -> Metadata {
    let (token_metadata, _) = find_metadata_account(token_mint);
    let token_metadata_account = banks_client
        .get_account(token_metadata)
        .await
        .unwrap()
        .unwrap();
    try_from_slice_unchecked(token_metadata_account.data.as_slice()).unwrap()
}
