//! Program state processor

use crate::instruction::DepositType;
use {
    borsh::{BorshDeserialize, BorshSerialize},
    num_traits::FromPrimitive,
    solana_program::{
        account_info::next_account_info,
        account_info::AccountInfo,
        borsh::try_from_slice_unchecked,
        clock::{Clock, Epoch},
        decode_error::DecodeError,
        entrypoint::ProgramResult,
        msg,
        program::{invoke, invoke_signed},
        program_error::PrintProgramError,
        program_error::ProgramError,
        program_pack::Pack,
        pubkey::Pubkey,
        rent::Rent,
        stake_history::StakeHistory,
        system_instruction, system_program,
        sysvar::Sysvar,
    },
    spl_token::state::Mint,
};

/// Program state handler.
pub struct Processor {}
impl Processor {
    /// Issue a spl_token `Transfer` instruction.
    #[allow(clippy::too_many_arguments)]
    fn token_transfer<'a>(
        token_program: AccountInfo<'a>,
        source: AccountInfo<'a>,
        destination: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        seeds: &[&[u8]],
        amount: u64,
    ) -> Result<(), ProgramError> {
        let ix = spl_token::instruction::transfer(
            token_program.key,
            source.key,
            destination.key,
            authority.key,
            &[],
            amount,
        )?;
        invoke_signed(&ix, &[source, destination, authority, token_program], seeds)
    }

    /// Processes `Initialize` instruction.
    fn process_accept_offer(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        maker_size: u64,
        taker_size: u64,
        bump_seed: u8,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let maker_src_account = next_account_info(account_info_iter)?;
        let maker_dst_account = next_account_info(account_info_iter)?;

        let taker_src_account = next_account_info(account_info_iter)?;
        let taker_dst_account = next_account_info(account_info_iter)?;

        let auth_maker_account = next_account_info(account_info_iter)?;
        let auth_taker_account = next_account_info(account_info_iter)?;

        let token_program_info = next_account_info(account_info_iter)?;

        let transfer_authority = next_account_info(account_info_iter)?;

        let seeds: &[&[_]] = &[
            &maker_src_account.key.to_bytes(),
            &maker_dst_account.key.to_bytes(),
            &taker_src_mint.key.to_bytes(),
            maker_size.to_bytes(),
            taker_size.to_bytes(),
            &[bump_seed],
        ];
        msg!("start");
        Self::token_transfer(
            token_program_info.clone(),
            maker_src_account.clone(),
            auth_maker_account.clone(),
            transfer_authority.clone(),
            maker_size,
            seeds,
        )?;
        msg!("done tx from maker to temp {}", maker_size);

        Self::token_transfer(
            token_program_info.clone(),
            auth_maker_account.clone(),
            taker_dst_account.clone(),
            transfer_authority.clone(),
            maker_size,
            seeds,
        )?;
        msg!("done tx from temp to taker {}", maker_size);

        Self::token_transfer(
            token_program_info.clone(),
            taker_src_account.clone(),
            auth_taker_account.clone(),
            transfer_authority.clone(),
            taker_size,
            seeds,
        )?;
        msg!("done tx from taker to temp {}", taker_size);

        Self::token_transfer(
            token_program_info.clone(),
            auth_taker_account.clone(),
            maker_dst_account.clone(),
            transfer_authority.clone(),
            taker_size,
            seeds,
        )?;
        msg!("done tx from temp to maker {}", taker_size);
        msg!("done!");
    }

    /// Processes [Instruction](enum.Instruction.html).
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        let instruction = StakePoolInstruction::try_from_slice(input)?;
        match instruction {
            StalessOfferInstruction::AcceptOffer {
                maker_size,
                taker_size,
                bump_seed,
            } => {
                msg!("Instruction: Initialize stake pool");
                Self::process_accept_offer(
                    program_id,
                    accounts,
                    maker_size,
                    taker_size,
                    bump_seed,
                )
            }
        }
    }
}
