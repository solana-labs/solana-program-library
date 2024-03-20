use {
    crate::{
        check_program_account,
        error::TokenError,
        extension::{
            transfer_fee::{
                instruction::TransferFeeInstruction, TransferFee, TransferFeeAmount,
                TransferFeeConfig, MAX_FEE_BASIS_POINTS,
            },
            BaseStateWithExtensions, BaseStateWithExtensionsMut, PodStateWithExtensions,
            PodStateWithExtensionsMut,
        },
        pod::{PodAccount, PodMint},
        processor::Processor,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        clock::Clock,
        entrypoint::ProgramResult,
        msg,
        program_option::COption,
        pubkey::Pubkey,
        sysvar::Sysvar,
    },
    std::convert::TryInto,
};

fn process_initialize_transfer_fee_config(
    accounts: &[AccountInfo],
    transfer_fee_config_authority: COption<Pubkey>,
    withdraw_withheld_authority: COption<Pubkey>,
    transfer_fee_basis_points: u16,
    maximum_fee: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;

    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack_uninitialized(&mut mint_data)?;
    let extension = mint.init_extension::<TransferFeeConfig>(true)?;
    extension.transfer_fee_config_authority = transfer_fee_config_authority.try_into()?;
    extension.withdraw_withheld_authority = withdraw_withheld_authority.try_into()?;
    extension.withheld_amount = 0u64.into();

    if transfer_fee_basis_points > MAX_FEE_BASIS_POINTS {
        return Err(TokenError::TransferFeeExceedsMaximum.into());
    }
    // To be safe, set newer and older transfer fees to the same thing on init,
    // but only newer will actually be used
    let epoch = Clock::get()?.epoch;
    let transfer_fee = TransferFee {
        epoch: epoch.into(),
        transfer_fee_basis_points: transfer_fee_basis_points.into(),
        maximum_fee: maximum_fee.into(),
    };
    extension.older_transfer_fee = transfer_fee;
    extension.newer_transfer_fee = transfer_fee;

    Ok(())
}

fn process_set_transfer_fee(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    transfer_fee_basis_points: u16,
    maximum_fee: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let authority_info_data_len = authority_info.data_len();

    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack(&mut mint_data)?;
    let extension = mint.get_extension_mut::<TransferFeeConfig>()?;

    let transfer_fee_config_authority =
        Option::<Pubkey>::from(extension.transfer_fee_config_authority)
            .ok_or(TokenError::NoAuthorityExists)?;
    Processor::validate_owner(
        program_id,
        &transfer_fee_config_authority,
        authority_info,
        authority_info_data_len,
        account_info_iter.as_slice(),
    )?;

    if transfer_fee_basis_points > MAX_FEE_BASIS_POINTS {
        return Err(TokenError::TransferFeeExceedsMaximum.into());
    }

    // When setting the transfer fee, we have two situations:
    // * newer transfer fee epoch <= current epoch: newer transfer fee is the active
    //   one, so overwrite older transfer fee with newer, then overwrite newer
    //   transfer fee
    // * newer transfer fee epoch >= next epoch: it was never used, so just
    //   overwrite next transfer fee
    let epoch = Clock::get()?.epoch;
    if u64::from(extension.newer_transfer_fee.epoch) <= epoch {
        extension.older_transfer_fee = extension.newer_transfer_fee;
    }
    // set two epochs ahead to avoid rug pulls at the end of an epoch
    let newer_fee_start_epoch = epoch.saturating_add(2);
    let transfer_fee = TransferFee {
        epoch: newer_fee_start_epoch.into(),
        transfer_fee_basis_points: transfer_fee_basis_points.into(),
        maximum_fee: maximum_fee.into(),
    };
    extension.newer_transfer_fee = transfer_fee;

    Ok(())
}

fn process_withdraw_withheld_tokens_from_mint(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;
    let destination_account_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let authority_info_data_len = authority_info.data_len();

    // unnecessary check, but helps for clarity
    check_program_account(mint_account_info.owner)?;

    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack(&mut mint_data)?;
    let extension = mint.get_extension_mut::<TransferFeeConfig>()?;

    let withdraw_withheld_authority = Option::<Pubkey>::from(extension.withdraw_withheld_authority)
        .ok_or(TokenError::NoAuthorityExists)?;
    Processor::validate_owner(
        program_id,
        &withdraw_withheld_authority,
        authority_info,
        authority_info_data_len,
        account_info_iter.as_slice(),
    )?;

    let mut destination_account_data = destination_account_info.data.borrow_mut();
    let destination_account =
        PodStateWithExtensionsMut::<PodAccount>::unpack(&mut destination_account_data)?;
    if destination_account.base.mint != *mint_account_info.key {
        return Err(TokenError::MintMismatch.into());
    }
    if destination_account.base.is_frozen() {
        return Err(TokenError::AccountFrozen.into());
    }
    let withheld_amount = u64::from(extension.withheld_amount);
    extension.withheld_amount = 0.into();
    destination_account.base.amount = u64::from(destination_account.base.amount)
        .checked_add(withheld_amount)
        .ok_or(TokenError::Overflow)?
        .into();

    Ok(())
}

fn harvest_from_account<'b>(
    mint_key: &'b Pubkey,
    token_account_info: &'b AccountInfo<'_>,
) -> Result<u64, TokenError> {
    let mut token_account_data = token_account_info.data.borrow_mut();
    let mut token_account =
        PodStateWithExtensionsMut::<PodAccount>::unpack(&mut token_account_data)
            .map_err(|_| TokenError::InvalidState)?;
    if token_account.base.mint != *mint_key {
        return Err(TokenError::MintMismatch);
    }
    check_program_account(token_account_info.owner).map_err(|_| TokenError::InvalidState)?;
    let token_account_extension = token_account
        .get_extension_mut::<TransferFeeAmount>()
        .map_err(|_| TokenError::InvalidState)?;
    let account_withheld_amount = u64::from(token_account_extension.withheld_amount);
    token_account_extension.withheld_amount = 0.into();
    Ok(account_withheld_amount)
}

fn process_harvest_withheld_tokens_to_mint(accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;
    let token_account_infos = account_info_iter.as_slice();

    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack(&mut mint_data)?;
    let mint_extension = mint.get_extension_mut::<TransferFeeConfig>()?;

    for token_account_info in token_account_infos {
        match harvest_from_account(mint_account_info.key, token_account_info) {
            Ok(amount) => {
                let mint_withheld_amount = u64::from(mint_extension.withheld_amount);
                mint_extension.withheld_amount = mint_withheld_amount
                    .checked_add(amount)
                    .ok_or(TokenError::Overflow)?
                    .into();
            }
            Err(e) => {
                msg!("Error harvesting from {}: {}", token_account_info.key, e);
            }
        }
    }
    Ok(())
}

fn process_withdraw_withheld_tokens_from_accounts(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    num_token_accounts: u8,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_account_info = next_account_info(account_info_iter)?;
    let destination_account_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let authority_info_data_len = authority_info.data_len();
    let account_infos = account_info_iter.as_slice();
    let num_signers = account_infos
        .len()
        .saturating_sub(num_token_accounts as usize);

    // unnecessary check, but helps for clarity
    check_program_account(mint_account_info.owner)?;

    let mint_data = mint_account_info.data.borrow();
    let mint = PodStateWithExtensions::<PodMint>::unpack(&mint_data)?;
    let extension = mint.get_extension::<TransferFeeConfig>()?;

    let withdraw_withheld_authority = Option::<Pubkey>::from(extension.withdraw_withheld_authority)
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
    for account_info in &account_infos[num_signers..] {
        // self-harvest, can't double-borrow the underlying data
        if account_info.key == destination_account_info.key {
            let token_account_extension = destination_account
                .get_extension_mut::<TransferFeeAmount>()
                .map_err(|_| TokenError::InvalidState)?;
            let account_withheld_amount = u64::from(token_account_extension.withheld_amount);
            token_account_extension.withheld_amount = 0.into();
            destination_account.base.amount = u64::from(destination_account.base.amount)
                .checked_add(account_withheld_amount)
                .ok_or(TokenError::Overflow)?
                .into();
        } else {
            match harvest_from_account(mint_account_info.key, account_info) {
                Ok(amount) => {
                    destination_account.base.amount = u64::from(destination_account.base.amount)
                        .checked_add(amount)
                        .ok_or(TokenError::Overflow)?
                        .into();
                }
                Err(e) => {
                    msg!("Error harvesting from {}: {}", account_info.key, e);
                }
            }
        }
    }

    Ok(())
}

pub(crate) fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = TransferFeeInstruction::unpack(input)?;
    check_program_account(program_id)?;

    match instruction {
        TransferFeeInstruction::InitializeTransferFeeConfig {
            transfer_fee_config_authority,
            withdraw_withheld_authority,
            transfer_fee_basis_points,
            maximum_fee,
        } => process_initialize_transfer_fee_config(
            accounts,
            transfer_fee_config_authority,
            withdraw_withheld_authority,
            transfer_fee_basis_points,
            maximum_fee,
        ),
        TransferFeeInstruction::TransferCheckedWithFee {
            amount,
            decimals,
            fee,
        } => {
            msg!("TransferFeeInstruction: TransferCheckedWithFee");
            Processor::process_transfer(program_id, accounts, amount, Some(decimals), Some(fee))
        }
        TransferFeeInstruction::WithdrawWithheldTokensFromMint => {
            msg!("TransferFeeInstruction: WithdrawWithheldTokensFromMint");
            process_withdraw_withheld_tokens_from_mint(program_id, accounts)
        }
        TransferFeeInstruction::WithdrawWithheldTokensFromAccounts { num_token_accounts } => {
            msg!("TransferFeeInstruction: WithdrawWithheldTokensFromAccounts");
            process_withdraw_withheld_tokens_from_accounts(program_id, accounts, num_token_accounts)
        }
        TransferFeeInstruction::HarvestWithheldTokensToMint => {
            msg!("TransferFeeInstruction: HarvestWithheldTokensToMint");
            process_harvest_withheld_tokens_to_mint(accounts)
        }
        TransferFeeInstruction::SetTransferFee {
            transfer_fee_basis_points,
            maximum_fee,
        } => {
            msg!("TransferFeeInstruction: SetTransferFee");
            process_set_transfer_fee(program_id, accounts, transfer_fee_basis_points, maximum_fee)
        }
    }
}
