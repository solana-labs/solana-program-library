#![allow(dead_code)]

use {
    mpl_token_metadata::{pda::find_metadata_account, state::Metadata},
    solana_program_test::BanksClient,
    solana_sdk::{
        borsh::try_from_slice_unchecked,
        hash::Hash,
        message::Message,
        program_pack::Pack,
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        transaction::Transaction,
    },
    spl_associated_token_account as atoken,
    spl_token::state::{Account, Mint},
};

pub async fn create_ata(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    owner: &Pubkey,
    recent_blockhash: &Hash,
    pool_mint: &Pubkey,
) {
    #[allow(deprecated)]
    let instruction = atoken::create_associated_token_account(&payer.pubkey(), owner, pool_mint);
    let message = Message::new(&[instruction], Some(&payer.pubkey()));
    let transaction = Transaction::new(&[payer], message, *recent_blockhash);

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

pub async fn get_metadata_account(banks_client: &mut BanksClient, token_mint: &Pubkey) -> Metadata {
    let (token_metadata, _) = find_metadata_account(token_mint);
    let token_metadata_account = banks_client
        .get_account(token_metadata)
        .await
        .unwrap()
        .unwrap();
    try_from_slice_unchecked(token_metadata_account.data.as_slice()).unwrap()
}
