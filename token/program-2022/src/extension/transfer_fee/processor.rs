use {
    crate::{
        check_program_account,
        error::TokenError,
        extension::{
            transfer_fee::{
                instruction::TransferFeeInstruction, TransferFee, TransferFeeConfig,
                MAX_FEE_BASIS_POINTS,
            },
            StateWithExtensionsMut,
        },
        processor::Processor,
        state::Mint,
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
    let mut mint = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut mint_data)?;
    let extension = mint.init_extension::<TransferFeeConfig>()?;
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
    let mut mint = StateWithExtensionsMut::<Mint>::unpack(&mut mint_data)?;
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
    // * newer transfer fee epoch <= current epoch:
    //     newer transfer fee is the active one, so overwrite older transfer fee with newer, then overwrite newer transfer fee
    // * newer transfer fee epoch == next epoch:
    //     it was never used, so just overwrite next transfer fee
    let epoch = Clock::get()?.epoch;
    let next_epoch = epoch.saturating_add(1);
    if u64::from(extension.newer_transfer_fee.epoch) <= epoch {
        extension.older_transfer_fee = extension.newer_transfer_fee;
    }
    let transfer_fee = TransferFee {
        epoch: next_epoch.into(),
        transfer_fee_basis_points: transfer_fee_basis_points.into(),
        maximum_fee: maximum_fee.into(),
    };
    extension.newer_transfer_fee = transfer_fee;

    Ok(())
}

pub(crate) fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction: TransferFeeInstruction,
) -> ProgramResult {
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
            msg!("Instruction: TransferCheckedWithFee");
            Processor::process_transfer(program_id, accounts, amount, Some(decimals), Some(fee))
        }
        TransferFeeInstruction::WithdrawWithheldTokensFromMint => {
            unimplemented!();
        }
        TransferFeeInstruction::WithdrawWithheldTokensFromAccounts => {
            unimplemented!();
        }
        TransferFeeInstruction::HarvestWithheldTokensToMint => {
            unimplemented!();
        }
        TransferFeeInstruction::SetTransferFee {
            transfer_fee_basis_points,
            maximum_fee,
        } => process_set_transfer_fee(program_id, accounts, transfer_fee_basis_points, maximum_fee),
    }
}
