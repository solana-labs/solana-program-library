//! Program state processor

#![cfg(feature = "program")]

use crate::{
    error::TokenError,
    instruction::{is_valid_signer_index, AuthorityType, TokenInstruction},
    option::COption,
    pack::{IsInitialized, Pack},
    state::{Account, AccountState, Mint, Multisig},
};
use num_traits::FromPrimitive;
use solana_sdk::{
    account_info::{next_account_info, AccountInfo},
    decode_error::DecodeError,
    entrypoint::ProgramResult,
    info,
    program_error::{PrintProgramError, ProgramError},
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
};
use std::mem::size_of;

/// Program state handler.
pub struct Processor {}
impl Processor {
    /// Processes an [InitializeMint](enum.TokenInstruction.html) instruction.
    pub fn process_initialize_mint(
        accounts: &[AccountInfo],
        decimals: u8,
        mint_authority: Pubkey,
        freeze_authority: COption<Pubkey>,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let mint_info = next_account_info(account_info_iter)?;
        let mint_data_len = mint_info.data_len();
        let rent = &Rent::from_account_info(next_account_info(account_info_iter)?)?;

        let mut mint_data = mint_info.data.borrow_mut();
        Mint::unpack_unchecked_mut(&mut mint_data, &mut |mint: &mut Mint| {
            if mint.is_initialized {
                return Err(TokenError::AlreadyInUse.into());
            }

            if !rent.is_exempt(mint_info.lamports(), mint_data_len) {
                return Err(TokenError::NotRentExempt.into());
            }

            mint.mint_authority = COption::Some(mint_authority);
            mint.decimals = decimals;
            mint.is_initialized = true;
            mint.freeze_authority = freeze_authority;

            Ok(())
        })
    }

    /// Processes an [InitializeAccount](enum.TokenInstruction.html) instruction.
    pub fn process_initialize_account(accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let new_account_info = next_account_info(account_info_iter)?;
        let mint_info = next_account_info(account_info_iter)?;
        let owner_info = next_account_info(account_info_iter)?;
        let new_account_info_data_len = new_account_info.data_len();
        let rent = &Rent::from_account_info(next_account_info(account_info_iter)?)?;

        let mut new_account_data = new_account_info.data.borrow_mut();
        Account::unpack_unchecked_mut(&mut new_account_data, &mut |account: &mut Account| {
            if account.is_initialized() {
                return Err(TokenError::AlreadyInUse.into());
            }

            if !rent.is_exempt(new_account_info.lamports(), new_account_info_data_len) {
                return Err(TokenError::NotRentExempt.into());
            }

            if *mint_info.key != crate::native_mint::id() {
                let mut mint_info_data = mint_info.data.borrow_mut();
                Mint::unpack_mut(&mut mint_info_data, &mut |_| Ok(()))
                    .map_err(|_| Into::<ProgramError>::into(TokenError::InvalidMint))?;
            }

            account.mint = *mint_info.key;
            account.owner = *owner_info.key;
            account.delegate = COption::None;
            account.delegated_amount = 0;
            account.state = AccountState::Initialized;
            if *mint_info.key == crate::native_mint::id() {
                let rent_exempt_reserve = rent.minimum_balance(new_account_info_data_len);
                account.is_native = COption::Some(rent_exempt_reserve);
                account.amount = new_account_info
                    .lamports()
                    .checked_sub(rent_exempt_reserve)
                    .ok_or(TokenError::Overflow)?;
            } else {
                account.is_native = COption::None;
                account.amount = 0;
            };

            Ok(())
        })
    }

    /// Processes a [InitializeMultisig](enum.TokenInstruction.html) instruction.
    pub fn process_initialize_multisig(accounts: &[AccountInfo], m: u8) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let multisig_info = next_account_info(account_info_iter)?;
        let multisig_info_data_len = multisig_info.data_len();
        let rent = &Rent::from_account_info(next_account_info(account_info_iter)?)?;

        let mut multisig_account_data = multisig_info.data.borrow_mut();
        Multisig::unpack_unchecked_mut(
            &mut multisig_account_data,
            &mut |multisig: &mut Multisig| {
                if multisig.is_initialized {
                    return Err(TokenError::AlreadyInUse.into());
                }

                if !rent.is_exempt(multisig_info.lamports(), multisig_info_data_len) {
                    return Err(TokenError::NotRentExempt.into());
                }

                let signer_infos = account_info_iter.as_slice();
                multisig.m = m;
                multisig.n = signer_infos.len() as u8;
                if !is_valid_signer_index(multisig.n as usize) {
                    return Err(TokenError::InvalidNumberOfProvidedSigners.into());
                }
                if !is_valid_signer_index(multisig.m as usize) {
                    return Err(TokenError::InvalidNumberOfRequiredSigners.into());
                }
                for (i, signer_info) in signer_infos.iter().enumerate() {
                    multisig.signers[i] = *signer_info.key;
                }
                multisig.is_initialized = true;

                Ok(())
            },
        )
    }

    /// Processes a [Transfer](enum.TokenInstruction.html) instruction.
    pub fn process_transfer(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
        expected_decimals: Option<u8>,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let source_account_info = next_account_info(account_info_iter)?;

        let expected_mint_info = if let Some(expected_decimals) = expected_decimals {
            Some((next_account_info(account_info_iter)?, expected_decimals))
        } else {
            None
        };

        let dest_account_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;

        if source_account_info.key == dest_account_info.key {
            return Ok(());
        }

        let mut source_data = source_account_info.data.borrow_mut();
        let mut dest_data = dest_account_info.data.borrow_mut();
        Account::unpack_mut(&mut source_data, &mut |source_account: &mut Account| {
            Account::unpack_mut(&mut dest_data, &mut |dest_account: &mut Account| {
                if source_account.amount < amount {
                    return Err(TokenError::InsufficientFunds.into());
                }
                if source_account.mint != dest_account.mint {
                    return Err(TokenError::MintMismatch.into());
                }
                if source_account.is_frozen() || dest_account.is_frozen() {
                    return Err(TokenError::AccountFrozen.into());
                }

                if let Some((mint_account_info, expected_decimals)) = expected_mint_info {
                    if source_account.mint != *mint_account_info.key {
                        return Err(TokenError::MintMismatch.into());
                    }

                    let mut mint_info_data = mint_account_info.data.borrow_mut();
                    Mint::unpack_mut(&mut mint_info_data, &mut |mint: &mut Mint| {
                        if expected_decimals != mint.decimals {
                            return Err(TokenError::MintDecimalsMismatch.into());
                        }
                        Ok(())
                    })?;
                }

                match source_account.delegate {
                    COption::Some(ref delegate) if authority_info.key == delegate => {
                        Self::validate_owner(
                            program_id,
                            delegate,
                            authority_info,
                            account_info_iter.as_slice(),
                        )?;
                        if source_account.delegated_amount < amount {
                            return Err(TokenError::InsufficientFunds.into());
                        }
                        source_account.delegated_amount = source_account
                            .delegated_amount
                            .checked_sub(amount)
                            .ok_or(TokenError::Overflow)?;
                        if source_account.delegated_amount == 0 {
                            source_account.delegate = COption::None;
                        }
                    }
                    _ => Self::validate_owner(
                        program_id,
                        &source_account.owner,
                        authority_info,
                        account_info_iter.as_slice(),
                    )?,
                };

                source_account.amount = source_account
                    .amount
                    .checked_sub(amount)
                    .ok_or(TokenError::Overflow)?;
                dest_account.amount = dest_account
                    .amount
                    .checked_add(amount)
                    .ok_or(TokenError::Overflow)?;

                if source_account.is_native() {
                    let source_starting_lamports = source_account_info.lamports();
                    **source_account_info.lamports.borrow_mut() = source_starting_lamports
                        .checked_sub(amount)
                        .ok_or(TokenError::Overflow)?;

                    let dest_starting_lamports = dest_account_info.lamports();
                    **dest_account_info.lamports.borrow_mut() = dest_starting_lamports
                        .checked_add(amount)
                        .ok_or(TokenError::Overflow)?;
                }

                Ok(())
            })
        })
    }

    /// Processes an [Approve](enum.TokenInstruction.html) instruction.
    pub fn process_approve(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
        expected_decimals: Option<u8>,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let source_account_info = next_account_info(account_info_iter)?;
        let expected_mint_info = if let Some(expected_decimals) = expected_decimals {
            Some((next_account_info(account_info_iter)?, expected_decimals))
        } else {
            None
        };
        let delegate_info = next_account_info(account_info_iter)?;
        let owner_info = next_account_info(account_info_iter)?;

        let mut source_data = source_account_info.data.borrow_mut();
        Account::unpack_mut(&mut source_data, &mut |source_account: &mut Account| {
            if source_account.is_frozen() {
                return Err(TokenError::AccountFrozen.into());
            }

            if let Some((mint_account_info, expected_decimals)) = expected_mint_info {
                if source_account.mint != *mint_account_info.key {
                    return Err(TokenError::MintMismatch.into());
                }

                let mut mint_info_data = mint_account_info.data.borrow_mut();
                Mint::unpack_mut(&mut mint_info_data, &mut |mint: &mut Mint| {
                    if expected_decimals != mint.decimals {
                        return Err(TokenError::MintDecimalsMismatch.into());
                    }
                    Ok(())
                })?;
            }

            Self::validate_owner(
                program_id,
                &source_account.owner,
                owner_info,
                account_info_iter.as_slice(),
            )?;

            source_account.delegate = COption::Some(*delegate_info.key);
            source_account.delegated_amount = amount;

            Ok(())
        })
    }

    /// Processes an [Revoke](enum.TokenInstruction.html) instruction.
    pub fn process_revoke(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;

        let mut source_data = source_account_info.data.borrow_mut();
        Account::unpack_mut(&mut source_data, &mut |source_account: &mut Account| {
            let owner_info = next_account_info(account_info_iter)?;

            if source_account.is_frozen() {
                return Err(TokenError::AccountFrozen.into());
            }

            Self::validate_owner(
                program_id,
                &source_account.owner,
                owner_info,
                account_info_iter.as_slice(),
            )?;

            source_account.delegate = COption::None;
            source_account.delegated_amount = 0;

            Ok(())
        })
    }

    /// Processes a [SetAuthority](enum.TokenInstruction.html) instruction.
    pub fn process_set_authority(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        authority_type: AuthorityType,
        new_authority: COption<Pubkey>,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let account_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;

        if account_info.data_len() == size_of::<Account>() {
            let mut account_data = account_info.data.borrow_mut();
            Account::unpack_mut(&mut account_data, &mut |account: &mut Account| {
                if account.is_frozen() {
                    return Err(TokenError::AccountFrozen.into());
                }

                match authority_type {
                    AuthorityType::AccountOwner => {
                        Self::validate_owner(
                            program_id,
                            &account.owner,
                            authority_info,
                            account_info_iter.as_slice(),
                        )?;

                        if let COption::Some(authority) = new_authority {
                            account.owner = authority;
                        } else {
                            return Err(TokenError::InvalidInstruction.into());
                        }
                    }
                    AuthorityType::CloseAccount => {
                        let authority = account.close_authority.unwrap_or(account.owner);
                        Self::validate_owner(
                            program_id,
                            &authority,
                            authority_info,
                            account_info_iter.as_slice(),
                        )?;
                        account.close_authority = new_authority;
                    }
                    _ => {
                        return Err(TokenError::AuthorityTypeNotSupported.into());
                    }
                }
                Ok(())
            })?;
        } else if account_info.data_len() == size_of::<Mint>() {
            let mut mint_data = account_info.data.borrow_mut();
            Mint::unpack_mut(&mut mint_data, &mut |mint: &mut Mint| {
                match authority_type {
                    AuthorityType::MintTokens => {
                        // Once a mint's supply is fixed, it cannot be undone by setting a new
                        // mint_authority
                        let mint_authority = mint
                            .mint_authority
                            .ok_or(Into::<ProgramError>::into(TokenError::FixedSupply))?;
                        Self::validate_owner(
                            program_id,
                            &mint_authority,
                            authority_info,
                            account_info_iter.as_slice(),
                        )?;
                        mint.mint_authority = new_authority;
                    }
                    AuthorityType::FreezeAccount => {
                        // Once a mint's freeze authority is disabled, it cannot be re-enabled by
                        // setting a new freeze_authority
                        let freeze_authority = mint
                            .freeze_authority
                            .ok_or(Into::<ProgramError>::into(TokenError::MintCannotFreeze))?;
                        Self::validate_owner(
                            program_id,
                            &freeze_authority,
                            authority_info,
                            account_info_iter.as_slice(),
                        )?;
                        mint.freeze_authority = new_authority;
                    }
                    _ => {
                        return Err(TokenError::AuthorityTypeNotSupported.into());
                    }
                }
                Ok(())
            })?;
        } else {
            return Err(ProgramError::InvalidArgument);
        }

        Ok(())
    }

    /// Processes a [MintTo](enum.TokenInstruction.html) instruction.
    pub fn process_mint_to(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
        expected_decimals: Option<u8>,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let mint_info = next_account_info(account_info_iter)?;
        let dest_account_info = next_account_info(account_info_iter)?;
        let owner_info = next_account_info(account_info_iter)?;

        let mut dest_account_data = dest_account_info.data.borrow_mut();
        Account::unpack_mut(&mut dest_account_data, &mut |dest_account: &mut Account| {
            if dest_account.is_frozen() {
                return Err(TokenError::AccountFrozen.into());
            }

            if dest_account.is_native() {
                return Err(TokenError::NativeNotSupported.into());
            }
            if mint_info.key != &dest_account.mint {
                return Err(TokenError::MintMismatch.into());
            }

            let mut mint_info_data = mint_info.data.borrow_mut();
            Mint::unpack_mut(&mut mint_info_data, &mut |mint: &mut Mint| {
                if let Some(expected_decimals) = expected_decimals {
                    if expected_decimals != mint.decimals {
                        return Err(TokenError::MintDecimalsMismatch.into());
                    }
                }

                match mint.mint_authority {
                    COption::Some(mint_authority) => Self::validate_owner(
                        program_id,
                        &mint_authority,
                        owner_info,
                        account_info_iter.as_slice(),
                    )?,
                    COption::None => return Err(TokenError::FixedSupply.into()),
                }

                dest_account.amount = dest_account
                    .amount
                    .checked_add(amount)
                    .ok_or(TokenError::Overflow)?;

                mint.supply = mint
                    .supply
                    .checked_add(amount)
                    .ok_or(TokenError::Overflow)?;

                Ok(())
            })
        })
    }

    /// Processes a [Burn](enum.TokenInstruction.html) instruction.
    pub fn process_burn(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
        expected_decimals: Option<u8>,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let source_account_info = next_account_info(account_info_iter)?;
        let mint_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;

        let mut mint_data = mint_info.data.borrow_mut();
        let mut source_data = source_account_info.data.borrow_mut();
        Mint::unpack_mut(&mut mint_data, &mut |mint: &mut Mint| {
            Account::unpack_mut(&mut source_data, &mut |source_account: &mut Account| {
                if source_account.is_native() {
                    return Err(TokenError::NativeNotSupported.into());
                }
                if mint_info.key != &source_account.mint {
                    return Err(TokenError::MintMismatch.into());
                }
                if source_account.amount < amount {
                    return Err(TokenError::InsufficientFunds.into());
                }
                if source_account.is_frozen() {
                    return Err(TokenError::AccountFrozen.into());
                }

                if let Some(expected_decimals) = expected_decimals {
                    if expected_decimals != mint.decimals {
                        return Err(TokenError::MintDecimalsMismatch.into());
                    }
                }

                match source_account.delegate {
                    COption::Some(ref delegate) if authority_info.key == delegate => {
                        Self::validate_owner(
                            program_id,
                            delegate,
                            authority_info,
                            account_info_iter.as_slice(),
                        )?;

                        if source_account.delegated_amount < amount {
                            return Err(TokenError::InsufficientFunds.into());
                        }
                        source_account.delegated_amount = source_account
                            .delegated_amount
                            .checked_sub(amount)
                            .ok_or(TokenError::Overflow)?;
                        if source_account.delegated_amount == 0 {
                            source_account.delegate = COption::None;
                        }
                    }
                    _ => Self::validate_owner(
                        program_id,
                        &source_account.owner,
                        authority_info,
                        account_info_iter.as_slice(),
                    )?,
                }

                source_account.amount = source_account
                    .amount
                    .checked_sub(amount)
                    .ok_or(TokenError::Overflow)?;
                mint.supply = mint
                    .supply
                    .checked_sub(amount)
                    .ok_or(TokenError::Overflow)?;

                Ok(())
            })
        })
    }

    /// Processes a [CloseAccount](enum.TokenInstruction.html) instruction.
    pub fn process_close_account(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let dest_account_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;

        let mut source_data = source_account_info.data.borrow_mut();
        Account::unpack_mut(&mut source_data, &mut |source_account: &mut Account| {
            if !source_account.is_native() && source_account.amount != 0 {
                return Err(TokenError::NonNativeHasBalance.into());
            }

            let authority = source_account
                .close_authority
                .unwrap_or(source_account.owner);
            Self::validate_owner(
                program_id,
                &authority,
                authority_info,
                account_info_iter.as_slice(),
            )?;

            let dest_starting_lamports = dest_account_info.lamports();
            **dest_account_info.lamports.borrow_mut() = dest_starting_lamports
                .checked_add(source_account_info.lamports())
                .ok_or(TokenError::Overflow)?;

            **source_account_info.lamports.borrow_mut() = 0;
            source_account.amount = 0;

            Ok(())
        })
    }

    /// Processes a [FreezeAccount](enum.TokenInstruction.html) or a
    /// [ThawAccount](enum.TokenInstruction.html) instruction.
    pub fn process_toggle_freeze_account(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        freeze: bool,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let mint_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;

        let mut source_data = source_account_info.data.borrow_mut();
        Account::unpack_mut(&mut source_data, &mut |source_account: &mut Account| {
            if source_account.is_native() {
                return Err(TokenError::NativeNotSupported.into());
            }
            if mint_info.key != &source_account.mint {
                return Err(TokenError::MintMismatch.into());
            }
            if freeze && source_account.is_frozen() || !freeze && !source_account.is_frozen() {
                return Err(TokenError::InvalidState.into());
            }

            let mut mint_data = mint_info.data.borrow_mut();
            Mint::unpack_mut(
                &mut mint_data,
                &mut |mint: &mut Mint| match mint.freeze_authority {
                    COption::Some(authority) => Self::validate_owner(
                        program_id,
                        &authority,
                        authority_info,
                        account_info_iter.as_slice(),
                    ),
                    COption::None => Err(TokenError::MintCannotFreeze.into()),
                },
            )?;

            source_account.state = if freeze {
                AccountState::Frozen
            } else {
                AccountState::Initialized
            };

            Ok(())
        })
    }

    /// Processes an [Instruction](enum.Instruction.html).
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        let instruction = TokenInstruction::unpack(input)?;

        match instruction {
            TokenInstruction::InitializeMint {
                decimals,
                mint_authority,
                freeze_authority,
            } => {
                info!("Instruction: InitializeMint");
                Self::process_initialize_mint(accounts, decimals, mint_authority, freeze_authority)
            }
            TokenInstruction::InitializeAccount => {
                info!("Instruction: InitializeAccount");
                Self::process_initialize_account(accounts)
            }
            TokenInstruction::InitializeMultisig { m } => {
                info!("Instruction: InitializeMultisig");
                Self::process_initialize_multisig(accounts, m)
            }
            TokenInstruction::Transfer { amount } => {
                info!("Instruction: Transfer");
                Self::process_transfer(program_id, accounts, amount, None)
            }
            TokenInstruction::Approve { amount } => {
                info!("Instruction: Approve");
                Self::process_approve(program_id, accounts, amount, None)
            }
            TokenInstruction::Revoke => {
                info!("Instruction: Revoke");
                Self::process_revoke(program_id, accounts)
            }
            TokenInstruction::SetAuthority {
                authority_type,
                new_authority,
            } => {
                info!("Instruction: SetAuthority");
                Self::process_set_authority(program_id, accounts, authority_type, new_authority)
            }
            TokenInstruction::MintTo { amount } => {
                info!("Instruction: MintTo");
                Self::process_mint_to(program_id, accounts, amount, None)
            }
            TokenInstruction::Burn { amount } => {
                info!("Instruction: Burn");
                Self::process_burn(program_id, accounts, amount, None)
            }
            TokenInstruction::CloseAccount => {
                info!("Instruction: CloseAccount");
                Self::process_close_account(program_id, accounts)
            }
            TokenInstruction::FreezeAccount => {
                info!("Instruction: FreezeAccount");
                Self::process_toggle_freeze_account(program_id, accounts, true)
            }
            TokenInstruction::ThawAccount => {
                info!("Instruction: FreezeAccount");
                Self::process_toggle_freeze_account(program_id, accounts, false)
            }
            TokenInstruction::Transfer2 { amount, decimals } => {
                info!("Instruction: Transfer");
                Self::process_transfer(program_id, accounts, amount, Some(decimals))
            }
            TokenInstruction::Approve2 { amount, decimals } => {
                info!("Instruction: Approve");
                Self::process_approve(program_id, accounts, amount, Some(decimals))
            }
            TokenInstruction::MintTo2 { amount, decimals } => {
                info!("Instruction: MintTo");
                Self::process_mint_to(program_id, accounts, amount, Some(decimals))
            }
            TokenInstruction::Burn2 { amount, decimals } => {
                info!("Instruction: Burn");
                Self::process_burn(program_id, accounts, amount, Some(decimals))
            }
        }
    }

    /// Validates owner(s) are present
    pub fn validate_owner(
        program_id: &Pubkey,
        expected_owner: &Pubkey,
        owner_account_info: &AccountInfo,
        signers: &[AccountInfo],
    ) -> ProgramResult {
        if expected_owner != owner_account_info.key {
            return Err(TokenError::OwnerMismatch.into());
        }
        if program_id == owner_account_info.owner
            && owner_account_info.data_len() == std::mem::size_of::<Multisig>()
        {
            let mut owner_data = owner_account_info.data.borrow_mut();
            Multisig::unpack_mut(&mut owner_data, &mut |multisig: &mut Multisig| {
                let mut num_signers = 0;
                for signer in signers.iter() {
                    if multisig.signers[0..multisig.n as usize].contains(signer.key) {
                        if !signer.is_signer {
                            return Err(ProgramError::MissingRequiredSignature);
                        }
                        num_signers += 1;
                    }
                }
                if num_signers < multisig.m {
                    return Err(ProgramError::MissingRequiredSignature);
                }
                Ok(())
            })?;
        } else if !owner_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        Ok(())
    }
}

impl PrintProgramError for TokenError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            TokenError::NotRentExempt => {
                info!("Error: Lamport balance below rent-exempt threshold")
            }
            TokenError::InsufficientFunds => info!("Error: insufficient funds"),
            TokenError::InvalidMint => info!("Error: Invalid Mint"),
            TokenError::MintMismatch => info!("Error: Account not associated with this Mint"),
            TokenError::OwnerMismatch => info!("Error: owner does not match"),
            TokenError::FixedSupply => info!("Error: the total supply of this token is fixed"),
            TokenError::AlreadyInUse => info!("Error: account or token already in use"),
            TokenError::InvalidNumberOfProvidedSigners => {
                info!("Error: Invalid number of provided signers")
            }
            TokenError::InvalidNumberOfRequiredSigners => {
                info!("Error: Invalid number of required signers")
            }
            TokenError::UninitializedState => info!("Error: State is uninitialized"),
            TokenError::NativeNotSupported => {
                info!("Error: Instruction does not support native tokens")
            }
            TokenError::NonNativeHasBalance => {
                info!("Error: Non-native account can only be closed if its balance is zero")
            }
            TokenError::InvalidInstruction => info!("Error: Invalid instruction"),
            TokenError::InvalidState => info!("Error: Invalid account state for operation"),
            TokenError::Overflow => info!("Error: Operation overflowed"),
            TokenError::AuthorityTypeNotSupported => {
                info!("Error: Account does not support specified authority type")
            }
            TokenError::MintCannotFreeze => info!("Error: This token mint cannot freeze accounts"),
            TokenError::AccountFrozen => info!("Error: Account is frozen"),
            TokenError::MintDecimalsMismatch => {
                info!("Error: decimals different from the Mint decimals")
            }
        }
    }
}

// Pull in syscall stubs when building for non-BPF targets
#[cfg(not(target_arch = "bpf"))]
solana_sdk::program_stubs!();

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::*;
    use solana_sdk::{
        account::Account as SolanaAccount, account_info::create_is_signer_account_infos,
        clock::Epoch, instruction::Instruction, sysvar::rent,
    };

    fn pubkey_rand() -> Pubkey {
        Pubkey::new(&rand::random::<[u8; 32]>())
    }

    fn do_process_instruction(
        instruction: Instruction,
        accounts: Vec<&mut SolanaAccount>,
    ) -> ProgramResult {
        let mut meta = instruction
            .accounts
            .iter()
            .zip(accounts)
            .map(|(account_meta, account)| (&account_meta.pubkey, account_meta.is_signer, account))
            .collect::<Vec<_>>();

        let account_infos = create_is_signer_account_infos(&mut meta);
        Processor::process(&instruction.program_id, &account_infos, &instruction.data)
    }

    fn return_token_error_as_program_error() -> ProgramError {
        TokenError::MintMismatch.into()
    }

    fn rent_sysvar() -> SolanaAccount {
        rent::create_account(42, &Rent::default())
    }

    fn mint_minimum_balance() -> u64 {
        Rent::default().minimum_balance(size_of::<Mint>())
    }

    fn account_minimum_balance() -> u64 {
        Rent::default().minimum_balance(size_of::<Account>())
    }

    fn multisig_minimum_balance() -> u64 {
        Rent::default().minimum_balance(size_of::<Multisig>())
    }

    #[test]
    fn test_print_error() {
        let error = return_token_error_as_program_error();
        error.print::<TokenError>();
    }

    #[test]
    #[should_panic(expected = "Custom(3)")]
    fn test_error_unwrap() {
        Err::<(), ProgramError>(return_token_error_as_program_error()).unwrap();
    }

    #[test]
    fn test_unique_account_sizes() {
        assert_ne!(size_of::<Mint>(), 0);
        assert_ne!(size_of::<Mint>(), size_of::<Account>());
        assert_ne!(size_of::<Mint>(), size_of::<Multisig>());
        assert_ne!(size_of::<Account>(), 0);
        assert_ne!(size_of::<Account>(), size_of::<Multisig>());
        assert_ne!(size_of::<Multisig>(), 0);
    }

    #[test]
    fn test_initialize_mint() {
        let program_id = pubkey_rand();
        let owner_key = pubkey_rand();
        let mint_key = pubkey_rand();
        let mut mint_account = SolanaAccount::new(42, size_of::<Mint>(), &program_id);
        let mint2_key = pubkey_rand();
        let mut mint2_account =
            SolanaAccount::new(mint_minimum_balance(), size_of::<Mint>(), &program_id);
        let mut rent_sysvar = rent_sysvar();

        // mint is not rent exempt
        assert_eq!(
            Err(TokenError::NotRentExempt.into()),
            do_process_instruction(
                initialize_mint(&program_id, &mint_key, &owner_key, None, 2).unwrap(),
                vec![&mut mint_account, &mut rent_sysvar]
            )
        );

        mint_account.lamports = mint_minimum_balance();

        // create new mint
        do_process_instruction(
            initialize_mint(&program_id, &mint_key, &owner_key, None, 2).unwrap(),
            vec![&mut mint_account, &mut rent_sysvar],
        )
        .unwrap();

        // create twice
        assert_eq!(
            Err(TokenError::AlreadyInUse.into()),
            do_process_instruction(
                initialize_mint(&program_id, &mint_key, &owner_key, None, 2,).unwrap(),
                vec![&mut mint_account, &mut rent_sysvar]
            )
        );

        // create another mint that can freeze
        do_process_instruction(
            initialize_mint(&program_id, &mint2_key, &owner_key, Some(&owner_key), 2).unwrap(),
            vec![&mut mint2_account, &mut rent_sysvar],
        )
        .unwrap();
        Mint::unpack_unchecked_mut(&mut mint2_account.data, &mut |mint: &mut Mint| {
            assert_eq!(mint.freeze_authority, COption::Some(owner_key));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_initialize_mint_account() {
        let program_id = pubkey_rand();
        let account_key = pubkey_rand();
        let mut account_account = SolanaAccount::new(42, size_of::<Account>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = SolanaAccount::default();
        let mint_key = pubkey_rand();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), size_of::<Mint>(), &program_id);
        let mut rent_sysvar = rent_sysvar();

        // account is not rent exempt
        assert_eq!(
            Err(TokenError::NotRentExempt.into()),
            do_process_instruction(
                initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
                vec![
                    &mut account_account,
                    &mut mint_account,
                    &mut owner_account,
                    &mut rent_sysvar
                ],
            )
        );

        account_account.lamports = account_minimum_balance();

        // mint is not valid (not initialized)
        assert_eq!(
            Err(TokenError::InvalidMint.into()),
            do_process_instruction(
                initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
                vec![
                    &mut account_account,
                    &mut mint_account,
                    &mut owner_account,
                    &mut rent_sysvar
                ],
            )
        );

        // create mint
        do_process_instruction(
            initialize_mint(&program_id, &mint_key, &owner_key, None, 2).unwrap(),
            vec![&mut mint_account, &mut rent_sysvar],
        )
        .unwrap();

        // create account
        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
            vec![
                &mut account_account,
                &mut mint_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();

        // create twice
        assert_eq!(
            Err(TokenError::AlreadyInUse.into()),
            do_process_instruction(
                initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
                vec![
                    &mut account_account,
                    &mut mint_account,
                    &mut owner_account,
                    &mut rent_sysvar
                ],
            )
        );
    }

    #[test]
    fn test_transfer() {
        let program_id = pubkey_rand();
        let account_key = pubkey_rand();
        let mut account_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let account2_key = pubkey_rand();
        let mut account2_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let account3_key = pubkey_rand();
        let mut account3_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let delegate_key = pubkey_rand();
        let mut delegate_account = SolanaAccount::default();
        let mismatch_key = pubkey_rand();
        let mut mismatch_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = SolanaAccount::default();
        let owner2_key = pubkey_rand();
        let mut owner2_account = SolanaAccount::default();
        let mint_key = pubkey_rand();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), size_of::<Mint>(), &program_id);
        let mint2_key = pubkey_rand();
        let mut rent_sysvar = rent_sysvar();

        // create mint
        do_process_instruction(
            initialize_mint(&program_id, &mint_key, &owner_key, None, 2).unwrap(),
            vec![&mut mint_account, &mut rent_sysvar],
        )
        .unwrap();

        // create account
        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
            vec![
                &mut account_account,
                &mut mint_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();

        // create another account
        do_process_instruction(
            initialize_account(&program_id, &account2_key, &mint_key, &owner_key).unwrap(),
            vec![
                &mut account2_account,
                &mut mint_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();

        // create another account
        do_process_instruction(
            initialize_account(&program_id, &account3_key, &mint_key, &owner_key).unwrap(),
            vec![
                &mut account3_account,
                &mut mint_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();

        // create mismatch account
        do_process_instruction(
            initialize_account(&program_id, &mismatch_key, &mint_key, &owner_key).unwrap(),
            vec![
                &mut mismatch_account,
                &mut mint_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();
        Account::unpack_unchecked_mut(&mut mismatch_account.data, &mut |account: &mut Account| {
            account.mint = mint2_key;
            Ok(())
        })
        .unwrap();

        // mint to account
        do_process_instruction(
            mint_to(&program_id, &mint_key, &account_key, &owner_key, &[], 1000).unwrap(),
            vec![&mut mint_account, &mut account_account, &mut owner_account],
        )
        .unwrap();

        // missing signer
        let mut instruction = transfer(
            &program_id,
            &account_key,
            &account2_key,
            &owner_key,
            &[],
            1000,
        )
        .unwrap();
        instruction.accounts[2].is_signer = false;
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            do_process_instruction(
                instruction,
                vec![
                    &mut account_account,
                    &mut account2_account,
                    &mut owner_account,
                ],
            )
        );

        // mismatch mint
        assert_eq!(
            Err(TokenError::MintMismatch.into()),
            do_process_instruction(
                transfer(
                    &program_id,
                    &account_key,
                    &mismatch_key,
                    &owner_key,
                    &[],
                    1000
                )
                .unwrap(),
                vec![
                    &mut account_account,
                    &mut mismatch_account,
                    &mut owner_account,
                ],
            )
        );

        // missing owner
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            do_process_instruction(
                transfer(
                    &program_id,
                    &account_key,
                    &account2_key,
                    &owner2_key,
                    &[],
                    1000
                )
                .unwrap(),
                vec![
                    &mut account_account,
                    &mut account2_account,
                    &mut owner2_account,
                ],
            )
        );

        // transfer
        do_process_instruction(
            transfer(
                &program_id,
                &account_key,
                &account2_key,
                &owner_key,
                &[],
                1000,
            )
            .unwrap(),
            vec![
                &mut account_account,
                &mut account2_account,
                &mut owner_account,
            ],
        )
        .unwrap();

        // insufficient funds
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            do_process_instruction(
                transfer(&program_id, &account_key, &account2_key, &owner_key, &[], 1).unwrap(),
                vec![
                    &mut account_account,
                    &mut account2_account,
                    &mut owner_account,
                ],
            )
        );

        // transfer half back
        do_process_instruction(
            transfer(
                &program_id,
                &account2_key,
                &account_key,
                &owner_key,
                &[],
                500,
            )
            .unwrap(),
            vec![
                &mut account2_account,
                &mut account_account,
                &mut owner_account,
            ],
        )
        .unwrap();

        // incorrect decimals
        assert_eq!(
            Err(TokenError::MintDecimalsMismatch.into()),
            do_process_instruction(
                transfer2(
                    &program_id,
                    &account2_key,
                    &mint_key,
                    &account_key,
                    &owner_key,
                    &[],
                    1,
                    10 // <-- incorrect decimals
                )
                .unwrap(),
                vec![
                    &mut account2_account,
                    &mut mint_account,
                    &mut account_account,
                    &mut owner_account,
                ],
            )
        );

        // incorrect mint
        assert_eq!(
            Err(TokenError::MintMismatch.into()),
            do_process_instruction(
                transfer2(
                    &program_id,
                    &account2_key,
                    &account3_key, // <-- incorrect mint
                    &account_key,
                    &owner_key,
                    &[],
                    1,
                    2
                )
                .unwrap(),
                vec![
                    &mut account2_account,
                    &mut account3_account, // <-- incorrect mint
                    &mut account_account,
                    &mut owner_account,
                ],
            )
        );
        // transfer rest with explicit decimals
        do_process_instruction(
            transfer2(
                &program_id,
                &account2_key,
                &mint_key,
                &account_key,
                &owner_key,
                &[],
                500,
                2,
            )
            .unwrap(),
            vec![
                &mut account2_account,
                &mut mint_account,
                &mut account_account,
                &mut owner_account,
            ],
        )
        .unwrap();

        // transfer to self
        {
            let instruction = transfer(
                &program_id,
                &account_key,
                &account_key,
                &owner_key,
                &[],
                500,
            )
            .unwrap();
            let account_account_info = AccountInfo::from((
                &instruction.accounts[0].pubkey,
                instruction.accounts[0].is_signer,
                &mut account_account,
            ));
            let owner_account_info = AccountInfo::from((
                &instruction.accounts[2].pubkey,
                instruction.accounts[2].is_signer,
                &mut owner_account,
            ));
            Processor::process(
                &instruction.program_id,
                &[
                    account_account_info.clone(),
                    account_account_info,
                    owner_account_info,
                ],
                &instruction.data,
            )
            .unwrap()
        }
        Account::unpack_unchecked_mut(&mut account_account.data, &mut |account: &mut Account| {
            assert_eq!(account.amount, 1000);
            Ok(())
        })
        .unwrap();

        // insufficient funds
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            do_process_instruction(
                transfer(&program_id, &account2_key, &account_key, &owner_key, &[], 1).unwrap(),
                vec![
                    &mut account2_account,
                    &mut account_account,
                    &mut owner_account,
                ],
            )
        );

        // approve delegate
        do_process_instruction(
            approve(
                &program_id,
                &account_key,
                &delegate_key,
                &owner_key,
                &[],
                100,
            )
            .unwrap(),
            vec![
                &mut account_account,
                &mut delegate_account,
                &mut owner_account,
            ],
        )
        .unwrap();

        // transfer via delegate
        do_process_instruction(
            transfer(
                &program_id,
                &account_key,
                &account2_key,
                &delegate_key,
                &[],
                100,
            )
            .unwrap(),
            vec![
                &mut account_account,
                &mut account2_account,
                &mut delegate_account,
            ],
        )
        .unwrap();

        // insufficient funds approved via delegate
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            do_process_instruction(
                transfer(
                    &program_id,
                    &account_key,
                    &account2_key,
                    &delegate_key,
                    &[],
                    100
                )
                .unwrap(),
                vec![
                    &mut account_account,
                    &mut account2_account,
                    &mut delegate_account,
                ],
            )
        );

        // transfer rest
        do_process_instruction(
            transfer(
                &program_id,
                &account_key,
                &account2_key,
                &owner_key,
                &[],
                900,
            )
            .unwrap(),
            vec![
                &mut account_account,
                &mut account2_account,
                &mut owner_account,
            ],
        )
        .unwrap();

        // approve delegate
        do_process_instruction(
            approve(
                &program_id,
                &account_key,
                &delegate_key,
                &owner_key,
                &[],
                100,
            )
            .unwrap(),
            vec![
                &mut account_account,
                &mut delegate_account,
                &mut owner_account,
            ],
        )
        .unwrap();

        // insufficient funds in source account via delegate
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            do_process_instruction(
                transfer(
                    &program_id,
                    &account_key,
                    &account2_key,
                    &delegate_key,
                    &[],
                    100
                )
                .unwrap(),
                vec![
                    &mut account_account,
                    &mut account2_account,
                    &mut delegate_account,
                ],
            )
        );
    }

    #[test]
    fn test_mintable_token_with_zero_supply() {
        let program_id = pubkey_rand();
        let account_key = pubkey_rand();
        let mut account_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = SolanaAccount::default();
        let mint_key = pubkey_rand();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), size_of::<Mint>(), &program_id);
        let mut rent_sysvar = rent_sysvar();

        // create mint-able token with zero supply
        let decimals = 2;
        do_process_instruction(
            initialize_mint(&program_id, &mint_key, &owner_key, None, decimals).unwrap(),
            vec![&mut mint_account, &mut rent_sysvar],
        )
        .unwrap();
        Mint::unpack_unchecked_mut(&mut mint_account.data, &mut |mint: &mut Mint| {
            assert_eq!(
                *mint,
                Mint {
                    mint_authority: COption::Some(owner_key),
                    supply: 0,
                    decimals,
                    is_initialized: true,
                    freeze_authority: COption::None,
                }
            );
            Ok(())
        })
        .unwrap();

        // create account
        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
            vec![
                &mut account_account,
                &mut mint_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();

        // mint to
        do_process_instruction(
            mint_to(&program_id, &mint_key, &account_key, &owner_key, &[], 42).unwrap(),
            vec![&mut mint_account, &mut account_account, &mut owner_account],
        )
        .unwrap();
        Mint::unpack_unchecked_mut(&mut mint_account.data, &mut |_| Ok(())).unwrap();
        Account::unpack_unchecked_mut(&mut account_account.data, &mut |account: &mut Account| {
            assert_eq!(account.amount, 42);
            Ok(())
        })
        .unwrap();

        // mint to 2, with incorrect decimals
        assert_eq!(
            Err(TokenError::MintDecimalsMismatch.into()),
            do_process_instruction(
                mint_to2(
                    &program_id,
                    &mint_key,
                    &account_key,
                    &owner_key,
                    &[],
                    42,
                    decimals + 1
                )
                .unwrap(),
                vec![&mut mint_account, &mut account_account, &mut owner_account],
            )
        );

        Mint::unpack_unchecked_mut(&mut mint_account.data, &mut |_| Ok(())).unwrap();
        Account::unpack_unchecked_mut(&mut account_account.data, &mut |account: &mut Account| {
            assert_eq!(account.amount, 42);
            Ok(())
        })
        .unwrap();

        // mint to 2
        do_process_instruction(
            mint_to2(
                &program_id,
                &mint_key,
                &account_key,
                &owner_key,
                &[],
                42,
                decimals,
            )
            .unwrap(),
            vec![&mut mint_account, &mut account_account, &mut owner_account],
        )
        .unwrap();
        Mint::unpack_unchecked_mut(&mut mint_account.data, &mut |_| Ok(())).unwrap();
        Account::unpack_unchecked_mut(&mut account_account.data, &mut |account: &mut Account| {
            assert_eq!(account.amount, 84);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_approve() {
        let program_id = pubkey_rand();
        let account_key = pubkey_rand();
        let mut account_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let account2_key = pubkey_rand();
        let mut account2_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let delegate_key = pubkey_rand();
        let mut delegate_account = SolanaAccount::default();
        let owner_key = pubkey_rand();
        let mut owner_account = SolanaAccount::default();
        let owner2_key = pubkey_rand();
        let mut owner2_account = SolanaAccount::default();
        let mint_key = pubkey_rand();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), size_of::<Mint>(), &program_id);
        let mut rent_sysvar = rent_sysvar();

        // create mint
        do_process_instruction(
            initialize_mint(&program_id, &mint_key, &owner_key, None, 2).unwrap(),
            vec![&mut mint_account, &mut rent_sysvar],
        )
        .unwrap();

        // create account
        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
            vec![
                &mut account_account,
                &mut mint_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();

        // create another account
        do_process_instruction(
            initialize_account(&program_id, &account2_key, &mint_key, &owner_key).unwrap(),
            vec![
                &mut account2_account,
                &mut mint_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();

        // mint to account
        do_process_instruction(
            mint_to(&program_id, &mint_key, &account_key, &owner_key, &[], 1000).unwrap(),
            vec![&mut mint_account, &mut account_account, &mut owner_account],
        )
        .unwrap();

        // missing signer
        let mut instruction = approve(
            &program_id,
            &account_key,
            &delegate_key,
            &owner_key,
            &[],
            100,
        )
        .unwrap();
        instruction.accounts[2].is_signer = false;
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            do_process_instruction(
                instruction,
                vec![
                    &mut account_account,
                    &mut delegate_account,
                    &mut owner_account,
                ],
            )
        );

        // no owner
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            do_process_instruction(
                approve(
                    &program_id,
                    &account_key,
                    &delegate_key,
                    &owner2_key,
                    &[],
                    100
                )
                .unwrap(),
                vec![
                    &mut account_account,
                    &mut delegate_account,
                    &mut owner2_account,
                ],
            )
        );

        // approve delegate
        do_process_instruction(
            approve(
                &program_id,
                &account_key,
                &delegate_key,
                &owner_key,
                &[],
                100,
            )
            .unwrap(),
            vec![
                &mut account_account,
                &mut delegate_account,
                &mut owner_account,
            ],
        )
        .unwrap();

        // approve delegate 2, with incorrect decimals
        assert_eq!(
            Err(TokenError::MintDecimalsMismatch.into()),
            do_process_instruction(
                approve2(
                    &program_id,
                    &account_key,
                    &mint_key,
                    &delegate_key,
                    &owner_key,
                    &[],
                    100,
                    0 // <-- incorrect decimals
                )
                .unwrap(),
                vec![
                    &mut account_account,
                    &mut mint_account,
                    &mut delegate_account,
                    &mut owner_account,
                ],
            )
        );

        // approve delegate 2, with incorrect mint
        assert_eq!(
            Err(TokenError::MintMismatch.into()),
            do_process_instruction(
                approve2(
                    &program_id,
                    &account_key,
                    &account2_key, // <-- bad mint
                    &delegate_key,
                    &owner_key,
                    &[],
                    100,
                    0
                )
                .unwrap(),
                vec![
                    &mut account_account,
                    &mut account2_account, // <-- bad mint
                    &mut delegate_account,
                    &mut owner_account,
                ],
            )
        );

        // approve delegate 2
        do_process_instruction(
            approve2(
                &program_id,
                &account_key,
                &mint_key,
                &delegate_key,
                &owner_key,
                &[],
                100,
                2,
            )
            .unwrap(),
            vec![
                &mut account_account,
                &mut mint_account,
                &mut delegate_account,
                &mut owner_account,
            ],
        )
        .unwrap();

        // revoke delegate
        do_process_instruction(
            revoke(&program_id, &account_key, &owner_key, &[]).unwrap(),
            vec![&mut account_account, &mut owner_account],
        )
        .unwrap();
    }

    #[test]
    fn test_set_authority() {
        let program_id = pubkey_rand();
        let account_key = pubkey_rand();
        let mut account_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let account2_key = pubkey_rand();
        let mut account2_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = SolanaAccount::default();
        let owner2_key = pubkey_rand();
        let mut owner2_account = SolanaAccount::default();
        let owner3_key = pubkey_rand();
        let mint_key = pubkey_rand();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), size_of::<Mint>(), &program_id);
        let mint2_key = pubkey_rand();
        let mut mint2_account =
            SolanaAccount::new(mint_minimum_balance(), size_of::<Mint>(), &program_id);
        let mut rent_sysvar = rent_sysvar();

        // create new mint with owner
        do_process_instruction(
            initialize_mint(&program_id, &mint_key, &owner_key, None, 2).unwrap(),
            vec![&mut mint_account, &mut rent_sysvar],
        )
        .unwrap();

        // create mint with owner and freeze_authority
        do_process_instruction(
            initialize_mint(&program_id, &mint2_key, &owner_key, Some(&owner_key), 2).unwrap(),
            vec![&mut mint2_account, &mut rent_sysvar],
        )
        .unwrap();

        // invalid account
        assert_eq!(
            Err(TokenError::UninitializedState.into()),
            do_process_instruction(
                set_authority(
                    &program_id,
                    &account_key,
                    Some(&owner2_key),
                    AuthorityType::AccountOwner,
                    &owner_key,
                    &[]
                )
                .unwrap(),
                vec![&mut account_account, &mut owner_account],
            )
        );

        // create account
        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
            vec![
                &mut account_account,
                &mut mint_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();

        // create another account
        do_process_instruction(
            initialize_account(&program_id, &account2_key, &mint2_key, &owner_key).unwrap(),
            vec![
                &mut account2_account,
                &mut mint2_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();

        // missing owner
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            do_process_instruction(
                set_authority(
                    &program_id,
                    &account_key,
                    Some(&owner_key),
                    AuthorityType::AccountOwner,
                    &owner2_key,
                    &[]
                )
                .unwrap(),
                vec![&mut account_account, &mut owner2_account],
            )
        );

        // owner did not sign
        let mut instruction = set_authority(
            &program_id,
            &account_key,
            Some(&owner2_key),
            AuthorityType::AccountOwner,
            &owner_key,
            &[],
        )
        .unwrap();
        instruction.accounts[1].is_signer = false;
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            do_process_instruction(instruction, vec![&mut account_account, &mut owner_account,],)
        );

        // wrong authority type
        assert_eq!(
            Err(TokenError::AuthorityTypeNotSupported.into()),
            do_process_instruction(
                set_authority(
                    &program_id,
                    &account_key,
                    Some(&owner2_key),
                    AuthorityType::FreezeAccount,
                    &owner_key,
                    &[],
                )
                .unwrap(),
                vec![&mut account_account, &mut owner_account],
            )
        );

        // account owner may not be set to None
        assert_eq!(
            Err(TokenError::InvalidInstruction.into()),
            do_process_instruction(
                set_authority(
                    &program_id,
                    &account_key,
                    None,
                    AuthorityType::AccountOwner,
                    &owner_key,
                    &[],
                )
                .unwrap(),
                vec![&mut account_account, &mut owner_account],
            )
        );

        // set owner
        do_process_instruction(
            set_authority(
                &program_id,
                &account_key,
                Some(&owner2_key),
                AuthorityType::AccountOwner,
                &owner_key,
                &[],
            )
            .unwrap(),
            vec![&mut account_account, &mut owner_account],
        )
        .unwrap();

        // set close_authority
        do_process_instruction(
            set_authority(
                &program_id,
                &account_key,
                Some(&owner2_key),
                AuthorityType::CloseAccount,
                &owner2_key,
                &[],
            )
            .unwrap(),
            vec![&mut account_account, &mut owner2_account],
        )
        .unwrap();

        // close_authority may be set to None
        do_process_instruction(
            set_authority(
                &program_id,
                &account_key,
                None,
                AuthorityType::CloseAccount,
                &owner2_key,
                &[],
            )
            .unwrap(),
            vec![&mut account_account, &mut owner2_account],
        )
        .unwrap();

        // wrong owner
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            do_process_instruction(
                set_authority(
                    &program_id,
                    &mint_key,
                    Some(&owner3_key),
                    AuthorityType::MintTokens,
                    &owner2_key,
                    &[]
                )
                .unwrap(),
                vec![&mut mint_account, &mut owner2_account],
            )
        );

        // owner did not sign
        let mut instruction = set_authority(
            &program_id,
            &mint_key,
            Some(&owner2_key),
            AuthorityType::MintTokens,
            &owner_key,
            &[],
        )
        .unwrap();
        instruction.accounts[1].is_signer = false;
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            do_process_instruction(instruction, vec![&mut mint_account, &mut owner_account],)
        );

        // cannot freeze
        assert_eq!(
            Err(TokenError::MintCannotFreeze.into()),
            do_process_instruction(
                set_authority(
                    &program_id,
                    &mint_key,
                    Some(&owner2_key),
                    AuthorityType::FreezeAccount,
                    &owner_key,
                    &[],
                )
                .unwrap(),
                vec![&mut mint_account, &mut owner_account],
            )
        );

        // set owner
        do_process_instruction(
            set_authority(
                &program_id,
                &mint_key,
                Some(&owner2_key),
                AuthorityType::MintTokens,
                &owner_key,
                &[],
            )
            .unwrap(),
            vec![&mut mint_account, &mut owner_account],
        )
        .unwrap();

        // set owner to None
        do_process_instruction(
            set_authority(
                &program_id,
                &mint_key,
                None,
                AuthorityType::MintTokens,
                &owner2_key,
                &[],
            )
            .unwrap(),
            vec![&mut mint_account, &mut owner2_account],
        )
        .unwrap();

        // test unsetting mint_authority is one-way operation
        assert_eq!(
            Err(TokenError::FixedSupply.into()),
            do_process_instruction(
                set_authority(
                    &program_id,
                    &mint2_key,
                    Some(&owner2_key),
                    AuthorityType::MintTokens,
                    &owner_key,
                    &[]
                )
                .unwrap(),
                vec![&mut mint_account, &mut owner_account],
            )
        );

        // set freeze_authority
        do_process_instruction(
            set_authority(
                &program_id,
                &mint2_key,
                Some(&owner2_key),
                AuthorityType::FreezeAccount,
                &owner_key,
                &[],
            )
            .unwrap(),
            vec![&mut mint2_account, &mut owner_account],
        )
        .unwrap();

        // test unsetting freeze_authority is one-way operation
        do_process_instruction(
            set_authority(
                &program_id,
                &mint2_key,
                None,
                AuthorityType::FreezeAccount,
                &owner2_key,
                &[],
            )
            .unwrap(),
            vec![&mut mint2_account, &mut owner2_account],
        )
        .unwrap();

        assert_eq!(
            Err(TokenError::MintCannotFreeze.into()),
            do_process_instruction(
                set_authority(
                    &program_id,
                    &mint2_key,
                    Some(&owner2_key),
                    AuthorityType::FreezeAccount,
                    &owner_key,
                    &[],
                )
                .unwrap(),
                vec![&mut mint2_account, &mut owner2_account],
            )
        );
    }

    #[test]
    fn test_mint_to() {
        let program_id = pubkey_rand();
        let account_key = pubkey_rand();
        let mut account_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let account2_key = pubkey_rand();
        let mut account2_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let account3_key = pubkey_rand();
        let mut account3_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let mismatch_key = pubkey_rand();
        let mut mismatch_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = SolanaAccount::default();
        let owner2_key = pubkey_rand();
        let mut owner2_account = SolanaAccount::default();
        let mint_key = pubkey_rand();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), size_of::<Mint>(), &program_id);
        let mint2_key = pubkey_rand();
        let uninitialized_key = pubkey_rand();
        let mut uninitialized_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let mut rent_sysvar = rent_sysvar();

        // create new mint with owner
        do_process_instruction(
            initialize_mint(&program_id, &mint_key, &owner_key, None, 2).unwrap(),
            vec![&mut mint_account, &mut rent_sysvar],
        )
        .unwrap();

        // create account
        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
            vec![
                &mut account_account,
                &mut mint_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();

        // create another account
        do_process_instruction(
            initialize_account(&program_id, &account2_key, &mint_key, &owner_key).unwrap(),
            vec![
                &mut account2_account,
                &mut mint_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();

        // create another account
        do_process_instruction(
            initialize_account(&program_id, &account3_key, &mint_key, &owner_key).unwrap(),
            vec![
                &mut account3_account,
                &mut mint_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();

        // create mismatch account
        do_process_instruction(
            initialize_account(&program_id, &mismatch_key, &mint_key, &owner_key).unwrap(),
            vec![
                &mut mismatch_account,
                &mut mint_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();
        Account::unpack_unchecked_mut(&mut mismatch_account.data, &mut |account: &mut Account| {
            account.mint = mint2_key;
            Ok(())
        })
        .unwrap();

        // mint to
        do_process_instruction(
            mint_to(&program_id, &mint_key, &account_key, &owner_key, &[], 42).unwrap(),
            vec![&mut mint_account, &mut account_account, &mut owner_account],
        )
        .unwrap();

        Mint::unpack_unchecked_mut(&mut mint_account.data, &mut |mint: &mut Mint| {
            assert_eq!(mint.supply, 42);
            Ok(())
        })
        .unwrap();
        Account::unpack_unchecked_mut(&mut account_account.data, &mut |account: &mut Account| {
            assert_eq!(account.amount, 42);
            Ok(())
        })
        .unwrap();

        // mint to another account to test supply accumulation
        do_process_instruction(
            mint_to(&program_id, &mint_key, &account2_key, &owner_key, &[], 42).unwrap(),
            vec![&mut mint_account, &mut account2_account, &mut owner_account],
        )
        .unwrap();

        Mint::unpack_unchecked_mut(&mut mint_account.data, &mut |mint: &mut Mint| {
            assert_eq!(mint.supply, 84);
            Ok(())
        })
        .unwrap();
        Account::unpack_unchecked_mut(&mut account2_account.data, &mut |account: &mut Account| {
            assert_eq!(account.amount, 42);
            Ok(())
        })
        .unwrap();

        // missing signer
        let mut instruction =
            mint_to(&program_id, &mint_key, &account2_key, &owner_key, &[], 42).unwrap();
        instruction.accounts[2].is_signer = false;
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            do_process_instruction(
                instruction,
                vec![&mut mint_account, &mut account2_account, &mut owner_account],
            )
        );

        // mismatch account
        assert_eq!(
            Err(TokenError::MintMismatch.into()),
            do_process_instruction(
                mint_to(&program_id, &mint_key, &mismatch_key, &owner_key, &[], 42).unwrap(),
                vec![&mut mint_account, &mut mismatch_account, &mut owner_account],
            )
        );

        // missing owner
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            do_process_instruction(
                mint_to(&program_id, &mint_key, &account2_key, &owner2_key, &[], 42).unwrap(),
                vec![
                    &mut mint_account,
                    &mut account2_account,
                    &mut owner2_account,
                ],
            )
        );

        // uninitialized destination account
        assert_eq!(
            Err(TokenError::UninitializedState.into()),
            do_process_instruction(
                mint_to(
                    &program_id,
                    &mint_key,
                    &uninitialized_key,
                    &owner_key,
                    &[],
                    42
                )
                .unwrap(),
                vec![
                    &mut mint_account,
                    &mut uninitialized_account,
                    &mut owner_account,
                ],
            )
        );

        // unset mint_authority and test minting fails
        do_process_instruction(
            set_authority(
                &program_id,
                &mint_key,
                None,
                AuthorityType::MintTokens,
                &owner_key,
                &[],
            )
            .unwrap(),
            vec![&mut mint_account, &mut owner_account],
        )
        .unwrap();
        assert_eq!(
            Err(TokenError::FixedSupply.into()),
            do_process_instruction(
                mint_to(&program_id, &mint_key, &account2_key, &owner_key, &[], 42).unwrap(),
                vec![&mut mint_account, &mut account2_account, &mut owner_account],
            )
        );
    }

    #[test]
    fn test_burn() {
        let program_id = pubkey_rand();
        let account_key = pubkey_rand();
        let mut account_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let account2_key = pubkey_rand();
        let mut account2_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let account3_key = pubkey_rand();
        let mut account3_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let delegate_key = pubkey_rand();
        let mut delegate_account = SolanaAccount::default();
        let mismatch_key = pubkey_rand();
        let mut mismatch_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = SolanaAccount::default();
        let owner2_key = pubkey_rand();
        let mut owner2_account = SolanaAccount::default();
        let mint_key = pubkey_rand();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), size_of::<Mint>(), &program_id);
        let mint2_key = pubkey_rand();
        let mut rent_sysvar = rent_sysvar();

        // create new mint
        do_process_instruction(
            initialize_mint(&program_id, &mint_key, &owner_key, None, 2).unwrap(),
            vec![&mut mint_account, &mut rent_sysvar],
        )
        .unwrap();

        // create account
        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
            vec![
                &mut account_account,
                &mut mint_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();

        // create another account
        do_process_instruction(
            initialize_account(&program_id, &account2_key, &mint_key, &owner_key).unwrap(),
            vec![
                &mut account2_account,
                &mut mint_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();

        // create another account
        do_process_instruction(
            initialize_account(&program_id, &account3_key, &mint_key, &owner_key).unwrap(),
            vec![
                &mut account3_account,
                &mut mint_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();

        // create mismatch account
        do_process_instruction(
            initialize_account(&program_id, &mismatch_key, &mint_key, &owner_key).unwrap(),
            vec![
                &mut mismatch_account,
                &mut mint_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();
        Account::unpack_unchecked_mut(&mut mismatch_account.data, &mut |account: &mut Account| {
            account.mint = mint2_key;
            Ok(())
        })
        .unwrap();

        // mint to account
        do_process_instruction(
            mint_to(&program_id, &mint_key, &account_key, &owner_key, &[], 1000).unwrap(),
            vec![&mut mint_account, &mut account_account, &mut owner_account],
        )
        .unwrap();

        // missing signer
        let mut instruction =
            burn(&program_id, &account_key, &mint_key, &delegate_key, &[], 42).unwrap();
        instruction.accounts[1].is_signer = false;
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            do_process_instruction(
                instruction,
                vec![
                    &mut account_account,
                    &mut mint_account,
                    &mut delegate_account
                ],
            )
        );

        // missing owner
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            do_process_instruction(
                burn(&program_id, &account_key, &mint_key, &owner2_key, &[], 42).unwrap(),
                vec![&mut account_account, &mut mint_account, &mut owner2_account],
            )
        );

        // mint mismatch
        assert_eq!(
            Err(TokenError::MintMismatch.into()),
            do_process_instruction(
                burn(&program_id, &mismatch_key, &mint_key, &owner_key, &[], 42).unwrap(),
                vec![&mut mismatch_account, &mut mint_account, &mut owner_account],
            )
        );

        // burn
        do_process_instruction(
            burn(&program_id, &account_key, &mint_key, &owner_key, &[], 21).unwrap(),
            vec![&mut account_account, &mut mint_account, &mut owner_account],
        )
        .unwrap();

        // burn2, with incorrect decimals
        assert_eq!(
            Err(TokenError::MintDecimalsMismatch.into()),
            do_process_instruction(
                burn2(&program_id, &account_key, &mint_key, &owner_key, &[], 21, 3).unwrap(),
                vec![&mut account_account, &mut mint_account, &mut owner_account],
            )
        );

        // burn2
        do_process_instruction(
            burn2(&program_id, &account_key, &mint_key, &owner_key, &[], 21, 2).unwrap(),
            vec![&mut account_account, &mut mint_account, &mut owner_account],
        )
        .unwrap();

        Mint::unpack_unchecked_mut(&mut mint_account.data, &mut |mint: &mut Mint| {
            assert_eq!(mint.supply, 1000 - 42);

            Ok(())
        })
        .unwrap();
        Account::unpack_unchecked_mut(&mut account_account.data, &mut |account: &mut Account| {
            assert_eq!(account.amount, 1000 - 42);
            Ok(())
        })
        .unwrap();

        // insufficient funds
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            do_process_instruction(
                burn(
                    &program_id,
                    &account_key,
                    &mint_key,
                    &owner_key,
                    &[],
                    100_000_000
                )
                .unwrap(),
                vec![&mut account_account, &mut mint_account, &mut owner_account],
            )
        );

        // approve delegate
        do_process_instruction(
            approve(
                &program_id,
                &account_key,
                &delegate_key,
                &owner_key,
                &[],
                84,
            )
            .unwrap(),
            vec![
                &mut account_account,
                &mut delegate_account,
                &mut owner_account,
            ],
        )
        .unwrap();

        // not a delegate of source account
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            do_process_instruction(
                burn(
                    &program_id,
                    &account_key,
                    &mint_key,
                    &owner_key,
                    &[],
                    100_000_000
                )
                .unwrap(),
                vec![&mut account_account, &mut mint_account, &mut owner_account],
            )
        );

        // burn via delegate
        do_process_instruction(
            burn(&program_id, &account_key, &mint_key, &delegate_key, &[], 84).unwrap(),
            vec![
                &mut account_account,
                &mut mint_account,
                &mut delegate_account,
            ],
        )
        .unwrap();

        // match
        Mint::unpack_unchecked_mut(&mut mint_account.data, &mut |mint: &mut Mint| {
            assert_eq!(mint.supply, 1000 - 42 - 84);
            Ok(())
        })
        .unwrap();
        Account::unpack_unchecked_mut(&mut account_account.data, &mut |account: &mut Account| {
            assert_eq!(account.amount, 1000 - 42 - 84);
            Ok(())
        })
        .unwrap();

        // insufficient funds approved via delegate
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            do_process_instruction(
                burn(
                    &program_id,
                    &account_key,
                    &mint_key,
                    &delegate_key,
                    &[],
                    100
                )
                .unwrap(),
                vec![
                    &mut account_account,
                    &mut mint_account,
                    &mut delegate_account
                ],
            )
        );
    }

    #[test]
    fn test_multisig() {
        let program_id = pubkey_rand();
        let mint_key = pubkey_rand();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), size_of::<Mint>(), &program_id);
        let account_key = pubkey_rand();
        let mut account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let account2_key = pubkey_rand();
        let mut account2_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = SolanaAccount::default();
        let multisig_key = pubkey_rand();
        let mut multisig_account = SolanaAccount::new(42, size_of::<Multisig>(), &program_id);
        let multisig_delegate_key = pubkey_rand();
        let mut multisig_delegate_account = SolanaAccount::new(
            multisig_minimum_balance(),
            size_of::<Multisig>(),
            &program_id,
        );
        let signer_keys = vec![pubkey_rand(); MAX_SIGNERS];
        let signer_key_refs: Vec<&Pubkey> = signer_keys.iter().map(|key| key).collect();
        let mut signer_accounts = vec![SolanaAccount::new(0, 0, &program_id); MAX_SIGNERS];
        let mut rent_sysvar = rent_sysvar();

        // multisig is not rent exempt
        let account_info_iter = &mut signer_accounts.iter_mut();
        assert_eq!(
            Err(TokenError::NotRentExempt.into()),
            do_process_instruction(
                initialize_multisig(&program_id, &multisig_key, &[&signer_keys[0]], 1).unwrap(),
                vec![
                    &mut multisig_account,
                    &mut rent_sysvar,
                    &mut account_info_iter.next().unwrap(),
                ],
            )
        );

        multisig_account.lamports = multisig_minimum_balance();

        // single signer
        let account_info_iter = &mut signer_accounts.iter_mut();
        do_process_instruction(
            initialize_multisig(&program_id, &multisig_key, &[&signer_keys[0]], 1).unwrap(),
            vec![
                &mut multisig_account,
                &mut rent_sysvar,
                &mut account_info_iter.next().unwrap(),
            ],
        )
        .unwrap();

        // multiple signer
        let account_info_iter = &mut signer_accounts.iter_mut();
        do_process_instruction(
            initialize_multisig(
                &program_id,
                &multisig_delegate_key,
                &signer_key_refs,
                MAX_SIGNERS as u8,
            )
            .unwrap(),
            vec![
                &mut multisig_delegate_account,
                &mut rent_sysvar,
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
            ],
        )
        .unwrap();

        // create new mint with multisig owner
        do_process_instruction(
            initialize_mint(&program_id, &mint_key, &multisig_key, None, 2).unwrap(),
            vec![&mut mint_account, &mut rent_sysvar],
        )
        .unwrap();

        // create account with multisig owner
        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, &multisig_key).unwrap(),
            vec![
                &mut account,
                &mut mint_account,
                &mut multisig_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();

        // create another account with multisig owner
        do_process_instruction(
            initialize_account(
                &program_id,
                &account2_key,
                &mint_key,
                &multisig_delegate_key,
            )
            .unwrap(),
            vec![
                &mut account2_account,
                &mut mint_account,
                &mut multisig_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();

        // mint to account
        let account_info_iter = &mut signer_accounts.iter_mut();
        do_process_instruction(
            mint_to(
                &program_id,
                &mint_key,
                &account_key,
                &multisig_key,
                &[&signer_keys[0]],
                1000,
            )
            .unwrap(),
            vec![
                &mut mint_account,
                &mut account,
                &mut multisig_account,
                &mut account_info_iter.next().unwrap(),
            ],
        )
        .unwrap();

        // approve
        let account_info_iter = &mut signer_accounts.iter_mut();
        do_process_instruction(
            approve(
                &program_id,
                &account_key,
                &multisig_delegate_key,
                &multisig_key,
                &[&signer_keys[0]],
                100,
            )
            .unwrap(),
            vec![
                &mut account,
                &mut multisig_delegate_account,
                &mut multisig_account,
                &mut account_info_iter.next().unwrap(),
            ],
        )
        .unwrap();

        // transfer
        let account_info_iter = &mut signer_accounts.iter_mut();
        do_process_instruction(
            transfer(
                &program_id,
                &account_key,
                &account2_key,
                &multisig_key,
                &[&signer_keys[0]],
                42,
            )
            .unwrap(),
            vec![
                &mut account,
                &mut account2_account,
                &mut multisig_account,
                &mut account_info_iter.next().unwrap(),
            ],
        )
        .unwrap();

        // transfer via delegate
        let account_info_iter = &mut signer_accounts.iter_mut();
        do_process_instruction(
            transfer(
                &program_id,
                &account_key,
                &account2_key,
                &multisig_delegate_key,
                &signer_key_refs,
                42,
            )
            .unwrap(),
            vec![
                &mut account,
                &mut account2_account,
                &mut multisig_delegate_account,
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
            ],
        )
        .unwrap();

        // mint to
        let account_info_iter = &mut signer_accounts.iter_mut();
        do_process_instruction(
            mint_to(
                &program_id,
                &mint_key,
                &account2_key,
                &multisig_key,
                &[&signer_keys[0]],
                42,
            )
            .unwrap(),
            vec![
                &mut mint_account,
                &mut account2_account,
                &mut multisig_account,
                &mut account_info_iter.next().unwrap(),
            ],
        )
        .unwrap();

        // burn
        let account_info_iter = &mut signer_accounts.iter_mut();
        do_process_instruction(
            burn(
                &program_id,
                &account_key,
                &mint_key,
                &multisig_key,
                &[&signer_keys[0]],
                42,
            )
            .unwrap(),
            vec![
                &mut account,
                &mut mint_account,
                &mut multisig_account,
                &mut account_info_iter.next().unwrap(),
            ],
        )
        .unwrap();

        // burn via delegate
        let account_info_iter = &mut signer_accounts.iter_mut();
        do_process_instruction(
            burn(
                &program_id,
                &account_key,
                &mint_key,
                &multisig_delegate_key,
                &signer_key_refs,
                42,
            )
            .unwrap(),
            vec![
                &mut account,
                &mut mint_account,
                &mut multisig_delegate_account,
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
            ],
        )
        .unwrap();

        // freeze account
        let account3_key = pubkey_rand();
        let mut account3_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let mint2_key = pubkey_rand();
        let mut mint2_account =
            SolanaAccount::new(mint_minimum_balance(), size_of::<Mint>(), &program_id);
        do_process_instruction(
            initialize_mint(
                &program_id,
                &mint2_key,
                &multisig_key,
                Some(&multisig_key),
                2,
            )
            .unwrap(),
            vec![&mut mint2_account, &mut rent_sysvar],
        )
        .unwrap();
        do_process_instruction(
            initialize_account(&program_id, &account3_key, &mint2_key, &owner_key).unwrap(),
            vec![
                &mut account3_account,
                &mut mint2_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();
        let account_info_iter = &mut signer_accounts.iter_mut();
        do_process_instruction(
            mint_to(
                &program_id,
                &mint2_key,
                &account3_key,
                &multisig_key,
                &[&signer_keys[0]],
                1000,
            )
            .unwrap(),
            vec![
                &mut mint2_account,
                &mut account3_account,
                &mut multisig_account,
                &mut account_info_iter.next().unwrap(),
            ],
        )
        .unwrap();
        let account_info_iter = &mut signer_accounts.iter_mut();
        do_process_instruction(
            freeze_account(
                &program_id,
                &account3_key,
                &mint2_key,
                &multisig_key,
                &[&signer_keys[0]],
            )
            .unwrap(),
            vec![
                &mut account3_account,
                &mut mint2_account,
                &mut multisig_account,
                &mut account_info_iter.next().unwrap(),
            ],
        )
        .unwrap();

        // do SetAuthority on mint
        let account_info_iter = &mut signer_accounts.iter_mut();
        do_process_instruction(
            set_authority(
                &program_id,
                &mint_key,
                Some(&owner_key),
                AuthorityType::MintTokens,
                &multisig_key,
                &[&signer_keys[0]],
            )
            .unwrap(),
            vec![
                &mut mint_account,
                &mut multisig_account,
                &mut account_info_iter.next().unwrap(),
            ],
        )
        .unwrap();

        // do SetAuthority on account
        let account_info_iter = &mut signer_accounts.iter_mut();
        do_process_instruction(
            set_authority(
                &program_id,
                &account_key,
                Some(&owner_key),
                AuthorityType::AccountOwner,
                &multisig_key,
                &[&signer_keys[0]],
            )
            .unwrap(),
            vec![
                &mut account,
                &mut multisig_account,
                &mut account_info_iter.next().unwrap(),
            ],
        )
        .unwrap();
    }

    #[test]
    fn test_validate_owner() {
        let program_id = pubkey_rand();
        let owner_key = pubkey_rand();
        let mut signer_keys = [Pubkey::default(); MAX_SIGNERS];
        for signer_key in signer_keys.iter_mut().take(MAX_SIGNERS) {
            *signer_key = pubkey_rand();
        }
        let mut signer_lamports = 0;
        let mut signer_data = vec![];
        let mut signers = vec![
            AccountInfo::new(
                &owner_key,
                true,
                false,
                &mut signer_lamports,
                &mut signer_data,
                &program_id,
                false,
                Epoch::default(),
            );
            MAX_SIGNERS + 1
        ];
        for (signer, key) in signers.iter_mut().zip(&signer_keys) {
            signer.key = key;
        }
        let mut lamports = 0;
        let mut data = vec![0; size_of::<Multisig>()];
        Multisig::unpack_unchecked_mut(&mut data, &mut |multisig: &mut Multisig| {
            multisig.m = MAX_SIGNERS as u8;
            multisig.n = MAX_SIGNERS as u8;
            multisig.signers = signer_keys;
            multisig.is_initialized = true;
            Ok(())
        })
        .unwrap();
        let owner_account_info = AccountInfo::new(
            &owner_key,
            false,
            false,
            &mut lamports,
            &mut data,
            &program_id,
            false,
            Epoch::default(),
        );

        // full 11 of 11
        Processor::validate_owner(&program_id, &owner_key, &owner_account_info, &signers).unwrap();

        // 1 of 11
        {
            let mut data_ref_mut = owner_account_info.data.borrow_mut();
            Multisig::unpack_unchecked_mut(&mut data_ref_mut, &mut |multisig: &mut Multisig| {
                multisig.m = 1;
                Ok(())
            })
            .unwrap();
        }
        Processor::validate_owner(&program_id, &owner_key, &owner_account_info, &signers).unwrap();

        // 2:1
        {
            let mut data_ref_mut = owner_account_info.data.borrow_mut();
            Multisig::unpack_unchecked_mut(&mut data_ref_mut, &mut |multisig: &mut Multisig| {
                multisig.m = 2;
                multisig.n = 1;
                Ok(())
            })
            .unwrap();
        }
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            Processor::validate_owner(&program_id, &owner_key, &owner_account_info, &signers)
        );

        // 0:11
        {
            let mut data_ref_mut = owner_account_info.data.borrow_mut();
            Multisig::unpack_unchecked_mut(&mut data_ref_mut, &mut |multisig: &mut Multisig| {
                multisig.m = 0;
                multisig.n = 11;
                Ok(())
            })
            .unwrap();
        }
        Processor::validate_owner(&program_id, &owner_key, &owner_account_info, &signers).unwrap();

        // 2:11 but 0 provided
        {
            let mut data_ref_mut = owner_account_info.data.borrow_mut();
            Multisig::unpack_unchecked_mut(&mut data_ref_mut, &mut |multisig: &mut Multisig| {
                multisig.m = 2;
                multisig.n = 11;
                Ok(())
            })
            .unwrap();
        }
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            Processor::validate_owner(&program_id, &owner_key, &owner_account_info, &[])
        );
        // 2:11 but 1 provided
        {
            let mut data_ref_mut = owner_account_info.data.borrow_mut();
            Multisig::unpack_unchecked_mut(&mut data_ref_mut, &mut |multisig: &mut Multisig| {
                multisig.m = 2;
                multisig.n = 11;
                Ok(())
            })
            .unwrap();
        }
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            Processor::validate_owner(&program_id, &owner_key, &owner_account_info, &signers[0..1])
        );

        // 2:11, 2 from middle provided
        {
            let mut data_ref_mut = owner_account_info.data.borrow_mut();
            Multisig::unpack_unchecked_mut(&mut data_ref_mut, &mut |multisig: &mut Multisig| {
                multisig.m = 2;
                multisig.n = 11;
                Ok(())
            })
            .unwrap();
        }
        Processor::validate_owner(&program_id, &owner_key, &owner_account_info, &signers[5..7])
            .unwrap();

        // 11:11, one is not a signer
        {
            let mut data_ref_mut = owner_account_info.data.borrow_mut();
            Multisig::unpack_unchecked_mut(&mut data_ref_mut, &mut |multisig: &mut Multisig| {
                multisig.m = 2; // TODO 11?
                multisig.n = 11;
                Ok(())
            })
            .unwrap();
        }
        signers[5].is_signer = false;
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            Processor::validate_owner(&program_id, &owner_key, &owner_account_info, &signers)
        );
        signers[5].is_signer = true;
    }

    #[test]
    fn test_close_account() {
        let program_id = pubkey_rand();
        let mint_key = pubkey_rand();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), size_of::<Mint>(), &program_id);
        let account_key = pubkey_rand();
        let mut account_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let account2_key = pubkey_rand();
        let mut account2_account = SolanaAccount::new(
            account_minimum_balance() + 42,
            size_of::<Account>(),
            &program_id,
        );
        let account3_key = pubkey_rand();
        let mut account3_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = SolanaAccount::default();
        let owner2_key = pubkey_rand();
        let mut owner2_account = SolanaAccount::default();
        let mut rent_sysvar = rent_sysvar();

        // uninitialized
        assert_eq!(
            Err(TokenError::UninitializedState.into()),
            do_process_instruction(
                close_account(&program_id, &account_key, &account3_key, &owner2_key, &[]).unwrap(),
                vec![
                    &mut account_account,
                    &mut account3_account,
                    &mut owner2_account,
                ],
            )
        );

        // initialize and mint to non-native account
        do_process_instruction(
            initialize_mint(&program_id, &mint_key, &owner_key, None, 2).unwrap(),
            vec![&mut mint_account, &mut rent_sysvar],
        )
        .unwrap();
        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
            vec![
                &mut account_account,
                &mut mint_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();
        do_process_instruction(
            mint_to(&program_id, &mint_key, &account_key, &owner_key, &[], 42).unwrap(),
            vec![
                &mut mint_account,
                &mut account_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();
        Account::unpack_unchecked_mut(&mut account_account.data, &mut |account: &mut Account| {
            assert_eq!(account.amount, 42);
            Ok(())
        })
        .unwrap();

        // initialize native account
        do_process_instruction(
            initialize_account(
                &program_id,
                &account2_key,
                &crate::native_mint::id(),
                &owner_key,
            )
            .unwrap(),
            vec![
                &mut account2_account,
                &mut mint_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();
        Account::unpack_unchecked_mut(&mut account2_account.data, &mut |account: &mut Account| {
            assert!(account.is_native());
            assert_eq!(account.amount, 42);
            Ok(())
        })
        .unwrap();

        // close non-native account with balance
        assert_eq!(
            Err(TokenError::NonNativeHasBalance.into()),
            do_process_instruction(
                close_account(&program_id, &account_key, &account3_key, &owner_key, &[]).unwrap(),
                vec![
                    &mut account_account,
                    &mut account3_account,
                    &mut owner_account,
                ],
            )
        );
        assert_eq!(account_account.lamports, account_minimum_balance());

        // empty account
        do_process_instruction(
            burn(&program_id, &account_key, &mint_key, &owner_key, &[], 42).unwrap(),
            vec![&mut account_account, &mut mint_account, &mut owner_account],
        )
        .unwrap();

        // wrong owner
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            do_process_instruction(
                close_account(&program_id, &account_key, &account3_key, &owner2_key, &[]).unwrap(),
                vec![
                    &mut account_account,
                    &mut account3_account,
                    &mut owner2_account,
                ],
            )
        );

        // close account
        do_process_instruction(
            close_account(&program_id, &account_key, &account3_key, &owner_key, &[]).unwrap(),
            vec![
                &mut account_account,
                &mut account3_account,
                &mut owner_account,
            ],
        )
        .unwrap();
        assert_eq!(account_account.lamports, 0);
        assert_eq!(account3_account.lamports, 2 * account_minimum_balance());
        Account::unpack_unchecked_mut(&mut account_account.data, &mut |account: &mut Account| {
            assert_eq!(account.amount, 0);
            Ok(())
        })
        .unwrap();

        // fund and initialize new non-native account to test close authority
        let account_key = pubkey_rand();
        let mut account_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let owner2_key = pubkey_rand();
        let mut owner2_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
            vec![
                &mut account_account,
                &mut mint_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();
        account_account.lamports = 2;

        do_process_instruction(
            set_authority(
                &program_id,
                &account_key,
                Some(&owner2_key),
                AuthorityType::CloseAccount,
                &owner_key,
                &[],
            )
            .unwrap(),
            vec![&mut account_account, &mut owner_account],
        )
        .unwrap();

        // account owner cannot authorize close if close_authority is set
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            do_process_instruction(
                close_account(&program_id, &account_key, &account3_key, &owner_key, &[]).unwrap(),
                vec![
                    &mut account_account,
                    &mut account3_account,
                    &mut owner_account,
                ],
            )
        );

        // close non-native account with close_authority
        do_process_instruction(
            close_account(&program_id, &account_key, &account3_key, &owner2_key, &[]).unwrap(),
            vec![
                &mut account_account,
                &mut account3_account,
                &mut owner2_account,
            ],
        )
        .unwrap();
        assert_eq!(account_account.lamports, 0);
        assert_eq!(account3_account.lamports, 2 * account_minimum_balance() + 2);
        Account::unpack_unchecked_mut(&mut account_account.data, &mut |account: &mut Account| {
            assert_eq!(account.amount, 0);
            Ok(())
        })
        .unwrap();

        // close native account
        do_process_instruction(
            close_account(&program_id, &account2_key, &account3_key, &owner_key, &[]).unwrap(),
            vec![
                &mut account2_account,
                &mut account3_account,
                &mut owner_account,
            ],
        )
        .unwrap();
        Account::unpack_unchecked_mut(&mut account2_account.data, &mut |account: &mut Account| {
            assert!(account.is_native());
            assert_eq!(account_account.lamports, 0);
            assert_eq!(account.amount, 0);
            assert_eq!(
                account3_account.lamports,
                3 * account_minimum_balance() + 2 + 42
            );
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_native_token() {
        let program_id = pubkey_rand();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), size_of::<Mint>(), &program_id);
        let account_key = pubkey_rand();
        let mut account_account = SolanaAccount::new(
            account_minimum_balance() + 40,
            size_of::<Account>(),
            &program_id,
        );
        let account2_key = pubkey_rand();
        let mut account2_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let account3_key = pubkey_rand();
        let mut account3_account = SolanaAccount::new(account_minimum_balance(), 0, &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = SolanaAccount::default();
        let mut rent_sysvar = rent_sysvar();

        // initialize native account
        do_process_instruction(
            initialize_account(
                &program_id,
                &account_key,
                &crate::native_mint::id(),
                &owner_key,
            )
            .unwrap(),
            vec![
                &mut account_account,
                &mut mint_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();
        Account::unpack_unchecked_mut(&mut account_account.data, &mut |account: &mut Account| {
            assert!(account.is_native());
            assert_eq!(account.amount, 40);
            Ok(())
        })
        .unwrap();

        // initialize native account
        do_process_instruction(
            initialize_account(
                &program_id,
                &account2_key,
                &crate::native_mint::id(),
                &owner_key,
            )
            .unwrap(),
            vec![
                &mut account2_account,
                &mut mint_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();
        Account::unpack_unchecked_mut(&mut account2_account.data, &mut |account: &mut Account| {
            assert!(account.is_native());
            assert_eq!(account.amount, 0);
            Ok(())
        })
        .unwrap();

        // mint_to unsupported
        assert_eq!(
            Err(TokenError::NativeNotSupported.into()),
            do_process_instruction(
                mint_to(
                    &program_id,
                    &crate::native_mint::id(),
                    &account_key,
                    &owner_key,
                    &[],
                    42
                )
                .unwrap(),
                vec![&mut mint_account, &mut account_account, &mut owner_account],
            )
        );

        // burn unsupported
        let bogus_mint_key = pubkey_rand();
        let mut bogus_mint_account =
            SolanaAccount::new(mint_minimum_balance(), size_of::<Mint>(), &program_id);
        do_process_instruction(
            initialize_mint(&program_id, &bogus_mint_key, &owner_key, None, 2).unwrap(),
            vec![&mut bogus_mint_account, &mut rent_sysvar],
        )
        .unwrap();

        assert_eq!(
            Err(TokenError::NativeNotSupported.into()),
            do_process_instruction(
                burn(
                    &program_id,
                    &account_key,
                    &bogus_mint_key,
                    &owner_key,
                    &[],
                    42
                )
                .unwrap(),
                vec![
                    &mut account_account,
                    &mut bogus_mint_account,
                    &mut owner_account
                ],
            )
        );

        // ensure can't transfer below rent-exempt reserve
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            do_process_instruction(
                transfer(
                    &program_id,
                    &account_key,
                    &account2_key,
                    &owner_key,
                    &[],
                    50,
                )
                .unwrap(),
                vec![
                    &mut account_account,
                    &mut account2_account,
                    &mut owner_account,
                ],
            )
        );

        // transfer between native accounts
        do_process_instruction(
            transfer(
                &program_id,
                &account_key,
                &account2_key,
                &owner_key,
                &[],
                40,
            )
            .unwrap(),
            vec![
                &mut account_account,
                &mut account2_account,
                &mut owner_account,
            ],
        )
        .unwrap();
        assert_eq!(account_account.lamports, account_minimum_balance());
        Account::unpack_unchecked_mut(&mut account_account.data, &mut |account: &mut Account| {
            assert!(account.is_native());
            assert_eq!(account.amount, 0);
            Ok(())
        })
        .unwrap();
        assert_eq!(account2_account.lamports, account_minimum_balance() + 40);
        Account::unpack_unchecked_mut(&mut account2_account.data, &mut |account: &mut Account| {
            assert!(account.is_native());
            assert_eq!(account.amount, 40);
            Ok(())
        })
        .unwrap();

        // close native account
        do_process_instruction(
            close_account(&program_id, &account_key, &account3_key, &owner_key, &[]).unwrap(),
            vec![
                &mut account_account,
                &mut account3_account,
                &mut owner_account,
            ],
        )
        .unwrap();
        assert_eq!(account_account.lamports, 0);
        assert_eq!(account3_account.lamports, 2 * account_minimum_balance());
        Account::unpack_unchecked_mut(&mut account_account.data, &mut |account: &mut Account| {
            assert!(account.is_native());
            assert_eq!(account.amount, 0);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_overflow() {
        let program_id = pubkey_rand();
        let account_key = pubkey_rand();
        let mut account_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let account2_key = pubkey_rand();
        let mut account2_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = SolanaAccount::default();
        let owner2_key = pubkey_rand();
        let mut owner2_account = SolanaAccount::default();
        let mint_owner_key = pubkey_rand();
        let mut mint_owner_account = SolanaAccount::default();
        let mint_key = pubkey_rand();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), size_of::<Mint>(), &program_id);
        let mut rent_sysvar = rent_sysvar();

        // create new mint with owner
        do_process_instruction(
            initialize_mint(&program_id, &mint_key, &mint_owner_key, None, 2).unwrap(),
            vec![&mut mint_account, &mut rent_sysvar],
        )
        .unwrap();

        // create an account
        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
            vec![
                &mut account_account,
                &mut mint_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();

        // create another account
        do_process_instruction(
            initialize_account(&program_id, &account2_key, &mint_key, &owner2_key).unwrap(),
            vec![
                &mut account2_account,
                &mut mint_account,
                &mut owner2_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();

        // mint the max to an account
        do_process_instruction(
            mint_to(
                &program_id,
                &mint_key,
                &account_key,
                &mint_owner_key,
                &[],
                u64::MAX,
            )
            .unwrap(),
            vec![
                &mut mint_account,
                &mut account_account,
                &mut mint_owner_account,
            ],
        )
        .unwrap();
        Account::unpack_unchecked_mut(&mut account_account.data, &mut |account: &mut Account| {
            assert_eq!(account.amount, u64::MAX);
            Ok(())
        })
        .unwrap();

        // attempt to mint one more to account
        assert_eq!(
            Err(TokenError::Overflow.into()),
            do_process_instruction(
                mint_to(
                    &program_id,
                    &mint_key,
                    &account_key,
                    &mint_owner_key,
                    &[],
                    1,
                )
                .unwrap(),
                vec![
                    &mut mint_account,
                    &mut account_account,
                    &mut mint_owner_account,
                ],
            )
        );
        Account::unpack_unchecked_mut(&mut account_account.data, &mut |account: &mut Account| {
            assert_eq!(account.amount, u64::MAX);
            Ok(())
        })
        .unwrap();

        // atttempt to mint one more to the other account
        assert_eq!(
            Err(TokenError::Overflow.into()),
            do_process_instruction(
                mint_to(
                    &program_id,
                    &mint_key,
                    &account2_key,
                    &mint_owner_key,
                    &[],
                    1,
                )
                .unwrap(),
                vec![
                    &mut mint_account,
                    &mut account2_account,
                    &mut mint_owner_account,
                ],
            )
        );

        // burn some of the supply
        do_process_instruction(
            burn(&program_id, &account_key, &mint_key, &owner_key, &[], 100).unwrap(),
            vec![&mut account_account, &mut mint_account, &mut owner_account],
        )
        .unwrap();
        Account::unpack_unchecked_mut(&mut account_account.data, &mut |account: &mut Account| {
            assert_eq!(account.amount, u64::MAX - 100);
            Ok(())
        })
        .unwrap();

        do_process_instruction(
            mint_to(
                &program_id,
                &mint_key,
                &account_key,
                &mint_owner_key,
                &[],
                100,
            )
            .unwrap(),
            vec![
                &mut mint_account,
                &mut account_account,
                &mut mint_owner_account,
            ],
        )
        .unwrap();
        Account::unpack_unchecked_mut(&mut account_account.data, &mut |account: &mut Account| {
            assert_eq!(account.amount, u64::MAX);
            Ok(())
        })
        .unwrap();

        // manipulate account balance to attempt overflow transfer
        Account::unpack_unchecked_mut(&mut account2_account.data, &mut |account: &mut Account| {
            account.amount = 1;
            Ok(())
        })
        .unwrap();

        assert_eq!(
            Err(TokenError::Overflow.into()),
            do_process_instruction(
                transfer(
                    &program_id,
                    &account2_key,
                    &account_key,
                    &owner2_key,
                    &[],
                    1,
                )
                .unwrap(),
                vec![
                    &mut account2_account,
                    &mut account_account,
                    &mut owner2_account,
                ],
            )
        );
    }

    #[test]
    fn test_frozen() {
        let program_id = pubkey_rand();
        let account_key = pubkey_rand();
        let mut account_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let account2_key = pubkey_rand();
        let mut account2_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = SolanaAccount::default();
        let mint_key = pubkey_rand();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), size_of::<Mint>(), &program_id);
        let mut rent_sysvar = rent_sysvar();

        // create new mint and fund first account
        do_process_instruction(
            initialize_mint(&program_id, &mint_key, &owner_key, None, 2).unwrap(),
            vec![&mut mint_account, &mut rent_sysvar],
        )
        .unwrap();

        // create account
        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
            vec![
                &mut account_account,
                &mut mint_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();

        // create another account
        do_process_instruction(
            initialize_account(&program_id, &account2_key, &mint_key, &owner_key).unwrap(),
            vec![
                &mut account2_account,
                &mut mint_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();

        // fund first account
        do_process_instruction(
            mint_to(&program_id, &mint_key, &account_key, &owner_key, &[], 1000).unwrap(),
            vec![&mut mint_account, &mut account_account, &mut owner_account],
        )
        .unwrap();

        // no transfer if either account is frozen
        Account::unpack_unchecked_mut(&mut account2_account.data, &mut |account: &mut Account| {
            account.state = AccountState::Frozen;
            Ok(())
        })
        .unwrap();
        assert_eq!(
            Err(TokenError::AccountFrozen.into()),
            do_process_instruction(
                transfer(
                    &program_id,
                    &account_key,
                    &account2_key,
                    &owner_key,
                    &[],
                    500,
                )
                .unwrap(),
                vec![
                    &mut account_account,
                    &mut account2_account,
                    &mut owner_account,
                ],
            )
        );

        Account::unpack_unchecked_mut(&mut account_account.data, &mut |account: &mut Account| {
            account.state = AccountState::Initialized;
            Ok(())
        })
        .unwrap();
        Account::unpack_unchecked_mut(&mut account2_account.data, &mut |account: &mut Account| {
            account.state = AccountState::Frozen;
            Ok(())
        })
        .unwrap();
        assert_eq!(
            Err(TokenError::AccountFrozen.into()),
            do_process_instruction(
                transfer(
                    &program_id,
                    &account_key,
                    &account2_key,
                    &owner_key,
                    &[],
                    500,
                )
                .unwrap(),
                vec![
                    &mut account_account,
                    &mut account2_account,
                    &mut owner_account,
                ],
            )
        );

        // no approve if account is frozen
        Account::unpack_unchecked_mut(&mut account_account.data, &mut |account: &mut Account| {
            account.state = AccountState::Frozen;
            Ok(())
        })
        .unwrap();
        let delegate_key = pubkey_rand();
        let mut delegate_account = SolanaAccount::default();
        assert_eq!(
            Err(TokenError::AccountFrozen.into()),
            do_process_instruction(
                approve(
                    &program_id,
                    &account_key,
                    &delegate_key,
                    &owner_key,
                    &[],
                    100
                )
                .unwrap(),
                vec![
                    &mut account_account,
                    &mut delegate_account,
                    &mut owner_account,
                ],
            )
        );

        // no revoke if account is frozen
        Account::unpack_unchecked_mut(&mut account_account.data, &mut |account: &mut Account| {
            account.delegate = COption::Some(delegate_key);
            account.delegated_amount = 100;
            Ok(())
        })
        .unwrap();
        assert_eq!(
            Err(TokenError::AccountFrozen.into()),
            do_process_instruction(
                revoke(&program_id, &account_key, &owner_key, &[]).unwrap(),
                vec![&mut account_account, &mut owner_account],
            )
        );

        // no set authority if account is frozen
        let new_owner_key = pubkey_rand();
        assert_eq!(
            Err(TokenError::AccountFrozen.into()),
            do_process_instruction(
                set_authority(
                    &program_id,
                    &account_key,
                    Some(&new_owner_key),
                    AuthorityType::AccountOwner,
                    &owner_key,
                    &[]
                )
                .unwrap(),
                vec![&mut account_account, &mut owner_account,],
            )
        );

        // no mint_to if destination account is frozen
        assert_eq!(
            Err(TokenError::AccountFrozen.into()),
            do_process_instruction(
                mint_to(&program_id, &mint_key, &account_key, &owner_key, &[], 100).unwrap(),
                vec![&mut mint_account, &mut account_account, &mut owner_account,],
            )
        );

        // no burn if account is frozen
        assert_eq!(
            Err(TokenError::AccountFrozen.into()),
            do_process_instruction(
                burn(&program_id, &account_key, &mint_key, &owner_key, &[], 100).unwrap(),
                vec![&mut account_account, &mut mint_account, &mut owner_account],
            )
        );
    }

    #[test]
    fn test_freeze_account() {
        let program_id = pubkey_rand();
        let account_key = pubkey_rand();
        let mut account_account =
            SolanaAccount::new(account_minimum_balance(), size_of::<Account>(), &program_id);
        let account_owner_key = pubkey_rand();
        let mut account_owner_account = SolanaAccount::default();
        let owner_key = pubkey_rand();
        let mut owner_account = SolanaAccount::default();
        let owner2_key = pubkey_rand();
        let mut owner2_account = SolanaAccount::default();
        let mint_key = pubkey_rand();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), size_of::<Mint>(), &program_id);
        let mut rent_sysvar = rent_sysvar();

        // create new mint with owner different from account owner
        do_process_instruction(
            initialize_mint(&program_id, &mint_key, &owner_key, None, 2).unwrap(),
            vec![&mut mint_account, &mut rent_sysvar],
        )
        .unwrap();

        // create account
        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, &account_owner_key).unwrap(),
            vec![
                &mut account_account,
                &mut mint_account,
                &mut account_owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();

        // mint to account
        do_process_instruction(
            mint_to(&program_id, &mint_key, &account_key, &owner_key, &[], 1000).unwrap(),
            vec![&mut mint_account, &mut account_account, &mut owner_account],
        )
        .unwrap();

        // mint cannot freeze
        assert_eq!(
            Err(TokenError::MintCannotFreeze.into()),
            do_process_instruction(
                freeze_account(&program_id, &account_key, &mint_key, &owner_key, &[]).unwrap(),
                vec![&mut account_account, &mut mint_account, &mut owner_account],
            )
        );

        // missing freeze_authority
        Mint::unpack_unchecked_mut(&mut mint_account.data, &mut |mint: &mut Mint| {
            mint.freeze_authority = COption::Some(owner_key);
            Ok(())
        })
        .unwrap();
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            do_process_instruction(
                freeze_account(&program_id, &account_key, &mint_key, &owner2_key, &[]).unwrap(),
                vec![&mut account_account, &mut mint_account, &mut owner2_account],
            )
        );

        // check explicit thaw
        assert_eq!(
            Err(TokenError::InvalidState.into()),
            do_process_instruction(
                thaw_account(&program_id, &account_key, &mint_key, &owner2_key, &[]).unwrap(),
                vec![&mut account_account, &mut mint_account, &mut owner2_account],
            )
        );

        // freeze
        do_process_instruction(
            freeze_account(&program_id, &account_key, &mint_key, &owner_key, &[]).unwrap(),
            vec![&mut account_account, &mut mint_account, &mut owner_account],
        )
        .unwrap();
        Account::unpack_unchecked_mut(&mut account_account.data, &mut |account: &mut Account| {
            assert_eq!(account.state, AccountState::Frozen);
            Ok(())
        })
        .unwrap();

        // check explicit freeze
        assert_eq!(
            Err(TokenError::InvalidState.into()),
            do_process_instruction(
                freeze_account(&program_id, &account_key, &mint_key, &owner_key, &[]).unwrap(),
                vec![&mut account_account, &mut mint_account, &mut owner_account],
            )
        );

        // check thaw authority
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            do_process_instruction(
                thaw_account(&program_id, &account_key, &mint_key, &owner2_key, &[]).unwrap(),
                vec![&mut account_account, &mut mint_account, &mut owner2_account],
            )
        );

        // thaw
        do_process_instruction(
            thaw_account(&program_id, &account_key, &mint_key, &owner_key, &[]).unwrap(),
            vec![&mut account_account, &mut mint_account, &mut owner_account],
        )
        .unwrap();
        Account::unpack_unchecked_mut(&mut account_account.data, &mut |account: &mut Account| {
            assert_eq!(account.state, AccountState::Initialized);
            Ok(())
        })
        .unwrap();
    }
}
