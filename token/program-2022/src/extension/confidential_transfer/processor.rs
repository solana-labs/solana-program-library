use {
    crate::{
        check_program_account,
        error::TokenError,
        extension::{
            confidential_transfer::{instruction::*, *},
            StateWithExtensions, StateWithExtensionsMut,
        },
        instruction::{decode_instruction_data, decode_instruction_type},
        processor::Processor,
        state::{Account, Mint},
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        instruction::Instruction,
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
        sysvar::instructions::get_instruction_relative,
    },
    solana_zk_token_sdk::zk_token_proof_program,
};
// Remove feature once zk ops syscalls are enabled on all networks
#[cfg(feature = "zk-ops")]
use {
    crate::extension::transfer_fee::TransferFeeConfig,
    solana_program::{clock::Clock, sysvar::Sysvar},
    solana_zk_token_sdk::zk_token_elgamal::ops,
};

fn decode_proof_instruction<T: Pod>(
    expected: ProofInstruction,
    instruction: &Instruction,
) -> Result<&T, ProgramError> {
    if ProofInstruction::decode_type(&instruction.data) != Some(expected) {
        msg!("decode type failed ----------------------------------");
    }

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
    confidential_transfer_mint: &ConfidentialTransferMint,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_info = next_account_info(account_info_iter)?;

    check_program_account(mint_info.owner)?;
    let mint_data = &mut mint_info.data.borrow_mut();
    let mut mint = StateWithExtensionsMut::<Mint>::unpack_uninitialized(mint_data)?;
    *mint.init_extension::<ConfidentialTransferMint>()? = *confidential_transfer_mint;

    Ok(())
}

/// Processes an [UpdateMint] instruction.
fn process_update_mint(
    accounts: &[AccountInfo],
    new_confidential_transfer_mint: &ConfidentialTransferMint,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let new_authority_info = next_account_info(account_info_iter)?;

    check_program_account(mint_info.owner)?;
    let mint_data = &mut mint_info.data.borrow_mut();
    let mut mint = StateWithExtensionsMut::<Mint>::unpack(mint_data)?;
    let confidential_transfer_mint = mint.get_extension_mut::<ConfidentialTransferMint>()?;

    if authority_info.is_signer
        && confidential_transfer_mint.authority == *authority_info.key
        && (new_authority_info.is_signer || *new_authority_info.key == Pubkey::default())
        && new_confidential_transfer_mint.authority == *new_authority_info.key
    {
        *confidential_transfer_mint = *new_confidential_transfer_mint;
        Ok(())
    } else {
        Err(ProgramError::MissingRequiredSignature)
    }
}

/// Processes a [ConfigureAccount] instruction.
fn process_configure_account(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    ConfigureAccountInstructionData {
        encryption_pubkey,
        decryptable_zero_balance,
    }: &ConfigureAccountInstructionData,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let token_account_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
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

    // Note: The caller is expected to use the `Reallocate` instruction to ensure there is
    // sufficient room in their token account for the new `ConfidentialTransferAccount` extension
    let mut confidential_transfer_account =
        token_account.init_extension::<ConfidentialTransferAccount>()?;
    confidential_transfer_account.approved = confidential_transfer_mint.auto_approve_new_accounts;
    confidential_transfer_account.encryption_pubkey = *encryption_pubkey;

    /*
        An ElGamal ciphertext is of the form
          ElGamalCiphertext {
            msg_comm: r * H + x * G
            decrypt_handle: r * P
          }

        where
        - G, H: constants for the system (RistrettoPoint)
        - P: ElGamal public key component (RistrettoPoint)
        - r: encryption randomness (Scalar)
        - x: message (Scalar)

        Upon receiving a `ConfigureAccount` instruction, the ZK Token program should encrypt x=0 (i.e.
        Scalar::zero()) and store it as `pending_balance` and `available_balance`.

        For regular encryption, it is important that r is generated from a proper randomness source. But
        for the `ConfigureAccount` instruction, it is already known that x is always 0. So r can just be
        set Scalar::zero().

        This means that the ElGamalCiphertext should simply be
          ElGamalCiphertext {
            msg_comm: 0 * H + 0 * G = 0
            decrypt_handle: 0 * P = 0
          }

        This should just be encoded as [0; 64]
    */
    confidential_transfer_account.pending_balance = EncryptedBalance::zeroed();
    confidential_transfer_account.available_balance = EncryptedBalance::zeroed();

    confidential_transfer_account.decryptable_available_balance = *decryptable_zero_balance;
    confidential_transfer_account.allow_balance_credits = true.into();
    confidential_transfer_account.pending_balance_credit_counter = 0.into();
    confidential_transfer_account.expected_pending_balance_credit_counter = 0.into();
    confidential_transfer_account.actual_pending_balance_credit_counter = 0.into();
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

    if authority_info.is_signer && *authority_info.key == confidential_transfer_mint.authority {
        let mut confidential_transfer_state =
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

    let previous_instruction =
        get_instruction_relative(proof_instruction_offset, instructions_sysvar_info)?;
    let proof_data = decode_proof_instruction::<CloseAccountData>(
        ProofInstruction::VerifyCloseAccount,
        &previous_instruction,
    )?;

    if confidential_transfer_account.pending_balance != EncryptedBalance::zeroed() {
        msg!("Pending balance is not zero");
        return Err(ProgramError::InvalidAccountData);
    }

    if confidential_transfer_account.available_balance != proof_data.ciphertext {
        msg!("Available balance mismatch");
        return Err(ProgramError::InvalidInstructionData);
    }

    confidential_transfer_account.available_balance = EncryptedBalance::zeroed();
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
    let destination_token_account_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let authority_info_data_len = authority_info.data_len();

    check_program_account(mint_info.owner)?;
    let mint_data = &mint_info.data.borrow_mut();
    let mint = StateWithExtensions::<Mint>::unpack(mint_data)?;

    if expected_decimals != mint.base.decimals {
        return Err(TokenError::MintDecimalsMismatch.into());
    }

    // Process source account
    {
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
    }

    //
    // Finished with the source token account at this point. Drop all references to it to avoid a
    // double borrow if the source and destination accounts are the same
    //

    // Process destination account
    {
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

        let mut destination_confidential_transfer_account =
            destination_token_account.get_extension_mut::<ConfidentialTransferAccount>()?;
        destination_confidential_transfer_account.approved()?;

        if !bool::from(&destination_confidential_transfer_account.allow_balance_credits) {
            return Err(TokenError::ConfidentialTransferDepositsAndTransfersDisabled.into());
        }

        destination_confidential_transfer_account.pending_balance = ops::add_to(
            &destination_confidential_transfer_account.pending_balance,
            amount,
        )
        .ok_or(ProgramError::InvalidInstructionData)?;

        destination_confidential_transfer_account.pending_balance_credit_counter =
            (u64::from(destination_confidential_transfer_account.pending_balance_credit_counter)
                .checked_add(1)
                .ok_or(ProgramError::InvalidInstructionData)?)
            .into();
    }

    Ok(())
}

/// Processes a [Withdraw] instruction.
#[cfg(feature = "zk-ops")]
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
    let destination_token_account_info = next_account_info(account_info_iter)?;
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

    let previous_instruction =
        get_instruction_relative(proof_instruction_offset, instructions_sysvar_info)?;

    let proof_data = decode_proof_instruction::<WithdrawData>(
        ProofInstruction::VerifyWithdraw,
        &previous_instruction,
    )?;

    // Process source account
    {
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

        let mut confidential_transfer_account =
            token_account.get_extension_mut::<ConfidentialTransferAccount>()?;

        confidential_transfer_account.available_balance =
            ops::subtract_from(&confidential_transfer_account.available_balance, amount)
                .ok_or(ProgramError::InvalidInstructionData)?;

        if confidential_transfer_account.available_balance != proof_data.final_ciphertext {
            return Err(TokenError::ConfidentialTransferBalanceMismatch.into());
        }

        confidential_transfer_account.decryptable_available_balance =
            new_decryptable_available_balance;
    }

    //
    // Finished with the source token account at this point. Drop all references to it to avoid a
    // double borrow if the source and destination accounts are the same
    //

    // Process destination account
    {
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

        // Wrapped SOL withdrawals are not supported because lamports cannot be apparated.
        assert!(!destination_token_account.base.is_native());
        destination_token_account.base.amount = destination_token_account
            .base
            .amount
            .checked_add(amount)
            .ok_or(TokenError::Overflow)?;

        destination_token_account.pack_base();
    }

    Ok(())
}

/// Processes an [Transfer] instruction.
#[cfg(feature = "zk-ops")]
fn process_transfer(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_source_decryptable_available_balance: DecryptableBalance,
    proof_instruction_offset: i64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let token_account_info = next_account_info(account_info_iter)?;
    let destination_token_account_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let instructions_sysvar_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;

    check_program_account(mint_info.owner)?;
    let mint_data = &mint_info.data.borrow_mut();
    let mint = StateWithExtensions::<Mint>::unpack(mint_data)?;
    let confidential_transfer_mint = mint.get_extension::<ConfidentialTransferMint>()?;

    let previous_instruction =
        get_instruction_relative(proof_instruction_offset, instructions_sysvar_info)?;

    if let Ok(transfer_fee_config) = mint.get_extension::<TransferFeeConfig>() {
        // mint is extended for fees
        let proof_data = decode_proof_instruction::<TransferWithFeeData>(
            ProofInstruction::VerifyTransferWithFee,
            &previous_instruction,
        )?;

        if proof_data.transfer_with_fee_pubkeys.auditor_pubkey
            != confidential_transfer_mint.auditor_pubkey
        {
            return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
        }

        // `withdraw_withheld_authority` ElGamal pubkey in proof data and mint must match
        if proof_data
            .transfer_with_fee_pubkeys
            .withdraw_withheld_authority_pubkey
            != confidential_transfer_mint.withdraw_withheld_authority_pubkey
        {
            return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
        }

        // fee parameters in proof data and mint must match
        let epoch = Clock::get()?.epoch;
        let (maximum_fee, transfer_fee_basis_points) =
            if u64::from(transfer_fee_config.newer_transfer_fee.epoch) < epoch {
                (
                    u64::from(transfer_fee_config.older_transfer_fee.maximum_fee),
                    u16::from(
                        transfer_fee_config
                            .older_transfer_fee
                            .transfer_fee_basis_points,
                    ),
                )
            } else {
                (
                    u64::from(transfer_fee_config.newer_transfer_fee.maximum_fee),
                    u16::from(
                        transfer_fee_config
                            .newer_transfer_fee
                            .transfer_fee_basis_points,
                    ),
                )
            };

        if u64::from(proof_data.fee_parameters.maximum_fee) != maximum_fee
            || u16::from(proof_data.fee_parameters.fee_rate_basis_points)
                != transfer_fee_basis_points
        {
            return Err(TokenError::FeeParametersMismatch.into());
        }

        let ciphertext_lo = EncryptedBalance::from((
            proof_data.ciphertext_lo.commitment,
            proof_data.ciphertext_lo.source_handle,
        ));
        let ciphertext_hi = EncryptedBalance::from((
            proof_data.ciphertext_hi.commitment,
            proof_data.ciphertext_hi.source_handle,
        ));

        process_source_for_transfer(
            program_id,
            token_account_info,
            mint_info,
            authority_info,
            account_info_iter.as_slice(),
            &proof_data.transfer_with_fee_pubkeys.source_pubkey,
            &ciphertext_lo,
            &ciphertext_hi,
            new_source_decryptable_available_balance,
        )?;
        process_destination_for_transfer(
            destination_token_account_info,
            mint_info,
            &proof_data.transfer_with_fee_pubkeys.destination_pubkey,
            &ciphertext_lo,
            &ciphertext_hi,
            Some(proof_data.fee_ciphertext),
        )?;
    } else {
        // mint is not extended for fees
        let proof_data = decode_proof_instruction::<TransferData>(
            ProofInstruction::VerifyTransfer,
            &previous_instruction,
        )?;

        if proof_data.transfer_pubkeys.auditor_pubkey != confidential_transfer_mint.auditor_pubkey {
            return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
        }

        let ciphertext_lo = EncryptedBalance::from((
            proof_data.ciphertext_lo.commitment,
            proof_data.ciphertext_lo.source_handle,
        ));
        let ciphertext_hi = EncryptedBalance::from((
            proof_data.ciphertext_hi.commitment,
            proof_data.ciphertext_hi.source_handle,
        ));

        process_source_for_transfer(
            program_id,
            token_account_info,
            mint_info,
            authority_info,
            account_info_iter.as_slice(),
            &proof_data.transfer_pubkeys.source_pubkey,
            &ciphertext_lo,
            &ciphertext_hi,
            new_source_decryptable_available_balance,
        )?;

        process_destination_for_transfer(
            destination_token_account_info,
            mint_info,
            &proof_data.transfer_pubkeys.destination_pubkey,
            &ciphertext_lo,
            &ciphertext_hi,
            None,
        )?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[cfg(feature = "zk-ops")]
fn process_source_for_transfer(
    program_id: &Pubkey,
    token_account_info: &AccountInfo,
    mint_info: &AccountInfo,
    authority_info: &AccountInfo,
    signers: &[AccountInfo],
    source_encryption_pubkey: &EncryptionPubkey,
    source_ciphertext_lo: &EncryptedBalance,
    source_ciphertext_hi: &EncryptedBalance,
    new_source_decryptable_available_balance: DecryptableBalance,
) -> ProgramResult {
    check_program_account(token_account_info.owner)?;
    let authority_info_data_len = authority_info.data_len();
    let token_account_data = &mut token_account_info.data.borrow_mut();
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
    confidential_transfer_account.approved()?;
    if *source_encryption_pubkey != confidential_transfer_account.encryption_pubkey {
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }

    let new_source_available_balance = {
        ops::subtract_with_lo_hi(
            &confidential_transfer_account.available_balance,
            source_ciphertext_lo,
            source_ciphertext_hi,
        )
        .ok_or(ProgramError::InvalidInstructionData)?
    };

    confidential_transfer_account.available_balance = new_source_available_balance;
    confidential_transfer_account.decryptable_available_balance =
        new_source_decryptable_available_balance;

    Ok(())
}

#[cfg(feature = "zk-ops")]
fn process_destination_for_transfer(
    destination_token_account_info: &AccountInfo,
    mint_info: &AccountInfo,
    destination_encryption_pubkey: &EncryptionPubkey,
    destination_ciphertext_lo: &EncryptedBalance,
    destination_ciphertext_hi: &EncryptedBalance,
    encrypted_fee: Option<EncryptedFee>,
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

    let mut destination_confidential_transfer_account =
        destination_token_account.get_extension_mut::<ConfidentialTransferAccount>()?;
    destination_confidential_transfer_account.approved()?;

    if !bool::from(&destination_confidential_transfer_account.allow_balance_credits) {
        return Err(TokenError::ConfidentialTransferDepositsAndTransfersDisabled.into());
    }

    if *destination_encryption_pubkey != destination_confidential_transfer_account.encryption_pubkey
    {
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }

    let new_destination_pending_balance = ops::add_with_lo_hi(
        &destination_confidential_transfer_account.pending_balance,
        destination_ciphertext_lo,
        destination_ciphertext_hi,
    )
    .ok_or(ProgramError::InvalidInstructionData)?;

    let new_destination_pending_balance_credit_counter =
        (u64::from(destination_confidential_transfer_account.pending_balance_credit_counter) + 1)
            .into();

    destination_confidential_transfer_account.pending_balance = new_destination_pending_balance;
    destination_confidential_transfer_account.pending_balance_credit_counter =
        new_destination_pending_balance_credit_counter;

    // update destination account withheld fees
    if let Some(ciphertext_fee) = encrypted_fee {
        let ciphertext_fee_destination: EncryptedWithheldAmount =
            (ciphertext_fee.commitment, ciphertext_fee.destination_handle).into();
        let ciphertext_fee_withheld_authority: EncryptedWithheldAmount = (
            ciphertext_fee.commitment,
            ciphertext_fee.withdraw_withheld_authority_handle,
        )
            .into();

        // subtract fee from destination pending balance
        let new_destination_pending_balance = ops::subtract(
            &destination_confidential_transfer_account.pending_balance,
            &ciphertext_fee_destination,
        )
        .ok_or(ProgramError::InvalidInstructionData)?;

        // add encrypted fee to current withheld fee
        let new_withheld_amount = ops::add(
            &destination_confidential_transfer_account.withheld_amount,
            &ciphertext_fee_withheld_authority,
        )
        .ok_or(ProgramError::InvalidInstructionData)?;

        destination_confidential_transfer_account.pending_balance = new_destination_pending_balance;
        destination_confidential_transfer_account.withheld_amount = new_withheld_amount;
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

    confidential_transfer_account.available_balance = ops::add(
        &confidential_transfer_account.available_balance,
        &confidential_transfer_account.pending_balance,
    )
    .ok_or(ProgramError::InvalidInstructionData)?;

    confidential_transfer_account.actual_pending_balance_credit_counter =
        confidential_transfer_account.pending_balance_credit_counter;
    confidential_transfer_account.expected_pending_balance_credit_counter =
        *expected_pending_balance_credit_counter;
    confidential_transfer_account.decryptable_available_balance =
        *new_decryptable_available_balance;
    confidential_transfer_account.pending_balance = EncryptedBalance::zeroed();

    Ok(())
}

/// Processes an [DisableBalanceCredits] or [EnableBalanceCredits] instruction.
fn process_allow_balance_credits(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    allow_balance_credits: bool,
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
    confidential_transfer_account.allow_balance_credits = allow_balance_credits.into();

    Ok(())
}

/// Processes an [WithdrawWithheldTokensFromMint] instruction.
#[cfg(feature = "zk-ops")]
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
    destination_confidential_transfer_account.approved()?;
    // verify consistency of proof data
    let previous_instruction =
        get_instruction_relative(proof_instruction_offset, instructions_sysvar_info)?;
    let proof_data = decode_proof_instruction::<WithdrawWithheldTokensData>(
        ProofInstruction::VerifyWithdrawWithheldTokens,
        &previous_instruction,
    )?;

    // withdraw withheld authority ElGamal pubkey should match in the proof data and mint
    if proof_data.withdraw_withheld_authority_pubkey
        != confidential_transfer_mint.withdraw_withheld_authority_pubkey
    {
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }

    // destination ElGamal pubkey should match in the proof data and destination account
    if proof_data.destination_pubkey != destination_confidential_transfer_account.encryption_pubkey
    {
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }

    // withheld amount ciphertext must match in the proof data and mint
    if proof_data.withdraw_withheld_authority_ciphertext
        != confidential_transfer_mint.withheld_amount
    {
        return Err(TokenError::ConfidentialTransferBalanceMismatch.into());
    }

    // The proof data contains the mint withheld amount encrypted under the destination ElGamal pubkey.
    // This amount should be added to the destination pending balance.
    let new_destination_pending_balance = ops::add(
        &destination_confidential_transfer_account.pending_balance,
        &proof_data.destination_ciphertext,
    )
    .ok_or(ProgramError::InvalidInstructionData)?;

    destination_confidential_transfer_account.pending_balance = new_destination_pending_balance;

    // fee is now withdrawn, so zero out mint withheld amount
    confidential_transfer_mint.withheld_amount = EncryptedWithheldAmount::zeroed();

    Ok(())
}

#[cfg(feature = "zk-ops")]
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

    // sum up the withheld amounts in all the accounts
    let mut aggregate_withheld_amount = EncryptedWithheldAmount::zeroed();
    for account_info in &account_infos[num_signers..] {
        // self-harvest, can't double-borrow the underlying data
        if account_info.key == destination_account_info.key {
            let confidential_transfer_destination_account = destination_account
                .get_extension_mut::<ConfidentialTransferAccount>()
                .map_err(|_| TokenError::InvalidState)?;

            aggregate_withheld_amount = ops::add(
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
                        ops::add(&aggregate_withheld_amount, &encrypted_withheld_amount)
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
    destination_confidential_transfer_account.approved()?;
    // verify consistency of proof data
    let previous_instruction =
        get_instruction_relative(proof_instruction_offset, instructions_sysvar_info)?;
    let proof_data = decode_proof_instruction::<WithdrawWithheldTokensData>(
        ProofInstruction::VerifyWithdrawWithheldTokens,
        &previous_instruction,
    )?;

    // withdraw withheld authority ElGamal pubkey should match in the proof data and mint
    let confidential_transfer_mint = mint.get_extension_mut::<ConfidentialTransferMint>()?;
    if proof_data.withdraw_withheld_authority_pubkey
        != confidential_transfer_mint.withdraw_withheld_authority_pubkey
    {
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }

    // destination ElGamal pubkey should match in the proof data and destination account
    if proof_data.destination_pubkey != destination_confidential_transfer_account.encryption_pubkey
    {
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }

    // withheld amount ciphertext must match in the proof data and mint
    if proof_data.withdraw_withheld_authority_ciphertext != aggregate_withheld_amount {
        return Err(TokenError::ConfidentialTransferBalanceMismatch.into());
    }

    // add the sum of the withheld fees to destination pending balance
    let new_destination_pending_balance = ops::add(
        &destination_confidential_transfer_account.pending_balance,
        &aggregate_withheld_amount,
    )
    .ok_or(ProgramError::InvalidInstructionData)?;

    destination_confidential_transfer_account.pending_balance = new_destination_pending_balance;

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
                let new_mint_withheld_amount = ops::add(
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
            process_initialize_mint(
                accounts,
                decode_instruction_data::<ConfidentialTransferMint>(input)?,
            )
        }
        ConfidentialTransferInstruction::UpdateMint => {
            msg!("ConfidentialTransferInstruction::UpdateMint");
            process_update_mint(
                accounts,
                decode_instruction_data::<ConfidentialTransferMint>(input)?,
            )
        }
        ConfidentialTransferInstruction::ConfigureAccount => {
            msg!("ConfidentialTransferInstruction::ConfigureAccount");
            process_configure_account(
                program_id,
                accounts,
                decode_instruction_data::<ConfigureAccountInstructionData>(input)?,
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
                    data.proof_instruction_offset as i64,
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
                    data.proof_instruction_offset as i64,
                )
            }
            #[cfg(not(feature = "zk-ops"))]
            Err(ProgramError::InvalidInstructionData)
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
                    data.proof_instruction_offset as i64,
                )
            }
            #[cfg(not(feature = "zk-ops"))]
            {
                Err(ProgramError::InvalidInstructionData)
            }
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
        ConfidentialTransferInstruction::DisableBalanceCredits => {
            msg!("ConfidentialTransferInstruction::DisableBalanceCredits");
            process_allow_balance_credits(program_id, accounts, false)
        }
        ConfidentialTransferInstruction::EnableBalanceCredits => {
            msg!("ConfidentialTransferInstruction::EnableBalanceCredits");
            process_allow_balance_credits(program_id, accounts, true)
        }
        ConfidentialTransferInstruction::WithdrawWithheldTokensFromMint => {
            msg!("ConfidentialTransferInstruction::WithdrawWithheldTokensFromMint");
            #[cfg(feature = "zk-ops")]
            {
                let data = decode_instruction_data::<WithdrawWithheldTokensFromMintData>(input)?;
                process_withdraw_withheld_tokens_from_mint(
                    program_id,
                    accounts,
                    data.proof_instruction_offset as i64,
                )
            }
            #[cfg(not(feature = "zk-ops"))]
            Err(ProgramError::InvalidInstructionData)
        }
        ConfidentialTransferInstruction::WithdrawWithheldTokensFromAccounts => {
            msg!("ConfidentialTransferInstruction::WithdrawWithheldTokensFromAccounts");
            #[cfg(feature = "zk-ops")]
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
            #[cfg(not(feature = "zk-ops"))]
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
