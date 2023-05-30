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
        extension::confidential_transfer::{
            instruction::{
                ProofInstruction, WithdrawWithheldTokensData,
                WithdrawWithheldTokensFromAccountsData, WithdrawWithheldTokensFromMintData,
            },
            processor::decode_proof_instruction,
            ConfidentialTransferAccount, ConfidentialTransferMint,
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
    }
}
