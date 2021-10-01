//! Program state processor

use crate::instruction::StatelessOfferInstruction;
use crate::validation_utils::{assert_is_ata, assert_keys_equal};
use {
    borsh::BorshDeserialize,
    solana_program::{
        account_info::next_account_info,
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        msg,
        program::{invoke, invoke_signed},
        pubkey::Pubkey,
    },
};

/// Program state handler.
pub struct Processor {}
impl Processor {
    /// Processes [Instruction](enum.Instruction.html).
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        let instruction = StatelessOfferInstruction::try_from_slice(input)?;
        match instruction {
            StatelessOfferInstruction::AcceptOffer {
                maker_size,
                taker_size,
                bump_seed,
            } => {
                msg!("Instruction: accept offer");
                process_accept_offer(program_id, accounts, maker_size, taker_size, bump_seed)
            }
        }
    }
}

fn process_accept_offer(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    maker_size: u64,
    taker_size: u64,
    bump_seed: u8,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let maker_wallet = next_account_info(account_info_iter)?;
    let taker_wallet = next_account_info(account_info_iter)?;
    let maker_src_account = next_account_info(account_info_iter)?;
    let maker_dst_account = next_account_info(account_info_iter)?;
    let taker_src_account = next_account_info(account_info_iter)?;
    let taker_dst_account = next_account_info(account_info_iter)?;
    let maker_src_mint = next_account_info(account_info_iter)?;
    let taker_src_mint = next_account_info(account_info_iter)?;
    let transfer_authority = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;
    let seeds = &[
        b"stateless_offer",
        maker_src_account.key.as_ref(),
        maker_dst_account.key.as_ref(),
        taker_src_mint.key.as_ref(),
        &maker_size.to_le_bytes(),
        &taker_size.to_le_bytes(),
        &[bump_seed],
    ];
    let authority_key = Pubkey::create_program_address(seeds, program_id).unwrap();
    assert_keys_equal(authority_key, *transfer_authority.key)?;
    assert_is_ata(maker_src_account, maker_wallet.key, maker_src_mint.key)?;
    assert_is_ata(maker_dst_account, maker_wallet.key, taker_src_mint.key)?;
    assert_is_ata(taker_src_account, taker_wallet.key, taker_src_mint.key)?;
    assert_is_ata(taker_dst_account, taker_wallet.key, maker_src_mint.key)?;
    msg!("start");
    // Both of these transfers will fail if the `transfer_authority` is the delegate of these ATA's
    // One consideration is that the taker can get tricked in the case that the maker size is greater than
    // the token amount in the maker's ATA, but these stateless offers should just be invalidated in
    // the client.
    invoke_signed(
        &spl_token::instruction::transfer(
            token_program_info.key,
            maker_src_account.key,
            taker_dst_account.key,
            transfer_authority.key,
            &[],
            maker_size,
        )?,
        &[
            maker_src_account.clone(),
            taker_dst_account.clone(),
            transfer_authority.clone(),
            token_program_info.clone(),
        ],
        &[seeds],
    )?;
    msg!("done tx from maker to taker {}", maker_size);
    invoke(
        &spl_token::instruction::transfer(
            token_program_info.key,
            taker_src_account.key,
            maker_dst_account.key,
            taker_wallet.key,
            &[],
            taker_size,
        )?,
        &[
            taker_src_account.clone(),
            maker_dst_account.clone(),
            taker_wallet.clone(),
            token_program_info.clone(),
        ],
    )?;
    msg!("done tx from taker to maker {}", taker_size);
    msg!("done!");
    Ok(())
}
