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
    processor::{AuctionData, Bid, BidderMetadata, BidderPot},
    utils::{
        assert_derivation, assert_initialized, assert_owned_by, create_or_allocate_account_raw,
        spl_token_transfer, TokenTransferParams,
    },
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
        rent::Rent,
        system_instruction,
        system_instruction::create_account,
        sysvar::{clock::Clock, Sysvar},
    },
    spl_token::state::Account,
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

pub fn place_bid(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: PlaceBidArgs,
) -> ProgramResult {
    msg!("Iterating Accounts");
    let account_iter = &mut accounts.iter();
    let bidder_act = next_account_info(account_iter)?;
    let bidder_pot_act = next_account_info(account_iter)?;
    let bidder_pot_token_act = next_account_info(account_iter)?;
    let bidder_meta_act = next_account_info(account_iter)?;
    let auction_act = next_account_info(account_iter)?;
    let mint_account = next_account_info(account_iter)?;
    let transfer_authority = next_account_info(account_iter)?;
    let payer = next_account_info(account_iter)?;
    let clock_sysvar = next_account_info(account_iter)?;
    let rent_act = next_account_info(account_iter)?;
    let system_account = next_account_info(account_iter)?;
    let token_program_account = next_account_info(account_iter)?;

    msg!("Assert Owner");
    assert_owned_by(auction_act, program_id)?;
    assert_owned_by(bidder_pot_token_act, &spl_token::id())?;
    let actual_account: Account = assert_initialized(bidder_pot_token_act)?;
    if actual_account.owner != *program_id {
        return Err(AuctionError::BidderPotTokenAccountOwnerMismatch.into());
    }

    // Load the auction, we'll need the state to do anything useful.
    msg!("Assert Auction");
    let auction_bump = assert_derivation(
        program_id,
        auction_act,
        &[
            PREFIX.as_bytes(),
            program_id.as_ref(),
            args.resource.as_ref(),
        ],
    )?;

    // Load the auction and verify this bid is valid.
    let mut auction: AuctionData = try_from_slice_unchecked(&auction_act.data.borrow())?;

    // If the auction mint does not match the passed mint, bail.
    msg!("{} == {}", auction.token_mint, mint_account.key);
    if auction.token_mint != *mint_account.key {
        return Err(AuctionError::InvalidState.into());
    }

    // Use the clock sysvar for timing the auction.
    msg!("Get Clock");
    let clock = Clock::from_account_info(clock_sysvar)?;

    // Do not allow bids post gap-time.
    if let Some(gap) = auction.end_auction_gap {
        if let Some(last_bid) = auction.last_bid {
            if clock.slot - last_bid > gap {
                msg!("Auction finished, gp time passed.");

                return Err(AuctionError::InvalidState.into());
            }
        }
    }

    // Do not allow bids post end-time
    if let Some(end) = auction.ended_at {
        msg!("Auction finished, passed end time.");
        if clock.slot > end {
            return Err(AuctionError::InvalidState.into());
        }
    }

    msg!("Assert Metadata");
    let metadata_bump = assert_derivation(
        program_id,
        bidder_meta_act,
        &[
            PREFIX.as_bytes(),
            program_id.as_ref(),
            auction_act.key.as_ref(),
            bidder_act.key.as_ref(),
            "metadata".as_bytes(),
        ],
    )?;

    // Load the users account metadata.
    msg!("Check Meta Allocation");
    if bidder_meta_act.owner != program_id {
        msg!("Failed, Creating");
        create_or_allocate_account_raw(
            *program_id,
            bidder_meta_act,
            rent_act,
            system_account,
            payer,
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
    let pot_bump = assert_derivation(
        program_id,
        bidder_pot_act,
        &[
            PREFIX.as_bytes(),
            program_id.as_ref(),
            auction_act.key.as_ref(),
            bidder_act.key.as_ref(),
        ],
    )?;

    let bump_authority_seeds = &[
        PREFIX.as_bytes(),
        program_id.as_ref(),
        auction_act.key.as_ref(),
        bidder_act.key.as_ref(),
        &[pot_bump],
    ];

    if bidder_pot_act.data_is_empty() {
        create_or_allocate_account_raw(
            spl_token::id(),
            bidder_pot_act,
            rent_act,
            system_account,
            payer,
            mem::size_of::<BidderPot>(),
            bump_authority_seeds,
        )?;

        let mut bidder_pot: BidderPot =
            try_from_slice_unchecked(&bidder_pot_act.data.borrow_mut())?;

        bidder_pot.bidder_pot = *bidder_pot_token_act.key;
        bidder_pot.serialize(&mut *bidder_pot_act.data.borrow_mut())?;

        msg!("Cool");
    } else {
        let bidder_pot: BidderPot = try_from_slice_unchecked(&bidder_pot_act.data.borrow_mut())?;
        if bidder_pot.bidder_pot != *bidder_pot_token_act.key {
            return Err(AuctionError::BidderPotTokenAccountMismatch.into());
        }
    }

    // Confirm payers SPL token balance is enough to pay the bid.
    msg!("Loading SPL Token");
    let account: Account = Account::unpack_from_slice(&bidder_act.data.borrow())?;

    msg!("Amount: {} < Cost: {}", args.amount, account.amount);
    if account.amount.saturating_sub(args.amount) <= 0 {
        return Err(AuctionError::BalanceTooLow.into());
    }

    // Transfer amount of SPL token to bid account.
    msg!("Transferring SPL Token");
    spl_token_transfer(TokenTransferParams {
        source: bidder_act.clone(),
        destination: bidder_pot_token_act.clone(),
        amount: args.amount,
        authority_signer_seeds: None,
        authority: bidder_act.clone(),
        destination: bidder_pot_act.clone(),
        source: bidder_act.clone(),
        token_program: token_program_account.clone(),
    })?;

    // Update Metadata
    BidderMetadata {
        bidder_pubkey: *bidder_act.key,
        auction_pubkey: *auction_act.key,
        last_bid: args.amount,
        last_bid_timestamp: clock.unix_timestamp,
        last_bid_timestamp_slot: clock.slot,
        cancelled: false,
    }
    .serialize(&mut *bidder_meta_act.data.borrow_mut())?;

    auction.last_bid = Some(clock.slot);
    auction
        .bid_state
        .place_bid(Bid(*bidder_pot_act.key, args.amount))?;
    auction.serialize(&mut *auction_act.data.borrow_mut())?;

    Ok(())
}
