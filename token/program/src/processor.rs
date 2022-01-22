//! Program state processor

use crate::{
    error::TokenError,
    instruction::{is_valid_signer_index, AuthorityType, TokenInstruction, MAX_SIGNERS},
    state::{Account, AccountState, Mint, Multisig},
};
use num_traits::FromPrimitive;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    decode_error::DecodeError,
    entrypoint::ProgramResult,
    msg,
    program_error::{PrintProgramError, ProgramError},
    program_memory::{sol_memcmp, sol_memset},
    program_option::COption,
    program_pack::{IsInitialized, Pack},
    pubkey::{Pubkey, PUBKEY_BYTES},
    sysvar::{rent::Rent, Sysvar},
};

/// Program state handler.
pub struct Processor {}
impl Processor {
    fn _process_initialize_mint(
        accounts: &[AccountInfo],
        decimals: u8,
        mint_authority: Pubkey,
        freeze_authority: COption<Pubkey>,
        rent_sysvar_account: bool,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let mint_info = next_account_info(account_info_iter)?;
        let mint_data_len = mint_info.data_len();
        let rent = if rent_sysvar_account {
            Rent::from_account_info(next_account_info(account_info_iter)?)?
        } else {
            Rent::get()?
        };

        let mut mint = Mint::unpack_unchecked(&mint_info.data.borrow())?;
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

        Mint::pack(mint, &mut mint_info.data.borrow_mut())?;

        Ok(())
    }

    /// Processes an [InitializeMint](enum.TokenInstruction.html) instruction.
    pub fn process_initialize_mint(
        accounts: &[AccountInfo],
        decimals: u8,
        mint_authority: Pubkey,
        freeze_authority: COption<Pubkey>,
    ) -> ProgramResult {
        Self::_process_initialize_mint(accounts, decimals, mint_authority, freeze_authority, true)
    }

    /// Processes an [InitializeMint2](enum.TokenInstruction.html) instruction.
    pub fn process_initialize_mint2(
        accounts: &[AccountInfo],
        decimals: u8,
        mint_authority: Pubkey,
        freeze_authority: COption<Pubkey>,
    ) -> ProgramResult {
        Self::_process_initialize_mint(accounts, decimals, mint_authority, freeze_authority, false)
    }

    fn _process_initialize_account(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        owner: Option<&Pubkey>,
        rent_sysvar_account: bool,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let new_account_info = next_account_info(account_info_iter)?;
        let mint_info = next_account_info(account_info_iter)?;
        let owner = if let Some(owner) = owner {
            owner
        } else {
            next_account_info(account_info_iter)?.key
        };
        let new_account_info_data_len = new_account_info.data_len();
        let rent = if rent_sysvar_account {
            Rent::from_account_info(next_account_info(account_info_iter)?)?
        } else {
            Rent::get()?
        };

        let mut account = Account::unpack_unchecked(&new_account_info.data.borrow())?;
        if account.is_initialized() {
            return Err(TokenError::AlreadyInUse.into());
        }

        if !rent.is_exempt(new_account_info.lamports(), new_account_info_data_len) {
            return Err(TokenError::NotRentExempt.into());
        }

        let is_native_mint = Self::cmp_pubkeys(mint_info.key, &crate::native_mint::id());
        if !is_native_mint {
            Self::check_account_owner(program_id, mint_info)?;
            let _ = Mint::unpack(&mint_info.data.borrow_mut())
                .map_err(|_| Into::<ProgramError>::into(TokenError::InvalidMint))?;
        }

        account.mint = *mint_info.key;
        account.owner = *owner;
        account.delegate = COption::None;
        account.delegated_amount = 0;
        account.state = AccountState::Initialized;
        if is_native_mint {
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

        Account::pack(account, &mut new_account_info.data.borrow_mut())?;

        Ok(())
    }

    /// Processes an [InitializeAccount](enum.TokenInstruction.html) instruction.
    pub fn process_initialize_account(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        Self::_process_initialize_account(program_id, accounts, None, true)
    }

    /// Processes an [InitializeAccount2](enum.TokenInstruction.html) instruction.
    pub fn process_initialize_account2(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        owner: Pubkey,
    ) -> ProgramResult {
        Self::_process_initialize_account(program_id, accounts, Some(&owner), true)
    }

    /// Processes an [InitializeAccount3](enum.TokenInstruction.html) instruction.
    pub fn process_initialize_account3(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        owner: Pubkey,
    ) -> ProgramResult {
        Self::_process_initialize_account(program_id, accounts, Some(&owner), false)
    }

    fn _process_initialize_multisig(
        accounts: &[AccountInfo],
        m: u8,
        rent_sysvar_account: bool,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let multisig_info = next_account_info(account_info_iter)?;
        let multisig_info_data_len = multisig_info.data_len();
        let rent = if rent_sysvar_account {
            Rent::from_account_info(next_account_info(account_info_iter)?)?
        } else {
            Rent::get()?
        };

        let mut multisig = Multisig::unpack_unchecked(&multisig_info.data.borrow())?;
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

        Multisig::pack(multisig, &mut multisig_info.data.borrow_mut())?;

        Ok(())
    }

    /// Processes a [InitializeMultisig](enum.TokenInstruction.html) instruction.
    pub fn process_initialize_multisig(accounts: &[AccountInfo], m: u8) -> ProgramResult {
        Self::_process_initialize_multisig(accounts, m, true)
    }

    /// Processes a [InitializeMultisig2](enum.TokenInstruction.html) instruction.
    pub fn process_initialize_multisig2(accounts: &[AccountInfo], m: u8) -> ProgramResult {
        Self::_process_initialize_multisig(accounts, m, false)
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

        let mut source_account = Account::unpack(&source_account_info.data.borrow())?;
        let mut dest_account = Account::unpack(&dest_account_info.data.borrow())?;

        if source_account.is_frozen() || dest_account.is_frozen() {
            return Err(TokenError::AccountFrozen.into());
        }
        if source_account.amount < amount {
            return Err(TokenError::InsufficientFunds.into());
        }
        if !Self::cmp_pubkeys(&source_account.mint, &dest_account.mint) {
            return Err(TokenError::MintMismatch.into());
        }

        if let Some((mint_info, expected_decimals)) = expected_mint_info {
            if !Self::cmp_pubkeys(mint_info.key, &source_account.mint) {
                return Err(TokenError::MintMismatch.into());
            }

            let mint = Mint::unpack(&mint_info.data.borrow_mut())?;
            if expected_decimals != mint.decimals {
                return Err(TokenError::MintDecimalsMismatch.into());
            }
        }

        let self_transfer = Self::cmp_pubkeys(source_account_info.key, dest_account_info.key);

        match source_account.delegate {
            COption::Some(ref delegate) if Self::cmp_pubkeys(authority_info.key, delegate) => {
                Self::validate_owner(
                    program_id,
                    delegate,
                    authority_info,
                    account_info_iter.as_slice(),
                )?;
                if source_account.delegated_amount < amount {
                    return Err(TokenError::InsufficientFunds.into());
                }
                if !self_transfer {
                    source_account.delegated_amount = source_account
                        .delegated_amount
                        .checked_sub(amount)
                        .ok_or(TokenError::Overflow)?;
                    if source_account.delegated_amount == 0 {
                        source_account.delegate = COption::None;
                    }
                }
            }
            _ => Self::validate_owner(
                program_id,
                &source_account.owner,
                authority_info,
                account_info_iter.as_slice(),
            )?,
        };

        if self_transfer || amount == 0 {
            Self::check_account_owner(program_id, source_account_info)?;
            Self::check_account_owner(program_id, dest_account_info)?;
        }

        // This check MUST occur just before the amounts are manipulated
        // to ensure self-transfers are fully validated
        if self_transfer {
            return Ok(());
        }

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

        Account::pack(source_account, &mut source_account_info.data.borrow_mut())?;
        Account::pack(dest_account, &mut dest_account_info.data.borrow_mut())?;

        Ok(())
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

        let mut source_account = Account::unpack(&source_account_info.data.borrow())?;

        if source_account.is_frozen() {
            return Err(TokenError::AccountFrozen.into());
        }

        if let Some((mint_info, expected_decimals)) = expected_mint_info {
            if !Self::cmp_pubkeys(mint_info.key, &source_account.mint) {
                return Err(TokenError::MintMismatch.into());
            }

            let mint = Mint::unpack(&mint_info.data.borrow_mut())?;
            if expected_decimals != mint.decimals {
                return Err(TokenError::MintDecimalsMismatch.into());
            }
        }

        Self::validate_owner(
            program_id,
            &source_account.owner,
            owner_info,
            account_info_iter.as_slice(),
        )?;

        source_account.delegate = COption::Some(*delegate_info.key);
        source_account.delegated_amount = amount;

        Account::pack(source_account, &mut source_account_info.data.borrow_mut())?;

        Ok(())
    }

    /// Processes an [Revoke](enum.TokenInstruction.html) instruction.
    pub fn process_revoke(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;

        let mut source_account = Account::unpack(&source_account_info.data.borrow())?;

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

        Account::pack(source_account, &mut source_account_info.data.borrow_mut())?;

        Ok(())
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

        if account_info.data_len() == Account::get_packed_len() {
            let mut account = Account::unpack(&account_info.data.borrow())?;

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

                    account.delegate = COption::None;
                    account.delegated_amount = 0;

                    if account.is_native() {
                        account.close_authority = COption::None;
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
            Account::pack(account, &mut account_info.data.borrow_mut())?;
        } else if account_info.data_len() == Mint::get_packed_len() {
            let mut mint = Mint::unpack(&account_info.data.borrow())?;
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
            Mint::pack(mint, &mut account_info.data.borrow_mut())?;
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

        let mut dest_account = Account::unpack(&dest_account_info.data.borrow())?;
        if dest_account.is_frozen() {
            return Err(TokenError::AccountFrozen.into());
        }

        if dest_account.is_native() {
            return Err(TokenError::NativeNotSupported.into());
        }
        if !Self::cmp_pubkeys(mint_info.key, &dest_account.mint) {
            return Err(TokenError::MintMismatch.into());
        }

        let mut mint = Mint::unpack(&mint_info.data.borrow())?;
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

        if amount == 0 {
            Self::check_account_owner(program_id, mint_info)?;
            Self::check_account_owner(program_id, dest_account_info)?;
        }

        dest_account.amount = dest_account
            .amount
            .checked_add(amount)
            .ok_or(TokenError::Overflow)?;

        mint.supply = mint
            .supply
            .checked_add(amount)
            .ok_or(TokenError::Overflow)?;

        Account::pack(dest_account, &mut dest_account_info.data.borrow_mut())?;
        Mint::pack(mint, &mut mint_info.data.borrow_mut())?;

        Ok(())
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

        let mut source_account = Account::unpack(&source_account_info.data.borrow())?;
        let mut mint = Mint::unpack(&mint_info.data.borrow())?;

        if source_account.is_frozen() {
            return Err(TokenError::AccountFrozen.into());
        }
        if source_account.is_native() {
            return Err(TokenError::NativeNotSupported.into());
        }
        if source_account.amount < amount {
            return Err(TokenError::InsufficientFunds.into());
        }
        if !Self::cmp_pubkeys(mint_info.key, &source_account.mint) {
            return Err(TokenError::MintMismatch.into());
        }

        if let Some(expected_decimals) = expected_decimals {
            if expected_decimals != mint.decimals {
                return Err(TokenError::MintDecimalsMismatch.into());
            }
        }

        match source_account.delegate {
            COption::Some(ref delegate) if Self::cmp_pubkeys(authority_info.key, delegate) => {
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

        if amount == 0 {
            Self::check_account_owner(program_id, source_account_info)?;
            Self::check_account_owner(program_id, mint_info)?;
        }

        source_account.amount = source_account
            .amount
            .checked_sub(amount)
            .ok_or(TokenError::Overflow)?;
        mint.supply = mint
            .supply
            .checked_sub(amount)
            .ok_or(TokenError::Overflow)?;

        Account::pack(source_account, &mut source_account_info.data.borrow_mut())?;
        Mint::pack(mint, &mut mint_info.data.borrow_mut())?;

        Ok(())
    }

    /// Processes a [CloseAccount](enum.TokenInstruction.html) instruction.
    pub fn process_close_account(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let dest_account_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;

        if Self::cmp_pubkeys(source_account_info.key, dest_account_info.key) {
            return Err(ProgramError::InvalidAccountData);
        }

        let source_account = Account::unpack(&source_account_info.data.borrow())?;
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

        sol_memset(*source_account_info.data.borrow_mut(), 0, Account::LEN);

        Ok(())
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

        let mut source_account = Account::unpack(&source_account_info.data.borrow())?;
        if freeze && source_account.is_frozen() || !freeze && !source_account.is_frozen() {
            return Err(TokenError::InvalidState.into());
        }
        if source_account.is_native() {
            return Err(TokenError::NativeNotSupported.into());
        }
        if !Self::cmp_pubkeys(mint_info.key, &source_account.mint) {
            return Err(TokenError::MintMismatch.into());
        }

        let mint = Mint::unpack(&mint_info.data.borrow_mut())?;
        match mint.freeze_authority {
            COption::Some(authority) => Self::validate_owner(
                program_id,
                &authority,
                authority_info,
                account_info_iter.as_slice(),
            ),
            COption::None => Err(TokenError::MintCannotFreeze.into()),
        }?;

        source_account.state = if freeze {
            AccountState::Frozen
        } else {
            AccountState::Initialized
        };

        Account::pack(source_account, &mut source_account_info.data.borrow_mut())?;

        Ok(())
    }

    /// Processes a [SyncNative](enum.TokenInstruction.html) instruction
    pub fn process_sync_native(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let native_account_info = next_account_info(account_info_iter)?;
        Self::check_account_owner(program_id, native_account_info)?;

        let mut native_account = Account::unpack(&native_account_info.data.borrow())?;

        if let COption::Some(rent_exempt_reserve) = native_account.is_native {
            let new_amount = native_account_info
                .lamports()
                .checked_sub(rent_exempt_reserve)
                .ok_or(TokenError::Overflow)?;
            if new_amount < native_account.amount {
                return Err(TokenError::InvalidState.into());
            }
            native_account.amount = new_amount;
        } else {
            return Err(TokenError::NonNativeNotSupported.into());
        }

        Account::pack(native_account, &mut native_account_info.data.borrow_mut())?;
        Ok(())
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
                msg!("Instruction: InitializeMint");
                Self::process_initialize_mint(accounts, decimals, mint_authority, freeze_authority)
            }
            TokenInstruction::InitializeMint2 {
                decimals,
                mint_authority,
                freeze_authority,
            } => {
                msg!("Instruction: InitializeMint2");
                Self::process_initialize_mint2(accounts, decimals, mint_authority, freeze_authority)
            }
            TokenInstruction::InitializeAccount => {
                msg!("Instruction: InitializeAccount");
                Self::process_initialize_account(program_id, accounts)
            }
            TokenInstruction::InitializeAccount2 { owner } => {
                msg!("Instruction: InitializeAccount2");
                Self::process_initialize_account2(program_id, accounts, owner)
            }
            TokenInstruction::InitializeAccount3 { owner } => {
                msg!("Instruction: InitializeAccount3");
                Self::process_initialize_account3(program_id, accounts, owner)
            }
            TokenInstruction::InitializeMultisig { m } => {
                msg!("Instruction: InitializeMultisig");
                Self::process_initialize_multisig(accounts, m)
            }
            TokenInstruction::InitializeMultisig2 { m } => {
                msg!("Instruction: InitializeMultisig2");
                Self::process_initialize_multisig2(accounts, m)
            }
            TokenInstruction::Transfer { amount } => {
                msg!("Instruction: Transfer");
                Self::process_transfer(program_id, accounts, amount, None)
            }
            TokenInstruction::Approve { amount } => {
                msg!("Instruction: Approve");
                Self::process_approve(program_id, accounts, amount, None)
            }
            TokenInstruction::Revoke => {
                msg!("Instruction: Revoke");
                Self::process_revoke(program_id, accounts)
            }
            TokenInstruction::SetAuthority {
                authority_type,
                new_authority,
            } => {
                msg!("Instruction: SetAuthority");
                Self::process_set_authority(program_id, accounts, authority_type, new_authority)
            }
            TokenInstruction::MintTo { amount } => {
                msg!("Instruction: MintTo");
                Self::process_mint_to(program_id, accounts, amount, None)
            }
            TokenInstruction::Burn { amount } => {
                msg!("Instruction: Burn");
                Self::process_burn(program_id, accounts, amount, None)
            }
            TokenInstruction::CloseAccount => {
                msg!("Instruction: CloseAccount");
                Self::process_close_account(program_id, accounts)
            }
            TokenInstruction::FreezeAccount => {
                msg!("Instruction: FreezeAccount");
                Self::process_toggle_freeze_account(program_id, accounts, true)
            }
            TokenInstruction::ThawAccount => {
                msg!("Instruction: ThawAccount");
                Self::process_toggle_freeze_account(program_id, accounts, false)
            }
            TokenInstruction::TransferChecked { amount, decimals } => {
                msg!("Instruction: TransferChecked");
                Self::process_transfer(program_id, accounts, amount, Some(decimals))
            }
            TokenInstruction::ApproveChecked { amount, decimals } => {
                msg!("Instruction: ApproveChecked");
                Self::process_approve(program_id, accounts, amount, Some(decimals))
            }
            TokenInstruction::MintToChecked { amount, decimals } => {
                msg!("Instruction: MintToChecked");
                Self::process_mint_to(program_id, accounts, amount, Some(decimals))
            }
            TokenInstruction::BurnChecked { amount, decimals } => {
                msg!("Instruction: BurnChecked");
                Self::process_burn(program_id, accounts, amount, Some(decimals))
            }
            TokenInstruction::SyncNative => {
                msg!("Instruction: SyncNative");
                Self::process_sync_native(program_id, accounts)
            }
        }
    }

    /// Checks that the account is owned by the expected program
    pub fn check_account_owner(program_id: &Pubkey, account_info: &AccountInfo) -> ProgramResult {
        if !Self::cmp_pubkeys(program_id, account_info.owner) {
            Err(ProgramError::IncorrectProgramId)
        } else {
            Ok(())
        }
    }

    /// Checks two pubkeys for equality in a computationally cheap way using
    /// `sol_memcmp`
    pub fn cmp_pubkeys(a: &Pubkey, b: &Pubkey) -> bool {
        sol_memcmp(a.as_ref(), b.as_ref(), PUBKEY_BYTES) == 0
    }

    /// Validates owner(s) are present
    pub fn validate_owner(
        program_id: &Pubkey,
        expected_owner: &Pubkey,
        owner_account_info: &AccountInfo,
        signers: &[AccountInfo],
    ) -> ProgramResult {
        if !Self::cmp_pubkeys(expected_owner, owner_account_info.key) {
            return Err(TokenError::OwnerMismatch.into());
        }
        if Self::cmp_pubkeys(program_id, owner_account_info.owner)
            && owner_account_info.data_len() == Multisig::get_packed_len()
        {
            let multisig = Multisig::unpack(&owner_account_info.data.borrow())?;
            let mut num_signers = 0;
            let mut matched = [false; MAX_SIGNERS];
            for signer in signers.iter() {
                for (position, key) in multisig.signers[0..multisig.n as usize].iter().enumerate() {
                    if Self::cmp_pubkeys(key, signer.key) && !matched[position] {
                        if !signer.is_signer {
                            return Err(ProgramError::MissingRequiredSignature);
                        }
                        matched[position] = true;
                        num_signers += 1;
                    }
                }
            }
            if num_signers < multisig.m {
                return Err(ProgramError::MissingRequiredSignature);
            }
            return Ok(());
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
            TokenError::NotRentExempt => msg!("Error: Lamport balance below rent-exempt threshold"),
            TokenError::InsufficientFunds => msg!("Error: insufficient funds"),
            TokenError::InvalidMint => msg!("Error: Invalid Mint"),
            TokenError::MintMismatch => msg!("Error: Account not associated with this Mint"),
            TokenError::OwnerMismatch => msg!("Error: owner does not match"),
            TokenError::FixedSupply => msg!("Error: the total supply of this token is fixed"),
            TokenError::AlreadyInUse => msg!("Error: account or token already in use"),
            TokenError::InvalidNumberOfProvidedSigners => {
                msg!("Error: Invalid number of provided signers")
            }
            TokenError::InvalidNumberOfRequiredSigners => {
                msg!("Error: Invalid number of required signers")
            }
            TokenError::UninitializedState => msg!("Error: State is uninitialized"),
            TokenError::NativeNotSupported => {
                msg!("Error: Instruction does not support native tokens")
            }
            TokenError::NonNativeHasBalance => {
                msg!("Error: Non-native account can only be closed if its balance is zero")
            }
            TokenError::InvalidInstruction => msg!("Error: Invalid instruction"),
            TokenError::InvalidState => msg!("Error: Invalid account state for operation"),
            TokenError::Overflow => msg!("Error: Operation overflowed"),
            TokenError::AuthorityTypeNotSupported => {
                msg!("Error: Account does not support specified authority type")
            }
            TokenError::MintCannotFreeze => msg!("Error: This token mint cannot freeze accounts"),
            TokenError::AccountFrozen => msg!("Error: Account is frozen"),
            TokenError::MintDecimalsMismatch => {
                msg!("Error: decimals different from the Mint decimals")
            }
            TokenError::NonNativeNotSupported => {
                msg!("Error: Instruction does not support non-native tokens")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::*;
    use solana_program::{
        account_info::IntoAccountInfo, clock::Epoch, instruction::Instruction, program_error,
        sysvar::rent,
    };
    use solana_sdk::account::{
        create_account_for_test, create_is_signer_account_infos, Account as SolanaAccount,
    };

    struct SyscallStubs {}
    impl solana_sdk::program_stubs::SyscallStubs for SyscallStubs {
        fn sol_log(&self, _message: &str) {}

        fn sol_invoke_signed(
            &self,
            _instruction: &Instruction,
            _account_infos: &[AccountInfo],
            _signers_seeds: &[&[&[u8]]],
        ) -> ProgramResult {
            Err(ProgramError::Custom(42)) // Not supported
        }

        fn sol_get_clock_sysvar(&self, _var_addr: *mut u8) -> u64 {
            program_error::UNSUPPORTED_SYSVAR
        }

        fn sol_get_epoch_schedule_sysvar(&self, _var_addr: *mut u8) -> u64 {
            program_error::UNSUPPORTED_SYSVAR
        }

        #[allow(deprecated)]
        fn sol_get_fees_sysvar(&self, _var_addr: *mut u8) -> u64 {
            program_error::UNSUPPORTED_SYSVAR
        }

        fn sol_get_rent_sysvar(&self, var_addr: *mut u8) -> u64 {
            unsafe {
                *(var_addr as *mut _ as *mut Rent) = Rent::default();
            }
            solana_program::entrypoint::SUCCESS
        }
    }

    fn do_process_instruction(
        instruction: Instruction,
        accounts: Vec<&mut SolanaAccount>,
    ) -> ProgramResult {
        {
            use std::sync::Once;
            static ONCE: Once = Once::new();

            ONCE.call_once(|| {
                solana_sdk::program_stubs::set_syscall_stubs(Box::new(SyscallStubs {}));
            });
        }

        let mut meta = instruction
            .accounts
            .iter()
            .zip(accounts)
            .map(|(account_meta, account)| (&account_meta.pubkey, account_meta.is_signer, account))
            .collect::<Vec<_>>();

        let account_infos = create_is_signer_account_infos(&mut meta);
        Processor::process(&instruction.program_id, &account_infos, &instruction.data)
    }

    fn do_process_instruction_dups(
        instruction: Instruction,
        account_infos: Vec<AccountInfo>,
    ) -> ProgramResult {
        Processor::process(&instruction.program_id, &account_infos, &instruction.data)
    }

    fn return_token_error_as_program_error() -> ProgramError {
        TokenError::MintMismatch.into()
    }

    fn rent_sysvar() -> SolanaAccount {
        create_account_for_test(&Rent::default())
    }

    fn mint_minimum_balance() -> u64 {
        Rent::default().minimum_balance(Mint::get_packed_len())
    }

    fn account_minimum_balance() -> u64 {
        Rent::default().minimum_balance(Account::get_packed_len())
    }

    fn multisig_minimum_balance() -> u64 {
        Rent::default().minimum_balance(Multisig::get_packed_len())
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
        assert_ne!(Mint::get_packed_len(), 0);
        assert_ne!(Mint::get_packed_len(), Account::get_packed_len());
        assert_ne!(Mint::get_packed_len(), Multisig::get_packed_len());
        assert_ne!(Account::get_packed_len(), 0);
        assert_ne!(Account::get_packed_len(), Multisig::get_packed_len());
        assert_ne!(Multisig::get_packed_len(), 0);
    }

    #[test]
    fn test_pack_unpack() {
        // Mint
        let check = Mint {
            mint_authority: COption::Some(Pubkey::new(&[1; 32])),
            supply: 42,
            decimals: 7,
            is_initialized: true,
            freeze_authority: COption::Some(Pubkey::new(&[2; 32])),
        };
        let mut packed = vec![0; Mint::get_packed_len() + 1];
        assert_eq!(
            Err(ProgramError::InvalidAccountData),
            Mint::pack(check, &mut packed)
        );
        let mut packed = vec![0; Mint::get_packed_len() - 1];
        assert_eq!(
            Err(ProgramError::InvalidAccountData),
            Mint::pack(check, &mut packed)
        );
        let mut packed = vec![0; Mint::get_packed_len()];
        Mint::pack(check, &mut packed).unwrap();
        let expect = vec![
            1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 42, 0, 0, 0, 0, 0, 0, 0, 7, 1, 1, 0, 0, 0, 2, 2, 2, 2, 2, 2, 2, 2,
            2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
        ];
        assert_eq!(packed, expect);
        let unpacked = Mint::unpack(&packed).unwrap();
        assert_eq!(unpacked, check);

        // Account
        let check = Account {
            mint: Pubkey::new(&[1; 32]),
            owner: Pubkey::new(&[2; 32]),
            amount: 3,
            delegate: COption::Some(Pubkey::new(&[4; 32])),
            state: AccountState::Frozen,
            is_native: COption::Some(5),
            delegated_amount: 6,
            close_authority: COption::Some(Pubkey::new(&[7; 32])),
        };
        let mut packed = vec![0; Account::get_packed_len() + 1];
        assert_eq!(
            Err(ProgramError::InvalidAccountData),
            Account::pack(check, &mut packed)
        );
        let mut packed = vec![0; Account::get_packed_len() - 1];
        assert_eq!(
            Err(ProgramError::InvalidAccountData),
            Account::pack(check, &mut packed)
        );
        let mut packed = vec![0; Account::get_packed_len()];
        Account::pack(check, &mut packed).unwrap();
        let expect = vec![
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
            2, 2, 2, 2, 2, 2, 3, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
            4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 2, 1, 0, 0, 0, 5, 0, 0,
            0, 0, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
            7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
        ];
        assert_eq!(packed, expect);
        let unpacked = Account::unpack(&packed).unwrap();
        assert_eq!(unpacked, check);

        // Multisig
        let check = Multisig {
            m: 1,
            n: 2,
            is_initialized: true,
            signers: [Pubkey::new(&[3; 32]); MAX_SIGNERS],
        };
        let mut packed = vec![0; Multisig::get_packed_len() + 1];
        assert_eq!(
            Err(ProgramError::InvalidAccountData),
            Multisig::pack(check, &mut packed)
        );
        let mut packed = vec![0; Multisig::get_packed_len() - 1];
        assert_eq!(
            Err(ProgramError::InvalidAccountData),
            Multisig::pack(check, &mut packed)
        );
        let mut packed = vec![0; Multisig::get_packed_len()];
        Multisig::pack(check, &mut packed).unwrap();
        let expect = vec![
            1, 2, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3,
        ];
        assert_eq!(packed, expect);
        let unpacked = Multisig::unpack(&packed).unwrap();
        assert_eq!(unpacked, check);
    }

    #[test]
    fn test_initialize_mint() {
        let program_id = crate::id();
        let owner_key = Pubkey::new_unique();
        let mint_key = Pubkey::new_unique();
        let mut mint_account = SolanaAccount::new(42, Mint::get_packed_len(), &program_id);
        let mint2_key = Pubkey::new_unique();
        let mut mint2_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
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
        let mint = Mint::unpack_unchecked(&mint2_account.data).unwrap();
        assert_eq!(mint.freeze_authority, COption::Some(owner_key));
    }

    #[test]
    fn test_initialize_mint2() {
        let program_id = crate::id();
        let owner_key = Pubkey::new_unique();
        let mint_key = Pubkey::new_unique();
        let mut mint_account = SolanaAccount::new(42, Mint::get_packed_len(), &program_id);
        let mint2_key = Pubkey::new_unique();
        let mut mint2_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);

        // mint is not rent exempt
        assert_eq!(
            Err(TokenError::NotRentExempt.into()),
            do_process_instruction(
                initialize_mint2(&program_id, &mint_key, &owner_key, None, 2).unwrap(),
                vec![&mut mint_account]
            )
        );

        mint_account.lamports = mint_minimum_balance();

        // create new mint
        do_process_instruction(
            initialize_mint2(&program_id, &mint_key, &owner_key, None, 2).unwrap(),
            vec![&mut mint_account],
        )
        .unwrap();

        // create twice
        assert_eq!(
            Err(TokenError::AlreadyInUse.into()),
            do_process_instruction(
                initialize_mint2(&program_id, &mint_key, &owner_key, None, 2,).unwrap(),
                vec![&mut mint_account]
            )
        );

        // create another mint that can freeze
        do_process_instruction(
            initialize_mint2(&program_id, &mint2_key, &owner_key, Some(&owner_key), 2).unwrap(),
            vec![&mut mint2_account],
        )
        .unwrap();
        let mint = Mint::unpack_unchecked(&mint2_account.data).unwrap();
        assert_eq!(mint.freeze_authority, COption::Some(owner_key));
    }

    #[test]
    fn test_initialize_mint_account() {
        let program_id = crate::id();
        let account_key = Pubkey::new_unique();
        let mut account_account = SolanaAccount::new(42, Account::get_packed_len(), &program_id);
        let owner_key = Pubkey::new_unique();
        let mut owner_account = SolanaAccount::default();
        let mint_key = Pubkey::new_unique();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
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

        // mint not owned by program
        let not_program_id = Pubkey::new_unique();
        mint_account.owner = not_program_id;
        assert_eq!(
            Err(ProgramError::IncorrectProgramId),
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
        mint_account.owner = program_id;

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
    fn test_transfer_dups() {
        let program_id = crate::id();
        let account1_key = Pubkey::new_unique();
        let mut account1_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let mut account1_info: AccountInfo = (&account1_key, true, &mut account1_account).into();
        let account2_key = Pubkey::new_unique();
        let mut account2_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let mut account2_info: AccountInfo = (&account2_key, false, &mut account2_account).into();
        let account3_key = Pubkey::new_unique();
        let mut account3_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let account3_info: AccountInfo = (&account3_key, false, &mut account3_account).into();
        let account4_key = Pubkey::new_unique();
        let mut account4_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let account4_info: AccountInfo = (&account4_key, true, &mut account4_account).into();
        let multisig_key = Pubkey::new_unique();
        let mut multisig_account = SolanaAccount::new(
            multisig_minimum_balance(),
            Multisig::get_packed_len(),
            &program_id,
        );
        let multisig_info: AccountInfo = (&multisig_key, true, &mut multisig_account).into();
        let owner_key = Pubkey::new_unique();
        let mut owner_account = SolanaAccount::default();
        let owner_info: AccountInfo = (&owner_key, true, &mut owner_account).into();
        let mint_key = Pubkey::new_unique();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
        let mint_info: AccountInfo = (&mint_key, false, &mut mint_account).into();
        let rent_key = rent::id();
        let mut rent_sysvar = rent_sysvar();
        let rent_info: AccountInfo = (&rent_key, false, &mut rent_sysvar).into();

        // create mint
        do_process_instruction_dups(
            initialize_mint(&program_id, &mint_key, &owner_key, None, 2).unwrap(),
            vec![mint_info.clone(), rent_info.clone()],
        )
        .unwrap();

        // create account
        do_process_instruction_dups(
            initialize_account(&program_id, &account1_key, &mint_key, &account1_key).unwrap(),
            vec![
                account1_info.clone(),
                mint_info.clone(),
                account1_info.clone(),
                rent_info.clone(),
            ],
        )
        .unwrap();

        // create another account
        do_process_instruction_dups(
            initialize_account(&program_id, &account2_key, &mint_key, &owner_key).unwrap(),
            vec![
                account2_info.clone(),
                mint_info.clone(),
                owner_info.clone(),
                rent_info.clone(),
            ],
        )
        .unwrap();

        // mint to account
        do_process_instruction_dups(
            mint_to(&program_id, &mint_key, &account1_key, &owner_key, &[], 1000).unwrap(),
            vec![mint_info.clone(), account1_info.clone(), owner_info.clone()],
        )
        .unwrap();

        // source-owner transfer
        do_process_instruction_dups(
            transfer(
                &program_id,
                &account1_key,
                &account2_key,
                &account1_key,
                &[],
                500,
            )
            .unwrap(),
            vec![
                account1_info.clone(),
                account2_info.clone(),
                account1_info.clone(),
            ],
        )
        .unwrap();

        // source-owner TransferChecked
        do_process_instruction_dups(
            transfer_checked(
                &program_id,
                &account1_key,
                &mint_key,
                &account2_key,
                &account1_key,
                &[],
                500,
                2,
            )
            .unwrap(),
            vec![
                account1_info.clone(),
                mint_info.clone(),
                account2_info.clone(),
                account1_info.clone(),
            ],
        )
        .unwrap();

        // source-delegate transfer
        let mut account = Account::unpack_unchecked(&account1_info.data.borrow()).unwrap();
        account.amount = 1000;
        account.delegated_amount = 1000;
        account.delegate = COption::Some(account1_key);
        account.owner = owner_key;
        Account::pack(account, &mut account1_info.data.borrow_mut()).unwrap();

        do_process_instruction_dups(
            transfer(
                &program_id,
                &account1_key,
                &account2_key,
                &account1_key,
                &[],
                500,
            )
            .unwrap(),
            vec![
                account1_info.clone(),
                account2_info.clone(),
                account1_info.clone(),
            ],
        )
        .unwrap();

        // source-delegate TransferChecked
        do_process_instruction_dups(
            transfer_checked(
                &program_id,
                &account1_key,
                &mint_key,
                &account2_key,
                &account1_key,
                &[],
                500,
                2,
            )
            .unwrap(),
            vec![
                account1_info.clone(),
                mint_info.clone(),
                account2_info.clone(),
                account1_info.clone(),
            ],
        )
        .unwrap();

        // test destination-owner transfer
        do_process_instruction_dups(
            initialize_account(&program_id, &account3_key, &mint_key, &account2_key).unwrap(),
            vec![
                account3_info.clone(),
                mint_info.clone(),
                account2_info.clone(),
                rent_info.clone(),
            ],
        )
        .unwrap();
        do_process_instruction_dups(
            mint_to(&program_id, &mint_key, &account3_key, &owner_key, &[], 1000).unwrap(),
            vec![mint_info.clone(), account3_info.clone(), owner_info.clone()],
        )
        .unwrap();

        account1_info.is_signer = false;
        account2_info.is_signer = true;
        do_process_instruction_dups(
            transfer(
                &program_id,
                &account3_key,
                &account2_key,
                &account2_key,
                &[],
                500,
            )
            .unwrap(),
            vec![
                account3_info.clone(),
                account2_info.clone(),
                account2_info.clone(),
            ],
        )
        .unwrap();

        // destination-owner TransferChecked
        do_process_instruction_dups(
            transfer_checked(
                &program_id,
                &account3_key,
                &mint_key,
                &account2_key,
                &account2_key,
                &[],
                500,
                2,
            )
            .unwrap(),
            vec![
                account3_info.clone(),
                mint_info.clone(),
                account2_info.clone(),
                account2_info.clone(),
            ],
        )
        .unwrap();

        // test source-multisig signer
        do_process_instruction_dups(
            initialize_multisig(&program_id, &multisig_key, &[&account4_key], 1).unwrap(),
            vec![
                multisig_info.clone(),
                rent_info.clone(),
                account4_info.clone(),
            ],
        )
        .unwrap();

        do_process_instruction_dups(
            initialize_account(&program_id, &account4_key, &mint_key, &multisig_key).unwrap(),
            vec![
                account4_info.clone(),
                mint_info.clone(),
                multisig_info.clone(),
                rent_info.clone(),
            ],
        )
        .unwrap();

        do_process_instruction_dups(
            mint_to(&program_id, &mint_key, &account4_key, &owner_key, &[], 1000).unwrap(),
            vec![mint_info.clone(), account4_info.clone(), owner_info.clone()],
        )
        .unwrap();

        // source-multisig-signer transfer
        do_process_instruction_dups(
            transfer(
                &program_id,
                &account4_key,
                &account2_key,
                &multisig_key,
                &[&account4_key],
                500,
            )
            .unwrap(),
            vec![
                account4_info.clone(),
                account2_info.clone(),
                multisig_info.clone(),
                account4_info.clone(),
            ],
        )
        .unwrap();

        // source-multisig-signer TransferChecked
        do_process_instruction_dups(
            transfer_checked(
                &program_id,
                &account4_key,
                &mint_key,
                &account2_key,
                &multisig_key,
                &[&account4_key],
                500,
                2,
            )
            .unwrap(),
            vec![
                account4_info.clone(),
                mint_info.clone(),
                account2_info.clone(),
                multisig_info.clone(),
                account4_info.clone(),
            ],
        )
        .unwrap();
    }

    #[test]
    fn test_transfer() {
        let program_id = crate::id();
        let account_key = Pubkey::new_unique();
        let mut account_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let account2_key = Pubkey::new_unique();
        let mut account2_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let account3_key = Pubkey::new_unique();
        let mut account3_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let delegate_key = Pubkey::new_unique();
        let mut delegate_account = SolanaAccount::default();
        let mismatch_key = Pubkey::new_unique();
        let mut mismatch_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let owner_key = Pubkey::new_unique();
        let mut owner_account = SolanaAccount::default();
        let owner2_key = Pubkey::new_unique();
        let mut owner2_account = SolanaAccount::default();
        let mint_key = Pubkey::new_unique();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
        let mint2_key = Pubkey::new_unique();
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
        let mut account = Account::unpack_unchecked(&mismatch_account.data).unwrap();
        account.mint = mint2_key;
        Account::pack(account, &mut mismatch_account.data).unwrap();

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

        // account not owned by program
        let not_program_id = Pubkey::new_unique();
        account_account.owner = not_program_id;
        assert_eq!(
            Err(ProgramError::IncorrectProgramId),
            do_process_instruction(
                transfer(&program_id, &account_key, &account2_key, &owner_key, &[], 0,).unwrap(),
                vec![
                    &mut account_account,
                    &mut account2_account,
                    &mut owner2_account,
                ],
            )
        );
        account_account.owner = program_id;

        // account 2 not owned by program
        let not_program_id = Pubkey::new_unique();
        account2_account.owner = not_program_id;
        assert_eq!(
            Err(ProgramError::IncorrectProgramId),
            do_process_instruction(
                transfer(&program_id, &account_key, &account2_key, &owner_key, &[], 0,).unwrap(),
                vec![
                    &mut account_account,
                    &mut account2_account,
                    &mut owner2_account,
                ],
            )
        );
        account2_account.owner = program_id;

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
                transfer_checked(
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
                transfer_checked(
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
            transfer_checked(
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
    fn test_self_transfer() {
        let program_id = crate::id();
        let account_key = Pubkey::new_unique();
        let mut account_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let account2_key = Pubkey::new_unique();
        let mut account2_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let account3_key = Pubkey::new_unique();
        let mut account3_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let delegate_key = Pubkey::new_unique();
        let mut delegate_account = SolanaAccount::default();
        let owner_key = Pubkey::new_unique();
        let mut owner_account = SolanaAccount::default();
        let owner2_key = Pubkey::new_unique();
        let mut owner2_account = SolanaAccount::default();
        let mint_key = Pubkey::new_unique();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
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

        // mint to account
        do_process_instruction(
            mint_to(&program_id, &mint_key, &account_key, &owner_key, &[], 1000).unwrap(),
            vec![&mut mint_account, &mut account_account, &mut owner_account],
        )
        .unwrap();

        let account_info = (&account_key, false, &mut account_account).into_account_info();
        let account3_info = (&account3_key, false, &mut account3_account).into_account_info();
        let delegate_info = (&delegate_key, true, &mut delegate_account).into_account_info();
        let owner_info = (&owner_key, true, &mut owner_account).into_account_info();
        let owner2_info = (&owner2_key, true, &mut owner2_account).into_account_info();
        let mint_info = (&mint_key, false, &mut mint_account).into_account_info();

        // transfer
        let instruction = transfer(
            &program_id,
            account_info.key,
            account_info.key,
            owner_info.key,
            &[],
            1000,
        )
        .unwrap();
        assert_eq!(
            Ok(()),
            Processor::process(
                &instruction.program_id,
                &[
                    account_info.clone(),
                    account_info.clone(),
                    owner_info.clone(),
                ],
                &instruction.data,
            )
        );
        // no balance change...
        let account = Account::unpack_unchecked(&account_info.try_borrow_data().unwrap()).unwrap();
        assert_eq!(account.amount, 1000);

        // transfer checked
        let instruction = transfer_checked(
            &program_id,
            account_info.key,
            mint_info.key,
            account_info.key,
            owner_info.key,
            &[],
            1000,
            2,
        )
        .unwrap();
        assert_eq!(
            Ok(()),
            Processor::process(
                &instruction.program_id,
                &[
                    account_info.clone(),
                    mint_info.clone(),
                    account_info.clone(),
                    owner_info.clone(),
                ],
                &instruction.data,
            )
        );
        // no balance change...
        let account = Account::unpack_unchecked(&account_info.try_borrow_data().unwrap()).unwrap();
        assert_eq!(account.amount, 1000);

        // missing signer
        let mut owner_no_sign_info = owner_info.clone();
        let mut instruction = transfer(
            &program_id,
            account_info.key,
            account_info.key,
            owner_no_sign_info.key,
            &[],
            1000,
        )
        .unwrap();
        instruction.accounts[2].is_signer = false;
        owner_no_sign_info.is_signer = false;
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            Processor::process(
                &instruction.program_id,
                &[
                    account_info.clone(),
                    account_info.clone(),
                    owner_no_sign_info.clone(),
                ],
                &instruction.data,
            )
        );

        // missing signer checked
        let mut instruction = transfer_checked(
            &program_id,
            account_info.key,
            mint_info.key,
            account_info.key,
            owner_no_sign_info.key,
            &[],
            1000,
            2,
        )
        .unwrap();
        instruction.accounts[3].is_signer = false;
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            Processor::process(
                &instruction.program_id,
                &[
                    account_info.clone(),
                    mint_info.clone(),
                    account_info.clone(),
                    owner_no_sign_info,
                ],
                &instruction.data,
            )
        );

        // missing owner
        let instruction = transfer(
            &program_id,
            account_info.key,
            account_info.key,
            owner2_info.key,
            &[],
            1000,
        )
        .unwrap();
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            Processor::process(
                &instruction.program_id,
                &[
                    account_info.clone(),
                    account_info.clone(),
                    owner2_info.clone(),
                ],
                &instruction.data,
            )
        );

        // missing owner checked
        let instruction = transfer_checked(
            &program_id,
            account_info.key,
            mint_info.key,
            account_info.key,
            owner2_info.key,
            &[],
            1000,
            2,
        )
        .unwrap();
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            Processor::process(
                &instruction.program_id,
                &[
                    account_info.clone(),
                    mint_info.clone(),
                    account_info.clone(),
                    owner2_info.clone(),
                ],
                &instruction.data,
            )
        );

        // insufficient funds
        let instruction = transfer(
            &program_id,
            account_info.key,
            account_info.key,
            owner_info.key,
            &[],
            1001,
        )
        .unwrap();
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            Processor::process(
                &instruction.program_id,
                &[
                    account_info.clone(),
                    account_info.clone(),
                    owner_info.clone(),
                ],
                &instruction.data,
            )
        );

        // insufficient funds checked
        let instruction = transfer_checked(
            &program_id,
            account_info.key,
            mint_info.key,
            account_info.key,
            owner_info.key,
            &[],
            1001,
            2,
        )
        .unwrap();
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            Processor::process(
                &instruction.program_id,
                &[
                    account_info.clone(),
                    mint_info.clone(),
                    account_info.clone(),
                    owner_info.clone(),
                ],
                &instruction.data,
            )
        );

        // incorrect decimals
        let instruction = transfer_checked(
            &program_id,
            account_info.key,
            mint_info.key,
            account_info.key,
            owner_info.key,
            &[],
            1,
            10, // <-- incorrect decimals
        )
        .unwrap();
        assert_eq!(
            Err(TokenError::MintDecimalsMismatch.into()),
            Processor::process(
                &instruction.program_id,
                &[
                    account_info.clone(),
                    mint_info.clone(),
                    account_info.clone(),
                    owner_info.clone(),
                ],
                &instruction.data,
            )
        );

        // incorrect mint
        let instruction = transfer_checked(
            &program_id,
            account_info.key,
            account3_info.key, // <-- incorrect mint
            account_info.key,
            owner_info.key,
            &[],
            1,
            2,
        )
        .unwrap();
        assert_eq!(
            Err(TokenError::MintMismatch.into()),
            Processor::process(
                &instruction.program_id,
                &[
                    account_info.clone(),
                    account3_info.clone(), // <-- incorrect mint
                    account_info.clone(),
                    owner_info.clone(),
                ],
                &instruction.data,
            )
        );

        // approve delegate
        let instruction = approve(
            &program_id,
            account_info.key,
            delegate_info.key,
            owner_info.key,
            &[],
            100,
        )
        .unwrap();
        Processor::process(
            &instruction.program_id,
            &[
                account_info.clone(),
                delegate_info.clone(),
                owner_info.clone(),
            ],
            &instruction.data,
        )
        .unwrap();

        // delegate transfer
        let instruction = transfer(
            &program_id,
            account_info.key,
            account_info.key,
            delegate_info.key,
            &[],
            100,
        )
        .unwrap();
        assert_eq!(
            Ok(()),
            Processor::process(
                &instruction.program_id,
                &[
                    account_info.clone(),
                    account_info.clone(),
                    delegate_info.clone(),
                ],
                &instruction.data,
            )
        );
        // no balance change...
        let account = Account::unpack_unchecked(&account_info.try_borrow_data().unwrap()).unwrap();
        assert_eq!(account.amount, 1000);
        assert_eq!(account.delegated_amount, 100);

        // delegate transfer checked
        let instruction = transfer_checked(
            &program_id,
            account_info.key,
            mint_info.key,
            account_info.key,
            delegate_info.key,
            &[],
            100,
            2,
        )
        .unwrap();
        assert_eq!(
            Ok(()),
            Processor::process(
                &instruction.program_id,
                &[
                    account_info.clone(),
                    mint_info.clone(),
                    account_info.clone(),
                    delegate_info.clone(),
                ],
                &instruction.data,
            )
        );
        // no balance change...
        let account = Account::unpack_unchecked(&account_info.try_borrow_data().unwrap()).unwrap();
        assert_eq!(account.amount, 1000);
        assert_eq!(account.delegated_amount, 100);

        // delegate insufficient funds
        let instruction = transfer(
            &program_id,
            account_info.key,
            account_info.key,
            delegate_info.key,
            &[],
            101,
        )
        .unwrap();
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            Processor::process(
                &instruction.program_id,
                &[
                    account_info.clone(),
                    account_info.clone(),
                    delegate_info.clone(),
                ],
                &instruction.data,
            )
        );

        // delegate insufficient funds checked
        let instruction = transfer_checked(
            &program_id,
            account_info.key,
            mint_info.key,
            account_info.key,
            delegate_info.key,
            &[],
            101,
            2,
        )
        .unwrap();
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            Processor::process(
                &instruction.program_id,
                &[
                    account_info.clone(),
                    mint_info.clone(),
                    account_info.clone(),
                    delegate_info.clone(),
                ],
                &instruction.data,
            )
        );

        // owner transfer with delegate assigned
        let instruction = transfer(
            &program_id,
            account_info.key,
            account_info.key,
            owner_info.key,
            &[],
            1000,
        )
        .unwrap();
        assert_eq!(
            Ok(()),
            Processor::process(
                &instruction.program_id,
                &[
                    account_info.clone(),
                    account_info.clone(),
                    owner_info.clone(),
                ],
                &instruction.data,
            )
        );
        // no balance change...
        let account = Account::unpack_unchecked(&account_info.try_borrow_data().unwrap()).unwrap();
        assert_eq!(account.amount, 1000);

        // owner transfer with delegate assigned checked
        let instruction = transfer_checked(
            &program_id,
            account_info.key,
            mint_info.key,
            account_info.key,
            owner_info.key,
            &[],
            1000,
            2,
        )
        .unwrap();
        assert_eq!(
            Ok(()),
            Processor::process(
                &instruction.program_id,
                &[
                    account_info.clone(),
                    mint_info.clone(),
                    account_info.clone(),
                    owner_info.clone(),
                ],
                &instruction.data,
            )
        );
        // no balance change...
        let account = Account::unpack_unchecked(&account_info.try_borrow_data().unwrap()).unwrap();
        assert_eq!(account.amount, 1000);
    }

    #[test]
    fn test_mintable_token_with_zero_supply() {
        let program_id = crate::id();
        let account_key = Pubkey::new_unique();
        let mut account_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let owner_key = Pubkey::new_unique();
        let mut owner_account = SolanaAccount::default();
        let mint_key = Pubkey::new_unique();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
        let mut rent_sysvar = rent_sysvar();

        // create mint-able token with zero supply
        let decimals = 2;
        do_process_instruction(
            initialize_mint(&program_id, &mint_key, &owner_key, None, decimals).unwrap(),
            vec![&mut mint_account, &mut rent_sysvar],
        )
        .unwrap();
        let mint = Mint::unpack_unchecked(&mint_account.data).unwrap();
        assert_eq!(
            mint,
            Mint {
                mint_authority: COption::Some(owner_key),
                supply: 0,
                decimals,
                is_initialized: true,
                freeze_authority: COption::None,
            }
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

        // mint to
        do_process_instruction(
            mint_to(&program_id, &mint_key, &account_key, &owner_key, &[], 42).unwrap(),
            vec![&mut mint_account, &mut account_account, &mut owner_account],
        )
        .unwrap();
        let _ = Mint::unpack(&mint_account.data).unwrap();
        let account = Account::unpack_unchecked(&account_account.data).unwrap();
        assert_eq!(account.amount, 42);

        // mint to 2, with incorrect decimals
        assert_eq!(
            Err(TokenError::MintDecimalsMismatch.into()),
            do_process_instruction(
                mint_to_checked(
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

        let _ = Mint::unpack(&mint_account.data).unwrap();
        let account = Account::unpack_unchecked(&account_account.data).unwrap();
        assert_eq!(account.amount, 42);

        // mint to 2
        do_process_instruction(
            mint_to_checked(
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
        let _ = Mint::unpack(&mint_account.data).unwrap();
        let account = Account::unpack_unchecked(&account_account.data).unwrap();
        assert_eq!(account.amount, 84);
    }

    #[test]
    fn test_approve_dups() {
        let program_id = crate::id();
        let account1_key = Pubkey::new_unique();
        let mut account1_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let account1_info: AccountInfo = (&account1_key, true, &mut account1_account).into();
        let account2_key = Pubkey::new_unique();
        let mut account2_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let account2_info: AccountInfo = (&account2_key, false, &mut account2_account).into();
        let account3_key = Pubkey::new_unique();
        let mut account3_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let account3_info: AccountInfo = (&account3_key, true, &mut account3_account).into();
        let multisig_key = Pubkey::new_unique();
        let mut multisig_account = SolanaAccount::new(
            multisig_minimum_balance(),
            Multisig::get_packed_len(),
            &program_id,
        );
        let multisig_info: AccountInfo = (&multisig_key, true, &mut multisig_account).into();
        let owner_key = Pubkey::new_unique();
        let mut owner_account = SolanaAccount::default();
        let owner_info: AccountInfo = (&owner_key, true, &mut owner_account).into();
        let mint_key = Pubkey::new_unique();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
        let mint_info: AccountInfo = (&mint_key, false, &mut mint_account).into();
        let rent_key = rent::id();
        let mut rent_sysvar = rent_sysvar();
        let rent_info: AccountInfo = (&rent_key, false, &mut rent_sysvar).into();

        // create mint
        do_process_instruction_dups(
            initialize_mint(&program_id, &mint_key, &owner_key, None, 2).unwrap(),
            vec![mint_info.clone(), rent_info.clone()],
        )
        .unwrap();

        // create account
        do_process_instruction_dups(
            initialize_account(&program_id, &account1_key, &mint_key, &account1_key).unwrap(),
            vec![
                account1_info.clone(),
                mint_info.clone(),
                account1_info.clone(),
                rent_info.clone(),
            ],
        )
        .unwrap();

        // create another account
        do_process_instruction_dups(
            initialize_account(&program_id, &account2_key, &mint_key, &owner_key).unwrap(),
            vec![
                account2_info.clone(),
                mint_info.clone(),
                owner_info.clone(),
                rent_info.clone(),
            ],
        )
        .unwrap();

        // mint to account
        do_process_instruction_dups(
            mint_to(&program_id, &mint_key, &account1_key, &owner_key, &[], 1000).unwrap(),
            vec![mint_info.clone(), account1_info.clone(), owner_info.clone()],
        )
        .unwrap();

        // source-owner approve
        do_process_instruction_dups(
            approve(
                &program_id,
                &account1_key,
                &account2_key,
                &account1_key,
                &[],
                500,
            )
            .unwrap(),
            vec![
                account1_info.clone(),
                account2_info.clone(),
                account1_info.clone(),
            ],
        )
        .unwrap();

        // source-owner approve_checked
        do_process_instruction_dups(
            approve_checked(
                &program_id,
                &account1_key,
                &mint_key,
                &account2_key,
                &account1_key,
                &[],
                500,
                2,
            )
            .unwrap(),
            vec![
                account1_info.clone(),
                mint_info.clone(),
                account2_info.clone(),
                account1_info.clone(),
            ],
        )
        .unwrap();

        // source-owner revoke
        do_process_instruction_dups(
            revoke(&program_id, &account1_key, &account1_key, &[]).unwrap(),
            vec![account1_info.clone(), account1_info.clone()],
        )
        .unwrap();

        // test source-multisig signer
        do_process_instruction_dups(
            initialize_multisig(&program_id, &multisig_key, &[&account3_key], 1).unwrap(),
            vec![
                multisig_info.clone(),
                rent_info.clone(),
                account3_info.clone(),
            ],
        )
        .unwrap();

        do_process_instruction_dups(
            initialize_account(&program_id, &account3_key, &mint_key, &multisig_key).unwrap(),
            vec![
                account3_info.clone(),
                mint_info.clone(),
                multisig_info.clone(),
                rent_info.clone(),
            ],
        )
        .unwrap();

        do_process_instruction_dups(
            mint_to(&program_id, &mint_key, &account3_key, &owner_key, &[], 1000).unwrap(),
            vec![mint_info.clone(), account3_info.clone(), owner_info.clone()],
        )
        .unwrap();

        // source-multisig-signer approve
        do_process_instruction_dups(
            approve(
                &program_id,
                &account3_key,
                &account2_key,
                &multisig_key,
                &[&account3_key],
                500,
            )
            .unwrap(),
            vec![
                account3_info.clone(),
                account2_info.clone(),
                multisig_info.clone(),
                account3_info.clone(),
            ],
        )
        .unwrap();

        // source-multisig-signer approve_checked
        do_process_instruction_dups(
            approve_checked(
                &program_id,
                &account3_key,
                &mint_key,
                &account2_key,
                &multisig_key,
                &[&account3_key],
                500,
                2,
            )
            .unwrap(),
            vec![
                account3_info.clone(),
                mint_info.clone(),
                account2_info.clone(),
                multisig_info.clone(),
                account3_info.clone(),
            ],
        )
        .unwrap();

        // source-owner multisig-signer
        do_process_instruction_dups(
            revoke(&program_id, &account3_key, &multisig_key, &[&account3_key]).unwrap(),
            vec![
                account3_info.clone(),
                multisig_info.clone(),
                account3_info.clone(),
            ],
        )
        .unwrap();
    }

    #[test]
    fn test_approve() {
        let program_id = crate::id();
        let account_key = Pubkey::new_unique();
        let mut account_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let account2_key = Pubkey::new_unique();
        let mut account2_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let delegate_key = Pubkey::new_unique();
        let mut delegate_account = SolanaAccount::default();
        let owner_key = Pubkey::new_unique();
        let mut owner_account = SolanaAccount::default();
        let owner2_key = Pubkey::new_unique();
        let mut owner2_account = SolanaAccount::default();
        let mint_key = Pubkey::new_unique();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
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
                approve_checked(
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
                approve_checked(
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
            approve_checked(
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
    fn test_set_authority_dups() {
        let program_id = crate::id();
        let account1_key = Pubkey::new_unique();
        let mut account1_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let account1_info: AccountInfo = (&account1_key, true, &mut account1_account).into();
        let owner_key = Pubkey::new_unique();
        let mint_key = Pubkey::new_unique();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
        let mint_info: AccountInfo = (&mint_key, true, &mut mint_account).into();
        let rent_key = rent::id();
        let mut rent_sysvar = rent_sysvar();
        let rent_info: AccountInfo = (&rent_key, false, &mut rent_sysvar).into();

        // create mint
        do_process_instruction_dups(
            initialize_mint(&program_id, &mint_key, &mint_key, Some(&mint_key), 2).unwrap(),
            vec![mint_info.clone(), rent_info.clone()],
        )
        .unwrap();

        // create account
        do_process_instruction_dups(
            initialize_account(&program_id, &account1_key, &mint_key, &account1_key).unwrap(),
            vec![
                account1_info.clone(),
                mint_info.clone(),
                account1_info.clone(),
                rent_info.clone(),
            ],
        )
        .unwrap();

        // set mint_authority when currently self
        do_process_instruction_dups(
            set_authority(
                &program_id,
                &mint_key,
                Some(&owner_key),
                AuthorityType::MintTokens,
                &mint_key,
                &[],
            )
            .unwrap(),
            vec![mint_info.clone(), mint_info.clone()],
        )
        .unwrap();

        // set freeze_authority when currently self
        do_process_instruction_dups(
            set_authority(
                &program_id,
                &mint_key,
                Some(&owner_key),
                AuthorityType::FreezeAccount,
                &mint_key,
                &[],
            )
            .unwrap(),
            vec![mint_info.clone(), mint_info.clone()],
        )
        .unwrap();

        // set account owner when currently self
        do_process_instruction_dups(
            set_authority(
                &program_id,
                &account1_key,
                Some(&owner_key),
                AuthorityType::AccountOwner,
                &account1_key,
                &[],
            )
            .unwrap(),
            vec![account1_info.clone(), account1_info.clone()],
        )
        .unwrap();

        // set close_authority when currently self
        let mut account = Account::unpack_unchecked(&account1_info.data.borrow()).unwrap();
        account.close_authority = COption::Some(account1_key);
        Account::pack(account, &mut account1_info.data.borrow_mut()).unwrap();

        do_process_instruction_dups(
            set_authority(
                &program_id,
                &account1_key,
                Some(&owner_key),
                AuthorityType::CloseAccount,
                &account1_key,
                &[],
            )
            .unwrap(),
            vec![account1_info.clone(), account1_info.clone()],
        )
        .unwrap();
    }

    #[test]
    fn test_set_authority() {
        let program_id = crate::id();
        let account_key = Pubkey::new_unique();
        let mut account_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let account2_key = Pubkey::new_unique();
        let mut account2_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let owner_key = Pubkey::new_unique();
        let mut owner_account = SolanaAccount::default();
        let owner2_key = Pubkey::new_unique();
        let mut owner2_account = SolanaAccount::default();
        let owner3_key = Pubkey::new_unique();
        let mut owner3_account = SolanaAccount::default();
        let mint_key = Pubkey::new_unique();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
        let mint2_key = Pubkey::new_unique();
        let mut mint2_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
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
            Err(ProgramError::UninitializedAccount),
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

        // set delegate
        do_process_instruction(
            approve(
                &program_id,
                &account_key,
                &owner2_key,
                &owner_key,
                &[],
                u64::MAX,
            )
            .unwrap(),
            vec![
                &mut account_account,
                &mut owner2_account,
                &mut owner_account,
            ],
        )
        .unwrap();
        let account = Account::unpack_unchecked(&account_account.data).unwrap();
        assert_eq!(account.delegate, COption::Some(owner2_key));
        assert_eq!(account.delegated_amount, u64::MAX);

        // set owner
        do_process_instruction(
            set_authority(
                &program_id,
                &account_key,
                Some(&owner3_key),
                AuthorityType::AccountOwner,
                &owner_key,
                &[],
            )
            .unwrap(),
            vec![&mut account_account, &mut owner_account],
        )
        .unwrap();

        // check delegate cleared
        let account = Account::unpack_unchecked(&account_account.data).unwrap();
        assert_eq!(account.delegate, COption::None);
        assert_eq!(account.delegated_amount, 0);

        // set owner without existing delegate
        do_process_instruction(
            set_authority(
                &program_id,
                &account_key,
                Some(&owner2_key),
                AuthorityType::AccountOwner,
                &owner3_key,
                &[],
            )
            .unwrap(),
            vec![&mut account_account, &mut owner3_account],
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
    fn test_mint_to_dups() {
        let program_id = crate::id();
        let account1_key = Pubkey::new_unique();
        let mut account1_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let account1_info: AccountInfo = (&account1_key, true, &mut account1_account).into();
        let owner_key = Pubkey::new_unique();
        let mut owner_account = SolanaAccount::default();
        let owner_info: AccountInfo = (&owner_key, true, &mut owner_account).into();
        let mint_key = Pubkey::new_unique();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
        let mint_info: AccountInfo = (&mint_key, true, &mut mint_account).into();
        let rent_key = rent::id();
        let mut rent_sysvar = rent_sysvar();
        let rent_info: AccountInfo = (&rent_key, false, &mut rent_sysvar).into();

        // create mint
        do_process_instruction_dups(
            initialize_mint(&program_id, &mint_key, &mint_key, None, 2).unwrap(),
            vec![mint_info.clone(), rent_info.clone()],
        )
        .unwrap();

        // create account
        do_process_instruction_dups(
            initialize_account(&program_id, &account1_key, &mint_key, &owner_key).unwrap(),
            vec![
                account1_info.clone(),
                mint_info.clone(),
                owner_info.clone(),
                rent_info.clone(),
            ],
        )
        .unwrap();

        // mint_to when mint_authority is self
        do_process_instruction_dups(
            mint_to(&program_id, &mint_key, &account1_key, &mint_key, &[], 42).unwrap(),
            vec![mint_info.clone(), account1_info.clone(), mint_info.clone()],
        )
        .unwrap();

        // mint_to_checked when mint_authority is self
        do_process_instruction_dups(
            mint_to_checked(&program_id, &mint_key, &account1_key, &mint_key, &[], 42, 2).unwrap(),
            vec![mint_info.clone(), account1_info.clone(), mint_info.clone()],
        )
        .unwrap();

        // mint_to when mint_authority is account owner
        let mut mint = Mint::unpack_unchecked(&mint_info.data.borrow()).unwrap();
        mint.mint_authority = COption::Some(account1_key);
        Mint::pack(mint, &mut mint_info.data.borrow_mut()).unwrap();
        do_process_instruction_dups(
            mint_to(
                &program_id,
                &mint_key,
                &account1_key,
                &account1_key,
                &[],
                42,
            )
            .unwrap(),
            vec![
                mint_info.clone(),
                account1_info.clone(),
                account1_info.clone(),
            ],
        )
        .unwrap();

        // mint_to_checked when mint_authority is account owner
        do_process_instruction_dups(
            mint_to(
                &program_id,
                &mint_key,
                &account1_key,
                &account1_key,
                &[],
                42,
            )
            .unwrap(),
            vec![
                mint_info.clone(),
                account1_info.clone(),
                account1_info.clone(),
            ],
        )
        .unwrap();
    }

    #[test]
    fn test_mint_to() {
        let program_id = crate::id();
        let account_key = Pubkey::new_unique();
        let mut account_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let account2_key = Pubkey::new_unique();
        let mut account2_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let account3_key = Pubkey::new_unique();
        let mut account3_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let mismatch_key = Pubkey::new_unique();
        let mut mismatch_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let owner_key = Pubkey::new_unique();
        let mut owner_account = SolanaAccount::default();
        let owner2_key = Pubkey::new_unique();
        let mut owner2_account = SolanaAccount::default();
        let mint_key = Pubkey::new_unique();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
        let mint2_key = Pubkey::new_unique();
        let uninitialized_key = Pubkey::new_unique();
        let mut uninitialized_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
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
        let mut account = Account::unpack_unchecked(&mismatch_account.data).unwrap();
        account.mint = mint2_key;
        Account::pack(account, &mut mismatch_account.data).unwrap();

        // mint to
        do_process_instruction(
            mint_to(&program_id, &mint_key, &account_key, &owner_key, &[], 42).unwrap(),
            vec![&mut mint_account, &mut account_account, &mut owner_account],
        )
        .unwrap();

        let mint = Mint::unpack_unchecked(&mint_account.data).unwrap();
        assert_eq!(mint.supply, 42);
        let account = Account::unpack_unchecked(&account_account.data).unwrap();
        assert_eq!(account.amount, 42);

        // mint to another account to test supply accumulation
        do_process_instruction(
            mint_to(&program_id, &mint_key, &account2_key, &owner_key, &[], 42).unwrap(),
            vec![&mut mint_account, &mut account2_account, &mut owner_account],
        )
        .unwrap();

        let mint = Mint::unpack_unchecked(&mint_account.data).unwrap();
        assert_eq!(mint.supply, 84);
        let account = Account::unpack_unchecked(&account2_account.data).unwrap();
        assert_eq!(account.amount, 42);

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

        // mint not owned by program
        let not_program_id = Pubkey::new_unique();
        mint_account.owner = not_program_id;
        assert_eq!(
            Err(ProgramError::IncorrectProgramId),
            do_process_instruction(
                mint_to(&program_id, &mint_key, &account_key, &owner_key, &[], 0).unwrap(),
                vec![&mut mint_account, &mut account_account, &mut owner_account],
            )
        );
        mint_account.owner = program_id;

        // account not owned by program
        let not_program_id = Pubkey::new_unique();
        account_account.owner = not_program_id;
        assert_eq!(
            Err(ProgramError::IncorrectProgramId),
            do_process_instruction(
                mint_to(&program_id, &mint_key, &account_key, &owner_key, &[], 0).unwrap(),
                vec![&mut mint_account, &mut account_account, &mut owner_account],
            )
        );
        account_account.owner = program_id;

        // uninitialized destination account
        assert_eq!(
            Err(ProgramError::UninitializedAccount),
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
    fn test_burn_dups() {
        let program_id = crate::id();
        let account1_key = Pubkey::new_unique();
        let mut account1_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let account1_info: AccountInfo = (&account1_key, true, &mut account1_account).into();
        let owner_key = Pubkey::new_unique();
        let mut owner_account = SolanaAccount::default();
        let owner_info: AccountInfo = (&owner_key, true, &mut owner_account).into();
        let mint_key = Pubkey::new_unique();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
        let mint_info: AccountInfo = (&mint_key, true, &mut mint_account).into();
        let rent_key = rent::id();
        let mut rent_sysvar = rent_sysvar();
        let rent_info: AccountInfo = (&rent_key, false, &mut rent_sysvar).into();

        // create mint
        do_process_instruction_dups(
            initialize_mint(&program_id, &mint_key, &owner_key, None, 2).unwrap(),
            vec![mint_info.clone(), rent_info.clone()],
        )
        .unwrap();

        // create account
        do_process_instruction_dups(
            initialize_account(&program_id, &account1_key, &mint_key, &account1_key).unwrap(),
            vec![
                account1_info.clone(),
                mint_info.clone(),
                account1_info.clone(),
                rent_info.clone(),
            ],
        )
        .unwrap();

        // mint to account
        do_process_instruction_dups(
            mint_to(&program_id, &mint_key, &account1_key, &owner_key, &[], 1000).unwrap(),
            vec![mint_info.clone(), account1_info.clone(), owner_info.clone()],
        )
        .unwrap();

        // source-owner burn
        do_process_instruction_dups(
            burn(
                &program_id,
                &mint_key,
                &account1_key,
                &account1_key,
                &[],
                500,
            )
            .unwrap(),
            vec![
                account1_info.clone(),
                mint_info.clone(),
                account1_info.clone(),
            ],
        )
        .unwrap();

        // source-owner burn_checked
        do_process_instruction_dups(
            burn_checked(
                &program_id,
                &account1_key,
                &mint_key,
                &account1_key,
                &[],
                500,
                2,
            )
            .unwrap(),
            vec![
                account1_info.clone(),
                mint_info.clone(),
                account1_info.clone(),
            ],
        )
        .unwrap();

        // mint-owner burn
        do_process_instruction_dups(
            mint_to(&program_id, &mint_key, &account1_key, &owner_key, &[], 1000).unwrap(),
            vec![mint_info.clone(), account1_info.clone(), owner_info.clone()],
        )
        .unwrap();
        let mut account = Account::unpack_unchecked(&account1_info.data.borrow()).unwrap();
        account.owner = mint_key;
        Account::pack(account, &mut account1_info.data.borrow_mut()).unwrap();
        do_process_instruction_dups(
            burn(&program_id, &account1_key, &mint_key, &mint_key, &[], 500).unwrap(),
            vec![account1_info.clone(), mint_info.clone(), mint_info.clone()],
        )
        .unwrap();

        // mint-owner burn_checked
        do_process_instruction_dups(
            burn_checked(
                &program_id,
                &account1_key,
                &mint_key,
                &mint_key,
                &[],
                500,
                2,
            )
            .unwrap(),
            vec![account1_info.clone(), mint_info.clone(), mint_info.clone()],
        )
        .unwrap();

        // source-delegate burn
        do_process_instruction_dups(
            mint_to(&program_id, &mint_key, &account1_key, &owner_key, &[], 1000).unwrap(),
            vec![mint_info.clone(), account1_info.clone(), owner_info.clone()],
        )
        .unwrap();
        let mut account = Account::unpack_unchecked(&account1_info.data.borrow()).unwrap();
        account.delegated_amount = 1000;
        account.delegate = COption::Some(account1_key);
        account.owner = owner_key;
        Account::pack(account, &mut account1_info.data.borrow_mut()).unwrap();
        do_process_instruction_dups(
            burn(
                &program_id,
                &account1_key,
                &mint_key,
                &account1_key,
                &[],
                500,
            )
            .unwrap(),
            vec![
                account1_info.clone(),
                mint_info.clone(),
                account1_info.clone(),
            ],
        )
        .unwrap();

        // source-delegate burn_checked
        do_process_instruction_dups(
            burn_checked(
                &program_id,
                &account1_key,
                &mint_key,
                &account1_key,
                &[],
                500,
                2,
            )
            .unwrap(),
            vec![
                account1_info.clone(),
                mint_info.clone(),
                account1_info.clone(),
            ],
        )
        .unwrap();

        // mint-delegate burn
        do_process_instruction_dups(
            mint_to(&program_id, &mint_key, &account1_key, &owner_key, &[], 1000).unwrap(),
            vec![mint_info.clone(), account1_info.clone(), owner_info.clone()],
        )
        .unwrap();
        let mut account = Account::unpack_unchecked(&account1_info.data.borrow()).unwrap();
        account.delegated_amount = 1000;
        account.delegate = COption::Some(mint_key);
        account.owner = owner_key;
        Account::pack(account, &mut account1_info.data.borrow_mut()).unwrap();
        do_process_instruction_dups(
            burn(&program_id, &account1_key, &mint_key, &mint_key, &[], 500).unwrap(),
            vec![account1_info.clone(), mint_info.clone(), mint_info.clone()],
        )
        .unwrap();

        // mint-delegate burn_checked
        do_process_instruction_dups(
            burn_checked(
                &program_id,
                &account1_key,
                &mint_key,
                &mint_key,
                &[],
                500,
                2,
            )
            .unwrap(),
            vec![account1_info.clone(), mint_info.clone(), mint_info.clone()],
        )
        .unwrap();
    }

    #[test]
    fn test_burn() {
        let program_id = crate::id();
        let account_key = Pubkey::new_unique();
        let mut account_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let account2_key = Pubkey::new_unique();
        let mut account2_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let account3_key = Pubkey::new_unique();
        let mut account3_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let delegate_key = Pubkey::new_unique();
        let mut delegate_account = SolanaAccount::default();
        let mismatch_key = Pubkey::new_unique();
        let mut mismatch_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let owner_key = Pubkey::new_unique();
        let mut owner_account = SolanaAccount::default();
        let owner2_key = Pubkey::new_unique();
        let mut owner2_account = SolanaAccount::default();
        let mint_key = Pubkey::new_unique();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
        let mint2_key = Pubkey::new_unique();
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

        // mint to account
        do_process_instruction(
            mint_to(&program_id, &mint_key, &account_key, &owner_key, &[], 1000).unwrap(),
            vec![&mut mint_account, &mut account_account, &mut owner_account],
        )
        .unwrap();

        // mint to mismatch account and change mint key
        do_process_instruction(
            mint_to(&program_id, &mint_key, &mismatch_key, &owner_key, &[], 1000).unwrap(),
            vec![&mut mint_account, &mut mismatch_account, &mut owner_account],
        )
        .unwrap();
        let mut account = Account::unpack_unchecked(&mismatch_account.data).unwrap();
        account.mint = mint2_key;
        Account::pack(account, &mut mismatch_account.data).unwrap();

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

        // account not owned by program
        let not_program_id = Pubkey::new_unique();
        account_account.owner = not_program_id;
        assert_eq!(
            Err(ProgramError::IncorrectProgramId),
            do_process_instruction(
                burn(&program_id, &account_key, &mint_key, &owner_key, &[], 0).unwrap(),
                vec![&mut account_account, &mut mint_account, &mut owner_account],
            )
        );
        account_account.owner = program_id;

        // mint not owned by program
        let not_program_id = Pubkey::new_unique();
        mint_account.owner = not_program_id;
        assert_eq!(
            Err(ProgramError::IncorrectProgramId),
            do_process_instruction(
                burn(&program_id, &account_key, &mint_key, &owner_key, &[], 0).unwrap(),
                vec![&mut account_account, &mut mint_account, &mut owner_account],
            )
        );
        mint_account.owner = program_id;

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

        // burn_checked, with incorrect decimals
        assert_eq!(
            Err(TokenError::MintDecimalsMismatch.into()),
            do_process_instruction(
                burn_checked(&program_id, &account_key, &mint_key, &owner_key, &[], 21, 3).unwrap(),
                vec![&mut account_account, &mut mint_account, &mut owner_account],
            )
        );

        // burn_checked
        do_process_instruction(
            burn_checked(&program_id, &account_key, &mint_key, &owner_key, &[], 21, 2).unwrap(),
            vec![&mut account_account, &mut mint_account, &mut owner_account],
        )
        .unwrap();

        let mint = Mint::unpack_unchecked(&mint_account.data).unwrap();
        assert_eq!(mint.supply, 2000 - 42);
        let account = Account::unpack_unchecked(&account_account.data).unwrap();
        assert_eq!(account.amount, 1000 - 42);

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
        let mint = Mint::unpack_unchecked(&mint_account.data).unwrap();
        assert_eq!(mint.supply, 2000 - 42 - 84);
        let account = Account::unpack_unchecked(&account_account.data).unwrap();
        assert_eq!(account.amount, 1000 - 42 - 84);

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
        let program_id = crate::id();
        let mint_key = Pubkey::new_unique();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
        let account_key = Pubkey::new_unique();
        let mut account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let account2_key = Pubkey::new_unique();
        let mut account2_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let owner_key = Pubkey::new_unique();
        let mut owner_account = SolanaAccount::default();
        let multisig_key = Pubkey::new_unique();
        let mut multisig_account = SolanaAccount::new(42, Multisig::get_packed_len(), &program_id);
        let multisig_delegate_key = Pubkey::new_unique();
        let mut multisig_delegate_account = SolanaAccount::new(
            multisig_minimum_balance(),
            Multisig::get_packed_len(),
            &program_id,
        );
        let signer_keys = vec![Pubkey::new_unique(); MAX_SIGNERS];
        let signer_key_refs: Vec<&Pubkey> = signer_keys.iter().collect();
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
                    account_info_iter.next().unwrap(),
                ],
            )
        );

        multisig_account.lamports = multisig_minimum_balance();
        let mut multisig_account2 = multisig_account.clone();

        // single signer
        let account_info_iter = &mut signer_accounts.iter_mut();
        do_process_instruction(
            initialize_multisig(&program_id, &multisig_key, &[&signer_keys[0]], 1).unwrap(),
            vec![
                &mut multisig_account,
                &mut rent_sysvar,
                account_info_iter.next().unwrap(),
            ],
        )
        .unwrap();

        // single signer using `initialize_multisig2`
        let account_info_iter = &mut signer_accounts.iter_mut();
        do_process_instruction(
            initialize_multisig2(&program_id, &multisig_key, &[&signer_keys[0]], 1).unwrap(),
            vec![&mut multisig_account2, account_info_iter.next().unwrap()],
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
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
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
                account_info_iter.next().unwrap(),
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
                account_info_iter.next().unwrap(),
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
                account_info_iter.next().unwrap(),
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
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
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
                account_info_iter.next().unwrap(),
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
                account_info_iter.next().unwrap(),
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
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
                account_info_iter.next().unwrap(),
            ],
        )
        .unwrap();

        // freeze account
        let account3_key = Pubkey::new_unique();
        let mut account3_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let mint2_key = Pubkey::new_unique();
        let mut mint2_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
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
                account_info_iter.next().unwrap(),
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
                account_info_iter.next().unwrap(),
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
                account_info_iter.next().unwrap(),
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
                account_info_iter.next().unwrap(),
            ],
        )
        .unwrap();
    }

    #[test]
    fn test_validate_owner() {
        let program_id = crate::id();
        let owner_key = Pubkey::new_unique();
        let mut signer_keys = [Pubkey::default(); MAX_SIGNERS];
        for signer_key in signer_keys.iter_mut().take(MAX_SIGNERS) {
            *signer_key = Pubkey::new_unique();
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
        let mut data = vec![0; Multisig::get_packed_len()];
        let mut multisig = Multisig::unpack_unchecked(&data).unwrap();
        multisig.m = MAX_SIGNERS as u8;
        multisig.n = MAX_SIGNERS as u8;
        multisig.signers = signer_keys;
        multisig.is_initialized = true;
        Multisig::pack(multisig, &mut data).unwrap();
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
            let mut multisig =
                Multisig::unpack_unchecked(&owner_account_info.data.borrow()).unwrap();
            multisig.m = 1;
            Multisig::pack(multisig, &mut owner_account_info.data.borrow_mut()).unwrap();
        }
        Processor::validate_owner(&program_id, &owner_key, &owner_account_info, &signers).unwrap();

        // 2:1
        {
            let mut multisig =
                Multisig::unpack_unchecked(&owner_account_info.data.borrow()).unwrap();
            multisig.m = 2;
            multisig.n = 1;
            Multisig::pack(multisig, &mut owner_account_info.data.borrow_mut()).unwrap();
        }
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            Processor::validate_owner(&program_id, &owner_key, &owner_account_info, &signers)
        );

        // 0:11
        {
            let mut multisig =
                Multisig::unpack_unchecked(&owner_account_info.data.borrow()).unwrap();
            multisig.m = 0;
            multisig.n = 11;
            Multisig::pack(multisig, &mut owner_account_info.data.borrow_mut()).unwrap();
        }
        Processor::validate_owner(&program_id, &owner_key, &owner_account_info, &signers).unwrap();

        // 2:11 but 0 provided
        {
            let mut multisig =
                Multisig::unpack_unchecked(&owner_account_info.data.borrow()).unwrap();
            multisig.m = 2;
            multisig.n = 11;
            Multisig::pack(multisig, &mut owner_account_info.data.borrow_mut()).unwrap();
        }
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            Processor::validate_owner(&program_id, &owner_key, &owner_account_info, &[])
        );
        // 2:11 but 1 provided
        {
            let mut multisig =
                Multisig::unpack_unchecked(&owner_account_info.data.borrow()).unwrap();
            multisig.m = 2;
            multisig.n = 11;
            Multisig::pack(multisig, &mut owner_account_info.data.borrow_mut()).unwrap();
        }
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            Processor::validate_owner(&program_id, &owner_key, &owner_account_info, &signers[0..1])
        );

        // 2:11, 2 from middle provided
        {
            let mut multisig =
                Multisig::unpack_unchecked(&owner_account_info.data.borrow()).unwrap();
            multisig.m = 2;
            multisig.n = 11;
            Multisig::pack(multisig, &mut owner_account_info.data.borrow_mut()).unwrap();
        }
        Processor::validate_owner(&program_id, &owner_key, &owner_account_info, &signers[5..7])
            .unwrap();

        // 11:11, one is not a signer
        {
            let mut multisig =
                Multisig::unpack_unchecked(&owner_account_info.data.borrow()).unwrap();
            multisig.m = 11;
            multisig.n = 11;
            Multisig::pack(multisig, &mut owner_account_info.data.borrow_mut()).unwrap();
        }
        signers[5].is_signer = false;
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            Processor::validate_owner(&program_id, &owner_key, &owner_account_info, &signers)
        );
        signers[5].is_signer = true;

        // 11:11, single signer signs multiple times
        {
            let mut signer_lamports = 0;
            let mut signer_data = vec![];
            let signers = vec![
                AccountInfo::new(
                    &signer_keys[5],
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
            let mut multisig =
                Multisig::unpack_unchecked(&owner_account_info.data.borrow()).unwrap();
            multisig.m = 11;
            multisig.n = 11;
            Multisig::pack(multisig, &mut owner_account_info.data.borrow_mut()).unwrap();
            assert_eq!(
                Err(ProgramError::MissingRequiredSignature),
                Processor::validate_owner(&program_id, &owner_key, &owner_account_info, &signers)
            );
        }
    }

    #[test]
    fn test_owner_close_account_dups() {
        let program_id = crate::id();
        let owner_key = Pubkey::new_unique();
        let mint_key = Pubkey::new_unique();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
        let mint_info: AccountInfo = (&mint_key, false, &mut mint_account).into();
        let rent_key = rent::id();
        let mut rent_sysvar = rent_sysvar();
        let rent_info: AccountInfo = (&rent_key, false, &mut rent_sysvar).into();

        // create mint
        do_process_instruction_dups(
            initialize_mint(&program_id, &mint_key, &owner_key, None, 2).unwrap(),
            vec![mint_info.clone(), rent_info.clone()],
        )
        .unwrap();

        let to_close_key = Pubkey::new_unique();
        let mut to_close_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let to_close_account_info: AccountInfo =
            (&to_close_key, true, &mut to_close_account).into();
        let destination_account_key = Pubkey::new_unique();
        let mut destination_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let destination_account_info: AccountInfo =
            (&destination_account_key, true, &mut destination_account).into();
        // create account
        do_process_instruction_dups(
            initialize_account(&program_id, &to_close_key, &mint_key, &to_close_key).unwrap(),
            vec![
                to_close_account_info.clone(),
                mint_info.clone(),
                to_close_account_info.clone(),
                rent_info.clone(),
            ],
        )
        .unwrap();

        // source-owner close
        do_process_instruction_dups(
            close_account(
                &program_id,
                &to_close_key,
                &destination_account_key,
                &to_close_key,
                &[],
            )
            .unwrap(),
            vec![
                to_close_account_info.clone(),
                destination_account_info.clone(),
                to_close_account_info.clone(),
            ],
        )
        .unwrap();
        assert_eq!(*to_close_account_info.data.borrow(), &[0u8; Account::LEN]);
    }

    #[test]
    fn test_close_authority_close_account_dups() {
        let program_id = crate::id();
        let owner_key = Pubkey::new_unique();
        let mint_key = Pubkey::new_unique();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
        let mint_info: AccountInfo = (&mint_key, false, &mut mint_account).into();
        let rent_key = rent::id();
        let mut rent_sysvar = rent_sysvar();
        let rent_info: AccountInfo = (&rent_key, false, &mut rent_sysvar).into();

        // create mint
        do_process_instruction_dups(
            initialize_mint(&program_id, &mint_key, &owner_key, None, 2).unwrap(),
            vec![mint_info.clone(), rent_info.clone()],
        )
        .unwrap();

        let to_close_key = Pubkey::new_unique();
        let mut to_close_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let to_close_account_info: AccountInfo =
            (&to_close_key, true, &mut to_close_account).into();
        let destination_account_key = Pubkey::new_unique();
        let mut destination_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let destination_account_info: AccountInfo =
            (&destination_account_key, true, &mut destination_account).into();
        // create account
        do_process_instruction_dups(
            initialize_account(&program_id, &to_close_key, &mint_key, &to_close_key).unwrap(),
            vec![
                to_close_account_info.clone(),
                mint_info.clone(),
                to_close_account_info.clone(),
                rent_info.clone(),
            ],
        )
        .unwrap();
        let mut account = Account::unpack_unchecked(&to_close_account_info.data.borrow()).unwrap();
        account.close_authority = COption::Some(to_close_key);
        account.owner = owner_key;
        Account::pack(account, &mut to_close_account_info.data.borrow_mut()).unwrap();
        do_process_instruction_dups(
            close_account(
                &program_id,
                &to_close_key,
                &destination_account_key,
                &to_close_key,
                &[],
            )
            .unwrap(),
            vec![
                to_close_account_info.clone(),
                destination_account_info.clone(),
                to_close_account_info.clone(),
            ],
        )
        .unwrap();
        assert_eq!(*to_close_account_info.data.borrow(), &[0u8; Account::LEN]);
    }

    #[test]
    fn test_close_account() {
        let program_id = crate::id();
        let mint_key = Pubkey::new_unique();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
        let account_key = Pubkey::new_unique();
        let mut account_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let account2_key = Pubkey::new_unique();
        let mut account2_account = SolanaAccount::new(
            account_minimum_balance() + 42,
            Account::get_packed_len(),
            &program_id,
        );
        let account3_key = Pubkey::new_unique();
        let mut account3_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let owner_key = Pubkey::new_unique();
        let mut owner_account = SolanaAccount::default();
        let owner2_key = Pubkey::new_unique();
        let mut owner2_account = SolanaAccount::default();
        let mut rent_sysvar = rent_sysvar();

        // uninitialized
        assert_eq!(
            Err(ProgramError::UninitializedAccount),
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
        let account = Account::unpack_unchecked(&account_account.data).unwrap();
        assert_eq!(account.amount, 42);

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
        let account = Account::unpack_unchecked(&account2_account.data).unwrap();
        assert!(account.is_native());
        assert_eq!(account.amount, 42);

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
        let account = Account::unpack_unchecked(&account_account.data).unwrap();
        assert_eq!(account.amount, 0);

        // fund and initialize new non-native account to test close authority
        let account_key = Pubkey::new_unique();
        let mut account_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let owner2_key = Pubkey::new_unique();
        let mut owner2_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
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
        let account = Account::unpack_unchecked(&account_account.data).unwrap();
        assert_eq!(account.amount, 0);

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
        assert_eq!(account2_account.data, [0u8; Account::LEN]);
        assert_eq!(
            account3_account.lamports,
            3 * account_minimum_balance() + 2 + 42
        );
    }

    #[test]
    fn test_native_token() {
        let program_id = crate::id();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
        let account_key = Pubkey::new_unique();
        let mut account_account = SolanaAccount::new(
            account_minimum_balance() + 40,
            Account::get_packed_len(),
            &program_id,
        );
        let account2_key = Pubkey::new_unique();
        let mut account2_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let account3_key = Pubkey::new_unique();
        let mut account3_account = SolanaAccount::new(account_minimum_balance(), 0, &program_id);
        let owner_key = Pubkey::new_unique();
        let mut owner_account = SolanaAccount::default();
        let owner2_key = Pubkey::new_unique();
        let mut owner2_account = SolanaAccount::default();
        let owner3_key = Pubkey::new_unique();
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
        let account = Account::unpack_unchecked(&account_account.data).unwrap();
        assert!(account.is_native());
        assert_eq!(account.amount, 40);

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
        let account = Account::unpack_unchecked(&account2_account.data).unwrap();
        assert!(account.is_native());
        assert_eq!(account.amount, 0);

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
        let bogus_mint_key = Pubkey::new_unique();
        let mut bogus_mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
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
        let account = Account::unpack_unchecked(&account_account.data).unwrap();
        assert!(account.is_native());
        assert_eq!(account.amount, 0);
        assert_eq!(account2_account.lamports, account_minimum_balance() + 40);
        let account = Account::unpack_unchecked(&account2_account.data).unwrap();
        assert!(account.is_native());
        assert_eq!(account.amount, 40);

        // set close authority
        do_process_instruction(
            set_authority(
                &program_id,
                &account_key,
                Some(&owner3_key),
                AuthorityType::CloseAccount,
                &owner_key,
                &[],
            )
            .unwrap(),
            vec![&mut account_account, &mut owner_account],
        )
        .unwrap();
        let account = Account::unpack_unchecked(&account_account.data).unwrap();
        assert_eq!(account.close_authority, COption::Some(owner3_key));

        // set new account owner
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

        // close authority cleared
        let account = Account::unpack_unchecked(&account_account.data).unwrap();
        assert_eq!(account.close_authority, COption::None);

        // close native account
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
        assert_eq!(account3_account.lamports, 2 * account_minimum_balance());
        assert_eq!(account_account.data, [0u8; Account::LEN]);
    }

    #[test]
    fn test_overflow() {
        let program_id = crate::id();
        let account_key = Pubkey::new_unique();
        let mut account_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let account2_key = Pubkey::new_unique();
        let mut account2_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let owner_key = Pubkey::new_unique();
        let mut owner_account = SolanaAccount::default();
        let owner2_key = Pubkey::new_unique();
        let mut owner2_account = SolanaAccount::default();
        let mint_owner_key = Pubkey::new_unique();
        let mut mint_owner_account = SolanaAccount::default();
        let mint_key = Pubkey::new_unique();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
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
        let account = Account::unpack_unchecked(&account_account.data).unwrap();
        assert_eq!(account.amount, u64::MAX);

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
        let account = Account::unpack_unchecked(&account_account.data).unwrap();
        assert_eq!(account.amount, u64::MAX);

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
        let account = Account::unpack_unchecked(&account_account.data).unwrap();
        assert_eq!(account.amount, u64::MAX - 100);

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
        let account = Account::unpack_unchecked(&account_account.data).unwrap();
        assert_eq!(account.amount, u64::MAX);

        // manipulate account balance to attempt overflow transfer
        let mut account = Account::unpack_unchecked(&account2_account.data).unwrap();
        account.amount = 1;
        Account::pack(account, &mut account2_account.data).unwrap();

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
        let program_id = crate::id();
        let account_key = Pubkey::new_unique();
        let mut account_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let account2_key = Pubkey::new_unique();
        let mut account2_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let owner_key = Pubkey::new_unique();
        let mut owner_account = SolanaAccount::default();
        let mint_key = Pubkey::new_unique();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
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
        let mut account = Account::unpack_unchecked(&account2_account.data).unwrap();
        account.state = AccountState::Frozen;
        Account::pack(account, &mut account2_account.data).unwrap();
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

        let mut account = Account::unpack_unchecked(&account_account.data).unwrap();
        account.state = AccountState::Initialized;
        Account::pack(account, &mut account_account.data).unwrap();
        let mut account = Account::unpack_unchecked(&account2_account.data).unwrap();
        account.state = AccountState::Frozen;
        Account::pack(account, &mut account2_account.data).unwrap();
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
        let mut account = Account::unpack_unchecked(&account_account.data).unwrap();
        account.state = AccountState::Frozen;
        Account::pack(account, &mut account_account.data).unwrap();
        let delegate_key = Pubkey::new_unique();
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
        let mut account = Account::unpack_unchecked(&account_account.data).unwrap();
        account.delegate = COption::Some(delegate_key);
        account.delegated_amount = 100;
        Account::pack(account, &mut account_account.data).unwrap();
        assert_eq!(
            Err(TokenError::AccountFrozen.into()),
            do_process_instruction(
                revoke(&program_id, &account_key, &owner_key, &[]).unwrap(),
                vec![&mut account_account, &mut owner_account],
            )
        );

        // no set authority if account is frozen
        let new_owner_key = Pubkey::new_unique();
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
    fn test_freeze_thaw_dups() {
        let program_id = crate::id();
        let account1_key = Pubkey::new_unique();
        let mut account1_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let account1_info: AccountInfo = (&account1_key, true, &mut account1_account).into();
        let owner_key = Pubkey::new_unique();
        let mint_key = Pubkey::new_unique();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
        let mint_info: AccountInfo = (&mint_key, true, &mut mint_account).into();
        let rent_key = rent::id();
        let mut rent_sysvar = rent_sysvar();
        let rent_info: AccountInfo = (&rent_key, false, &mut rent_sysvar).into();

        // create mint
        do_process_instruction_dups(
            initialize_mint(&program_id, &mint_key, &owner_key, Some(&account1_key), 2).unwrap(),
            vec![mint_info.clone(), rent_info.clone()],
        )
        .unwrap();

        // create account
        do_process_instruction_dups(
            initialize_account(&program_id, &account1_key, &mint_key, &account1_key).unwrap(),
            vec![
                account1_info.clone(),
                mint_info.clone(),
                account1_info.clone(),
                rent_info.clone(),
            ],
        )
        .unwrap();

        // freeze where mint freeze_authority is account
        do_process_instruction_dups(
            freeze_account(&program_id, &account1_key, &mint_key, &account1_key, &[]).unwrap(),
            vec![
                account1_info.clone(),
                mint_info.clone(),
                account1_info.clone(),
            ],
        )
        .unwrap();

        // thaw where mint freeze_authority is account
        let mut account = Account::unpack_unchecked(&account1_info.data.borrow()).unwrap();
        account.state = AccountState::Frozen;
        Account::pack(account, &mut account1_info.data.borrow_mut()).unwrap();
        do_process_instruction_dups(
            thaw_account(&program_id, &account1_key, &mint_key, &account1_key, &[]).unwrap(),
            vec![
                account1_info.clone(),
                mint_info.clone(),
                account1_info.clone(),
            ],
        )
        .unwrap();
    }

    #[test]
    fn test_freeze_account() {
        let program_id = crate::id();
        let account_key = Pubkey::new_unique();
        let mut account_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let account_owner_key = Pubkey::new_unique();
        let mut account_owner_account = SolanaAccount::default();
        let owner_key = Pubkey::new_unique();
        let mut owner_account = SolanaAccount::default();
        let owner2_key = Pubkey::new_unique();
        let mut owner2_account = SolanaAccount::default();
        let mint_key = Pubkey::new_unique();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
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
        let mut mint = Mint::unpack_unchecked(&mint_account.data).unwrap();
        mint.freeze_authority = COption::Some(owner_key);
        Mint::pack(mint, &mut mint_account.data).unwrap();
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
        let account = Account::unpack_unchecked(&account_account.data).unwrap();
        assert_eq!(account.state, AccountState::Frozen);

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
        let account = Account::unpack_unchecked(&account_account.data).unwrap();
        assert_eq!(account.state, AccountState::Initialized);
    }

    #[test]
    fn test_initialize_account2_and_3() {
        let program_id = crate::id();
        let account_key = Pubkey::new_unique();
        let mut account_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let mut account2_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let mut account3_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let owner_key = Pubkey::new_unique();
        let mut owner_account = SolanaAccount::default();
        let mint_key = Pubkey::new_unique();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
        let mut rent_sysvar = rent_sysvar();

        // create mint
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
            initialize_account2(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
            vec![&mut account2_account, &mut mint_account, &mut rent_sysvar],
        )
        .unwrap();

        assert_eq!(account_account, account2_account);

        do_process_instruction(
            initialize_account3(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
            vec![&mut account3_account, &mut mint_account],
        )
        .unwrap();

        assert_eq!(account_account, account3_account);
    }

    #[test]
    fn test_sync_native() {
        let program_id = crate::id();
        let mint_key = Pubkey::new_unique();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
        let native_account_key = Pubkey::new_unique();
        let lamports = 40;
        let mut native_account = SolanaAccount::new(
            account_minimum_balance() + lamports,
            Account::get_packed_len(),
            &program_id,
        );
        let non_native_account_key = Pubkey::new_unique();
        let mut non_native_account = SolanaAccount::new(
            account_minimum_balance() + 50,
            Account::get_packed_len(),
            &program_id,
        );

        let owner_key = Pubkey::new_unique();
        let mut owner_account = SolanaAccount::default();
        let mut rent_sysvar = rent_sysvar();

        // initialize non-native mint
        do_process_instruction(
            initialize_mint(&program_id, &mint_key, &owner_key, None, 2).unwrap(),
            vec![&mut mint_account, &mut rent_sysvar],
        )
        .unwrap();

        // initialize non-native account
        do_process_instruction(
            initialize_account(&program_id, &non_native_account_key, &mint_key, &owner_key)
                .unwrap(),
            vec![
                &mut non_native_account,
                &mut mint_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();

        let account = Account::unpack_unchecked(&non_native_account.data).unwrap();
        assert!(!account.is_native());
        assert_eq!(account.amount, 0);

        // fail sync non-native
        assert_eq!(
            Err(TokenError::NonNativeNotSupported.into()),
            do_process_instruction(
                sync_native(&program_id, &non_native_account_key,).unwrap(),
                vec![&mut non_native_account],
            )
        );

        // fail sync uninitialized
        assert_eq!(
            Err(ProgramError::UninitializedAccount),
            do_process_instruction(
                sync_native(&program_id, &native_account_key,).unwrap(),
                vec![&mut native_account],
            )
        );

        // wrap native account
        do_process_instruction(
            initialize_account(
                &program_id,
                &native_account_key,
                &crate::native_mint::id(),
                &owner_key,
            )
            .unwrap(),
            vec![
                &mut native_account,
                &mut mint_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();

        // fail sync, not owned by program
        let not_program_id = Pubkey::new_unique();
        native_account.owner = not_program_id;
        assert_eq!(
            Err(ProgramError::IncorrectProgramId),
            do_process_instruction(
                sync_native(&program_id, &native_account_key,).unwrap(),
                vec![&mut native_account],
            )
        );
        native_account.owner = program_id;

        let account = Account::unpack_unchecked(&native_account.data).unwrap();
        assert!(account.is_native());
        assert_eq!(account.amount, lamports);

        // sync, no change
        do_process_instruction(
            sync_native(&program_id, &native_account_key).unwrap(),
            vec![&mut native_account],
        )
        .unwrap();
        let account = Account::unpack_unchecked(&native_account.data).unwrap();
        assert_eq!(account.amount, lamports);

        // transfer sol
        let new_lamports = lamports + 50;
        native_account.lamports = account_minimum_balance() + new_lamports;

        // success sync
        do_process_instruction(
            sync_native(&program_id, &native_account_key).unwrap(),
            vec![&mut native_account],
        )
        .unwrap();
        let account = Account::unpack_unchecked(&native_account.data).unwrap();
        assert_eq!(account.amount, new_lamports);

        // reduce sol
        native_account.lamports -= 1;

        // fail sync
        assert_eq!(
            Err(TokenError::InvalidState.into()),
            do_process_instruction(
                sync_native(&program_id, &native_account_key,).unwrap(),
                vec![&mut native_account],
            )
        );
    }
}
