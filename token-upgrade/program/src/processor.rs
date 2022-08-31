//! Program state processor

use {
    crate::{
        get_token_upgrade_authority_address_and_bump_seed,
        get_token_upgrade_authority_signer_seeds, instruction::TokenUpgradeInstruction,
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

fn token_burn<'a>(
    token_program: AccountInfo<'a>,
    source: AccountInfo<'a>,
    mint: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    multisig_signers: &[AccountInfo<'a>],
    amount: u64,
    decimals: u8,
) -> ProgramResult {
    let multisig_pubkeys = multisig_signers.iter().map(|s| s.key).collect::<Vec<_>>();
    let ix = spl_token_2022::instruction::burn_checked(
        token_program.key,
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
fn token_transfer<'a>(
    token_program: AccountInfo<'a>,
    source: AccountInfo<'a>,
    mint: AccountInfo<'a>,
    destination: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    authority_seeds: &[&[u8]],
    amount: u64,
    decimals: u8,
) -> Result<(), ProgramError> {
    let ix = spl_token_2022::instruction::transfer_checked(
        token_program.key,
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
        &[source, mint, destination, authority, token_program],
        &[authority_seeds],
    )
}

fn process_exchange(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let source_account_info = next_account_info(account_info_iter)?;
    let source_mint_info = next_account_info(account_info_iter)?;
    let destination_bag_info = next_account_info(account_info_iter)?;
    let destination_account_info = next_account_info(account_info_iter)?;
    let destination_mint_info = next_account_info(account_info_iter)?;
    let destination_transfer_authority_info = next_account_info(account_info_iter)?;
    let source_token_program = next_account_info(account_info_iter)?;
    let destination_token_program = next_account_info(account_info_iter)?;
    let source_transfer_authority_info = next_account_info(account_info_iter)?;

    // owner checks
    check_owner(source_account_info, source_token_program.key)?;
    check_owner(source_mint_info, source_token_program.key)?;
    check_owner(destination_bag_info, destination_token_program.key)?;
    check_owner(destination_account_info, destination_token_program.key)?;
    check_owner(destination_mint_info, destination_token_program.key)?;

    // PDA derivation check
    let (expected_escrow_authority, bump_seed) = get_token_upgrade_authority_address_and_bump_seed(
        source_mint_info.key,
        destination_mint_info.key,
        program_id,
    );
    let bump_seed = [bump_seed];
    let authority_seeds = get_token_upgrade_authority_signer_seeds(
        source_mint_info.key,
        destination_mint_info.key,
        &bump_seed,
    );
    if expected_escrow_authority != *destination_transfer_authority_info.key {
        msg!(
            "Expected escrow authority {}, received {}",
            &expected_escrow_authority,
            destination_transfer_authority_info.key
        );
        return Err(ProgramError::InvalidSeeds);
    }

    // pull out these values in a block to drop all data before performing CPIs
    let (token_amount, source_decimals, destination_decimals) = {
        // check mints are actually mints
        let source_mint_data = source_mint_info.try_borrow_data()?;
        let source_mint = StateWithExtensions::<Mint>::unpack(&source_mint_data)?;
        let destination_mint_data = destination_mint_info.try_borrow_data()?;
        let destination_mint = StateWithExtensions::<Mint>::unpack(&destination_mint_data)?;

        // check accounts are actually accounts
        let source_account_data = source_account_info.try_borrow_data()?;
        let source_account = StateWithExtensions::<Account>::unpack(&source_account_data)?;
        let destination_bag_data = destination_bag_info.try_borrow_data()?;
        let destination_bag = StateWithExtensions::<Account>::unpack(&destination_bag_data)?;
        let destination_account_data = destination_account_info.try_borrow_data()?;
        let _ = StateWithExtensions::<Account>::unpack(&destination_account_data)?;

        let token_amount = source_account.base.amount;
        if destination_bag.base.amount < token_amount {
            msg!(
                "Bag only has {} tokens, needs at least {}",
                destination_bag.base.amount,
                token_amount
            );
            return Err(ProgramError::InsufficientFunds);
        }

        (
            source_account.base.amount,
            source_mint.base.decimals,
            destination_mint.base.decimals,
        )
    };

    token_burn(
        source_token_program.clone(),
        source_account_info.clone(),
        source_mint_info.clone(),
        source_transfer_authority_info.clone(),
        account_info_iter.as_slice(),
        token_amount,
        source_decimals,
    )?;

    token_transfer(
        destination_token_program.clone(),
        destination_bag_info.clone(),
        destination_mint_info.clone(),
        destination_account_info.clone(),
        destination_transfer_authority_info.clone(),
        &authority_seeds,
        token_amount,
        destination_decimals,
    )?;

    Ok(())
}

/// Instruction processor
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    match decode_instruction_type(input)? {
        TokenUpgradeInstruction::Exchange => process_exchange(program_id, accounts),
    }
}
