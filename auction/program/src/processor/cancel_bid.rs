
//! Cancels an existing bid. This only works in two cases:
//!
//! 1) The auction is still going on, in which case it is possible to cancel a bid at any time.
//! 2) The auction has finished, but the bid did not win. This allows users to claim back their
//!    funds from bid accounts.

use crate::{
    errors::AuctionError,
    processor::AuctionData,
    utils::{assert_owned_by, create_or_allocate_account_raw},
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
        pubkey::Pubkey,
        system_instruction,
    },
};

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct CancelBidArgs {
    pub resource: Pubkey
}

pub fn cancel_bid(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let bidder_act = next_account_info(account_iter)?;
    let auction_act = next_account_info(account_iter)?;
    let bidder_pot = next_account_info(account_iter)?;

    // This  path references an account to store the users bid SOL in, if the user wins the auction
    // this is claimed by the auction authority, otherwise the user can request to have the SOL
    // sent back.
    let pot_path = [
        PREFIX.as_bytes(),
        program_id.as_ref(),
        auction_act.key.as_ref(),
        bidder_act.key.as_ref(),
    ];

    // Derive pot key, confirm it matches the users sent pot address.
    let (pot_key, pot_bump) = Pubkey::find_program_address(&pot_path, program_id);
    if pot_key != *bidder_pot.key {
        return Err(AuctionError::InvalidBidAccount.into());
    }

    // Scan and remove the bid (Expensive, need a better datastructure).
    msg!("Loading AuctionData");
    let mut auction: AuctionData = try_from_slice_unchecked(&auction_act.data.borrow())?;
    msg!("Cancelling Bid");
    auction.bid_state.cancel_bid(pot_key)?;

    // Pot path including the bump for seeds.
    let pot_seeds = [
        PREFIX.as_bytes(),
        program_id.as_ref(),
        auction_act.key.as_ref(),
        bidder_act.key.as_ref(),
        &[pot_bump],
    ];

    // Transfer SOL from the bidder's SOL account into their pot.
    msg!("Invoking Transfer back to the bidders account");
    invoke_signed(
        &system_instruction::transfer(&pot_key, bidder_act.key, bidder_pot.lamports()),
        &[bidder_pot.clone(), bidder_act.clone()],
        &[&pot_seeds],
    )?;

    // Write modified AuctionData.
    auction.serialize(&mut *auction_act.data.borrow_mut())?;

    Ok(())
}
