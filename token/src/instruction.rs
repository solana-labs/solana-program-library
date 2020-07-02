//! Instruction types

use crate::error::TokenError;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
};
use std::mem::size_of;

/// Maximum number of multisignature signers (max N)
pub const MAX_SIGNERS: usize = 11;
/// Minimum number of multisignature signers (max N)
pub const MIN_SIGNERS: usize = 1;

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
    /// Initializes a new mint and deposits all the newly minted tokens in an account.
    ///
    /// # Accounts expected by this instruction:
    ///
    ///   0. `[writable, signer]` Mint account to initialize
    ///   1.
    ///      * If supply is non-zero: `[writable]` Account to hold all the newly minted tokens.
    ///      * If supply is zero: `[]` Owner of the mint.
    ///   2. Optional: `[]` Owner of the mint if supply is non-zero, if present then the
    ///      token allows further minting of tokens.
    InitializeMint(TokenInfo),
    /// Initializes a new account.  The new account can either hold tokens or be a delegate
    /// for another account.
    ///
    /// # Accounts expected by this instruction:
    ///
    ///   0. `[writable, signer]`  Account to initialize
    ///   1. `[]` Owner of the new account.
    ///   2. `[]` Token this account will be associated with.
    ///   3. Optional: `[]` Source account that this account will be a delegate for.
    InitializeAccount,
    /// Initializes a multisignature account with N provided signers.
    /// Multisignature accounts can take the place of an "Owner" account in any token instructions that
    /// require an owner to be a signer.  The variant field represents the number of required
    /// signers (M).
    ///
    /// # Accounts expected by this instruction:
    ///
    ///   0. `[signer]` Multisignature account to initialize
    ///   1-11. `[]` Signer accounts, must equal to N where 1 <= N <= 11
    InitializeMultisig(u8),
    /// Transfers tokens from one account to another either directly or via a delegate.
    ///
    /// # Accounts expected by this instruction:
    ///
    ///   0. `[signer]` Owner of the source account.
    ///   1. `[writable]` Source/Delegate account.
    ///   2. `[writable]` Destination account.
    ///   3. Optional: `[writable]` Source account if key 1 is a delegate account.
    ///   4-14. Optional: `[Signer]` M multisignature Signer accounts
    Transfer(u64),
    /// Approves a delegate.  A delegate account is given the authority to transfer
    /// another accounts tokens without the other account's owner signing the transfer.
    ///
    /// # Accounts expected by this instruction:
    ///
    ///   0. `[signer]` Owner of the source account.
    ///   1. `[]` Source account.
    ///   2. `[writable]` Delegate account.
    ///   3-13. Optional: `[Signer]` M multisignature Signer accounts
    Approve(u64),
    /// Sets a new owner of a token or account.
    ///
    /// # Accounts expected by this instruction:
    ///
    ///   0. `[signer]` Current owner of the token or account.
    ///   1. `[writable]` token or account to change the owner of.
    ///   2. `[]` New owner
    ///   2-12. Optional: `[Signer]` M multisignature Signer accounts
    SetOwner,
    /// Mints new tokens to an account.
    ///
    /// # Accounts expected by this instruction:
    ///
    ///   0. `[signer]` Owner of the token.
    ///   1. `[writable]` Token to mint.
    ///   2. `[writable]` Account to mint tokens to.
    ///   3-13. Optional: `[Signer]` M multisignature Signer accounts
    MintTo(u64),
    /// Burns tokens by removing them from an account and the total supply.
    ///
    /// # Accounts expected by this instruction:
    ///
    ///   0. `[signer]` Owner of the account to burn.
    ///   1. `[writable]` Account to burn from.
    ///   2. `[writable]` Token being burned.
    ///   3. Optional: `[writable]` Source account if key 1 is a delegate account.
    ///   4-14. Optional: `[Signer]` M multisignature Signer accounts
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
                if input.len() < size_of::<u8>() + size_of::<u8>() {
                    return Err(ProgramError::InvalidAccountData);
                }
                #[allow(clippy::cast_ptr_alignment)]
                let m: &u8 = unsafe { &*(&input[1] as *const u8 as *const u8) };
                Self::InitializeMultisig(*m)
            }
            3 => {
                if input.len() < size_of::<u8>() + size_of::<u64>() {
                    return Err(ProgramError::InvalidAccountData);
                }
                #[allow(clippy::cast_ptr_alignment)]
                let amount: &u64 = unsafe { &*(&input[1] as *const u8 as *const u64) };
                Self::Transfer(*amount)
            }
            4 => {
                if input.len() < size_of::<u8>() + size_of::<u64>() {
                    return Err(ProgramError::InvalidAccountData);
                }
                #[allow(clippy::cast_ptr_alignment)]
                let amount: &u64 = unsafe { &*(&input[1] as *const u8 as *const u64) };
                Self::Approve(*amount)
            }
            5 => Self::SetOwner,
            6 => {
                if input.len() < size_of::<u8>() + size_of::<u64>() {
                    return Err(ProgramError::InvalidAccountData);
                }
                #[allow(clippy::cast_ptr_alignment)]
                let amount: &u64 = unsafe { &*(&input[1] as *const u8 as *const u64) };
                Self::MintTo(*amount)
            }
            7 => {
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
            Self::InitializeMultisig(m) => {
                output[0] = 2;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut u8) };
                *value = *m;
            }
            Self::Transfer(amount) => {
                output[0] = 3;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut u64) };
                *value = *amount;
            }
            Self::Approve(amount) => {
                output[0] = 4;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut u64) };
                *value = *amount;
            }
            Self::SetOwner => output[0] = 5,
            Self::MintTo(amount) => {
                output[0] = 6;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut u64) };
                *value = *amount;
            }
            Self::Burn(amount) => {
                output[0] = 7;
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
    token_pubkey: &Pubkey,
    account_pubkey: Option<&Pubkey>,
    owner_pubkey: Option<&Pubkey>,
    token_info: TokenInfo,
) -> Result<Instruction, ProgramError> {
    let data = TokenInstruction::InitializeMint(token_info).serialize()?;

    let mut accounts = vec![AccountMeta::new(*token_pubkey, true)];
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
    owner_pubkey: &Pubkey,
    token_pubkey: &Pubkey,
    source_pubkey: Option<&Pubkey>,
) -> Result<Instruction, ProgramError> {
    let data = TokenInstruction::InitializeAccount.serialize()?;

    let mut accounts = vec![
        AccountMeta::new(*account_pubkey, true),
        AccountMeta::new_readonly(*owner_pubkey, false),
        AccountMeta::new_readonly(*token_pubkey, false),
    ];
    if let Some(pubkey) = source_pubkey {
        accounts.push(AccountMeta::new_readonly(*pubkey, false));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates a `InitializeMultisig` instruction.
pub fn initialize_multisig(
    token_program_id: &Pubkey,
    multisig_pubkey: &Pubkey,
    signers: &[&Pubkey],
    m: u8,
) -> Result<Instruction, ProgramError> {
    if !(MIN_SIGNERS..MAX_SIGNERS + 1).contains(&(m as usize))
        || !(MIN_SIGNERS..MAX_SIGNERS + 1).contains(&signers.len())
        || m as usize > signers.len()
    {
        return Err(ProgramError::MissingRequiredSignature);
    }
    let data = TokenInstruction::InitializeMultisig(m).serialize()?;

    let mut accounts = vec![AccountMeta::new(*multisig_pubkey, true)];
    for signer in signers.iter() {
        accounts.push(AccountMeta::new_readonly(**signer, false));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates a `Transfer` instruction.
pub fn transfer(
    token_program_id: &Pubkey,
    owner_pubkey: &Pubkey,
    account_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    source_pubkey: Option<&Pubkey>,
    signer_pubkeys: &[&Pubkey],
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let data = TokenInstruction::Transfer(amount).serialize()?;

    let mut accounts = vec![
        AccountMeta::new_readonly(*owner_pubkey, true),
        AccountMeta::new(*account_pubkey, false),
        AccountMeta::new(*destination_pubkey, false),
    ];
    if let Some(pubkey) = source_pubkey {
        accounts.push(AccountMeta::new(*pubkey, false));
    }
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates an `Approve` instruction.
pub fn approve(
    token_program_id: &Pubkey,
    owner_pubkey: &Pubkey,
    source_pubkey: &Pubkey,
    delegate_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let data = TokenInstruction::Approve(amount).serialize()?;

    let mut accounts = vec![
        AccountMeta::new_readonly(*owner_pubkey, true),
        AccountMeta::new_readonly(*source_pubkey, false),
        AccountMeta::new(*delegate_pubkey, false),
    ];
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates an `SetOwner` instruction.
pub fn set_owner(
    token_program_id: &Pubkey,
    owner_pubkey: &Pubkey,
    account_pubkey: &Pubkey,
    new_owner_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    let data = TokenInstruction::SetOwner.serialize()?;

    let mut accounts = vec![
        AccountMeta::new_readonly(*owner_pubkey, true),
        AccountMeta::new(*account_pubkey, false),
        AccountMeta::new_readonly(*new_owner_pubkey, false),
    ];
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates an `MintTo` instruction.
pub fn mint_to(
    token_program_id: &Pubkey,
    owner_pubkey: &Pubkey,
    token_pubkey: &Pubkey,
    account_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let data = TokenInstruction::MintTo(amount).serialize()?;

    let mut accounts = vec![
        AccountMeta::new_readonly(*owner_pubkey, true),
        AccountMeta::new(*token_pubkey, false),
        AccountMeta::new(*account_pubkey, false),
    ];
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates an `Burn` instruction.
pub fn burn(
    token_program_id: &Pubkey,
    owner_pubkey: &Pubkey,
    account_pubkey: &Pubkey,
    token_pubkey: &Pubkey,
    source_pubkey: Option<&Pubkey>,
    signer_pubkeys: &[&Pubkey],
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let data = TokenInstruction::Burn(amount).serialize()?;

    let mut accounts = vec![
        AccountMeta::new_readonly(*owner_pubkey, true),
        AccountMeta::new(*account_pubkey, false),
        AccountMeta::new(*token_pubkey, false),
    ];
    if let Some(pubkey) = source_pubkey {
        accounts.push(AccountMeta::new(*pubkey, false));
    }
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}
