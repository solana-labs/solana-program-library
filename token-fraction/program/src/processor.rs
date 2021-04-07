use spl_token::state::Account;

use {
    crate::{
        error::FractionError,
        instruction::FractionInstruction,
        state::{
            FractionalizedTokenPool, FractionalizedTokenRegistry, PricingLookupType, MAX_POOL_SIZE,
            MAX_TOKEN_REGISTRY_SIZE, POOL_KEY, PREFIX, REGISTRY_KEY,
        },
        utils::{
            assert_inactive, assert_initialized, create_or_allocate_account_raw,
            spl_token_transfer, TokenTransferParams,
        },
    },
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        borsh::try_from_slice_unchecked,
        entrypoint::ProgramResult,
        msg,
        pubkey::Pubkey,
    },
    spl_token::state::Mint,
};

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = FractionInstruction::try_from_slice(input)?;
    match instruction {
        FractionInstruction::InitFractionalizedTokenPool(args) => {
            msg!("Instruction: Init Fractionalized Token Pool");
            process_init_fractionalized_token_pool(
                program_id,
                accounts,
                args.allow_share_redemption,
                args.pricing_lookup_type,
            )
        }
        FractionInstruction::AddTokenToInactivatedFractionalizedTokenPool(args) => {
            msg!("Instruction: Init Fractionalized Token Pool");
            process_add_token_to_inactivated_fractionalized_token_pool(
                program_id,
                accounts,
                args.amount,
            )
        }
    }
}

pub fn process_activate_fractionalized_token_pool(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    allow_share_redemption: bool,
    pricing_lookup_type: PricingLookupType,
    number_of_shares: u64,
) -> ProgramResult {
    Ok(())
}

pub fn process_add_token_to_inactivated_fractionalized_token_pool(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let registry_account_info = next_account_info(account_info_iter)?;
    let token_account_info = next_account_info(account_info_iter)?;
    let vault_info = next_account_info(account_info_iter)?;
    let fractionalized_token_pool_info = next_account_info(account_info_iter)?;
    let payer_info = next_account_info(account_info_iter)?;
    let transfer_authority_info = next_account_info(account_info_iter)?;

    let token_program_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;
    let system_account_info = next_account_info(account_info_iter)?;

    let token_account: Account = assert_initialized(token_account_info)?;
    let vault: Account = assert_initialized(vault_info)?;
    let mut fractionalized_token_pool: FractionalizedTokenPool =
        try_from_slice_unchecked(&fractionalized_token_pool_info.data.borrow_mut())?;
    assert_inactive(&fractionalized_token_pool)?;

    if token_account.amount == 0 {
        return Err(FractionError::TokenAccountContainsNoTokens.into());
    }

    if token_account.amount < amount {
        return Err(FractionError::TokenAccountAmountLessThanAmountSpecified.into());
    }

    if vault.amount > 0 {
        return Err(FractionError::VaultAccountIsNotEmpty.into());
    }

    if vault.owner != *program_id {
        return Err(FractionError::VaultAccountIsNotOwnedByProgram.into());
    }

    let seeds = &[
        PREFIX.as_bytes(),
        fractionalized_token_pool_info.key.as_ref(),
        token_account.mint.as_ref(),
    ];
    let (registry_key, bump_seed) = Pubkey::find_program_address(seeds, program_id);

    if registry_key != *registry_account_info.key {
        return Err(FractionError::RegistryAccountAddressInvalid.into());
    }
    let authority_signer_seeds = &[
        PREFIX.as_bytes(),
        fractionalized_token_pool_info.key.as_ref(),
        token_account.mint.as_ref(),
        &[bump_seed],
    ];
    create_or_allocate_account_raw(
        *program_id,
        registry_account_info,
        rent_info,
        system_account_info,
        payer_info,
        MAX_TOKEN_REGISTRY_SIZE,
        authority_signer_seeds,
    )?;

    fractionalized_token_pool.token_type_count =
        match fractionalized_token_pool.token_type_count.checked_add(1) {
            Some(val) => val,
            None => return Err(FractionError::NumericalOverflowError.into()),
        };
    fractionalized_token_pool.serialize(&mut *fractionalized_token_pool_info.data.borrow_mut())?;

    let mut registry: FractionalizedTokenRegistry =
        try_from_slice_unchecked(&registry_account_info.data.borrow_mut())?;
    registry.key = REGISTRY_KEY;
    registry.fractionalized_token_pool = *fractionalized_token_pool_info.key;
    registry.token_mint = token_account.mint;
    registry.vault = *vault_info.key;

    registry.serialize(&mut *registry_account_info.data.borrow_mut())?;

    spl_token_transfer(TokenTransferParams {
        source: token_account_info.clone(),
        destination: vault_info.clone(),
        amount: amount,
        authority: transfer_authority_info.clone(),
        authority_signer_seeds: authority_signer_seeds,
        token_program: token_program_info.clone(),
    })?;

    Ok(())
}

pub fn process_init_fractionalized_token_pool(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    allow_share_redemption: bool,
    pricing_lookup_type: PricingLookupType,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let fraction_mint_info = next_account_info(account_info_iter)?;
    let treasury_info = next_account_info(account_info_iter)?;
    let fractionalized_token_pool_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let pricing_lookup_address = next_account_info(account_info_iter)?;

    let fraction_mint: Mint = assert_initialized(fraction_mint_info)?;
    let treasury: Account = assert_initialized(treasury_info)?;
    let mut fractionalized_token_pool: FractionalizedTokenPool =
        try_from_slice_unchecked(&fractionalized_token_pool_info.data.borrow())?;

    if fraction_mint.supply != 0 {
        return Err(FractionError::FractionMintNotEmpty.into());
    }

    match fraction_mint.mint_authority {
        solana_program::program_option::COption::None => {
            return Err(FractionError::FractionAuthorityNotProgram.into());
        }
        solana_program::program_option::COption::Some(val) => {
            if val != *program_id {
                return Err(FractionError::FractionAuthorityNotProgram.into());
            }
        }
    }
    match fraction_mint.freeze_authority {
        solana_program::program_option::COption::None => {
            return Err(FractionError::FractionAuthorityNotProgram.into());
        }
        solana_program::program_option::COption::Some(val) => {
            if val != *program_id {
                return Err(FractionError::FractionAuthorityNotProgram.into());
            }
        }
    }

    if treasury.amount != 0 {
        return Err(FractionError::TreasuryNotEmpty.into());
    }

    if treasury.owner != *program_id {
        return Err(FractionError::TreasuryOwnerNotProgram.into());
    }

    fractionalized_token_pool.key = POOL_KEY;
    fractionalized_token_pool.treasury = *treasury_info.key;
    fractionalized_token_pool.fraction_mint = *fraction_mint_info.key;
    fractionalized_token_pool.pricing_lookup_address = *pricing_lookup_address.key;
    fractionalized_token_pool.pricing_lookup_type = pricing_lookup_type;
    fractionalized_token_pool.allow_share_redemption = allow_share_redemption;
    fractionalized_token_pool.authority = *authority_info.key;
    fractionalized_token_pool.token_type_count = 0;

    // This is how we determine inactive pool - all zeroes means no hashing done yet
    // when activate called, the number of token_type_count addresses must be provided,
    // they all must point to initiated accounts, which can only be made by this program since
    // they are pdas, and they will be hashed and set on this field. Then shares distributed,
    // and pool is active.
    let arr_of_zeroes: [u8; 32] = [0; 32];
    fractionalized_token_pool.hashed_fractionalized_token_registry = arr_of_zeroes;

    Ok(())
}
