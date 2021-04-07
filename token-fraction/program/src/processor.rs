use {
    crate::{
        error::FractionError,
        instruction::FractionInstruction,
        state::{
            ExternalPriceAccount, FractionalizedTokenPool, FractionalizedTokenRegistry, PoolState,
            MAX_POOL_SIZE, MAX_TOKEN_REGISTRY_SIZE, POOL_KEY, PREFIX, REGISTRY_KEY,
        },
        utils::{
            assert_initialized, assert_owned_by, assert_rent_exempt,
            create_or_allocate_account_raw, spl_token_burn, spl_token_mint_to, spl_token_transfer,
            TokenBurnParams, TokenMintToParams, TokenTransferParams,
        },
    },
    borsh::{BorshDeserialize, BorshSerialize},
    sha2::{Digest, Sha256},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        borsh::try_from_slice_unchecked,
        entrypoint::ProgramResult,
        msg,
        pubkey::Pubkey,
        rent::Rent,
        sysvar::Sysvar,
    },
    spl_token::state::{Account, Mint},
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
                args.allow_further_share_creation,
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
        FractionInstruction::ActivateFractionalizedTokenPool(args) => {
            msg!("Instruction: Activate Fractionalized Token Pool");
            process_activate_fractionalized_token_pool(program_id, accounts, args.number_of_shares)
        }
        FractionInstruction::CombineFractionalizedTokenPool => {
            msg!("Instruction: Activate Fractionalized Token Pool");
            process_combine_fractionalized_token_pool(program_id, accounts)
        }
    }
}

pub fn process_combine_fractionalized_token_pool(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let fractionalized_token_pool_info = next_account_info(account_info_iter)?;
    let your_outstanding_shares_info = next_account_info(account_info_iter)?;
    let your_payment_info = next_account_info(account_info_iter)?;
    let fraction_mint_info = next_account_info(account_info_iter)?;
    let redeem_treasury_info = next_account_info(account_info_iter)?;
    let transfer_authority_info = next_account_info(account_info_iter)?;
    let burn_authority_info = next_account_info(account_info_iter)?;
    let external_pricing_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;

    let mut fractionalized_token_pool: FractionalizedTokenPool =
        try_from_slice_unchecked(&fractionalized_token_pool_info.data.borrow_mut())?;
    let fraction_mint: Mint = assert_initialized(fraction_mint_info)?;
    let your_payment_account: Account = assert_initialized(your_payment_info)?;
    let your_outstanding_shares: Account = assert_initialized(your_outstanding_shares_info)?;
    let external_pricing: ExternalPriceAccount =
        try_from_slice_unchecked(&external_pricing_info.data.borrow_mut())?;

    if fractionalized_token_pool.state != PoolState::Active {
        return Err(FractionError::PoolShouldBeActive.into());
    }

    if your_payment_account.mint != external_pricing.price_mint {
        return Err(FractionError::PaymentMintShouldMatchPricingMint.into());
    }

    if your_outstanding_shares.mint != *fraction_mint_info.key {
        return Err(FractionError::ShareMintShouldMatchFractionalMint.into());
    }

    if fraction_mint_info.key != &fractionalized_token_pool.fraction_mint {
        return Err(FractionError::FractionMintNeedsToMatchPool.into());
    }

    if redeem_treasury_info.key != &fractionalized_token_pool.redeem_treasury {
        return Err(FractionError::RedeemTreasuryNeedsToMatchPool.into());
    }

    if !external_pricing.allowed_to_combine {
        return Err(FractionError::NotAllowedToCombine.into());
    }

    let market_cap = match fraction_mint
        .supply
        .checked_mul(external_pricing.price_per_share)
    {
        Some(val) => val,
        None => return Err(FractionError::NumericalOverflowError.into()),
    };

    let your_share_value = match your_outstanding_shares
        .amount
        .checked_mul(external_pricing.price_per_share)
    {
        Some(val) => val,
        None => return Err(FractionError::NumericalOverflowError.into()),
    };

    let what_you_owe = match market_cap.checked_sub(your_share_value) {
        Some(val) => val,
        None => return Err(FractionError::NumericalOverflowError.into()),
    };

    if your_payment_account.amount < what_you_owe {
        return Err(FractionError::CannotAffordToCombineThisPool.into());
    }

    let (_, bump_seed) = Pubkey::find_program_address(&[program_id.as_ref()], program_id);
    let authority_signer_seeds = &[program_id.as_ref(), &[bump_seed]];

    spl_token_transfer(TokenTransferParams {
        source: your_payment_info.clone(),
        destination: redeem_treasury_info.clone(),
        amount: what_you_owe,
        authority: transfer_authority_info.clone(),
        authority_signer_seeds: authority_signer_seeds,
        token_program: token_program_info.clone(),
    })?;

    spl_token_burn(TokenBurnParams {
        mint: fraction_mint_info.clone(),
        amount: your_outstanding_shares.amount,
        authority: burn_authority_info.clone(),
        authority_signer_seeds: authority_signer_seeds,
        token_program: token_program_info.clone(),
        source: your_outstanding_shares_info.clone(),
    })?;

    fractionalized_token_pool.state = PoolState::Combined;
    fractionalized_token_pool.serialize(&mut *fractionalized_token_pool_info.data.borrow_mut())?;

    Ok(())
}

pub fn process_activate_fractionalized_token_pool(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    number_of_shares: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let fractionalized_token_pool_info = next_account_info(account_info_iter)?;
    let fraction_mint_info = next_account_info(account_info_iter)?;
    let fraction_treasury_info = next_account_info(account_info_iter)?;
    let fractional_mint_authority_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;

    let mut fractionalized_token_pool: FractionalizedTokenPool =
        try_from_slice_unchecked(&fractionalized_token_pool_info.data.borrow_mut())?;

    if fractionalized_token_pool.state != PoolState::Inactive {
        return Err(FractionError::PoolShouldBeInactive.into());
    }

    let (authority_key, bump_seed) =
        Pubkey::find_program_address(&[program_id.as_ref()], program_id);
    if fractional_mint_authority_info.key != &authority_key {
        return Err(FractionError::InvalidAuthority.into());
    }
    let authority_signer_seeds = &[program_id.as_ref(), &[bump_seed]];

    spl_token_mint_to(TokenMintToParams {
        mint: fraction_mint_info.clone(),
        destination: fraction_treasury_info.clone(),
        amount: number_of_shares,
        authority: fractional_mint_authority_info.clone(),
        authority_signer_seeds: authority_signer_seeds,
        token_program: token_program_info.clone(),
    })?;

    fractionalized_token_pool.state = PoolState::Active;
    fractionalized_token_pool.serialize(&mut *fractionalized_token_pool_info.data.borrow_mut())?;

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

    let rent = &Rent::from_account_info(rent_info)?;
    assert_rent_exempt(rent, token_account_info)?;
    assert_rent_exempt(rent, vault_info)?;

    let token_account: Account = assert_initialized(token_account_info)?;
    let vault: Account = assert_initialized(vault_info)?;
    let mut fractionalized_token_pool: FractionalizedTokenPool =
        try_from_slice_unchecked(&fractionalized_token_pool_info.data.borrow_mut())?;

    if fractionalized_token_pool.state != PoolState::Inactive {
        return Err(FractionError::PoolShouldBeInactive.into());
    }

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

    let mut registry: FractionalizedTokenRegistry =
        try_from_slice_unchecked(&registry_account_info.data.borrow_mut())?;
    registry.key = REGISTRY_KEY;
    registry.fractionalized_token_pool = *fractionalized_token_pool_info.key;
    registry.token_mint = token_account.mint;
    registry.vault = *vault_info.key;
    registry.order = fractionalized_token_pool.token_type_count;

    registry.serialize(&mut *registry_account_info.data.borrow_mut())?;

    fractionalized_token_pool.token_type_count =
        match fractionalized_token_pool.token_type_count.checked_add(1) {
            Some(val) => val,
            None => return Err(FractionError::NumericalOverflowError.into()),
        };
    let mut hasher = Sha256::new();
    let mut new_arr: [u8; 64] = [0; 64];
    for n in 0..63 {
        if n < 32 {
            new_arr[n] = fractionalized_token_pool.hashed_fractionalized_token_registry[n];
        } else {
            new_arr[n] = registry_account_info.key.as_ref()[n - 32];
        }
    }
    hasher.update(new_arr);
    let result = hasher.finalize();

    let mut hashed_arr: [u8; 32] = [0; 32];
    let slice = result.as_slice();
    for n in 0..31 {
        hashed_arr[n] = slice[n];
    }

    fractionalized_token_pool.hashed_fractionalized_token_registry = hashed_arr;

    fractionalized_token_pool.serialize(&mut *fractionalized_token_pool_info.data.borrow_mut())?;

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
    allow_further_share_creation: bool,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let fraction_mint_info = next_account_info(account_info_iter)?;
    let redeem_treasury_info = next_account_info(account_info_iter)?;
    let fraction_treasury_info = next_account_info(account_info_iter)?;
    let fractionalized_token_pool_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let pricing_lookup_address = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(next_account_info(account_info_iter)?)?;

    let fraction_mint: Mint = assert_initialized(fraction_mint_info)?;
    let redeem_treasury: Account = assert_initialized(redeem_treasury_info)?;
    let fraction_treasury: Account = assert_initialized(fraction_treasury_info)?;
    let mut fractionalized_token_pool: FractionalizedTokenPool =
        try_from_slice_unchecked(&fractionalized_token_pool_info.data.borrow())?;
    let external_pricing_lookup: ExternalPriceAccount =
        try_from_slice_unchecked(&pricing_lookup_address.data.borrow_mut())?;

    assert_rent_exempt(rent, redeem_treasury_info)?;
    assert_rent_exempt(rent, fraction_treasury_info)?;
    assert_rent_exempt(rent, fraction_mint_info)?;
    assert_rent_exempt(rent, fractionalized_token_pool_info)?;
    assert_rent_exempt(rent, pricing_lookup_address)?;
    assert_owned_by(fraction_mint_info, token_program_info.key)?;
    assert_owned_by(fraction_treasury_info, token_program_info.key)?;
    assert_owned_by(redeem_treasury_info, token_program_info.key)?;

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

    if redeem_treasury.amount != 0 {
        return Err(FractionError::TreasuryNotEmpty.into());
    }

    if redeem_treasury.owner != *program_id {
        return Err(FractionError::TreasuryOwnerNotProgram.into());
    }

    if redeem_treasury.mint == external_pricing_lookup.price_mint {
        return Err(FractionError::RedeemTreasuryMintMustMatchLookupMint.into());
    }

    if redeem_treasury.mint != *fraction_mint_info.key {
        return Err(FractionError::RedeemTreasuryCantShareSameMintAsFraction.into());
    }

    if fraction_treasury.amount != 0 {
        return Err(FractionError::TreasuryNotEmpty.into());
    }

    if fraction_treasury.owner != *program_id {
        return Err(FractionError::TreasuryOwnerNotProgram.into());
    }

    if fraction_treasury.mint != *fraction_mint_info.key {
        return Err(FractionError::FractionTreasuryMintDoesNotMatchFractionMint.into());
    }

    fractionalized_token_pool.key = POOL_KEY;
    fractionalized_token_pool.redeem_treasury = *redeem_treasury_info.key;
    fractionalized_token_pool.fraction_treasury = *fraction_treasury_info.key;
    fractionalized_token_pool.fraction_mint = *fraction_mint_info.key;
    fractionalized_token_pool.pricing_lookup_address = *pricing_lookup_address.key;
    fractionalized_token_pool.allow_further_share_creation = allow_further_share_creation;
    fractionalized_token_pool.authority = *authority_info.key;
    fractionalized_token_pool.token_type_count = 0;
    fractionalized_token_pool.state = PoolState::Inactive;

    let arr_of_zeroes: [u8; 32] = [0; 32];
    fractionalized_token_pool.hashed_fractionalized_token_registry = arr_of_zeroes;
    fractionalized_token_pool.serialize(&mut *fractionalized_token_pool_info.data.borrow_mut())?;

    Ok(())
}
