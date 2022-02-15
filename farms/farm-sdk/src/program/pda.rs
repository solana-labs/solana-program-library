//! Common PDA functions

use {
    crate::{
        id::{main_router, main_router_admin},
        refdb,
        string::ArrayString64,
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, program, program_error::ProgramError,
        program_pack::Pack, pubkey::Pubkey, rent::Rent, system_instruction, sysvar, sysvar::Sysvar,
    },
};

/// Derives the RefDB storage address and the bump seed for the given string
pub fn find_refdb_pda(refdb_name: &str) -> (Pubkey, u8) {
    if refdb::REFDB_ONCHAIN_INIT {
        Pubkey::find_program_address(&[refdb_name.as_bytes()], &main_router::id())
    } else {
        (
            Pubkey::create_with_seed(&main_router_admin::id(), refdb_name, &main_router::id())
                .unwrap(),
            0,
        )
    }
}

/// Derives the target metadata object address for the given storage type and object name
pub fn find_target_pda(
    storage_type: refdb::StorageType,
    target_name: &ArrayString64,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[storage_type.to_string().as_bytes(), target_name.as_bytes()],
        &main_router::id(),
    )
}

/// Returns the target metadata object address for the given storage type, object name, and bump
pub fn find_target_pda_with_bump(
    storage_type: refdb::StorageType,
    target_name: &ArrayString64,
    bump: u8,
) -> Result<Pubkey, ProgramError> {
    Pubkey::create_program_address(
        &[
            storage_type.to_string().as_bytes(),
            target_name.as_bytes(),
            &[bump],
        ],
        &main_router::id(),
    )
    .map_err(|_| ProgramError::InvalidSeeds)
}

pub fn init_token_account<'a, 'b>(
    funding_account: &'a AccountInfo<'b>,
    target_account: &'a AccountInfo<'b>,
    mint_account: &'a AccountInfo<'b>,
    owner_account: &'a AccountInfo<'b>,
    rent_program: &'a AccountInfo<'b>,
    base_address: &Pubkey,
    seeds: &[&[u8]],
) -> ProgramResult {
    if !target_account.data_is_empty() {
        return Ok(());
    }

    init_system_account(
        funding_account,
        target_account,
        &spl_token::id(),
        base_address,
        seeds,
        spl_token::state::Account::get_packed_len(),
    )?;

    program::invoke(
        &spl_token::instruction::initialize_account(
            &spl_token::id(),
            target_account.key,
            mint_account.key,
            owner_account.key,
        )?,
        &[
            target_account.clone(),
            mint_account.clone(),
            owner_account.clone(),
            rent_program.clone(),
        ],
    )
}

pub fn init_associated_token_account<'a, 'b>(
    funding_account: &'a AccountInfo<'b>,
    wallet_account: &'a AccountInfo<'b>,
    target_account: &'a AccountInfo<'b>,
    mint_account: &'a AccountInfo<'b>,
    rent_program: &'a AccountInfo<'b>,
) -> ProgramResult {
    if !target_account.data_is_empty() {
        return Ok(());
    }

    program::invoke(
        &spl_associated_token_account::create_associated_token_account(
            funding_account.key,
            wallet_account.key,
            mint_account.key,
        ),
        &[
            funding_account.clone(),
            target_account.clone(),
            wallet_account.clone(),
            mint_account.clone(),
            rent_program.clone(),
        ],
    )
}

pub fn close_token_account<'a, 'b>(
    receiving_account: &'a AccountInfo<'b>,
    target_account: &'a AccountInfo<'b>,
    authority_account: &'a AccountInfo<'b>,
    base_address: &Pubkey,
    seeds: &[&[u8]],
) -> ProgramResult {
    if target_account.data_is_empty() {
        return Ok(());
    }
    let (_, bump) = Pubkey::find_program_address(seeds, base_address);

    program::invoke_signed(
        &spl_token::instruction::close_account(
            &spl_token::id(),
            target_account.key,
            receiving_account.key,
            authority_account.key,
            &[],
        )?,
        &[
            target_account.clone(),
            receiving_account.clone(),
            authority_account.clone(),
        ],
        &[&[seeds, &[&[bump]]].concat()],
    )?;
    Ok(())
}

pub fn transfer_tokens_with_seeds<'a, 'b>(
    source_account: &'a AccountInfo<'b>,
    destination_account: &'a AccountInfo<'b>,
    authority_account: &'a AccountInfo<'b>,
    seeds: &[&[&[u8]]],
    amount: u64,
) -> ProgramResult {
    if source_account.key == destination_account.key {
        return Err(ProgramError::InvalidArgument);
    }
    program::invoke_signed(
        &spl_token::instruction::transfer(
            &spl_token::id(),
            source_account.key,
            destination_account.key,
            authority_account.key,
            &[],
            amount,
        )?,
        &[
            source_account.clone(),
            destination_account.clone(),
            authority_account.clone(),
        ],
        seeds,
    )
}

pub fn transfer_tokens<'a, 'b>(
    source_account: &'a AccountInfo<'b>,
    destination_account: &'a AccountInfo<'b>,
    authority_account: &'a AccountInfo<'b>,
    base_address: &Pubkey,
    seeds: &[&[u8]],
    amount: u64,
) -> ProgramResult {
    let (_, bump) = Pubkey::find_program_address(seeds, base_address);

    transfer_tokens_with_seeds(
        source_account,
        destination_account,
        authority_account,
        &[&[seeds, &[&[bump]]].concat()],
        amount,
    )
}

pub fn init_system_account<'a, 'b>(
    funding_account: &'a AccountInfo<'b>,
    target_account: &'a AccountInfo<'b>,
    owner_key: &Pubkey,
    base_address: &Pubkey,
    seeds: &[&[u8]],
    data_size: usize,
) -> ProgramResult {
    if !target_account.data_is_empty() || target_account.try_lamports()? != 0 {
        return Ok(());
    }

    let (key, bump) = Pubkey::find_program_address(seeds, base_address);
    if target_account.key != &key {
        return Err(ProgramError::InvalidSeeds);
    }

    let min_balance = sysvar::rent::Rent::get()
        .unwrap()
        .minimum_balance(data_size);
    program::invoke_signed(
        &system_instruction::create_account(
            funding_account.key,
            target_account.key,
            min_balance,
            data_size as u64,
            owner_key,
        ),
        &[funding_account.clone(), target_account.clone()],
        &[&[seeds, &[&[bump]]].concat()],
    )
}

pub fn init_mint<'a, 'b>(
    funding_account: &'a AccountInfo<'b>,
    mint_account: &'a AccountInfo<'b>,
    owner_account: &'a AccountInfo<'b>,
    rent_program: &'a AccountInfo<'b>,
    base_address: &Pubkey,
    seeds: &[&[u8]],
    decimals: u8,
) -> ProgramResult {
    if !mint_account.data_is_empty() {
        return Ok(());
    }

    let acc_size = spl_token::state::Mint::get_packed_len();
    init_system_account(
        funding_account,
        mint_account,
        &spl_token::id(),
        base_address,
        seeds,
        acc_size,
    )?;

    program::invoke(
        &spl_token::instruction::initialize_mint(
            &spl_token::id(),
            mint_account.key,
            owner_account.key,
            Some(owner_account.key),
            decimals,
        )?,
        &[
            mint_account.clone(),
            owner_account.clone(),
            rent_program.clone(),
        ],
    )
}

pub fn mint_to_with_seeds<'a, 'b>(
    target_token_account: &'a AccountInfo<'b>,
    mint_account: &'a AccountInfo<'b>,
    mint_authority_account: &'a AccountInfo<'b>,
    seeds: &[&[&[u8]]],
    amount: u64,
) -> ProgramResult {
    solana_program::program::invoke_signed(
        &spl_token::instruction::mint_to(
            &spl_token::id(),
            mint_account.key,
            target_token_account.key,
            mint_authority_account.key,
            &[],
            amount,
        )?,
        &[
            mint_account.clone(),
            target_token_account.clone(),
            mint_authority_account.clone(),
        ],
        seeds,
    )?;
    Ok(())
}

pub fn mint_to<'a, 'b>(
    target_token_account: &'a AccountInfo<'b>,
    mint_account: &'a AccountInfo<'b>,
    mint_authority_account: &'a AccountInfo<'b>,
    base_address: &Pubkey,
    seeds: &[&[u8]],
    amount: u64,
) -> ProgramResult {
    let (_, bump) = Pubkey::find_program_address(seeds, base_address);

    mint_to_with_seeds(
        target_token_account,
        mint_account,
        mint_authority_account,
        &[&[seeds, &[&[bump]]].concat()],
        amount,
    )
}

pub fn check_pda_data_size<'a, 'b>(
    target_account: &'a AccountInfo<'b>,
    seeds: &[&[u8]],
    data_size: usize,
    fix: bool,
) -> ProgramResult {
    if fix && target_account.data_is_empty() {
        program::invoke_signed(
            &system_instruction::allocate(target_account.key, data_size as u64),
            &[target_account.clone()],
            &[seeds],
        )?;
    }
    if target_account.data_len() < data_size {
        Err(ProgramError::AccountDataTooSmall)
    } else {
        Ok(())
    }
}

pub fn check_pda_rent_exempt<'a, 'b>(
    signer_account: &'a AccountInfo<'b>,
    target_account: &'a AccountInfo<'b>,
    seeds: &[&[u8]],
    data_size: usize,
    fix: bool,
) -> ProgramResult {
    let rent = Rent::get()?;
    let cur_balance = target_account.try_lamports()?;
    let min_balance = rent.minimum_balance(data_size);
    if cur_balance < min_balance {
        let signer_balance = signer_account.try_lamports()?;
        let signer_min_balance = rent.minimum_balance(signer_account.data_len());
        if !fix
            || signer_balance <= signer_min_balance
            || min_balance.checked_sub(cur_balance).unwrap()
                > signer_balance.checked_sub(signer_min_balance).unwrap()
        {
            return Err(ProgramError::InsufficientFunds);
        }
        program::invoke_signed(
            &system_instruction::transfer(
                signer_account.key,
                target_account.key,
                min_balance.checked_sub(cur_balance).unwrap(),
            ),
            &[signer_account.clone(), target_account.clone()],
            &[seeds],
        )?;
        assert!(target_account.try_lamports()? >= min_balance);
    }
    Ok(())
}

pub fn check_pda_owner<'a, 'b>(
    program_id: &Pubkey,
    target_account: &'a AccountInfo<'b>,
    seeds: &[&[u8]],
    fix: bool,
) -> ProgramResult {
    if *target_account.owner != *program_id {
        if fix {
            program::invoke_signed(
                &system_instruction::assign(target_account.key, program_id),
                &[target_account.clone()],
                &[seeds],
            )?;
            assert!(*target_account.owner == *program_id);
        } else {
            return Err(ProgramError::IllegalOwner);
        }
    }
    Ok(())
}
