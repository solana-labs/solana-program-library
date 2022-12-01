//! Program state processor
use crate::error::MultiDelegateError;
use crate::{
    state::MultiDelegate,
    instruction::MultiDelegateInstruction,
    tools::account::create_pda_account,
    *,
};
use borsh::{BorshSerialize, BorshDeserialize};
use solana_program::borsh::try_from_slice_unchecked;
use solana_program::program_pack::Pack;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar, system_program,
};
use spl_token::instruction::transfer;
use spl_token::state::Account;

/// Instruction processor
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = MultiDelegateInstruction::try_from_slice(input)
            .map_err(|_| ProgramError::InvalidInstructionData)?;

    msg!("{:?}", instruction);

    match instruction {
        MultiDelegateInstruction::Create => {
            process_create(program_id, accounts)
        }
        MultiDelegateInstruction::Approve { amount} => {
            process_approve(program_id, accounts, amount)
        }
        MultiDelegateInstruction::Revoke => {
            process_revoke(program_id, accounts)
        }
        MultiDelegateInstruction::Transfer { amount } => {
            process_transfer(program_id, accounts, amount)
        }
        MultiDelegateInstruction::Close => {
            panic!("TODO")
        }
    }
}

fn process_create(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let funder_info = next_account_info(account_info_iter)?;
    let token_account_owner_info = next_account_info(account_info_iter)?;
    let token_account_info = next_account_info(account_info_iter)?;
    let multi_delegate_account_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;
    let spl_token_program_info = next_account_info(account_info_iter)?;
    let spl_token_program_id = spl_token_program_info.key;

    let (multi_delegate_address, bump_seed) = get_multi_delegate_address_and_bump_seed_internal(
        token_account_owner_info.key,
        token_account_info.key,
        program_id,
    );
    if multi_delegate_address != *multi_delegate_account_info.key {
        msg!("Error: Multi delegate address does not match seed derivation");
        return Err(ProgramError::InvalidSeeds);
    }

    if *token_account_info.owner != system_program::id() {
        return Err(ProgramError::IllegalOwner);
    }

    let rent = Rent::get()?;

    let multi_delegate_signer_seeds: &[&[_]] = &[
        &token_account_owner_info.key.to_bytes(),
        &token_account_info.key.to_bytes(),
        &[bump_seed],
    ];

    create_pda_account(
        funder_info,
        &rent,
        MultiDelegate::DEFAULT_LEN,
        program_id,
        system_program_info,
        multi_delegate_account_info,
        multi_delegate_signer_seeds,
    )?;

    // Only the program can allocate the valid PDA so there is nothing in particular to write to the account on creation
    Ok(())
}

fn process_approve(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let token_account_owner_info = next_account_info(account_info_iter)?;
    let token_account_info = next_account_info(account_info_iter)?;
    let multi_delegate_account_info = next_account_info(account_info_iter)?;
    let delegate_account_info = next_account_info(account_info_iter)?;
    let spl_token_program_info = next_account_info(account_info_iter)?;
    let spl_token_program_id = spl_token_program_info.key;

    if !token_account_owner_info.is_signer {
        msg!("Token account owner must sign");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let token_account = Account::unpack(&token_account_info.data.borrow())?;
    if token_account_owner_info.key != &token_account.owner {
        return Err(MultiDelegateError::TokenAccountNotOwnedByProvidedOwner.into());
    }

    let multi_delegate_address = get_multi_delegate_address_and_bump_seed_internal(
        token_account_owner_info.key,
        token_account_info.key,
        program_id,
    ).0;
    if multi_delegate_address != *multi_delegate_account_info.key {
        msg!("Error: Multi delegate address does not match seed derivation");
        return Err(ProgramError::InvalidSeeds);
    }

    let mut multi_delegate: MultiDelegate = try_from_slice_unchecked(&multi_delegate_account_info.data.borrow()).unwrap();
    multi_delegate.approve(delegate_account_info.key, amount);

    multi_delegate.serialize(&mut *multi_delegate_account_info.data.borrow_mut())?;

    Ok(())
}

fn process_revoke(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let token_account_owner_info = next_account_info(account_info_iter)?;
    let token_account_info = next_account_info(account_info_iter)?;
    let multi_delegate_account_info = next_account_info(account_info_iter)?;
    let delegate_account_info = next_account_info(account_info_iter)?;
    let spl_token_program_info = next_account_info(account_info_iter)?;
    let spl_token_program_id = spl_token_program_info.key;

    if !token_account_owner_info.is_signer {
        msg!("Token account owner must sign");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let token_account = Account::unpack(&token_account_info.data.borrow())?;
    if token_account_owner_info.key != &token_account.owner {
        return Err(MultiDelegateError::TokenAccountNotOwnedByProvidedOwner.into());
    }

    let multi_delegate_address = get_multi_delegate_address_and_bump_seed_internal(
        token_account_owner_info.key,
        token_account_info.key,
        program_id,
    ).0;
    if multi_delegate_address != *multi_delegate_account_info.key {
        msg!("Error: Multi delegate address does not match seed derivation");
        return Err(ProgramError::InvalidSeeds);
    }

    let mut multi_delegate: MultiDelegate = try_from_slice_unchecked(&multi_delegate_account_info.data.borrow()).unwrap();
    multi_delegate.revoke(delegate_account_info.key);

    multi_delegate.serialize(&mut *multi_delegate_account_info.data.borrow_mut())?;

    Ok(())
}

fn process_transfer(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let multi_delegate_account_info = next_account_info(account_info_iter)?;
    let delegate_account_info = next_account_info(account_info_iter)?;
    let spl_token_program_info = next_account_info(account_info_iter)?;
    let spl_token_program_id = spl_token_program_info.key;
    let source_account_info = next_account_info(account_info_iter)?;
    let destination_account_info = next_account_info(account_info_iter)?;

    let source_token_account = Account::unpack(&source_account_info.data.borrow())?;

    let (multi_delegate_address, bump_seed) = get_multi_delegate_address_and_bump_seed_internal(
        &source_token_account.owner,
        source_account_info.key,
        program_id,
    );
    if multi_delegate_address != *multi_delegate_account_info.key {
        msg!("Error: Multi delegate address does not match seed derivation");
        return Err(ProgramError::InvalidSeeds);
    }

    let mut multi_delegate: MultiDelegate = try_from_slice_unchecked(&multi_delegate_account_info.data.borrow()).unwrap();
    multi_delegate.transfer_with_delegate(delegate_account_info.key, amount)?;

    let multi_delegate_signer_seeds: &[&[_]] = &[
        &source_token_account.owner.to_bytes(),
        &source_account_info.key.to_bytes(),
        &[bump_seed],
    ];

    let ix = transfer(
        spl_token_program_info.key,
        source_account_info.key,
        destination_account_info.key,
        multi_delegate_account_info.key,
        &[],
        amount,
    )?;
    invoke_signed(
        &ix,
        &[
            source_account_info.clone(),
            destination_account_info.clone(),
            multi_delegate_account_info.clone(),
            spl_token_program_info.clone(),
        ],
        &[multi_delegate_signer_seeds],
    )?;

    multi_delegate.serialize(&mut *multi_delegate_account_info.data.borrow_mut())?;

    Ok(())
}
