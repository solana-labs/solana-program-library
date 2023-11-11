//! Program instructions
use num_enum::{IntoPrimitive, TryFromPrimitive};
use solana_program::program_error::ProgramError;
use std::convert::{TryFrom, TryInto};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use borsh::{BorshSerialize, BorshDeserialize};
/// Instructions supported by the Token Wrap program
#[derive(Clone, Debug, PartialEq, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum TokenWrapInstruction {
    /// Create a wrapped token mint
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. `[writeable,signer]` Funding account for mint and backpointer (must
    ///    be a system account)
    /// 1. `[writeable]` Unallocated wrapped mint account to create, address
    ///    must be: `get_wrapped_mint_address(unwrapped_mint_address,
    ///    wrapped_token_program_id)`
    /// 2. `[writeable]` Unallocated wrapped backpointer account to create
    ///    `get_wrapped_mint_backpointer_address(wrapped_mint_address)`
    /// 3. `[]` Existing unwrapped mint
    /// 4. `[]` System program
    /// 5. `[]` SPL Token program for wrapped mint
    ///
    /// Data expected by this instruction:
    ///   * bool: true = idempotent creation, false = non-idempotent creation
    CreateMint,

    /// Wrap tokens
    ///
    /// Move a user's unwrapped tokens into an escrow account and mint the same
    /// number of wrapped tokens into the provided account.
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. `[writeable]` Unwrapped token account to wrap
    /// 1. `[writeable]` Escrow of unwrapped tokens, must be owned by:
    ///    `get_wrapped_mint_authority(wrapped_mint_address)`
    /// 2. `[]` Unwrapped token mint
    /// 3. `[writeable]` Wrapped mint, must be initialized, address must be:
    ///    `get_wrapped_mint_address(unwrapped_mint_address,
    ///    wrapped_token_program_id)`
    /// 4. `[writeable]` Recipient wrapped token account
    /// 5. `[]` Escrow mint authority, address must be:
    ///    `get_wrapped_mint_authority(wrapped_mint)`
    /// 6. `[]` SPL Token program for unwrapped mint
    /// 7. `[]` SPL Token program for wrapped mint
    /// 8. `[signer]` Transfer authority on unwrapped token account
    /// 8..8+M. `[signer]` (Optional) M multisig signers on unwrapped token
    /// account
    ///
    /// Data expected by this instruction:
    ///   * little-endian u64 representing the amount to wrap
    Wrap,

    /// Unwrap tokens
    ///
    /// Burn user wrapped tokens and transfer the same amount of unwrapped
    /// tokens from the escrow account to the provided account.
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. `[writeable]` Wrapped token account to unwrap
    /// 1. `[writeable]` Wrapped mint, address must be:
    ///    `get_wrapped_mint_address(unwrapped_mint_address,
    ///    wrapped_token_program_id)`
    /// 2. `[writeable]` Escrow of unwrapped tokens, must be owned by:
    ///    `get_wrapped_mint_authority(wrapped_mint_address)`
    /// 3. `[writeable]` Recipient unwrapped tokens
    /// 4. `[]` Unwrapped token mint
    /// 5. `[]` Escrow unwrapped token authority
    ///    `get_wrapped_mint_authority(wrapped_mint)`
    /// 6. `[]` SPL Token program for wrapped mint
    /// 7. `[]` SPL Token program for unwrapped mint
    /// 8. `[signer]` Transfer authority on wrapped token account
    /// 8..8+M. `[signer]` (Optional) M multisig signers on wrapped token
    /// account
    ///
    /// Data expected by this instruction:
    ///   * little-endian u64 representing the amount to unwrap
    Unwrap,
}

/// Creates a 'transfer' instruction.
pub fn transfer(
    program_id: &Pubkey,
    from_pubkey: &Pubkey,
    to_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let data = TransferData { amount }.amount.try_to_vec()?;

    let accounts = vec![
        AccountMeta::new(*from_pubkey, false),
        AccountMeta::new(*to_pubkey, false),
        AccountMeta::new_readonly(*authority_pubkey, true),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Transfer instruction data
#[repr(C)]
pub struct TransferData {
    /// Amount of the token to transfer
    pub amount: u64,
}

/// Creates a 'mint_to' instruction.
pub fn mint_to(
    program_id: &Pubkey,
    mint_pubkey: &Pubkey,
    recipient_pubkey: &Pubkey,
    mint_authority_pubkey: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let data = MintToData { amount }.amount.try_to_vec()?;

    let accounts = vec![
        AccountMeta::new(*mint_pubkey, false),
        AccountMeta::new(*recipient_pubkey, false),
        AccountMeta::new_readonly(*mint_authority_pubkey, true),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// MintTo instruction data
#[repr(C)]
pub struct MintToData {
    /// Amount of the token to mint
    pub amount: u64,
}
/// Creates a 'burn' instruction.
pub fn burn(
    program_id: &Pubkey,
    burn_account_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    burn_authority_pubkey: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let data = BurnData { amount }.amount.try_to_vec()?;

    let accounts = vec![
        AccountMeta::new(*burn_account_pubkey, false),
        AccountMeta::new(*mint_pubkey, false),
        AccountMeta::new_readonly(*burn_authority_pubkey, true),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Burn instruction data
#[repr(C)]
pub struct BurnData {
    /// Amount of the token to burn
    pub amount: u64,
}


/// Creates a 'create_mint' instruction.
pub fn create_mint(
    program_id: &Pubkey,
    funder_pubkey: &Pubkey,
    wrapped_mint_pubkey: &Pubkey,
    backpointer_pubkey: &Pubkey,
    unwrapped_mint_pubkey: &Pubkey,
    system_program_id: &Pubkey,
    wrapped_token_program_id: &Pubkey,
    idempotent: bool,
) -> Result<Instruction, ProgramError> {
    let data = CreateMintData { idempotent }.try_to_vec()?;

    let accounts = vec![
        AccountMeta::new(*funder_pubkey, true),
        AccountMeta::new(*wrapped_mint_pubkey, false),
        AccountMeta::new(*backpointer_pubkey, false),
        AccountMeta::new_readonly(*unwrapped_mint_pubkey, false),
        AccountMeta::new_readonly(*system_program_id, false),
        AccountMeta::new_readonly(*wrapped_token_program_id, false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Creates a 'wrap' instruction.
pub fn wrap(
    program_id: &Pubkey,
    unwrapped_token_account_pubkey: &Pubkey,
    escrow_pubkey: &Pubkey,
    unwrapped_mint_pubkey: &Pubkey,
    wrapped_mint_pubkey: &Pubkey,
    recipient_pubkey: &Pubkey,
    escrow_authority_pubkey: &Pubkey,
    wrapped_token_program_id: &Pubkey,
    transfer_authority_pubkey: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let data = WrapData { amount }.try_to_vec()?;

    let accounts = vec![
        AccountMeta::new(*unwrapped_token_account_pubkey, false),
        AccountMeta::new(*escrow_pubkey, false),
        AccountMeta::new_readonly(*unwrapped_mint_pubkey, false),
        AccountMeta::new(*wrapped_mint_pubkey, false),
        AccountMeta::new(*recipient_pubkey, false),
        AccountMeta::new_readonly(*escrow_authority_pubkey, false),
        AccountMeta::new_readonly(*wrapped_token_program_id, false),
        AccountMeta::new_readonly(*transfer_authority_pubkey, true),
    ];
    Ok( Instruction {
        program_id: *program_id,
        accounts,
        data,
    })  
}

/// Creates an 'unwrap' instruction.
pub fn unwrap(
    program_id: &Pubkey,
    wrapped_token_account_pubkey: &Pubkey,
    wrapped_mint_pubkey: &Pubkey,
    escrow_pubkey: &Pubkey,
    recipient_pubkey: &Pubkey,
    unwrapped_mint_pubkey: &Pubkey,
    escrow_authority_pubkey: &Pubkey,
    wrapped_token_program_id: &Pubkey,
    transfer_authority_pubkey: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let data = UnwrapData { amount }.try_to_vec()?;

    let accounts = vec![
        AccountMeta::new(*wrapped_token_account_pubkey, false),
        AccountMeta::new(*wrapped_mint_pubkey, false),
        AccountMeta::new(*escrow_pubkey, false),
        AccountMeta::new(*recipient_pubkey, false),
        AccountMeta::new_readonly(*unwrapped_mint_pubkey, false),
        AccountMeta::new_readonly(*escrow_authority_pubkey, false),
        AccountMeta::new_readonly(*wrapped_token_program_id, false),
        AccountMeta::new_readonly(*transfer_authority_pubkey, true),
    ];
    burn(wrapped_token_program_id, wrapped_token_account_pubkey, wrapped_mint_pubkey, transfer_authority_pubkey, amount)?;
    transfer(program_id, escrow_pubkey, recipient_pubkey, transfer_authority_pubkey, amount)?;
    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Create Mint instruction data
#[repr(C)]
pub struct CreateMintData {
    /// Whether the creation is idempotent
    pub idempotent: bool,
}
impl BorshSerialize for CreateMintData {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.idempotent.serialize(writer)
    }
}
impl BorshDeserialize for CreateMintData {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let idempotent = bool::deserialize(buf)?;
        Ok(Self { idempotent })
    }
}
/// Wrap instruction data
#[repr(C)]
pub struct WrapData {
    /// Amount of the token to wrap
    pub amount: u64,
}


impl BorshSerialize for WrapData {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.amount.serialize(writer)
    }
}

impl BorshDeserialize for WrapData {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let amount = u64::deserialize(buf)?;
        Ok(Self { amount })
    }
}

/// Unwrap instruction data
#[repr(C)]
pub struct UnwrapData {
    /// Amount of the token to unwrap
    pub amount: u64,
}

impl BorshSerialize for UnwrapData {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.amount.serialize(writer)
    }
}
impl BorshDeserialize for UnwrapData {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let amount = u64::deserialize(buf)?;
        Ok(Self { amount })
    }
}

impl TryFrom<&[u8]> for CreateMintData {
    type Error = ProgramError;

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        if slice.len() != 1 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let idempotent = match slice[0] {
            0 => false,
            1 => true,
            _ => return Err(ProgramError::InvalidInstructionData),
        };
        Ok(CreateMintData { idempotent })
    }
}

impl TryFrom<&[u8]> for WrapData {
    type Error = ProgramError;

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        if slice.len() != 8 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let amount = slice
            .try_into()
            .map(u64::from_le_bytes)
            .map_err(|_| ProgramError::InvalidInstructionData)?;
        Ok(WrapData { amount })
    }
}

impl TryFrom<&[u8]> for UnwrapData {
    type Error = ProgramError;

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        if slice.len() != 8 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let amount = slice
            .try_into()
            .map(u64::from_le_bytes)
            .map_err(|_| ProgramError::InvalidInstructionData)?;
        Ok(UnwrapData { amount })
    }
}