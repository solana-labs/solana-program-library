use {
    crate::{
        error::MetaplexError,
        instruction::MetaplexInstruction,
        state::PREFIX,
        utils::{assert_initialized, assert_owned_by, assert_rent_exempt},
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
    let instruction = MetaplexInstruction::try_from_slice(input)?;
    match instruction {
        MetaplexInstruction::InitMetaplex(args) => {
            msg!("Instruction: Init Auction Manager");
            process_init_auction_manager(program_id, accounts, args.allow_further_share_creation)
        }
    }
}

pub fn process_init_auction_manager(
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
    let mut vault: Metaplex = try_from_slice_unchecked(&vault_info.data.borrow())?;
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
        return Err(MetaplexError::MetaplexMintNotEmpty.into());
    }

    let seeds = &[PREFIX.as_bytes(), &program_id.as_ref()];
    let (authority, _) = Pubkey::find_program_address(seeds, &program_id);

    match fraction_mint.mint_authority {
        solana_program::program_option::COption::None => {
            return Err(MetaplexError::MetaplexAuthorityNotProgram.into());
        }
        solana_program::program_option::COption::Some(val) => {
            if val != authority {
                return Err(MetaplexError::MetaplexAuthorityNotProgram.into());
            }
        }
    }
    match fraction_mint.freeze_authority {
        solana_program::program_option::COption::None => {
            return Err(MetaplexError::MetaplexAuthorityNotProgram.into());
        }
        solana_program::program_option::COption::Some(val) => {
            if val != authority {
                return Err(MetaplexError::MetaplexAuthorityNotProgram.into());
            }
        }
    }

    if redeem_treasury.amount != 0 {
        return Err(MetaplexError::TreasuryNotEmpty.into());
    }

    if redeem_treasury.owner != authority {
        return Err(MetaplexError::TreasuryOwnerNotProgram.into());
    }

    if redeem_treasury.mint != external_pricing_lookup.price_mint {
        return Err(MetaplexError::RedeemTreasuryMintMustMatchLookupMint.into());
    }

    if redeem_treasury.mint == *fraction_mint_info.key {
        return Err(MetaplexError::RedeemTreasuryCantShareSameMintAsFraction.into());
    }

    if fraction_treasury.amount != 0 {
        return Err(MetaplexError::TreasuryNotEmpty.into());
    }

    if fraction_treasury.owner != authority {
        return Err(MetaplexError::TreasuryOwnerNotProgram.into());
    }

    if fraction_treasury.mint != *fraction_mint_info.key {
        return Err(MetaplexError::MetaplexTreasuryMintDoesNotMatchMetaplexMint.into());
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
    vault.state = MetaplexState::Inactive;

    vault.serialize(&mut *vault_info.data.borrow_mut())?;

    Ok(())
}
