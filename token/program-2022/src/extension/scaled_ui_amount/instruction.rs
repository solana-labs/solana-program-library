#[cfg(feature = "serde-traits")]
use serde::{Deserialize, Serialize};
use {
    crate::{
        check_program_account,
        extension::scaled_ui_amount::{PodF64, UnixTimestamp},
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

/// Interesting-bearing mint extension instructions
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum ScaledUiAmountMintInstruction {
    /// Initialize a new mint with scaled UI amounts.
    ///
    /// Fails if the mint has already been initialized, so must be called before
    /// `InitializeMint`.
    ///
    /// Fails if the multiplier is less than or equal to 0 or if it's
    /// [subnormal](https://en.wikipedia.org/wiki/Subnormal_number).
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
    ///   `crate::extension::scaled_ui_amount::instruction::InitializeInstructionData`
    Initialize,
    /// Update the multiplier. Only supported for mints that include the
    /// `ScaledUiAmount` extension.
    ///
    /// Fails if the multiplier is less than or equal to 0 or if it's
    /// [subnormal](https://en.wikipedia.org/wiki/Subnormal_number).
    ///
    /// The authority provides a new multiplier and a unix timestamp on which
    /// it should take effect. If the timestamp is before the current time,
    /// immediately sets the multiplier.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single authority
    ///   0. `[writable]` The mint.
    ///   1. `[signer]` The multiplier authority.
    ///
    ///   * Multisignature authority
    ///   0. `[writable]` The mint.
    ///   1. `[]` The mint's multisignature multiplier authority.
    ///   2. `..2+M` `[signer]` M signer accounts.
    ///
    /// Data expected by this instruction:
    ///   `crate::extension::scaled_ui_amount::instruction::UpdateMultiplierInstructionData`
    UpdateMultiplier,
}

/// Data expected by `ScaledUiAmountMint::Initialize`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct InitializeInstructionData {
    /// The public key for the account that can update the multiplier
    pub authority: OptionalNonZeroPubkey,
    /// The initial multiplier
    pub multiplier: PodF64,
}

/// Data expected by `ScaledUiAmountMint::UpdateMultiplier`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct UpdateMultiplierInstructionData {
    /// The new multiplier
    pub multiplier: PodF64,
    /// Timestamp at which the new multiplier will take effect
    pub effective_timestamp: UnixTimestamp,
}

/// Create an `Initialize` instruction
pub fn initialize(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    authority: Option<Pubkey>,
    multiplier: f64,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let accounts = vec![AccountMeta::new(*mint, false)];
    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ScaledUiAmountExtension,
        ScaledUiAmountMintInstruction::Initialize,
        &InitializeInstructionData {
            authority: authority.try_into()?,
            multiplier: multiplier.into(),
        },
    ))
}

/// Create an `UpdateMultiplier` instruction
pub fn update_multiplier(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
    signers: &[&Pubkey],
    multiplier: f64,
    effective_timestamp: i64,
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
        TokenInstruction::ScaledUiAmountExtension,
        ScaledUiAmountMintInstruction::UpdateMultiplier,
        &UpdateMultiplierInstructionData {
            effective_timestamp: effective_timestamp.into(),
            multiplier: multiplier.into(),
        },
    ))
}
