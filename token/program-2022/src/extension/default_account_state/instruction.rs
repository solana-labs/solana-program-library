#[cfg(feature = "serde-traits")]
use serde::{Deserialize, Serialize};
use {
    crate::{
        check_program_account, error::TokenError, instruction::TokenInstruction,
        state::AccountState,
    },
    num_enum::{IntoPrimitive, TryFromPrimitive},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    std::convert::TryFrom,
};

/// Default Account State extension instructions
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum DefaultAccountStateInstruction {
    /// Initialize a new mint with the default state for new Accounts.
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
    ///   `crate::state::AccountState`
    Initialize,
    /// Update the default state for new Accounts. Only supported for mints that
    /// include the `DefaultAccountState` extension.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single authority
    ///   0. `[writable]` The mint.
    ///   1. `[signer]` The mint freeze authority.
    ///
    ///   * Multisignature authority
    ///   0. `[writable]` The mint.
    ///   1. `[]` The mint's multisignature freeze authority.
    ///   2. ..2+M `[signer]` M signer accounts.
    ///
    /// Data expected by this instruction:
    ///   `crate::state::AccountState`
    Update,
}

/// Utility function for decoding a DefaultAccountState instruction and its data
pub fn decode_instruction(
    input: &[u8],
) -> Result<(DefaultAccountStateInstruction, AccountState), ProgramError> {
    if input.len() != 2 {
        return Err(TokenError::InvalidInstruction.into());
    }
    Ok((
        DefaultAccountStateInstruction::try_from(input[0])
            .or(Err(TokenError::InvalidInstruction))?,
        AccountState::try_from(input[1]).or(Err(TokenError::InvalidInstruction))?,
    ))
}

fn encode_instruction(
    token_program_id: &Pubkey,
    accounts: Vec<AccountMeta>,
    instruction_type: DefaultAccountStateInstruction,
    state: &AccountState,
) -> Instruction {
    let mut data = TokenInstruction::DefaultAccountStateExtension.pack();
    data.push(instruction_type.into());
    data.push((*state).into());
    Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    }
}

/// Create an `Initialize` instruction
pub fn initialize_default_account_state(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    state: &AccountState,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let accounts = vec![AccountMeta::new(*mint, false)];
    Ok(encode_instruction(
        token_program_id,
        accounts,
        DefaultAccountStateInstruction::Initialize,
        state,
    ))
}

/// Create an `Initialize` instruction
pub fn update_default_account_state(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    freeze_authority: &Pubkey,
    signers: &[&Pubkey],
    state: &AccountState,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*mint, false),
        AccountMeta::new_readonly(*freeze_authority, signers.is_empty()),
    ];
    for signer_pubkey in signers.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }
    Ok(encode_instruction(
        token_program_id,
        accounts,
        DefaultAccountStateInstruction::Update,
        state,
    ))
}
