#[cfg(feature = "serde-traits")]
use serde::{Deserialize, Serialize};
use {
    crate::{
        check_program_account,
        instruction::{encode_instruction, TokenInstruction},
    },
    bytemuck::{Pod, Zeroable},
    num_enum::{IntoPrimitive, TryFromPrimitive},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    spl_pod::optional_keys::OptionalNonZeroPubkey,
    std::convert::TryInto,
};

/// Group member pointer extension instructions
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum GroupMemberPointerInstruction {
    /// Initialize a new mint with a group member pointer
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
    ///   1. `[]`         The group mint.
    ///   2. `[signer]`   The group's update authority.
    ///
    /// Data expected by this instruction:
    ///   `crate::extension::group_member_pointer::instruction::InitializeInstructionData`
    Initialize,
    /// Update the group member pointer address. Only supported for mints that
    /// include the `GroupMemberPointer` extension.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single authority
    ///   0. `[writable]` The mint.
    ///   1. `[]`         The group mint.
    ///   2. `[signer]`   The group's update authority.
    ///
    ///   * Multisignature authority
    ///   0. `[writable]` The mint.
    ///   1. `[]`         The group mint.
    ///   2. `[]`         The mint's group member pointer authority.
    ///   3. `[signer]`   The group's update authority.
    ///   4. ..4+M `[signer]` M signer accounts.
    ///
    /// Data expected by this instruction:
    ///   `crate::extension::group_member_pointer::instruction::UpdateInstructionData`
    Update,
}

/// Data expected by `Initialize`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct InitializeInstructionData {
    /// The public key for the account that can update the group address
    pub authority: OptionalNonZeroPubkey,
    /// The account address that holds the group
    pub group_address: Pubkey,
    /// The account address that holds the member
    pub member_address: OptionalNonZeroPubkey,
}

/// Data expected by `Update`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct UpdateInstructionData {
    /// The account address that holds the group
    pub group_address: Pubkey,
    /// The new account address that holds the group
    pub member_address: OptionalNonZeroPubkey,
}

/// Create an `Initialize` instruction
pub fn initialize(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    authority: Option<Pubkey>,
    group_update_authority: &Pubkey,
    group_address: &Pubkey,
    member_address: Option<Pubkey>,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let accounts = vec![
        AccountMeta::new(*mint, false),
        AccountMeta::new(*group_address, false),
        AccountMeta::new(*group_update_authority, true),
    ];
    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::GroupMemberPointerExtension,
        GroupMemberPointerInstruction::Initialize,
        &InitializeInstructionData {
            authority: authority.try_into()?,
            group_address: *group_address,
            member_address: member_address.try_into()?,
        },
    ))
}

/// Create an `Update` instruction
pub fn update(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
    group_update_authority: &Pubkey,
    signers: &[&Pubkey],
    group_address: &Pubkey,
    member_address: Option<Pubkey>,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*mint, false),
        AccountMeta::new(*group_address, false),
        AccountMeta::new_readonly(*authority, signers.is_empty()),
        AccountMeta::new_readonly(*group_update_authority, true),
    ];
    for signer_pubkey in signers.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }
    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::GroupMemberPointerExtension,
        GroupMemberPointerInstruction::Update,
        &UpdateInstructionData {
            group_address: *group_address,
            member_address: member_address.try_into()?,
        },
    ))
}
