//! Common PDA functions

use {
    crate::program::account,
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, program, program_error::ProgramError,
        program_pack::Pack, pubkey::Pubkey, rent::Rent, system_instruction, sysvar, sysvar::Sysvar,
    },
};

pub fn init_token_account<'a, 'b>(
    funding_account: &'a AccountInfo<'b>,
    target_account: &'a AccountInfo<'b>,
    mint_account: &'a AccountInfo<'b>,
    owner_account: &'a AccountInfo<'b>,
    rent_program: &'a AccountInfo<'b>,
    base_address: &Pubkey,
    seeds: &[&[u8]],
) -> ProgramResult {
    if account::exists(target_account)? {
        if !account::check_token_account_owner(target_account, owner_account.key)? {
            return Err(ProgramError::IllegalOwner);
        }
        if target_account.data_len() != spl_token::state::Account::get_packed_len()
            || mint_account.key != &account::get_token_account_mint(target_account)?
        {
            return Err(ProgramError::InvalidAccountData);
        }
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
    if account::exists(target_account)? {
        if !account::check_token_account_owner(target_account, wallet_account.key)? {
            return Err(ProgramError::IllegalOwner);
        }
        if target_account.data_len() != spl_token::state::Account::get_packed_len()
            || mint_account.key != &account::get_token_account_mint(target_account)?
        {
            return Err(ProgramError::InvalidAccountData);
        }
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

pub fn close_token_account_with_seeds<'a, 'b>(
    receiving_account: &'a AccountInfo<'b>,
    target_account: &'a AccountInfo<'b>,
    authority_account: &'a AccountInfo<'b>,
    seeds: &[&[&[u8]]],
) -> ProgramResult {
    if !account::exists(target_account)? {
        return Ok(());
    }

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
        seeds,
    )
}

pub fn close_token_account<'a, 'b>(
    receiving_account: &'a AccountInfo<'b>,
    target_account: &'a AccountInfo<'b>,
    authority_account: &'a AccountInfo<'b>,
    base_address: &Pubkey,
    seeds: &[&[u8]],
) -> Result<u8, ProgramError> {
    let (_, bump) = Pubkey::find_program_address(seeds, base_address);

    close_token_account_with_seeds(
        receiving_account,
        target_account,
        authority_account,
        &[&[seeds, &[&[bump]]].concat()],
    )?;

    Ok(bump)
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
) -> Result<u8, ProgramError> {
    let (_, bump) = Pubkey::find_program_address(seeds, base_address);

    transfer_tokens_with_seeds(
        source_account,
        destination_account,
        authority_account,
        &[&[seeds, &[&[bump]]].concat()],
        amount,
    )?;

    Ok(bump)
}

pub fn init_system_account<'a, 'b>(
    funding_account: &'a AccountInfo<'b>,
    target_account: &'a AccountInfo<'b>,
    owner_key: &Pubkey,
    base_address: &Pubkey,
    seeds: &[&[u8]],
    data_size: usize,
) -> Result<u8, ProgramError> {
    if account::exists(target_account)? {
        if target_account.owner != owner_key {
            return Err(ProgramError::IllegalOwner);
        }
        if target_account.data_len() != data_size {
            return Err(ProgramError::InvalidAccountData);
        }
        return Ok(Pubkey::find_program_address(seeds, base_address).1);
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
    )?;

    Ok(bump)
}

pub fn init_mint<'a, 'b>(
    funding_account: &'a AccountInfo<'b>,
    mint_account: &'a AccountInfo<'b>,
    authority_account: &'a AccountInfo<'b>,
    rent_program: &'a AccountInfo<'b>,
    base_address: &Pubkey,
    seeds: &[&[u8]],
    decimals: u8,
) -> ProgramResult {
    if account::exists(mint_account)? {
        if !account::check_mint_authority(mint_account, Some(*authority_account.key))? {
            return Err(ProgramError::IllegalOwner);
        }
        if mint_account.data_len() != spl_token::state::Mint::get_packed_len() {
            return Err(ProgramError::InvalidAccountData);
        }
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
            authority_account.key,
            Some(authority_account.key),
            decimals,
        )?,
        &[
            mint_account.clone(),
            authority_account.clone(),
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
    )
}

pub fn mint_to<'a, 'b>(
    target_token_account: &'a AccountInfo<'b>,
    mint_account: &'a AccountInfo<'b>,
    mint_authority_account: &'a AccountInfo<'b>,
    base_address: &Pubkey,
    seeds: &[&[u8]],
    amount: u64,
) -> Result<u8, ProgramError> {
    let (_, bump) = Pubkey::find_program_address(seeds, base_address);

    mint_to_with_seeds(
        target_token_account,
        mint_account,
        mint_authority_account,
        &[&[seeds, &[&[bump]]].concat()],
        amount,
    )?;

    Ok(bump)
}

pub fn burn_tokens_with_seeds<'a, 'b>(
    from_token_account: &'a AccountInfo<'b>,
    mint_account: &'a AccountInfo<'b>,
    authority_account: &'a AccountInfo<'b>,
    seeds: &[&[&[u8]]],
    amount: u64,
) -> ProgramResult {
    solana_program::program::invoke_signed(
        &spl_token::instruction::burn(
            &spl_token::id(),
            from_token_account.key,
            mint_account.key,
            authority_account.key,
            &[],
            amount,
        )?,
        &[
            from_token_account.clone(),
            mint_account.clone(),
            authority_account.clone(),
        ],
        seeds,
    )
}

pub fn burn_tokens<'a, 'b>(
    from_token_account: &'a AccountInfo<'b>,
    mint_account: &'a AccountInfo<'b>,
    authority_account: &'a AccountInfo<'b>,
    base_address: &Pubkey,
    seeds: &[&[u8]],
    amount: u64,
) -> Result<u8, ProgramError> {
    let (_, bump) = Pubkey::find_program_address(seeds, base_address);

    burn_tokens_with_seeds(
        from_token_account,
        mint_account,
        authority_account,
        &[&[seeds, &[&[bump]]].concat()],
        amount,
    )?;

    Ok(bump)
}

pub fn approve_delegate_with_seeds<'a, 'b>(
    source_account: &'a AccountInfo<'b>,
    delegate_account: &'a AccountInfo<'b>,
    authority_account: &'a AccountInfo<'b>,
    seeds: &[&[&[u8]]],
    amount: u64,
) -> ProgramResult {
    solana_program::program::invoke_signed(
        &spl_token::instruction::approve(
            &spl_token::id(),
            source_account.key,
            delegate_account.key,
            authority_account.key,
            &[],
            amount,
        )?,
        &[
            source_account.clone(),
            delegate_account.clone(),
            authority_account.clone(),
        ],
        seeds,
    )
}

pub fn approve_delegate<'a, 'b>(
    source_account: &'a AccountInfo<'b>,
    delegate_account: &'a AccountInfo<'b>,
    authority_account: &'a AccountInfo<'b>,
    base_address: &Pubkey,
    seeds: &[&[u8]],
    amount: u64,
) -> Result<u8, ProgramError> {
    let (_, bump) = Pubkey::find_program_address(seeds, base_address);

    approve_delegate_with_seeds(
        source_account,
        delegate_account,
        authority_account,
        &[&[seeds, &[&[bump]]].concat()],
        amount,
    )?;

    Ok(bump)
}

pub fn revoke_delegate_with_seeds<'a, 'b>(
    source_account: &'a AccountInfo<'b>,
    authority_account: &'a AccountInfo<'b>,
    seeds: &[&[&[u8]]],
) -> ProgramResult {
    solana_program::program::invoke_signed(
        &spl_token::instruction::revoke(
            &spl_token::id(),
            source_account.key,
            authority_account.key,
            &[],
        )?,
        &[source_account.clone(), authority_account.clone()],
        seeds,
    )
}

pub fn revoke_delegate<'a, 'b>(
    source_account: &'a AccountInfo<'b>,
    authority_account: &'a AccountInfo<'b>,
    base_address: &Pubkey,
    seeds: &[&[u8]],
) -> Result<u8, ProgramError> {
    let (_, bump) = Pubkey::find_program_address(seeds, base_address);

    revoke_delegate_with_seeds(
        source_account,
        authority_account,
        &[&[seeds, &[&[bump]]].concat()],
    )?;

    Ok(bump)
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
