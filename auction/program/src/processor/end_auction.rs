use crate::{
    errors::AuctionError,
    processor::{AuctionData, AuctionState, Bid, BidState, PriceFloor, WinnerLimit},
    utils::{assert_derivation, assert_signer, assert_owned_by, create_or_allocate_account_raw},
    PREFIX,
};

use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        borsh::try_from_slice_unchecked,
        clock::Clock,
        entrypoint::ProgramResult,
        hash, msg,
        program_error::ProgramError,
        pubkey::Pubkey,
        sysvar::Sysvar,
    },
    std::mem,
};

type Price = u64;
type Salt = u64;
type Revealer = (Price, Salt);

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct EndAuctionArgs {
    /// The resource being auctioned. See AuctionData.
    pub resource: Pubkey,
    /// If the auction was blinded, a revealing price must be specified to release the auction
    /// winnings.
    pub reveal: Option<Revealer>,
}

struct Accounts<'a, 'b: 'a> {
    authority: &'a AccountInfo<'b>,
    auction: &'a AccountInfo<'b>,
    clock_sysvar: &'a AccountInfo<'b>,
}

fn parse_accounts<'a, 'b: 'a>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'b>],
) -> Result<Accounts<'a, 'b>, ProgramError> {
    let account_iter = &mut accounts.iter();
    let accounts = Accounts {
        authority: next_account_info(account_iter)?,
        auction: next_account_info(account_iter)?,
        clock_sysvar: next_account_info(account_iter)?,
    };
    assert_owned_by(accounts.auction, program_id)?;
    assert_signer(accounts.authority)?;
    Ok(accounts)
}

fn reveal(price_floor: PriceFloor, revealer: Option<Revealer>) -> Result<PriceFloor, ProgramError> {
    // If the price floor was blinded, we update it.
    if let PriceFloor::BlindedPrice(blinded) = price_floor {
        // If the hash matches, update the price to the actual minimum.
        if let Some(reveal) = revealer {
            let reveal_hash = hash::hashv(&[&reveal.0.to_be_bytes(), &reveal.1.to_be_bytes()]);
            if reveal_hash != blinded {
                return Err(AuctionError::InvalidReveal.into());
            }
            Ok(PriceFloor::MinimumPrice(reveal.0))
        } else {
            return Err(AuctionError::MustReveal.into());
        }
    } else {
        // No change needed in the else case.
        Ok(price_floor)
    }
}

pub fn end_auction<'a, 'b: 'a>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'b>],
    args: EndAuctionArgs,
) -> ProgramResult {
    msg!("+ Processing EndAuction");
    let accounts = parse_accounts(program_id, accounts)?;
    let clock = Clock::from_account_info(accounts.clock_sysvar)?;

    assert_derivation(
        program_id,
        accounts.auction,
        &[
            PREFIX.as_bytes(),
            program_id.as_ref(),
            &args.resource.as_ref(),
        ],
    )?;

    // End auction.
    let mut auction: AuctionData = try_from_slice_unchecked(&accounts.auction.data.borrow())?;

    // Check authority is correct.
    if auction.authority != *accounts.authority.key {
        return Err(AuctionError::InvalidAuthority.into());
    }

    // As long as it hasn't already ended.
    if auction.ended_at.is_some() {
        return Err(AuctionError::AuctionTransitionInvalid.into());
    }

    AuctionData {
        ended_at: Some(clock.slot),
        state: auction.state.end()?,
        price_floor: reveal(auction.price_floor, args.reveal)?,
        ..auction
    }
    .serialize(&mut *accounts.auction.data.borrow_mut())?;

    Ok(())
}
