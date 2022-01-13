use {
    crate::{check_program_account, extension::transfer_fee::instruction::TransferFeeInstruction},
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, program_option::COption,
        pubkey::Pubkey,
    },
};

fn process_initialize_transfer_fee_config(
    _accounts: &[AccountInfo],
    _fee_config_authority: COption<Pubkey>,
    _withdraw_withheld_authority: COption<Pubkey>,
    _transfer_fee_basis_points: u16,
    _maximum_fee: u64,
) -> ProgramResult {
    unimplemented!();
}

pub(crate) fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction: TransferFeeInstruction,
) -> ProgramResult {
    check_program_account(program_id)?;

    match instruction {
        TransferFeeInstruction::InitializeTransferFeeConfig {
            fee_config_authority,
            withdraw_withheld_authority,
            transfer_fee_basis_points,
            maximum_fee,
        } => process_initialize_transfer_fee_config(
            accounts,
            fee_config_authority,
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
