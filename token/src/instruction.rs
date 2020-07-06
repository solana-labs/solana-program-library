//! Instruction types

use crate::error::TokenError;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
};
use std::mem::size_of;

/// Specifies the financial specifics of a token.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TokenInfo {
    /// Total supply of tokens.
    pub supply: u64,
    /// Number of base 10 digits to the right of the decimal place in the total supply.
    pub decimals: u64,
}

/// Instructions supported by the token program.
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum TokenInstruction {
    /// Initializes a new mint and optionally deposits all the newly minted tokens in an account.
    ///
    /// # Accounts expected by this instruction:
    ///
    ///   0. `[writable, signer]` New mint to create.
    ///   1.
    ///      * If supply is non-zero: `[writable]` Account to hold all the newly minted tokens.
    ///      * If supply is zero: `[]` Owner of the mint.
    ///   2. Optional: `[]` Owner of the mint if supply is non-zero, if present then further
    ///      minting is supported.
    InitializeMint(TokenInfo),
    /// Initializes a new account.
    ///
    /// # Accounts expected by this instruction:
    ///
    ///   0. `[writable, signer]`  New account being initialized.
    ///   1. `[]` The mint this account will be associated with.
    ///   2. `[]` Owner of the new account.
    InitializeAccount,
    /// Transfers tokens from one account to another either directly or via a delegate.
    ///
    /// # Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The source account.
    ///   1. `[writable]` The destination account.
    ///   2. '[signer]' The source's owner or delegate
    Transfer(u64),
    /// Approves a delegate.  A delegate is given the authority over
    /// tokens on behalf of the source account's owner.  If the amount to
    /// delegate is zero then delegation is rescinded
    ///
    /// # Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The source account.
    ///   1. Optional: `[writable]` The delegate if amount is non-zero.
    ///   2. `[signer]`The source account owner address
    Approve(u64),
    /// Sets a new owner of a mint or account.
    ///
    /// # Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The mint or account to change the owner of.
    ///   1. `[]` The new owner
    ///   2. `[signer]` The owner of the mint or account.
    SetOwner,
    /// Mints new tokens to an account.
    ///
    /// # Accounts expected by this instruction:
    ///
    ///   1. `[writable]` The mint.
    ///   2. `[writable]` The account to mint tokens to.
    ///   0. `[signer]` The owner of the mint.
    MintTo(u64),
    /// Burns tokens by removing them from an account and the total supply.
    ///
    /// # Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The account to burn from.
    ///   1. `[writable]` The mint being burned.
    ///   2. `[signer]` The owner or delegate address of the account to burn from.
    Burn(u64),
}
impl TokenInstruction {
    /// Deserializes a byte buffer into an [TokenInstruction](enum.TokenInstruction.html).
    pub fn deserialize(input: &[u8]) -> Result<Self, ProgramError> {
        if input.len() < size_of::<u8>() {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(match input[0] {
            0 => {
                if input.len() < size_of::<u8>() + size_of::<TokenInfo>() {
                    return Err(ProgramError::InvalidAccountData);
                }
                #[allow(clippy::cast_ptr_alignment)]
                let info: &TokenInfo = unsafe { &*(&input[1] as *const u8 as *const TokenInfo) };
                Self::InitializeMint(*info)
            }
            1 => Self::InitializeAccount,
            2 => {
                if input.len() < size_of::<u8>() + size_of::<u64>() {
                    return Err(ProgramError::InvalidAccountData);
                }
                #[allow(clippy::cast_ptr_alignment)]
                let amount: &u64 = unsafe { &*(&input[1] as *const u8 as *const u64) };
                Self::Transfer(*amount)
            }
            3 => {
                if input.len() < size_of::<u8>() + size_of::<u64>() {
                    return Err(ProgramError::InvalidAccountData);
                }
                #[allow(clippy::cast_ptr_alignment)]
                let amount: &u64 = unsafe { &*(&input[1] as *const u8 as *const u64) };
                Self::Approve(*amount)
            }
            4 => Self::SetOwner,
            5 => {
                if input.len() < size_of::<u8>() + size_of::<u64>() {
                    return Err(ProgramError::InvalidAccountData);
                }
                #[allow(clippy::cast_ptr_alignment)]
                let amount: &u64 = unsafe { &*(&input[1] as *const u8 as *const u64) };
                Self::MintTo(*amount)
            }
            6 => {
                if input.len() < size_of::<u8>() + size_of::<u64>() {
                    return Err(ProgramError::InvalidAccountData);
                }
                #[allow(clippy::cast_ptr_alignment)]
                let amount: &u64 = unsafe { &*(&input[1] as *const u8 as *const u64) };
                Self::Burn(*amount)
            }
            _ => return Err(ProgramError::InvalidAccountData),
        })
    }

    /// Serializes an [TokenInstruction](enum.TokenInstruction.html) into a byte buffer.
    pub fn serialize(self: &Self) -> Result<Vec<u8>, ProgramError> {
        let mut output = vec![0u8; size_of::<TokenInstruction>()];
        match self {
            Self::InitializeMint(info) => {
                output[0] = 0;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut TokenInfo) };
                *value = *info;
            }
            Self::InitializeAccount => output[0] = 1,
            Self::Transfer(amount) => {
                output[0] = 2;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut u64) };
                *value = *amount;
            }
            Self::Approve(amount) => {
                output[0] = 3;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut u64) };
                *value = *amount;
            }
            Self::SetOwner => output[0] = 4,
            Self::MintTo(amount) => {
                output[0] = 5;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut u64) };
                *value = *amount;
            }
            Self::Burn(amount) => {
                output[0] = 6;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut u64) };
                *value = *amount;
            }
        }
        Ok(output)
    }
}

/// Creates a 'InitializeMint' instruction.
pub fn initialize_mint(
    token_program_id: &Pubkey,
    mint_pubkey: &Pubkey,
    account_pubkey: Option<&Pubkey>,
    owner_pubkey: Option<&Pubkey>,
    token_info: TokenInfo,
) -> Result<Instruction, ProgramError> {
    let data = TokenInstruction::InitializeMint(token_info).serialize()?;

    let mut accounts = vec![AccountMeta::new(*mint_pubkey, true)];
    if token_info.supply != 0 {
        match account_pubkey {
            Some(pubkey) => accounts.push(AccountMeta::new(*pubkey, false)),
            None => {
                return Err(ProgramError::NotEnoughAccountKeys);
            }
        }
    }
    match owner_pubkey {
        Some(pubkey) => accounts.push(AccountMeta::new_readonly(*pubkey, false)),
        None => {
            if token_info.supply == 0 {
                return Err(TokenError::OwnerRequiredIfNoInitialSupply.into());
            }
        }
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates a `InitializeAccount` instruction.
pub fn initialize_account(
    token_program_id: &Pubkey,
    account_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = TokenInstruction::InitializeAccount.serialize()?;

    let accounts = vec![
        AccountMeta::new(*account_pubkey, true),
        AccountMeta::new_readonly(*mint_pubkey, false),
        AccountMeta::new_readonly(*owner_pubkey, false),
    ];

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates a `Transfer` instruction.
pub fn transfer(
    token_program_id: &Pubkey,
    source_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let data = TokenInstruction::Transfer(amount).serialize()?;

    let accounts = vec![
        AccountMeta::new(*source_pubkey, false),
        AccountMeta::new(*destination_pubkey, false),
        AccountMeta::new_readonly(*authority_pubkey, true),
    ];

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates an `Approve` instruction.
pub fn approve(
    token_program_id: &Pubkey,
    source_pubkey: &Pubkey,
    delegate_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let data = TokenInstruction::Approve(amount).serialize()?;

    let mut accounts = vec![AccountMeta::new_readonly(*source_pubkey, false)];
    if amount > 0 {
        accounts.push(AccountMeta::new(*delegate_pubkey, false));
    }
    accounts.push(AccountMeta::new_readonly(*owner_pubkey, true));

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates an `SetOwner` instruction.
pub fn set_owner(
    token_program_id: &Pubkey,
    owned_pubkey: &Pubkey,
    new_owner_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = TokenInstruction::SetOwner.serialize()?;

    let accounts = vec![
        AccountMeta::new(*owned_pubkey, false),
        AccountMeta::new_readonly(*new_owner_pubkey, false),
        AccountMeta::new_readonly(*owner_pubkey, true),
    ];

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates an `MintTo` instruction.
pub fn mint_to(
    token_program_id: &Pubkey,
    mint_pubkey: &Pubkey,
    account_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let data = TokenInstruction::MintTo(amount).serialize()?;

    let accounts = vec![
        AccountMeta::new(*mint_pubkey, false),
        AccountMeta::new(*account_pubkey, false),
        AccountMeta::new_readonly(*owner_pubkey, true),
    ];

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates an `Burn` instruction.
pub fn burn(
    token_program_id: &Pubkey,
    account_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let data = TokenInstruction::Burn(amount).serialize()?;

    let accounts = vec![
        AccountMeta::new(*account_pubkey, false),
        AccountMeta::new(*mint_pubkey, false),
        AccountMeta::new_readonly(*authority_pubkey, true),
    ];

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}
