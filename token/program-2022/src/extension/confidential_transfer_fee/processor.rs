use {
    crate::{
        check_program_account,
        error::TokenError,
        extension::{
            confidential_transfer_fee::{
                instruction::{
                    ConfidentialTransferFeeInstruction, InitializeConfidentialTransferFeeConfigData,
                },
                ConfidentialTransferFeeAmount, ConfidentialTransferFeeConfig,
                EncryptedWithheldAmount,
            },
            transfer_fee::TransferFeeConfig,
            BaseStateWithExtensions, StateWithExtensionsMut,
        },
        instruction::{decode_instruction_data, decode_instruction_type},
        pod::{EncryptionPubkey, OptionalNonZeroPubkey},
        state::{Account, Mint},
    },
    bytemuck::Zeroable,
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
use solana_zk_token_sdk::zk_token_elgamal::ops as syscall;

#[cfg(feature = "proof-program")]
use {
    crate::{
        extension::{
            confidential_transfer::{
                instruction::{ProofInstruction, WithdrawWithheldTokensData},
                processor::decode_proof_instruction,
                ConfidentialTransferAccount, ConfidentialTransferMint,
            },
            confidential_transfer_fee::instruction::{
                WithdrawWithheldTokensFromAccountsData, WithdrawWithheldTokensFromMintData,
            },
        },
        processor::Processor,
    },
    solana_program::sysvar::instructions::get_instruction_relative,
};

/// Processes an [InitializeConfidentialTransferFeeConfig] instruction.
fn process_initialize_confidential_transfer_fee_config(
    accounts: &[AccountInfo],
    authority: &OptionalNonZeroPubkey,
    withdraw_withheld_authority_encryption_pubkey: &EncryptionPubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;

    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut mint_data)?;
    let extension = mint.init_extension::<ConfidentialTransferFeeConfig>(true)?;
    extension.authority = *authority;
    extension.withdraw_withheld_authority_encryption_pubkey =
        *withdraw_withheld_authority_encryption_pubkey;
    extension.withheld_amount = EncryptedWithheldAmount::zeroed();

    Ok(())
}

/// Processes a [WithdrawWithheldTokensFromMint] instruction.
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
    } // free `transfer_fee_config` to borrow `confidential_transfer_fee_config` as mutable

    // mint must also be extended for confidential transfers, but forgo an explicit check since it
    // is not possible to initialize a confidential transfer mint without it

    let confidential_transfer_fee_config =
        mint.get_extension_mut::<ConfidentialTransferFeeConfig>()?;

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

    // Zero-knowledge proof certifies that the exact withheld amount is credited to the destination
    // account.
    let zkp_instruction =
        get_instruction_relative(proof_instruction_offset, instructions_sysvar_info)?;
    let proof_data = decode_proof_instruction::<WithdrawWithheldTokensData>(
        ProofInstruction::VerifyWithdrawWithheldTokens,
        &zkp_instruction,
    )?;
    // Check that the withdraw authority encryption public key associated with the mint is
    // consistent with what was actually used to generate the zkp.
    if proof_data.withdraw_withheld_authority_pubkey
        != confidential_transfer_fee_config.withdraw_withheld_authority_encryption_pubkey
    {
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }
    // Check that the encryption public key associated with the destination account is consistent
    // with what was actually used to generate the zkp.
    if proof_data.destination_pubkey != destination_confidential_transfer_account.encryption_pubkey
    {
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }
    // Check that the withheld amount ciphertext is consistent with the ciphertext data that was
    // actually used to generate the zkp.
    if proof_data.withdraw_withheld_authority_ciphertext
        != confidential_transfer_fee_config.withheld_amount
    {
        return Err(TokenError::ConfidentialTransferBalanceMismatch.into());
    }

    // The proof data contains the mint withheld amount encrypted under the destination ElGamal pubkey.
    // Add this amount to the destination pending balance.
    destination_confidential_transfer_account.pending_balance_lo = syscall::add(
        &destination_confidential_transfer_account.pending_balance_lo,
        &proof_data.destination_ciphertext,
    )
    .ok_or(ProgramError::InvalidInstructionData)?;

    destination_confidential_transfer_account.increment_pending_balance_credit_counter()?;

    // Fee is now withdrawn, so zero out the mint withheld amount.
    confidential_transfer_fee_config.withheld_amount = EncryptedWithheldAmount::zeroed();

    Ok(())
}

/// Processes a [WithdrawWithheldTokensFromAccounts] instruction.
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
            let destination_confidential_transfer_fee_amount = destination_account
                .get_extension_mut::<ConfidentialTransferFeeAmount>()
                .map_err(|_| TokenError::InvalidState)?;

            aggregate_withheld_amount = syscall::add(
                &aggregate_withheld_amount,
                &destination_confidential_transfer_fee_amount.withheld_amount,
            )
            .ok_or(ProgramError::InvalidInstructionData)?;

            destination_confidential_transfer_fee_amount.withheld_amount =
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
    let confidential_transfer_fee_config =
        mint.get_extension_mut::<ConfidentialTransferFeeConfig>()?;
    if proof_data.withdraw_withheld_authority_pubkey
        != confidential_transfer_fee_config.withdraw_withheld_authority_encryption_pubkey
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
fn harvest_from_account<'b>(
    mint_key: &'b Pubkey,
    token_account_info: &'b AccountInfo<'_>,
) -> Result<EncryptedWithheldAmount, TokenError> {
    let mut token_account_data = token_account_info.data.borrow_mut();
    let mut token_account = StateWithExtensionsMut::<Account>::unpack(&mut token_account_data)
        .map_err(|_| TokenError::InvalidState)?;
    if token_account.base.mint != *mint_key {
        return Err(TokenError::MintMismatch);
    }
    check_program_account(token_account_info.owner).map_err(|_| TokenError::InvalidState)?;

    let confidential_transfer_token_account = token_account
        .get_extension_mut::<ConfidentialTransferFeeAmount>()
        .map_err(|_| TokenError::InvalidState)?;

    let withheld_amount = confidential_transfer_token_account.withheld_amount;
    confidential_transfer_token_account.withheld_amount = EncryptedWithheldAmount::zeroed();

    Ok(withheld_amount)
}

/// Process a [HarvestWithheldTokensToMint] instruction.
#[cfg(feature = "zk-ops")]
fn process_harvest_withheld_tokens_to_mint(accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;
    let token_account_infos = account_info_iter.as_slice();

    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = StateWithExtensionsMut::<Mint>::unpack(&mut mint_data)?;
    mint.get_extension::<TransferFeeConfig>()?;
    let confidential_transfer_fee_mint =
        mint.get_extension_mut::<ConfidentialTransferFeeConfig>()?;

    for token_account_info in token_account_infos {
        match harvest_from_account(mint_account_info.key, token_account_info) {
            Ok(withheld_amount) => {
                let new_mint_withheld_amount = syscall::add(
                    &confidential_transfer_fee_mint.withheld_amount,
                    &withheld_amount,
                )
                .ok_or(ProgramError::InvalidInstructionData)?;

                confidential_transfer_fee_mint.withheld_amount = new_mint_withheld_amount;
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
        ConfidentialTransferFeeInstruction::InitializeConfidentialTransferFeeConfig => {
            msg!("ConfidentialTransferInstruction::InitializeConfidentialTransferFeeConfig");
            let data =
                decode_instruction_data::<InitializeConfidentialTransferFeeConfigData>(input)?;
            process_initialize_confidential_transfer_fee_config(
                accounts,
                &data.authority,
                &data.withdraw_withheld_authority_encryption_pubkey,
            )
        }
        ConfidentialTransferFeeInstruction::WithdrawWithheldTokensFromMint => {
            msg!("ConfidentialTransferInstruction::WithdrawWithheldTokensFromMint");
            #[cfg(all(feature = "zk-ops", feature = "proof-program"))]
            {
                let data = decode_instruction_data::<WithdrawWithheldTokensFromMintData>(input)?;
                return process_withdraw_withheld_tokens_from_mint(
                    program_id,
                    accounts,
                    data.proof_instruction_offset as i64,
                );
            }
            #[cfg(not(all(feature = "zk-ops", feature = "proof_program")))]
            {
                Err(ProgramError::InvalidInstructionData)
            }
        }
        ConfidentialTransferFeeInstruction::WithdrawWithheldTokensFromAccounts => {
            msg!("ConfidentialTransferInstruction::WithdrawWithheldTokensFromAccounts");
            #[cfg(all(feature = "zk-ops", feature = "proof-program"))]
            {
                let data =
                    decode_instruction_data::<WithdrawWithheldTokensFromAccountsData>(input)?;
                return process_withdraw_withheld_tokens_from_accounts(
                    program_id,
                    accounts,
                    data.num_token_accounts,
                    data.proof_instruction_offset as i64,
                );
            }
            #[cfg(not(all(feature = "zk-ops", feature = "proof_program")))]
            {
                Err(ProgramError::InvalidInstructionData)
            }
        }
        ConfidentialTransferFeeInstruction::HarvestWithheldTokensToMint => {
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
