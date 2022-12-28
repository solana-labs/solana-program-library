use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    program::{invoke, invoke_signed},
    pubkey::Pubkey,
};

pub(crate) fn initialize_mint<'a, 'b>(
    freeze_authority: &Pubkey,
    mint_authority: &Pubkey,
    mint: &'a AccountInfo<'b>,
    token_program: &'a AccountInfo<'b>,
    decimals: u8,
) -> ProgramResult {
    invoke(
        &spl_token::instruction::initialize_mint2(
            token_program.key,
            mint.key,
            mint_authority,
            Some(freeze_authority),
            decimals,
        )?,
        &[token_program.clone(), mint.clone()],
    )
}

pub(crate) fn thaw<'a, 'b>(
    freeze_authority: &'a AccountInfo<'b>,
    mint: &'a AccountInfo<'b>,
    target: &'a AccountInfo<'b>,
    token_program: &'a AccountInfo<'b>,
    seeds: &[Vec<u8>],
) -> ProgramResult {
    invoke_signed(
        &spl_token::instruction::thaw_account(
            token_program.key,
            target.key,
            mint.key,
            freeze_authority.key,
            &[],
        )?,
        &[
            token_program.clone(),
            mint.clone(),
            freeze_authority.clone(),
            target.clone(),
        ],
        &[&seeds.iter().map(|s| s.as_slice()).collect::<Vec<&[u8]>>()],
    )
}

pub(crate) fn freeze<'a, 'b>(
    freeze_authority: &'a AccountInfo<'b>,
    mint: &'a AccountInfo<'b>,
    target: &'a AccountInfo<'b>,
    token_program: &'a AccountInfo<'b>,
    seeds: &[Vec<u8>],
) -> ProgramResult {
    invoke_signed(
        &spl_token::instruction::freeze_account(
            token_program.key,
            target.key,
            mint.key,
            freeze_authority.key,
            &[],
        )?,
        &[
            token_program.clone(),
            mint.clone(),
            freeze_authority.clone(),
            target.clone(),
        ],
        &[&seeds.iter().map(|s| s.as_slice()).collect::<Vec<&[u8]>>()],
    )
}

pub(crate) fn transfer<'a, 'b>(
    src: &'a AccountInfo<'b>,
    dst: &'a AccountInfo<'b>,
    owner: &'a AccountInfo<'b>,
    token_program: &'a AccountInfo<'b>,
    amount: u64,
) -> ProgramResult {
    invoke(
        &spl_token::instruction::transfer(
            token_program.key,
            src.key,
            dst.key,
            owner.key,
            &[],
            amount,
        )?,
        &[
            token_program.clone(),
            src.clone(),
            dst.clone(),
            owner.clone(),
        ],
    )
}

pub(crate) fn mint_to<'a, 'b>(
    mint: &'a AccountInfo<'b>,
    account: &'a AccountInfo<'b>,
    owner: &'a AccountInfo<'b>,
    token_program: &'a AccountInfo<'b>,
    amount: u64,
    seeds: &[Vec<u8>],
) -> ProgramResult {
    invoke_signed(
        &spl_token::instruction::mint_to(
            token_program.key,
            mint.key,
            account.key,
            owner.key,
            &[],
            amount,
        )?,
        &[
            token_program.clone(),
            mint.clone(),
            account.clone(),
            owner.clone(),
        ],
        &[&seeds.iter().map(|s| s.as_slice()).collect::<Vec<&[u8]>>()],
    )
}

pub(crate) fn burn<'a, 'b>(
    mint: &'a AccountInfo<'b>,
    account: &'a AccountInfo<'b>,
    owner: &'a AccountInfo<'b>,
    token_program: &'a AccountInfo<'b>,
    amount: u64,
) -> ProgramResult {
    invoke(
        &spl_token::instruction::burn(
            token_program.key,
            account.key,
            mint.key,
            owner.key,
            &[],
            amount,
        )?,
        &[
            token_program.clone(),
            mint.clone(),
            account.clone(),
            owner.clone(),
        ],
    )
}

pub(crate) fn approve<'a, 'b>(
    account: &'a AccountInfo<'b>,
    owner: &'a AccountInfo<'b>,
    delegate: &'a AccountInfo<'b>,
    token_program: &'a AccountInfo<'b>,
    amount: u64,
) -> ProgramResult {
    invoke(
        &spl_token::instruction::approve(
            token_program.key,
            account.key,
            delegate.key,
            owner.key,
            &[],
            amount,
        )?,
        &[
            token_program.clone(),
            account.clone(),
            delegate.clone(),
            owner.clone(),
        ],
    )
}

pub(crate) fn revoke<'a, 'b>(
    account: &'a AccountInfo<'b>,
    owner: &'a AccountInfo<'b>,
    token_program: &'a AccountInfo<'b>,
) -> ProgramResult {
    invoke(
        &spl_token::instruction::revoke(token_program.key, account.key, owner.key, &[])?,
        &[token_program.clone(), account.clone(), owner.clone()],
    )
}

pub(crate) fn close<'a, 'b>(
    account: &'a AccountInfo<'b>,
    destination: &'a AccountInfo<'b>,
    owner: &'a AccountInfo<'b>,
    token_program: &'a AccountInfo<'b>,
) -> ProgramResult {
    invoke(
        &spl_token::instruction::close_account(
            token_program.key,
            account.key,
            destination.key,
            owner.key,
            &[],
        )?,
        &[
            token_program.clone(),
            destination.clone(),
            account.clone(),
            owner.clone(),
        ],
    )
}
