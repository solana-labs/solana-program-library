// Remove feature once zk ops syscalls are enabled on all networks
#[cfg(feature = "zk-ops")]
use spl_token_confidential_transfer_ciphertext_arithmetic as ciphertext_arithmetic;
use {
    crate::{
        check_program_account,
        error::TokenError,
        extension::{
            confidential_transfer::{
                instruction::{
                    CiphertextCiphertextEqualityProofContext, CiphertextCiphertextEqualityProofData,
                },
                ConfidentialTransferAccount, DecryptableBalance,
            },
            confidential_transfer_fee::{
                instruction::{
                    ConfidentialTransferFeeInstruction,
                    InitializeConfidentialTransferFeeConfigData,
                    WithdrawWithheldTokensFromAccountsData, WithdrawWithheldTokensFromMintData,
                },
                ConfidentialTransferFeeAmount, ConfidentialTransferFeeConfig,
                EncryptedWithheldAmount,
            },
            transfer_fee::TransferFeeConfig,
            BaseStateWithExtensions, BaseStateWithExtensionsMut, PodStateWithExtensionsMut,
        },
        instruction::{decode_instruction_data, decode_instruction_type},
        pod::{PodAccount, PodMint},
        processor::Processor,
        solana_zk_sdk::encryption::pod::elgamal::PodElGamalPubkey,
    },
    bytemuck::Zeroable,
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    spl_pod::optional_keys::OptionalNonZeroPubkey,
    spl_token_confidential_transfer_proof_extraction::instruction::verify_and_extract_context,
};

/// Processes an [InitializeConfidentialTransferFeeConfig] instruction.
fn process_initialize_confidential_transfer_fee_config(
    accounts: &[AccountInfo],
    authority: &OptionalNonZeroPubkey,
    withdraw_withheld_authority_elgamal_pubkey: &PodElGamalPubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;

    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack_uninitialized(&mut mint_data)?;
    let extension = mint.init_extension::<ConfidentialTransferFeeConfig>(true)?;
    extension.authority = *authority;
    extension.withdraw_withheld_authority_elgamal_pubkey =
        *withdraw_withheld_authority_elgamal_pubkey;
    extension.harvest_to_mint_enabled = true.into();
    extension.withheld_amount = EncryptedWithheldAmount::zeroed();

    Ok(())
}

/// Processes a [WithdrawWithheldTokensFromMint] instruction.
#[cfg(feature = "zk-ops")]
fn process_withdraw_withheld_tokens_from_mint(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_decryptable_available_balance: &DecryptableBalance,
    proof_instruction_offset: i64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;
    let destination_account_info = next_account_info(account_info_iter)?;

    // zero-knowledge proof certifies that the exact withheld amount is credited to
    // the destination account.
    let proof_context = verify_and_extract_context::<
        CiphertextCiphertextEqualityProofData,
        CiphertextCiphertextEqualityProofContext,
    >(account_info_iter, proof_instruction_offset, None)?;

    let authority_info = next_account_info(account_info_iter)?;
    let authority_info_data_len = authority_info.data_len();

    // unnecessary check, but helps for clarity
    check_program_account(mint_account_info.owner)?;
    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack(&mut mint_data)?;

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
    } // free `transfer_fee_config` to borrow `confidential_transfer_fee_config` as
      // mutable

    // mint must also be extended for confidential transfers, but forgo an explicit
    // check since it is not possible to initialize a confidential transfer mint
    // without it

    let confidential_transfer_fee_config =
        mint.get_extension_mut::<ConfidentialTransferFeeConfig>()?;

    // basic checks for the destination account - must be extended for confidential
    // transfers
    let mut destination_account_data = destination_account_info.data.borrow_mut();
    let mut destination_account =
        PodStateWithExtensionsMut::<PodAccount>::unpack(&mut destination_account_data)?;

    if destination_account.base.mint != *mint_account_info.key {
        return Err(TokenError::MintMismatch.into());
    }
    if destination_account.base.is_frozen() {
        return Err(TokenError::AccountFrozen.into());
    }
    let destination_confidential_transfer_account =
        destination_account.get_extension_mut::<ConfidentialTransferAccount>()?;
    destination_confidential_transfer_account.valid_as_destination()?;

    // The funds are moved from the mint to a destination account. Here, the
    // `source` equates to the withdraw withheld authority associated in the
    // mint.

    // Check that the withdraw authority ElGamal public key associated with the mint
    // is consistent with what was actually used to generate the zkp.
    if proof_context.first_pubkey
        != confidential_transfer_fee_config.withdraw_withheld_authority_elgamal_pubkey
    {
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }
    // Check that the ElGamal public key associated with the destination account is
    // consistent with what was actually used to generate the zkp.
    if proof_context.second_pubkey != destination_confidential_transfer_account.elgamal_pubkey {
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }
    // Check that the withheld amount ciphertext is consistent with the ciphertext
    // data that was actually used to generate the zkp.
    if proof_context.first_ciphertext != confidential_transfer_fee_config.withheld_amount {
        return Err(TokenError::ConfidentialTransferBalanceMismatch.into());
    }

    // The proof data contains the mint withheld amount encrypted under the
    // destination ElGamal pubkey. Add this amount to the available balance.
    destination_confidential_transfer_account.available_balance = ciphertext_arithmetic::add(
        &destination_confidential_transfer_account.available_balance,
        &proof_context.second_ciphertext,
    )
    .ok_or(ProgramError::InvalidInstructionData)?;

    destination_confidential_transfer_account.decryptable_available_balance =
        *new_decryptable_available_balance;

    // Fee is now withdrawn, so zero out the mint withheld amount.
    confidential_transfer_fee_config.withheld_amount = EncryptedWithheldAmount::zeroed();

    Ok(())
}

/// Processes a [WithdrawWithheldTokensFromAccounts] instruction.
#[cfg(feature = "zk-ops")]
fn process_withdraw_withheld_tokens_from_accounts(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    num_token_accounts: u8,
    new_decryptable_available_balance: &DecryptableBalance,
    proof_instruction_offset: i64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;
    let destination_account_info = next_account_info(account_info_iter)?;

    // zero-knowledge proof certifies that the exact aggregate withheld amount is
    // credited to the destination account.
    let proof_context = verify_and_extract_context::<
        CiphertextCiphertextEqualityProofData,
        CiphertextCiphertextEqualityProofContext,
    >(account_info_iter, proof_instruction_offset, None)?;

    let authority_info = next_account_info(account_info_iter)?;
    let authority_info_data_len = authority_info.data_len();
    let account_infos = account_info_iter.as_slice();
    let num_signers = account_infos
        .len()
        .saturating_sub(num_token_accounts as usize);

    // unnecessary check, but helps for clarity
    check_program_account(mint_account_info.owner)?;
    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack(&mut mint_data)?;

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
        PodStateWithExtensionsMut::<PodAccount>::unpack(&mut destination_account_data)?;
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

            aggregate_withheld_amount = ciphertext_arithmetic::add(
                &aggregate_withheld_amount,
                &destination_confidential_transfer_fee_amount.withheld_amount,
            )
            .ok_or(ProgramError::InvalidInstructionData)?;

            destination_confidential_transfer_fee_amount.withheld_amount =
                EncryptedWithheldAmount::zeroed();
        } else {
            match harvest_from_account(mint_account_info.key, account_info) {
                Ok(encrypted_withheld_amount) => {
                    aggregate_withheld_amount = ciphertext_arithmetic::add(
                        &aggregate_withheld_amount,
                        &encrypted_withheld_amount,
                    )
                    .ok_or(ProgramError::InvalidInstructionData)?;
                }
                Err(e) => {
                    msg!("Error harvesting from {}: {}", account_info.key, e);
                }
            }
        }
    }

    let destination_confidential_transfer_account =
        destination_account.get_extension_mut::<ConfidentialTransferAccount>()?;
    destination_confidential_transfer_account.valid_as_destination()?;

    // The funds are moved from the accounts to a destination account. Here, the
    // `source` equates to the withdraw withheld authority associated in the
    // mint.

    // Checks that the withdraw authority ElGamal public key associated with the
    // mint is consistent with what was actually used to generate the zkp.
    let confidential_transfer_fee_config =
        mint.get_extension_mut::<ConfidentialTransferFeeConfig>()?;
    if proof_context.first_pubkey
        != confidential_transfer_fee_config.withdraw_withheld_authority_elgamal_pubkey
    {
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }
    // Checks that the ElGamal public key associated with the destination account is
    // consistent with what was actually used to generate the zkp.
    if proof_context.second_pubkey != destination_confidential_transfer_account.elgamal_pubkey {
        return Err(TokenError::ConfidentialTransferElGamalPubkeyMismatch.into());
    }
    // Checks that the withheld amount ciphertext is consistent with the ciphertext
    // data that was actually used to generate the zkp.
    if proof_context.first_ciphertext != aggregate_withheld_amount {
        return Err(TokenError::ConfidentialTransferBalanceMismatch.into());
    }

    // The proof data contains the mint withheld amount encrypted under the
    // destination ElGamal pubkey. This amount is added to the destination
    // available balance.
    destination_confidential_transfer_account.available_balance = ciphertext_arithmetic::add(
        &destination_confidential_transfer_account.available_balance,
        &proof_context.second_ciphertext,
    )
    .ok_or(ProgramError::InvalidInstructionData)?;

    destination_confidential_transfer_account.decryptable_available_balance =
        *new_decryptable_available_balance;

    Ok(())
}

#[cfg(feature = "zk-ops")]
fn harvest_from_account<'b>(
    mint_key: &'b Pubkey,
    token_account_info: &'b AccountInfo<'_>,
) -> Result<EncryptedWithheldAmount, TokenError> {
    let mut token_account_data = token_account_info.data.borrow_mut();
    let mut token_account =
        PodStateWithExtensionsMut::<PodAccount>::unpack(&mut token_account_data)
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
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack(&mut mint_data)?;
    mint.get_extension::<TransferFeeConfig>()?;
    let confidential_transfer_fee_mint =
        mint.get_extension_mut::<ConfidentialTransferFeeConfig>()?;

    let harvest_to_mint_enabled: bool = confidential_transfer_fee_mint
        .harvest_to_mint_enabled
        .into();
    if !harvest_to_mint_enabled {
        return Err(TokenError::HarvestToMintDisabled.into());
    }

    for token_account_info in token_account_infos {
        match harvest_from_account(mint_account_info.key, token_account_info) {
            Ok(withheld_amount) => {
                let new_mint_withheld_amount = ciphertext_arithmetic::add(
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

/// Process a [EnableHarvestToMint] instruction.
fn process_enable_harvest_to_mint(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let authority_info_data_len = authority_info.data_len();

    check_program_account(mint_info.owner)?;
    let mint_data = &mut mint_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack(mint_data)?;
    let confidential_transfer_fee_mint =
        mint.get_extension_mut::<ConfidentialTransferFeeConfig>()?;

    let maybe_confidential_transfer_fee_authority: Option<Pubkey> =
        confidential_transfer_fee_mint.authority.into();
    let confidential_transfer_fee_authority =
        maybe_confidential_transfer_fee_authority.ok_or(TokenError::NoAuthorityExists)?;

    Processor::validate_owner(
        program_id,
        &confidential_transfer_fee_authority,
        authority_info,
        authority_info_data_len,
        account_info_iter.as_slice(),
    )?;

    confidential_transfer_fee_mint.harvest_to_mint_enabled = true.into();
    Ok(())
}

/// Process a [DisableHarvestToMint] instruction.
fn process_disable_harvest_to_mint(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let authority_info_data_len = authority_info.data_len();

    check_program_account(mint_info.owner)?;
    let mint_data = &mut mint_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack(mint_data)?;
    let confidential_transfer_fee_mint =
        mint.get_extension_mut::<ConfidentialTransferFeeConfig>()?;

    let maybe_confidential_transfer_fee_authority: Option<Pubkey> =
        confidential_transfer_fee_mint.authority.into();
    let confidential_transfer_fee_authority =
        maybe_confidential_transfer_fee_authority.ok_or(TokenError::NoAuthorityExists)?;

    Processor::validate_owner(
        program_id,
        &confidential_transfer_fee_authority,
        authority_info,
        authority_info_data_len,
        account_info_iter.as_slice(),
    )?;

    confidential_transfer_fee_mint.harvest_to_mint_enabled = false.into();
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
            msg!("ConfidentialTransferFeeInstruction::InitializeConfidentialTransferFeeConfig");
            let data =
                decode_instruction_data::<InitializeConfidentialTransferFeeConfigData>(input)?;
            process_initialize_confidential_transfer_fee_config(
                accounts,
                &data.authority,
                &data.withdraw_withheld_authority_elgamal_pubkey,
            )
        }
        ConfidentialTransferFeeInstruction::WithdrawWithheldTokensFromMint => {
            msg!("ConfidentialTransferFeeInstruction::WithdrawWithheldTokensFromMint");
            #[cfg(feature = "zk-ops")]
            {
                let data = decode_instruction_data::<WithdrawWithheldTokensFromMintData>(input)?;
                process_withdraw_withheld_tokens_from_mint(
                    program_id,
                    accounts,
                    &data.new_decryptable_available_balance,
                    data.proof_instruction_offset as i64,
                )
            }
            #[cfg(not(feature = "zk-ops"))]
            {
                Err(ProgramError::InvalidInstructionData)
            }
        }
        ConfidentialTransferFeeInstruction::WithdrawWithheldTokensFromAccounts => {
            msg!("ConfidentialTransferFeeInstruction::WithdrawWithheldTokensFromAccounts");
            #[cfg(feature = "zk-ops")]
            {
                let data =
                    decode_instruction_data::<WithdrawWithheldTokensFromAccountsData>(input)?;
                process_withdraw_withheld_tokens_from_accounts(
                    program_id,
                    accounts,
                    data.num_token_accounts,
                    &data.new_decryptable_available_balance,
                    data.proof_instruction_offset as i64,
                )
            }
            #[cfg(not(feature = "zk-ops"))]
            {
                Err(ProgramError::InvalidInstructionData)
            }
        }
        ConfidentialTransferFeeInstruction::HarvestWithheldTokensToMint => {
            msg!("ConfidentialTransferFeeInstruction::HarvestWithheldTokensToMint");
            #[cfg(feature = "zk-ops")]
            {
                process_harvest_withheld_tokens_to_mint(accounts)
            }
            #[cfg(not(feature = "zk-ops"))]
            {
                Err(ProgramError::InvalidInstructionData)
            }
        }
        ConfidentialTransferFeeInstruction::EnableHarvestToMint => {
            msg!("ConfidentialTransferFeeInstruction::EnableHarvestToMint");
            process_enable_harvest_to_mint(program_id, accounts)
        }
        ConfidentialTransferFeeInstruction::DisableHarvestToMint => {
            msg!("ConfidentialTransferFeeInstruction::DisableHarvestToMint");
            process_disable_harvest_to_mint(program_id, accounts)
        }
    }
}
