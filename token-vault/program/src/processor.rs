use {
    crate::{
        error::VaultError,
        instruction::VaultInstruction,
        state::{
            ExternalPriceAccount, SafetyDepositBox, Vault, VaultState, MAX_TOKEN_REGISTRY_SIZE,
            PREFIX, SAFETY_DEPOSIT_KEY, VAULT_KEY,
        },
        utils::{
            assert_initialized, assert_owned_by, assert_rent_exempt, assert_token_matching,
            assert_vault_authority_correct, create_or_allocate_account_raw, spl_token_burn,
            spl_token_mint_to, spl_token_transfer, TokenBurnParams, TokenMintToParams,
            TokenTransferParams,
        },
    },
    borsh::{BorshDeserialize, BorshSerialize},
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
    let instruction = VaultInstruction::try_from_slice(input)?;
    match instruction {
        VaultInstruction::InitVault(args) => {
            msg!("Instruction: Init Vault");
            process_init_vault(program_id, accounts, args.allow_further_share_creation)
        }
        VaultInstruction::AddTokenToInactiveVault(args) => {
            msg!("Instruction: Add token to vault");
            process_add_token_to_inactivated_vault(program_id, accounts, args.amount)
        }
        VaultInstruction::ActivateVault(args) => {
            msg!("Instruction: Activate Vault ");
            process_activate_vault(program_id, accounts, args.number_of_shares)
        }
        VaultInstruction::CombineVault => {
            msg!("Instruction: Combine Vault");
            process_combine_vault(program_id, accounts)
        }
        VaultInstruction::RedeemShares => {
            msg!("Instruction: Redeem Shares");
            process_redeem_shares(program_id, accounts)
        }
        VaultInstruction::WithdrawTokenFromSafetyDepositBox => {
            msg!("Instruction: Withdraw Token from Safety Deposit Box");
            process_withdraw_token_from_safety_deposit_box(program_id, accounts)
        }
        VaultInstruction::MintFractionalShares(args) => {
            msg!("Instruction: Mint new fractional shares");
            process_mint_fractional_shares(program_id, accounts, args.number_of_shares)
        }
        VaultInstruction::WithdrawSharesFromTreasury(args) => {
            msg!("Instruction: Withdraw fractional shares");
            process_withdraw_fractional_shares_from_treasury(
                program_id,
                accounts,
                args.number_of_shares,
            )
        }
        VaultInstruction::AddSharesToTreasury(args) => {
            msg!("Instruction: Add fractional shares to treasury");
            process_add_fractional_shares_to_treasury(program_id, accounts, args.number_of_shares)
        }

        VaultInstruction::UpdateExternalPriceAccount(args) => {
            msg!("Instruction: Update External Price Account");
            process_update_external_price_account(
                program_id,
                accounts,
                args.price_per_share,
                args.price_mint,
                args.allowed_to_combine,
            )
        }
    }
}

pub fn process_update_external_price_account(
    _: &Pubkey,
    accounts: &[AccountInfo],
    price_per_share: u64,
    price_mint: Pubkey,
    allowed_to_combine: bool,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let account = next_account_info(account_info_iter)?;
    if !account.is_signer {
        return Err(VaultError::ExternalPriceAccountMustBeSigner.into());
    }

    let mut external_price_account: ExternalPriceAccount =
        try_from_slice_unchecked(&account.data.borrow_mut())?;

    external_price_account.price_per_share = price_per_share;
    external_price_account.price_mint = price_mint;
    external_price_account.allowed_to_combine = allowed_to_combine;

    external_price_account.serialize(&mut *account.data.borrow_mut())?;

    Ok(())
}

pub fn process_add_fractional_shares_to_treasury(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    number_of_shares: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let source_info = next_account_info(account_info_iter)?;
    let fraction_treasury_info = next_account_info(account_info_iter)?;
    let vault_info = next_account_info(account_info_iter)?;
    let transfer_authority_info = next_account_info(account_info_iter)?;
    let vault_authority_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;

    let vault: Vault = try_from_slice_unchecked(&vault_info.data.borrow_mut())?;
    let source: Account = assert_initialized(source_info)?;

    assert_owned_by(source_info, token_program_info.key)?;
    assert_token_matching(&vault, token_program_info)?;
    assert_owned_by(vault_info, program_id)?;
    assert_vault_authority_correct(&vault, vault_authority_info)?;

    if vault.state != VaultState::Active {
        return Err(VaultError::VaultShouldBeActive.into());
    }

    if *fraction_treasury_info.key != vault.fraction_treasury {
        return Err(VaultError::FractionTreasuryNeedsToMatchVault.into());
    }

    if source.mint != vault.fraction_mint {
        return Err(VaultError::SourceAccountNeedsToMatchFractionMint.into());
    }

    if source.amount < number_of_shares {
        return Err(VaultError::NotEnoughShares.into());
    }

    let (_, bump_seed) =
        Pubkey::find_program_address(&[PREFIX.as_bytes(), program_id.as_ref()], program_id);
    let authority_signer_seeds = &[PREFIX.as_bytes(), program_id.as_ref(), &[bump_seed]];

    spl_token_transfer(TokenTransferParams {
        source: source_info.clone(),
        destination: fraction_treasury_info.clone(),
        amount: number_of_shares,
        authority: transfer_authority_info.clone(),
        authority_signer_seeds,
        token_program: token_program_info.clone(),
    })?;

    Ok(())
}

pub fn process_withdraw_fractional_shares_from_treasury(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    number_of_shares: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let destination_info = next_account_info(account_info_iter)?;
    let fraction_treasury_info = next_account_info(account_info_iter)?;
    let vault_info = next_account_info(account_info_iter)?;
    let transfer_authority_info = next_account_info(account_info_iter)?;
    let vault_authority_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;

    let rent = &Rent::from_account_info(rent_info)?;
    let vault: Vault = try_from_slice_unchecked(&vault_info.data.borrow_mut())?;
    let destination: Account = assert_initialized(destination_info)?;
    let fraction_treasury: Account = assert_initialized(fraction_treasury_info)?;

    // We watch out for you!
    assert_rent_exempt(rent, destination_info)?;
    assert_owned_by(destination_info, token_program_info.key)?;
    assert_token_matching(&vault, token_program_info)?;
    assert_owned_by(vault_info, program_id)?;
    assert_vault_authority_correct(&vault, vault_authority_info)?;

    if vault.state != VaultState::Active {
        return Err(VaultError::VaultShouldBeActive.into());
    }

    if *fraction_treasury_info.key != vault.fraction_treasury {
        return Err(VaultError::FractionTreasuryNeedsToMatchVault.into());
    }

    if destination.mint != vault.fraction_mint {
        return Err(VaultError::DestinationAccountNeedsToMatchFractionMint.into());
    }

    if fraction_treasury.amount < number_of_shares {
        return Err(VaultError::NotEnoughShares.into());
    }

    let (authority, bump_seed) =
        Pubkey::find_program_address(&[PREFIX.as_bytes(), program_id.as_ref()], program_id);
    let authority_signer_seeds = &[PREFIX.as_bytes(), program_id.as_ref(), &[bump_seed]];

    if authority != *transfer_authority_info.key {
        return Err(VaultError::InvalidAuthority.into());
    }

    spl_token_transfer(TokenTransferParams {
        source: fraction_treasury_info.clone(),
        destination: destination_info.clone(),
        amount: number_of_shares,
        authority: transfer_authority_info.clone(),
        authority_signer_seeds,
        token_program: token_program_info.clone(),
    })?;

    Ok(())
}

pub fn process_mint_fractional_shares(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    number_of_shares: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let fraction_treasury_info = next_account_info(account_info_iter)?;
    let fraction_mint_info = next_account_info(account_info_iter)?;
    let vault_info = next_account_info(account_info_iter)?;
    let mint_authority_info = next_account_info(account_info_iter)?;
    let vault_authority_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;

    let vault: Vault = try_from_slice_unchecked(&vault_info.data.borrow_mut())?;

    assert_token_matching(&vault, token_program_info)?;
    assert_owned_by(vault_info, program_id)?;
    assert_vault_authority_correct(&vault, vault_authority_info)?;

    if vault.state != VaultState::Active {
        return Err(VaultError::VaultShouldBeActive.into());
    }

    if *fraction_treasury_info.key != vault.fraction_treasury {
        return Err(VaultError::FractionTreasuryNeedsToMatchVault.into());
    }

    if fraction_mint_info.key != &vault.fraction_mint {
        return Err(VaultError::VaultMintNeedsToMatchVault.into());
    }

    if !vault.allow_further_share_creation {
        return Err(VaultError::VaultDoesNotAllowNewShareMinting.into());
    }

    let (authority, bump_seed) =
        Pubkey::find_program_address(&[PREFIX.as_bytes(), program_id.as_ref()], program_id);
    let authority_signer_seeds = &[PREFIX.as_bytes(), program_id.as_ref(), &[bump_seed]];

    if authority != *mint_authority_info.key {
        return Err(VaultError::InvalidAuthority.into());
    }

    spl_token_mint_to(TokenMintToParams {
        mint: fraction_mint_info.clone(),
        destination: fraction_treasury_info.clone(),
        amount: number_of_shares,
        authority: mint_authority_info.clone(),
        authority_signer_seeds,
        token_program: token_program_info.clone(),
    })?;

    Ok(())
}

pub fn process_withdraw_token_from_safety_deposit_box(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let destination_info = next_account_info(account_info_iter)?;
    let safety_deposit_info = next_account_info(account_info_iter)?;
    let store_info = next_account_info(account_info_iter)?;
    let vault_info = next_account_info(account_info_iter)?;
    let fraction_mint_info = next_account_info(account_info_iter)?;
    let vault_authority_info = next_account_info(account_info_iter)?;
    let transfer_authority_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;

    let rent = &Rent::from_account_info(rent_info)?;
    let mut vault: Vault = try_from_slice_unchecked(&vault_info.data.borrow_mut())?;
    let safety_deposit: SafetyDepositBox =
        try_from_slice_unchecked(&safety_deposit_info.data.borrow_mut())?;
    let fraction_mint: Mint = assert_initialized(fraction_mint_info)?;
    let destination: Account = assert_initialized(destination_info)?;
    let store: Account = assert_initialized(store_info)?;

    // We watch out for you!
    assert_rent_exempt(rent, destination_info)?;
    assert_owned_by(destination_info, token_program_info.key)?;
    assert_owned_by(safety_deposit_info, program_id)?;
    assert_owned_by(vault_info, program_id)?;
    assert_token_matching(&vault, token_program_info)?;
    assert_vault_authority_correct(&vault, vault_authority_info)?;

    if vault.state != VaultState::Combined && vault.state != VaultState::Inactive {
        return Err(VaultError::VaultShouldBeCombinedOrInactive.into());
    }

    if safety_deposit.vault != *vault_info.key {
        return Err(VaultError::SafetyDepositBoxVaultMismatch.into());
    }

    if fraction_mint_info.key != &vault.fraction_mint {
        return Err(VaultError::VaultMintNeedsToMatchVault.into());
    }

    if *store_info.key != safety_deposit.store {
        return Err(VaultError::StoreDoesNotMatchSafetyDepositBox.into());
    }

    if store.amount == 0 {
        return Err(VaultError::StoreEmpty.into());
    }

    if destination.mint != safety_deposit.token_mint {
        return Err(VaultError::DestinationAccountNeedsToMatchTokenMint.into());
    }

    let (authority, bump_seed) =
        Pubkey::find_program_address(&[PREFIX.as_bytes(), program_id.as_ref()], program_id);
    let authority_signer_seeds = &[PREFIX.as_bytes(), program_id.as_ref(), &[bump_seed]];

    if authority != *transfer_authority_info.key {
        return Err(VaultError::InvalidAuthority.into());
    }

    spl_token_transfer(TokenTransferParams {
        source: store_info.clone(),
        destination: destination_info.clone(),
        amount: store.amount,
        authority: transfer_authority_info.clone(),
        authority_signer_seeds,
        token_program: token_program_info.clone(),
    })?;

    vault.token_type_count = match vault.token_type_count.checked_sub(1) {
        Some(val) => val,
        None => return Err(VaultError::NumericalOverflowError.into()),
    };

    if fraction_mint.supply == 0
        && vault.token_type_count == 0
        && vault.state == VaultState::Combined
    {
        vault.state = VaultState::Deactivated;
        vault.serialize(&mut *vault_info.data.borrow_mut())?;
    }
    Ok(())
}

pub fn process_redeem_shares(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let outstanding_shares_info = next_account_info(account_info_iter)?;
    let destination_info = next_account_info(account_info_iter)?;
    let fraction_mint_info = next_account_info(account_info_iter)?;
    let redeem_treasury_info = next_account_info(account_info_iter)?;
    let transfer_authority_info = next_account_info(account_info_iter)?;
    let burn_authority_info = next_account_info(account_info_iter)?;
    let vault_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;

    let rent = &Rent::from_account_info(rent_info)?;
    let mut vault: Vault = try_from_slice_unchecked(&vault_info.data.borrow_mut())?;
    let fraction_mint: Mint = assert_initialized(fraction_mint_info)?;
    let outstanding_shares: Account = assert_initialized(outstanding_shares_info)?;
    let destination: Account = assert_initialized(destination_info)?;
    let redeem_treasury: Account = assert_initialized(redeem_treasury_info)?;
    // We watch out for you!
    assert_rent_exempt(rent, destination_info)?;
    assert_owned_by(destination_info, token_program_info.key)?;
    assert_owned_by(vault_info, program_id)?;
    assert_owned_by(outstanding_shares_info, token_program_info.key)?;
    assert_token_matching(&vault, token_program_info)?;

    if outstanding_shares.amount == 0 {
        return Err(VaultError::NoShares.into());
    }

    if outstanding_shares.mint != *fraction_mint_info.key {
        return Err(VaultError::OutstandingShareAccountNeedsToMatchFractionalMint.into());
    }

    if destination.mint != redeem_treasury.mint {
        return Err(VaultError::DestinationAccountNeedsToMatchRedeemMint.into());
    }

    if vault.state != VaultState::Combined {
        return Err(VaultError::VaultShouldBeCombined.into());
    }

    if fraction_mint_info.key != &vault.fraction_mint {
        return Err(VaultError::VaultMintNeedsToMatchVault.into());
    }

    if redeem_treasury_info.key != &vault.redeem_treasury {
        return Err(VaultError::RedeemTreasuryNeedsToMatchVault.into());
    }

    if fraction_mint.supply == 0 {
        // Basically impossible but I want to be safe
        return Err(VaultError::FractionSupplyEmpty.into());
    }

    let we_owe_you = match vault
        .locked_price_per_share
        .checked_mul(outstanding_shares.amount)
    {
        Some(val) => val,
        None => return Err(VaultError::NumericalOverflowError.into()),
    };

    let (_, bump_seed) =
        Pubkey::find_program_address(&[PREFIX.as_bytes(), program_id.as_ref()], program_id);
    let authority_signer_seeds = &[PREFIX.as_bytes(), program_id.as_ref(), &[bump_seed]];

    spl_token_transfer(TokenTransferParams {
        source: redeem_treasury_info.clone(),
        destination: destination_info.clone(),
        amount: we_owe_you,
        authority: transfer_authority_info.clone(),
        authority_signer_seeds,
        token_program: token_program_info.clone(),
    })?;

    spl_token_burn(TokenBurnParams {
        mint: fraction_mint_info.clone(),
        amount: outstanding_shares.amount,
        authority: burn_authority_info.clone(),
        authority_signer_seeds,
        token_program: token_program_info.clone(),
        source: outstanding_shares_info.clone(),
    })?;

    let fractional_remaining = match fraction_mint.supply.checked_sub(outstanding_shares.amount) {
        Some(val) => val,
        None => return Err(VaultError::NumericalOverflowError.into()),
    };

    if fractional_remaining == 0 && vault.token_type_count == 0 {
        vault.state = VaultState::Deactivated;
        vault.serialize(&mut *vault_info.data.borrow_mut())?;
    }

    Ok(())
}

pub fn process_combine_vault(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let vault_info = next_account_info(account_info_iter)?;
    let your_outstanding_shares_info = next_account_info(account_info_iter)?;
    let your_payment_info = next_account_info(account_info_iter)?;
    let fraction_mint_info = next_account_info(account_info_iter)?;
    let fraction_treasury_info = next_account_info(account_info_iter)?;
    let redeem_treasury_info = next_account_info(account_info_iter)?;
    let vault_authority_info = next_account_info(account_info_iter)?;
    let transfer_authority_info = next_account_info(account_info_iter)?;
    let fraction_burn_authority_info = next_account_info(account_info_iter)?;
    let external_pricing_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;

    let mut vault: Vault = try_from_slice_unchecked(&vault_info.data.borrow_mut())?;
    let fraction_mint: Mint = assert_initialized(fraction_mint_info)?;
    let fraction_treasury: Account = assert_initialized(fraction_treasury_info)?;
    let redeem_treasury: Account = assert_initialized(redeem_treasury_info)?;
    let your_payment_account: Account = assert_initialized(your_payment_info)?;
    let your_outstanding_shares: Account = assert_initialized(your_outstanding_shares_info)?;
    let external_pricing: ExternalPriceAccount =
        try_from_slice_unchecked(&external_pricing_info.data.borrow_mut())?;

    assert_token_matching(&vault, token_program_info)?;
    assert_owned_by(vault_info, program_id)?;
    assert_vault_authority_correct(&vault, vault_authority_info)?;

    if vault.state != VaultState::Active {
        return Err(VaultError::VaultShouldBeActive.into());
    }

    if your_payment_account.mint != external_pricing.price_mint {
        return Err(VaultError::PaymentMintShouldMatchPricingMint.into());
    }

    if redeem_treasury.mint != external_pricing.price_mint {
        // Did someone mess with our oracle?
        return Err(VaultError::RedeemTreasuryMintShouldMatchPricingMint.into());
    }

    if your_outstanding_shares.mint != *fraction_mint_info.key {
        return Err(VaultError::ShareMintShouldMatchFractionalMint.into());
    }

    if fraction_mint_info.key != &vault.fraction_mint {
        return Err(VaultError::VaultMintNeedsToMatchVault.into());
    }

    if redeem_treasury_info.key != &vault.redeem_treasury {
        return Err(VaultError::RedeemTreasuryNeedsToMatchVault.into());
    }

    if !external_pricing.allowed_to_combine {
        return Err(VaultError::NotAllowedToCombine.into());
    }

    let total_market_cap = match fraction_mint
        .supply
        .checked_mul(external_pricing.price_per_share)
    {
        Some(val) => val,
        None => return Err(VaultError::NumericalOverflowError.into()),
    };

    let stored_market_cap = match fraction_treasury
        .amount
        .checked_mul(external_pricing.price_per_share)
    {
        Some(val) => val,
        None => return Err(VaultError::NumericalOverflowError.into()),
    };

    let circulating_market_cap = match total_market_cap.checked_sub(stored_market_cap) {
        Some(val) => val,
        None => return Err(VaultError::NumericalOverflowError.into()),
    };

    let your_share_value = match your_outstanding_shares
        .amount
        .checked_mul(external_pricing.price_per_share)
    {
        Some(val) => val,
        None => return Err(VaultError::NumericalOverflowError.into()),
    };

    let what_you_owe = match circulating_market_cap.checked_sub(your_share_value) {
        Some(val) => val,
        None => return Err(VaultError::NumericalOverflowError.into()),
    };

    if your_payment_account.amount < what_you_owe {
        return Err(VaultError::CannotAffordToCombineThisVault.into());
    }

    let (authority, bump_seed) =
        Pubkey::find_program_address(&[PREFIX.as_bytes(), program_id.as_ref()], program_id);
    let authority_signer_seeds = &[PREFIX.as_bytes(), program_id.as_ref(), &[bump_seed]];

    if authority != *fraction_burn_authority_info.key {
        return Err(VaultError::InvalidAuthority.into());
    }

    spl_token_transfer(TokenTransferParams {
        source: your_payment_info.clone(),
        destination: redeem_treasury_info.clone(),
        amount: what_you_owe,
        authority: transfer_authority_info.clone(),
        authority_signer_seeds,
        token_program: token_program_info.clone(),
    })?;

    spl_token_burn(TokenBurnParams {
        mint: fraction_mint_info.clone(),
        amount: your_outstanding_shares.amount,
        authority: transfer_authority_info.clone(),
        authority_signer_seeds,
        token_program: token_program_info.clone(),
        source: your_outstanding_shares_info.clone(),
    })?;

    spl_token_burn(TokenBurnParams {
        mint: fraction_mint_info.clone(),
        amount: fraction_treasury.amount,
        authority: fraction_burn_authority_info.clone(),
        authority_signer_seeds,
        token_program: token_program_info.clone(),
        source: fraction_treasury_info.clone(),
    })?;

    vault.state = VaultState::Combined;
    vault.locked_price_per_share = external_pricing.price_per_share;
    vault.serialize(&mut *vault_info.data.borrow_mut())?;

    Ok(())
}

pub fn process_activate_vault(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    number_of_shares: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let vault_info = next_account_info(account_info_iter)?;
    let fraction_mint_info = next_account_info(account_info_iter)?;
    let fraction_treasury_info = next_account_info(account_info_iter)?;
    let fractional_mint_authority_info = next_account_info(account_info_iter)?;
    let vault_authority_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;

    let mut vault: Vault = try_from_slice_unchecked(&vault_info.data.borrow_mut())?;
    assert_owned_by(vault_info, program_id)?;
    assert_token_matching(&vault, token_program_info)?;
    assert_vault_authority_correct(&vault, vault_authority_info)?;

    if vault.state != VaultState::Inactive {
        return Err(VaultError::VaultShouldBeInactive.into());
    }

    let (authority_key, bump_seed) =
        Pubkey::find_program_address(&[PREFIX.as_bytes(), program_id.as_ref()], program_id);
    if fractional_mint_authority_info.key != &authority_key {
        return Err(VaultError::InvalidAuthority.into());
    }
    let authority_signer_seeds = &[PREFIX.as_bytes(), program_id.as_ref(), &[bump_seed]];

    spl_token_mint_to(TokenMintToParams {
        mint: fraction_mint_info.clone(),
        destination: fraction_treasury_info.clone(),
        amount: number_of_shares,
        authority: fractional_mint_authority_info.clone(),
        authority_signer_seeds,
        token_program: token_program_info.clone(),
    })?;

    vault.state = VaultState::Active;
    vault.serialize(&mut *vault_info.data.borrow_mut())?;

    Ok(())
}

pub fn process_add_token_to_inactivated_vault(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let safety_deposit_account_info = next_account_info(account_info_iter)?;
    let token_account_info = next_account_info(account_info_iter)?;
    let store_info = next_account_info(account_info_iter)?;
    let vault_info = next_account_info(account_info_iter)?;
    let vault_authority_info = next_account_info(account_info_iter)?;
    let payer_info = next_account_info(account_info_iter)?;
    let transfer_authority_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;
    let system_account_info = next_account_info(account_info_iter)?;

    let rent = &Rent::from_account_info(rent_info)?;
    assert_owned_by(vault_info, program_id)?;
    assert_rent_exempt(rent, token_account_info)?;
    assert_rent_exempt(rent, vault_info)?;
    assert_owned_by(store_info, token_program_info.key)?;
    assert_owned_by(token_account_info, token_program_info.key)?;

    let token_account: Account = assert_initialized(token_account_info)?;
    let store: Account = assert_initialized(store_info)?;
    let mut vault: Vault = try_from_slice_unchecked(&vault_info.data.borrow_mut())?;
    assert_token_matching(&vault, token_program_info)?;
    assert_vault_authority_correct(&vault, vault_authority_info)?;

    if vault.state != VaultState::Inactive {
        return Err(VaultError::VaultShouldBeInactive.into());
    }

    if token_account.amount == 0 {
        return Err(VaultError::TokenAccountContainsNoTokens.into());
    }

    if token_account.amount < amount {
        return Err(VaultError::TokenAccountAmountLessThanAmountSpecified.into());
    }

    if store.amount > 0 {
        return Err(VaultError::VaultAccountIsNotEmpty.into());
    }

    let seeds = &[PREFIX.as_bytes(), &program_id.as_ref()];
    let (authority, _) = Pubkey::find_program_address(seeds, program_id);

    if store.owner != authority {
        return Err(VaultError::VaultAccountIsNotOwnedByProgram.into());
    }

    let seeds = &[
        PREFIX.as_bytes(),
        vault_info.key.as_ref(),
        token_account.mint.as_ref(),
    ];
    let (safety_deposit_account_key, bump_seed) = Pubkey::find_program_address(seeds, program_id);

    if safety_deposit_account_key != *safety_deposit_account_info.key {
        return Err(VaultError::RegistryAccountAddressInvalid.into());
    }
    let authority_signer_seeds = &[
        PREFIX.as_bytes(),
        vault_info.key.as_ref(),
        token_account.mint.as_ref(),
        &[bump_seed],
    ];
    create_or_allocate_account_raw(
        *program_id,
        safety_deposit_account_info,
        rent_info,
        system_account_info,
        payer_info,
        MAX_TOKEN_REGISTRY_SIZE,
        authority_signer_seeds,
    )?;

    let mut safety_deposit_account: SafetyDepositBox =
        try_from_slice_unchecked(&safety_deposit_account_info.data.borrow_mut())?;
    safety_deposit_account.key = SAFETY_DEPOSIT_KEY;
    safety_deposit_account.vault = *vault_info.key;
    safety_deposit_account.token_mint = token_account.mint;
    safety_deposit_account.store = *store_info.key;
    safety_deposit_account.order = vault.token_type_count;

    safety_deposit_account.serialize(&mut *safety_deposit_account_info.data.borrow_mut())?;

    vault.token_type_count = match vault.token_type_count.checked_add(1) {
        Some(val) => val,
        None => return Err(VaultError::NumericalOverflowError.into()),
    };

    vault.serialize(&mut *vault_info.data.borrow_mut())?;

    spl_token_transfer(TokenTransferParams {
        source: token_account_info.clone(),
        destination: store_info.clone(),
        amount,
        authority: transfer_authority_info.clone(),
        authority_signer_seeds,
        token_program: token_program_info.clone(),
    })?;

    Ok(())
}

pub fn process_init_vault(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    allow_further_share_creation: bool,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let fraction_mint_info = next_account_info(account_info_iter)?;
    let redeem_treasury_info = next_account_info(account_info_iter)?;
    let fraction_treasury_info = next_account_info(account_info_iter)?;
    let vault_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let pricing_lookup_address = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(next_account_info(account_info_iter)?)?;

    let fraction_mint: Mint = assert_initialized(fraction_mint_info)?;
    let redeem_treasury: Account = assert_initialized(redeem_treasury_info)?;
    let fraction_treasury: Account = assert_initialized(fraction_treasury_info)?;
    let mut vault: Vault = try_from_slice_unchecked(&vault_info.data.borrow())?;
    let external_pricing_lookup: ExternalPriceAccount =
        try_from_slice_unchecked(&pricing_lookup_address.data.borrow_mut())?;

    assert_rent_exempt(rent, redeem_treasury_info)?;
    assert_rent_exempt(rent, fraction_treasury_info)?;
    assert_rent_exempt(rent, fraction_mint_info)?;
    assert_rent_exempt(rent, vault_info)?;
    assert_rent_exempt(rent, pricing_lookup_address)?;
    assert_owned_by(fraction_mint_info, token_program_info.key)?;
    assert_owned_by(fraction_treasury_info, token_program_info.key)?;
    assert_owned_by(redeem_treasury_info, token_program_info.key)?;

    if fraction_mint.supply != 0 {
        return Err(VaultError::VaultMintNotEmpty.into());
    }

    let seeds = &[PREFIX.as_bytes(), &program_id.as_ref()];
    let (authority, _) = Pubkey::find_program_address(seeds, &program_id);

    match fraction_mint.mint_authority {
        solana_program::program_option::COption::None => {
            return Err(VaultError::VaultAuthorityNotProgram.into());
        }
        solana_program::program_option::COption::Some(val) => {
            if val != authority {
                return Err(VaultError::VaultAuthorityNotProgram.into());
            }
        }
    }
    match fraction_mint.freeze_authority {
        solana_program::program_option::COption::None => {
            return Err(VaultError::VaultAuthorityNotProgram.into());
        }
        solana_program::program_option::COption::Some(val) => {
            if val != authority {
                return Err(VaultError::VaultAuthorityNotProgram.into());
            }
        }
    }

    if redeem_treasury.amount != 0 {
        return Err(VaultError::TreasuryNotEmpty.into());
    }

    if redeem_treasury.owner != authority {
        return Err(VaultError::TreasuryOwnerNotProgram.into());
    }

    if redeem_treasury.mint != external_pricing_lookup.price_mint {
        return Err(VaultError::RedeemTreasuryMintMustMatchLookupMint.into());
    }

    if redeem_treasury.mint == *fraction_mint_info.key {
        return Err(VaultError::RedeemTreasuryCantShareSameMintAsFraction.into());
    }

    if fraction_treasury.amount != 0 {
        return Err(VaultError::TreasuryNotEmpty.into());
    }

    if fraction_treasury.owner != authority {
        return Err(VaultError::TreasuryOwnerNotProgram.into());
    }

    if fraction_treasury.mint != *fraction_mint_info.key {
        return Err(VaultError::VaultTreasuryMintDoesNotMatchVaultMint.into());
    }

    vault.key = VAULT_KEY;
    vault.token_program = *token_program_info.key;
    vault.redeem_treasury = *redeem_treasury_info.key;
    vault.fraction_treasury = *fraction_treasury_info.key;
    vault.fraction_mint = *fraction_mint_info.key;
    vault.pricing_lookup_address = *pricing_lookup_address.key;
    vault.allow_further_share_creation = allow_further_share_creation;
    vault.authority = *authority_info.key;
    vault.token_type_count = 0;
    vault.state = VaultState::Inactive;

    vault.serialize(&mut *vault_info.data.borrow_mut())?;

    Ok(())
}
