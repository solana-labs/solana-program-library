use {
    crate::{
        check_program_account,
        extension::{
            transfer_fee::{instruction::TransferFeeInstruction, TransferFee, TransferFeeConfig},
            StateWithExtensionsMut,
        },
        state::Mint,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        clock::Clock,
        entrypoint::ProgramResult,
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
        TransferFeeInstruction::TransferCheckedWithFee { .. } => {
            unimplemented!();
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
        TransferFeeInstruction::SetTransferFee { .. } => {
            unimplemented!();
        }
    }
}
