use {
    crate::{
        check_program_account,
        error::TokenError,
        extension::{
            confidential_transfer::{instruction::*, *},
            BaseStateWithExtensions, StateWithExtensions, StateWithExtensionsMut,
        },
        instruction::{decode_instruction_data, decode_instruction_type},
        processor::Processor,
        state::{Account, Mint},
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
    },
};
// Remove feature once zk ops syscalls are enabled on all networks
#[cfg(feature = "zk-ops")]
use {
    crate::extension::{non_transferable::NonTransferable, transfer_fee::TransferFeeConfig},
    solana_zk_token_sdk::zk_token_elgamal::ops as syscall,
};

#[cfg(feature = "proof-program")]
use {
    crate::extension::memo_transfer::{check_previous_sibling_instruction_is_memo, memo_required},
    solana_program::instruction::Instruction,
    solana_program::sysvar::instructions::get_instruction_relative,
    solana_program::{clock::Clock, sysvar::Sysvar},
    solana_zk_token_sdk::zk_token_proof_program,
};

/// Decodes the zero-knowledge proof instruction associated with the token instruction.
///
/// `ConfigureAccount`, `EmptyAccount`, `Withdraw`, `Transfer`, `WithdrawWithheldTokensFromMint`,
/// and `WithdrawWithheldTokensFromAccounts` instructions require corresponding zero-knowledge
/// proof instructions.
#[cfg(feature = "proof-program")]
fn decode_proof_instruction<T: Pod>(
    expected: ProofInstruction,
    instruction: &Instruction,
) -> Result<&T, ProgramError> {
    if instruction.program_id != zk_token_proof_program::id()
        || ProofInstruction::decode_type(&instruction.data) != Some(expected)
    {
        msg!("Unexpected proof instruction");
        return Err(ProgramError::InvalidInstructionData);
    }

    ProofInstruction::decode_data(&instruction.data).ok_or(ProgramError::InvalidInstructionData)
}

/// Processes an [InitializeMint] instruction.
fn process_initialize_mint(
    accounts: &[AccountInfo],
    authority: &OptionalNonZeroPubkey,
    auto_approve_new_account: PodBool,
    auditor_encryption_pubkey: &OptionalNonZeroEncryptionPubkey,
    withdraw_withheld_authority_encryption_pubkey: &OptionalNonZeroEncryptionPubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_info = next_account_info(account_info_iter)?;

    check_program_account(mint_info.owner)?;
    let mint_data = &mut mint_info.data.borrow_mut();
    let mut mint = StateWithExtensionsMut::<Mint>::unpack_uninitialized(mint_data)?;
    let confidential_transfer_mint = mint.init_extension::<ConfidentialTransferMint>(true)?;

    confidential_transfer_mint.authority = *authority;
    confidential_transfer_mint.auto_approve_new_accounts = auto_approve_new_account;
    confidential_transfer_mint.auditor_encryption_pubkey = *auditor_encryption_pubkey;
    confidential_transfer_mint.withdraw_withheld_authority_encryption_pubkey =
        *withdraw_withheld_authority_encryption_pubkey;
    confidential_transfer_mint.withheld_amount = EncryptedWithheldAmount::zeroed();

    Ok(())
}

/// Processes an [UpdateMint] instruction.
fn process_update_mint(
    accounts: &[AccountInfo],
    auto_approve_new_account: PodBool,
    auditor_encryption_pubkey: &OptionalNonZeroEncryptionPubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;

    check_program_account(mint_info.owner)?;
    let mint_data = &mut mint_info.data.borrow_mut();
    let mut mint = StateWithExtensionsMut::<Mint>::unpack(mint_data)?;
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
    confidential_transfer_mint.auditor_encryption_pubkey = *auditor_encryption_pubkey;
    Ok(())
}

/// Processes a [ConfigureAccount] instruction.
#[cfg(feature = "proof-program")]
fn process_configure_account(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    decryptable_zero_balance: &DecryptableBalance,
    maximum_pending_balance_credit_counter: &PodU64,
    proof_instruction_offset: i64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let token_account_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let instructions_sysvar_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let authority_info_data_len = authority_info.data_len();

    check_program_account(token_account_info.owner)?;
    let token_account_data = &mut token_account_info.data.borrow_mut();
    let mut token_account = StateWithExtensionsMut::<Account>::unpack(token_account_data)?;

    if token_account.base.mint != *mint_info.key {
        return Err(TokenError::MintMismatch.into());
    }

    Processor::validate_owner(
        program_id,
        &token_account.base.owner,
        authority_info,
        authority_info_data_len,
        account_info_iter.as_slice(),
    )?;

    check_program_account(mint_info.owner)?;
    let mint_data = &mut mint_info.data.borrow();
    let mint = StateWithExtensions::<Mint>::unpack(mint_data)?;
    let confidential_transfer_mint = mint.get_extension::<ConfidentialTransferMint>()?;

    // zero-knowledge proof certifies that the supplied encryption (ElGamal) public key is valid
    let zkp_instruction =
        get_instruction_relative(proof_instruction_offset, instructions_sysvar_info)?;
    let proof_data = decode_proof_instruction::<PubkeyValidityData>(
        ProofInstruction::VerifyPubkeyValidity,
        &zkp_instruction,
    )?;

    // Note: The caller is expected to use the `Reallocate` instruction to ensure there is
    // sufficient room in their token account for the new `ConfidentialTransferAccount` extension
    let mut confidential_transfer_account =
        token_account.init_extension::<ConfidentialTransferAccount>(false)?;
    confidential_transfer_account.approved = confidential_transfer_mint.auto_approve_new_accounts;
    confidential_transfer_account.encryption_pubkey = proof_data.pubkey;
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
    confidential_transfer_account.withheld_amount = EncryptedWithheldAmount::zeroed();

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
    let mut token_account = StateWithExtensionsMut::<Account>::unpack(token_account_data)?;

    check_program_account(mint_info.owner)?;
    let mint_data = &mint_info.data.borrow_mut();
    let mint = StateWithExtensions::<Mint>::unpack(mint_data)?;
    let confidential_transfer_mint = mint.get_extension::<ConfidentialTransferMint>()?;
    let maybe_confidential_transfer_mint_authority: Option<Pubkey> =
        confidential_transfer_mint.authority.into();
    let confidential_transfer_mint_authority =
        maybe_confidential_transfer_mint_authority.ok_or(TokenError::NoAuthorityExists)?;

    if authority_info.is_signer && *authority_info.key == confidential_transfer_mint_authority {
        let mut confidential_transfer_state =
            token_account.get_extension_mut::<ConfidentialTransferAccount>()?;
        confidential_transfer_state.approved = true.into();
        Ok(())
    } else {
        Err(ProgramError::MissingRequiredSignature)
    }
}

/// Processes an [EmptyAccount] instruction.
#[cfg(feature = "proof-program")]
fn process_empty_account(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    proof_instruction_offset: i64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let token_account_info = next_account_info(account_info_iter)?;
    let instructions_sysvar_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let authority_info_data_len = authority_info.data_len();

    check_program_account(token_account_info.owner)?;
    let token_account_data = &mut token_account_info.data.borrow_mut();
    let mut token_account = StateWithExtensionsMut::<Account>::unpack(token_account_data)?;

    Processor::validate_owner(
        program_id,
        &token_account.base.owner,
        authority_info,
        authority_info_data_len,
        account_info_iter.as_slice(),
    )?;

    let mut confidential_transfer_account =
        token_account.get_extension_mut::<ConfidentialTransferAccount>()?;

    // An account can be closed only if the remaining balance is zero. This means that for the
    // confidential extension account, the ciphertexts associated with the following components
    // must be an encryption of zero:
    //   1. The pending balance
    //   2. The available balance
    //   3. The withheld balance
    //
    // For the pending and withheld balance ciphertexts, it suffices to check that they are
    // all-zero ciphertexts (i.e. [0; 64]). If any of these ciphertexts are valid encryption of
    // zero but not an all-zero ciphertext, then an `ApplyPendingBalance` or
    // `HarvestWithheldTokensToMint` instructions can be used to flush-out these balances first.
    //
    // For the available balance, it is not possible to deduce whether the ciphertext encrypts zero
    // or not by simply inspecting the ciphertext bytes (otherwise, this would violate
    // confidentiality). The available balance is verified using a zero-knowledge proof.
    let zkp_instruction =
        get_instruction_relative(proof_instruction_offset, instructions_sysvar_info)?;
    let proof_data = decode_proof_instruction::<CloseAccountData>(
        ProofInstruction::VerifyCloseAccount,
        &zkp_instruction,
    )?;
    // Check that the encryption public key and ciphertext associated with the confidential
    // extension account are consistent with those that were actually used to generate the zkp.
    if confidential_transfer_account.encryption_pubkey != proof_data.pubkey {
        msg!("Encryption public-key mismatch");
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }
    if confidential_transfer_account.available_balance != proof_data.ciphertext {
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
    let mint = StateWithExtensions::<Mint>::unpack(mint_data)?;

    if expected_decimals != mint.base.decimals {
        return Err(TokenError::MintDecimalsMismatch.into());
    }

    if mint.get_extension::<NonTransferable>().is_ok() {
        return Err(TokenError::NonTransferable.into());
    }

    check_program_account(token_account_info.owner)?;
    let token_account_data = &mut token_account_info.data.borrow_mut();
    let mut token_account = StateWithExtensionsMut::<Account>::unpack(token_account_data)?;

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

    token_account.base.amount = token_account
        .base
        .amount
        .checked_sub(amount)
        .ok_or(TokenError::Overflow)?;
    token_account.pack_base();

    let mut confidential_transfer_account =
        token_account.get_extension_mut::<ConfidentialTransferAccount>()?;
    confidential_transfer_account.valid_as_destination()?;

    // A deposit amount must be a 48-bit number
    let (amount_lo, amount_hi) = verify_and_split_deposit_amount(amount)?;

    // Prevent unnecessary ciphertext arithmetic syscalls if `amount_lo` or `amount_hi` is zero
    if amount_lo > 0 {
        confidential_transfer_account.pending_balance_lo =
            syscall::add_to(&confidential_transfer_account.pending_balance_lo, amount_lo)
                .ok_or(ProgramError::InvalidInstructionData)?;
    }
    if amount_hi > 0 {
        confidential_transfer_account.pending_balance_hi =
            syscall::add_to(&confidential_transfer_account.pending_balance_hi, amount_hi)
                .ok_or(ProgramError::InvalidInstructionData)?;
    }

    confidential_transfer_account.increment_pending_balance_credit_counter()?;

    Ok(())
}

/// Verifies that a deposit amount is a 48-bit number and returns the least significant 16 bits and
/// most significant 32 bits of the amount.
#[cfg(feature = "zk-ops")]
fn verify_and_split_deposit_amount(amount: u64) -> Result<(u64, u64), TokenError> {
    if amount >> MAXIMUM_DEPOSIT_TRANSFER_AMOUNT_BIT_LENGTH > 0 {
        return Err(TokenError::MaximumDepositAmountExceeded);
    }
    let deposit_amount_lo =
        amount << (64 - PENDING_BALANCE_LO_BIT_LENGTH) >> PENDING_BALANCE_HI_BIT_LENGTH;
    let deposit_amount_hi = amount >> PENDING_BALANCE_LO_BIT_LENGTH;

    Ok((deposit_amount_lo, deposit_amount_hi))
}

/// Processes a [Withdraw] instruction.
#[cfg(all(feature = "zk-ops", feature = "proof-program"))]
fn process_withdraw(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
    expected_decimals: u8,
    new_decryptable_available_balance: DecryptableBalance,
    proof_instruction_offset: i64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let token_account_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let instructions_sysvar_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let authority_info_data_len = authority_info.data_len();

    check_program_account(mint_info.owner)?;
    let mint_data = &mint_info.data.borrow_mut();
    let mint = StateWithExtensions::<Mint>::unpack(mint_data)?;

    if expected_decimals != mint.base.decimals {
        return Err(TokenError::MintDecimalsMismatch.into());
    }

    if mint.get_extension::<NonTransferable>().is_ok() {
        return Err(TokenError::NonTransferable.into());
    }

    check_program_account(token_account_info.owner)?;
    let token_account_data = &mut token_account_info.data.borrow_mut();
    let mut token_account = StateWithExtensionsMut::<Account>::unpack(token_account_data)?;

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

    // Wrapped SOL withdrawals are not supported because lamports cannot be apparated.
    assert!(!token_account.base.is_native());

    let mut confidential_transfer_account =
        token_account.get_extension_mut::<ConfidentialTransferAccount>()?;
    confidential_transfer_account.valid_as_source()?;

    // Zero-knowledge proof certifies that the account has enough available balance to withdraw the
    // amount.
    let zkp_instruction =
        get_instruction_relative(proof_instruction_offset, instructions_sysvar_info)?;
    let proof_data = decode_proof_instruction::<WithdrawData>(
        ProofInstruction::VerifyWithdraw,
        &zkp_instruction,
    )?;
    // Check that the encryption public key associated with the confidential extension is
    // consistent with the public key that was actually used to generate the zkp.
    if confidential_transfer_account.encryption_pubkey != proof_data.pubkey {
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }

    // Prevent unnecessary ciphertext arithmetic syscalls if the withdraw amount is zero
    if amount > 0 {
        confidential_transfer_account.available_balance =
            syscall::subtract_from(&confidential_transfer_account.available_balance, amount)
                .ok_or(ProgramError::InvalidInstructionData)?;
    }
    // Check that the final available balance ciphertext is consistent with the actual ciphertext
    // for which the zero-knowledge proof was generated for.
    if confidential_transfer_account.available_balance != proof_data.final_ciphertext {
        return Err(TokenError::ConfidentialTransferBalanceMismatch.into());
    }

    confidential_transfer_account.decryptable_available_balance = new_decryptable_available_balance;
    token_account.base.amount = token_account
        .base
        .amount
        .checked_add(amount)
        .ok_or(TokenError::Overflow)?;
    token_account.pack_base();

    Ok(())
}

/// Processes an [Transfer] instruction.
#[cfg(all(feature = "zk-ops", feature = "proof-program"))]
fn process_transfer(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_source_decryptable_available_balance: DecryptableBalance,
    proof_instruction_offset: i64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let source_account_info = next_account_info(account_info_iter)?;
    let destination_token_account_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let instructions_sysvar_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;

    check_program_account(mint_info.owner)?;
    let mint_data = &mint_info.data.borrow_mut();
    let mint = StateWithExtensions::<Mint>::unpack(mint_data)?;

    if mint.get_extension::<NonTransferable>().is_ok() {
        return Err(TokenError::NonTransferable.into());
    }
    let confidential_transfer_mint = mint.get_extension::<ConfidentialTransferMint>()?;

    // A `Transfer` instruction must be accompanied by a zero-knowledge proof instruction that
    // certify the validity of the transfer amounts. The kind of zero-knowledge proof instruction
    // depends on whether a transfer incurs a fee or not.
    //   - If the mint is not extended for fees or the instruction is for a self-transfer, then
    //   transfer fee is not required.
    //   - If the mint is extended for fees and the instruction is not a self-transfer, then
    //   transfer fee is required.
    if mint.get_extension::<TransferFeeConfig>().is_err()
        || source_account_info.key == destination_token_account_info.key
    {
        // Transfer fee is not required. Decode the zero-knowledge proof as `TransferData`.
        //
        // The zero-knowledge proof certifies that:
        //   1. the transfer amount is encrypted in the correct form
        //   2. the source account has enough balance to send the transfer amount
        let zkp_instruction =
            get_instruction_relative(proof_instruction_offset, instructions_sysvar_info)?;
        let proof_data = decode_proof_instruction::<TransferData>(
            ProofInstruction::VerifyTransfer,
            &zkp_instruction,
        )?;
        // Check that the auditor encryption public key associated wth the confidential mint is
        // consistent with what was actually used to generate the zkp.
        if !confidential_transfer_mint
            .auditor_encryption_pubkey
            .equals(&proof_data.transfer_pubkeys.auditor_pubkey)
        {
            return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
        }

        let source_ciphertext_lo = EncryptedBalance::from((
            proof_data.ciphertext_lo.commitment,
            proof_data.ciphertext_lo.source_handle,
        ));
        let source_ciphertext_hi = EncryptedBalance::from((
            proof_data.ciphertext_hi.commitment,
            proof_data.ciphertext_hi.source_handle,
        ));

        process_source_for_transfer(
            program_id,
            source_account_info,
            mint_info,
            authority_info,
            account_info_iter.as_slice(),
            &proof_data.transfer_pubkeys.source_pubkey,
            &source_ciphertext_lo,
            &source_ciphertext_hi,
            &proof_data.new_source_ciphertext,
            new_source_decryptable_available_balance,
        )?;

        let destination_ciphertext_lo = EncryptedBalance::from((
            proof_data.ciphertext_lo.commitment,
            proof_data.ciphertext_lo.destination_handle,
        ));
        let destination_ciphertext_hi = EncryptedBalance::from((
            proof_data.ciphertext_hi.commitment,
            proof_data.ciphertext_hi.destination_handle,
        ));

        process_destination_for_transfer(
            destination_token_account_info,
            mint_info,
            &proof_data.transfer_pubkeys.destination_pubkey,
            &destination_ciphertext_lo,
            &destination_ciphertext_hi,
            None,
        )?;
    } else {
        // Transfer fee is required. Decode the zero-knowledge proof as `TransferWithFeeData`.
        //
        // The zero-knowledge proof certifies that:
        //   1. the transfer amount is encrypted in the correct form
        //   2. the source account has enough balance to send the transfer amount
        //   3. the transfer fee is computed correctly and encrypted in the correct form
        let zkp_instruction =
            get_instruction_relative(proof_instruction_offset, instructions_sysvar_info)?;
        let proof_data = decode_proof_instruction::<TransferWithFeeData>(
            ProofInstruction::VerifyTransferWithFee,
            &zkp_instruction,
        )?;
        // Check that the encryption public keys associated with the confidential extension mint
        // are consistent with the keys that were used to generate the zkp.
        if !confidential_transfer_mint
            .auditor_encryption_pubkey
            .equals(&proof_data.transfer_with_fee_pubkeys.auditor_pubkey)
        {
            return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
        }
        if !confidential_transfer_mint
            .withdraw_withheld_authority_encryption_pubkey
            .equals(
                &proof_data
                    .transfer_with_fee_pubkeys
                    .withdraw_withheld_authority_pubkey,
            )
        {
            return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
        }
        // Check that the fee parameters in the mint are consistent with what were used to generate
        // the zkp.
        let transfer_fee_config = mint.get_extension::<TransferFeeConfig>()?;
        let fee_parameters = transfer_fee_config.get_epoch_fee(Clock::get()?.epoch);
        if u64::from(fee_parameters.maximum_fee) != u64::from(proof_data.fee_parameters.maximum_fee)
            || u16::from(fee_parameters.transfer_fee_basis_points)
                != u16::from(proof_data.fee_parameters.fee_rate_basis_points)
        {
            return Err(TokenError::FeeParametersMismatch.into());
        }

        // From the proof data, decode lo and hi transfer amounts encrypted under the source
        // encryption public key
        let source_transfer_amount_lo = EncryptedBalance::from((
            proof_data.ciphertext_lo.commitment,
            proof_data.ciphertext_lo.source_handle,
        ));
        let source_transfer_amount_hi = EncryptedBalance::from((
            proof_data.ciphertext_hi.commitment,
            proof_data.ciphertext_hi.source_handle,
        ));

        process_source_for_transfer(
            program_id,
            source_account_info,
            mint_info,
            authority_info,
            account_info_iter.as_slice(),
            &proof_data.transfer_with_fee_pubkeys.source_pubkey,
            &source_transfer_amount_lo,
            &source_transfer_amount_hi,
            &proof_data.new_source_ciphertext,
            new_source_decryptable_available_balance,
        )?;

        // From the proof datay, decode lo and hi transfer amounts encrypted under the destination
        // encryption public key
        let destination_transfer_amount_lo = EncryptedBalance::from((
            proof_data.ciphertext_lo.commitment,
            proof_data.ciphertext_lo.destination_handle,
        ));
        let destination_transfer_amount_hi = EncryptedBalance::from((
            proof_data.ciphertext_hi.commitment,
            proof_data.ciphertext_hi.destination_handle,
        ));

        process_destination_for_transfer(
            destination_token_account_info,
            mint_info,
            &proof_data.transfer_with_fee_pubkeys.destination_pubkey,
            &destination_transfer_amount_lo,
            &destination_transfer_amount_hi,
            Some((&proof_data.fee_ciphertext_lo, &proof_data.fee_ciphertext_hi)),
        )?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[cfg(all(feature = "zk-ops", feature = "proof-program"))]
fn process_source_for_transfer(
    program_id: &Pubkey,
    source_account_info: &AccountInfo,
    mint_info: &AccountInfo,
    authority_info: &AccountInfo,
    signers: &[AccountInfo],
    source_encryption_pubkey: &EncryptionPubkey,
    source_transfer_amount_lo: &EncryptedBalance,
    source_transfer_amount_hi: &EncryptedBalance,
    expected_new_source_available_balance: &EncryptedBalance,
    new_source_decryptable_available_balance: DecryptableBalance,
) -> ProgramResult {
    check_program_account(source_account_info.owner)?;
    let authority_info_data_len = authority_info.data_len();
    let token_account_data = &mut source_account_info.data.borrow_mut();
    let mut token_account = StateWithExtensionsMut::<Account>::unpack(token_account_data)?;

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

    let mut confidential_transfer_account =
        token_account.get_extension_mut::<ConfidentialTransferAccount>()?;
    confidential_transfer_account.valid_as_source()?;

    // Check that the source encryption public key is consistent with what was actually used to
    // generate the zkp.
    if *source_encryption_pubkey != confidential_transfer_account.encryption_pubkey {
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }

    let new_source_available_balance = syscall::subtract_with_lo_hi(
        &confidential_transfer_account.available_balance,
        source_transfer_amount_lo,
        source_transfer_amount_hi,
    )
    .ok_or(ProgramError::InvalidInstructionData)?;

    // Check that the computed available balance is consistent with what was actually used to
    // generate the zkp on the client side.
    if new_source_available_balance != *expected_new_source_available_balance {
        return Err(TokenError::ConfidentialTransferBalanceMismatch.into());
    }

    confidential_transfer_account.available_balance = new_source_available_balance;
    confidential_transfer_account.decryptable_available_balance =
        new_source_decryptable_available_balance;

    Ok(())
}

#[cfg(all(feature = "zk-ops", feature = "proof-program"))]
fn process_destination_for_transfer(
    destination_token_account_info: &AccountInfo,
    mint_info: &AccountInfo,
    destination_encryption_pubkey: &EncryptionPubkey,
    destination_transfer_amount_lo: &EncryptedBalance,
    destination_transfer_amount_hi: &EncryptedBalance,
    encrypted_fee: Option<(&EncryptedFee, &EncryptedFee)>,
) -> ProgramResult {
    check_program_account(destination_token_account_info.owner)?;
    let destination_token_account_data = &mut destination_token_account_info.data.borrow_mut();
    let mut destination_token_account =
        StateWithExtensionsMut::<Account>::unpack(destination_token_account_data)?;

    if destination_token_account.base.is_frozen() {
        return Err(TokenError::AccountFrozen.into());
    }

    if destination_token_account.base.mint != *mint_info.key {
        return Err(TokenError::MintMismatch.into());
    }

    if memo_required(&destination_token_account) {
        check_previous_sibling_instruction_is_memo()?;
    }

    let mut destination_confidential_transfer_account =
        destination_token_account.get_extension_mut::<ConfidentialTransferAccount>()?;
    destination_confidential_transfer_account.valid_as_destination()?;

    if *destination_encryption_pubkey != destination_confidential_transfer_account.encryption_pubkey
    {
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }

    destination_confidential_transfer_account.pending_balance_lo = syscall::add(
        &destination_confidential_transfer_account.pending_balance_lo,
        destination_transfer_amount_lo,
    )
    .ok_or(ProgramError::InvalidInstructionData)?;

    destination_confidential_transfer_account.pending_balance_hi = syscall::add(
        &destination_confidential_transfer_account.pending_balance_hi,
        destination_transfer_amount_hi,
    )
    .ok_or(ProgramError::InvalidInstructionData)?;

    destination_confidential_transfer_account.increment_pending_balance_credit_counter()?;

    // Process transfer fee
    if let Some((ciphertext_fee_lo, ciphertext_fee_hi)) = encrypted_fee {
        // Decode lo and hi fee amounts encrypted under the destination encryption public key
        let destination_fee_lo: EncryptedWithheldAmount = (
            ciphertext_fee_lo.commitment,
            ciphertext_fee_lo.destination_handle,
        )
            .into();
        let destination_fee_hi: EncryptedWithheldAmount = (
            ciphertext_fee_hi.commitment,
            ciphertext_fee_hi.destination_handle,
        )
            .into();

        // Subtract the fee amount from the destination pending balance
        destination_confidential_transfer_account.pending_balance_lo = syscall::subtract(
            &destination_confidential_transfer_account.pending_balance_lo,
            &destination_fee_lo,
        )
        .ok_or(ProgramError::InvalidInstructionData)?;
        destination_confidential_transfer_account.pending_balance_hi = syscall::subtract(
            &destination_confidential_transfer_account.pending_balance_hi,
            &destination_fee_hi,
        )
        .ok_or(ProgramError::InvalidInstructionData)?;

        // Decode lo and hi fee amounts encrypted under the withdraw authority encryption public
        // key
        let withdraw_withheld_authority_fee_lo: EncryptedWithheldAmount = (
            ciphertext_fee_lo.commitment,
            ciphertext_fee_lo.withdraw_withheld_authority_handle,
        )
            .into();
        let withdraw_withheld_authority_fee_hi: EncryptedWithheldAmount = (
            ciphertext_fee_hi.commitment,
            ciphertext_fee_hi.withdraw_withheld_authority_handle,
        )
            .into();

        // Add the fee amount to the destination withheld fee
        destination_confidential_transfer_account.withheld_amount = syscall::add_with_lo_hi(
            &destination_confidential_transfer_account.withheld_amount,
            &withdraw_withheld_authority_fee_lo,
            &withdraw_withheld_authority_fee_hi,
        )
        .ok_or(ProgramError::InvalidInstructionData)?;
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
    let mut token_account = StateWithExtensionsMut::<Account>::unpack(token_account_data)?;

    Processor::validate_owner(
        program_id,
        &token_account.base.owner,
        authority_info,
        authority_info_data_len,
        account_info_iter.as_slice(),
    )?;

    let mut confidential_transfer_account =
        token_account.get_extension_mut::<ConfidentialTransferAccount>()?;

    confidential_transfer_account.available_balance = syscall::add_with_lo_hi(
        &confidential_transfer_account.available_balance,
        &confidential_transfer_account.pending_balance_lo,
        &confidential_transfer_account.pending_balance_hi,
    )
    .ok_or(ProgramError::InvalidInstructionData)?;

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

/// Processes a [DisableConfidentialCredits] or [EnableConfidentialCredits] instruction.
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
    let mut token_account = StateWithExtensionsMut::<Account>::unpack(token_account_data)?;

    Processor::validate_owner(
        program_id,
        &token_account.base.owner,
        authority_info,
        authority_info_data_len,
        account_info_iter.as_slice(),
    )?;

    let mut confidential_transfer_account =
        token_account.get_extension_mut::<ConfidentialTransferAccount>()?;
    confidential_transfer_account.allow_confidential_credits = allow_confidential_credits.into();

    Ok(())
}

/// Processes an [DisableNonConfidentialCredits] or [EnableNonConfidentialCredits] instruction.
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
    let mut token_account = StateWithExtensionsMut::<Account>::unpack(token_account_data)?;

    Processor::validate_owner(
        program_id,
        &token_account.base.owner,
        authority_info,
        authority_info_data_len,
        account_info_iter.as_slice(),
    )?;

    let mut confidential_transfer_account =
        token_account.get_extension_mut::<ConfidentialTransferAccount>()?;
    confidential_transfer_account.allow_non_confidential_credits =
        allow_non_confidential_credits.into();

    Ok(())
}

/// Processes an [WithdrawWithheldTokensFromMint] instruction.
#[cfg(all(feature = "zk-ops", feature = "proof-program"))]
fn process_withdraw_withheld_tokens_from_mint(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    proof_instruction_offset: i64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;
    let destination_account_info = next_account_info(account_info_iter)?;
    let instructions_sysvar_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let authority_info_data_len = authority_info.data_len();

    // unnecessary check, but helps for clarity
    check_program_account(mint_account_info.owner)?;
    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = StateWithExtensionsMut::<Mint>::unpack(&mut mint_data)?;

    // mint must be extended for fees
    {
        let transfer_fee_config = mint.get_extension::<TransferFeeConfig>()?;
        let withdraw_withheld_authority =
            Option::<Pubkey>::from(transfer_fee_config.withdraw_withheld_authority)
                .ok_or(TokenError::NoAuthorityExists)?;
        Processor::validate_owner(
            program_id,
            &withdraw_withheld_authority,
            authority_info,
            authority_info_data_len,
            account_info_iter.as_slice(),
        )?;
    } // free `transfer_fee_config` to borrow `confidential_transfer_mint` as mutable

    let confidential_transfer_mint = mint.get_extension_mut::<ConfidentialTransferMint>()?;

    // basic checks for the destination account - must be extended for confidential transfers
    let mut destination_account_data = destination_account_info.data.borrow_mut();
    let mut destination_account =
        StateWithExtensionsMut::<Account>::unpack(&mut destination_account_data)?;

    if destination_account.base.mint != *mint_account_info.key {
        return Err(TokenError::MintMismatch.into());
    }
    if destination_account.base.is_frozen() {
        return Err(TokenError::AccountFrozen.into());
    }
    let mut destination_confidential_transfer_account =
        destination_account.get_extension_mut::<ConfidentialTransferAccount>()?;
    destination_confidential_transfer_account.valid_as_destination()?;

    // Zero-knowledge proof certifies that the exact withheld amount is credited to the source
    // account.
    let zkp_instruction =
        get_instruction_relative(proof_instruction_offset, instructions_sysvar_info)?;
    let proof_data = decode_proof_instruction::<WithdrawWithheldTokensData>(
        ProofInstruction::VerifyWithdrawWithheldTokens,
        &zkp_instruction,
    )?;
    // Checks that the withdraw authority encryption public key associated with the mint is
    // consistent with what was actually used to generate the zkp.
    if !confidential_transfer_mint
        .withdraw_withheld_authority_encryption_pubkey
        .equals(&proof_data.withdraw_withheld_authority_pubkey)
    {
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }
    // Checks that the encryption public key associated with the destination account is consistent
    // with what was actually used to generate the zkp.
    if proof_data.destination_pubkey != destination_confidential_transfer_account.encryption_pubkey
    {
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }
    // Checks that the withheld amount ciphertext is consistent with the ciphertext data that was
    // actually used to generate the zkp.
    if proof_data.withdraw_withheld_authority_ciphertext
        != confidential_transfer_mint.withheld_amount
    {
        return Err(TokenError::ConfidentialTransferBalanceMismatch.into());
    }

    // The proof data contains the mint withheld amount encrypted under the destination ElGamal pubkey.
    // This amount is added to the destination pending balance.
    destination_confidential_transfer_account.pending_balance_lo = syscall::add(
        &destination_confidential_transfer_account.pending_balance_lo,
        &proof_data.destination_ciphertext,
    )
    .ok_or(ProgramError::InvalidInstructionData)?;

    destination_confidential_transfer_account.increment_pending_balance_credit_counter()?;

    // Fee is now withdrawn, so zero out the mint withheld amount.
    confidential_transfer_mint.withheld_amount = EncryptedWithheldAmount::zeroed();

    Ok(())
}

/// Processes an [WithdrawWithheldTokensFromAccounts] instruction.
#[cfg(all(feature = "zk-ops", feature = "proof-program"))]
fn process_withdraw_withheld_tokens_from_accounts(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    num_token_accounts: u8,
    proof_instruction_offset: i64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;
    let destination_account_info = next_account_info(account_info_iter)?;
    let instructions_sysvar_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let authority_info_data_len = authority_info.data_len();
    let account_infos = account_info_iter.as_slice();
    let num_signers = account_infos
        .len()
        .saturating_sub(num_token_accounts as usize);

    // unnecessary check, but helps for clarity
    check_program_account(mint_account_info.owner)?;
    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = StateWithExtensionsMut::<Mint>::unpack(&mut mint_data)?;

    // mint must be extended for fees
    let transfer_fee_config = mint.get_extension::<TransferFeeConfig>()?;
    let withdraw_withheld_authority =
        Option::<Pubkey>::from(transfer_fee_config.withdraw_withheld_authority)
            .ok_or(TokenError::NoAuthorityExists)?;
    Processor::validate_owner(
        program_id,
        &withdraw_withheld_authority,
        authority_info,
        authority_info_data_len,
        &account_infos[..num_signers],
    )?;

    let mut destination_account_data = destination_account_info.data.borrow_mut();
    let mut destination_account =
        StateWithExtensionsMut::<Account>::unpack(&mut destination_account_data)?;
    if destination_account.base.mint != *mint_account_info.key {
        return Err(TokenError::MintMismatch.into());
    }
    if destination_account.base.is_frozen() {
        return Err(TokenError::AccountFrozen.into());
    }

    // Sum up the withheld amounts in all the accounts.
    let mut aggregate_withheld_amount = EncryptedWithheldAmount::zeroed();
    for account_info in &account_infos[num_signers..] {
        // self-harvest, can't double-borrow the underlying data
        if account_info.key == destination_account_info.key {
            let confidential_transfer_destination_account = destination_account
                .get_extension_mut::<ConfidentialTransferAccount>()
                .map_err(|_| TokenError::InvalidState)?;

            aggregate_withheld_amount = syscall::add(
                &aggregate_withheld_amount,
                &confidential_transfer_destination_account.withheld_amount,
            )
            .ok_or(ProgramError::InvalidInstructionData)?;

            confidential_transfer_destination_account.withheld_amount =
                EncryptedWithheldAmount::zeroed();
        } else {
            match harvest_from_account(mint_account_info.key, account_info) {
                Ok(encrypted_withheld_amount) => {
                    aggregate_withheld_amount =
                        syscall::add(&aggregate_withheld_amount, &encrypted_withheld_amount)
                            .ok_or(ProgramError::InvalidInstructionData)?;
                }
                Err(e) => {
                    msg!("Error harvesting from {}: {}", account_info.key, e);
                }
            }
        }
    }

    let mut destination_confidential_transfer_account =
        destination_account.get_extension_mut::<ConfidentialTransferAccount>()?;
    destination_confidential_transfer_account.valid_as_destination()?;

    // Zero-knowledge proof certifies that the exact aggregate withheld amount is credited to the
    // source account.
    let zkp_instruction =
        get_instruction_relative(proof_instruction_offset, instructions_sysvar_info)?;
    let proof_data = decode_proof_instruction::<WithdrawWithheldTokensData>(
        ProofInstruction::VerifyWithdrawWithheldTokens,
        &zkp_instruction,
    )?;
    // Checks that the withdraw authority encryption public key associated with the mint is
    // consistent with what was actually used to generate the zkp.
    let confidential_transfer_mint = mint.get_extension_mut::<ConfidentialTransferMint>()?;
    if !confidential_transfer_mint
        .withdraw_withheld_authority_encryption_pubkey
        .equals(&proof_data.withdraw_withheld_authority_pubkey)
    {
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }
    // Checks that the encryption public key associated with the destination account is consistent
    // with what was actually used to generate the zkp.
    if proof_data.destination_pubkey != destination_confidential_transfer_account.encryption_pubkey
    {
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }
    // Checks that the withheld amount ciphertext is consistent with the ciphertext data that was
    // actually used to generate the zkp.
    if proof_data.withdraw_withheld_authority_ciphertext != aggregate_withheld_amount {
        return Err(TokenError::ConfidentialTransferBalanceMismatch.into());
    }

    // The proof data contains the mint withheld amount encrypted under the destination ElGamal pubkey.
    // This amount is added to the destination pending balance.
    destination_confidential_transfer_account.pending_balance_lo = syscall::add(
        &destination_confidential_transfer_account.pending_balance_lo,
        &proof_data.destination_ciphertext,
    )
    .ok_or(ProgramError::InvalidInstructionData)?;

    destination_confidential_transfer_account.increment_pending_balance_credit_counter()?;

    Ok(())
}

#[cfg(feature = "zk-ops")]
fn harvest_from_account<'a, 'b>(
    mint_key: &'b Pubkey,
    token_account_info: &'b AccountInfo<'a>,
) -> Result<EncryptedWithheldAmount, TokenError> {
    let mut token_account_data = token_account_info.data.borrow_mut();
    let mut token_account = StateWithExtensionsMut::<Account>::unpack(&mut token_account_data)
        .map_err(|_| TokenError::InvalidState)?;
    if token_account.base.mint != *mint_key {
        return Err(TokenError::MintMismatch);
    }
    check_program_account(token_account_info.owner).map_err(|_| TokenError::InvalidState)?;

    let confidential_transfer_token_account = token_account
        .get_extension_mut::<ConfidentialTransferAccount>()
        .map_err(|_| TokenError::InvalidState)?;

    let withheld_amount = confidential_transfer_token_account.withheld_amount;
    confidential_transfer_token_account.withheld_amount = EncryptedWithheldAmount::zeroed();

    Ok(withheld_amount)
}

/// Processes an [HarvestWithheldTokensToMint] instruction.
#[cfg(feature = "zk-ops")]
fn process_harvest_withheld_tokens_to_mint(accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;
    let token_account_infos = account_info_iter.as_slice();

    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = StateWithExtensionsMut::<Mint>::unpack(&mut mint_data)?;
    mint.get_extension::<TransferFeeConfig>()?;
    let confidential_transfer_mint = mint.get_extension_mut::<ConfidentialTransferMint>()?;

    for token_account_info in token_account_infos {
        match harvest_from_account(mint_account_info.key, token_account_info) {
            Ok(withheld_amount) => {
                let new_mint_withheld_amount = syscall::add(
                    &confidential_transfer_mint.withheld_amount,
                    &withheld_amount,
                )
                .ok_or(ProgramError::InvalidInstructionData)?;

                confidential_transfer_mint.withheld_amount = new_mint_withheld_amount;
            }
            Err(e) => {
                msg!("Error harvesting from {}: {}", token_account_info.key, e);
            }
        }
    }
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
                &data.auditor_encryption_pubkey,
                &data.withdraw_withheld_authority_encryption_pubkey,
            )
        }
        ConfidentialTransferInstruction::UpdateMint => {
            msg!("ConfidentialTransferInstruction::UpdateMint");
            let data = decode_instruction_data::<UpdateMintData>(input)?;
            process_update_mint(
                accounts,
                data.auto_approve_new_accounts,
                &data.auditor_encryption_pubkey,
            )
        }
        ConfidentialTransferInstruction::ConfigureAccount => {
            msg!("ConfidentialTransferInstruction::ConfigureAccount");
            #[cfg(feature = "proof-program")]
            {
                let data = decode_instruction_data::<ConfigureAccountInstructionData>(input)?;
                process_configure_account(
                    program_id,
                    accounts,
                    &data.decryptable_zero_balance,
                    &data.maximum_pending_balance_credit_counter,
                    data.proof_instruction_offset as i64,
                )
            }
            #[cfg(not(feature = "proof-program"))]
            Err(ProgramError::InvalidInstructionData)
        }
        ConfidentialTransferInstruction::ApproveAccount => {
            msg!("ConfidentialTransferInstruction::ApproveAccount");
            process_approve_account(accounts)
        }
        ConfidentialTransferInstruction::EmptyAccount => {
            msg!("ConfidentialTransferInstruction::EmptyAccount");
            #[cfg(feature = "proof-program")]
            {
                let data = decode_instruction_data::<EmptyAccountInstructionData>(input)?;
                process_empty_account(program_id, accounts, data.proof_instruction_offset as i64)
            }
            #[cfg(not(feature = "proof-program"))]
            Err(ProgramError::InvalidInstructionData)
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
            #[cfg(all(feature = "zk-ops", feature = "proof-program"))]
            {
                let data = decode_instruction_data::<WithdrawInstructionData>(input)?;
                process_withdraw(
                    program_id,
                    accounts,
                    data.amount.into(),
                    data.decimals,
                    data.new_decryptable_available_balance,
                    data.proof_instruction_offset as i64,
                )
            }
            #[cfg(not(all(feature = "zk-ops", feature = "proof-program")))]
            Err(ProgramError::InvalidInstructionData)
        }
        ConfidentialTransferInstruction::Transfer => {
            msg!("ConfidentialTransferInstruction::Transfer");
            #[cfg(all(feature = "zk-ops", feature = "proof-program"))]
            {
                let data = decode_instruction_data::<TransferInstructionData>(input)?;
                process_transfer(
                    program_id,
                    accounts,
                    data.new_source_decryptable_available_balance,
                    data.proof_instruction_offset as i64,
                )
            }
            #[cfg(not(all(feature = "zk-ops", feature = "proof-program")))]
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
        ConfidentialTransferInstruction::WithdrawWithheldTokensFromMint => {
            msg!("ConfidentialTransferInstruction::WithdrawWithheldTokensFromMint");
            #[cfg(all(feature = "zk-ops", feature = "proof-program"))]
            {
                let data = decode_instruction_data::<WithdrawWithheldTokensFromMintData>(input)?;
                process_withdraw_withheld_tokens_from_mint(
                    program_id,
                    accounts,
                    data.proof_instruction_offset as i64,
                )
            }
            #[cfg(not(all(feature = "zk-ops", feature = "proof-program")))]
            Err(ProgramError::InvalidInstructionData)
        }
        ConfidentialTransferInstruction::WithdrawWithheldTokensFromAccounts => {
            msg!("ConfidentialTransferInstruction::WithdrawWithheldTokensFromAccounts");
            #[cfg(all(feature = "zk-ops", feature = "proof-program"))]
            {
                let data =
                    decode_instruction_data::<WithdrawWithheldTokensFromAccountsData>(input)?;
                process_withdraw_withheld_tokens_from_accounts(
                    program_id,
                    accounts,
                    data.num_token_accounts,
                    data.proof_instruction_offset as i64,
                )
            }
            #[cfg(not(all(feature = "zk-ops", feature = "proof-program")))]
            Err(ProgramError::InvalidInstructionData)
        }
        ConfidentialTransferInstruction::HarvestWithheldTokensToMint => {
            msg!("ConfidentialTransferInstruction::HarvestWithheldTokensToMint");
            #[cfg(feature = "zk-ops")]
            {
                process_harvest_withheld_tokens_to_mint(accounts)
            }
            #[cfg(not(feature = "zk-ops"))]
            {
                Err(ProgramError::InvalidInstructionData)
            }
        }
    }
}
