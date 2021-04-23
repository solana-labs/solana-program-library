use crate::{
    errors::AuctionError,
    processor::{AuctionData, AuctionState, Bid, BidState, WinnerLimit},
    utils::{assert_derivation, assert_owned_by, create_or_allocate_account_raw},
    PREFIX,
};

use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        borsh::try_from_slice_unchecked,
        clock::Clock,
        entrypoint::ProgramResult,
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
        sysvar::Sysvar,
    },
    std::mem,
};

pub fn start_auction<'a, 'b: 'a>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'b>],
    args: StartAuctionArgs,
) -> ProgramResult {
    let accounts = parse_accounts(program_id, accounts)?;
    let clock = Clock::from_account_info(accounts.clock_sysvar)?;

    // Derive auction address so we can make the modifications necessary to start it.
    assert_derivation(
        program_id,
        accounts.auction,
        &[
            PREFIX.as_bytes(),
            program_id.as_ref(),
            &args.resource.as_ref(),
        ],
    )?;

    // Initialise a new auction.
    let mut auction: AuctionData = try_from_slice_unchecked(&accounts.auction.data.borrow())?;
    let ended_at = if let Some(end_auction_at) = auction.end_auction_at {
        match clock.slot.checked_add(end_auction_at) {
            Some(val) => Some(val),
            None => return Err(AuctionError::NumericalOverflowError.into()),
        }
    } else {
        None
    };

    AuctionData {
        ended_at,
        state: auction.state.start()?,
        ..auction
    }
    .serialize(&mut *accounts.auction.data.borrow_mut())?;

    Ok(())
}

struct Accounts<'a, 'b: 'a> {
    creator: &'a AccountInfo<'b>,
    auction: &'a AccountInfo<'b>,
    clock_sysvar: &'a AccountInfo<'b>,
}

fn parse_accounts<'a, 'b: 'a>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'b>],
) -> Result<Accounts<'a, 'b>, ProgramError> {
    let account_iter = &mut accounts.iter();
    let accounts = Accounts {
        creator: next_account_info(account_iter)?,
        auction: next_account_info(account_iter)?,
        clock_sysvar: next_account_info(account_iter)?,
    };
    assert_owned_by(accounts.auction, program_id)?;
    Ok(accounts)
}

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct StartAuctionArgs {
    /// The resource being auctioned. See AuctionData.
    pub resource: Pubkey,
}
