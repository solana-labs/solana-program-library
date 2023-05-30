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
    unimplemented!()
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
            unimplemented!()
        }
    }
}
