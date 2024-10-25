// Remove feature once zk ops syscalls are enabled on all networks
#[cfg(feature = "zk-ops")]
use {
    crate::extension::non_transferable::NonTransferableAccount,
    spl_token_confidential_transfer_ciphertext_arithmetic as ciphertext_arithmetic,
};
use {
    crate::{
        check_elgamal_registry_program_account, check_program_account,
        error::TokenError,
        extension::{
            confidential_transfer::{instruction::*, verify_proof::*, *},
            confidential_transfer_fee::{
                ConfidentialTransferFeeAmount, ConfidentialTransferFeeConfig,
                EncryptedWithheldAmount,
            },
            memo_transfer::{check_previous_sibling_instruction_is_memo, memo_required},
            set_account_type,
            transfer_fee::TransferFeeConfig,
            transfer_hook, BaseStateWithExtensions, BaseStateWithExtensionsMut,
            PodStateWithExtensions, PodStateWithExtensionsMut,
        },
        instruction::{decode_instruction_data, decode_instruction_type},
        pod::{PodAccount, PodMint},
        processor::Processor,
        state::Account,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        clock::Clock,
        entrypoint::ProgramResult,
        msg,
        program::invoke,
        program_error::ProgramError,
        pubkey::Pubkey,
        system_instruction,
        sysvar::{rent::Rent, Sysvar},
    },
    spl_elgamal_registry::state::ElGamalRegistry,
    spl_pod::bytemuck::pod_from_bytes,
    spl_token_confidential_transfer_proof_extraction::{
        instruction::verify_and_extract_context, transfer::TransferProofContext,
        transfer_with_fee::TransferWithFeeProofContext,
    },
};

/// Processes an [InitializeMint] instruction.
fn process_initialize_mint(
    accounts: &[AccountInfo],
    authority: &OptionalNonZeroPubkey,
    auto_approve_new_account: PodBool,
    auditor_encryption_pubkey: &OptionalNonZeroElGamalPubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_info = next_account_info(account_info_iter)?;

    check_program_account(mint_info.owner)?;
    let mint_data = &mut mint_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack_uninitialized(mint_data)?;
    let confidential_transfer_mint = mint.init_extension::<ConfidentialTransferMint>(true)?;

    confidential_transfer_mint.authority = *authority;
    confidential_transfer_mint.auto_approve_new_accounts = auto_approve_new_account;
    confidential_transfer_mint.auditor_elgamal_pubkey = *auditor_encryption_pubkey;

    Ok(())
}

/// Processes an [UpdateMint] instruction.
fn process_update_mint(
    accounts: &[AccountInfo],
    auto_approve_new_account: PodBool,
    auditor_encryption_pubkey: &OptionalNonZeroElGamalPubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;

    check_program_account(mint_info.owner)?;
    let mint_data = &mut mint_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack(mint_data)?;
    let confidential_transfer_mint = mint.get_extension_mut::<ConfidentialTransferMint>()?;
    let maybe_confidential_transfer_mint_authority: Option<Pubkey> =
        confidential_transfer_mint.authority.into();
    let confidential_transfer_mint_authority =
        maybe_confidential_transfer_mint_authority.ok_or(TokenError::NoAuthorityExists)?;

    if !authority_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if confidential_transfer_mint_authority != *authority_info.key {
        return Err(TokenError::OwnerMismatch.into());
    }

    confidential_transfer_mint.auto_approve_new_accounts = auto_approve_new_account;
    confidential_transfer_mint.auditor_elgamal_pubkey = *auditor_encryption_pubkey;
    Ok(())
}

enum ElGamalPubkeySource<'a> {
    ProofInstructionOffset(i64),
    ElGamalRegistry(&'a ElGamalRegistry),
}

/// Processes a [ConfigureAccountWithRegistry] instruction.
fn process_configure_account_with_registry(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let token_account_info = next_account_info(account_info_iter)?;
    let _mint_info = next_account_info(account_info_iter)?;
    let elgamal_registry_account = next_account_info(account_info_iter)?;

    check_elgamal_registry_program_account(elgamal_registry_account.owner)?;

    // if a payer account for reallcation is provided, then reallocate
    if let Ok(payer_info) = next_account_info(account_info_iter) {
        let system_program_info = next_account_info(account_info_iter)?;
        reallocate_for_configure_account_with_registry(
            token_account_info,
            payer_info,
            system_program_info,
        )?;
    }

    let elgamal_registry_account_data = &elgamal_registry_account.data.borrow();
    let elgamal_registry_account =
        pod_from_bytes::<ElGamalRegistry>(elgamal_registry_account_data)?;

    let decryptable_zero_balance = PodAeCiphertext::default();
    let maximum_pending_balance_credit_counter =
        DEFAULT_MAXIMUM_PENDING_BALANCE_CREDIT_COUNTER.into();

    process_configure_account(
        program_id,
        accounts,
        &decryptable_zero_balance,
        &maximum_pending_balance_credit_counter,
        ElGamalPubkeySource::ElGamalRegistry(elgamal_registry_account),
    )
}

fn reallocate_for_configure_account_with_registry<'a>(
    token_account_info: &AccountInfo<'a>,
    payer_info: &AccountInfo<'a>,
    system_program_info: &AccountInfo<'a>,
) -> ProgramResult {
    let mut current_extension_types = {
        let token_account = token_account_info.data.borrow();
        let account = PodStateWithExtensions::<PodAccount>::unpack(&token_account)?;
        account.get_extension_types()?
    };
    // `try_calculate_account_len` dedupes extension types, so always push
    // the `ConfidentialTransferAccount` type
    current_extension_types.push(ExtensionType::ConfidentialTransferAccount);
    let needed_account_len =
        ExtensionType::try_calculate_account_len::<Account>(&current_extension_types)?;

    // if account is already large enough, return early
    if token_account_info.data_len() >= needed_account_len {
        return Ok(());
    }

    // reallocate
    msg!(
        "account needs realloc, +{:?} bytes",
        needed_account_len - token_account_info.data_len()
    );
    token_account_info.realloc(needed_account_len, false)?;

    // if additional lamports needed to remain rent-exempt, transfer them
    let rent = Rent::get()?;
    let new_rent_exempt_reserve = rent.minimum_balance(needed_account_len);

    let current_lamport_reserve = token_account_info.lamports();
    let lamports_diff = new_rent_exempt_reserve.saturating_sub(current_lamport_reserve);
    if lamports_diff > 0 {
        invoke(
            &system_instruction::transfer(payer_info.key, token_account_info.key, lamports_diff),
            &[
                payer_info.clone(),
                token_account_info.clone(),
                system_program_info.clone(),
            ],
        )?;
    }

    // set account_type, if needed
    let mut token_account_data = token_account_info.data.borrow_mut();
    set_account_type::<Account>(&mut token_account_data)?;

    Ok(())
}

/// Processes a [ConfigureAccount] instruction.
fn process_configure_account(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    decryptable_zero_balance: &DecryptableBalance,
    maximum_pending_balance_credit_counter: &PodU64,
    elgamal_pubkey_source: ElGamalPubkeySource,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let token_account_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;

    let elgamal_pubkey = match elgamal_pubkey_source {
        ElGamalPubkeySource::ProofInstructionOffset(offset) => {
            // zero-knowledge proof certifies that the supplied ElGamal public key is valid
            let proof_context = verify_and_extract_context::<
                PubkeyValidityProofData,
                PubkeyValidityProofContext,
            >(account_info_iter, offset, None)?;
            proof_context.pubkey
        }
        ElGamalPubkeySource::ElGamalRegistry(elgamal_registry_account) => {
            let _elgamal_registry_account = next_account_info(account_info_iter)?;
            elgamal_registry_account.elgamal_pubkey
        }
    };

    check_program_account(token_account_info.owner)?;
    let token_account_data = &mut token_account_info.data.borrow_mut();
    let mut token_account = PodStateWithExtensionsMut::<PodAccount>::unpack(token_account_data)?;

    if token_account.base.mint != *mint_info.key {
        return Err(TokenError::MintMismatch.into());
    }

    match elgamal_pubkey_source {
        ElGamalPubkeySource::ProofInstructionOffset(_) => {
            let authority_info = next_account_info(account_info_iter)?;
            let authority_info_data_len = authority_info.data_len();

            Processor::validate_owner(
                program_id,
                &token_account.base.owner,
                authority_info,
                authority_info_data_len,
                account_info_iter.as_slice(),
            )?;
        }
        ElGamalPubkeySource::ElGamalRegistry(elgamal_registry_account) => {
            // if ElGamal registry was provided, then just verify that the owners of the
            // registry and token accounts match, then skip the signature
            // verification check
            if elgamal_registry_account.owner != token_account.base.owner {
                return Err(TokenError::OwnerMismatch.into());
            }
        }
    };

    check_program_account(mint_info.owner)?;
    let mint_data = &mut mint_info.data.borrow();
    let mint = PodStateWithExtensions::<PodMint>::unpack(mint_data)?;
    let confidential_transfer_mint = mint.get_extension::<ConfidentialTransferMint>()?;

    // Note: The caller is expected to use the `Reallocate` instruction to ensure
    // there is sufficient room in their token account for the new
    // `ConfidentialTransferAccount` extension
    let confidential_transfer_account =
        token_account.init_extension::<ConfidentialTransferAccount>(false)?;
    confidential_transfer_account.approved = confidential_transfer_mint.auto_approve_new_accounts;
    confidential_transfer_account.elgamal_pubkey = elgamal_pubkey;
    confidential_transfer_account.maximum_pending_balance_credit_counter =
        *maximum_pending_balance_credit_counter;

    // The all-zero ciphertext [0; 64] is a valid encryption of zero
    confidential_transfer_account.pending_balance_lo = EncryptedBalance::zeroed();
    confidential_transfer_account.pending_balance_hi = EncryptedBalance::zeroed();
    confidential_transfer_account.available_balance = EncryptedBalance::zeroed();

    confidential_transfer_account.decryptable_available_balance = *decryptable_zero_balance;
    confidential_transfer_account.allow_confidential_credits = true.into();
    confidential_transfer_account.pending_balance_credit_counter = 0.into();
    confidential_transfer_account.expected_pending_balance_credit_counter = 0.into();
    confidential_transfer_account.actual_pending_balance_credit_counter = 0.into();
    confidential_transfer_account.allow_non_confidential_credits = true.into();

    // if the mint is extended for fees, then initialize account for confidential
    // transfer fees
    if mint.get_extension::<TransferFeeConfig>().is_ok() {
        let confidential_transfer_fee_amount =
            token_account.init_extension::<ConfidentialTransferFeeAmount>(false)?;
        confidential_transfer_fee_amount.withheld_amount = EncryptedWithheldAmount::zeroed();
    }

    Ok(())
}

/// Processes an [ApproveAccount] instruction.
fn process_approve_account(accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let token_account_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;

    check_program_account(token_account_info.owner)?;
    let token_account_data = &mut token_account_info.data.borrow_mut();
    let mut token_account = PodStateWithExtensionsMut::<PodAccount>::unpack(token_account_data)?;

    if *mint_info.key != token_account.base.mint {
        return Err(TokenError::MintMismatch.into());
    }

    check_program_account(mint_info.owner)?;
    let mint_data = &mint_info.data.borrow_mut();
    let mint = PodStateWithExtensions::<PodMint>::unpack(mint_data)?;
    let confidential_transfer_mint = mint.get_extension::<ConfidentialTransferMint>()?;
    let maybe_confidential_transfer_mint_authority: Option<Pubkey> =
        confidential_transfer_mint.authority.into();
    let confidential_transfer_mint_authority =
        maybe_confidential_transfer_mint_authority.ok_or(TokenError::NoAuthorityExists)?;

    if authority_info.is_signer && *authority_info.key == confidential_transfer_mint_authority {
        let confidential_transfer_state =
            token_account.get_extension_mut::<ConfidentialTransferAccount>()?;
        confidential_transfer_state.approved = true.into();
        Ok(())
    } else {
        Err(ProgramError::MissingRequiredSignature)
    }
}

/// Processes an [EmptyAccount] instruction.
fn process_empty_account(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    proof_instruction_offset: i64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let token_account_info = next_account_info(account_info_iter)?;

    // zero-knowledge proof certifies that the available balance ciphertext holds
    // the balance of 0.
    let proof_context = verify_and_extract_context::<
        ZeroCiphertextProofData,
        ZeroCiphertextProofContext,
    >(account_info_iter, proof_instruction_offset, None)?;

    let authority_info = next_account_info(account_info_iter)?;
    let authority_info_data_len = authority_info.data_len();

    check_program_account(token_account_info.owner)?;
    let token_account_data = &mut token_account_info.data.borrow_mut();
    let mut token_account = PodStateWithExtensionsMut::<PodAccount>::unpack(token_account_data)?;

    Processor::validate_owner(
        program_id,
        &token_account.base.owner,
        authority_info,
        authority_info_data_len,
        account_info_iter.as_slice(),
    )?;

    let confidential_transfer_account =
        token_account.get_extension_mut::<ConfidentialTransferAccount>()?;

    // Check that the encryption public key and ciphertext associated with the
    // confidential extension account are consistent with those that were
    // actually used to generate the zkp.
    if confidential_transfer_account.elgamal_pubkey != proof_context.pubkey {
        msg!("Encryption public-key mismatch");
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }
    if confidential_transfer_account.available_balance != proof_context.ciphertext {
        msg!("Available balance mismatch");
        return Err(ProgramError::InvalidInstructionData);
    }
    confidential_transfer_account.available_balance = EncryptedBalance::zeroed();

    // check that all balances are all-zero ciphertexts
    confidential_transfer_account.closable()?;

    Ok(())
}

/// Processes a [Deposit] instruction.
#[cfg(feature = "zk-ops")]
fn process_deposit(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
    expected_decimals: u8,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let token_account_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let authority_info_data_len = authority_info.data_len();

    check_program_account(mint_info.owner)?;
    let mint_data = &mint_info.data.borrow_mut();
    let mint = PodStateWithExtensions::<PodMint>::unpack(mint_data)?;

    if expected_decimals != mint.base.decimals {
        return Err(TokenError::MintDecimalsMismatch.into());
    }

    check_program_account(token_account_info.owner)?;
    let token_account_data = &mut token_account_info.data.borrow_mut();
    let mut token_account = PodStateWithExtensionsMut::<PodAccount>::unpack(token_account_data)?;
    if token_account
        .get_extension::<NonTransferableAccount>()
        .is_ok()
    {
        return Err(TokenError::NonTransferable.into());
    }

    Processor::validate_owner(
        program_id,
        &token_account.base.owner,
        authority_info,
        authority_info_data_len,
        account_info_iter.as_slice(),
    )?;

    if token_account.base.is_frozen() {
        return Err(TokenError::AccountFrozen.into());
    }

    if token_account.base.mint != *mint_info.key {
        return Err(TokenError::MintMismatch.into());
    }

    // Wrapped SOL deposits are not supported because lamports cannot be vanished.
    assert!(!token_account.base.is_native());

    token_account.base.amount = u64::from(token_account.base.amount)
        .checked_sub(amount)
        .ok_or(TokenError::Overflow)?
        .into();

    let confidential_transfer_account =
        token_account.get_extension_mut::<ConfidentialTransferAccount>()?;
    confidential_transfer_account.valid_as_destination()?;

    // A deposit amount must be a 48-bit number
    let (amount_lo, amount_hi) = verify_and_split_deposit_amount(amount)?;

    // Prevent unnecessary ciphertext arithmetic syscalls if `amount_lo` or
    // `amount_hi` is zero
    if amount_lo > 0 {
        confidential_transfer_account.pending_balance_lo = ciphertext_arithmetic::add_to(
            &confidential_transfer_account.pending_balance_lo,
            amount_lo,
        )
        .ok_or(TokenError::CiphertextArithmeticFailed)?;
    }
    if amount_hi > 0 {
        confidential_transfer_account.pending_balance_hi = ciphertext_arithmetic::add_to(
            &confidential_transfer_account.pending_balance_hi,
            amount_hi,
        )
        .ok_or(TokenError::CiphertextArithmeticFailed)?;
    }

    confidential_transfer_account.increment_pending_balance_credit_counter()?;

    Ok(())
}

/// Verifies that a deposit amount is a 48-bit number and returns the least
/// significant 16 bits and most significant 32 bits of the amount.
#[cfg(feature = "zk-ops")]
pub fn verify_and_split_deposit_amount(amount: u64) -> Result<(u64, u64), TokenError> {
    if amount > MAXIMUM_DEPOSIT_TRANSFER_AMOUNT {
        return Err(TokenError::MaximumDepositAmountExceeded);
    }
    let deposit_amount_lo = amount & (u16::MAX as u64);
    let deposit_amount_hi = amount.checked_shr(u16::BITS).unwrap();
    Ok((deposit_amount_lo, deposit_amount_hi))
}

/// Processes a [Withdraw] instruction.
#[cfg(feature = "zk-ops")]
fn process_withdraw(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
    expected_decimals: u8,
    new_decryptable_available_balance: DecryptableBalance,
    equality_proof_instruction_offset: i64,
    range_proof_instruction_offset: i64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let token_account_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;

    // zero-knowledge proof certifies that the account has enough available balance
    // to withdraw the amount.
    let proof_context = verify_withdraw_proof(
        account_info_iter,
        equality_proof_instruction_offset,
        range_proof_instruction_offset,
    )?;

    let authority_info = next_account_info(account_info_iter)?;
    let authority_info_data_len = authority_info.data_len();

    check_program_account(mint_info.owner)?;
    let mint_data = &mint_info.data.borrow_mut();
    let mint = PodStateWithExtensions::<PodMint>::unpack(mint_data)?;

    if expected_decimals != mint.base.decimals {
        return Err(TokenError::MintDecimalsMismatch.into());
    }

    check_program_account(token_account_info.owner)?;
    let token_account_data = &mut token_account_info.data.borrow_mut();
    let mut token_account = PodStateWithExtensionsMut::<PodAccount>::unpack(token_account_data)?;
    if token_account
        .get_extension::<NonTransferableAccount>()
        .is_ok()
    {
        return Err(TokenError::NonTransferable.into());
    }

    Processor::validate_owner(
        program_id,
        &token_account.base.owner,
        authority_info,
        authority_info_data_len,
        account_info_iter.as_slice(),
    )?;

    if token_account.base.is_frozen() {
        return Err(TokenError::AccountFrozen.into());
    }

    if token_account.base.mint != *mint_info.key {
        return Err(TokenError::MintMismatch.into());
    }

    // Wrapped SOL withdrawals are not supported because lamports cannot be
    // apparated.
    assert!(!token_account.base.is_native());

    let confidential_transfer_account =
        token_account.get_extension_mut::<ConfidentialTransferAccount>()?;
    confidential_transfer_account.valid_as_source()?;

    // Check that the encryption public key associated with the confidential
    // extension is consistent with the public key that was actually used to
    // generate the zkp.
    if confidential_transfer_account.elgamal_pubkey != proof_context.source_pubkey {
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }

    // Prevent unnecessary ciphertext arithmetic syscalls if the withdraw amount is
    // zero
    if amount > 0 {
        confidential_transfer_account.available_balance = ciphertext_arithmetic::subtract_from(
            &confidential_transfer_account.available_balance,
            amount,
        )
        .ok_or(TokenError::CiphertextArithmeticFailed)?;
    }
    // Check that the final available balance ciphertext is consistent with the
    // actual ciphertext for which the zero-knowledge proof was generated for.
    if confidential_transfer_account.available_balance != proof_context.remaining_balance_ciphertext
    {
        return Err(TokenError::ConfidentialTransferBalanceMismatch.into());
    }

    confidential_transfer_account.decryptable_available_balance = new_decryptable_available_balance;
    token_account.base.amount = u64::from(token_account.base.amount)
        .checked_add(amount)
        .ok_or(TokenError::Overflow)?
        .into();

    Ok(())
}

/// Processes a [Transfer] or [TransferWithFee] instruction.
#[allow(clippy::too_many_arguments)]
#[cfg(feature = "zk-ops")]
fn process_transfer(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_source_decryptable_available_balance: DecryptableBalance,
    equality_proof_instruction_offset: i64,
    transfer_amount_ciphertext_validity_proof_instruction_offset: i64,
    fee_sigma_proof_instruction_offset: Option<i64>,
    fee_ciphertext_validity_proof_instruction_offset: Option<i64>,
    range_proof_instruction_offset: i64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let source_account_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let destination_account_info = next_account_info(account_info_iter)?;

    check_program_account(mint_info.owner)?;
    let mint_data = mint_info.data.borrow_mut();
    let mint = PodStateWithExtensions::<PodMint>::unpack(&mint_data)?;

    let confidential_transfer_mint = mint.get_extension::<ConfidentialTransferMint>()?;

    // A `Transfer` instruction must be accompanied by a zero-knowledge proof
    // instruction that certify the validity of the transfer amounts. The kind
    // of zero-knowledge proof instruction depends on whether a transfer incurs
    // a fee or not.
    //   - If the mint is not extended for fees or the instruction is for a
    //     self-transfer, then
    //   transfer fee is not required.
    //   - If the mint is extended for fees and the instruction is not a
    //     self-transfer, then
    //   transfer fee is required.
    let authority_info = if mint.get_extension::<TransferFeeConfig>().is_err() {
        // Transfer fee is not required. Decode the zero-knowledge proof as
        // `TransferContext`.
        //
        // The zero-knowledge proof certifies that:
        //   1. the transfer amount is encrypted in the correct form
        //   2. the source account has enough balance to send the transfer amount
        let proof_context = verify_transfer_proof(
            account_info_iter,
            equality_proof_instruction_offset,
            transfer_amount_ciphertext_validity_proof_instruction_offset,
            range_proof_instruction_offset,
        )?;

        let authority_info = next_account_info(account_info_iter)?;

        // Check that the auditor encryption public key associated wth the confidential
        // mint is consistent with what was actually used to generate the zkp.
        if !confidential_transfer_mint
            .auditor_elgamal_pubkey
            .equals(&proof_context.transfer_pubkeys.auditor)
        {
            return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
        }

        process_source_for_transfer(
            program_id,
            source_account_info,
            mint_info,
            authority_info,
            account_info_iter.as_slice(),
            &proof_context,
            new_source_decryptable_available_balance,
        )?;

        process_destination_for_transfer(destination_account_info, mint_info, &proof_context)?;

        authority_info
    } else {
        // Transfer fee is required.
        let transfer_fee_config = mint.get_extension::<TransferFeeConfig>()?;
        let fee_parameters = transfer_fee_config.get_epoch_fee(Clock::get()?.epoch);

        let fee_sigma_proof_insruction_offset =
            fee_sigma_proof_instruction_offset.ok_or(ProgramError::InvalidInstructionData)?;
        let fee_ciphertext_validity_proof_insruction_offset =
            fee_ciphertext_validity_proof_instruction_offset
                .ok_or(ProgramError::InvalidInstructionData)?;

        // Decode the zero-knowledge proof as `TransferWithFeeContext`.
        //
        // The zero-knowledge proof certifies that:
        //   1. the transfer amount is encrypted in the correct form
        //   2. the source account has enough balance to send the transfer amount
        //   3. the transfer fee is computed correctly and encrypted in the correct form
        let proof_context = verify_transfer_with_fee_proof(
            account_info_iter,
            equality_proof_instruction_offset,
            transfer_amount_ciphertext_validity_proof_instruction_offset,
            fee_sigma_proof_insruction_offset,
            fee_ciphertext_validity_proof_insruction_offset,
            range_proof_instruction_offset,
            fee_parameters,
        )?;

        let authority_info = next_account_info(account_info_iter)?;

        // Check that the encryption public keys associated with the mint confidential
        // transfer and confidential transfer fee extensions are consistent with
        // the keys that were used to generate the zkp.
        if !confidential_transfer_mint
            .auditor_elgamal_pubkey
            .equals(&proof_context.transfer_with_fee_pubkeys.auditor)
        {
            return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
        }

        let confidential_transfer_fee_config =
            mint.get_extension::<ConfidentialTransferFeeConfig>()?;

        // Check that the withdraw withheld authority ElGamal public key in the mint is
        // consistent with what was used to generate the zkp.
        if proof_context
            .transfer_with_fee_pubkeys
            .withdraw_withheld_authority
            != confidential_transfer_fee_config.withdraw_withheld_authority_elgamal_pubkey
        {
            return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
        }

        process_source_for_transfer_with_fee(
            program_id,
            source_account_info,
            mint_info,
            authority_info,
            account_info_iter.as_slice(),
            &proof_context,
            new_source_decryptable_available_balance,
        )?;

        let is_self_transfer = source_account_info.key == destination_account_info.key;
        process_destination_for_transfer_with_fee(
            destination_account_info,
            mint_info,
            &proof_context,
            is_self_transfer,
        )?;

        authority_info
    };

    if let Some(program_id) = transfer_hook::get_program_id(&mint) {
        // set transferring flags, scope the borrow to avoid double-borrow during CPI
        {
            let mut source_account_data = source_account_info.data.borrow_mut();
            let mut source_account =
                PodStateWithExtensionsMut::<PodAccount>::unpack(&mut source_account_data)?;
            transfer_hook::set_transferring(&mut source_account)?;
        }
        {
            let mut destination_account_data = destination_account_info.data.borrow_mut();
            let mut destination_account =
                PodStateWithExtensionsMut::<PodAccount>::unpack(&mut destination_account_data)?;
            transfer_hook::set_transferring(&mut destination_account)?;
        }

        // can't doubly-borrow the mint data either
        drop(mint_data);

        // Since the amount is unknown during a confidential transfer, pass in
        // u64::MAX as a convention.
        spl_transfer_hook_interface::onchain::invoke_execute(
            &program_id,
            source_account_info.clone(),
            mint_info.clone(),
            destination_account_info.clone(),
            authority_info.clone(),
            account_info_iter.as_slice(),
            u64::MAX,
        )?;

        // unset transferring flag
        transfer_hook::unset_transferring(source_account_info)?;
        transfer_hook::unset_transferring(destination_account_info)?;
    }

    Ok(())
}

#[cfg(feature = "zk-ops")]
fn process_source_for_transfer(
    program_id: &Pubkey,
    source_account_info: &AccountInfo,
    mint_info: &AccountInfo,
    authority_info: &AccountInfo,
    signers: &[AccountInfo],
    proof_context: &TransferProofContext,
    new_source_decryptable_available_balance: DecryptableBalance,
) -> ProgramResult {
    check_program_account(source_account_info.owner)?;
    let authority_info_data_len = authority_info.data_len();
    let token_account_data = &mut source_account_info.data.borrow_mut();
    let mut token_account = PodStateWithExtensionsMut::<PodAccount>::unpack(token_account_data)?;
    if token_account
        .get_extension::<NonTransferableAccount>()
        .is_ok()
    {
        return Err(TokenError::NonTransferable.into());
    }

    Processor::validate_owner(
        program_id,
        &token_account.base.owner,
        authority_info,
        authority_info_data_len,
        signers,
    )?;

    if token_account.base.is_frozen() {
        return Err(TokenError::AccountFrozen.into());
    }

    if token_account.base.mint != *mint_info.key {
        return Err(TokenError::MintMismatch.into());
    }

    let confidential_transfer_account =
        token_account.get_extension_mut::<ConfidentialTransferAccount>()?;
    confidential_transfer_account.valid_as_source()?;

    // Check that the source encryption public key is consistent with what was
    // actually used to generate the zkp.
    if proof_context.transfer_pubkeys.source != confidential_transfer_account.elgamal_pubkey {
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }

    let source_transfer_amount_lo = proof_context
        .ciphertext_lo
        .try_extract_ciphertext(0)
        .map_err(|e| -> TokenError { e.into() })?;
    let source_transfer_amount_hi = proof_context
        .ciphertext_hi
        .try_extract_ciphertext(0)
        .map_err(|e| -> TokenError { e.into() })?;

    let new_source_available_balance = ciphertext_arithmetic::subtract_with_lo_hi(
        &confidential_transfer_account.available_balance,
        &source_transfer_amount_lo,
        &source_transfer_amount_hi,
    )
    .ok_or(TokenError::CiphertextArithmeticFailed)?;

    // Check that the computed available balance is consistent with what was
    // actually used to generate the zkp on the client side.
    if new_source_available_balance != proof_context.new_source_ciphertext {
        return Err(TokenError::ConfidentialTransferBalanceMismatch.into());
    }

    confidential_transfer_account.available_balance = new_source_available_balance;
    confidential_transfer_account.decryptable_available_balance =
        new_source_decryptable_available_balance;

    Ok(())
}

#[cfg(feature = "zk-ops")]
fn process_destination_for_transfer(
    destination_account_info: &AccountInfo,
    mint_info: &AccountInfo,
    proof_context: &TransferProofContext,
) -> ProgramResult {
    check_program_account(destination_account_info.owner)?;
    let destination_token_account_data = &mut destination_account_info.data.borrow_mut();
    let mut destination_token_account =
        PodStateWithExtensionsMut::<PodAccount>::unpack(destination_token_account_data)?;

    if destination_token_account.base.is_frozen() {
        return Err(TokenError::AccountFrozen.into());
    }

    if destination_token_account.base.mint != *mint_info.key {
        return Err(TokenError::MintMismatch.into());
    }

    if memo_required(&destination_token_account) {
        check_previous_sibling_instruction_is_memo()?;
    }

    let destination_confidential_transfer_account =
        destination_token_account.get_extension_mut::<ConfidentialTransferAccount>()?;
    destination_confidential_transfer_account.valid_as_destination()?;

    if proof_context.transfer_pubkeys.destination
        != destination_confidential_transfer_account.elgamal_pubkey
    {
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }

    let destination_ciphertext_lo = proof_context
        .ciphertext_lo
        .try_extract_ciphertext(1)
        .map_err(|e| -> TokenError { e.into() })?;
    let destination_ciphertext_hi = proof_context
        .ciphertext_hi
        .try_extract_ciphertext(1)
        .map_err(|e| -> TokenError { e.into() })?;

    destination_confidential_transfer_account.pending_balance_lo = ciphertext_arithmetic::add(
        &destination_confidential_transfer_account.pending_balance_lo,
        &destination_ciphertext_lo,
    )
    .ok_or(TokenError::CiphertextArithmeticFailed)?;

    destination_confidential_transfer_account.pending_balance_hi = ciphertext_arithmetic::add(
        &destination_confidential_transfer_account.pending_balance_hi,
        &destination_ciphertext_hi,
    )
    .ok_or(TokenError::CiphertextArithmeticFailed)?;

    destination_confidential_transfer_account.increment_pending_balance_credit_counter()?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[cfg(feature = "zk-ops")]
fn process_source_for_transfer_with_fee(
    program_id: &Pubkey,
    source_account_info: &AccountInfo,
    mint_info: &AccountInfo,
    authority_info: &AccountInfo,
    signers: &[AccountInfo],
    proof_context: &TransferWithFeeProofContext,
    new_source_decryptable_available_balance: DecryptableBalance,
) -> ProgramResult {
    check_program_account(source_account_info.owner)?;
    let authority_info_data_len = authority_info.data_len();
    let token_account_data = &mut source_account_info.data.borrow_mut();
    let mut token_account = PodStateWithExtensionsMut::<PodAccount>::unpack(token_account_data)?;
    if token_account
        .get_extension::<NonTransferableAccount>()
        .is_ok()
    {
        return Err(TokenError::NonTransferable.into());
    }

    Processor::validate_owner(
        program_id,
        &token_account.base.owner,
        authority_info,
        authority_info_data_len,
        signers,
    )?;

    if token_account.base.is_frozen() {
        return Err(TokenError::AccountFrozen.into());
    }

    if token_account.base.mint != *mint_info.key {
        return Err(TokenError::MintMismatch.into());
    }

    let confidential_transfer_account =
        token_account.get_extension_mut::<ConfidentialTransferAccount>()?;
    confidential_transfer_account.valid_as_source()?;

    // Check that the source encryption public key is consistent with what was
    // actually used to generate the zkp.
    if proof_context.transfer_with_fee_pubkeys.source
        != confidential_transfer_account.elgamal_pubkey
    {
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }

    let source_transfer_amount_lo = proof_context
        .ciphertext_lo
        .try_extract_ciphertext(0)
        .map_err(|e| -> TokenError { e.into() })?;
    let source_transfer_amount_hi = proof_context
        .ciphertext_hi
        .try_extract_ciphertext(0)
        .map_err(|e| -> TokenError { e.into() })?;

    let new_source_available_balance = ciphertext_arithmetic::subtract_with_lo_hi(
        &confidential_transfer_account.available_balance,
        &source_transfer_amount_lo,
        &source_transfer_amount_hi,
    )
    .ok_or(TokenError::CiphertextArithmeticFailed)?;

    // Check that the computed available balance is consistent with what was
    // actually used to generate the zkp on the client side.
    if new_source_available_balance != proof_context.new_source_ciphertext {
        return Err(TokenError::ConfidentialTransferBalanceMismatch.into());
    }

    confidential_transfer_account.available_balance = new_source_available_balance;
    confidential_transfer_account.decryptable_available_balance =
        new_source_decryptable_available_balance;

    Ok(())
}

#[cfg(feature = "zk-ops")]
fn process_destination_for_transfer_with_fee(
    destination_account_info: &AccountInfo,
    mint_info: &AccountInfo,
    proof_context: &TransferWithFeeProofContext,
    is_self_transfer: bool,
) -> ProgramResult {
    check_program_account(destination_account_info.owner)?;
    let destination_token_account_data = &mut destination_account_info.data.borrow_mut();
    let mut destination_token_account =
        PodStateWithExtensionsMut::<PodAccount>::unpack(destination_token_account_data)?;

    if destination_token_account.base.is_frozen() {
        return Err(TokenError::AccountFrozen.into());
    }

    if destination_token_account.base.mint != *mint_info.key {
        return Err(TokenError::MintMismatch.into());
    }

    if memo_required(&destination_token_account) {
        check_previous_sibling_instruction_is_memo()?;
    }

    let destination_confidential_transfer_account =
        destination_token_account.get_extension_mut::<ConfidentialTransferAccount>()?;
    destination_confidential_transfer_account.valid_as_destination()?;

    if proof_context.transfer_with_fee_pubkeys.destination
        != destination_confidential_transfer_account.elgamal_pubkey
    {
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }

    let destination_transfer_amount_lo = proof_context
        .ciphertext_lo
        .try_extract_ciphertext(1)
        .map_err(|e| -> TokenError { e.into() })?;
    let destination_transfer_amount_hi = proof_context
        .ciphertext_hi
        .try_extract_ciphertext(1)
        .map_err(|e| -> TokenError { e.into() })?;

    destination_confidential_transfer_account.pending_balance_lo = ciphertext_arithmetic::add(
        &destination_confidential_transfer_account.pending_balance_lo,
        &destination_transfer_amount_lo,
    )
    .ok_or(TokenError::CiphertextArithmeticFailed)?;

    destination_confidential_transfer_account.pending_balance_hi = ciphertext_arithmetic::add(
        &destination_confidential_transfer_account.pending_balance_hi,
        &destination_transfer_amount_hi,
    )
    .ok_or(TokenError::CiphertextArithmeticFailed)?;

    destination_confidential_transfer_account.increment_pending_balance_credit_counter()?;

    // process transfer fee
    if !is_self_transfer {
        // Decode lo and hi fee amounts encrypted under the destination encryption
        // public key
        let destination_fee_lo = proof_context
            .fee_ciphertext_lo
            .try_extract_ciphertext(0)
            .map_err(|e| -> TokenError { e.into() })?;
        let destination_fee_hi = proof_context
            .fee_ciphertext_hi
            .try_extract_ciphertext(0)
            .map_err(|e| -> TokenError { e.into() })?;

        // Subtract the fee amount from the destination pending balance
        destination_confidential_transfer_account.pending_balance_lo =
            ciphertext_arithmetic::subtract(
                &destination_confidential_transfer_account.pending_balance_lo,
                &destination_fee_lo,
            )
            .ok_or(TokenError::CiphertextArithmeticFailed)?;
        destination_confidential_transfer_account.pending_balance_hi =
            ciphertext_arithmetic::subtract(
                &destination_confidential_transfer_account.pending_balance_hi,
                &destination_fee_hi,
            )
            .ok_or(TokenError::CiphertextArithmeticFailed)?;

        // Decode lo and hi fee amounts encrypted under the withdraw authority
        // encryption public key
        let withdraw_withheld_authority_fee_lo = proof_context
            .fee_ciphertext_lo
            .try_extract_ciphertext(1)
            .map_err(|e| -> TokenError { e.into() })?;
        let withdraw_withheld_authority_fee_hi = proof_context
            .fee_ciphertext_hi
            .try_extract_ciphertext(1)
            .map_err(|e| -> TokenError { e.into() })?;

        let destination_confidential_transfer_fee_amount =
            destination_token_account.get_extension_mut::<ConfidentialTransferFeeAmount>()?;

        // Add the fee amount to the destination withheld fee
        destination_confidential_transfer_fee_amount.withheld_amount =
            ciphertext_arithmetic::add_with_lo_hi(
                &destination_confidential_transfer_fee_amount.withheld_amount,
                &withdraw_withheld_authority_fee_lo,
                &withdraw_withheld_authority_fee_hi,
            )
            .ok_or(TokenError::CiphertextArithmeticFailed)?;
    }

    Ok(())
}

/// Processes an [ApplyPendingBalance] instruction.
#[cfg(feature = "zk-ops")]
fn process_apply_pending_balance(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    ApplyPendingBalanceData {
        expected_pending_balance_credit_counter,
        new_decryptable_available_balance,
    }: &ApplyPendingBalanceData,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let token_account_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let authority_info_data_len = authority_info.data_len();

    check_program_account(token_account_info.owner)?;
    let token_account_data = &mut token_account_info.data.borrow_mut();
    let mut token_account = PodStateWithExtensionsMut::<PodAccount>::unpack(token_account_data)?;

    Processor::validate_owner(
        program_id,
        &token_account.base.owner,
        authority_info,
        authority_info_data_len,
        account_info_iter.as_slice(),
    )?;

    let confidential_transfer_account =
        token_account.get_extension_mut::<ConfidentialTransferAccount>()?;

    confidential_transfer_account.available_balance = ciphertext_arithmetic::add_with_lo_hi(
        &confidential_transfer_account.available_balance,
        &confidential_transfer_account.pending_balance_lo,
        &confidential_transfer_account.pending_balance_hi,
    )
    .ok_or(TokenError::CiphertextArithmeticFailed)?;

    confidential_transfer_account.actual_pending_balance_credit_counter =
        confidential_transfer_account.pending_balance_credit_counter;
    confidential_transfer_account.expected_pending_balance_credit_counter =
        *expected_pending_balance_credit_counter;
    confidential_transfer_account.decryptable_available_balance =
        *new_decryptable_available_balance;
    confidential_transfer_account.pending_balance_credit_counter = 0.into();
    confidential_transfer_account.pending_balance_lo = EncryptedBalance::zeroed();
    confidential_transfer_account.pending_balance_hi = EncryptedBalance::zeroed();

    Ok(())
}

/// Processes a [DisableConfidentialCredits] or [EnableConfidentialCredits]
/// instruction.
fn process_allow_confidential_credits(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    allow_confidential_credits: bool,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let token_account_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let authority_info_data_len = authority_info.data_len();

    check_program_account(token_account_info.owner)?;
    let token_account_data = &mut token_account_info.data.borrow_mut();
    let mut token_account = PodStateWithExtensionsMut::<PodAccount>::unpack(token_account_data)?;

    Processor::validate_owner(
        program_id,
        &token_account.base.owner,
        authority_info,
        authority_info_data_len,
        account_info_iter.as_slice(),
    )?;

    let confidential_transfer_account =
        token_account.get_extension_mut::<ConfidentialTransferAccount>()?;
    confidential_transfer_account.allow_confidential_credits = allow_confidential_credits.into();

    Ok(())
}

/// Processes an [DisableNonConfidentialCredits] or
/// [EnableNonConfidentialCredits] instruction.
fn process_allow_non_confidential_credits(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    allow_non_confidential_credits: bool,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let token_account_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let authority_info_data_len = authority_info.data_len();

    check_program_account(token_account_info.owner)?;
    let token_account_data = &mut token_account_info.data.borrow_mut();
    let mut token_account = PodStateWithExtensionsMut::<PodAccount>::unpack(token_account_data)?;

    Processor::validate_owner(
        program_id,
        &token_account.base.owner,
        authority_info,
        authority_info_data_len,
        account_info_iter.as_slice(),
    )?;

    let confidential_transfer_account =
        token_account.get_extension_mut::<ConfidentialTransferAccount>()?;
    confidential_transfer_account.allow_non_confidential_credits =
        allow_non_confidential_credits.into();

    Ok(())
}

#[allow(dead_code)]
pub(crate) fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    check_program_account(program_id)?;

    match decode_instruction_type(input)? {
        ConfidentialTransferInstruction::InitializeMint => {
            msg!("ConfidentialTransferInstruction::InitializeMint");
            let data = decode_instruction_data::<InitializeMintData>(input)?;
            process_initialize_mint(
                accounts,
                &data.authority,
                data.auto_approve_new_accounts,
                &data.auditor_elgamal_pubkey,
            )
        }
        ConfidentialTransferInstruction::UpdateMint => {
            msg!("ConfidentialTransferInstruction::UpdateMint");
            let data = decode_instruction_data::<UpdateMintData>(input)?;
            process_update_mint(
                accounts,
                data.auto_approve_new_accounts,
                &data.auditor_elgamal_pubkey,
            )
        }
        ConfidentialTransferInstruction::ConfigureAccount => {
            msg!("ConfidentialTransferInstruction::ConfigureAccount");
            let data = decode_instruction_data::<ConfigureAccountInstructionData>(input)?;
            process_configure_account(
                program_id,
                accounts,
                &data.decryptable_zero_balance,
                &data.maximum_pending_balance_credit_counter,
                ElGamalPubkeySource::ProofInstructionOffset(data.proof_instruction_offset as i64),
            )
        }
        ConfidentialTransferInstruction::ApproveAccount => {
            msg!("ConfidentialTransferInstruction::ApproveAccount");
            process_approve_account(accounts)
        }
        ConfidentialTransferInstruction::EmptyAccount => {
            msg!("ConfidentialTransferInstruction::EmptyAccount");
            let data = decode_instruction_data::<EmptyAccountInstructionData>(input)?;
            process_empty_account(program_id, accounts, data.proof_instruction_offset as i64)
        }
        ConfidentialTransferInstruction::Deposit => {
            msg!("ConfidentialTransferInstruction::Deposit");
            #[cfg(feature = "zk-ops")]
            {
                let data = decode_instruction_data::<DepositInstructionData>(input)?;
                process_deposit(program_id, accounts, data.amount.into(), data.decimals)
            }
            #[cfg(not(feature = "zk-ops"))]
            Err(ProgramError::InvalidInstructionData)
        }
        ConfidentialTransferInstruction::Withdraw => {
            msg!("ConfidentialTransferInstruction::Withdraw");
            #[cfg(feature = "zk-ops")]
            {
                let data = decode_instruction_data::<WithdrawInstructionData>(input)?;
                process_withdraw(
                    program_id,
                    accounts,
                    data.amount.into(),
                    data.decimals,
                    data.new_decryptable_available_balance,
                    data.equality_proof_instruction_offset as i64,
                    data.range_proof_instruction_offset as i64,
                )
            }
            #[cfg(not(feature = "zk-ops"))]
            Err(ProgramError::InvalidInstructionData)
        }
        ConfidentialTransferInstruction::Transfer => {
            msg!("ConfidentialTransferInstruction::Transfer");
            #[cfg(feature = "zk-ops")]
            {
                let data = decode_instruction_data::<TransferInstructionData>(input)?;
                process_transfer(
                    program_id,
                    accounts,
                    data.new_source_decryptable_available_balance,
                    data.equality_proof_instruction_offset as i64,
                    data.ciphertext_validity_proof_instruction_offset as i64,
                    None,
                    None,
                    data.range_proof_instruction_offset as i64,
                )
            }
            #[cfg(not(feature = "zk-ops"))]
            Err(ProgramError::InvalidInstructionData)
        }
        ConfidentialTransferInstruction::ApplyPendingBalance => {
            msg!("ConfidentialTransferInstruction::ApplyPendingBalance");
            #[cfg(feature = "zk-ops")]
            {
                process_apply_pending_balance(
                    program_id,
                    accounts,
                    decode_instruction_data::<ApplyPendingBalanceData>(input)?,
                )
            }
            #[cfg(not(feature = "zk-ops"))]
            {
                Err(ProgramError::InvalidInstructionData)
            }
        }
        ConfidentialTransferInstruction::DisableConfidentialCredits => {
            msg!("ConfidentialTransferInstruction::DisableConfidentialCredits");
            process_allow_confidential_credits(program_id, accounts, false)
        }
        ConfidentialTransferInstruction::EnableConfidentialCredits => {
            msg!("ConfidentialTransferInstruction::EnableConfidentialCredits");
            process_allow_confidential_credits(program_id, accounts, true)
        }
        ConfidentialTransferInstruction::DisableNonConfidentialCredits => {
            msg!("ConfidentialTransferInstruction::DisableNonConfidentialCredits");
            process_allow_non_confidential_credits(program_id, accounts, false)
        }
        ConfidentialTransferInstruction::EnableNonConfidentialCredits => {
            msg!("ConfidentialTransferInstruction::EnableNonConfidentialCredits");
            process_allow_non_confidential_credits(program_id, accounts, true)
        }
        ConfidentialTransferInstruction::TransferWithFee => {
            msg!("ConfidentialTransferInstruction::TransferWithFee");
            #[cfg(feature = "zk-ops")]
            {
                let data = decode_instruction_data::<TransferWithFeeInstructionData>(input)?;
                process_transfer(
                    program_id,
                    accounts,
                    data.new_source_decryptable_available_balance,
                    data.equality_proof_instruction_offset as i64,
                    data.transfer_amount_ciphertext_validity_proof_instruction_offset as i64,
                    Some(data.fee_sigma_proof_instruction_offset as i64),
                    Some(data.fee_ciphertext_validity_proof_instruction_offset as i64),
                    data.range_proof_instruction_offset as i64,
                )
            }
            #[cfg(not(feature = "zk-ops"))]
            Err(ProgramError::InvalidInstructionData)
        }
        ConfidentialTransferInstruction::ConfigureAccountWithRegistry => {
            msg!("ConfidentialTransferInstruction::ConfigureAccountWithRegistry");
            process_configure_account_with_registry(program_id, accounts)
        }
    }
}
