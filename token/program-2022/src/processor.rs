//! Program state processor

use {
    crate::{
        check_program_account, cmp_pubkeys,
        error::TokenError,
        extension::{
            confidential_transfer::{self, ConfidentialTransferAccount, ConfidentialTransferMint},
            confidential_transfer_fee::{
                self, ConfidentialTransferFeeAmount, ConfidentialTransferFeeConfig,
            },
            cpi_guard::{self, in_cpi, CpiGuard},
            default_account_state::{self, DefaultAccountState},
            group_member_pointer::{self, GroupMemberPointer},
            group_pointer::{self, GroupPointer},
            immutable_owner::ImmutableOwner,
            interest_bearing_mint::{self, InterestBearingConfig},
            memo_transfer::{self, check_previous_sibling_instruction_is_memo, memo_required},
            metadata_pointer::{self, MetadataPointer},
            mint_close_authority::MintCloseAuthority,
            non_transferable::{NonTransferable, NonTransferableAccount},
            permanent_delegate::{get_permanent_delegate, PermanentDelegate},
            reallocate, token_group, token_metadata,
            transfer_fee::{self, TransferFeeAmount, TransferFeeConfig},
            transfer_hook::{self, TransferHook, TransferHookAccount},
            AccountType, BaseStateWithExtensions, ExtensionType, StateWithExtensions,
            StateWithExtensionsMut,
        },
        instruction::{is_valid_signer_index, AuthorityType, TokenInstruction, MAX_SIGNERS},
        native_mint,
        state::{Account, AccountState, Mint, Multisig},
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        clock::Clock,
        entrypoint::ProgramResult,
        msg,
        program::{invoke, invoke_signed, set_return_data},
        program_error::ProgramError,
        program_option::COption,
        program_pack::Pack,
        pubkey::Pubkey,
        system_instruction, system_program,
        sysvar::{rent::Rent, Sysvar},
    },
    spl_token_group_interface::instruction::TokenGroupInstruction,
    spl_token_metadata_interface::instruction::TokenMetadataInstruction,
    std::convert::{TryFrom, TryInto},
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
        let mut mint_data = mint_info.data.borrow_mut();
        let rent = if rent_sysvar_account {
            Rent::from_account_info(next_account_info(account_info_iter)?)?
        } else {
            Rent::get()?
        };

        if !rent.is_exempt(mint_info.lamports(), mint_data_len) {
            return Err(TokenError::NotRentExempt.into());
        }

        let mut mint = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut mint_data)?;
        let extension_types = mint.get_extension_types()?;
        if ExtensionType::try_calculate_account_len::<Mint>(&extension_types)? != mint_data_len {
            return Err(ProgramError::InvalidAccountData);
        }
        ExtensionType::check_for_invalid_mint_extension_combinations(&extension_types)?;

        if let Ok(default_account_state) = mint.get_extension_mut::<DefaultAccountState>() {
            let default_account_state = AccountState::try_from(default_account_state.state)
                .or(Err(ProgramError::InvalidAccountData))?;
            if default_account_state == AccountState::Frozen && freeze_authority.is_none() {
                return Err(TokenError::MintCannotFreeze.into());
            }
        }

        mint.base.mint_authority = COption::Some(mint_authority);
        mint.base.decimals = decimals;
        mint.base.is_initialized = true;
        mint.base.freeze_authority = freeze_authority;
        mint.pack_base();
        mint.init_account_type()?;

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

        let mut account_data = new_account_info.data.borrow_mut();
        // unpack_uninitialized checks account.base.is_initialized() under the hood
        let mut account =
            StateWithExtensionsMut::<Account>::unpack_uninitialized(&mut account_data)?;

        if !rent.is_exempt(new_account_info.lamports(), new_account_info_data_len) {
            return Err(TokenError::NotRentExempt.into());
        }

        // get_required_account_extensions checks mint validity
        let mint_data = mint_info.data.borrow();
        let mint = StateWithExtensions::<Mint>::unpack(&mint_data)
            .map_err(|_| Into::<ProgramError>::into(TokenError::InvalidMint))?;
        if mint
            .get_extension::<PermanentDelegate>()
            .map(|e| Option::<Pubkey>::from(e.delegate).is_some())
            .unwrap_or(false)
        {
            msg!("Warning: Mint has a permanent delegate, so tokens in this account may be seized at any time");
        }
        let required_extensions =
            Self::get_required_account_extensions_from_unpacked_mint(mint_info.owner, &mint)?;
        if ExtensionType::try_calculate_account_len::<Account>(&required_extensions)?
            > new_account_info_data_len
        {
            return Err(ProgramError::InvalidAccountData);
        }
        for extension in required_extensions {
            account.init_account_extension_from_type(extension)?;
        }

        let starting_state =
            if let Ok(default_account_state) = mint.get_extension::<DefaultAccountState>() {
                AccountState::try_from(default_account_state.state)
                    .or(Err(ProgramError::InvalidAccountData))?
            } else {
                AccountState::Initialized
            };

        account.base.mint = *mint_info.key;
        account.base.owner = *owner;
        account.base.close_authority = COption::None;
        account.base.delegate = COption::None;
        account.base.delegated_amount = 0;
        account.base.state = starting_state;
        if cmp_pubkeys(mint_info.key, &native_mint::id()) {
            let rent_exempt_reserve = rent.minimum_balance(new_account_info_data_len);
            account.base.is_native = COption::Some(rent_exempt_reserve);
            account.base.amount = new_account_info
                .lamports()
                .checked_sub(rent_exempt_reserve)
                .ok_or(TokenError::Overflow)?;
        } else {
            account.base.is_native = COption::None;
            account.base.amount = 0;
        };

        account.pack_base();
        account.init_account_type()?;

        Ok(())
    }

    /// Processes an [InitializeAccount](enum.TokenInstruction.html)
    /// instruction.
    pub fn process_initialize_account(accounts: &[AccountInfo]) -> ProgramResult {
        Self::_process_initialize_account(accounts, None, true)
    }

    /// Processes an [InitializeAccount2](enum.TokenInstruction.html)
    /// instruction.
    pub fn process_initialize_account2(accounts: &[AccountInfo], owner: Pubkey) -> ProgramResult {
        Self::_process_initialize_account(accounts, Some(&owner), true)
    }

    /// Processes an [InitializeAccount3](enum.TokenInstruction.html)
    /// instruction.
    pub fn process_initialize_account3(accounts: &[AccountInfo], owner: Pubkey) -> ProgramResult {
        Self::_process_initialize_account(accounts, Some(&owner), false)
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

    /// Processes a [InitializeMultisig](enum.TokenInstruction.html)
    /// instruction.
    pub fn process_initialize_multisig(accounts: &[AccountInfo], m: u8) -> ProgramResult {
        Self::_process_initialize_multisig(accounts, m, true)
    }

    /// Processes a [InitializeMultisig2](enum.TokenInstruction.html)
    /// instruction.
    pub fn process_initialize_multisig2(accounts: &[AccountInfo], m: u8) -> ProgramResult {
        Self::_process_initialize_multisig(accounts, m, false)
    }

    /// Processes a [Transfer](enum.TokenInstruction.html) instruction.
    pub fn process_transfer(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
        expected_decimals: Option<u8>,
        expected_fee: Option<u64>,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let source_account_info = next_account_info(account_info_iter)?;

        let expected_mint_info = if let Some(expected_decimals) = expected_decimals {
            Some((next_account_info(account_info_iter)?, expected_decimals))
        } else {
            None
        };

        let destination_account_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let authority_info_data_len = authority_info.data_len();

        let mut source_account_data = source_account_info.data.borrow_mut();
        let mut source_account =
            StateWithExtensionsMut::<Account>::unpack(&mut source_account_data)?;
        if source_account.base.is_frozen() {
            return Err(TokenError::AccountFrozen.into());
        }
        if source_account.base.amount < amount {
            return Err(TokenError::InsufficientFunds.into());
        }
        if source_account
            .get_extension::<NonTransferableAccount>()
            .is_ok()
        {
            return Err(TokenError::NonTransferable.into());
        }
        let (fee, maybe_permanent_delegate, maybe_transfer_hook_program_id) =
            if let Some((mint_info, expected_decimals)) = expected_mint_info {
                if !cmp_pubkeys(&source_account.base.mint, mint_info.key) {
                    return Err(TokenError::MintMismatch.into());
                }

                let mint_data = mint_info.try_borrow_data()?;
                let mint = StateWithExtensions::<Mint>::unpack(&mint_data)?;

                if expected_decimals != mint.base.decimals {
                    return Err(TokenError::MintDecimalsMismatch.into());
                }

                let fee = if let Ok(transfer_fee_config) = mint.get_extension::<TransferFeeConfig>()
                {
                    transfer_fee_config
                        .calculate_epoch_fee(Clock::get()?.epoch, amount)
                        .ok_or(TokenError::Overflow)?
                } else {
                    0
                };

                let maybe_permanent_delegate = get_permanent_delegate(&mint);
                let maybe_transfer_hook_program_id = transfer_hook::get_program_id(&mint);

                (
                    fee,
                    maybe_permanent_delegate,
                    maybe_transfer_hook_program_id,
                )
            } else {
                // Transfer hook extension exists on the account, but no mint
                // was provided to figure out required accounts, abort
                if source_account
                    .get_extension::<TransferHookAccount>()
                    .is_ok()
                {
                    return Err(TokenError::MintRequiredForTransfer.into());
                }

                // Transfer fee amount extension exists on the account, but no mint
                // was provided to calculate the fee, abort
                if source_account
                    .get_extension_mut::<TransferFeeAmount>()
                    .is_ok()
                {
                    return Err(TokenError::MintRequiredForTransfer.into());
                } else {
                    (0, None, None)
                }
            };
        if let Some(expected_fee) = expected_fee {
            if expected_fee != fee {
                msg!("Calculated fee {}, received {}", fee, expected_fee);
                return Err(TokenError::FeeMismatch.into());
            }
        }

        let self_transfer = cmp_pubkeys(source_account_info.key, destination_account_info.key);
        match (source_account.base.delegate, maybe_permanent_delegate) {
            (_, Some(ref delegate)) if cmp_pubkeys(authority_info.key, delegate) => {
                Self::validate_owner(
                    program_id,
                    delegate,
                    authority_info,
                    authority_info_data_len,
                    account_info_iter.as_slice(),
                )?
            }
            (COption::Some(ref delegate), _) if cmp_pubkeys(authority_info.key, delegate) => {
                Self::validate_owner(
                    program_id,
                    delegate,
                    authority_info,
                    authority_info_data_len,
                    account_info_iter.as_slice(),
                )?;
                if source_account.base.delegated_amount < amount {
                    return Err(TokenError::InsufficientFunds.into());
                }
                if !self_transfer {
                    source_account.base.delegated_amount = source_account
                        .base
                        .delegated_amount
                        .checked_sub(amount)
                        .ok_or(TokenError::Overflow)?;
                    if source_account.base.delegated_amount == 0 {
                        source_account.base.delegate = COption::None;
                    }
                }
            }
            _ => {
                Self::validate_owner(
                    program_id,
                    &source_account.base.owner,
                    authority_info,
                    authority_info_data_len,
                    account_info_iter.as_slice(),
                )?;

                if let Ok(cpi_guard) = source_account.get_extension::<CpiGuard>() {
                    if cpi_guard.lock_cpi.into() && in_cpi() {
                        return Err(TokenError::CpiGuardTransferBlocked.into());
                    }
                }
            }
        }

        // Revisit this later to see if it's worth adding a check to reduce
        // compute costs, ie:
        // if self_transfer || amount == 0
        check_program_account(source_account_info.owner)?;
        check_program_account(destination_account_info.owner)?;

        // This check MUST occur just before the amounts are manipulated
        // to ensure self-transfers are fully validated
        if self_transfer {
            return Ok(());
        }

        // self-transfer was dealt with earlier, so this *should* be safe
        let mut destination_account_data = destination_account_info.data.borrow_mut();
        let mut destination_account =
            StateWithExtensionsMut::<Account>::unpack(&mut destination_account_data)?;

        if destination_account.base.is_frozen() {
            return Err(TokenError::AccountFrozen.into());
        }
        if !cmp_pubkeys(&source_account.base.mint, &destination_account.base.mint) {
            return Err(TokenError::MintMismatch.into());
        }

        if memo_required(&destination_account) {
            check_previous_sibling_instruction_is_memo()?;
        }

        if let Ok(confidential_transfer_state) =
            destination_account.get_extension::<ConfidentialTransferAccount>()
        {
            confidential_transfer_state.non_confidential_transfer_allowed()?
        }

        source_account.base.amount = source_account
            .base
            .amount
            .checked_sub(amount)
            .ok_or(TokenError::Overflow)?;
        let credited_amount = amount.checked_sub(fee).ok_or(TokenError::Overflow)?;
        destination_account.base.amount = destination_account
            .base
            .amount
            .checked_add(credited_amount)
            .ok_or(TokenError::Overflow)?;
        if fee > 0 {
            if let Ok(extension) = destination_account.get_extension_mut::<TransferFeeAmount>() {
                let new_withheld_amount = u64::from(extension.withheld_amount)
                    .checked_add(fee)
                    .ok_or(TokenError::Overflow)?;
                extension.withheld_amount = new_withheld_amount.into();
            } else {
                // Use the generic error since this should never happen. If there's
                // a fee, then the mint has a fee configured, which means all accounts
                // must have the withholding.
                return Err(TokenError::InvalidState.into());
            }
        }

        if source_account.base.is_native() {
            let source_starting_lamports = source_account_info.lamports();
            **source_account_info.lamports.borrow_mut() = source_starting_lamports
                .checked_sub(amount)
                .ok_or(TokenError::Overflow)?;

            let destination_starting_lamports = destination_account_info.lamports();
            **destination_account_info.lamports.borrow_mut() = destination_starting_lamports
                .checked_add(amount)
                .ok_or(TokenError::Overflow)?;
        }

        source_account.pack_base();
        destination_account.pack_base();

        if let Some(program_id) = maybe_transfer_hook_program_id {
            if let Some((mint_info, _)) = expected_mint_info {
                // set transferring flags
                transfer_hook::set_transferring(&mut source_account)?;
                transfer_hook::set_transferring(&mut destination_account)?;

                // must drop these to avoid the double-borrow during CPI
                drop(source_account_data);
                drop(destination_account_data);
                spl_transfer_hook_interface::onchain::invoke_execute(
                    &program_id,
                    source_account_info.clone(),
                    mint_info.clone(),
                    destination_account_info.clone(),
                    authority_info.clone(),
                    account_info_iter.as_slice(),
                    amount,
                )?;

                // unset transferring flag
                transfer_hook::unset_transferring(source_account_info)?;
                transfer_hook::unset_transferring(destination_account_info)?;
            } else {
                return Err(TokenError::MintRequiredForTransfer.into());
            }
        }

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
        let owner_info_data_len = owner_info.data_len();

        let mut source_account_data = source_account_info.data.borrow_mut();
        let mut source_account =
            StateWithExtensionsMut::<Account>::unpack(&mut source_account_data)?;

        if source_account.base.is_frozen() {
            return Err(TokenError::AccountFrozen.into());
        }

        if let Some((mint_info, expected_decimals)) = expected_mint_info {
            if !cmp_pubkeys(&source_account.base.mint, mint_info.key) {
                return Err(TokenError::MintMismatch.into());
            }

            let mint_data = mint_info.data.borrow();
            let mint = StateWithExtensions::<Mint>::unpack(&mint_data)?;
            if expected_decimals != mint.base.decimals {
                return Err(TokenError::MintDecimalsMismatch.into());
            }
        }

        Self::validate_owner(
            program_id,
            &source_account.base.owner,
            owner_info,
            owner_info_data_len,
            account_info_iter.as_slice(),
        )?;

        if let Ok(cpi_guard) = source_account.get_extension::<CpiGuard>() {
            if cpi_guard.lock_cpi.into() && in_cpi() {
                return Err(TokenError::CpiGuardApproveBlocked.into());
            }
        }

        source_account.base.delegate = COption::Some(*delegate_info.key);
        source_account.base.delegated_amount = amount;
        source_account.pack_base();

        Ok(())
    }

    /// Processes an [Revoke](enum.TokenInstruction.html) instruction.
    pub fn process_revoke(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let authority_info_data_len = authority_info.data_len();

        let mut source_account_data = source_account_info.data.borrow_mut();
        let mut source_account =
            StateWithExtensionsMut::<Account>::unpack(&mut source_account_data)?;
        if source_account.base.is_frozen() {
            return Err(TokenError::AccountFrozen.into());
        }

        Self::validate_owner(
            program_id,
            match source_account.base.delegate {
                COption::Some(ref delegate) if cmp_pubkeys(authority_info.key, delegate) => {
                    delegate
                }
                _ => &source_account.base.owner,
            },
            authority_info,
            authority_info_data_len,
            account_info_iter.as_slice(),
        )?;

        source_account.base.delegate = COption::None;
        source_account.base.delegated_amount = 0;
        source_account.pack_base();

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
        let authority_info_data_len = authority_info.data_len();

        let mut account_data = account_info.data.borrow_mut();
        if let Ok(mut account) = StateWithExtensionsMut::<Account>::unpack(&mut account_data) {
            if account.base.is_frozen() {
                return Err(TokenError::AccountFrozen.into());
            }

            match authority_type {
                AuthorityType::AccountOwner => {
                    Self::validate_owner(
                        program_id,
                        &account.base.owner,
                        authority_info,
                        authority_info_data_len,
                        account_info_iter.as_slice(),
                    )?;

                    if account.get_extension_mut::<ImmutableOwner>().is_ok() {
                        return Err(TokenError::ImmutableOwner.into());
                    }

                    if let Ok(cpi_guard) = account.get_extension::<CpiGuard>() {
                        if cpi_guard.lock_cpi.into() && in_cpi() {
                            return Err(TokenError::CpiGuardSetAuthorityBlocked.into());
                        } else if cpi_guard.lock_cpi.into() {
                            return Err(TokenError::CpiGuardOwnerChangeBlocked.into());
                        }
                    }

                    if let COption::Some(authority) = new_authority {
                        account.base.owner = authority;
                    } else {
                        return Err(TokenError::InvalidInstruction.into());
                    }

                    account.base.delegate = COption::None;
                    account.base.delegated_amount = 0;

                    if account.base.is_native() {
                        account.base.close_authority = COption::None;
                    }
                }
                AuthorityType::CloseAccount => {
                    let authority = account.base.close_authority.unwrap_or(account.base.owner);
                    Self::validate_owner(
                        program_id,
                        &authority,
                        authority_info,
                        authority_info_data_len,
                        account_info_iter.as_slice(),
                    )?;

                    if let Ok(cpi_guard) = account.get_extension::<CpiGuard>() {
                        if cpi_guard.lock_cpi.into() && in_cpi() && new_authority != COption::None {
                            return Err(TokenError::CpiGuardSetAuthorityBlocked.into());
                        }
                    }

                    account.base.close_authority = new_authority;
                }
                _ => {
                    return Err(TokenError::AuthorityTypeNotSupported.into());
                }
            }
            account.pack_base();
        } else if let Ok(mut mint) = StateWithExtensionsMut::<Mint>::unpack(&mut account_data) {
            match authority_type {
                AuthorityType::MintTokens => {
                    // Once a mint's supply is fixed, it cannot be undone by setting a new
                    // mint_authority
                    let mint_authority = mint
                        .base
                        .mint_authority
                        .ok_or(Into::<ProgramError>::into(TokenError::FixedSupply))?;
                    Self::validate_owner(
                        program_id,
                        &mint_authority,
                        authority_info,
                        authority_info_data_len,
                        account_info_iter.as_slice(),
                    )?;
                    mint.base.mint_authority = new_authority;
                    mint.pack_base();
                }
                AuthorityType::FreezeAccount => {
                    // Once a mint's freeze authority is disabled, it cannot be re-enabled by
                    // setting a new freeze_authority
                    let freeze_authority = mint
                        .base
                        .freeze_authority
                        .ok_or(Into::<ProgramError>::into(TokenError::MintCannotFreeze))?;
                    Self::validate_owner(
                        program_id,
                        &freeze_authority,
                        authority_info,
                        authority_info_data_len,
                        account_info_iter.as_slice(),
                    )?;
                    mint.base.freeze_authority = new_authority;
                    mint.pack_base();
                }
                AuthorityType::CloseMint => {
                    let extension = mint.get_extension_mut::<MintCloseAuthority>()?;
                    let maybe_close_authority: Option<Pubkey> = extension.close_authority.into();
                    let close_authority =
                        maybe_close_authority.ok_or(TokenError::AuthorityTypeNotSupported)?;
                    Self::validate_owner(
                        program_id,
                        &close_authority,
                        authority_info,
                        authority_info_data_len,
                        account_info_iter.as_slice(),
                    )?;
                    extension.close_authority = new_authority.try_into()?;
                }
                AuthorityType::TransferFeeConfig => {
                    let extension = mint.get_extension_mut::<TransferFeeConfig>()?;
                    let maybe_transfer_fee_config_authority: Option<Pubkey> =
                        extension.transfer_fee_config_authority.into();
                    let transfer_fee_config_authority = maybe_transfer_fee_config_authority
                        .ok_or(TokenError::AuthorityTypeNotSupported)?;
                    Self::validate_owner(
                        program_id,
                        &transfer_fee_config_authority,
                        authority_info,
                        authority_info_data_len,
                        account_info_iter.as_slice(),
                    )?;
                    extension.transfer_fee_config_authority = new_authority.try_into()?;
                }
                AuthorityType::WithheldWithdraw => {
                    let extension = mint.get_extension_mut::<TransferFeeConfig>()?;
                    let maybe_withdraw_withheld_authority: Option<Pubkey> =
                        extension.withdraw_withheld_authority.into();
                    let withdraw_withheld_authority = maybe_withdraw_withheld_authority
                        .ok_or(TokenError::AuthorityTypeNotSupported)?;
                    Self::validate_owner(
                        program_id,
                        &withdraw_withheld_authority,
                        authority_info,
                        authority_info_data_len,
                        account_info_iter.as_slice(),
                    )?;
                    extension.withdraw_withheld_authority = new_authority.try_into()?;
                }
                AuthorityType::InterestRate => {
                    let extension = mint.get_extension_mut::<InterestBearingConfig>()?;
                    let maybe_rate_authority: Option<Pubkey> = extension.rate_authority.into();
                    let rate_authority =
                        maybe_rate_authority.ok_or(TokenError::AuthorityTypeNotSupported)?;
                    Self::validate_owner(
                        program_id,
                        &rate_authority,
                        authority_info,
                        authority_info_data_len,
                        account_info_iter.as_slice(),
                    )?;
                    extension.rate_authority = new_authority.try_into()?;
                }
                AuthorityType::PermanentDelegate => {
                    let extension = mint.get_extension_mut::<PermanentDelegate>()?;
                    let maybe_delegate: Option<Pubkey> = extension.delegate.into();
                    let delegate = maybe_delegate.ok_or(TokenError::AuthorityTypeNotSupported)?;
                    Self::validate_owner(
                        program_id,
                        &delegate,
                        authority_info,
                        authority_info_data_len,
                        account_info_iter.as_slice(),
                    )?;
                    extension.delegate = new_authority.try_into()?;
                }
                AuthorityType::ConfidentialTransferMint => {
                    let extension = mint.get_extension_mut::<ConfidentialTransferMint>()?;
                    let maybe_confidential_transfer_mint_authority: Option<Pubkey> =
                        extension.authority.into();
                    let confidential_transfer_mint_authority =
                        maybe_confidential_transfer_mint_authority
                            .ok_or(TokenError::AuthorityTypeNotSupported)?;
                    Self::validate_owner(
                        program_id,
                        &confidential_transfer_mint_authority,
                        authority_info,
                        authority_info_data_len,
                        account_info_iter.as_slice(),
                    )?;
                    extension.authority = new_authority.try_into()?;
                }
                AuthorityType::TransferHookProgramId => {
                    let extension = mint.get_extension_mut::<TransferHook>()?;
                    let maybe_authority: Option<Pubkey> = extension.authority.into();
                    let authority = maybe_authority.ok_or(TokenError::AuthorityTypeNotSupported)?;
                    Self::validate_owner(
                        program_id,
                        &authority,
                        authority_info,
                        authority_info_data_len,
                        account_info_iter.as_slice(),
                    )?;
                    extension.authority = new_authority.try_into()?;
                }
                AuthorityType::ConfidentialTransferFeeConfig => {
                    let extension = mint.get_extension_mut::<ConfidentialTransferFeeConfig>()?;
                    let maybe_authority: Option<Pubkey> = extension.authority.into();
                    let authority = maybe_authority.ok_or(TokenError::AuthorityTypeNotSupported)?;
                    Self::validate_owner(
                        program_id,
                        &authority,
                        authority_info,
                        authority_info_data_len,
                        account_info_iter.as_slice(),
                    )?;
                    extension.authority = new_authority.try_into()?;
                }
                AuthorityType::MetadataPointer => {
                    let extension = mint.get_extension_mut::<MetadataPointer>()?;
                    let maybe_authority: Option<Pubkey> = extension.authority.into();
                    let authority = maybe_authority.ok_or(TokenError::AuthorityTypeNotSupported)?;
                    Self::validate_owner(
                        program_id,
                        &authority,
                        authority_info,
                        authority_info_data_len,
                        account_info_iter.as_slice(),
                    )?;
                    extension.authority = new_authority.try_into()?;
                }
                AuthorityType::GroupPointer => {
                    let extension = mint.get_extension_mut::<GroupPointer>()?;
                    let maybe_authority: Option<Pubkey> = extension.authority.into();
                    let authority = maybe_authority.ok_or(TokenError::AuthorityTypeNotSupported)?;
                    Self::validate_owner(
                        program_id,
                        &authority,
                        authority_info,
                        authority_info_data_len,
                        account_info_iter.as_slice(),
                    )?;
                    extension.authority = new_authority.try_into()?;
                }
                AuthorityType::GroupMemberPointer => {
                    let extension = mint.get_extension_mut::<GroupMemberPointer>()?;
                    let maybe_authority: Option<Pubkey> = extension.authority.into();
                    let authority = maybe_authority.ok_or(TokenError::AuthorityTypeNotSupported)?;
                    Self::validate_owner(
                        program_id,
                        &authority,
                        authority_info,
                        authority_info_data_len,
                        account_info_iter.as_slice(),
                    )?;
                    extension.authority = new_authority.try_into()?;
                }
                _ => {
                    return Err(TokenError::AuthorityTypeNotSupported.into());
                }
            }
        } else {
            return Err(ProgramError::InvalidAccountData);
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
        let destination_account_info = next_account_info(account_info_iter)?;
        let owner_info = next_account_info(account_info_iter)?;
        let owner_info_data_len = owner_info.data_len();

        let mut destination_account_data = destination_account_info.data.borrow_mut();
        let mut destination_account =
            StateWithExtensionsMut::<Account>::unpack(&mut destination_account_data)?;
        if destination_account.base.is_frozen() {
            return Err(TokenError::AccountFrozen.into());
        }

        if destination_account.base.is_native() {
            return Err(TokenError::NativeNotSupported.into());
        }
        if !cmp_pubkeys(mint_info.key, &destination_account.base.mint) {
            return Err(TokenError::MintMismatch.into());
        }

        let mut mint_data = mint_info.data.borrow_mut();
        let mut mint = StateWithExtensionsMut::<Mint>::unpack(&mut mint_data)?;

        // If the mint if non-transferable, only allow minting to accounts
        // with immutable ownership.
        if mint.get_extension::<NonTransferable>().is_ok()
            && destination_account
                .get_extension::<ImmutableOwner>()
                .is_err()
        {
            return Err(TokenError::NonTransferableNeedsImmutableOwnership.into());
        }

        if let Some(expected_decimals) = expected_decimals {
            if expected_decimals != mint.base.decimals {
                return Err(TokenError::MintDecimalsMismatch.into());
            }
        }

        match mint.base.mint_authority {
            COption::Some(mint_authority) => Self::validate_owner(
                program_id,
                &mint_authority,
                owner_info,
                owner_info_data_len,
                account_info_iter.as_slice(),
            )?,
            COption::None => return Err(TokenError::FixedSupply.into()),
        }

        // Revisit this later to see if it's worth adding a check to reduce
        // compute costs, ie:
        // if amount == 0
        check_program_account(mint_info.owner)?;
        check_program_account(destination_account_info.owner)?;

        destination_account.base.amount = destination_account
            .base
            .amount
            .checked_add(amount)
            .ok_or(TokenError::Overflow)?;

        mint.base.supply = mint
            .base
            .supply
            .checked_add(amount)
            .ok_or(TokenError::Overflow)?;

        mint.pack_base();
        destination_account.pack_base();

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
        let authority_info_data_len = authority_info.data_len();

        let mut source_account_data = source_account_info.data.borrow_mut();
        let mut source_account =
            StateWithExtensionsMut::<Account>::unpack(&mut source_account_data)?;
        let mut mint_data = mint_info.data.borrow_mut();
        let mut mint = StateWithExtensionsMut::<Mint>::unpack(&mut mint_data)?;

        if source_account.base.is_frozen() {
            return Err(TokenError::AccountFrozen.into());
        }
        if source_account.base.is_native() {
            return Err(TokenError::NativeNotSupported.into());
        }
        if source_account.base.amount < amount {
            return Err(TokenError::InsufficientFunds.into());
        }
        if mint_info.key != &source_account.base.mint {
            return Err(TokenError::MintMismatch.into());
        }

        if let Some(expected_decimals) = expected_decimals {
            if expected_decimals != mint.base.decimals {
                return Err(TokenError::MintDecimalsMismatch.into());
            }
        }
        let maybe_permanent_delegate = get_permanent_delegate(&mint);

        if !source_account
            .base
            .is_owned_by_system_program_or_incinerator()
        {
            match (source_account.base.delegate, maybe_permanent_delegate) {
                (_, Some(ref delegate)) if cmp_pubkeys(authority_info.key, delegate) => {
                    Self::validate_owner(
                        program_id,
                        delegate,
                        authority_info,
                        authority_info_data_len,
                        account_info_iter.as_slice(),
                    )?
                }
                (COption::Some(ref delegate), _) if cmp_pubkeys(authority_info.key, delegate) => {
                    Self::validate_owner(
                        program_id,
                        delegate,
                        authority_info,
                        authority_info_data_len,
                        account_info_iter.as_slice(),
                    )?;

                    if source_account.base.delegated_amount < amount {
                        return Err(TokenError::InsufficientFunds.into());
                    }
                    source_account.base.delegated_amount = source_account
                        .base
                        .delegated_amount
                        .checked_sub(amount)
                        .ok_or(TokenError::Overflow)?;
                    if source_account.base.delegated_amount == 0 {
                        source_account.base.delegate = COption::None;
                    }
                }
                _ => {
                    Self::validate_owner(
                        program_id,
                        &source_account.base.owner,
                        authority_info,
                        authority_info_data_len,
                        account_info_iter.as_slice(),
                    )?;

                    if let Ok(cpi_guard) = source_account.get_extension::<CpiGuard>() {
                        if cpi_guard.lock_cpi.into() && in_cpi() {
                            return Err(TokenError::CpiGuardBurnBlocked.into());
                        }
                    }
                }
            }
        }

        // Revisit this later to see if it's worth adding a check to reduce
        // compute costs, ie:
        // if amount == 0
        check_program_account(source_account_info.owner)?;
        check_program_account(mint_info.owner)?;

        source_account.base.amount = source_account
            .base
            .amount
            .checked_sub(amount)
            .ok_or(TokenError::Overflow)?;
        mint.base.supply = mint
            .base
            .supply
            .checked_sub(amount)
            .ok_or(TokenError::Overflow)?;

        source_account.pack_base();
        mint.pack_base();

        Ok(())
    }

    /// Processes a [CloseAccount](enum.TokenInstruction.html) instruction.
    pub fn process_close_account(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let destination_account_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let authority_info_data_len = authority_info.data_len();

        if cmp_pubkeys(source_account_info.key, destination_account_info.key) {
            return Err(ProgramError::InvalidAccountData);
        }

        let source_account_data = source_account_info.data.borrow();
        if let Ok(source_account) = StateWithExtensions::<Account>::unpack(&source_account_data) {
            if !source_account.base.is_native() && source_account.base.amount != 0 {
                return Err(TokenError::NonNativeHasBalance.into());
            }

            let authority = source_account
                .base
                .close_authority
                .unwrap_or(source_account.base.owner);

            if !source_account
                .base
                .is_owned_by_system_program_or_incinerator()
            {
                if let Ok(cpi_guard) = source_account.get_extension::<CpiGuard>() {
                    if cpi_guard.lock_cpi.into()
                        && in_cpi()
                        && !cmp_pubkeys(destination_account_info.key, &source_account.base.owner)
                    {
                        return Err(TokenError::CpiGuardCloseAccountBlocked.into());
                    }
                }

                Self::validate_owner(
                    program_id,
                    &authority,
                    authority_info,
                    authority_info_data_len,
                    account_info_iter.as_slice(),
                )?;
            } else if !solana_program::incinerator::check_id(destination_account_info.key) {
                return Err(ProgramError::InvalidAccountData);
            }

            if let Ok(confidential_transfer_state) =
                source_account.get_extension::<ConfidentialTransferAccount>()
            {
                confidential_transfer_state.closable()?
            }

            if let Ok(confidential_transfer_fee_state) =
                source_account.get_extension::<ConfidentialTransferFeeAmount>()
            {
                confidential_transfer_fee_state.closable()?
            }

            if let Ok(transfer_fee_state) = source_account.get_extension::<TransferFeeAmount>() {
                transfer_fee_state.closable()?
            }
        } else if let Ok(mint) = StateWithExtensions::<Mint>::unpack(&source_account_data) {
            let extension = mint.get_extension::<MintCloseAuthority>()?;
            let maybe_authority: Option<Pubkey> = extension.close_authority.into();
            let authority = maybe_authority.ok_or(TokenError::AuthorityTypeNotSupported)?;
            Self::validate_owner(
                program_id,
                &authority,
                authority_info,
                authority_info_data_len,
                account_info_iter.as_slice(),
            )?;

            if mint.base.supply != 0 {
                return Err(TokenError::MintHasSupply.into());
            }
        } else {
            return Err(ProgramError::UninitializedAccount);
        }

        let destination_starting_lamports = destination_account_info.lamports();
        **destination_account_info.lamports.borrow_mut() = destination_starting_lamports
            .checked_add(source_account_info.lamports())
            .ok_or(TokenError::Overflow)?;

        **source_account_info.lamports.borrow_mut() = 0;
        drop(source_account_data);
        delete_account(source_account_info)?;

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
        let authority_info_data_len = authority_info.data_len();

        let mut source_account_data = source_account_info.data.borrow_mut();
        let mut source_account =
            StateWithExtensionsMut::<Account>::unpack(&mut source_account_data)?;
        if freeze && source_account.base.is_frozen() || !freeze && !source_account.base.is_frozen()
        {
            return Err(TokenError::InvalidState.into());
        }
        if source_account.base.is_native() {
            return Err(TokenError::NativeNotSupported.into());
        }
        if !cmp_pubkeys(mint_info.key, &source_account.base.mint) {
            return Err(TokenError::MintMismatch.into());
        }

        let mint_data = mint_info.data.borrow();
        let mint = StateWithExtensions::<Mint>::unpack(&mint_data)?;
        match mint.base.freeze_authority {
            COption::Some(authority) => Self::validate_owner(
                program_id,
                &authority,
                authority_info,
                authority_info_data_len,
                account_info_iter.as_slice(),
            ),
            COption::None => Err(TokenError::MintCannotFreeze.into()),
        }?;

        source_account.base.state = if freeze {
            AccountState::Frozen
        } else {
            AccountState::Initialized
        };

        source_account.pack_base();

        Ok(())
    }

    /// Processes a [SyncNative](enum.TokenInstruction.html) instruction
    pub fn process_sync_native(accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let native_account_info = next_account_info(account_info_iter)?;

        check_program_account(native_account_info.owner)?;
        let mut native_account_data = native_account_info.data.borrow_mut();
        let mut native_account =
            StateWithExtensionsMut::<Account>::unpack(&mut native_account_data)?;

        if let COption::Some(rent_exempt_reserve) = native_account.base.is_native {
            let new_amount = native_account_info
                .lamports()
                .checked_sub(rent_exempt_reserve)
                .ok_or(TokenError::Overflow)?;
            if new_amount < native_account.base.amount {
                return Err(TokenError::InvalidState.into());
            }
            native_account.base.amount = new_amount;
        } else {
            return Err(TokenError::NonNativeNotSupported.into());
        }

        native_account.pack_base();
        Ok(())
    }

    /// Processes an [InitializeMintCloseAuthority](enum.TokenInstruction.html)
    /// instruction
    pub fn process_initialize_mint_close_authority(
        accounts: &[AccountInfo],
        close_authority: COption<Pubkey>,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let mint_account_info = next_account_info(account_info_iter)?;

        let mut mint_data = mint_account_info.data.borrow_mut();
        let mut mint = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut mint_data)?;
        let extension = mint.init_extension::<MintCloseAuthority>(true)?;
        extension.close_authority = close_authority.try_into()?;

        Ok(())
    }

    /// Processes a [GetAccountDataSize](enum.TokenInstruction.html) instruction
    pub fn process_get_account_data_size(
        accounts: &[AccountInfo],
        new_extension_types: Vec<ExtensionType>,
    ) -> ProgramResult {
        if new_extension_types
            .iter()
            .any(|&t| t.get_account_type() != AccountType::Account)
        {
            return Err(TokenError::ExtensionTypeMismatch.into());
        }

        let account_info_iter = &mut accounts.iter();
        let mint_account_info = next_account_info(account_info_iter)?;

        let mut account_extensions = Self::get_required_account_extensions(mint_account_info)?;
        // ExtensionType::try_calculate_account_len() dedupes types, so just a dumb
        // concatenation is fine here
        account_extensions.extend_from_slice(&new_extension_types);

        let account_len = ExtensionType::try_calculate_account_len::<Account>(&account_extensions)?;
        set_return_data(&account_len.to_le_bytes());

        Ok(())
    }

    /// Processes an [InitializeImmutableOwner](enum.TokenInstruction.html)
    /// instruction
    pub fn process_initialize_immutable_owner(accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let token_account_info = next_account_info(account_info_iter)?;
        let token_account_data = &mut token_account_info.data.borrow_mut();
        let mut token_account =
            StateWithExtensionsMut::<Account>::unpack_uninitialized(token_account_data)?;
        token_account
            .init_extension::<ImmutableOwner>(true)
            .map(|_| ())
    }

    /// Processes an [AmountToUiAmount](enum.TokenInstruction.html) instruction
    pub fn process_amount_to_ui_amount(accounts: &[AccountInfo], amount: u64) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let mint_info = next_account_info(account_info_iter)?;
        check_program_account(mint_info.owner)?;

        let mint_data = mint_info.data.borrow();
        let mint = StateWithExtensions::<Mint>::unpack(&mint_data)
            .map_err(|_| Into::<ProgramError>::into(TokenError::InvalidMint))?;
        let ui_amount = if let Ok(extension) = mint.get_extension::<InterestBearingConfig>() {
            let unix_timestamp = Clock::get()?.unix_timestamp;
            extension
                .amount_to_ui_amount(amount, mint.base.decimals, unix_timestamp)
                .ok_or(ProgramError::InvalidArgument)?
        } else {
            crate::amount_to_ui_amount_string_trimmed(amount, mint.base.decimals)
        };

        set_return_data(&ui_amount.into_bytes());
        Ok(())
    }

    /// Processes an [AmountToUiAmount](enum.TokenInstruction.html) instruction
    pub fn process_ui_amount_to_amount(accounts: &[AccountInfo], ui_amount: &str) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let mint_info = next_account_info(account_info_iter)?;
        check_program_account(mint_info.owner)?;

        let mint_data = mint_info.data.borrow();
        let mint = StateWithExtensions::<Mint>::unpack(&mint_data)
            .map_err(|_| Into::<ProgramError>::into(TokenError::InvalidMint))?;
        let amount = if let Ok(extension) = mint.get_extension::<InterestBearingConfig>() {
            let unix_timestamp = Clock::get()?.unix_timestamp;
            extension.try_ui_amount_into_amount(ui_amount, mint.base.decimals, unix_timestamp)?
        } else {
            crate::try_ui_amount_into_amount(ui_amount.to_string(), mint.base.decimals)?
        };

        set_return_data(&amount.to_le_bytes());
        Ok(())
    }

    /// Processes a [CreateNativeMint](enum.TokenInstruction.html) instruction
    pub fn process_create_native_mint(accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let payer_info = next_account_info(account_info_iter)?;
        let native_mint_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;

        if *native_mint_info.key != native_mint::id() {
            return Err(TokenError::InvalidMint.into());
        }

        let rent = Rent::get()?;
        let new_minimum_balance = rent.minimum_balance(Mint::get_packed_len());
        let lamports_diff = new_minimum_balance.saturating_sub(native_mint_info.lamports());
        invoke(
            &system_instruction::transfer(payer_info.key, native_mint_info.key, lamports_diff),
            &[
                payer_info.clone(),
                native_mint_info.clone(),
                system_program_info.clone(),
            ],
        )?;

        invoke_signed(
            &system_instruction::allocate(native_mint_info.key, Mint::get_packed_len() as u64),
            &[native_mint_info.clone(), system_program_info.clone()],
            &[native_mint::PROGRAM_ADDRESS_SEEDS],
        )?;

        invoke_signed(
            &system_instruction::assign(native_mint_info.key, &crate::id()),
            &[native_mint_info.clone(), system_program_info.clone()],
            &[native_mint::PROGRAM_ADDRESS_SEEDS],
        )?;

        Mint::pack(
            Mint {
                decimals: native_mint::DECIMALS,
                is_initialized: true,
                ..Mint::default()
            },
            &mut native_mint_info.data.borrow_mut(),
        )
    }

    /// Processes an [InitializeNonTransferableMint](enum.TokenInstruction.html)
    /// instruction
    pub fn process_initialize_non_transferable_mint(accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let mint_account_info = next_account_info(account_info_iter)?;

        let mut mint_data = mint_account_info.data.borrow_mut();
        let mut mint = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut mint_data)?;
        mint.init_extension::<NonTransferable>(true)?;

        Ok(())
    }

    /// Processes an [InitializePermanentDelegate](enum.TokenInstruction.html)
    /// instruction
    pub fn process_initialize_permanent_delegate(
        accounts: &[AccountInfo],
        delegate: Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let mint_account_info = next_account_info(account_info_iter)?;

        let mut mint_data = mint_account_info.data.borrow_mut();
        let mut mint = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut mint_data)?;
        let extension = mint.init_extension::<PermanentDelegate>(true)?;
        extension.delegate = Some(delegate).try_into()?;

        Ok(())
    }

    /// Withdraw Excess Lamports is used to recover Lamports transfered to any
    /// TokenProgram owned account by moving them to another account
    /// of the source account.
    pub fn process_withdraw_excess_lamports(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let source_info = next_account_info(account_info_iter)?;
        let destination_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;

        let source_data = source_info.data.borrow();

        if let Ok(account) = StateWithExtensions::<Account>::unpack(&source_data) {
            if account.base.is_native() {
                return Err(TokenError::NativeNotSupported.into());
            }
            Self::validate_owner(
                program_id,
                &account.base.owner,
                authority_info,
                authority_info.data_len(),
                account_info_iter.as_slice(),
            )?;
        } else if let Ok(mint) = StateWithExtensions::<Mint>::unpack(&source_data) {
            if let COption::Some(mint_authority) = mint.base.mint_authority {
                Self::validate_owner(
                    program_id,
                    &mint_authority,
                    authority_info,
                    authority_info.data_len(),
                    account_info_iter.as_slice(),
                )?;
            } else {
                return Err(TokenError::AuthorityTypeNotSupported.into());
            }
        } else if source_data.len() == Multisig::LEN {
            Self::validate_owner(
                program_id,
                source_info.key,
                authority_info,
                authority_info.data_len(),
                account_info_iter.as_slice(),
            )?;
        } else {
            return Err(TokenError::InvalidState.into());
        }

        let source_rent_exempt_reserve = Rent::get()?.minimum_balance(source_info.data_len());

        let transfer_amount = source_info
            .lamports()
            .checked_sub(source_rent_exempt_reserve)
            .ok_or(TokenError::NotRentExempt)?;

        let source_starting_lamports = source_info.lamports();
        **source_info.lamports.borrow_mut() = source_starting_lamports
            .checked_sub(transfer_amount)
            .ok_or(TokenError::Overflow)?;

        let destination_starting_lamports = destination_info.lamports();
        **destination_info.lamports.borrow_mut() = destination_starting_lamports
            .checked_add(transfer_amount)
            .ok_or(TokenError::Overflow)?;

        Ok(())
    }

    /// Processes an [Instruction](enum.Instruction.html).
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        if let Ok(instruction) = TokenInstruction::unpack(input) {
            match instruction {
                TokenInstruction::InitializeMint {
                    decimals,
                    mint_authority,
                    freeze_authority,
                } => {
                    msg!("Instruction: InitializeMint");
                    Self::process_initialize_mint(
                        accounts,
                        decimals,
                        mint_authority,
                        freeze_authority,
                    )
                }
                TokenInstruction::InitializeMint2 {
                    decimals,
                    mint_authority,
                    freeze_authority,
                } => {
                    msg!("Instruction: InitializeMint2");
                    Self::process_initialize_mint2(
                        accounts,
                        decimals,
                        mint_authority,
                        freeze_authority,
                    )
                }
                TokenInstruction::InitializeAccount => {
                    msg!("Instruction: InitializeAccount");
                    Self::process_initialize_account(accounts)
                }
                TokenInstruction::InitializeAccount2 { owner } => {
                    msg!("Instruction: InitializeAccount2");
                    Self::process_initialize_account2(accounts, owner)
                }
                TokenInstruction::InitializeAccount3 { owner } => {
                    msg!("Instruction: InitializeAccount3");
                    Self::process_initialize_account3(accounts, owner)
                }
                TokenInstruction::InitializeMultisig { m } => {
                    msg!("Instruction: InitializeMultisig");
                    Self::process_initialize_multisig(accounts, m)
                }
                TokenInstruction::InitializeMultisig2 { m } => {
                    msg!("Instruction: InitializeMultisig2");
                    Self::process_initialize_multisig2(accounts, m)
                }
                #[allow(deprecated)]
                TokenInstruction::Transfer { amount } => {
                    msg!("Instruction: Transfer");
                    Self::process_transfer(program_id, accounts, amount, None, None)
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
                    Self::process_transfer(program_id, accounts, amount, Some(decimals), None)
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
                    Self::process_sync_native(accounts)
                }
                TokenInstruction::GetAccountDataSize { extension_types } => {
                    msg!("Instruction: GetAccountDataSize");
                    Self::process_get_account_data_size(accounts, extension_types)
                }
                TokenInstruction::InitializeMintCloseAuthority { close_authority } => {
                    msg!("Instruction: InitializeMintCloseAuthority");
                    Self::process_initialize_mint_close_authority(accounts, close_authority)
                }
                TokenInstruction::TransferFeeExtension(instruction) => {
                    transfer_fee::processor::process_instruction(program_id, accounts, instruction)
                }
                TokenInstruction::ConfidentialTransferExtension => {
                    confidential_transfer::processor::process_instruction(
                        program_id,
                        accounts,
                        &input[1..],
                    )
                }
                TokenInstruction::DefaultAccountStateExtension => {
                    default_account_state::processor::process_instruction(
                        program_id,
                        accounts,
                        &input[1..],
                    )
                }
                TokenInstruction::InitializeImmutableOwner => {
                    msg!("Instruction: InitializeImmutableOwner");
                    Self::process_initialize_immutable_owner(accounts)
                }
                TokenInstruction::AmountToUiAmount { amount } => {
                    msg!("Instruction: AmountToUiAmount");
                    Self::process_amount_to_ui_amount(accounts, amount)
                }
                TokenInstruction::UiAmountToAmount { ui_amount } => {
                    msg!("Instruction: UiAmountToAmount");
                    Self::process_ui_amount_to_amount(accounts, ui_amount)
                }
                TokenInstruction::Reallocate { extension_types } => {
                    msg!("Instruction: Reallocate");
                    reallocate::process_reallocate(program_id, accounts, extension_types)
                }
                TokenInstruction::MemoTransferExtension => {
                    memo_transfer::processor::process_instruction(program_id, accounts, &input[1..])
                }
                TokenInstruction::CreateNativeMint => {
                    msg!("Instruction: CreateNativeMint");
                    Self::process_create_native_mint(accounts)
                }
                TokenInstruction::InitializeNonTransferableMint => {
                    msg!("Instruction: InitializeNonTransferableMint");
                    Self::process_initialize_non_transferable_mint(accounts)
                }
                TokenInstruction::InterestBearingMintExtension => {
                    interest_bearing_mint::processor::process_instruction(
                        program_id,
                        accounts,
                        &input[1..],
                    )
                }
                TokenInstruction::CpiGuardExtension => {
                    cpi_guard::processor::process_instruction(program_id, accounts, &input[1..])
                }
                TokenInstruction::InitializePermanentDelegate { delegate } => {
                    msg!("Instruction: InitializePermanentDelegate");
                    Self::process_initialize_permanent_delegate(accounts, delegate)
                }
                TokenInstruction::TransferHookExtension => {
                    transfer_hook::processor::process_instruction(program_id, accounts, &input[1..])
                }
                TokenInstruction::ConfidentialTransferFeeExtension => {
                    confidential_transfer_fee::processor::process_instruction(
                        program_id,
                        accounts,
                        &input[1..],
                    )
                }
                TokenInstruction::WithdrawExcessLamports => {
                    msg!("Instruction: WithdrawExcessLamports");
                    Self::process_withdraw_excess_lamports(program_id, accounts)
                }
                TokenInstruction::MetadataPointerExtension => {
                    metadata_pointer::processor::process_instruction(
                        program_id,
                        accounts,
                        &input[1..],
                    )
                }
                TokenInstruction::GroupPointerExtension => {
                    group_pointer::processor::process_instruction(program_id, accounts, &input[1..])
                }
                TokenInstruction::GroupMemberPointerExtension => {
                    group_member_pointer::processor::process_instruction(
                        program_id,
                        accounts,
                        &input[1..],
                    )
                }
            }
        } else if let Ok(instruction) = TokenMetadataInstruction::unpack(input) {
            token_metadata::processor::process_instruction(program_id, accounts, instruction)
        } else if let Ok(instruction) = TokenGroupInstruction::unpack(input) {
            token_group::processor::process_instruction(program_id, accounts, instruction)
        } else {
            Err(TokenError::InvalidInstruction.into())
        }
    }

    /// Validates owner(s) are present. Used for Mints and Accounts only.
    pub fn validate_owner(
        program_id: &Pubkey,
        expected_owner: &Pubkey,
        owner_account_info: &AccountInfo,
        owner_account_data_len: usize,
        signers: &[AccountInfo],
    ) -> ProgramResult {
        if !cmp_pubkeys(expected_owner, owner_account_info.key) {
            return Err(TokenError::OwnerMismatch.into());
        }

        if cmp_pubkeys(program_id, owner_account_info.owner)
            && owner_account_data_len == Multisig::get_packed_len()
        {
            let multisig = Multisig::unpack(&owner_account_info.data.borrow())?;
            let mut num_signers = 0;
            let mut matched = [false; MAX_SIGNERS];
            for signer in signers.iter() {
                for (position, key) in multisig.signers[0..multisig.n as usize].iter().enumerate() {
                    if cmp_pubkeys(key, signer.key) && !matched[position] {
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

    fn get_required_account_extensions(
        mint_account_info: &AccountInfo,
    ) -> Result<Vec<ExtensionType>, ProgramError> {
        let mint_data = mint_account_info.data.borrow();
        let state = StateWithExtensions::<Mint>::unpack(&mint_data)
            .map_err(|_| Into::<ProgramError>::into(TokenError::InvalidMint))?;
        Self::get_required_account_extensions_from_unpacked_mint(mint_account_info.owner, &state)
    }

    fn get_required_account_extensions_from_unpacked_mint(
        token_program_id: &Pubkey,
        state: &StateWithExtensions<Mint>,
    ) -> Result<Vec<ExtensionType>, ProgramError> {
        check_program_account(token_program_id)?;
        let mint_extensions: Vec<ExtensionType> = state.get_extension_types()?;
        Ok(ExtensionType::get_required_init_account_extensions(
            &mint_extensions,
        ))
    }
}

/// Helper function to mostly delete an account in a test environment.  We could
/// potentially muck around the bytes assuming that a vec is passed in, but that
/// would be more trouble than it's worth.
#[cfg(not(target_os = "solana"))]
fn delete_account(account_info: &AccountInfo) -> Result<(), ProgramError> {
    account_info.assign(&system_program::id());
    let mut account_data = account_info.data.borrow_mut();
    let data_len = account_data.len();
    solana_program::program_memory::sol_memset(*account_data, 0, data_len);
    Ok(())
}

/// Helper function to totally delete an account on-chain
#[cfg(target_os = "solana")]
fn delete_account(account_info: &AccountInfo) -> Result<(), ProgramError> {
    account_info.assign(&system_program::id());
    account_info.realloc(0, false)
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{
            extension::transfer_fee::instruction::initialize_transfer_fee_config, instruction::*,
        },
        serial_test::serial,
        solana_program::{
            account_info::IntoAccountInfo,
            clock::Epoch,
            instruction::Instruction,
            program_error::{self, PrintProgramError},
            sysvar::{clock::Clock, rent},
        },
        solana_sdk::account::{
            create_account_for_test, create_is_signer_account_infos, Account as SolanaAccount,
        },
        std::sync::{Arc, RwLock},
    };

    lazy_static::lazy_static! {
        static ref EXPECTED_DATA: Arc<RwLock<Vec<u8>>> = Arc::new(RwLock::new(Vec::new()));
    }

    fn set_expected_data(expected_data: Vec<u8>) {
        *EXPECTED_DATA.write().unwrap() = expected_data;
    }

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

        fn sol_get_clock_sysvar(&self, var_addr: *mut u8) -> u64 {
            unsafe {
                *(var_addr as *mut _ as *mut Clock) = Clock::default();
            }
            solana_program::entrypoint::SUCCESS
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

        fn sol_set_return_data(&self, data: &[u8]) {
            assert_eq!(&*EXPECTED_DATA.read().unwrap(), data)
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

    fn native_mint() -> SolanaAccount {
        let mut rent_sysvar = rent_sysvar();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &crate::id());
        do_process_instruction(
            initialize_mint(
                &crate::id(),
                &crate::native_mint::id(),
                &Pubkey::default(),
                None,
                crate::native_mint::DECIMALS,
            )
            .unwrap(),
            vec![&mut mint_account, &mut rent_sysvar],
        )
        .unwrap();
        mint_account
    }

    #[test]
    fn test_print_error() {
        let error = return_token_error_as_program_error();
        error.print::<TokenError>();
    }

    #[test]
    fn test_error_as_custom() {
        assert_eq!(
            return_token_error_as_program_error(),
            ProgramError::Custom(3)
        );
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
            #[allow(deprecated)]
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
            #[allow(deprecated)]
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
            #[allow(deprecated)]
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
            #[allow(deprecated)]
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
        #[allow(deprecated)]
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
                #[allow(deprecated)]
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
                #[allow(deprecated)]
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
                #[allow(deprecated)]
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
                #[allow(deprecated)]
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
            #[allow(deprecated)]
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
                #[allow(deprecated)]
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
            #[allow(deprecated)]
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
                #[allow(deprecated)]
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

        // not a delegate of source account
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            do_process_instruction(
                #[allow(deprecated)]
                transfer(
                    &program_id,
                    &account_key,
                    &account2_key,
                    &owner2_key, // <-- incorrect owner or delegate
                    &[],
                    1,
                )
                .unwrap(),
                vec![
                    &mut account_account,
                    &mut account2_account,
                    &mut owner2_account,
                ],
            )
        );

        // insufficient funds approved via delegate
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            do_process_instruction(
                #[allow(deprecated)]
                transfer(
                    &program_id,
                    &account_key,
                    &account2_key,
                    &delegate_key,
                    &[],
                    101
                )
                .unwrap(),
                vec![
                    &mut account_account,
                    &mut account2_account,
                    &mut delegate_account,
                ],
            )
        );

        // transfer via delegate
        do_process_instruction(
            #[allow(deprecated)]
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
                #[allow(deprecated)]
                transfer(
                    &program_id,
                    &account_key,
                    &account2_key,
                    &delegate_key,
                    &[],
                    1
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
            #[allow(deprecated)]
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
                #[allow(deprecated)]
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
        #[allow(deprecated)]
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
        #[allow(deprecated)]
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
        #[allow(deprecated)]
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
        #[allow(deprecated)]
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
        #[allow(deprecated)]
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
        #[allow(deprecated)]
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
        #[allow(deprecated)]
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

        // approve to source
        do_process_instruction_dups(
            approve_checked(
                &program_id,
                &account2_key,
                &mint_key,
                &account2_key,
                &owner_key,
                &[],
                500,
                2,
            )
            .unwrap(),
            vec![
                account2_info.clone(),
                mint_info.clone(),
                account2_info.clone(),
                owner_info.clone(),
            ],
        )
        .unwrap();

        // source-delegate revoke, force account2 to be a signer
        let account2_info: AccountInfo = (&account2_key, true, &mut account2_account).into();
        do_process_instruction_dups(
            revoke(&program_id, &account2_key, &account2_key, &[]).unwrap(),
            vec![account2_info.clone(), account2_info.clone()],
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

        // approve delegate 3
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

        // revoke by delegate
        do_process_instruction(
            revoke(&program_id, &account_key, &delegate_key, &[]).unwrap(),
            vec![&mut account_account, &mut delegate_account],
        )
        .unwrap();

        // fails the second time
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            do_process_instruction(
                revoke(&program_id, &account_key, &delegate_key, &[]).unwrap(),
                vec![&mut account_account, &mut delegate_account],
            )
        );
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
            Err(ProgramError::InvalidAccountData),
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
    fn test_set_authority_with_immutable_owner_extension() {
        let program_id = crate::id();
        let account_key = Pubkey::new_unique();

        let account_len =
            ExtensionType::try_calculate_account_len::<Account>(&[ExtensionType::ImmutableOwner])
                .unwrap();
        let mut account_account = SolanaAccount::new(
            Rent::default().minimum_balance(account_len),
            account_len,
            &program_id,
        );
        let owner_key = Pubkey::new_unique();
        let mut owner_account = SolanaAccount::default();
        let owner2_key = Pubkey::new_unique();

        let mint_key = Pubkey::new_unique();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
        let mut rent_sysvar = rent_sysvar();

        // create mint
        assert_eq!(
            Ok(()),
            do_process_instruction(
                initialize_mint(&program_id, &mint_key, &owner_key, None, 2).unwrap(),
                vec![&mut mint_account, &mut rent_sysvar],
            )
        );

        // create account
        assert_eq!(
            Ok(()),
            do_process_instruction(
                initialize_immutable_owner(&program_id, &account_key).unwrap(),
                vec![&mut account_account],
            )
        );
        assert_eq!(
            Ok(()),
            do_process_instruction(
                initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
                vec![
                    &mut account_account,
                    &mut mint_account,
                    &mut owner_account,
                    &mut rent_sysvar,
                ],
            )
        );

        // Immutable Owner extension blocks account owner authority changes
        assert_eq!(
            Err(TokenError::ImmutableOwner.into()),
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
            Err(TokenError::OwnerMismatch.into()),
            do_process_instruction(
                burn(
                    &program_id,
                    &account_key,
                    &mint_key,
                    &owner2_key, // <-- incorrect owner or delegate
                    &[],
                    1,
                )
                .unwrap(),
                vec![&mut account_account, &mut mint_account, &mut owner2_account],
            )
        );

        // insufficient funds approved via delegate
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            do_process_instruction(
                burn(&program_id, &account_key, &mint_key, &delegate_key, &[], 85).unwrap(),
                vec![
                    &mut account_account,
                    &mut mint_account,
                    &mut delegate_account
                ],
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
                burn(&program_id, &account_key, &mint_key, &delegate_key, &[], 1).unwrap(),
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
            #[allow(deprecated)]
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
            #[allow(deprecated)]
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
        let account_to_validate = Pubkey::new_unique();
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

        // no multisig, but the account is its own authority, and data is mutably
        // borrowed
        {
            let mut lamports = 0;
            let mut data = vec![0; Account::get_packed_len()];
            let mut account = Account::unpack_unchecked(&data).unwrap();
            account.owner = account_to_validate;
            Account::pack(account, &mut data).unwrap();
            let account_info = AccountInfo::new(
                &account_to_validate,
                true,
                false,
                &mut lamports,
                &mut data,
                &program_id,
                false,
                Epoch::default(),
            );
            let account_info_data_len = account_info.data_len();
            let mut borrowed_data = account_info.try_borrow_mut_data().unwrap();
            Processor::validate_owner(
                &program_id,
                &account_to_validate,
                &account_info,
                account_info_data_len,
                &[],
            )
            .unwrap();
            // modify the data to be sure that it wasn't silently dropped by the compiler
            borrowed_data[0] = 1;
        }

        // full 11 of 11
        Processor::validate_owner(
            &program_id,
            &owner_key,
            &owner_account_info,
            owner_account_info.data_len(),
            &signers,
        )
        .unwrap();

        // 1 of 11
        {
            let mut multisig =
                Multisig::unpack_unchecked(&owner_account_info.data.borrow()).unwrap();
            multisig.m = 1;
            Multisig::pack(multisig, &mut owner_account_info.data.borrow_mut()).unwrap();
        }
        Processor::validate_owner(
            &program_id,
            &owner_key,
            &owner_account_info,
            owner_account_info.data_len(),
            &signers,
        )
        .unwrap();

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
            Processor::validate_owner(
                &program_id,
                &owner_key,
                &owner_account_info,
                owner_account_info.data_len(),
                &signers
            )
        );

        // 0:11
        {
            let mut multisig =
                Multisig::unpack_unchecked(&owner_account_info.data.borrow()).unwrap();
            multisig.m = 0;
            multisig.n = 11;
            Multisig::pack(multisig, &mut owner_account_info.data.borrow_mut()).unwrap();
        }
        Processor::validate_owner(
            &program_id,
            &owner_key,
            &owner_account_info,
            owner_account_info.data_len(),
            &signers,
        )
        .unwrap();

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
            Processor::validate_owner(
                &program_id,
                &owner_key,
                &owner_account_info,
                owner_account_info.data_len(),
                &[]
            )
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
            Processor::validate_owner(
                &program_id,
                &owner_key,
                &owner_account_info,
                owner_account_info.data_len(),
                &signers[0..1]
            )
        );

        // 2:11, 2 from middle provided
        {
            let mut multisig =
                Multisig::unpack_unchecked(&owner_account_info.data.borrow()).unwrap();
            multisig.m = 2;
            multisig.n = 11;
            Multisig::pack(multisig, &mut owner_account_info.data.borrow_mut()).unwrap();
        }
        Processor::validate_owner(
            &program_id,
            &owner_key,
            &owner_account_info,
            owner_account_info.data_len(),
            &signers[5..7],
        )
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
            Processor::validate_owner(
                &program_id,
                &owner_key,
                &owner_account_info,
                owner_account_info.data_len(),
                &signers
            )
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
                Processor::validate_owner(
                    &program_id,
                    &owner_key,
                    &owner_account_info,
                    owner_account_info.data_len(),
                    &signers
                )
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
        let mut mint_account = native_mint();
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
                #[allow(deprecated)]
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
            #[allow(deprecated)]
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

        // attempt to mint one more to the other account
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
                #[allow(deprecated)]
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
                #[allow(deprecated)]
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
                #[allow(deprecated)]
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
    fn initialize_account_on_non_transferable_mint() {
        let program_id = crate::id();
        let account = Pubkey::new_unique();
        let account_len = ExtensionType::try_calculate_account_len::<Mint>(&[
            ExtensionType::NonTransferableAccount,
        ])
        .unwrap();
        let mut account_without_enough_length = SolanaAccount::new(
            Rent::default().minimum_balance(account_len),
            account_len,
            &program_id,
        );

        let account2 = Pubkey::new_unique();
        let account2_len = ExtensionType::try_calculate_account_len::<Mint>(&[
            ExtensionType::NonTransferableAccount,
            ExtensionType::ImmutableOwner,
        ])
        .unwrap();
        let mut account_with_enough_length = SolanaAccount::new(
            Rent::default().minimum_balance(account2_len),
            account2_len,
            &program_id,
        );

        let owner_key = Pubkey::new_unique();
        let mut owner_account = SolanaAccount::default();
        let mint_key = Pubkey::new_unique();
        let mint_len =
            ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::NonTransferable])
                .unwrap();
        let mut mint_account = SolanaAccount::new(
            Rent::default().minimum_balance(mint_len),
            mint_len,
            &program_id,
        );
        let mut rent_sysvar = rent_sysvar();

        // create a non-transferable mint
        assert_eq!(
            Ok(()),
            do_process_instruction(
                initialize_non_transferable_mint(&program_id, &mint_key).unwrap(),
                vec![&mut mint_account],
            )
        );
        assert_eq!(
            Ok(()),
            do_process_instruction(
                initialize_mint(&program_id, &mint_key, &owner_key, None, 2).unwrap(),
                vec![&mut mint_account, &mut rent_sysvar]
            )
        );

        //fail when account space is not enough for adding the immutable ownership
        // extension
        assert_eq!(
            Err(ProgramError::InvalidAccountData),
            do_process_instruction(
                initialize_account(&program_id, &account, &mint_key, &owner_key).unwrap(),
                vec![
                    &mut account_without_enough_length,
                    &mut mint_account,
                    &mut owner_account,
                    &mut rent_sysvar,
                ]
            )
        );

        //success to initialize an account with enough data space
        assert_eq!(
            Ok(()),
            do_process_instruction(
                initialize_account(&program_id, &account2, &mint_key, &owner_key).unwrap(),
                vec![
                    &mut account_with_enough_length,
                    &mut mint_account,
                    &mut owner_account,
                    &mut rent_sysvar,
                ]
            )
        );
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

    #[test]
    #[serial]
    fn test_get_account_data_size() {
        // see integration tests for return-data validity
        let program_id = crate::id();
        let owner_key = Pubkey::new_unique();
        let mut owner_account = SolanaAccount::default();
        let mut rent_sysvar = rent_sysvar();

        // Base mint
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
        let mint_key = Pubkey::new_unique();
        do_process_instruction(
            initialize_mint(&program_id, &mint_key, &owner_key, None, 2).unwrap(),
            vec![&mut mint_account, &mut rent_sysvar],
        )
        .unwrap();

        set_expected_data(
            ExtensionType::try_calculate_account_len::<Account>(&[])
                .unwrap()
                .to_le_bytes()
                .to_vec(),
        );
        do_process_instruction(
            get_account_data_size(&program_id, &mint_key, &[]).unwrap(),
            vec![&mut mint_account],
        )
        .unwrap();

        set_expected_data(
            ExtensionType::try_calculate_account_len::<Account>(&[
                ExtensionType::TransferFeeAmount,
            ])
            .unwrap()
            .to_le_bytes()
            .to_vec(),
        );
        do_process_instruction(
            get_account_data_size(
                &program_id,
                &mint_key,
                &[
                    ExtensionType::TransferFeeAmount,
                    ExtensionType::TransferFeeAmount, // Duplicate user input ignored...
                ],
            )
            .unwrap(),
            vec![&mut mint_account],
        )
        .unwrap();

        // Native mint
        let mut mint_account = native_mint();
        set_expected_data(
            ExtensionType::try_calculate_account_len::<Account>(&[])
                .unwrap()
                .to_le_bytes()
                .to_vec(),
        );
        do_process_instruction(
            get_account_data_size(&program_id, &mint_key, &[]).unwrap(),
            vec![&mut mint_account],
        )
        .unwrap();

        // Extended mint
        let mint_len =
            ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::TransferFeeConfig])
                .unwrap();
        let mut extended_mint_account = SolanaAccount::new(
            Rent::default().minimum_balance(mint_len),
            mint_len,
            &program_id,
        );
        let extended_mint_key = Pubkey::new_unique();
        do_process_instruction(
            initialize_transfer_fee_config(&program_id, &extended_mint_key, None, None, 10, 4242)
                .unwrap(),
            vec![&mut extended_mint_account],
        )
        .unwrap();
        do_process_instruction(
            initialize_mint(&program_id, &extended_mint_key, &owner_key, None, 2).unwrap(),
            vec![&mut extended_mint_account, &mut rent_sysvar],
        )
        .unwrap();

        set_expected_data(
            ExtensionType::try_calculate_account_len::<Account>(&[
                ExtensionType::TransferFeeAmount,
            ])
            .unwrap()
            .to_le_bytes()
            .to_vec(),
        );
        do_process_instruction(
            get_account_data_size(&program_id, &mint_key, &[]).unwrap(),
            vec![&mut extended_mint_account],
        )
        .unwrap();

        do_process_instruction(
            get_account_data_size(
                &program_id,
                &mint_key,
                // User extension that's also added by the mint ignored...
                &[ExtensionType::TransferFeeAmount],
            )
            .unwrap(),
            vec![&mut extended_mint_account],
        )
        .unwrap();

        // Invalid mint
        let mut invalid_mint_account = SolanaAccount::new(
            account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );
        let invalid_mint_key = Pubkey::new_unique();
        do_process_instruction(
            initialize_account(&program_id, &invalid_mint_key, &mint_key, &owner_key).unwrap(),
            vec![
                &mut invalid_mint_account,
                &mut mint_account,
                &mut owner_account,
                &mut rent_sysvar,
            ],
        )
        .unwrap();

        assert_eq!(
            do_process_instruction(
                get_account_data_size(&program_id, &invalid_mint_key, &[]).unwrap(),
                vec![&mut invalid_mint_account],
            ),
            Err(TokenError::InvalidMint.into())
        );

        // Invalid mint owner
        let invalid_program_id = Pubkey::new_unique();
        let mut invalid_mint_account = SolanaAccount::new(
            mint_minimum_balance(),
            Mint::get_packed_len(),
            &invalid_program_id,
        );
        let invalid_mint_key = Pubkey::new_unique();
        let mut instruction =
            initialize_mint(&program_id, &invalid_mint_key, &owner_key, None, 2).unwrap();
        instruction.program_id = invalid_program_id;
        do_process_instruction(
            instruction,
            vec![&mut invalid_mint_account, &mut rent_sysvar],
        )
        .unwrap();

        assert_eq!(
            do_process_instruction(
                get_account_data_size(&program_id, &invalid_mint_key, &[]).unwrap(),
                vec![&mut invalid_mint_account],
            ),
            Err(ProgramError::IncorrectProgramId)
        );

        // Invalid Extension Type for mint and uninitialized account
        assert_eq!(
            do_process_instruction(
                get_account_data_size(&program_id, &mint_key, &[ExtensionType::Uninitialized])
                    .unwrap(),
                vec![&mut mint_account],
            ),
            Err(TokenError::ExtensionTypeMismatch.into())
        );
        assert_eq!(
            do_process_instruction(
                get_account_data_size(
                    &program_id,
                    &mint_key,
                    &[
                        ExtensionType::MemoTransfer,
                        ExtensionType::MintCloseAuthority
                    ]
                )
                .unwrap(),
                vec![&mut mint_account],
            ),
            Err(TokenError::ExtensionTypeMismatch.into())
        );
    }

    #[test]
    #[serial]
    fn test_amount_to_ui_amount() {
        let program_id = crate::id();
        let owner_key = Pubkey::new_unique();
        let mint_key = Pubkey::new_unique();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
        let mut rent_sysvar = rent_sysvar();

        // fail if an invalid mint is passed in
        assert_eq!(
            Err(TokenError::InvalidMint.into()),
            do_process_instruction(
                amount_to_ui_amount(&program_id, &mint_key, 110).unwrap(),
                vec![&mut mint_account],
            )
        );

        // create mint
        do_process_instruction(
            initialize_mint(&program_id, &mint_key, &owner_key, None, 2).unwrap(),
            vec![&mut mint_account, &mut rent_sysvar],
        )
        .unwrap();

        set_expected_data("0.23".as_bytes().to_vec());
        do_process_instruction(
            amount_to_ui_amount(&program_id, &mint_key, 23).unwrap(),
            vec![&mut mint_account],
        )
        .unwrap();

        set_expected_data("1.1".as_bytes().to_vec());
        do_process_instruction(
            amount_to_ui_amount(&program_id, &mint_key, 110).unwrap(),
            vec![&mut mint_account],
        )
        .unwrap();

        set_expected_data("42".as_bytes().to_vec());
        do_process_instruction(
            amount_to_ui_amount(&program_id, &mint_key, 4200).unwrap(),
            vec![&mut mint_account],
        )
        .unwrap();

        set_expected_data("0".as_bytes().to_vec());
        do_process_instruction(
            amount_to_ui_amount(&program_id, &mint_key, 0).unwrap(),
            vec![&mut mint_account],
        )
        .unwrap();
    }

    #[test]
    #[serial]
    fn test_ui_amount_to_amount() {
        let program_id = crate::id();
        let owner_key = Pubkey::new_unique();
        let mint_key = Pubkey::new_unique();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);
        let mut rent_sysvar = rent_sysvar();

        // fail if an invalid mint is passed in
        assert_eq!(
            Err(TokenError::InvalidMint.into()),
            do_process_instruction(
                ui_amount_to_amount(&program_id, &mint_key, "1.1").unwrap(),
                vec![&mut mint_account],
            )
        );

        // create mint
        do_process_instruction(
            initialize_mint(&program_id, &mint_key, &owner_key, None, 2).unwrap(),
            vec![&mut mint_account, &mut rent_sysvar],
        )
        .unwrap();

        set_expected_data(23u64.to_le_bytes().to_vec());
        do_process_instruction(
            ui_amount_to_amount(&program_id, &mint_key, "0.23").unwrap(),
            vec![&mut mint_account],
        )
        .unwrap();

        set_expected_data(20u64.to_le_bytes().to_vec());
        do_process_instruction(
            ui_amount_to_amount(&program_id, &mint_key, "0.20").unwrap(),
            vec![&mut mint_account],
        )
        .unwrap();

        set_expected_data(20u64.to_le_bytes().to_vec());
        do_process_instruction(
            ui_amount_to_amount(&program_id, &mint_key, "0.2000").unwrap(),
            vec![&mut mint_account],
        )
        .unwrap();

        set_expected_data(20u64.to_le_bytes().to_vec());
        do_process_instruction(
            ui_amount_to_amount(&program_id, &mint_key, ".20").unwrap(),
            vec![&mut mint_account],
        )
        .unwrap();

        set_expected_data(110u64.to_le_bytes().to_vec());
        do_process_instruction(
            ui_amount_to_amount(&program_id, &mint_key, "1.1").unwrap(),
            vec![&mut mint_account],
        )
        .unwrap();

        set_expected_data(110u64.to_le_bytes().to_vec());
        do_process_instruction(
            ui_amount_to_amount(&program_id, &mint_key, "1.10").unwrap(),
            vec![&mut mint_account],
        )
        .unwrap();

        set_expected_data(4200u64.to_le_bytes().to_vec());
        do_process_instruction(
            ui_amount_to_amount(&program_id, &mint_key, "42").unwrap(),
            vec![&mut mint_account],
        )
        .unwrap();

        set_expected_data(4200u64.to_le_bytes().to_vec());
        do_process_instruction(
            ui_amount_to_amount(&program_id, &mint_key, "42.").unwrap(),
            vec![&mut mint_account],
        )
        .unwrap();

        set_expected_data(0u64.to_le_bytes().to_vec());
        do_process_instruction(
            ui_amount_to_amount(&program_id, &mint_key, "0").unwrap(),
            vec![&mut mint_account],
        )
        .unwrap();

        // fail if invalid ui_amount passed in
        assert_eq!(
            Err(ProgramError::InvalidArgument),
            do_process_instruction(
                ui_amount_to_amount(&program_id, &mint_key, "").unwrap(),
                vec![&mut mint_account],
            )
        );
        assert_eq!(
            Err(ProgramError::InvalidArgument),
            do_process_instruction(
                ui_amount_to_amount(&program_id, &mint_key, ".").unwrap(),
                vec![&mut mint_account],
            )
        );
        assert_eq!(
            Err(ProgramError::InvalidArgument),
            do_process_instruction(
                ui_amount_to_amount(&program_id, &mint_key, "0.111").unwrap(),
                vec![&mut mint_account],
            )
        );
        assert_eq!(
            Err(ProgramError::InvalidArgument),
            do_process_instruction(
                ui_amount_to_amount(&program_id, &mint_key, "0.t").unwrap(),
                vec![&mut mint_account],
            )
        );
    }

    #[test]
    #[serial]
    fn test_withdraw_excess_lamports_from_multisig() {
        {
            use std::sync::Once;
            static ONCE: Once = Once::new();

            ONCE.call_once(|| {
                solana_sdk::program_stubs::set_syscall_stubs(Box::new(SyscallStubs {}));
            });
        }
        let program_id = crate::id();

        let mut lamports = 0;
        let mut destination_data = vec![];
        let system_program_id = system_program::id();
        let destination_key = Pubkey::new_unique();
        let destination_info = AccountInfo::new(
            &destination_key,
            true,
            false,
            &mut lamports,
            &mut destination_data,
            &system_program_id,
            false,
            Epoch::default(),
        );

        let multisig_key = Pubkey::new_unique();
        let mut multisig_account = SolanaAccount::new(0, Multisig::get_packed_len(), &program_id);
        let excess_lamports = 4_000_000_000_000;
        multisig_account.lamports = excess_lamports + multisig_minimum_balance();
        let mut signer_keys = [Pubkey::default(); MAX_SIGNERS];

        for signer_key in signer_keys.iter_mut().take(MAX_SIGNERS) {
            *signer_key = Pubkey::new_unique();
        }
        let signer_refs: Vec<&Pubkey> = signer_keys.iter().collect();
        let mut signer_lamports = 0;
        let mut signer_data = vec![];
        let mut signers: Vec<AccountInfo<'_>> = vec![
            AccountInfo::new(
                &destination_key,
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

        let mut multisig =
            Multisig::unpack_unchecked(&vec![0; Multisig::get_packed_len()]).unwrap();
        multisig.m = MAX_SIGNERS as u8;
        multisig.n = MAX_SIGNERS as u8;
        multisig.signers = signer_keys;
        multisig.is_initialized = true;
        Multisig::pack(multisig, &mut multisig_account.data).unwrap();

        let multisig_info: AccountInfo = (&multisig_key, true, &mut multisig_account).into();

        let mut signers_infos = vec![
            multisig_info.clone(),
            destination_info.clone(),
            multisig_info.clone(),
        ];
        signers_infos.extend(signers);
        do_process_instruction_dups(
            withdraw_excess_lamports(
                &program_id,
                &multisig_key,
                &destination_key,
                &multisig_key,
                &signer_refs,
            )
            .unwrap(),
            signers_infos,
        )
        .unwrap();

        assert_eq!(destination_info.lamports(), excess_lamports);
    }

    #[test]
    #[serial]
    fn test_withdraw_excess_lamports_from_account() {
        let excess_lamports = 4_000_000_000_000;

        let program_id = crate::id();
        let account_key = Pubkey::new_unique();
        let mut account_account = SolanaAccount::new(
            excess_lamports + account_minimum_balance(),
            Account::get_packed_len(),
            &program_id,
        );

        let system_program_id = system_program::id();
        let owner_key = Pubkey::new_unique();

        let mut destination_lamports = 0;
        let mut destination_data = vec![];
        let destination_key = Pubkey::new_unique();
        let destination_info = AccountInfo::new(
            &destination_key,
            true,
            false,
            &mut destination_lamports,
            &mut destination_data,
            &system_program_id,
            false,
            Epoch::default(),
        );
        let mint_key = Pubkey::new_unique();
        let mut mint_account =
            SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &program_id);

        let mut rent_sysvar = rent_sysvar();
        do_process_instruction(
            initialize_mint(&program_id, &mint_key, &owner_key, None, 2).unwrap(),
            vec![&mut mint_account, &mut rent_sysvar],
        )
        .unwrap();

        let mint_info = AccountInfo::new(
            &mint_key,
            true,
            false,
            &mut mint_account.lamports,
            &mut mint_account.data,
            &program_id,
            false,
            Epoch::default(),
        );

        let account_info: AccountInfo = (&account_key, true, &mut account_account).into();

        do_process_instruction_dups(
            initialize_account3(&program_id, &account_key, &mint_key, &account_key).unwrap(),
            vec![account_info.clone(), mint_info.clone()],
        )
        .unwrap();

        do_process_instruction_dups(
            withdraw_excess_lamports(
                &program_id,
                &account_key,
                &destination_key,
                &account_key,
                &[],
            )
            .unwrap(),
            vec![
                account_info.clone(),
                destination_info.clone(),
                account_info.clone(),
            ],
        )
        .unwrap();

        assert_eq!(destination_info.lamports(), excess_lamports);
    }

    #[test]
    #[serial]
    fn test_withdraw_excess_lamports_from_mint() {
        let excess_lamports = 4_000_000_000_000;

        let program_id = crate::id();
        let system_program_id = system_program::id();

        let mut destination_lamports = 0;
        let mut destination_data = vec![];
        let destination_key = Pubkey::new_unique();
        let destination_info = AccountInfo::new(
            &destination_key,
            true,
            false,
            &mut destination_lamports,
            &mut destination_data,
            &system_program_id,
            false,
            Epoch::default(),
        );
        let mint_key = Pubkey::new_unique();
        let mut mint_account = SolanaAccount::new(
            excess_lamports + mint_minimum_balance(),
            Mint::get_packed_len(),
            &program_id,
        );
        let mut rent_sysvar = rent_sysvar();

        do_process_instruction(
            initialize_mint(&program_id, &mint_key, &mint_key, None, 2).unwrap(),
            vec![&mut mint_account, &mut rent_sysvar],
        )
        .unwrap();

        let mint_info: AccountInfo = (&mint_key, true, &mut mint_account).into();

        do_process_instruction_dups(
            withdraw_excess_lamports(&program_id, &mint_key, &destination_key, &mint_key, &[])
                .unwrap(),
            vec![
                mint_info.clone(),
                destination_info.clone(),
                mint_info.clone(),
            ],
        )
        .unwrap();

        assert_eq!(destination_info.lamports(), excess_lamports);
    }
}
