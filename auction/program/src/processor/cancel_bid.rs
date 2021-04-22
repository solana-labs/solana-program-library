//! Cancels an existing bid. This only works in two cases:
//!
//! 1) The auction is still going on, in which case it is possible to cancel a bid at any time.
//! 2) The auction has finished, but the bid did not win. This allows users to claim back their
//!    funds from bid accounts.

use crate::{
    errors::AuctionError,
    processor::{AuctionData, BidderMetadata, BidderPot},
    utils::{
        assert_derivation, assert_initialized, assert_owned_by, create_or_allocate_account_raw, spl_token_transfer, TokenTransferParams
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
        program::invoke_signed,
        program_error::ProgramError,
        program_pack::Pack,
        pubkey::Pubkey,
        system_instruction,
        sysvar::{clock::Clock, Sysvar},
    },
    spl_token::state::Account,
};

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct CancelBidArgs {
    pub resource: Pubkey,
}

struct Accounts<'a, 'b: 'a> {
    auction: &'a AccountInfo<'b>,
    bidder_meta: &'a AccountInfo<'b>,
    bidder_pot: &'a AccountInfo<'b>,
    bidder_pot_token: &'a AccountInfo<'b>,
    bidder: &'a AccountInfo<'b>,
    clock_sysvar: &'a AccountInfo<'b>,
    mint: &'a AccountInfo<'b>,
    payer: &'a AccountInfo<'b>,
    rent: &'a AccountInfo<'b>,
    system: &'a AccountInfo<'b>,
    token_program: &'a AccountInfo<'b>,
}

fn parse_accounts<'a, 'b: 'a>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'b>],
) -> Result<Accounts<'a, 'b>, ProgramError> {
    let account_iter = &mut accounts.iter();
    let accounts = Accounts {
        bidder: next_account_info(account_iter)?,
        bidder_pot: next_account_info(account_iter)?,
        bidder_pot_token: next_account_info(account_iter)?,
        bidder_meta: next_account_info(account_iter)?,
        auction: next_account_info(account_iter)?,
        mint: next_account_info(account_iter)?,
        payer: next_account_info(account_iter)?,
        clock_sysvar: next_account_info(account_iter)?,
        rent: next_account_info(account_iter)?,
        system: next_account_info(account_iter)?,
        token_program: next_account_info(account_iter)?,
    };

    assert_owned_by(accounts.auction, program_id)?;
    assert_owned_by(accounts.bidder_pot_token, &spl_token::id())?;
    Ok(accounts)
}

pub fn cancel_bid(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: CancelBidArgs,
) -> ProgramResult {
    let accounts = parse_accounts(program_id, accounts)?;

    // The account within the pot must be owned by us.
    let actual_account: Account = assert_initialized(accounts.bidder_pot_token)?;
    if actual_account.owner != *accounts.auction.key {
        return Err(AuctionError::BidderPotTokenAccountOwnerMismatch.into());
    }

    // Derive and load Auction.
    let auction_bump = assert_derivation(
        program_id,
        accounts.auction,
        &[
            PREFIX.as_bytes(),
            program_id.as_ref(),
            args.resource.as_ref(),
        ],
    )?;

    let auction_seeds = 
        &[
            PREFIX.as_bytes(),
            program_id.as_ref(),
            args.resource.as_ref(),
            &[auction_bump],
        ];

    // Load the auction and verify this bid is valid.
    let mut auction: AuctionData = try_from_slice_unchecked(&accounts.auction.data.borrow())?;

    // The mint provided in this bid must match the one the auction was initialized with.
    if auction.token_mint != *accounts.mint.key {
        return Ok(());
    }

    // Load the clock, used for various auction timing.
    let clock = Clock::from_account_info(accounts.clock_sysvar)?;

    // If the auction has finished, and this bid was not a winning bid, the user can claim their
    // funds back with a cancel.
    // TODO: Fix
    match (auction.ended_at, auction.end_auction_at) {
        (Some(end), _) if clock.slot > end => return Err(AuctionError::InvalidState.into()),
        (_, Some(end)) if clock.slot > end => return Err(AuctionError::InvalidState.into()),
        _ => {}
    }

    // Derive Metadata key and load it.
    let metadata_bump = assert_derivation(
        program_id,
        accounts.bidder_meta,
        &[
            PREFIX.as_bytes(),
            program_id.as_ref(),
            accounts.auction.key.as_ref(),
            accounts.bidder.key.as_ref(),
            "metadata".as_bytes(),
        ],
    )?;

    // If metadata doesn't exist, error, can't cancel a bid that doesn't exist and metadata must
    // exist if a bid was placed.
    if accounts.bidder_meta.owner != program_id {
        return Err(AuctionError::MetadataInvalid.into());
    }

    // Derive Pot address, this account wraps/holds an SPL account to transfer tokens into.
    let pot_seeds = [
        PREFIX.as_bytes(),
        program_id.as_ref(),
        accounts.auction.key.as_ref(),
        accounts.bidder.key.as_ref(),
    ];

    let pot_bump = assert_derivation(
        program_id,
        accounts.bidder_pot,
        &pot_seeds,
    )?;

    msg!("Bump: {} {}", accounts.bidder_pot.key, pot_bump);
    let bump_authority_seeds = &[
        PREFIX.as_bytes(),
        program_id.as_ref(),
        accounts.auction.key.as_ref(),
        accounts.bidder.key.as_ref(),
        &[pot_bump],
    ];

    // If the bidder pot account is empty, this bid is invalid.
    if accounts.bidder_pot.data_is_empty() {
        return Err(AuctionError::BidderPotDoesNotExist.into());
    }

    // Confirm we're looking at the real SPL account for this bidder.
    let bidder_pot: BidderPot = try_from_slice_unchecked(&accounts.bidder_pot.data.borrow_mut())?;
    if bidder_pot.bidder_pot != *accounts.bidder_pot_token.key {
        return Err(AuctionError::BidderPotTokenAccountOwnerMismatch.into());
    }

    // Transfer SPL bid balance back to the user.
    let account: Account = Account::unpack_from_slice(&accounts.bidder_pot_token.data.borrow())?;
    spl_token_transfer(TokenTransferParams {
        source: accounts.bidder_pot_token.clone(),
        destination: accounts.bidder.clone(),
        authority: accounts.auction.clone(),
        authority_signer_seeds: auction_seeds,
        token_program: accounts.token_program.clone(),
        amount: account.amount,
    })?;

    // Update Metadata
    let metadata: BidderMetadata = try_from_slice_unchecked(&accounts.bidder_meta.data.borrow_mut())?;
    BidderMetadata {
        cancelled: true,
        ..metadata
    }.serialize(&mut *accounts.bidder_meta.data.borrow_mut())?;

    Ok(())
}
