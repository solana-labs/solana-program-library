use {
    crate::{
        check_program_account,
        instruction::{encode_instruction, TokenInstruction},
        pod::OptionalNonZeroPubkey,
    },
    bytemuck::{Pod, Zeroable},
    num_enum::{IntoPrimitive, TryFromPrimitive},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    std::convert::TryInto,
};

/// Permissioned transfer mint extension instructions
#[derive(Clone, Copy, Debug, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum PermissionedTransferInstruction {
    /// Initialize a new mint with permissioned transfer.
    ///
    /// Fails if the mint has already been initialized, so must be called before
    /// `InitializeMint`.
    ///
    /// The mint must have exactly enough space allocated for the base mint (82
    /// bytes), plus 83 bytes of padding, 1 byte reserved for the account type,
    /// then space required for this extension, plus any others.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The mint to initialize.
    ///
    /// Data expected by this instruction:
    ///   `crate::extension::permissioned_transfer::instruction::InitializeInstructionData`
    ///
    Initialize,
    /// Update the permissioned transfer program id. Only supported for mints that
    /// include the `PermissionedTransfer` extension.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single authority
    ///   0. `[writable]` The mint.
    ///   1. `[signer]` The permissioned transfer authority.
    ///
    ///   * Multisignature authority
    ///   0. `[writable]` The mint.
    ///   1. `[]` The mint's permissioned transfer authority.
    ///   2. ..2+M `[signer]` M signer accounts.
    ///
    /// Data expected by this instruction:
    ///   `crate::extension::permissioned_transfer::UpdateInstructionData`
    ///
    Update,
}

/// Data expected by `Initialize`
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct InitializeInstructionData {
    /// The public key for the account that can update the program id
    pub authority: OptionalNonZeroPubkey,
    /// The program id that validates transfers
    pub permissioned_transfer_program_id: OptionalNonZeroPubkey,
}

/// Data expected by `Update`
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct UpdateInstructionData {
    /// The program id that validates transfers
    pub permissioned_transfer_program_id: OptionalNonZeroPubkey,
}

/// Create an `Initialize` instruction
pub fn initialize(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    authority: Option<Pubkey>,
    permissioned_transfer_program_id: Option<Pubkey>,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let accounts = vec![AccountMeta::new(*mint, false)];
    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::PermissionedTransferExtension,
        PermissionedTransferInstruction::Initialize,
        &InitializeInstructionData {
            authority: authority.try_into()?,
            permissioned_transfer_program_id: permissioned_transfer_program_id.try_into()?,
        },
    ))
}

/// Create an `Update` instruction
pub fn update(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
    signers: &[&Pubkey],
    permissioned_transfer_program_id: Option<Pubkey>,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*mint, false),
        AccountMeta::new_readonly(*authority, signers.is_empty()),
    ];
    for signer_pubkey in signers.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }
    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::PermissionedTransferExtension,
        PermissionedTransferInstruction::Update,
        &UpdateInstructionData {
            permissioned_transfer_program_id: permissioned_transfer_program_id.try_into()?,
        },
    ))
}
