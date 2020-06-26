//! State transition types

use crate::{
    error::TokenError,
    instruction::{TokenInfo, TokenInstruction},
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

impl State {
    /// Processes a [NewToken](enum.TokenInstruction.html) instruction.
    pub fn process_new_token(accounts: &[AccountInfo], info: TokenInfo) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
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

    /// Processes a [NewAccount](enum.TokenInstruction.html) instruction.
    pub fn process_new_account(accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
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

    /// Processes a [Transfer](enum.TokenInstruction.html) instruction.
    pub fn process_transfer(accounts: &[AccountInfo], amount: u64) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
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

    /// Processes an [Approve](enum.TokenInstruction.html) instruction.
    pub fn process_approve(accounts: &[AccountInfo], amount: u64) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
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

    /// Processes a [SetOwner](enum.TokenInstruction.html) instruction.
    pub fn process_set_owner(accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
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

    /// Processes a [MintTo](enum.TokenInstruction.html) instruction.
    pub fn process_mintto(accounts: &[AccountInfo], amount: u64) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
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

    /// Processes a [Burn](enum.TokenInstruction.html) instruction.
    pub fn process_burn(accounts: &[AccountInfo], amount: u64) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
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
    pub fn process(_program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        let instruction = TokenInstruction::deserialize(input)?;

        match instruction {
            TokenInstruction::NewToken(info) => {
                info!("Instruction: NewToken");
                Self::process_new_token(accounts, info)
            }
            TokenInstruction::NewAccount => {
                info!("Instruction: NewAccount");
                Self::process_new_account(accounts)
            }
            TokenInstruction::Transfer(amount) => {
                info!("Instruction: Transfer");
                Self::process_transfer(accounts, amount)
            }
            TokenInstruction::Approve(amount) => {
                info!("Instruction: Approve");
                Self::process_approve(accounts, amount)
            }
            TokenInstruction::SetOwner => {
                info!("Instruction: SetOwner");
                Self::process_set_owner(accounts)
            }
            TokenInstruction::MintTo(amount) => {
                info!("Instruction: MintTo");
                Self::process_mintto(accounts, amount)
            }
            TokenInstruction::Burn(amount) => {
                info!("Instruction: Burn");
                Self::process_burn(accounts, amount)
            }
        }
    }

    /// Deserializes a byte buffer into a Token Program [State](struct.State.html)
    pub fn deserialize(input: &[u8]) -> Result<Self, ProgramError> {
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

// Pulls in the stubs required for `info!()`
#[cfg(not(target_arch = "bpf"))]
solana_sdk_bpf_test::stubs!();

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::{approve, burn, mint_to, new_account, new_token, set_owner, transfer};
    use solana_sdk::{
        account::Account, account_info::create_is_signer_account_infos, instruction::Instruction,
    };

    fn pubkey_rand() -> Pubkey {
        Pubkey::new(&rand::random::<[u8; 32]>())
    }

    fn do_process_instruction(
        instruction: Instruction,
        accounts: Vec<&mut Account>,
    ) -> ProgramResult {
        let mut meta = instruction
            .accounts
            .iter()
            .zip(accounts)
            .map(|(account_meta, account)| (&account_meta.pubkey, account_meta.is_signer, account))
            .collect::<Vec<_>>();

        let account_infos = create_is_signer_account_infos(&mut meta);
        State::process(&instruction.program_id, &account_infos, &instruction.data)
    }

    #[test]
    fn test_new_token() {
        let program_id = pubkey_rand();
        let token_account_key = pubkey_rand();
        let mut token_account_account = Account::new(0, size_of::<State>(), &program_id);
        let token_account2_key = pubkey_rand();
        let mut token_account2_account = Account::new(0, size_of::<State>(), &program_id);
        let delegate_account_key = pubkey_rand();
        let mut delegate_account_account = Account::new(0, size_of::<State>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = Account::default();
        let token_key = pubkey_rand();
        let mut token_account = Account::new(0, size_of::<State>(), &program_id);
        let token2_key = pubkey_rand();
        let mut token2_account = Account::new(0, size_of::<State>(), &program_id);

        // account not created
        assert_eq!(
            Err(ProgramError::InvalidArgument),
            do_process_instruction(
                new_token(
                    &program_id,
                    &token_key,
                    Some(&token_account_key),
                    None,
                    TokenInfo {
                        supply: 1000,
                        decimals: 2,
                    }
                )
                .unwrap(),
                vec![&mut token_account, &mut token_account_account]
            )
        );

        // create account
        do_process_instruction(
            new_account(
                &program_id,
                &token_account_key,
                &owner_key,
                &token_key,
                None,
            )
            .unwrap(),
            vec![
                &mut token_account_account,
                &mut owner_account,
                &mut token_account,
            ],
        )
        .unwrap();

        // create new token
        do_process_instruction(
            new_token(
                &program_id,
                &token_key,
                Some(&token_account_key),
                None,
                TokenInfo {
                    supply: 1000,
                    decimals: 2,
                },
            )
            .unwrap(),
            vec![&mut token_account, &mut token_account_account],
        )
        .unwrap();

        // create another account
        do_process_instruction(
            new_account(
                &program_id,
                &token_account2_key,
                &owner_key,
                &token_key,
                None,
            )
            .unwrap(),
            vec![
                &mut token_account2_account,
                &mut owner_account,
                &mut token_account,
            ],
        )
        .unwrap();

        // token mismatch
        assert_eq!(
            Err(TokenError::TokenMismatch.into()),
            do_process_instruction(
                new_token(
                    &program_id,
                    &token2_key,
                    Some(&token_account2_key),
                    None,
                    TokenInfo {
                        supply: 1000,
                        decimals: 2,
                    },
                )
                .unwrap(),
                vec![&mut token2_account, &mut token_account2_account]
            )
        );

        // create delegate account
        do_process_instruction(
            new_account(
                &program_id,
                &delegate_account_key,
                &owner_key,
                &token_key,
                Some(&token_account_key),
            )
            .unwrap(),
            vec![
                &mut delegate_account_account,
                &mut owner_account,
                &mut token_account,
                &mut token_account_account,
            ],
        )
        .unwrap();

        // account is a delegate token
        assert_eq!(
            Err(TokenError::AlreadyInUse.into()),
            do_process_instruction(
                new_token(
                    &program_id,
                    &token_key,
                    Some(&delegate_account_key),
                    None,
                    TokenInfo {
                        supply: 1000,
                        decimals: 2,
                    },
                )
                .unwrap(),
                vec![&mut token_account, &mut delegate_account_account]
            )
        );

        // create twice
        assert_eq!(
            Err(TokenError::AlreadyInUse.into()),
            do_process_instruction(
                new_token(
                    &program_id,
                    &token_key,
                    Some(&token_account_key),
                    None,
                    TokenInfo {
                        supply: 1000,
                        decimals: 2,
                    },
                )
                .unwrap(),
                vec![&mut token_account, &mut token_account_account]
            )
        );
    }

    #[test]
    fn test_new_token_account() {
        let program_id = pubkey_rand();
        let token_account_key = pubkey_rand();
        let mut token_account_account = Account::new(0, size_of::<State>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = Account::default();
        let token_key = pubkey_rand();
        let mut token_account = Account::new(0, size_of::<State>(), &program_id);

        // missing signer
        let mut instruction = new_account(
            &program_id,
            &token_account_key,
            &owner_key,
            &token_key,
            None,
        )
        .unwrap();
        instruction.accounts[0].is_signer = false;
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            do_process_instruction(
                instruction,
                vec![
                    &mut token_account_account,
                    &mut owner_account,
                    &mut token_account,
                ],
            )
        );

        // create account
        do_process_instruction(
            new_account(
                &program_id,
                &token_account_key,
                &owner_key,
                &token_key,
                None,
            )
            .unwrap(),
            vec![
                &mut token_account_account,
                &mut owner_account,
                &mut token_account,
            ],
        )
        .unwrap();

        // create twice
        assert_eq!(
            Err(TokenError::AlreadyInUse.into()),
            do_process_instruction(
                new_account(
                    &program_id,
                    &token_account_key,
                    &owner_key,
                    &token_key,
                    None,
                )
                .unwrap(),
                vec![
                    &mut token_account_account,
                    &mut owner_account,
                    &mut token_account,
                ],
            )
        );
    }

    #[test]
    fn test_transfer() {
        let program_id = pubkey_rand();
        let token_account_key = pubkey_rand();
        let mut token_account_account = Account::new(0, size_of::<State>(), &program_id);
        let token_account2_key = pubkey_rand();
        let mut token_account2_account = Account::new(0, size_of::<State>(), &program_id);
        let token_account3_key = pubkey_rand();
        let mut token_account3_account = Account::new(0, size_of::<State>(), &program_id);
        let delegate_account_key = pubkey_rand();
        let mut delegate_account_account = Account::new(0, size_of::<State>(), &program_id);
        let mismatch_account_key = pubkey_rand();
        let mut mismatch_account_account = Account::new(0, size_of::<State>(), &program_id);
        let mismatch_delegate_account_key = pubkey_rand();
        let mut mismatch_delegate_account_account =
            Account::new(0, size_of::<State>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = Account::default();
        let owner2_key = pubkey_rand();
        let mut owner2_account = Account::default();
        let token_key = pubkey_rand();
        let mut token_account = Account::new(0, size_of::<State>(), &program_id);
        let token2_key = pubkey_rand();
        let mut token2_account = Account::new(0, size_of::<State>(), &program_id);

        // create account
        do_process_instruction(
            new_account(
                &program_id,
                &token_account_key,
                &owner_key,
                &token_key,
                None,
            )
            .unwrap(),
            vec![
                &mut token_account_account,
                &mut owner_account,
                &mut token_account,
            ],
        )
        .unwrap();

        // create another account
        do_process_instruction(
            new_account(
                &program_id,
                &token_account2_key,
                &owner_key,
                &token_key,
                None,
            )
            .unwrap(),
            vec![
                &mut token_account2_account,
                &mut owner_account,
                &mut token_account,
            ],
        )
        .unwrap();

        // create another account
        do_process_instruction(
            new_account(
                &program_id,
                &token_account3_key,
                &owner_key,
                &token_key,
                None,
            )
            .unwrap(),
            vec![
                &mut token_account3_account,
                &mut owner_account,
                &mut token_account,
            ],
        )
        .unwrap();

        // create mismatch account
        do_process_instruction(
            new_account(
                &program_id,
                &mismatch_account_key,
                &owner_key,
                &token2_key,
                None,
            )
            .unwrap(),
            vec![
                &mut mismatch_account_account,
                &mut owner_account,
                &mut token2_account,
            ],
        )
        .unwrap();

        // create delegate account
        do_process_instruction(
            new_account(
                &program_id,
                &delegate_account_key,
                &owner_key,
                &token_key,
                Some(&token_account_key),
            )
            .unwrap(),
            vec![
                &mut delegate_account_account,
                &mut owner_account,
                &mut token_account,
                &mut token_account_account,
            ],
        )
        .unwrap();

        // create mismatch delegate account
        do_process_instruction(
            new_account(
                &program_id,
                &mismatch_delegate_account_key,
                &owner_key,
                &token2_key,
                Some(&token_account_key),
            )
            .unwrap(),
            vec![
                &mut mismatch_delegate_account_account,
                &mut owner_account,
                &mut token2_account,
                &mut token_account_account,
            ],
        )
        .unwrap();

        // create new token
        do_process_instruction(
            new_token(
                &program_id,
                &token_key,
                Some(&token_account_key),
                None,
                TokenInfo {
                    supply: 1000,
                    decimals: 2,
                },
            )
            .unwrap(),
            vec![&mut token_account, &mut token_account_account],
        )
        .unwrap();

        // missing signer
        let mut instruction = transfer(
            &program_id,
            &owner_key,
            &token_account_key,
            &token_account2_key,
            None,
            1000,
        )
        .unwrap();
        instruction.accounts[0].is_signer = false;
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            do_process_instruction(
                instruction,
                vec![
                    &mut owner_account,
                    &mut token_account_account,
                    &mut token_account2_account,
                ],
            )
        );

        // destination is delegate
        assert_eq!(
            Err(TokenError::DestinationIsDelegate.into()),
            do_process_instruction(
                transfer(
                    &program_id,
                    &owner_key,
                    &token_account2_key,
                    &delegate_account_key,
                    None,
                    1000,
                )
                .unwrap(),
                vec![
                    &mut owner_account,
                    &mut token_account2_account,
                    &mut delegate_account_account,
                ],
            )
        );

        // mismatch token
        assert_eq!(
            Err(TokenError::TokenMismatch.into()),
            do_process_instruction(
                transfer(
                    &program_id,
                    &owner_key,
                    &token_account2_key,
                    &mismatch_account_key,
                    None,
                    1000,
                )
                .unwrap(),
                vec![
                    &mut owner_account,
                    &mut token_account2_account,
                    &mut mismatch_account_account,
                ],
            )
        );

        // missing owner
        assert_eq!(
            Err(TokenError::NoOwner.into()),
            do_process_instruction(
                transfer(
                    &program_id,
                    &owner2_key,
                    &token_account_key,
                    &token_account2_key,
                    None,
                    1000,
                )
                .unwrap(),
                vec![
                    &mut owner2_account,
                    &mut token_account_account,
                    &mut token_account2_account,
                ],
            )
        );

        // transfer
        do_process_instruction(
            transfer(
                &program_id,
                &owner_key,
                &token_account_key,
                &token_account2_key,
                None,
                1000,
            )
            .unwrap(),
            vec![
                &mut owner_account,
                &mut token_account_account,
                &mut token_account2_account,
            ],
        )
        .unwrap();

        // insufficient funds
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            do_process_instruction(
                transfer(
                    &program_id,
                    &owner_key,
                    &token_account_key,
                    &token_account2_key,
                    None,
                    1,
                )
                .unwrap(),
                vec![
                    &mut owner_account,
                    &mut token_account_account,
                    &mut token_account2_account,
                ],
            )
        );

        // transfer half back
        do_process_instruction(
            transfer(
                &program_id,
                &owner_key,
                &token_account2_key,
                &token_account_key,
                None,
                500,
            )
            .unwrap(),
            vec![
                &mut owner_account,
                &mut token_account2_account,
                &mut token_account_account,
            ],
        )
        .unwrap();

        // transfer rest
        do_process_instruction(
            transfer(
                &program_id,
                &owner_key,
                &token_account2_key,
                &token_account_key,
                None,
                500,
            )
            .unwrap(),
            vec![
                &mut owner_account,
                &mut token_account2_account,
                &mut token_account_account,
            ],
        )
        .unwrap();

        // insufficient funds
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            do_process_instruction(
                transfer(
                    &program_id,
                    &owner_key,
                    &token_account2_key,
                    &token_account_key,
                    None,
                    1,
                )
                .unwrap(),
                vec![
                    &mut owner_account,
                    &mut token_account2_account,
                    &mut token_account_account,
                ],
            )
        );

        // approve delegate
        do_process_instruction(
            approve(
                &program_id,
                &owner_key,
                &token_account_key,
                &delegate_account_key,
                100,
            )
            .unwrap(),
            vec![
                &mut owner_account,
                &mut token_account_account,
                &mut delegate_account_account,
            ],
        )
        .unwrap();

        // not a delegate of source account
        assert_eq!(
            Err(TokenError::NotDelegate.into()),
            do_process_instruction(
                transfer(
                    &program_id,
                    &owner_key,
                    &delegate_account_key,
                    &token_account2_key,
                    Some(&token_account3_key),
                    100,
                )
                .unwrap(),
                vec![
                    &mut owner_account,
                    &mut delegate_account_account,
                    &mut token_account2_account,
                    &mut token_account3_account
                ],
            )
        );

        // transfer via delegate
        do_process_instruction(
            transfer(
                &program_id,
                &owner_key,
                &delegate_account_key,
                &token_account2_key,
                Some(&token_account_key),
                100,
            )
            .unwrap(),
            vec![
                &mut owner_account,
                &mut delegate_account_account,
                &mut token_account2_account,
                &mut token_account_account,
            ],
        )
        .unwrap();

        // insufficient funds approved via delegate
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            do_process_instruction(
                transfer(
                    &program_id,
                    &owner_key,
                    &delegate_account_key,
                    &token_account2_key,
                    Some(&token_account_key),
                    100,
                )
                .unwrap(),
                vec![
                    &mut owner_account,
                    &mut delegate_account_account,
                    &mut token_account2_account,
                    &mut token_account_account
                ],
            )
        );

        // transfer rest
        do_process_instruction(
            transfer(
                &program_id,
                &owner_key,
                &token_account_key,
                &token_account2_key,
                None,
                900,
            )
            .unwrap(),
            vec![
                &mut owner_account,
                &mut token_account_account,
                &mut token_account2_account,
            ],
        )
        .unwrap();

        // approve delegate
        do_process_instruction(
            approve(
                &program_id,
                &owner_key,
                &token_account_key,
                &delegate_account_key,
                100,
            )
            .unwrap(),
            vec![
                &mut owner_account,
                &mut token_account_account,
                &mut delegate_account_account,
            ],
        )
        .unwrap();

        // insufficient funds in source account via delegate
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            do_process_instruction(
                transfer(
                    &program_id,
                    &owner_key,
                    &delegate_account_key,
                    &token_account2_key,
                    Some(&token_account_key),
                    100,
                )
                .unwrap(),
                vec![
                    &mut owner_account,
                    &mut delegate_account_account,
                    &mut token_account2_account,
                    &mut token_account_account
                ],
            )
        );
    }

    #[test]
    fn test_mintable_token_with_zero_supply() {
        let program_id = pubkey_rand();
        let token_account_key = pubkey_rand();
        let mut token_account_account = Account::new(0, size_of::<State>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = Account::default();
        let token_key = pubkey_rand();
        let mut token_account = Account::new(0, size_of::<State>(), &program_id);

        // create account
        do_process_instruction(
            new_account(
                &program_id,
                &token_account_key,
                &owner_key,
                &token_key,
                None,
            )
            .unwrap(),
            vec![
                &mut token_account_account,
                &mut owner_account,
                &mut token_account,
            ],
        )
        .unwrap();

        // create mintable token without owner
        let mut instruction = new_token(
            &program_id,
            &token_key,
            None,
            Some(&owner_key),
            TokenInfo {
                supply: 0,
                decimals: 2,
            },
        )
        .unwrap();
        instruction.accounts.pop();
        assert_eq!(
            Err(TokenError::OwnerRequiredIfNoInitialSupply.into()),
            do_process_instruction(instruction, vec![&mut token_account])
        );

        // create mintable token with zero supply
        let info = TokenInfo {
            supply: 0,
            decimals: 2,
        };
        do_process_instruction(
            new_token(&program_id, &token_key, None, Some(&owner_key), info).unwrap(),
            vec![&mut token_account, &mut token_account_account],
        )
        .unwrap();
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
        do_process_instruction(
            mint_to(&program_id, &owner_key, &token_key, &token_account_key, 42).unwrap(),
            vec![
                &mut owner_account,
                &mut token_account,
                &mut token_account_account,
            ],
        )
        .unwrap();

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
        let program_id = pubkey_rand();
        let token_account_key = pubkey_rand();
        let mut token_account_account = Account::new(0, size_of::<State>(), &program_id);
        let token_account2_key = pubkey_rand();
        let mut token_account2_account = Account::new(0, size_of::<State>(), &program_id);
        let delegate_account_key = pubkey_rand();
        let mut delegate_account_account = Account::new(0, size_of::<State>(), &program_id);
        let mismatch_delegate_account_key = pubkey_rand();
        let mut mismatch_delegate_account_account =
            Account::new(0, size_of::<State>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = Account::default();
        let owner2_key = pubkey_rand();
        let mut owner2_account = Account::default();
        let token_key = pubkey_rand();
        let mut token_account = Account::new(0, size_of::<State>(), &program_id);
        let token2_key = pubkey_rand();
        let mut token2_account = Account::new(0, size_of::<State>(), &program_id);

        // create account
        do_process_instruction(
            new_account(
                &program_id,
                &token_account_key,
                &owner_key,
                &token_key,
                None,
            )
            .unwrap(),
            vec![
                &mut token_account_account,
                &mut owner_account,
                &mut token_account,
            ],
        )
        .unwrap();

        // create another account
        do_process_instruction(
            new_account(
                &program_id,
                &token_account2_key,
                &owner_key,
                &token_key,
                None,
            )
            .unwrap(),
            vec![
                &mut token_account2_account,
                &mut owner_account,
                &mut token_account,
            ],
        )
        .unwrap();

        // create delegate account
        do_process_instruction(
            new_account(
                &program_id,
                &delegate_account_key,
                &owner_key,
                &token_key,
                Some(&token_account_key),
            )
            .unwrap(),
            vec![
                &mut delegate_account_account,
                &mut owner_account,
                &mut token_account,
                &mut token_account_account,
            ],
        )
        .unwrap();

        // create mismatch delegate account
        do_process_instruction(
            new_account(
                &program_id,
                &mismatch_delegate_account_key,
                &owner_key,
                &token2_key,
                Some(&token_account_key),
            )
            .unwrap(),
            vec![
                &mut mismatch_delegate_account_account,
                &mut owner_account,
                &mut token2_account,
                &mut token_account_account,
            ],
        )
        .unwrap();

        // create new token
        do_process_instruction(
            new_token(
                &program_id,
                &token_key,
                Some(&token_account_key),
                None,
                TokenInfo {
                    supply: 1000,
                    decimals: 2,
                },
            )
            .unwrap(),
            vec![&mut token_account, &mut token_account_account],
        )
        .unwrap();

        // token mismatch
        assert_eq!(
            Err(TokenError::TokenMismatch.into()),
            do_process_instruction(
                approve(
                    &program_id,
                    &owner_key,
                    &token_account_key,
                    &mismatch_delegate_account_key,
                    100,
                )
                .unwrap(),
                vec![
                    &mut owner_account,
                    &mut token_account_account,
                    &mut mismatch_delegate_account_account,
                ],
            )
        );

        // missing signer
        let mut instruction = approve(
            &program_id,
            &owner_key,
            &token_account_key,
            &delegate_account_key,
            100,
        )
        .unwrap();
        instruction.accounts[0].is_signer = false;
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            do_process_instruction(
                instruction,
                vec![
                    &mut owner_account,
                    &mut token_account_account,
                    &mut delegate_account_account,
                ],
            )
        );

        // no owner
        assert_eq!(
            Err(TokenError::NoOwner.into()),
            do_process_instruction(
                approve(
                    &program_id,
                    &owner2_key,
                    &token_account_key,
                    &delegate_account_key,
                    100,
                )
                .unwrap(),
                vec![
                    &mut owner2_account,
                    &mut token_account_account,
                    &mut delegate_account_account,
                ],
            )
        );

        // destination is delegate
        assert_eq!(
            Err(ProgramError::InvalidArgument),
            do_process_instruction(
                approve(
                    &program_id,
                    &owner_key,
                    &delegate_account_key,
                    &token_account_key,
                    100,
                )
                .unwrap(),
                vec![
                    &mut owner_account,
                    &mut delegate_account_account,
                    &mut token_account_account,
                ],
            )
        );

        // not a delegate
        assert_eq!(
            Err(TokenError::NotDelegate.into()),
            do_process_instruction(
                approve(
                    &program_id,
                    &owner_key,
                    &token_account2_key,
                    &token_account_key,
                    100,
                )
                .unwrap(),
                vec![
                    &mut owner_account,
                    &mut token_account2_account,
                    &mut token_account_account,
                ],
            )
        );

        // not a delegate of source
        assert_eq!(
            Err(TokenError::NotDelegate.into()),
            do_process_instruction(
                approve(
                    &program_id,
                    &owner_key,
                    &token_account2_key,
                    &delegate_account_key,
                    100,
                )
                .unwrap(),
                vec![
                    &mut owner_account,
                    &mut token_account2_account,
                    &mut delegate_account_account,
                ],
            )
        );

        // approve delegate
        do_process_instruction(
            approve(
                &program_id,
                &owner_key,
                &token_account_key,
                &delegate_account_key,
                100,
            )
            .unwrap(),
            vec![
                &mut owner_account,
                &mut token_account_account,
                &mut delegate_account_account,
            ],
        )
        .unwrap();
    }

    #[test]
    fn test_set_owner() {
        let program_id = pubkey_rand();
        let token_account_key = pubkey_rand();
        let mut token_account_account = Account::new(0, size_of::<State>(), &program_id);
        let token_account2_key = pubkey_rand();
        let mut token_account2_account = Account::new(0, size_of::<State>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = Account::default();
        let owner2_key = pubkey_rand();
        let mut owner2_account = Account::default();
        let owner3_key = pubkey_rand();
        let mut owner3_account = Account::default();
        let token_key = pubkey_rand();
        let mut token_account = Account::new(0, size_of::<State>(), &program_id);
        let token2_key = pubkey_rand();
        let mut token2_account = Account::new(0, size_of::<State>(), &program_id);

        // invalid account
        assert_eq!(
            Err(ProgramError::InvalidArgument),
            do_process_instruction(
                set_owner(&program_id, &owner_key, &token_account_key, &owner2_key,).unwrap(),
                vec![
                    &mut owner_account,
                    &mut token_account_account,
                    &mut owner2_account,
                ],
            )
        );

        // create account
        do_process_instruction(
            new_account(
                &program_id,
                &token_account_key,
                &owner_key,
                &token_key,
                None,
            )
            .unwrap(),
            vec![
                &mut token_account_account,
                &mut owner_account,
                &mut token_account,
            ],
        )
        .unwrap();

        // create token account
        do_process_instruction(
            new_account(
                &program_id,
                &token_account2_key,
                &owner_key,
                &token2_key,
                None,
            )
            .unwrap(),
            vec![
                &mut token_account2_account,
                &mut owner_account,
                &mut token2_account,
            ],
        )
        .unwrap();

        // missing owner
        assert_eq!(
            Err(TokenError::NoOwner.into()),
            do_process_instruction(
                set_owner(&program_id, &owner2_key, &token_account_key, &owner_key,).unwrap(),
                vec![
                    &mut owner2_account,
                    &mut token_account_account,
                    &mut owner_account,
                ],
            )
        );

        // owner did not sign
        let mut instruction =
            set_owner(&program_id, &owner_key, &token_account_key, &owner2_key).unwrap();
        instruction.accounts[0].is_signer = false;
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            do_process_instruction(
                instruction,
                vec![
                    &mut owner_account,
                    &mut token_account_account,
                    &mut owner2_account,
                ],
            )
        );

        // set owner
        do_process_instruction(
            set_owner(&program_id, &owner_key, &token_account_key, &owner2_key).unwrap(),
            vec![
                &mut owner_account,
                &mut token_account_account,
                &mut owner2_account,
            ],
        )
        .unwrap();

        // create new token with owner
        do_process_instruction(
            new_token(
                &program_id,
                &token_key,
                Some(&token_account_key),
                Some(&owner_key),
                TokenInfo {
                    supply: 1000,
                    decimals: 2,
                },
            )
            .unwrap(),
            vec![
                &mut token_account,
                &mut token_account_account,
                &mut owner_account,
            ],
        )
        .unwrap();

        // wrong account
        assert_eq!(
            Err(TokenError::NoOwner.into()),
            do_process_instruction(
                set_owner(&program_id, &owner2_key, &token_key, &owner3_key,).unwrap(),
                vec![&mut owner2_account, &mut token_account, &mut owner3_account,],
            )
        );

        // owner did not sign
        let mut instruction = set_owner(&program_id, &owner_key, &token_key, &owner2_key).unwrap();
        instruction.accounts[0].is_signer = false;
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            do_process_instruction(
                instruction,
                vec![&mut owner_account, &mut token_account, &mut owner2_account,],
            )
        );

        // set owner
        do_process_instruction(
            set_owner(&program_id, &owner_key, &token_key, &owner2_key).unwrap(),
            vec![&mut owner_account, &mut token_account, &mut owner2_account],
        )
        .unwrap();

        // create new token without owner
        do_process_instruction(
            new_token(
                &program_id,
                &token2_key,
                Some(&token_account2_key),
                None,
                TokenInfo {
                    supply: 1000,
                    decimals: 2,
                },
            )
            .unwrap(),
            vec![&mut token2_account, &mut token_account2_account],
        )
        .unwrap();

        // set owner for unownable token
        assert_eq!(
            Err(TokenError::NoOwner.into()),
            do_process_instruction(
                set_owner(&program_id, &owner_key, &token2_key, &owner2_key,).unwrap(),
                vec![&mut owner_account, &mut token_account, &mut owner2_account,],
            )
        );
    }

    #[test]
    fn test_mint_to() {
        let program_id = pubkey_rand();
        let token_account_key = pubkey_rand();
        let mut token_account_account = Account::new(0, size_of::<State>(), &program_id);
        let token_account2_key = pubkey_rand();
        let mut token_account2_account = Account::new(0, size_of::<State>(), &program_id);
        let token_account3_key = pubkey_rand();
        let mut token_account3_account = Account::new(0, size_of::<State>(), &program_id);
        let delegate_account_key = pubkey_rand();
        let mut delegate_account_account = Account::new(0, size_of::<State>(), &program_id);
        let mismatch_account_key = pubkey_rand();
        let mut mismatch_account_account = Account::new(0, size_of::<State>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = Account::default();
        let owner2_key = pubkey_rand();
        let mut owner2_account = Account::default();
        let token_key = pubkey_rand();
        let mut token_account = Account::new(0, size_of::<State>(), &program_id);
        let token2_key = pubkey_rand();
        let mut token2_account = Account::new(0, size_of::<State>(), &program_id);
        let uninitialized_key = pubkey_rand();
        let mut uninitialized_account = Account::new(0, size_of::<State>(), &program_id);

        // create token account
        do_process_instruction(
            new_account(
                &program_id,
                &token_account_key,
                &owner_key,
                &token_key,
                None,
            )
            .unwrap(),
            vec![
                &mut token_account_account,
                &mut owner_account,
                &mut token_account,
            ],
        )
        .unwrap();

        // create another token account
        do_process_instruction(
            new_account(
                &program_id,
                &token_account2_key,
                &owner_key,
                &token_key,
                None,
            )
            .unwrap(),
            vec![
                &mut token_account2_account,
                &mut owner_account,
                &mut token_account,
            ],
        )
        .unwrap();

        // create another token account
        do_process_instruction(
            new_account(
                &program_id,
                &token_account3_key,
                &owner_key,
                &token_key,
                None,
            )
            .unwrap(),
            vec![
                &mut token_account3_account,
                &mut owner_account,
                &mut token_account,
            ],
        )
        .unwrap();

        // create mismatch token account
        do_process_instruction(
            new_account(
                &program_id,
                &mismatch_account_key,
                &owner_key,
                &token2_key,
                None,
            )
            .unwrap(),
            vec![
                &mut mismatch_account_account,
                &mut owner_account,
                &mut token2_account,
            ],
        )
        .unwrap();

        // create delegate account
        do_process_instruction(
            new_account(
                &program_id,
                &delegate_account_key,
                &owner_key,
                &token_key,
                Some(&token_account_key),
            )
            .unwrap(),
            vec![
                &mut delegate_account_account,
                &mut owner_account,
                &mut token_account,
                &mut token_account_account,
            ],
        )
        .unwrap();

        // create new token with owner
        do_process_instruction(
            new_token(
                &program_id,
                &token_key,
                Some(&token_account_key),
                Some(&owner_key),
                TokenInfo {
                    supply: 1000,
                    decimals: 2,
                },
            )
            .unwrap(),
            vec![
                &mut token_account,
                &mut token_account_account,
                &mut owner_account,
            ],
        )
        .unwrap();

        // mint to
        do_process_instruction(
            mint_to(&program_id, &owner_key, &token_key, &token_account2_key, 42).unwrap(),
            vec![
                &mut owner_account,
                &mut token_account,
                &mut token_account2_account,
            ],
        )
        .unwrap();

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
        let mut instruction =
            mint_to(&program_id, &owner_key, &token_key, &token_account2_key, 42).unwrap();
        instruction.accounts[0].is_signer = false;
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            do_process_instruction(
                instruction,
                vec![
                    &mut owner_account,
                    &mut token_account,
                    &mut token_account2_account,
                ],
            )
        );

        // destination is delegate
        assert_eq!(
            Err(TokenError::DestinationIsDelegate.into()),
            do_process_instruction(
                mint_to(
                    &program_id,
                    &owner_key,
                    &token_key,
                    &delegate_account_key,
                    42
                )
                .unwrap(),
                vec![
                    &mut owner_account,
                    &mut token_account,
                    &mut delegate_account_account,
                ],
            )
        );

        // mismatch token
        assert_eq!(
            Err(TokenError::TokenMismatch.into()),
            do_process_instruction(
                mint_to(
                    &program_id,
                    &owner_key,
                    &token_key,
                    &mismatch_account_key,
                    42
                )
                .unwrap(),
                vec![
                    &mut owner_account,
                    &mut token_account,
                    &mut mismatch_account_account,
                ],
            )
        );

        // missing owner
        assert_eq!(
            Err(TokenError::NoOwner.into()),
            do_process_instruction(
                mint_to(
                    &program_id,
                    &owner2_key,
                    &token_key,
                    &token_account2_key,
                    42
                )
                .unwrap(),
                vec![
                    &mut owner2_account,
                    &mut token_account,
                    &mut token_account2_account,
                ],
            )
        );

        // uninitialized destination account
        assert_eq!(
            Err(ProgramError::InvalidArgument),
            do_process_instruction(
                mint_to(&program_id, &owner_key, &token_key, &uninitialized_key, 42).unwrap(),
                vec![
                    &mut owner_account,
                    &mut token_account,
                    &mut uninitialized_account,
                ],
            )
        );
    }

    #[test]
    fn test_burn() {
        let program_id = pubkey_rand();
        let token_account_key = pubkey_rand();
        let mut token_account_account = Account::new(0, size_of::<State>(), &program_id);
        let token_account2_key = pubkey_rand();
        let mut token_account2_account = Account::new(0, size_of::<State>(), &program_id);
        let token_account3_key = pubkey_rand();
        let mut token_account3_account = Account::new(0, size_of::<State>(), &program_id);
        let delegate_account_key = pubkey_rand();
        let mut delegate_account_account = Account::new(0, size_of::<State>(), &program_id);
        let mismatch_account_key = pubkey_rand();
        let mut mismatch_account_account = Account::new(0, size_of::<State>(), &program_id);
        let mismatch_delegate_account_key = pubkey_rand();
        let mut mismatch_delegate_account_account =
            Account::new(0, size_of::<State>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = Account::default();
        let owner2_key = pubkey_rand();
        let mut owner2_account = Account::default();
        let token_key = pubkey_rand();
        let mut token_account = Account::new(0, size_of::<State>(), &program_id);
        let token2_key = pubkey_rand();
        let mut token2_account = Account::new(0, size_of::<State>(), &program_id);

        // create token account
        do_process_instruction(
            new_account(
                &program_id,
                &token_account_key,
                &owner_key,
                &token_key,
                None,
            )
            .unwrap(),
            vec![
                &mut token_account_account,
                &mut owner_account,
                &mut token_account,
            ],
        )
        .unwrap();

        // create another token account
        do_process_instruction(
            new_account(
                &program_id,
                &token_account2_key,
                &owner_key,
                &token_key,
                None,
            )
            .unwrap(),
            vec![
                &mut token_account2_account,
                &mut owner_account,
                &mut token_account,
            ],
        )
        .unwrap();

        // create another token account
        do_process_instruction(
            new_account(
                &program_id,
                &token_account3_key,
                &owner_key,
                &token_key,
                None,
            )
            .unwrap(),
            vec![
                &mut token_account3_account,
                &mut owner_account,
                &mut token_account,
            ],
        )
        .unwrap();

        // create mismatch token account
        do_process_instruction(
            new_account(
                &program_id,
                &mismatch_account_key,
                &owner_key,
                &token2_key,
                None,
            )
            .unwrap(),
            vec![
                &mut mismatch_account_account,
                &mut owner_account,
                &mut token2_account,
            ],
        )
        .unwrap();

        // create delegate account
        do_process_instruction(
            new_account(
                &program_id,
                &delegate_account_key,
                &owner_key,
                &token_key,
                Some(&token_account_key),
            )
            .unwrap(),
            vec![
                &mut delegate_account_account,
                &mut owner_account,
                &mut token_account,
                &mut token_account_account,
            ],
        )
        .unwrap();
        // create mismatch delegate account
        do_process_instruction(
            new_account(
                &program_id,
                &mismatch_delegate_account_key,
                &owner_key,
                &token2_key,
                Some(&token_account_key),
            )
            .unwrap(),
            vec![
                &mut mismatch_delegate_account_account,
                &mut owner_account,
                &mut token2_account,
                &mut token_account_account,
            ],
        )
        .unwrap();

        // create new token
        do_process_instruction(
            new_token(
                &program_id,
                &token_key,
                Some(&token_account_key),
                None,
                TokenInfo {
                    supply: 1000,
                    decimals: 2,
                },
            )
            .unwrap(),
            vec![&mut token_account, &mut token_account_account],
        )
        .unwrap();

        // missing signer
        let mut instruction = burn(
            &program_id,
            &owner_key,
            &token_account_key,
            &token_key,
            None,
            42,
        )
        .unwrap();
        instruction.accounts[0].is_signer = false;
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            do_process_instruction(
                instruction,
                vec![
                    &mut owner_account,
                    &mut token_account_account,
                    &mut token_account
                ],
            )
        );

        // mismatch token
        assert_eq!(
            Err(TokenError::TokenMismatch.into()),
            do_process_instruction(
                burn(
                    &program_id,
                    &owner_key,
                    &mismatch_account_key,
                    &token_key,
                    None,
                    42
                )
                .unwrap(),
                vec![
                    &mut owner_account,
                    &mut mismatch_account_account,
                    &mut token_account
                ],
            )
        );

        // missing owner
        assert_eq!(
            Err(TokenError::NoOwner.into()),
            do_process_instruction(
                burn(
                    &program_id,
                    &owner2_key,
                    &token_account_key,
                    &token_key,
                    None,
                    42
                )
                .unwrap(),
                vec![
                    &mut owner2_account,
                    &mut token_account_account,
                    &mut token_account
                ],
            )
        );

        // burn
        do_process_instruction(
            burn(
                &program_id,
                &owner_key,
                &token_account_key,
                &token_key,
                None,
                42,
            )
            .unwrap(),
            vec![
                &mut owner_account,
                &mut token_account_account,
                &mut token_account,
            ],
        )
        .unwrap();

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
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            do_process_instruction(
                burn(
                    &program_id,
                    &owner_key,
                    &token_account_key,
                    &token_key,
                    None,
                    100_000_000
                )
                .unwrap(),
                vec![
                    &mut owner_account,
                    &mut token_account_account,
                    &mut token_account
                ],
            )
        );

        // approve delegate
        do_process_instruction(
            approve(
                &program_id,
                &owner_key,
                &token_account_key,
                &delegate_account_key,
                84,
            )
            .unwrap(),
            vec![
                &mut owner_account,
                &mut token_account_account,
                &mut delegate_account_account,
            ],
        )
        .unwrap();

        // not a delegate of source account
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            do_process_instruction(
                burn(
                    &program_id,
                    &owner_key,
                    &token_account_key,
                    &token_key,
                    None,
                    100_000_000
                )
                .unwrap(),
                vec![
                    &mut owner_account,
                    &mut token_account_account,
                    &mut token_account
                ],
            )
        );

        // burn via delegate
        do_process_instruction(
            burn(
                &program_id,
                &owner_key,
                &delegate_account_key,
                &token_key,
                Some(&token_account_key),
                84,
            )
            .unwrap(),
            vec![
                &mut owner_account,
                &mut delegate_account_account,
                &mut token_account,
                &mut token_account_account,
            ],
        )
        .unwrap();

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
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            do_process_instruction(
                burn(
                    &program_id,
                    &owner_key,
                    &delegate_account_key,
                    &token_key,
                    Some(&token_account_key),
                    100,
                )
                .unwrap(),
                vec![
                    &mut owner_account,
                    &mut delegate_account_account,
                    &mut token_account,
                    &mut token_account_account,
                ],
            )
        );
    }
}
