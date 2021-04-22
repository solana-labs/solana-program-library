use crate::PREFIX;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    sysvar,
};

pub use crate::processor::{
    cancel_bid::CancelBidArgs, create_auction::CreateAuctionArgs, place_bid::PlaceBidArgs,
    start_auction::StartAuctionArgs, end_auction::EndAuctionArgs,
};

#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum AuctionInstruction {
    /// Create a new auction account bound to a resource, initially in a pending state.
    ///   0. `[signer]` The account creating the auction, which is authorised to make changes.
    ///   1. `[writable]` Uninitialized auction account.
    ///   2. `[]` Rent sysvar
    ///   3. `[]` System account
    CreateAuction(CreateAuctionArgs),

    /// Start an inactive auction.
    ///   0. `[signer]` The creator/authorised account.
    ///   1. `[writable]` Initialized auction account.
    ///   2. `[]` Clock sysvar
    StartAuction(StartAuctionArgs),

    /// Ends an auction, regardless of end timing conditions
    EndAuction(EndAuctionArgs),

    /// Place a bid on a running auction.
    ///   0. `[signer]` The bidders primary account, for PDA calculation/transit auth.
    ///   1. `[writable]` The pot, containing a reference to the stored SPL token account.
    ///   2. `[writable]` The pot SPL account, where the tokens will be deposited.
    ///   3. `[writable]` The metadata account, storing information about the bidders actions.
    ///   4. `[writable]` Auction account, containing data about the auction and item being bid on.
    ///   5. `[writable]` Token mint, for transfer instructions and verification.
    ///   6. `[signer]` Transfer authority, for moving tokens into the bid pot.
    ///   7. `[signer]` Payer
    ///   8. `[]` Clock sysvar
    ///   9. `[]` Rent sysvar
    ///   10. `[]` System program
    ///   11. `[]` SPL Token Program
    PlaceBid(PlaceBidArgs),

    /// Place a bid on a running auction.
    ///   0. `[signer]` The bidders primary account, for PDA calculation/transit auth.
    ///   1. `[writable]` The pot, containing a reference to the stored SPL token account.
    ///   2. `[writable]` The pot SPL account, where the tokens will be deposited.
    ///   3. `[writable]` The metadata account, storing information about the bidders actions.
    ///   4. `[writable]` Auction account, containing data about the auction and item being bid on.
    ///   5. `[writable]` Token mint, for transfer instructions and verification.
    ///   7. `[signer]` Payer
    ///   8. `[]` Clock sysvar
    ///   9. `[]` Rent sysvar
    ///   10. `[]` System program
    ///   11. `[]` SPL Token Program
    CancelBid(CancelBidArgs),

    /// Update the authority for an auction account.
    SetAuthority,
}

/// Creates an CreateAuction instruction.
pub fn create_auction_instruction(
    program_id: Pubkey,
    creator_pubkey: Pubkey,
    args: CreateAuctionArgs,
) -> Instruction {
    let seeds = &[
        PREFIX.as_bytes(),
        &program_id.as_ref(),
        args.resource.as_ref(),
    ];
    let (auction_pubkey, _) = Pubkey::find_program_address(seeds, &program_id);
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(creator_pubkey, true),
            AccountMeta::new(auction_pubkey, false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
        ],
        data: AuctionInstruction::CreateAuction(args)
            .try_to_vec()
            .unwrap(),
    }
}

/// Creates an SetAuthority instruction.
pub fn set_authority_instruction(
    program_id: Pubkey,
    resource: Pubkey,
    authority: Pubkey,
    new_authority: Pubkey,
) -> Instruction {
    let seeds = &[PREFIX.as_bytes(), &program_id.as_ref(), resource.as_ref()];
    let (auction_pubkey, _) = Pubkey::find_program_address(seeds, &program_id);
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(auction_pubkey, false),
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(new_authority, false),
        ],
        data: AuctionInstruction::SetAuthority.try_to_vec().unwrap(),
    }
}

/// Creates an StartAuction instruction.
pub fn start_auction_instruction(
    program_id: Pubkey,
    creator_pubkey: Pubkey,
    args: StartAuctionArgs,
) -> Instruction {
    // Derive Auction Key
    let seeds = &[
        PREFIX.as_bytes(),
        &program_id.as_ref(),
        args.resource.as_ref(),
    ];
    let (auction_pubkey, _) = Pubkey::find_program_address(seeds, &program_id);

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(creator_pubkey, true),
            AccountMeta::new(auction_pubkey, false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ],
        data: AuctionInstruction::StartAuction(args).try_to_vec().unwrap(),
    }
}

/// Creates an PlaceBid instruction.
pub fn place_bid_instruction(
    program_id: Pubkey,
    bidder_pubkey: Pubkey,
    bidder_pot_token_pubkey: Pubkey,
    token_mint_pubkey: Pubkey,
    transfer_authority: Pubkey,
    payer: Pubkey,
    args: PlaceBidArgs,
) -> Instruction {
    // Derive Auction Key
    let seeds = &[
        PREFIX.as_bytes(),
        program_id.as_ref(),
        args.resource.as_ref(),
    ];
    let (auction_pubkey, _) = Pubkey::find_program_address(seeds, &program_id);

    // Derive Bidder Pot
    let seeds = &[
        PREFIX.as_bytes(),
        &program_id.as_ref(),
        auction_pubkey.as_ref(),
        bidder_pubkey.as_ref(),
    ];
    let (bidder_pot_pubkey, _) = Pubkey::find_program_address(seeds, &program_id);

    // Derive Bidder Meta
    let seeds = &[
        PREFIX.as_bytes(),
        &program_id.as_ref(),
        auction_pubkey.as_ref(),
        bidder_pubkey.as_ref(),
        "metadata".as_bytes(),
    ];
    let (bidder_meta_pubkey, _) = Pubkey::find_program_address(seeds, &program_id);

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(bidder_pubkey, true),
            AccountMeta::new(bidder_pot_pubkey, false),
            AccountMeta::new(bidder_pot_token_pubkey, false),
            AccountMeta::new(bidder_meta_pubkey, false),
            AccountMeta::new(auction_pubkey, false),
            AccountMeta::new(token_mint_pubkey, false),
            AccountMeta::new_readonly(transfer_authority, true),
            AccountMeta::new_readonly(payer, true),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: AuctionInstruction::PlaceBid(args).try_to_vec().unwrap(),
    }
}

/// Creates an CancelBidinstruction.
pub fn cancel_bid_instruction(
    program_id: Pubkey,
    bidder_pubkey: Pubkey,
    bidder_pot_token_pubkey: Pubkey,
    token_mint_pubkey: Pubkey,
    payer: Pubkey,
    args: CancelBidArgs,
) -> Instruction {
    // Derive Auction Key
    let seeds = &[
        PREFIX.as_bytes(),
        program_id.as_ref(),
        args.resource.as_ref(),
    ];
    let (auction_pubkey, _) = Pubkey::find_program_address(seeds, &program_id);

    // Derive Bidder Pot
    let seeds = &[
        PREFIX.as_bytes(),
        &program_id.as_ref(),
        auction_pubkey.as_ref(),
        bidder_pubkey.as_ref(),
    ];
    let (bidder_pot_pubkey, _) = Pubkey::find_program_address(seeds, &program_id);

    // Derive Bidder Meta
    let seeds = &[
        PREFIX.as_bytes(),
        &program_id.as_ref(),
        auction_pubkey.as_ref(),
        bidder_pubkey.as_ref(),
        "metadata".as_bytes(),
    ];
    let (bidder_meta_pubkey, _) = Pubkey::find_program_address(seeds, &program_id);

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(bidder_pubkey, true),
            AccountMeta::new(bidder_pot_pubkey, false),
            AccountMeta::new(bidder_pot_token_pubkey, false),
            AccountMeta::new(bidder_meta_pubkey, false),
            AccountMeta::new(auction_pubkey, false),
            AccountMeta::new(token_mint_pubkey, false),
            AccountMeta::new_readonly(payer, true),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: AuctionInstruction::CancelBid(args).try_to_vec().unwrap(),
    }
}
