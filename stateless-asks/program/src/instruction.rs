//! Instruction types

#![allow(clippy::too_many_arguments)]
use {
    borsh::{BorshDeserialize, BorshSchema, BorshSerialize},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        system_program, sysvar,
    },
};

/// Instructions supported by the StatelessOffer program.
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize, BorshSchema)]
pub enum StatelessOfferInstruction {
    ///  Accept a StatelessOffer
    ///
    AcceptOffer {
        maker_size: u64,
        taker_size: u64,
        bump_seed: u8,
    },
}

/// Creates an 'initialize' instruction.
pub fn accept_offer(
    program_id: &Pubkey,
    maker_src_account: &Pubkey,
    maker_dst_account: &Pubkey,
    taker_src_account: &Pubkey,
    taker_dst_account: &Pubkey,
    auth_maker_account: &Pubkey,
    auth_taker_account: &Pubkey,
    authority: &Pubkey,
    token_program_id: &Pubkey,
    bump_seed: u8,
) -> Instruction {
    let init_data = StatelessOfferInstruction::AcceptOffer {
        maker_size,
        taker_size,
        bump_seed,
    };
    let data = init_data.try_to_vec().unwrap();
    let mut accounts = vec![
        AccountMeta::new(*program_id, false),

        AccountMeta::new(*maker_src_account, false),
        AccountMeta::new(*maker_dst_account, false),


        AccountMeta::new(*taker_src_account, false),
        AccountMeta::new(*taker_dst_account, false),

        AccountMeta::new(*auth_maker_account, false),
        AccountMeta::new(*auth_taker_account, false),

        AccountMeta::new_readonly(*token_program_id, false),
        AccountMeta::new_readonly(*authority, false),
    ];
    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}
