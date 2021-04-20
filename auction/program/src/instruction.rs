use crate::PREFIX;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    sysvar,
};

pub use crate::processor::{
    cancel_bid::CancelBidArgs, create_auction::CreateAuctionArgs, place_bid::PlaceBidArgs,
    start_auction::StartAuctionArgs,
};

#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum AuctionInstruction {
    CreateAuction(CreateAuctionArgs),
    StartAuction(StartAuctionArgs),
    PlaceBid(PlaceBidArgs),
    CancelBid(CancelBidArgs),
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
            AccountMeta::new(creator_pubkey, false),
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
            AccountMeta::new(bidder_pubkey, false),
            AccountMeta::new(auction_pubkey, false),
            AccountMeta::new(bidder_pot_pubkey, false),
            AccountMeta::new(bidder_meta_pubkey, false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
        ],
        data: AuctionInstruction::CancelBid(args).try_to_vec().unwrap(),
    }
}
