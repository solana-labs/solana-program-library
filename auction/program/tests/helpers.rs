use solana_program::{
    hash::Hash,
    program_pack::Pack,
    pubkey::Pubkey,
    system_instruction,
};
use solana_program_test::*;
use solana_sdk::{
    account::Account,
    signature::{Keypair, Signer},
    transaction::Transaction,
    transport::TransportError,
};
use spl_auction::{
    instruction,
    processor::{
        CancelBidArgs,
        CreateAuctionArgs,
        ClaimBidArgs,
        EndAuctionArgs,
        PlaceBidArgs,
        StartAuctionArgs,
        WinnerLimit,
        PriceFloor,
    },
};

pub async fn get_account(banks_client: &mut BanksClient, pubkey: &Pubkey) -> Account {
    banks_client
        .get_account(*pubkey)
        .await
        .expect("account not found")
        .expect("account empty")
}

pub async fn create_mint(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
) -> Result<(Keypair, Keypair), TransportError> {
    let rent = banks_client.get_rent().await.unwrap();
    let mint_rent = rent.minimum_balance(spl_token::state::Mint::LEN);
    let pool_mint = Keypair::new();
    let manager = Keypair::new();
    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &pool_mint.pubkey(),
                mint_rent,
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &pool_mint.pubkey(),
                &manager.pubkey(),
                None,
                0,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, &pool_mint], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok((pool_mint, manager))
}

pub async fn create_token_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    account: &Keypair,
    pool_mint: &Pubkey,
    manager: &Pubkey,
) -> Result<(), TransportError> {
    let rent = banks_client.get_rent().await.unwrap();
    let account_rent = rent.minimum_balance(spl_token::state::Account::LEN);

    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &account.pubkey(),
                account_rent,
                spl_token::state::Account::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_account(
                &spl_token::id(),
                &account.pubkey(),
                pool_mint,
                manager,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, account], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn mint_tokens(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    mint: &Pubkey,
    account: &Pubkey,
    mint_authority: &Keypair,
    amount: u64,
) -> Result<(), TransportError> {
    let transaction = Transaction::new_signed_with_payer(
        &[spl_token::instruction::mint_to(
            &spl_token::id(),
            mint,
            account,
            &mint_authority.pubkey(),
            &[],
            amount,
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[payer, mint_authority],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn get_token_balance(banks_client: &mut BanksClient, token: &Pubkey) -> u64 {
    let token_account = banks_client.get_account(*token).await.unwrap().unwrap();
    let account_info: spl_token::state::Account =
        spl_token::state::Account::unpack_from_slice(token_account.data.as_slice()).unwrap();
    account_info.amount
}

pub async fn get_token_supply(banks_client: &mut BanksClient, mint: &Pubkey) -> u64 {
    let mint_account = banks_client.get_account(*mint).await.unwrap().unwrap();
    let account_info =
        spl_token::state::Mint::unpack_from_slice(mint_account.data.as_slice()).unwrap();
    account_info.supply
}

pub async fn create_auction(
    banks_client: &mut BanksClient,
    program_id: &Pubkey,
    payer: &Keypair,
    recent_blockhash: &Hash,
    resource: &Pubkey,
    mint_keypair: &Pubkey,
    max_winners: usize,
) -> Result<(), TransportError> {
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::create_auction_instruction(
            *program_id,
            payer.pubkey(),
            CreateAuctionArgs {
                authority: payer.pubkey(),
                end_auction_at: None,
                end_auction_gap: None,
                resource: *resource,
                token_mint: *mint_keypair,
                winners: WinnerLimit::Capped(max_winners),
                price_floor: PriceFloor::None,
            },
        )],
        Some(&payer.pubkey()),
        &[payer],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn end_auction(
    banks_client: &mut BanksClient,
    program_id: &Pubkey,
    recent_blockhash: &Hash,
    payer: &Keypair,
    resource: &Pubkey,
) -> Result<(), TransportError> {
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::end_auction_instruction(
            *program_id,
            payer.pubkey(),
            EndAuctionArgs {
                resource: *resource,
                reveal: None,
            },
        )],
        Some(&payer.pubkey()),
        &[payer],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn start_auction(
    banks_client: &mut BanksClient,
    program_id: &Pubkey,
    recent_blockhash: &Hash,
    payer: &Keypair,
    resource: &Pubkey,
) -> Result<(), TransportError> {
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::start_auction_instruction(
            *program_id,
            payer.pubkey(),
            StartAuctionArgs {
                resource: *resource,
            },
        )],
        Some(&payer.pubkey()),
        &[payer],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn place_bid(
    banks_client: &mut BanksClient,
    recent_blockhash: &Hash,
    program_id: &Pubkey,
    payer: &Keypair,
    bidder: &Keypair,
    bidder_spl_account: &Keypair,
    transfer_authority: &Keypair,
    resource: &Pubkey,
    mint: &Pubkey,
    amount: u64,
) -> Result<(), TransportError> {
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::place_bid_instruction(
            *program_id,
            bidder.pubkey(),                // SPL Token Account (Source)
            bidder_spl_account.pubkey(),    // SPL Token Account (Destination)
            *mint,                          // Token Mint
            transfer_authority.pubkey(),    // Approved to Move Tokens
            payer.pubkey(),                 // Pays for Transactions
            PlaceBidArgs {
                amount: amount,
                resource: *resource,
            },
        )],
        Some(&payer.pubkey()),
        &[bidder, transfer_authority, payer],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn cancel_bid(
    banks_client: &mut BanksClient,
    recent_blockhash: &Hash,
    program_id: &Pubkey,
    payer: &Keypair,
    bidder: &Keypair,
    bidder_spl_account: &Keypair,
    resource: &Pubkey,
    mint: &Pubkey,
) -> Result<(), TransportError> {
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::cancel_bid_instruction(
            *program_id,
            bidder.pubkey(),
            bidder_spl_account.pubkey(),
            *mint,
            CancelBidArgs { resource: *resource },
        )],
        Some(&payer.pubkey()),
        &[bidder, payer],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn approve(
    banks_client: &mut BanksClient,
    recent_blockhash: &Hash,
    payer: &Keypair,
    transfer_authority: &Pubkey,
    spl_wallet: &Keypair,
    amount: u64,
) -> Result<(), TransportError> {
    let transaction = Transaction::new_signed_with_payer(
        &[spl_token::instruction::approve(
            &spl_token::id(),
            &spl_wallet.pubkey(),
            transfer_authority,
            &payer.pubkey(),
            &[&payer.pubkey()],
            amount,
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[payer],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn claim_bid(
    banks_client: &mut BanksClient,
    recent_blockhash: &Hash,
    program_id: &Pubkey,
    payer: &Keypair,
    authority: &Keypair,
    bidder: &Keypair,
    bidder_spl_account: &Keypair,
    seller: &Pubkey,
    resource: &Pubkey,
    mint: &Pubkey,
) -> Result<(), TransportError> {
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::claim_bid_instruction(
            *program_id,
            authority.pubkey(),
            *seller,
            bidder.pubkey(),
            bidder_spl_account.pubkey(),
            *mint,
            ClaimBidArgs { resource: *resource },
        )],
        Some(&payer.pubkey()),
        &[payer, authority],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await?;
    Ok(())
}
