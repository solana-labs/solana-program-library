//! State transition types

use crate::{
    error::TokenError,
    instruction::{Instruction, TokenInfo},
};
use solana_sdk::{
    account_info::AccountInfo, entrypoint::ProgramResult, info, program_error::ProgramError,
    program_utils::next_account_info, pubkey::Pubkey,
};
use std::mem::size_of;

/// Represents a token type identified and identified by its public key.  Accounts
/// are associated with a specific token type and only accounts with
/// matching types my inter-opt.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Token {
    /// The total supply of tokens.
    pub info: TokenInfo,
    /// Optional token owner, used to mint new tokens.  The owner may only
    /// be provided during token creation.  If no owner is present then the token
    /// has a fixed supply and no further tokens may be minted.
    pub owner: Option<Pubkey>,
}

/// Delegation details.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AccountDelegate {
    /// The source account for the tokens.
    pub source: Pubkey,
    /// The original maximum amount that this delegate account was authorized to spend.
    pub original_amount: u64,
}

/// Account that holds or may delegate tokens.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Account {
    /// The type of token this account holds.
    pub token: Pubkey,
    /// Owner of this account.
    pub owner: Pubkey,
    /// Amount of tokens this account holds.
    pub amount: u64,
    /// If `delegate`  is None, `amount` belongs to this account.
    /// If `delegate` is Option<_>, `amount` represents the remaining allowance
    /// of tokens this delegate is authorized to transfer from the `source` account.
    pub delegate: Option<AccountDelegate>,
}

/// Token program states.
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// Unallocated state, may be initialized into another state.
    Unallocated,
    /// A token type.
    Token(Token),
    /// An account that holds an amount of tokens or was delegated the authority to transfer
    /// tokens on behalf of another account.
    Account(Account),
    /// Invalid state, cannot be modified by the token program.
    Invalid,
}
impl Default for State {
    fn default() -> Self {
        Self::Unallocated
    }
}

impl<'a> State {
    /// Processes a [NewToken](enum.Instruction.html) instruction.
    pub fn process_new_token<I: Iterator<Item = &'a AccountInfo<'a>>>(
        account_info_iter: &mut I,
        info: TokenInfo,
    ) -> ProgramResult {
        let token_account_info = next_account_info(account_info_iter)?;

        if State::Unallocated != State::deserialize(&token_account_info.data.borrow())? {
            return Err(TokenError::AlreadyInUse.into());
        }

        let owner = if info.supply != 0 {
            let dest_account_info = next_account_info(account_info_iter)?;
            let mut dest_account_data = dest_account_info.data.borrow_mut();
            if let State::Account(mut dest_token_account) = State::deserialize(&dest_account_data)?
            {
                if !token_account_info.is_signer {
                    return Err(ProgramError::MissingRequiredSignature);
                }
                if token_account_info.key != &dest_token_account.token {
                    return Err(TokenError::TokenMismatch.into());
                }
                if dest_token_account.delegate.is_some() {
                    return Err(TokenError::DestinationIsDelegate.into());
                }

                dest_token_account.amount = info.supply;
                State::Account(dest_token_account).serialize(&mut dest_account_data)?;
            } else {
                return Err(ProgramError::InvalidArgument);
            }

            if let Ok(owner_account_into) = next_account_info(account_info_iter) {
                Some(*owner_account_into.key)
            } else {
                None
            }
        } else if let Ok(owner_account_into) = next_account_info(account_info_iter) {
            Some(*owner_account_into.key)
        } else {
            return Err(TokenError::OwnerRequiredIfNoInitialSupply.into());
        };

        State::Token(Token { info, owner }).serialize(&mut token_account_info.data.borrow_mut())
    }

    /// Processes a [NewAccount](enum.Instruction.html) instruction.
    pub fn process_new_account<I: Iterator<Item = &'a AccountInfo<'a>>>(
        account_info_iter: &mut I,
    ) -> ProgramResult {
        let new_account_info = next_account_info(account_info_iter)?;
        let owner_account_info = next_account_info(account_info_iter)?;
        let token_account_info = next_account_info(account_info_iter)?;

        if !new_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let mut new_account_data = new_account_info.data.borrow_mut();

        if State::Unallocated != State::deserialize(&new_account_data)? {
            return Err(TokenError::AlreadyInUse.into());
        }

        let mut token_account = Account {
            token: *token_account_info.key,
            owner: *owner_account_info.key,
            amount: 0,
            delegate: None,
        };
        if let Ok(delegate_account) = next_account_info(account_info_iter) {
            token_account.delegate = Some(AccountDelegate {
                source: *delegate_account.key,
                original_amount: 0,
            });
        }

        State::Account(token_account).serialize(&mut new_account_data)
    }

    /// Processes a [Transfer](enum.Instruction.html) instruction.
    pub fn process_transfer<I: Iterator<Item = &'a AccountInfo<'a>>>(
        account_info_iter: &mut I,
        amount: u64,
    ) -> ProgramResult {
        let owner_account_info = next_account_info(account_info_iter)?;
        let source_account_info = next_account_info(account_info_iter)?;
        let dest_account_info = next_account_info(account_info_iter)?;

        let mut source_data = source_account_info.data.borrow_mut();
        let mut dest_data = dest_account_info.data.borrow_mut();
        if let (State::Account(mut source_account), State::Account(mut dest_account)) = (
            State::deserialize(&source_data)?,
            State::deserialize(&dest_data)?,
        ) {
            if source_account.token != dest_account.token {
                return Err(TokenError::TokenMismatch.into());
            }
            if dest_account.delegate.is_some() {
                return Err(TokenError::DestinationIsDelegate.into());
            }
            if owner_account_info.key != &source_account.owner {
                return Err(TokenError::NoOwner.into());
            }
            if !owner_account_info.is_signer {
                return Err(ProgramError::MissingRequiredSignature);
            }
            if source_account.amount < amount {
                return Err(TokenError::InsufficientFunds.into());
            }

            if let Some(ref delegate) = source_account.delegate {
                let source_account_info = next_account_info(account_info_iter)?;
                let mut actual_source_data = source_account_info.data.borrow_mut();
                if let State::Account(mut actual_source_account) =
                    State::deserialize(&actual_source_data)?
                {
                    if source_account_info.key != &delegate.source {
                        return Err(TokenError::NotDelegate.into());
                    }

                    if actual_source_account.amount < amount {
                        return Err(TokenError::InsufficientFunds.into());
                    }

                    actual_source_account.amount -= amount;
                    State::Account(actual_source_account).serialize(&mut actual_source_data)?;
                } else {
                    return Err(ProgramError::InvalidArgument);
                }
            }

            source_account.amount -= amount;
            State::Account(source_account).serialize(&mut source_data)?;

            dest_account.amount += amount;
            State::Account(dest_account).serialize(&mut dest_data)?;
        } else {
            return Err(ProgramError::InvalidArgument);
        }
        Ok(())
    }

    /// Processes an [Approve](enum.Instruction.html) instruction.
    pub fn process_approve<I: Iterator<Item = &'a AccountInfo<'a>>>(
        account_info_iter: &mut I,
        amount: u64,
    ) -> ProgramResult {
        let owner_account_info = next_account_info(account_info_iter)?;
        let source_account_info = next_account_info(account_info_iter)?;
        let delegate_account_info = next_account_info(account_info_iter)?;

        let source_data = source_account_info.data.borrow_mut();
        let mut delegate_data = delegate_account_info.data.borrow_mut();
        if let (State::Account(source_account), State::Account(mut delegate_account)) = (
            State::deserialize(&source_data)?,
            State::deserialize(&delegate_data)?,
        ) {
            if source_account.token != delegate_account.token {
                return Err(TokenError::TokenMismatch.into());
            }
            if owner_account_info.key != &source_account.owner {
                return Err(TokenError::NoOwner.into());
            }
            if !owner_account_info.is_signer {
                return Err(ProgramError::MissingRequiredSignature);
            }
            if source_account.delegate.is_some() {
                return Err(ProgramError::InvalidArgument);
            }

            match &delegate_account.delegate {
                None => {
                    return Err(TokenError::NotDelegate.into());
                }
                Some(delegate) => {
                    if source_account_info.key != &delegate.source {
                        return Err(TokenError::NotDelegate.into());
                    }

                    delegate_account.amount = amount;
                    delegate_account.delegate = Some(AccountDelegate {
                        source: delegate.source,
                        original_amount: amount,
                    });
                    State::Account(delegate_account).serialize(&mut delegate_data)?;
                }
            }
        } else {
            return Err(ProgramError::InvalidArgument);
        }
        Ok(())
    }

    /// Processes a [SetOwner](enum.Instruction.html) instruction.
    pub fn process_set_owner<I: Iterator<Item = &'a AccountInfo<'a>>>(
        account_info_iter: &mut I,
    ) -> ProgramResult {
        let owner_account_info = next_account_info(account_info_iter)?;
        let account_info = next_account_info(account_info_iter)?;
        let new_owner_account_info = next_account_info(account_info_iter)?;

        let mut account_data = account_info.data.borrow_mut();
        match State::deserialize(&account_data)? {
            State::Account(mut account) => {
                if owner_account_info.key != &account.owner {
                    return Err(TokenError::NoOwner.into());
                }
                if !owner_account_info.is_signer {
                    return Err(ProgramError::MissingRequiredSignature);
                }

                account.owner = *new_owner_account_info.key;
                State::Account(account).serialize(&mut account_data)?;
            }
            State::Token(mut token) => {
                if Some(*owner_account_info.key) != token.owner {
                    return Err(TokenError::NoOwner.into());
                }
                if !owner_account_info.is_signer {
                    return Err(ProgramError::MissingRequiredSignature);
                }

                token.owner = Some(*new_owner_account_info.key);
                State::Token(token).serialize(&mut account_data)?;
            }
            _ => {
                return Err(ProgramError::InvalidArgument);
            }
        }
        Ok(())
    }

    /// Processes a [MintTo](enum.Instruction.html) instruction.
    pub fn process_mintto<I: Iterator<Item = &'a AccountInfo<'a>>>(
        account_info_iter: &mut I,
        amount: u64,
    ) -> ProgramResult {
        let owner_account_info = next_account_info(account_info_iter)?;
        let token_account_info = next_account_info(account_info_iter)?;
        let dest_account_info = next_account_info(account_info_iter)?;

        if !owner_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let mut token_account_data = token_account_info.data.borrow_mut();
        if let State::Token(mut token) = State::deserialize(&token_account_data)? {
            match token.owner {
                Some(owner) => {
                    if *owner_account_info.key != owner {
                        return Err(TokenError::NoOwner.into());
                    }
                }
                None => {
                    return Err(TokenError::FixedSupply.into());
                }
            }

            let mut dest_account_data = dest_account_info.data.borrow_mut();
            if let State::Account(mut dest_token_account) = State::deserialize(&dest_account_data)?
            {
                if token_account_info.key != &dest_token_account.token {
                    return Err(TokenError::TokenMismatch.into());
                }
                if dest_token_account.delegate.is_some() {
                    return Err(TokenError::DestinationIsDelegate.into());
                }

                token.info.supply += amount;
                State::Token(token).serialize(&mut token_account_data)?;

                dest_token_account.amount = amount;
                State::Account(dest_token_account).serialize(&mut dest_account_data)?;
            } else {
                return Err(ProgramError::InvalidArgument);
            }
        } else {
            return Err(ProgramError::InvalidArgument);
        }
        Ok(())
    }

    /// Processes a [Burn](enum.Instruction.html) instruction.
    pub fn process_burn<I: Iterator<Item = &'a AccountInfo<'a>>>(
        account_info_iter: &mut I,
        amount: u64,
    ) -> ProgramResult {
        let owner_account_info = next_account_info(account_info_iter)?;
        let source_account_info = next_account_info(account_info_iter)?;
        let token_account_info = next_account_info(account_info_iter)?;

        let (mut source_account, mut source_data) = {
            let source_data = source_account_info.data.borrow_mut();
            match State::deserialize(&source_data)? {
                State::Account(source_account) => (source_account, source_data),
                _ => {
                    return Err(ProgramError::InvalidArgument);
                }
            }
        };

        let (mut token_account, mut token_data) = {
            let token_data = token_account_info.data.borrow_mut();
            match State::deserialize(&token_data)? {
                State::Token(token_account) => (token_account, token_data),
                _ => {
                    return Err(ProgramError::InvalidArgument);
                }
            }
        };

        if token_account_info.key != &source_account.token {
            return Err(TokenError::TokenMismatch.into());
        }
        if owner_account_info.key != &source_account.owner {
            return Err(TokenError::NoOwner.into());
        }
        if !owner_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if source_account.amount < amount {
            return Err(TokenError::InsufficientFunds.into());
        }

        if let Some(ref delegate) = source_account.delegate {
            let source_account_info = next_account_info(account_info_iter)?;
            let mut actual_source_data = source_account_info.data.borrow_mut();
            if let State::Account(mut actual_source_account) =
                State::deserialize(&actual_source_data)?
            {
                if source_account_info.key != &delegate.source {
                    return Err(TokenError::NotDelegate.into());
                }

                if actual_source_account.amount < amount {
                    return Err(TokenError::InsufficientFunds.into());
                }

                actual_source_account.amount -= amount;
                State::Account(actual_source_account).serialize(&mut actual_source_data)?;
            } else {
                return Err(ProgramError::InvalidArgument);
            }
        }

        source_account.amount -= amount;
        State::Account(source_account).serialize(&mut source_data)?;

        token_account.info.supply -= amount;
        State::Token(token_account).serialize(&mut token_data)?;
        Ok(())
    }

    /// Processes an [Instruction](enum.Instruction.html).
    pub fn process(
        _program_id: &Pubkey,
        accounts: &'a [AccountInfo<'a>],
        input: &[u8],
    ) -> ProgramResult {
        let instruction = Instruction::deserialize(input)?;
        let account_info_iter = &mut accounts.iter();

        match instruction {
            Instruction::NewToken(info) => {
                info!("Instruction: NewToken");
                Self::process_new_token(account_info_iter, info)
            }
            Instruction::NewAccount => {
                info!("Instruction: NewAccount");
                Self::process_new_account(account_info_iter)
            }
            Instruction::Transfer(amount) => {
                info!("Instruction: Transfer");
                Self::process_transfer(account_info_iter, amount)
            }
            Instruction::Approve(amount) => {
                info!("Instruction: Approve");
                Self::process_approve(account_info_iter, amount)
            }
            Instruction::SetOwner => {
                info!("Instruction: SetOwner");
                Self::process_set_owner(account_info_iter)
            }
            Instruction::MintTo(amount) => {
                info!("Instruction: MintTo");
                Self::process_mintto(account_info_iter, amount)
            }
            Instruction::Burn(amount) => {
                info!("Instruction: Burn");
                Self::process_burn(account_info_iter, amount)
            }
        }
    }

    /// Deserializes a byte buffer into a Token Program [State](struct.State.html)
    pub fn deserialize(input: &'a [u8]) -> Result<Self, ProgramError> {
        if input.len() < size_of::<u8>() {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(match input[0] {
            0 => Self::Unallocated,
            1 => {
                if input.len() < size_of::<u8>() + size_of::<Token>() {
                    return Err(ProgramError::InvalidAccountData);
                }
                #[allow(clippy::cast_ptr_alignment)]
                let token: &Token = unsafe { &*(&input[1] as *const u8 as *const Token) };
                Self::Token(*token)
            }
            2 => {
                if input.len() < size_of::<u8>() + size_of::<Account>() {
                    return Err(ProgramError::InvalidAccountData);
                }
                #[allow(clippy::cast_ptr_alignment)]
                let account: &Account = unsafe { &*(&input[1] as *const u8 as *const Account) };
                Self::Account(*account)
            }
            3 => Self::Invalid,
            _ => return Err(ProgramError::InvalidAccountData),
        })
    }

    /// Serializes Token Program [State](struct.State.html) into a byte buffer
    pub fn serialize(self: &Self, output: &mut [u8]) -> ProgramResult {
        if output.len() < size_of::<u8>() {
            return Err(ProgramError::InvalidAccountData);
        }
        match self {
            Self::Unallocated => output[0] = 0,
            Self::Token(token) => {
                if output.len() < size_of::<u8>() + size_of::<Token>() {
                    return Err(ProgramError::InvalidAccountData);
                }
                output[0] = 1;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut Token) };
                *value = *token;
            }
            Self::Account(account) => {
                if output.len() < size_of::<u8>() + size_of::<Account>() {
                    return Err(ProgramError::InvalidAccountData);
                }
                output[0] = 2;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut Account) };
                *value = *account;
            }
            Self::Invalid => output[0] = 3,
        }
        Ok(())
    }
}

impl Instruction {
    /// Deserializes a byte buffer into an [Instruction](enum.Instruction.html)
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
                Self::NewToken(*info)
            }
            1 => Self::NewAccount,
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

    /// Serializes an [Instruction](enum.Instruction.html) into a byte buffer
    pub fn serialize(self: &Self, output: &mut [u8]) -> ProgramResult {
        if output.len() < size_of::<u8>() {
            return Err(ProgramError::InvalidAccountData);
        }
        match self {
            Self::NewToken(info) => {
                if output.len() < size_of::<u8>() + size_of::<TokenInfo>() {
                    return Err(ProgramError::InvalidAccountData);
                }
                output[0] = 0;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut TokenInfo) };
                *value = *info;
            }
            Self::NewAccount => output[0] = 1,
            Self::Transfer(amount) => {
                if output.len() < size_of::<u8>() + size_of::<u64>() {
                    return Err(ProgramError::InvalidAccountData);
                }
                output[0] = 2;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut u64) };
                *value = *amount;
            }
            Self::Approve(amount) => {
                if output.len() < size_of::<u8>() + size_of::<u64>() {
                    return Err(ProgramError::InvalidAccountData);
                }
                output[0] = 3;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut u64) };
                *value = *amount;
            }
            Self::SetOwner => output[0] = 4,
            Self::MintTo(amount) => {
                if output.len() < size_of::<u8>() + size_of::<u64>() {
                    return Err(ProgramError::InvalidAccountData);
                }
                output[0] = 5;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut u64) };
                *value = *amount;
            }
            Self::Burn(amount) => {
                if output.len() < size_of::<u8>() + size_of::<u64>() {
                    return Err(ProgramError::InvalidAccountData);
                }
                output[0] = 6;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut u64) };
                *value = *amount;
            }
        }
        Ok(())
    }
}

// Pulls in the stubs required for `info!()`
#[cfg(not(target_arch = "bpf"))]
solana_sdk_bpf_test::stubs!();

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{account::Account, account_info::create_is_signer_account_infos};

    fn new_pubkey(id: u8) -> Pubkey {
        Pubkey::new(&vec![
            id, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1,
        ])
    }

    #[test]
    fn test_new_token() {
        let program_id = new_pubkey(1);
        let mut instruction_data = vec![0u8; size_of::<Instruction>()];
        let token_account_key = new_pubkey(2);
        let mut token_account_account = Account::new(0, size_of::<State>(), &program_id);
        let token_account2_key = new_pubkey(3);
        let mut token_account2_account = Account::new(0, size_of::<State>(), &program_id);
        let delegate_account_key = new_pubkey(4);
        let mut delegate_account_account = Account::new(0, size_of::<State>(), &program_id);
        let owner_key = new_pubkey(5);
        let mut owner_account = Account::default();
        let token_key = new_pubkey(6);
        let mut token_account = Account::new(0, size_of::<State>(), &program_id);
        let token2_key = new_pubkey(7);
        let mut token2_account = Account::new(0, size_of::<State>(), &program_id);

        // account not created
        let instruction = Instruction::NewToken(TokenInfo {
            supply: 1000,
            decimals: 2,
        });
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&token_key, true, &mut token_account),
            (&token_account_key, false, &mut token_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(ProgramError::InvalidArgument),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // create account
        let instruction = Instruction::NewAccount;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&token_account_key, true, &mut token_account_account),
            (&owner_key, false, &mut owner_account),
            (&token_key, false, &mut token_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // create new token
        let instruction = Instruction::NewToken(TokenInfo {
            supply: 1000,
            decimals: 2,
        });
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&token_key, true, &mut token_account),
            (&token_account_key, false, &mut token_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // create another account
        let instruction = Instruction::NewAccount;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&token_account2_key, true, &mut token_account2_account),
            (&owner_key, false, &mut owner_account),
            (&token_key, false, &mut token_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // token mismatch
        let instruction = Instruction::NewToken(TokenInfo {
            supply: 1000,
            decimals: 2,
        });
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&token2_key, true, &mut token2_account),
            (&token_account2_key, false, &mut token_account2_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(TokenError::TokenMismatch.into()),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // create delegate account
        let instruction = Instruction::NewAccount;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&delegate_account_key, true, &mut delegate_account_account),
            (&owner_key, false, &mut owner_account),
            (&token_key, false, &mut token_account),
            (&token_account_key, false, &mut token_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // account is a delegate token
        let instruction = Instruction::NewToken(TokenInfo {
            supply: 1000,
            decimals: 2,
        });
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&token_key, true, &mut token_account),
            (&delegate_account_key, false, &mut delegate_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(TokenError::AlreadyInUse.into()),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // create twice
        let mut accounts = vec![
            (&token_key, true, &mut token_account),
            (&token_account_key, false, &mut token_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(TokenError::AlreadyInUse.into()),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );
    }

    #[test]
    fn test_new_token_account() {
        let program_id = new_pubkey(1);
        let mut instruction_data = vec![0u8; size_of::<Instruction>()];
        let token_account_key = new_pubkey(2);
        let mut token_account_account = Account::new(0, size_of::<State>(), &program_id);
        let owner_key = new_pubkey(3);
        let mut owner_account = Account::default();
        let token_key = new_pubkey(4);
        let mut token_account = Account::new(0, size_of::<State>(), &program_id);

        // missing signer
        let instruction = Instruction::NewAccount;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&token_account_key, false, &mut token_account_account),
            (&owner_key, false, &mut owner_account),
            (&token_key, false, &mut token_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // create account
        let mut accounts = vec![
            (&token_account_key, true, &mut token_account_account),
            (&owner_key, false, &mut owner_account),
            (&token_key, false, &mut token_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // create twice
        let mut accounts = vec![
            (&token_account_key, true, &mut token_account_account),
            (&owner_key, false, &mut owner_account),
            (&token_key, false, &mut token_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(TokenError::AlreadyInUse.into()),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );
    }

    #[test]
    fn test_transfer() {
        let program_id = new_pubkey(1);
        let mut instruction_data = vec![0u8; size_of::<Instruction>()];
        let token_account_key = new_pubkey(2);
        let mut token_account_account = Account::new(0, size_of::<State>(), &program_id);
        let token_account2_key = new_pubkey(3);
        let mut token_account2_account = Account::new(0, size_of::<State>(), &program_id);
        let token_account3_key = new_pubkey(3);
        let mut token_account3_account = Account::new(0, size_of::<State>(), &program_id);
        let delegate_account_key = new_pubkey(4);
        let mut delegate_account_account = Account::new(0, size_of::<State>(), &program_id);
        let mismatch_account_key = new_pubkey(5);
        let mut mismatch_account_account = Account::new(0, size_of::<State>(), &program_id);
        let mismatch_delegate_account_key = new_pubkey(5);
        let mut mismatch_delegate_account_account =
            Account::new(0, size_of::<State>(), &program_id);
        let owner_key = new_pubkey(6);
        let mut owner_account = Account::default();
        let owner2_key = new_pubkey(7);
        let mut owner2_account = Account::default();
        let token_key = new_pubkey(8);
        let mut token_account = Account::new(0, size_of::<State>(), &program_id);
        let token2_key = new_pubkey(9);
        let mut token2_account = Account::new(0, size_of::<State>(), &program_id);

        // create account
        let instruction = Instruction::NewAccount;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&token_account_key, true, &mut token_account_account),
            (&owner_key, false, &mut owner_account),
            (&token_key, false, &mut token_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // create another account
        let instruction = Instruction::NewAccount;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&token_account2_key, true, &mut token_account2_account),
            (&owner_key, false, &mut owner_account),
            (&token_key, false, &mut token_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // create another account
        let instruction = Instruction::NewAccount;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&token_account3_key, true, &mut token_account3_account),
            (&owner_key, false, &mut owner_account),
            (&token_key, false, &mut token_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // create mismatch account
        let instruction = Instruction::NewAccount;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&mismatch_account_key, true, &mut mismatch_account_account),
            (&owner_key, false, &mut owner_account),
            (&token2_key, false, &mut token2_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // create delegate account
        let instruction = Instruction::NewAccount;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&delegate_account_key, true, &mut delegate_account_account),
            (&owner_key, false, &mut owner_account),
            (&token_key, false, &mut token_account),
            (&token_account_key, false, &mut token_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // create mismatch delegate account
        let instruction = Instruction::NewAccount;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (
                &mismatch_delegate_account_key,
                true,
                &mut mismatch_delegate_account_account,
            ),
            (&owner_key, false, &mut owner_account),
            (&token2_key, false, &mut token2_account),
            (&token_account_key, false, &mut token_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // create new token
        let instruction = Instruction::NewToken(TokenInfo {
            supply: 1000,
            decimals: 2,
        });
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&token_key, true, &mut token_account),
            (&token_account_key, false, &mut token_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // missing signer
        let instruction = Instruction::Transfer(1000);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, false, &mut owner_account),
            (&token_account_key, false, &mut token_account_account),
            (&token_account2_key, false, &mut token_account2_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // destination is delegate
        let instruction = Instruction::Transfer(1000);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&token_account2_key, false, &mut token_account2_account),
            (&delegate_account_key, false, &mut delegate_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(TokenError::DestinationIsDelegate.into()),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // mismatch token
        let instruction = Instruction::Transfer(1000);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&token_account2_key, false, &mut token_account2_account),
            (&mismatch_account_key, false, &mut mismatch_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(TokenError::TokenMismatch.into()),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // missing owner
        let instruction = Instruction::Transfer(1000);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner2_key, true, &mut owner2_account),
            (&token_account_key, false, &mut token_account_account),
            (&token_account2_key, false, &mut token_account2_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(TokenError::NoOwner.into()),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // transfer
        let instruction = Instruction::Transfer(1000);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&token_account_key, false, &mut token_account_account),
            (&token_account2_key, false, &mut token_account2_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // insufficient funds
        let instruction = Instruction::Transfer(1);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&token_account_key, false, &mut token_account_account),
            (&token_account2_key, false, &mut token_account2_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // transfer half back
        let instruction = Instruction::Transfer(500);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&token_account2_key, false, &mut token_account2_account),
            (&token_account_key, false, &mut token_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // transfer rest
        let instruction = Instruction::Transfer(500);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&token_account2_key, false, &mut token_account2_account),
            (&token_account_key, false, &mut token_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // insufficient funds
        let instruction = Instruction::Transfer(1);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&token_account2_key, false, &mut token_account2_account),
            (&token_account_key, false, &mut token_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // approve delegate
        let instruction = Instruction::Approve(100);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&token_account_key, false, &mut token_account_account),
            (&delegate_account_key, false, &mut delegate_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // not a delegate of source account
        let instruction = Instruction::Transfer(100);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&delegate_account_key, false, &mut delegate_account_account),
            (&token_account2_key, false, &mut token_account2_account),
            (&token_account3_key, false, &mut token_account3_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(TokenError::NotDelegate.into()),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // transfer via delegate
        let instruction = Instruction::Transfer(100);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&delegate_account_key, false, &mut delegate_account_account),
            (&token_account2_key, false, &mut token_account2_account),
            (&token_account_key, false, &mut token_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // insufficient funds approved via delegate
        let instruction = Instruction::Transfer(100);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&delegate_account_key, false, &mut delegate_account_account),
            (&token_account2_key, false, &mut token_account2_account),
            (&token_account_key, false, &mut token_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // transfer rest
        let instruction = Instruction::Transfer(900);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&token_account_key, false, &mut token_account_account),
            (&token_account2_key, false, &mut token_account2_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // approve delegate
        let instruction = Instruction::Approve(100);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&token_account_key, false, &mut token_account_account),
            (&delegate_account_key, false, &mut delegate_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // insufficient funds in source account via delegate
        let instruction = Instruction::Transfer(100);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&delegate_account_key, false, &mut delegate_account_account),
            (&token_account2_key, false, &mut token_account2_account),
            (&token_account_key, false, &mut token_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );
    }

    #[test]
    fn test_mintable_token_with_zero_supply() {
        let program_id = new_pubkey(1);
        let mut instruction_data = vec![0u8; size_of::<Instruction>()];
        let token_account_key = new_pubkey(2);
        let mut token_account_account = Account::new(0, size_of::<State>(), &program_id);
        let owner_key = new_pubkey(6);
        let mut owner_account = Account::default();
        let token_key = new_pubkey(8);
        let mut token_account = Account::new(0, size_of::<State>(), &program_id);

        // create account
        let instruction = Instruction::NewAccount;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&token_account_key, true, &mut token_account_account),
            (&owner_key, false, &mut owner_account),
            (&token_key, false, &mut token_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // create mintable token without owner
        let instruction = Instruction::NewToken(TokenInfo {
            supply: 0,
            decimals: 2,
        });
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![(&token_key, true, &mut token_account)];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(TokenError::OwnerRequiredIfNoInitialSupply.into()),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // create mintable token with zero supply
        let info = TokenInfo {
            supply: 0,
            decimals: 2,
        };
        let instruction = Instruction::NewToken(info);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&token_key, true, &mut token_account),
            (&owner_key, false, &mut owner_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        if let State::Token(token) = State::deserialize(&token_account.data).unwrap() {
            assert_eq!(
                token,
                Token {
                    info,
                    owner: Some(owner_key)
                }
            );
        } else {
            panic!("not an account");
        }

        // mint to
        let instruction = Instruction::MintTo(42);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&token_key, false, &mut token_account),
            (&token_account_key, false, &mut token_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        if let State::Token(token) = State::deserialize(&token_account.data).unwrap() {
            assert_eq!(token.info.supply, 42);
        } else {
            panic!("not an account");
        }
        if let State::Account(dest_token_account) =
            State::deserialize(&token_account_account.data).unwrap()
        {
            assert_eq!(dest_token_account.amount, 42);
        } else {
            panic!("not an account");
        }
    }

    #[test]
    fn test_approve() {
        let program_id = new_pubkey(1);
        let mut instruction_data = vec![0u8; size_of::<Instruction>()];
        let token_account_key = new_pubkey(2);
        let mut token_account_account = Account::new(0, size_of::<State>(), &program_id);
        let token_account2_key = new_pubkey(3);
        let mut token_account2_account = Account::new(0, size_of::<State>(), &program_id);
        let delegate_account_key = new_pubkey(4);
        let mut delegate_account_account = Account::new(0, size_of::<State>(), &program_id);
        let mismatch_delegate_account_key = new_pubkey(5);
        let mut mismatch_delegate_account_account =
            Account::new(0, size_of::<State>(), &program_id);
        let owner_key = new_pubkey(6);
        let mut owner_account = Account::default();
        let owner2_key = new_pubkey(7);
        let mut owner2_account = Account::default();
        let token_key = new_pubkey(8);
        let mut token_account = Account::new(0, size_of::<State>(), &program_id);
        let token2_key = new_pubkey(9);
        let mut token2_account = Account::new(0, size_of::<State>(), &program_id);

        // create account
        let instruction = Instruction::NewAccount;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&token_account_key, true, &mut token_account_account),
            (&owner_key, false, &mut owner_account),
            (&token_key, false, &mut token_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // create another account
        let instruction = Instruction::NewAccount;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&token_account2_key, true, &mut token_account2_account),
            (&owner_key, false, &mut owner_account),
            (&token_key, false, &mut token_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // create delegate account
        let instruction = Instruction::NewAccount;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&delegate_account_key, true, &mut delegate_account_account),
            (&owner_key, false, &mut owner_account),
            (&token_key, false, &mut token_account),
            (&token_account_key, false, &mut token_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // create mismatch delegate account
        let instruction = Instruction::NewAccount;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (
                &mismatch_delegate_account_key,
                true,
                &mut mismatch_delegate_account_account,
            ),
            (&owner_key, false, &mut owner_account),
            (&token2_key, false, &mut token2_account),
            (&token_account_key, false, &mut token_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // create new token
        let instruction = Instruction::NewToken(TokenInfo {
            supply: 1000,
            decimals: 2,
        });
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&token_key, true, &mut token_account),
            (&token_account_key, false, &mut token_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // token mismatch
        let instruction = Instruction::Approve(100);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&token_account_key, false, &mut token_account_account),
            (
                &mismatch_delegate_account_key,
                false,
                &mut mismatch_delegate_account_account,
            ),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(TokenError::TokenMismatch.into()),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // missing signer
        let instruction = Instruction::Approve(100);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, false, &mut owner_account),
            (&token_account_key, false, &mut token_account_account),
            (&delegate_account_key, false, &mut delegate_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // missing signer
        let instruction = Instruction::Approve(100);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner2_key, true, &mut owner2_account),
            (&token_account_key, false, &mut token_account_account),
            (&delegate_account_key, false, &mut delegate_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(TokenError::NoOwner.into()),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // destination is delegate
        let instruction = Instruction::Approve(100);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&delegate_account_key, false, &mut delegate_account_account),
            (&token_account_key, false, &mut token_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(ProgramError::InvalidArgument),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // not a delegate
        let instruction = Instruction::Approve(100);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&token_account2_key, false, &mut token_account2_account),
            (&token_account_key, false, &mut token_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(TokenError::NotDelegate.into()),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // not a delegate of source
        let instruction = Instruction::Approve(100);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&token_account2_key, false, &mut token_account2_account),
            (&delegate_account_key, false, &mut delegate_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(TokenError::NotDelegate.into()),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // approve delegate
        let instruction = Instruction::Approve(100);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&token_account_key, false, &mut token_account_account),
            (&delegate_account_key, false, &mut delegate_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();
    }

    #[test]
    fn test_set_owner() {
        let program_id = new_pubkey(1);
        let mut instruction_data = vec![0u8; size_of::<Instruction>()];
        let token_account_key = new_pubkey(2);
        let mut token_account_account = Account::new(0, size_of::<State>(), &program_id);
        let token_account2_key = new_pubkey(2);
        let mut token_account2_account = Account::new(0, size_of::<State>(), &program_id);
        let owner_key = new_pubkey(3);
        let mut owner_account = Account::default();
        let owner2_key = new_pubkey(4);
        let mut owner2_account = Account::default();
        let owner3_key = new_pubkey(5);
        let mut owner3_account = Account::default();
        let token_key = new_pubkey(8);
        let mut token_account = Account::new(0, size_of::<State>(), &program_id);
        let token2_key = new_pubkey(9);
        let mut token2_account = Account::new(0, size_of::<State>(), &program_id);

        // invalid account
        let instruction = Instruction::SetOwner;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, false, &mut owner_account),
            (&token_account_key, false, &mut token_account_account),
            (&owner2_key, false, &mut owner2_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(ProgramError::InvalidArgument),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // create account
        let instruction = Instruction::NewAccount;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&token_account_key, true, &mut token_account_account),
            (&owner_key, false, &mut owner_account),
            (&token_key, false, &mut token_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // create token account
        let instruction = Instruction::NewAccount;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&token_account2_key, true, &mut token_account2_account),
            (&owner_key, false, &mut owner_account),
            (&token2_key, false, &mut token2_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // missing owner
        let instruction = Instruction::SetOwner;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner2_key, false, &mut owner2_account),
            (&token_account_key, false, &mut token_account_account),
            (&owner3_key, false, &mut owner3_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(TokenError::NoOwner.into()),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // owner did not sign
        let mut accounts = vec![
            (&owner_key, false, &mut owner_account),
            (&token_account_key, true, &mut token_account_account),
            (&owner2_key, false, &mut owner2_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // set owner
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&token_account_key, true, &mut token_account_account),
            (&owner2_key, false, &mut owner2_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // create new token with owner
        let instruction = Instruction::NewToken(TokenInfo {
            supply: 1000,
            decimals: 2,
        });
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&token_key, true, &mut token_account),
            (&token_account_key, false, &mut token_account_account),
            (&owner_key, false, &mut owner_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // missing owner
        let instruction = Instruction::SetOwner;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner2_key, false, &mut owner2_account),
            (&token_key, false, &mut token_account),
            (&owner3_key, false, &mut owner3_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(TokenError::NoOwner.into()),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // owner did not sign
        let mut accounts = vec![
            (&owner_key, false, &mut owner_account),
            (&token_key, true, &mut token_account),
            (&owner2_key, false, &mut owner2_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // set owner
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&token_key, true, &mut token_account),
            (&owner2_key, false, &mut owner2_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // create new token without owner
        let instruction = Instruction::NewToken(TokenInfo {
            supply: 1000,
            decimals: 2,
        });
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&token2_key, true, &mut token2_account),
            (&token_account2_key, false, &mut token_account2_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // set owner for unownable token
        let instruction = Instruction::SetOwner;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&token2_key, true, &mut token2_account),
            (&owner2_key, false, &mut owner2_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(TokenError::NoOwner.into()),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );
    }

    #[test]
    fn test_mint_to() {
        let program_id = new_pubkey(1);
        let mut instruction_data = vec![0u8; size_of::<Instruction>()];
        let token_account_key = new_pubkey(2);
        let mut token_account_account = Account::new(0, size_of::<State>(), &program_id);
        let token_account2_key = new_pubkey(3);
        let mut token_account2_account = Account::new(0, size_of::<State>(), &program_id);
        let token_account3_key = new_pubkey(3);
        let mut token_account3_account = Account::new(0, size_of::<State>(), &program_id);
        let delegate_account_key = new_pubkey(4);
        let mut delegate_account_account = Account::new(0, size_of::<State>(), &program_id);
        let mismatch_account_key = new_pubkey(5);
        let mut mismatch_account_account = Account::new(0, size_of::<State>(), &program_id);
        let owner_key = new_pubkey(6);
        let mut owner_account = Account::default();
        let owner2_key = new_pubkey(7);
        let mut owner2_account = Account::default();
        let token_key = new_pubkey(8);
        let mut token_account = Account::new(0, size_of::<State>(), &program_id);
        let token2_key = new_pubkey(9);
        let mut token2_account = Account::new(0, size_of::<State>(), &program_id);
        let uninitialized_key = new_pubkey(9);
        let mut uninitialized_account = Account::new(0, size_of::<State>(), &program_id);

        // create token account
        let instruction = Instruction::NewAccount;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&token_account_key, true, &mut token_account_account),
            (&owner_key, false, &mut owner_account),
            (&token_key, false, &mut token_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // create another token account
        let instruction = Instruction::NewAccount;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&token_account2_key, true, &mut token_account2_account),
            (&owner_key, false, &mut owner_account),
            (&token_key, false, &mut token_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // create another token account
        let instruction = Instruction::NewAccount;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&token_account3_key, true, &mut token_account3_account),
            (&owner_key, false, &mut owner_account),
            (&token_key, false, &mut token_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // create mismatch token account
        let instruction = Instruction::NewAccount;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&mismatch_account_key, true, &mut mismatch_account_account),
            (&owner_key, false, &mut owner_account),
            (&token2_key, false, &mut token2_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // create delegate account
        let instruction = Instruction::NewAccount;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&delegate_account_key, true, &mut delegate_account_account),
            (&owner_key, false, &mut owner_account),
            (&token_key, false, &mut token_account),
            (&token_account_key, false, &mut token_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // create new token with owner
        let instruction = Instruction::NewToken(TokenInfo {
            supply: 1000,
            decimals: 2,
        });
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&token_key, true, &mut token_account),
            (&token_account_key, false, &mut token_account_account),
            (&owner_key, false, &mut owner_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // mint to
        let instruction = Instruction::MintTo(42);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&token_key, false, &mut token_account),
            (&token_account2_key, false, &mut token_account2_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        if let State::Token(token) = State::deserialize(&token_account.data).unwrap() {
            assert_eq!(token.info.supply, 1000 + 42);
        } else {
            panic!("not an account");
        }
        if let State::Account(dest_token_account) =
            State::deserialize(&token_account2_account.data).unwrap()
        {
            assert_eq!(dest_token_account.amount, 42);
        } else {
            panic!("not an account");
        }

        // missing signer
        let instruction = Instruction::MintTo(42);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, false, &mut owner_account),
            (&token_key, false, &mut token_account),
            (&token_account2_key, false, &mut token_account2_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // destination is delegate
        let instruction = Instruction::MintTo(42);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&token_key, false, &mut token_account),
            (&delegate_account_key, false, &mut delegate_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(TokenError::DestinationIsDelegate.into()),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // mismatch token
        let instruction = Instruction::MintTo(42);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&token_key, false, &mut token_account),
            (&mismatch_account_key, false, &mut mismatch_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(TokenError::TokenMismatch.into()),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // missing owner
        let instruction = Instruction::MintTo(42);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner2_key, true, &mut owner2_account),
            (&token_key, false, &mut token_account),
            (&token_account2_key, false, &mut token_account2_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(TokenError::NoOwner.into()),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // uninitialized destination account
        let instruction = Instruction::MintTo(42);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&token_key, false, &mut token_account),
            (&uninitialized_key, false, &mut uninitialized_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(ProgramError::InvalidArgument),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );
    }

    #[test]
    fn test_burn() {
        let program_id = new_pubkey(1);
        let mut instruction_data = vec![0u8; size_of::<Instruction>()];
        let token_account_key = new_pubkey(2);
        let mut token_account_account = Account::new(0, size_of::<State>(), &program_id);
        let token_account2_key = new_pubkey(3);
        let mut token_account2_account = Account::new(0, size_of::<State>(), &program_id);
        let token_account3_key = new_pubkey(3);
        let mut token_account3_account = Account::new(0, size_of::<State>(), &program_id);
        let delegate_account_key = new_pubkey(4);
        let mut delegate_account_account = Account::new(0, size_of::<State>(), &program_id);
        let mismatch_account_key = new_pubkey(5);
        let mut mismatch_account_account = Account::new(0, size_of::<State>(), &program_id);
        let mismatch_delegate_account_key = new_pubkey(5);
        let mut mismatch_delegate_account_account =
            Account::new(0, size_of::<State>(), &program_id);
        let owner_key = new_pubkey(6);
        let mut owner_account = Account::default();
        let owner2_key = new_pubkey(7);
        let mut owner2_account = Account::default();
        let token_key = new_pubkey(8);
        let mut token_account = Account::new(0, size_of::<State>(), &program_id);
        let token2_key = new_pubkey(9);
        let mut token2_account = Account::new(0, size_of::<State>(), &program_id);

        // create token account
        let instruction = Instruction::NewAccount;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&token_account_key, true, &mut token_account_account),
            (&owner_key, false, &mut owner_account),
            (&token_key, false, &mut token_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // create another token account
        let instruction = Instruction::NewAccount;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&token_account2_key, true, &mut token_account2_account),
            (&owner_key, false, &mut owner_account),
            (&token_key, false, &mut token_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // create another token account
        let instruction = Instruction::NewAccount;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&token_account3_key, true, &mut token_account3_account),
            (&owner_key, false, &mut owner_account),
            (&token_key, false, &mut token_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // create mismatch token account
        let instruction = Instruction::NewAccount;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&mismatch_account_key, true, &mut mismatch_account_account),
            (&owner_key, false, &mut owner_account),
            (&token2_key, false, &mut token2_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // create delegate account
        let instruction = Instruction::NewAccount;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&delegate_account_key, true, &mut delegate_account_account),
            (&owner_key, false, &mut owner_account),
            (&token_key, false, &mut token_account),
            (&token_account_key, false, &mut token_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // create mismatch delegate account
        let instruction = Instruction::NewAccount;
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (
                &mismatch_delegate_account_key,
                true,
                &mut mismatch_delegate_account_account,
            ),
            (&owner_key, false, &mut owner_account),
            (&token2_key, false, &mut token2_account),
            (&token_account_key, false, &mut token_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // create new token
        let instruction = Instruction::NewToken(TokenInfo {
            supply: 1000,
            decimals: 2,
        });
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&token_key, true, &mut token_account),
            (&token_account_key, false, &mut token_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // missing signer
        let instruction = Instruction::Burn(42);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, false, &mut owner_account),
            (&token_account_key, false, &mut token_account_account),
            (&token_key, false, &mut token_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // mismatch token
        let instruction = Instruction::Burn(42);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&mismatch_account_key, false, &mut mismatch_account_account),
            (&token_key, false, &mut token_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(TokenError::TokenMismatch.into()),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // missing owner
        let instruction = Instruction::Burn(42);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner2_key, true, &mut owner2_account),
            (&token_account_key, false, &mut token_account_account),
            (&token_key, false, &mut token_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(TokenError::NoOwner.into()),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // burn
        let instruction = Instruction::Burn(42);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&token_account_key, false, &mut token_account_account),
            (&token_key, false, &mut token_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        if let State::Token(token) = State::deserialize(&token_account.data).unwrap() {
            assert_eq!(token.info.supply, 1000 - 42);
        } else {
            panic!("not a token account");
        }
        if let State::Account(account) = State::deserialize(&token_account_account.data).unwrap() {
            assert_eq!(account.amount, 1000 - 42);
        } else {
            panic!("not an account");
        }

        // insufficient funds
        let instruction = Instruction::Burn(100000000);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&token_account_key, false, &mut token_account_account),
            (&token_key, false, &mut token_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // approve delegate
        let instruction = Instruction::Approve(84);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&token_account_key, false, &mut token_account_account),
            (&delegate_account_key, false, &mut delegate_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        // not a delegate of source account
        let instruction = Instruction::Burn(84);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&delegate_account_key, false, &mut delegate_account_account),
            (&token_key, false, &mut token_account),
            (&token_account2_key, false, &mut token_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(TokenError::NotDelegate.into()),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );

        // burn via delegate
        let instruction = Instruction::Burn(84);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&delegate_account_key, false, &mut delegate_account_account),
            (&token_key, false, &mut token_account),
            (&token_account_key, false, &mut token_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        State::process(&program_id, &mut account_infos, &instruction_data).unwrap();

        if let State::Token(token) = State::deserialize(&token_account.data).unwrap() {
            assert_eq!(token.info.supply, 1000 - 42 - 84);
        } else {
            panic!("not a token account");
        }
        if let State::Account(account) = State::deserialize(&token_account_account.data).unwrap() {
            assert_eq!(account.amount, 1000 - 42 - 84);
        } else {
            panic!("not an account");
        }

        // insufficient funds approved via delegate
        let instruction = Instruction::Burn(100);
        instruction.serialize(&mut instruction_data).unwrap();
        let mut accounts = vec![
            (&owner_key, true, &mut owner_account),
            (&delegate_account_key, false, &mut delegate_account_account),
            (&token_key, false, &mut token_account),
            (&token_account_key, false, &mut token_account_account),
        ];
        let mut account_infos = create_is_signer_account_infos(&mut accounts);
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            State::process(&program_id, &mut account_infos, &instruction_data)
        );
    }
}
