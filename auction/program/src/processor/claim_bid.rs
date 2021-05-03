//! Claim bid winnings into a target SPL account, only the authorised key can do this, though the
//! target can be any SPL account.

use crate::{
    errors::AuctionError,
    processor::{AuctionData, BidderMetadata, BidderPot},
    utils::{
        assert_derivation, assert_signer, assert_initialized, assert_owned_by, create_or_allocate_account_raw,
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
pub struct ClaimBidArgs {
    pub resource: Pubkey,
}

struct Accounts<'a, 'b: 'a> {
    authority: &'a AccountInfo<'b>,
    auction: &'a AccountInfo<'b>,
    bidder_pot: &'a AccountInfo<'b>,
    bidder_pot_token: &'a AccountInfo<'b>,
    bidder: &'a AccountInfo<'b>,
    clock_sysvar: &'a AccountInfo<'b>,
    mint: &'a AccountInfo<'b>,
    destination: &'a AccountInfo<'b>,
    token_program: &'a AccountInfo<'b>,
}

fn parse_accounts<'a, 'b: 'a>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'b>],
) -> Result<Accounts<'a, 'b>, ProgramError> {
    let account_iter = &mut accounts.iter();
    let accounts = Accounts {
        authority: next_account_info(account_iter)?,
        destination: next_account_info(account_iter)?,
        bidder: next_account_info(account_iter)?,
        bidder_pot: next_account_info(account_iter)?,
        bidder_pot_token: next_account_info(account_iter)?,
        auction: next_account_info(account_iter)?,
        mint: next_account_info(account_iter)?,
        clock_sysvar: next_account_info(account_iter)?,
        token_program: next_account_info(account_iter)?,
    };

    assert_owned_by(accounts.auction, program_id)?;
    assert_owned_by(accounts.mint, &spl_token::id())?;
    assert_owned_by(accounts.destination, &spl_token::id())?;
    assert_owned_by(accounts.bidder_pot_token, &spl_token::id())?;
    assert_signer(accounts.authority)?;

    Ok(accounts)
}

pub fn claim_bid(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: ClaimBidArgs,
) -> ProgramResult {
    msg!("+ Processing ClaimBid");
    let accounts = parse_accounts(program_id, accounts)?;
    let clock = Clock::from_account_info(accounts.clock_sysvar)?;

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

    let auction_seeds = &[
        PREFIX.as_bytes(),
        program_id.as_ref(),
        args.resource.as_ref(),
        &[auction_bump],
    ];

    // Load the auction and verify this bid is valid.
    let mut auction: AuctionData = try_from_slice_unchecked(&accounts.auction.data.borrow())?;

    // User must have won the auction in order to claim their funds. Check early as the rest of the
    // checks will be for nothing otherwise.
    if auction.is_winner(*accounts.bidder_pot.key).is_none() {
        return Err(AuctionError::InvalidState.into());
    }

    // Auction must have ended.
    if !auction.ended(clock.slot) {
        return Err(AuctionError::InvalidState.into());
    }

    // Pot must contain tokens.
    let account: Account = Account::unpack_from_slice(&accounts.bidder_pot_token.data.borrow())?;
    if account.amount <= 0 {
        return Err(AuctionError::BidderPotEmpty.into());
    }

    // The mint provided in this claim must match the one the auction was initialized with.
    if auction.token_mint != *accounts.mint.key {
        return Err(AuctionError::IncorrectMint.into());
    }

    // Derive Pot address, this account wraps/holds an SPL account to transfer tokens into.
    let pot_seeds = [
        PREFIX.as_bytes(),
        program_id.as_ref(),
        accounts.auction.key.as_ref(),
        accounts.bidder.key.as_ref(),
    ];

    let pot_bump = assert_derivation(program_id, accounts.bidder_pot, &pot_seeds)?;

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
    spl_token_transfer(TokenTransferParams {
        source: accounts.bidder_pot_token.clone(),
        destination: accounts.destination.clone(),
        authority: accounts.auction.clone(),
        authority_signer_seeds: auction_seeds,
        token_program: accounts.token_program.clone(),
        amount: account.amount,
    })?;

    Ok(())
}

