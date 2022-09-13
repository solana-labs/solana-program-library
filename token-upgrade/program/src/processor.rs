//! Program state processor

use {
    crate::{
        collect_token_upgrade_authority_signer_seeds, error::TokenUpgradeError,
        get_token_upgrade_authority_address_and_bump_seed, instruction::TokenUpgradeInstruction,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program::{invoke, invoke_signed},
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    spl_token_2022::{
        extension::StateWithExtensions,
        instruction::decode_instruction_type,
        state::{Account, Mint},
    },
};

fn check_owner(account_info: &AccountInfo, expected_owner: &Pubkey) -> ProgramResult {
    if account_info.owner != expected_owner {
        Err(ProgramError::IllegalOwner)
    } else {
        Ok(())
    }
}

fn burn_original_tokens<'a>(
    original_token_program: AccountInfo<'a>,
    source: AccountInfo<'a>,
    mint: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    multisig_signers: &[AccountInfo<'a>],
    amount: u64,
    decimals: u8,
) -> ProgramResult {
    let multisig_pubkeys = multisig_signers.iter().map(|s| s.key).collect::<Vec<_>>();
    let ix = spl_token_2022::instruction::burn_checked(
        original_token_program.key,
        source.key,
        mint.key,
        authority.key,
        &multisig_pubkeys,
        amount,
        decimals,
    )?;
    let mut account_infos = vec![source, mint, authority];
    account_infos.extend_from_slice(multisig_signers);
    invoke(&ix, &account_infos)
}

#[allow(clippy::too_many_arguments)]
fn transfer_new_tokens<'a>(
    new_token_program: AccountInfo<'a>,
    source: AccountInfo<'a>,
    mint: AccountInfo<'a>,
    destination: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    authority_seeds: &[&[u8]],
    amount: u64,
    decimals: u8,
) -> Result<(), ProgramError> {
    let ix = spl_token_2022::instruction::transfer_checked(
        new_token_program.key,
        source.key,
        mint.key,
        destination.key,
        authority.key,
        &[],
        amount,
        decimals,
    )?;
    invoke_signed(
        &ix,
        &[source, mint, destination, authority],
        &[authority_seeds],
    )
}

fn process_exchange(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let original_account_info = next_account_info(account_info_iter)?;
    let original_mint_info = next_account_info(account_info_iter)?;
    let new_escrow_info = next_account_info(account_info_iter)?;
    let new_account_info = next_account_info(account_info_iter)?;
    let new_mint_info = next_account_info(account_info_iter)?;
    let new_transfer_authority_info = next_account_info(account_info_iter)?;
    let original_token_program = next_account_info(account_info_iter)?;
    let new_token_program = next_account_info(account_info_iter)?;
    let original_transfer_authority_info = next_account_info(account_info_iter)?;

    // owner checks
    check_owner(original_account_info, original_token_program.key)?;
    check_owner(original_mint_info, original_token_program.key)?;
    check_owner(new_escrow_info, new_token_program.key)?;
    check_owner(new_account_info, new_token_program.key)?;
    check_owner(new_mint_info, new_token_program.key)?;

    // PDA derivation check
    let (expected_escrow_authority, bump_seed) = get_token_upgrade_authority_address_and_bump_seed(
        original_mint_info.key,
        new_mint_info.key,
        program_id,
    );
    let bump_seed = [bump_seed];
    let authority_seeds = collect_token_upgrade_authority_signer_seeds(
        original_mint_info.key,
        new_mint_info.key,
        &bump_seed,
    );
    if expected_escrow_authority != *new_transfer_authority_info.key {
        msg!(
            "Expected escrow authority {}, received {}",
            &expected_escrow_authority,
            new_transfer_authority_info.key
        );
        return Err(TokenUpgradeError::InvalidOwner.into());
    }

    // pull out these values in a block to drop all data before performing CPIs
    let (token_amount, decimals) = {
        // check mints are actually mints
        let original_mint_data = original_mint_info.try_borrow_data()?;
        let original_mint = StateWithExtensions::<Mint>::unpack(&original_mint_data)?;
        let new_mint_data = new_mint_info.try_borrow_data()?;
        let new_mint = StateWithExtensions::<Mint>::unpack(&new_mint_data)?;

        // check accounts are actually accounts
        let original_account_data = original_account_info.try_borrow_data()?;
        let original_account = StateWithExtensions::<Account>::unpack(&original_account_data)?;
        let new_escrow_data = new_escrow_info.try_borrow_data()?;
        let new_escrow = StateWithExtensions::<Account>::unpack(&new_escrow_data)?;
        let new_account_data = new_account_info.try_borrow_data()?;
        let _ = StateWithExtensions::<Account>::unpack(&new_account_data)?;

        let token_amount = original_account.base.amount;
        if new_escrow.base.amount < token_amount {
            msg!(
                "Escrow only has {} tokens, needs at least {}",
                new_escrow.base.amount,
                token_amount
            );
            return Err(ProgramError::InsufficientFunds);
        }
        if original_mint.base.decimals != new_mint.base.decimals {
            msg!(
                "Original and new token mint decimals mismatch: original has {} decimals, and new has {}",
                original_mint.base.decimals,
                new_mint.base.decimals,
            );
            return Err(TokenUpgradeError::DecimalsMismatch.into());
        }

        (original_account.base.amount, original_mint.base.decimals)
    };

    burn_original_tokens(
        original_token_program.clone(),
        original_account_info.clone(),
        original_mint_info.clone(),
        original_transfer_authority_info.clone(),
        account_info_iter.as_slice(),
        token_amount,
        decimals,
    )?;

    transfer_new_tokens(
        new_token_program.clone(),
        new_escrow_info.clone(),
        new_mint_info.clone(),
        new_account_info.clone(),
        new_transfer_authority_info.clone(),
        &authority_seeds,
        token_amount,
        decimals,
    )?;

    Ok(())
}

/// Instruction processor
pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
    match decode_instruction_type(input)? {
        TokenUpgradeInstruction::Exchange => process_exchange(program_id, accounts),
    }
}
