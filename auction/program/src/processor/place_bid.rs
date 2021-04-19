//! Places a bid on a running auction, the logic here implements a standard English auction
//! mechanism, once the auction starts, new bids can be made until 10 minutes has passed with no
//! new bid. At this point the auction ends.
//!
//! Possible Attacks to Consider:
//!
//! 1) A user bids many many small bids to fill up the buffer, so that his max bid wins.
//! 2) A user bids a large amount repeatedly to indefinitely delay the auction finishing.
//!
//! A few solutions come to mind: don't allow cancelling bids, and simply prune all bids that
//! are not winning bids from the state.

use crate::{
    errors::AuctionError,
    processor::{AuctionData, Bid, BidderMetadata},
    utils::{assert_owned_by, create_or_allocate_account_raw, assert_derivation, spl_token_transfer},
    PREFIX,
};

use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        borsh::try_from_slice_unchecked,
        entrypoint::ProgramResult,
        msg,
        program::{invoke, invoke_signed},
        program_pack::Pack,
        pubkey::Pubkey,
        system_instruction,
        sysvar::{clock::Clock, Sysvar},
    },
    std::mem,
};

/// Arguments for the PlaceBid instruction discriminant .
#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct PlaceBidArgs {
    /// Size of the bid being placed. The user must have enough SOL to satisfy this amount.
    pub amount: u64,
    /// Resource being bid on.
    pub resource: Pubkey,
}

pub fn place_bid(program_id: &Pubkey, accounts: &[AccountInfo], args: PlaceBidArgs) -> ProgramResult {
    msg!("Iterating Accounts");
    let account_iter = &mut accounts.iter();
    let bidder_act = next_account_info(account_iter)?;
    let bidder_spl_act = next_account_info(account_iter)?;
    let bidder_pot_act = next_account_info(account_iter)?;
    let bidder_meta_act = next_account_info(account_iter)?;
    let auction_act = next_account_info(account_iter)?;
    let mint_account = next_account_info(account_iter)?;
    let mint_authority_account = next_account_info(account_iter)?;
    let clock_sysvar = next_account_info(account_iter)?;
    let rent_act = next_account_info(account_iter)?;
    let system_account = next_account_info(account_iter)?;
    let token_program_account = next_account_info(account_iter)?;

    msg!("Assert Owner");
    assert_owned_by(auction_act, program_id)?;

    // Load the auction, we'll need the state to do anything useful.
    msg!("Assert Auction");
    let auction_bump = assert_derivation(program_id, auction_act, &[
        PREFIX.as_bytes(),
        program_id.as_ref(),
        args.resource.as_ref(),
    ])?;

    // Load the auction and verify this bid is valid.
    let mut auction: AuctionData = try_from_slice_unchecked(&auction_act.data.borrow())?;

    // If the auction mint does not match the passed mint, bail.
    if auction.token_mint != *mint_account.key {
        return Ok(());
    }

    // Use the clock sysvar for timing the auction.
    msg!("Get Clock");
    let clock = Clock::from_account_info(clock_sysvar)?;

    // Do not allow bids post gap-time.
    if let Some(gap) = auction.end_auction_gap {
        msg!("Auction finished, gp time passed.");
        if clock.slot - gap > 10 * 60 {
            return Err(AuctionError::InvalidState.into());
        }
    }

    // Do not allow bids post end-time
    if let Some(end) = auction.end_auction_at {
        msg!("Auction finished, passed end time.");
        if clock.slot > end {
            return Err(AuctionError::InvalidState.into());
        }
    }

    msg!("Assert Metadata");
    let metadata_bump = assert_derivation(program_id, bidder_meta_act, &[
        PREFIX.as_bytes(),
        program_id.as_ref(),
        auction_act.key.as_ref(),
        bidder_act.key.as_ref(),
        "metadata".as_bytes(),
    ])?;

    // Load the users account metadata.
    msg!("Check Meta Allocation");
    if bidder_meta_act.owner != program_id {
        msg!("Failed, Creating");
        create_or_allocate_account_raw(
            *program_id,
            bidder_meta_act,
            rent_act,
            system_account,
            bidder_act,
            mem::size_of::<BidderMetadata>(),
            &[
                PREFIX.as_bytes(),
                program_id.as_ref(),
                auction_act.key.as_ref(),
                bidder_act.key.as_ref(),
                "metadata".as_bytes(),
                &[metadata_bump],
            ],
        )?;
    }

    msg!("Checking Pot Allocation");
    let pot_bump = assert_derivation(program_id, bidder_pot_act, &[
        PREFIX.as_bytes(),
        program_id.as_ref(),
        auction_act.key.as_ref(),
        bidder_act.key.as_ref(),
    ])?;

    if *bidder_pot_act.owner != spl_token::id() {
        msg!("Allocating SPL Account");
        create_or_allocate_account_raw(
            *program_id,
            bidder_pot_act,
            rent_act,
            system_account,
            bidder_act,
            spl_token::state::Account::LEN,
            &[
                PREFIX.as_bytes(),
                program_id.as_ref(),
                auction_act.key.as_ref(),
                bidder_act.key.as_ref(),
                &[pot_bump],
            ],
        )?;

        msg!("Initializing SPL");
        invoke_signed(
            &spl_token::instruction::initialize_account(
                &spl_token::id(),
                bidder_pot_act.key,
                mint_account.key,
                auction_act.key,
            )?,
            &[
                auction_act.clone(),
                bidder_pot_act.clone(),
                mint_account.clone()
            ],
            &[
                // Auction Signs
                &[
                    PREFIX.as_bytes(),
                    program_id.as_ref(),
                    args.resource.as_ref(),
                    &[auction_bump],
                ],
            ],
        );

        msg!("Cool");
        return Ok(());
    }

    // Confirm payers SPL token balance is enough to pay the bid.
    msg!("Loading SPL Token");
    let account_info: spl_token::state::Account =
        spl_token::state::Account::unpack_from_slice(
            &bidder_spl_act.data.borrow()
        )?;

    msg!("Amount: {} < Cost: {}", args.amount, account_info.amount);
    if account_info.amount.saturating_sub(args.amount) <= 0 {
        return Err(AuctionError::BalanceTooLow.into());
    }

    // Transfer amount of SPL token to bid account.
    msg!("Transferring SPL Token");
    let result = invoke(
        &spl_token::instruction::transfer(
            token_program_account.key,
            bidder_spl_act.key,
            bidder_pot_act.key,
            bidder_act.key,
            &[],
            args.amount,
        )?,
        &[
            bidder_spl_act.clone(),
            bidder_pot_act.clone(),
            bidder_act.clone(),
            token_program_account.clone(),
        ],
    );

    // result.map_err(|_| VaultError::TokenTransferFailed.into());

//
//    msg!("Storing new auction state");
//    auction.last_bid = Some(clock.slot);
//    auction.bid_state.place_bid(Bid(pot_key, args.amount))?;
//    auction.serialize(&mut *auction_act.data.borrow_mut())?;

    Ok(())
}

