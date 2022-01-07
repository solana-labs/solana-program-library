use {
    crate::{
        extension::confidential_transfer::{
            get_omnibus_token_address, ConfidentialTransferAuditor,
        },
        id,
        instruction::TokenInstruction,
        pod::*,
    },
    bytemuck::Pod,
    num_derive::{FromPrimitive, ToPrimitive},
    num_traits::{FromPrimitive, ToPrimitive},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
    },
};

/// Confidential Transfer extension instructions
#[derive(Clone, Copy, Debug, FromPrimitive, ToPrimitive)]
#[repr(u8)]
pub enum ConfidentialTransferInstruction {
    /// Configures the confidential transfer auditor for a given SPL Token mint.
    ///
    /// The `InitializeAuditor` instruction requires no signers and MUST be included within the
    /// same Transaction as `TokenInstruction::InitializeMint`.  Otherwise another party can
    /// initialize the auditor.
    ///
    /// The instruction fails if the `TokenInstruction::InitializeMint` instruction has already
    /// executed for the mint.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The SPL Token mint
    //
    /// Data expected by this instruction:
    ///   `ConfidentialTransferAuditor`
    ///
    InitializeAuditor,

    /// Configures the confidential transfer omnibus account for a given SPL Token mint.
    ///
    /// The instruction fails if the omnibus account is already configured for the mint.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writeable,signer]` Funding account (must be a system account)
    ///   1. `[]` The SPL Token mint
    ///   2. `[writable]` The omnibus SPL Token account to create, computed by `get_omnibus_token_address()`
    ///   3. `[]` System program
    ///
    /// Data expected by this instruction:
    ///   None
    ///
    ConfigureOmnibusAccount,

    /// Updates the confidential transfer auditor for a given SPL Token mint.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The SPL Token mint
    ///   1. `[signer]` Confidential transfer auditor authority
    ///   2. `[signer]` New confidential transfer auditor authority
    ///
    /// Data expected by this instruction:
    ///   `ConfidentialTransferAuditor`
    ///
    UpdateAuditor,

    /// Approves a token account for confidential transfers.
    ///
    /// Approval is only required when the `ConfidentialTransferAuditor::approve_new_accounts`
    /// field is set in the SPL Token mint.  This instruction must be executed after the account
    /// owner configures their account for confidential transfers with
    /// `ConfidentialTransferInstruction::ConfigureAccount`.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The SPL Token account to approve
    ///   1. `[]` The SPL Token mint
    ///   2. `[signer]` Confidential transfer auditor authority
    ///
    /// Data expected by this instruction:
    ///   None
    ///
    ApproveAccount,
    // TODO: Add remaining zk-token insructions here..
}

pub(crate) fn decode_instruction_type(
    input: &[u8],
) -> Result<ConfidentialTransferInstruction, ProgramError> {
    if input.is_empty() {
        Err(ProgramError::InvalidInstructionData)
    } else {
        FromPrimitive::from_u8(input[0]).ok_or(ProgramError::InvalidInstructionData)
    }
}

pub(crate) fn decode_instruction_data<T: Pod>(input: &[u8]) -> Result<&T, ProgramError> {
    if input.is_empty() {
        Err(ProgramError::InvalidInstructionData)
    } else {
        pod_from_bytes(&input[1..])
    }
}

fn encode_instruction<T: Pod>(
    accounts: Vec<AccountMeta>,
    instruction_type: ConfidentialTransferInstruction,
    instruction_data: &T,
) -> Instruction {
    let mut data = TokenInstruction::ConfidentialTransferExtension.pack();
    data.push(ToPrimitive::to_u8(&instruction_type).unwrap());
    data.extend_from_slice(bytemuck::bytes_of(instruction_data));
    Instruction {
        program_id: id(),
        accounts,
        data,
    }
}

/// Create a `InitializeAuditor` instruction
pub fn initialize_auditor(mint: Pubkey, auditor: &ConfidentialTransferAuditor) -> Instruction {
    let accounts = vec![AccountMeta::new(mint, false)];
    encode_instruction(
        accounts,
        ConfidentialTransferInstruction::InitializeAuditor,
        auditor,
    )
}

/// Create a `ConfigureOmnibusAccount` instruction
pub fn configure_omnibus_account(funding_address: Pubkey, mint: Pubkey) -> Instruction {
    let accounts = vec![
        AccountMeta::new(funding_address, true),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new(get_omnibus_token_address(&mint), false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];
    encode_instruction(
        accounts,
        ConfidentialTransferInstruction::ConfigureOmnibusAccount,
        &(),
    )
}

/// Create a `UpdateAuditor` instruction
pub fn update_auditor(
    mint: Pubkey,
    new_auditor: &ConfidentialTransferAuditor,
    authority: Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(mint, false),
        AccountMeta::new_readonly(authority, true),
        AccountMeta::new_readonly(
            new_auditor.authority,
            new_auditor.authority != Pubkey::default(),
        ),
    ];
    encode_instruction(
        accounts,
        ConfidentialTransferInstruction::UpdateAuditor,
        new_auditor,
    )
}

/// Create an `ApproveAccount` instruction
pub fn approve_account(mint: Pubkey, account_to_approve: Pubkey, authority: Pubkey) -> Instruction {
    let accounts = vec![
        AccountMeta::new(account_to_approve, false),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new_readonly(authority, true),
    ];
    encode_instruction(
        accounts,
        ConfidentialTransferInstruction::ApproveAccount,
        &(),
    )
}
