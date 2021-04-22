#![allow(warnings)]

use solana_sdk::program_pack::Pack;
use byteorder::{ByteOrder, LittleEndian};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program_test::*;
use solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    system_program,
    transaction::Transaction,
};
use solana_program::{
    borsh::try_from_slice_unchecked,
};
use std::mem;
use spl_auction::{
    instruction,
    processor::{
        process_instruction,
        AuctionData,
        AuctionState,
        BidderPot,
        CancelBidArgs,
        CreateAuctionArgs,
        PlaceBidArgs,
        StartAuctionArgs,
        WinnerLimit,
    },
    PREFIX,
};

mod helpers;

/// Initialize an auction with a random resource, and generate bidders with tokens that can be used
/// for testing.
async fn setup_auction() -> (
    Pubkey,
    BanksClient,
    Vec<(Keypair, Keypair)>,
    Keypair,
    Pubkey,
    Pubkey,
    Pubkey,
    solana_sdk::hash::Hash,
) {
    // Create a program to attach accounts to.
    let program_id = Pubkey::new_unique();
    let mut program_test =
        ProgramTest::new(
            "spl_auction",
            program_id,
            processor!(process_instruction)
        );

    // Start executing test.
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    // Create a Token mint to mint some test tokens with.
    let (mint_keypair, mint_manager) = helpers::create_mint(
        &mut banks_client,
        &payer,
        &recent_blockhash,
    )
    .await
    .unwrap();

    // Derive Auction PDA account for lookup.
    let resource = Pubkey::new_unique();
    let seeds = &[PREFIX.as_bytes(), &program_id.as_ref(), resource.as_ref()];
    let (auction_pubkey, _) = Pubkey::find_program_address(seeds, &program_id);

    // Run Create Auction instruction.
    helpers::create_auction(
        &mut banks_client,
        &program_id,
        &payer,
        &recent_blockhash,
        &resource,
        &mint_keypair.pubkey(),
    ).await;

    // Attach useful Accounts for testing.
    let mut bidders = vec![];
    for n in 0..5 {
        // Bidder SPL Account, with Minted Tokens
        let bidder = Keypair::new();
        // PDA in the auction for the Bidder to deposit their funds to.
        let auction_spl_pot = Keypair::new();

        // Generate User SPL Wallet Account
        helpers::create_token_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &bidder,
            &mint_keypair.pubkey(),
            &payer.pubkey(),
        )
        .await
        .unwrap();

        // Owner via pot PDA.
        let (bid_pot_pubkey, pot_bump) = Pubkey::find_program_address(
            &[
                PREFIX.as_bytes(),
                program_id.as_ref(),
                auction_pubkey.as_ref(),
                bidder.pubkey().as_ref(),
            ],
            &program_id,
        );
        println!("-- placing bid");
        println!("PREFIX {}", PREFIX);
        println!("{}", program_id);
        println!("{}", auction_pubkey);
        println!("{}", bidder.pubkey());

        // Generate Auction SPL Pot to Transfer to.
        println!("{} {}", program_id, bid_pot_pubkey);
        helpers::create_token_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &auction_spl_pot,
            &mint_keypair.pubkey(),
            &bid_pot_pubkey, // Manager
        )
        .await
        .unwrap();

        // Mint Tokens
        helpers::mint_tokens(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &mint_keypair.pubkey(),
            &bidder.pubkey(),
            &mint_manager,
            10_000_000,
        )
        .await
        .unwrap();

        bidders.push((
            bidder,
            auction_spl_pot,
        ));
    }

    // Verify Auction was created as expected.
    let auction: AuctionData = try_from_slice_unchecked(
        &banks_client
            .get_account(auction_pubkey)
            .await
            .expect("get_account")
            .expect("account not found")
            .data
    ).unwrap();

    assert_eq!(auction.authority, payer.pubkey());
    assert_eq!(auction.last_bid, None);
    assert_eq!(auction.resource, resource);
    assert_eq!(auction.state as i32, AuctionState::create() as i32);
    assert_eq!(auction.end_auction_at, None);

    // Start Auction.
    let mut transaction = Transaction::new_with_payer(
        &[instruction::start_auction_instruction(
            program_id,
            payer.pubkey(),
            StartAuctionArgs {
                resource: resource,
            },
        )],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    return (
        program_id,
        banks_client,
        bidders,
        payer,
        resource,
        mint_keypair.pubkey(),
        mint_manager.pubkey(),
        recent_blockhash,
    );
}

#[cfg(feature = "test-bpf")]
#[tokio::test]
async fn test_english_auction() {
    let (
        program_id,
        mut banks_client,
        bidders,
        payer,
        resource,
        mint,
        mint_authority,
        recent_blockhash,
    ) = setup_auction().await;

    enum Action {
        Bid(usize, u64),
        Cancel(usize),
    }

    let strategies = [
        // Test bidding increments with no cancel or low bids.
        vec![
            Action::Bid(0, 1000),
            Action::Bid(1, 2000),
            Action::Bid(2, 3000),
            Action::Bid(3, 4000),
        ],
        vec![
            Action::Cancel(0),
            Action::Bid(0, 5000),
        ],
    ];

    for strategy in strategies.iter() {
        for action in strategy.iter() {
            match *action {
                Action::Bid(bidder, amount) => {
                    let transfer_authority = Keypair::new();
                    helpers::approve(
                        &mut banks_client,
                        &program_id,
                        &recent_blockhash,
                        &payer,
                        &transfer_authority.pubkey(),
                        &bidders[bidder].0,
                        amount,
                    )
                    .await
                    .expect("approve");

                    println!("Bid Time");
                    helpers::place_bid(
                        &mut banks_client,
                        &recent_blockhash,
                        &program_id,
                        &payer,
                        &bidders[bidder].0,
                        &bidders[bidder].1,
                        &transfer_authority,
                        &resource,
                        &mint,
                        amount,
                    )
                    .await
                    .expect("place_bid");

                    // Verify a bid was created, and Metadata for this bidder correctly reflects
                    // the last bid as expected.
                    let bidder_account = banks_client
                        .get_account(bidders[bidder].0.pubkey())
                        .await
                        .expect("get_account")
                        .expect("account not found");
                }

                Action::Cancel(bidder) => {
                    println!("Cancel Bid");
                    helpers::cancel_bid(
                        &mut banks_client,
                        &recent_blockhash,
                        &program_id,
                        &payer,
                        &bidders[bidder].0,
                        &bidders[bidder].1,
                        &resource,
                        &mint,
                    )
                    .await
                    .expect("cancel_bid");

                    let bidder_account = banks_client
                        .get_account(bidders[bidder].0.pubkey())
                        .await
                        .expect("get_account")
                        .expect("account not found");
                }
            }
        }
    }
}
