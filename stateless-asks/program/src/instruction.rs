//! Instruction types

use {
    borsh::{BorshDeserialize, BorshSchema, BorshSerialize},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        system_program,
    },
};

/// Instructions supported by the StatelessOffer program.
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize, BorshSchema)]
pub enum StatelessOfferInstruction {
    ///  Accept a StatelessOffer
    ///  Let's walk through the actions of Alice (maker) and Bob (taker)
    ///
    ///  Alice has some amount Token A in mkr_src_account and she creates mkr_dst_account if it doesn't exist
    ///  Alice calls Approve on mkr_src_account for maker_size to some transfer_authority owned by the Stateless Ask program.
    ///  This transfer_authority's approval size/mint are expressed in the seeds of the PDA
    ///
    ///  Some time later:
    ///
    ///  Bob initializes tkr_src_account (Token B) and tkr_dst_account (Token A) if they don't exist
    ///  Bob (or anyone) executes AcceptOffer
    ///
    AcceptOffer {
        #[allow(dead_code)]
        has_metadata: bool,
        #[allow(dead_code)]
        maker_size: u64,
        #[allow(dead_code)]
        taker_size: u64,
        #[allow(dead_code)]
        bump_seed: u8,
    },
}

/// Creates an 'initialize' instruction.
#[allow(clippy::too_many_arguments)]
pub fn accept_offer(
    program_id: &Pubkey,
    maker_wallet: &Pubkey,
    taker_wallet: &Pubkey,
    maker_src_account: &Pubkey,
    maker_dst_account: &Pubkey,
    taker_src_account: &Pubkey,
    taker_dst_account: &Pubkey,
    maker_mint: &Pubkey,
    taker_mint: &Pubkey,
    authority: &Pubkey,
    token_program_id: &Pubkey,
    is_native: bool,
    maker_size: u64,
    taker_size: u64,
    bump_seed: u8,
) -> Instruction {
    let init_data = StatelessOfferInstruction::AcceptOffer {
        has_metadata: false,
        maker_size,
        taker_size,
        bump_seed,
    };
    let data = init_data.try_to_vec().unwrap();
    let mut accounts = vec![
        AccountMeta::new_readonly(*maker_wallet, false),
        AccountMeta::new_readonly(*taker_wallet, true),
        AccountMeta::new(*maker_src_account, false),
        AccountMeta::new(*maker_dst_account, false),
        AccountMeta::new(*taker_src_account, false),
        AccountMeta::new(*taker_dst_account, false),
        AccountMeta::new_readonly(*maker_mint, false),
        AccountMeta::new_readonly(*taker_mint, false),
        AccountMeta::new_readonly(*authority, false),
        AccountMeta::new_readonly(*token_program_id, false),
    ];
    if is_native {
        accounts.push(AccountMeta::new_readonly(system_program::id(), false));
    }
    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

/// Creates an 'initialize' instruction.
#[allow(clippy::too_many_arguments)]
pub fn accept_offer_with_metadata(
    program_id: &Pubkey,
    maker_wallet: &Pubkey,
    taker_wallet: &Pubkey,
    maker_src_account: &Pubkey,
    maker_dst_account: &Pubkey,
    taker_src_account: &Pubkey,
    taker_dst_account: &Pubkey,
    maker_mint: &Pubkey,
    taker_mint: &Pubkey,
    authority: &Pubkey,
    token_program_id: &Pubkey,
    metadata: &Pubkey,
    creators: &[&Pubkey],
    is_native: bool,
    maker_size: u64,
    taker_size: u64,
    bump_seed: u8,
) -> Instruction {
    let init_data = StatelessOfferInstruction::AcceptOffer {
        has_metadata: true,
        maker_size,
        taker_size,
        bump_seed,
    };
    let data = init_data.try_to_vec().unwrap();
    let mut accounts = vec![
        AccountMeta::new_readonly(*maker_wallet, false),
        AccountMeta::new_readonly(*taker_wallet, true),
        AccountMeta::new(*maker_src_account, false),
        AccountMeta::new(*maker_dst_account, false),
        AccountMeta::new(*taker_src_account, false),
        AccountMeta::new(*taker_dst_account, false),
        AccountMeta::new_readonly(*maker_mint, false),
        AccountMeta::new_readonly(*taker_mint, false),
        AccountMeta::new_readonly(*authority, false),
        AccountMeta::new_readonly(*token_program_id, false),
    ];
    if is_native {
        accounts.push(AccountMeta::new_readonly(system_program::id(), false));
    }
    accounts.push(AccountMeta::new_readonly(*metadata, false));
    for creator in creators.iter() {
        accounts.push(AccountMeta::new(**creator, false));
    }
    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}
