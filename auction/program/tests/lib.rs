#![allow(warnings)]

use borsh::{BorshDeserialize, BorshSerialize};
use byteorder::{ByteOrder, LittleEndian};
use solana_program::borsh::try_from_slice_unchecked;
use solana_program_test::*;
use solana_sdk::program_pack::Pack;
use solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction, system_program,
    hash::Hash,
    transaction::Transaction,
    transport::TransportError,
};
use spl_auction::{
    instruction,
    processor::{
        process_instruction, AuctionData, AuctionState, Bid, BidderPot, BidState, CancelBidArgs,
        CreateAuctionArgs, PlaceBidArgs, PriceFloor, StartAuctionArgs, WinnerLimit,
    },
    PREFIX,
};
use std::mem;

mod helpers;

/// Initialize an auction with a random resource, and generate bidders with tokens that can be used
/// for testing.
async fn setup_auction(
    start: bool,
    max_winners: usize,
) -> (
    Pubkey,
    BanksClient,
    Vec<(Keypair, Keypair, Pubkey)>,
    Keypair,
    Pubkey,
    Pubkey,
    Pubkey,
    Pubkey,
    Hash,
) {
    // Create a program to attach accounts to.
    let program_id = Pubkey::new_unique();
    let mut program_test =
        ProgramTest::new("spl_auction", program_id, processor!(process_instruction));

    // Start executing test.
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    // Create a Token mint to mint some test tokens with.
    let (mint_keypair, mint_manager) =
        helpers::create_mint(&mut banks_client, &payer, &recent_blockhash)
            .await
            .unwrap();

    // Derive Auction PDA account for lookup.
    let resource = Pubkey::new_unique();
    let seeds = &[PREFIX.as_bytes(), &program_id.as_ref(), resource.as_ref()];
    let (auction_pubkey, _) = Pubkey::find_program_address(seeds, &program_id);

    // Run Create Auction instruction.
    let err = helpers::create_auction(
        &mut banks_client,
        &program_id,
        &payer,
        &recent_blockhash,
        &resource,
        &mint_keypair.pubkey(),
        max_winners,
    )
    .await
    .unwrap();

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

        // Generate Auction SPL Pot to Transfer to.
        helpers::create_token_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &auction_spl_pot,
            &mint_keypair.pubkey(),
            &auction_pubkey,
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

        bidders.push((bidder, auction_spl_pot, bid_pot_pubkey));
    }

    // Verify Auction was created as expected.
    let auction: AuctionData = try_from_slice_unchecked(
        &banks_client
            .get_account(auction_pubkey)
            .await
            .expect("get_account")
            .expect("account not found")
            .data,
    )
    .unwrap();

    assert_eq!(auction.authority, payer.pubkey());
    assert_eq!(auction.last_bid, None);
    assert_eq!(auction.resource, resource);
    assert_eq!(auction.state as i32, AuctionState::create() as i32);
    assert_eq!(auction.end_auction_at, None);

    // Start Auction.
    if start {
        helpers::start_auction(
            &mut banks_client,
            &program_id,
            &recent_blockhash,
            &payer,
            &resource,
        )
        .await
        .unwrap();
    }

    return (
        program_id,
        banks_client,
        bidders,
        payer,
        resource,
        mint_keypair.pubkey(),
        mint_manager.pubkey(),
        auction_pubkey,
        recent_blockhash,
    );
}

/// Used to drive tests in the functions below.
#[derive(Debug)]
enum Action {
    Bid(usize, u64),
    Cancel(usize),
    End
}

#[cfg(feature = "test-bpf")]
#[tokio::test]
async fn test_correct_runs() {
    // Local wrapper around a small test description described by actions.
    struct Test {
        actions: Vec<Action>,
        expect: Vec<(usize, u64)>,
        max_winners: usize,
        price_floor: PriceFloor,
        seller_collects: u64,
    }

    // A list of auction runs that should succeed. At the end of the run the winning bid state
    // should match the expected result.
    let strategies = [
        // Simple successive bids should work.
        Test {
            actions: vec![
                Action::Bid(0, 1000),
                Action::Bid(1, 2000),
                Action::Bid(2, 3000),
                Action::Bid(3, 4000),
                Action::End,
            ],
            max_winners: 3,
            price_floor: PriceFloor::None,
            seller_collects: 9000,
            expect: vec![
                (1, 2000),
                (2, 3000),
                (3, 4000),
            ],
        },

        // A single bidder should be able to cancel and rebid lower.
        Test {
            actions: vec![
                Action::Bid(0, 5000),
                Action::Cancel(0),
                Action::Bid(0, 4000),
                Action::End,
            ],
            expect: vec![(0, 4000)],
            max_winners: 3,
            price_floor: PriceFloor::None,
            seller_collects: 4000,
        },

        // The top bidder when cancelling should allow room for lower bidders.
        Test {
            actions: vec![
                Action::Bid(0, 5000),
                Action::Bid(1, 6000),
                Action::Cancel(1),
                Action::Bid(2, 5500),
                Action::Bid(1, 6000),
                Action::Bid(3, 7000),
                Action::Cancel(0),
                Action::End,
            ],
            expect: vec![(2, 5500), (1, 6000), (3, 7000)],
            max_winners: 3,
            price_floor: PriceFloor::None,
            seller_collects: 18500,
        },

        // An auction where everyone cancels should still succeed, with no winners.
        Test {
            actions: vec![
                Action::Bid(0, 5000),
                Action::Bid(1, 6000),
                Action::Bid(2, 7000),
                Action::Cancel(0),
                Action::Cancel(1),
                Action::Cancel(2),
                Action::End,
            ],
            expect: vec![],
            max_winners: 3,
            price_floor: PriceFloor::None,
            seller_collects: 0,
        },

        // An auction where no one bids should still succeed.
        Test {
            actions: vec![
                Action::End,
            ],
            expect: vec![],
            max_winners: 3,
            price_floor: PriceFloor::None,
            seller_collects: 0,
        },
    ];

    // Run each strategy with a new auction.
    for strategy in strategies.iter() {
        let (
            program_id,
            mut banks_client,
            bidders,
            payer,
            resource,
            mint,
            mint_authority,
            auction_pubkey,
            recent_blockhash,
        ) = setup_auction(true, strategy.max_winners).await;

        // Interpret test actions one by one.
        for action in strategy.actions.iter() {
            println!("Strategy: {} Step {:?}", strategy.actions.len(), action);
            match *action {
                Action::Bid(bidder, amount) => {
                    // Get balances pre bidding.
                    let pre_balance = (
                        helpers::get_token_balance(&mut banks_client, &bidders[bidder].0.pubkey())
                            .await,
                        helpers::get_token_balance(&mut banks_client, &bidders[bidder].1.pubkey())
                            .await,
                    );

                    let transfer_authority = Keypair::new();
                    helpers::approve(
                        &mut banks_client,
                        &recent_blockhash,
                        &payer,
                        &transfer_authority.pubkey(),
                        &bidders[bidder].0,
                        amount,
                    )
                    .await
                    .expect("approve");

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

                    let post_balance = (
                        helpers::get_token_balance(&mut banks_client, &bidders[bidder].0.pubkey())
                            .await,
                        helpers::get_token_balance(&mut banks_client, &bidders[bidder].1.pubkey())
                            .await,
                    );

                    assert_eq!(post_balance.0, pre_balance.0 - amount);
                    assert_eq!(post_balance.1, pre_balance.1 + amount);
                }

                Action::Cancel(bidder) => {
                    // Get balances pre bidding.
                    let pre_balance = (
                        helpers::get_token_balance(&mut banks_client, &bidders[bidder].0.pubkey())
                            .await,
                        helpers::get_token_balance(&mut banks_client, &bidders[bidder].1.pubkey())
                            .await,
                    );

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

                    let post_balance = (
                        helpers::get_token_balance(&mut banks_client, &bidders[bidder].0.pubkey())
                            .await,
                        helpers::get_token_balance(&mut banks_client, &bidders[bidder].1.pubkey())
                            .await,
                    );

                    // Assert the balance successfully moves.
                    assert_eq!(post_balance.0, pre_balance.0 + pre_balance.1);
                    assert_eq!(post_balance.1, 0);
                }

                Action::End => {
                    helpers::end_auction(
                        &mut banks_client,
                        &program_id,
                        &recent_blockhash,
                        &payer,
                        &resource,
                    )
                    .await
                    .expect("end_auction");

                    // Assert Auction is actually in ended state.
                    let auction: AuctionData = try_from_slice_unchecked(
                        &banks_client
                            .get_account(auction_pubkey)
                            .await
                            .expect("get_account")
                            .expect("account not found")
                            .data,
                    )
                    .unwrap();

                    assert!(auction.ended_at.is_some());
                }
            }
        }

        // Verify a bid was created, and Metadata for this bidder correctly reflects
        // the last bid as expected.
        let auction: AuctionData = try_from_slice_unchecked(
            &banks_client
                .get_account(auction_pubkey)
                .await
                .expect("get_account")
                .expect("account not found")
                .data,
        )
        .unwrap();

        // Verify BidState, all winners should be as expected
        match auction.bid_state {
            BidState::EnglishAuction { ref bids, .. } => {
                // Zip internal bid state with the expected indices this strategy expects winners
                // to result in.
                let results: Vec<(_, _)> = strategy.expect.iter().zip(bids).collect();
                for (index, bid) in results.iter() {
                    let bidder = &bidders[index.0];
                    let amount = index.1;

                    // Winners should match the keypair indices we expected.
                    // bid.0 is the pubkey.
                    // bidder.2 is the derived potkey we expect Bid.0 to be.
                    assert_eq!(bid.0, bidder.2);
                    // Must have bid the amount we expected. 
                    // bid.1 is the amount.
                    assert_eq!(bid.1, amount);
                }

                // If the auction has ended, attempt to claim back SPL tokens into a new account.
                if auction.ended(0) {
                    let collection = Keypair::new();

                    // Generate Collection Pot.
                    helpers::create_token_account(
                        &mut banks_client,
                        &payer,
                        &recent_blockhash,
                        &collection,
                        &mint,
                        &payer.pubkey(),
                    )
                    .await
                    .unwrap();

                    // For each winning bid, claim into auction.
                    for (index, bid) in results {
                        let err = helpers::claim_bid(
                            &mut banks_client,
                            &recent_blockhash,
                            &program_id,
                            &payer,
                            &payer,
                            &bidders[index.0].0,
                            &bidders[index.0].1,
                            &collection.pubkey(),
                            &resource,
                            &mint,
                        )
                        .await;
                        println!("{:?}", err);
                        err.expect("claim_bid");

                        // Bid pot should be empty
                        let balance = helpers::get_token_balance(&mut banks_client, &bidders[index.0].1.pubkey())
                            .await;
                        assert_eq!(balance, 0);
                    }

                    // Total claimed balance should match what we expect
                    let balance = helpers::get_token_balance(&mut banks_client, &collection.pubkey())
                        .await;
                    assert_eq!(balance, strategy.seller_collects);
                }
            }
            _ => {}
        }
    }
}

// Function wrapper expected to fail for testing failures.
async fn handle_failing_action(
    banks_client: &mut BanksClient,
    recent_blockhash: &Hash,
    program_id: &Pubkey,
    bidders: &Vec<(Keypair, Keypair, Pubkey)>,
    mint: &Pubkey,
    payer: &Keypair,
    resource: &Pubkey,
    auction_pubkey: &Pubkey,
    action: &Action,
) -> Result<(), TransportError> {
    match *action {
        Action::Bid(bidder, amount) => {
            // Get balances pre bidding.
            let pre_balance = (
                helpers::get_token_balance(banks_client, &bidders[bidder].0.pubkey())
                    .await,
                helpers::get_token_balance(banks_client, &bidders[bidder].1.pubkey())
                    .await,
            );

            let transfer_authority = Keypair::new();
            helpers::approve(
                banks_client,
                &recent_blockhash,
                &payer,
                &transfer_authority.pubkey(),
                &bidders[bidder].0,
                amount,
            )
            .await?;

            let value = helpers::place_bid(
                banks_client,
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
            .await?;

            let post_balance = (
                helpers::get_token_balance(banks_client, &bidders[bidder].0.pubkey())
                    .await,
                helpers::get_token_balance(banks_client, &bidders[bidder].1.pubkey())
                    .await,
            );

            assert_eq!(post_balance.0, pre_balance.0 - amount);
            assert_eq!(post_balance.1, pre_balance.1 + amount);
        }

        Action::Cancel(bidder) => {
            // Get balances pre bidding.
            let pre_balance = (
                helpers::get_token_balance(banks_client, &bidders[bidder].0.pubkey())
                    .await,
                helpers::get_token_balance(banks_client, &bidders[bidder].1.pubkey())
                    .await,
            );

            helpers::cancel_bid(
                banks_client,
                &recent_blockhash,
                &program_id,
                &payer,
                &bidders[bidder].0,
                &bidders[bidder].1,
                &resource,
                &mint,
            )
            .await?;

            let bidder_account = banks_client
                .get_account(bidders[bidder].0.pubkey())
                .await
                .expect("get_account")
                .expect("account not found");

            let post_balance = (
                helpers::get_token_balance(banks_client, &bidders[bidder].0.pubkey())
                    .await,
                helpers::get_token_balance(banks_client, &bidders[bidder].1.pubkey())
                    .await,
            );

            // Assert the balance successfully moves.
            assert_eq!(post_balance.0, pre_balance.0 + pre_balance.1);
            assert_eq!(post_balance.1, 0);
        }

        Action::End => {
            helpers::end_auction(
                banks_client,
                &program_id,
                &recent_blockhash,
                &payer,
                &resource,
            )
            .await?;

            // Assert Auction is actually in ended state.
            let auction: AuctionData = try_from_slice_unchecked(
                &banks_client
                    .get_account(*auction_pubkey)
                    .await
                    .expect("get_account")
                    .expect("account not found")
                    .data,
            )?;

            assert!(auction.ended_at.is_some());
        }
    }

    Ok(())
}

#[cfg(feature = "test-bpf")]
#[tokio::test]
async fn test_incorrect_runs() {
    // Local wrapper around a small test description described by actions.
    #[derive(Debug)]
    struct Test {
        actions: Vec<Action>,
        max_winners: usize,
        price_floor: PriceFloor,
    }

    // A list of auction runs that should succeed. At the end of the run the winning bid state
    // should match the expected result.
    let strategies = [
        Test {
            actions: vec![
                Action::Cancel(0),
                Action::End,
            ],
            max_winners: 3,
            price_floor: PriceFloor::None,
        },

        // Cancel a non-existing bid.
        // Bidding less than the top bidder should fail.
        Test {
            actions: vec![
                Action::Bid(0, 5000),
                Action::Bid(1, 6000),
                Action::Bid(2, 5500),
                Action::Bid(0, 1000),
                Action::Bid(1, 2000),
                Action::Bid(2, 3000),
                Action::Bid(3, 4000),
                Action::Bid(3, 4000),
                Action::End,
            ],
            max_winners: 3,
            price_floor: PriceFloor::None,
        },

        // Bidding less than any bidder should fail.
        Test {
            actions: vec![
                Action::Bid(0, 5000),
                Action::Bid(1, 6000),
                Action::Bid(2, 1000),
                Action::End,
            ],
            max_winners: 3,
            price_floor: PriceFloor::None,
        },

        // Bidding after an auction has been explicitly ended should fail.
        Test {
            actions: vec![
                Action::Bid(0, 5000),
                Action::End,
                Action::Bid(1, 6000),
            ],
            max_winners: 3,
            price_floor: PriceFloor::None,
        },
    ];

    // Run each strategy with a new auction.
    for strategy in strategies.iter() {
        let (
            program_id,
            mut banks_client,
            bidders,
            payer,
            resource,
            mint,
            mint_authority,
            auction_pubkey,
            recent_blockhash,
        ) = setup_auction(true, strategy.max_winners).await;

        let mut failed = false;

        for action in strategy.actions.iter() {
            failed = failed || handle_failing_action(
                &mut banks_client,
                &recent_blockhash,
                &program_id,
                &bidders,
                &mint,
                &payer,
                &resource,
                &auction_pubkey,
                action,
            ).await.is_err();
        }

        // Expect to fail.
        assert!(failed);
    }
}
