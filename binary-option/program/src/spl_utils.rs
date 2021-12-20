use {
    solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        msg,
        program::{invoke, invoke_signed},
        pubkey::Pubkey,
    },
    spl_token::instruction::{
        approve_checked, burn, initialize_account, initialize_mint, mint_to, set_authority,
        transfer, AuthorityType,
    },
};

pub fn spl_initialize<'a>(
    token_program: &AccountInfo<'a>,
    new_account: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    rent: &AccountInfo<'a>,
) -> ProgramResult {
    let ix = initialize_account(token_program.key, new_account.key, mint.key, authority.key)?;
    invoke(
        &ix,
        &[
            new_account.clone(),
            mint.clone(),
            authority.clone(),
            rent.clone(),
            token_program.clone(),
        ],
    )?;
    Ok(())
}

pub fn spl_mint_initialize<'a>(
    token_program: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    mint_authority: &AccountInfo<'a>,
    freeze_authority: &AccountInfo<'a>,
    rent_info: &AccountInfo<'a>,
    decimals: u8,
) -> ProgramResult {
    let ix = initialize_mint(
        token_program.key,
        mint.key,
        mint_authority.key,
        Some(freeze_authority.key),
        decimals,
    )?;
    invoke(
        &ix,
        &[mint.clone(), rent_info.clone(), token_program.clone()],
    )?;
    Ok(())
}

pub fn spl_approve<'a>(
    token_program: &AccountInfo<'a>,
    source_account: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    delegate: &AccountInfo<'a>,
    owner: &AccountInfo<'a>,
    amount: u64,
    decimals: u8,
) -> ProgramResult {
    let ix = approve_checked(
        token_program.key,
        source_account.key,
        mint.key,
        delegate.key,
        owner.key,
        &[],
        amount,
        decimals,
    )?;
    invoke(
        &ix,
        &[
            source_account.clone(),
            mint.clone(),
            delegate.clone(),
            owner.clone(),
            token_program.clone(),
        ],
    )?;
    Ok(())
}

pub fn spl_burn<'a>(
    token_program: &AccountInfo<'a>,
    burn_account: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    amount: u64,
) -> ProgramResult {
    if amount > 0 {
        let ix = burn(
            token_program.key,
            burn_account.key,
            mint.key,
            authority.key,
            &[],
            amount,
        )?;
        invoke(
            &ix,
            &[
                burn_account.clone(),
                mint.clone(),
                authority.clone(),
                token_program.clone(),
            ],
        )?;
    }
    Ok(())
}

pub fn spl_burn_signed<'a>(
    token_program: &AccountInfo<'a>,
    burn_account: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    amount: u64,
    signers: &[&[u8]],
) -> ProgramResult {
    msg!("Burn Signed");
    if amount > 0 {
        let ix = burn(
            token_program.key,
            burn_account.key,
            mint.key,
            authority.key,
            &[],
            amount,
        )?;
        invoke_signed(
            &ix,
            &[
                burn_account.clone(),
                mint.clone(),
                authority.clone(),
                token_program.clone(),
            ],
            &[signers],
        )?;
    }
    Ok(())
}

pub fn spl_mint_to<'a>(
    token_program: &AccountInfo<'a>,
    dest_account: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    amount: u64,
    signers: &[&[u8]],
) -> ProgramResult {
    let ix = mint_to(
        token_program.key,
        mint.key,
        dest_account.key,
        authority.key,
        &[],
        amount,
    )?;
    invoke_signed(
        &ix,
        &[
            mint.clone(),
            dest_account.clone(),
            authority.clone(),
            token_program.clone(),
        ],
        &[signers],
    )?;
    Ok(())
}

pub fn spl_token_transfer<'a>(
    token_program: &AccountInfo<'a>,
    source: &AccountInfo<'a>,
    destination: &AccountInfo<'a>,
    owner: &AccountInfo<'a>,
    amount: u64,
) -> ProgramResult {
    if amount > 0 {
        let ix = transfer(
            token_program.key,
            source.key,
            destination.key,
            owner.key,
            &[],
            amount,
        )?;
        invoke(
            &ix,
            &[
                source.clone(),
                destination.clone(),
                owner.clone(),
                token_program.clone(),
            ],
        )?;
    }
    Ok(())
}

pub fn spl_token_transfer_signed<'a>(
    token_program: &AccountInfo<'a>,
    source: &AccountInfo<'a>,
    destination: &AccountInfo<'a>,
    pda_account: &AccountInfo<'a>,
    amount: u64,
    signers: &[&[u8]],
) -> ProgramResult {
    if amount > 0 {
        let ix = transfer(
            token_program.key,
            source.key,
            destination.key,
            pda_account.key,
            &[],
            amount,
        )?;
        invoke_signed(
            &ix,
            &[
                source.clone(),
                destination.clone(),
                pda_account.clone(),
                token_program.clone(),
            ],
            &[signers],
        )?;
    }
    Ok(())
}

pub fn spl_set_authority<'a>(
    token_program: &AccountInfo<'a>,
    account_to_transfer_ownership: &AccountInfo<'a>,
    new_authority: Option<Pubkey>,
    authority_type: AuthorityType,
    owner: &AccountInfo<'a>,
) -> ProgramResult {
    let ix = set_authority(
        token_program.key,
        account_to_transfer_ownership.key,
        new_authority.as_ref(),
        authority_type,
        owner.key,
        &[],
    )?;
    invoke(
        &ix,
        &[
            account_to_transfer_ownership.clone(),
            owner.clone(),
            token_program.clone(),
        ],
    )?;
    Ok(())
}
