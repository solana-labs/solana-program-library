//! Program state processor

use metaplex_token_metadata::state::Metadata;
use solana_program::program_option::COption;

use crate::error::UtilError;
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
        program_error::ProgramError,
        program_pack::Pack,
        pubkey::Pubkey,
        system_instruction, system_program,
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
                has_metadata,
                maker_size,
                taker_size,
                bump_seed,
            } => {
                msg!("Instruction: accept offer");
                process_accept_offer(
                    program_id,
                    accounts,
                    has_metadata,
                    maker_size,
                    taker_size,
                    bump_seed,
                )
            }
        }
    }
}

fn process_accept_offer(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    has_metadata: bool,
    maker_size: u64,
    mut taker_size: u64,
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
    let mut system_program_info: Option<&AccountInfo> = None;
    let is_native = *taker_src_mint.key == spl_token::native_mint::id();
    if is_native {
        assert_keys_equal(*taker_wallet.key, *taker_src_account.key)?;
        assert_keys_equal(*maker_wallet.key, *maker_dst_account.key)?;
        system_program_info = Some(next_account_info(account_info_iter)?);
    }
    taker_size = if has_metadata {
        let metadata_info = next_account_info(account_info_iter)?;
        let metadata = Metadata::from_account_info(metadata_info)?;
        let fees = metadata.data.seller_fee_basis_points;
        let total_fee = (fees as u64)
            .checked_mul(taker_size)
            .ok_or(UtilError::NumericalOverflow)?
            .checked_div(10000)
            .ok_or(UtilError::NumericalOverflow)?;
        let remaining_size = taker_size
            .checked_sub(total_fee)
            .ok_or(UtilError::NumericalOverflow)?;
        match metadata.data.creators {
            Some(creators) => {
                for creator in creators {
                    let pct = creator.share as u64;
                    let creator_fee = pct
                        .checked_mul(total_fee)
                        .ok_or(UtilError::NumericalOverflow)?
                        .checked_div(100)
                        .ok_or(UtilError::NumericalOverflow)?;
                    let current_creator_info = next_account_info(account_info_iter)?;
                    assert_keys_equal(creator.address, *current_creator_info.key)?;
                    if !is_native {
                        let current_creator_token_account_info =
                            next_account_info(account_info_iter)?;
                        assert_is_ata(
                            current_creator_token_account_info,
                            current_creator_info.key,
                            taker_src_mint.key,
                        )?;
                        if creator_fee > 0 {
                            invoke(
                                &spl_token::instruction::transfer(
                                    token_program_info.key,
                                    taker_src_account.key,
                                    current_creator_token_account_info.key,
                                    taker_wallet.key,
                                    &[],
                                    creator_fee,
                                )?,
                                &[
                                    taker_src_account.clone(),
                                    current_creator_token_account_info.clone(),
                                    taker_wallet.clone(),
                                    token_program_info.clone(),
                                ],
                            )?;
                        }
                    } else if creator_fee > 0 {
                        match system_program_info {
                            Some(sys_program_info) => {
                                invoke(
                                    &system_instruction::transfer(
                                        taker_src_account.key,
                                        current_creator_info.key,
                                        creator_fee,
                                    ),
                                    &[
                                        taker_src_account.clone(),
                                        current_creator_info.clone(),
                                        sys_program_info.clone(),
                                    ],
                                )?;
                            }
                            None => {
                                msg!("Invalid System Program Info");
                                return Err(ProgramError::IncorrectProgramId);
                            }
                        }
                    }
                }
                remaining_size
            }
            None => taker_size,
        }
    } else {
        taker_size
    };

    let maker_src_token_account: spl_token::state::Account =
        spl_token::state::Account::unpack(&maker_src_account.data.borrow())?;
    // Ensure that the delegated amount is exactly equal to the maker_size
    msg!(
        "Delegate {}",
        maker_src_token_account
            .delegate
            .unwrap_or(*maker_wallet.key)
    );
    msg!(
        "Delegated Amount {}",
        maker_src_token_account.delegated_amount
    );
    if maker_src_token_account.delegated_amount != maker_size {
        return Err(ProgramError::InvalidAccountData);
    }
    let seeds = &[
        b"stateless_offer",
        maker_wallet.key.as_ref(),
        maker_src_mint.key.as_ref(),
        taker_src_mint.key.as_ref(),
        &maker_size.to_le_bytes(),
        &taker_size.to_le_bytes(),
        &[bump_seed],
    ];
    let authority_key = Pubkey::create_program_address(seeds, program_id)?;
    assert_keys_equal(authority_key, *transfer_authority.key)?;
    // Ensure that authority is the delegate of this token account
    msg!("Authority key matches");
    if maker_src_token_account.delegate != COption::Some(authority_key) {
        return Err(ProgramError::InvalidAccountData);
    }
    msg!("Delegate matches");
    assert_keys_equal(spl_token::id(), *token_program_info.key)?;
    // Both of these transfers will fail if the `transfer_authority` is the delegate of these ATA's
    // One consideration is that the taker can get tricked in the case that the maker size is greater than
    // the token amount in the maker's ATA, but these stateless offers should just be invalidated in
    // the client.
    assert_is_ata(maker_src_account, maker_wallet.key, maker_src_mint.key)?;
    assert_is_ata(taker_dst_account, taker_wallet.key, maker_src_mint.key)?;
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
    if *taker_src_mint.key == spl_token::native_mint::id() {
        match system_program_info {
            Some(sys_program_info) => {
                assert_keys_equal(system_program::id(), *sys_program_info.key)?;
                invoke(
                    &system_instruction::transfer(
                        taker_src_account.key,
                        maker_dst_account.key,
                        taker_size,
                    ),
                    &[
                        taker_src_account.clone(),
                        maker_dst_account.clone(),
                        sys_program_info.clone(),
                    ],
                )?;
            }
            _ => return Err(ProgramError::InvalidAccountData),
        }
    } else {
        assert_is_ata(maker_dst_account, maker_wallet.key, taker_src_mint.key)?;
        assert_is_ata(taker_src_account, taker_wallet.key, taker_src_mint.key)?;
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
    }
    msg!("done tx from taker to maker {}", taker_size);
    msg!("done!");
    Ok(())
}
