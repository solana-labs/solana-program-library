//! State transition types

use crate::{
    error::TokenError,
    instruction::{is_valid_signer_index, TokenInfo, TokenInstruction, MAX_SIGNERS},
    option::COption,
};
use solana_sdk::{
    account_info::AccountInfo, entrypoint::ProgramResult, info, program_error::ProgramError,
    program_utils::next_account_info, pubkey::Pubkey,
};
use std::mem::size_of;

/// Represents a token type identified by its public key.  Accounts
/// are associated with a specific token type and only accounts with
/// matching types my inter-opt.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Token {
    /// Token details.
    pub info: TokenInfo,
    /// Optional owner, used to mint new tokens.  The owner may only
    /// be provided during mint creation.  If no owner is present then the mint
    /// has a fixed supply and no further tokens may be minted.
    pub owner: COption<Pubkey>,
}

/// Account that holds tokens or may delegate tokens.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Account {
    /// The mint associated with this account
    pub mint: Pubkey,
    /// The owner of this account.
    pub owner: Pubkey,
    /// The amount of tokens this account holds.
    pub amount: u64,
    /// If `delegate` is `Some` then `delegated_amount` represents
    /// the amount authorized by the delegate
    pub delegate: COption<Pubkey>,
    /// The amount delegated
    pub delegated_amount: u64,
}

/// Multisignature account data.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Multisig {
    /// Number of signers required
    pub m: u8,
    /// Number of valid signers
    pub n: u8,
    /// Signer public keys
    pub signers: [Pubkey; MAX_SIGNERS],
}
impl Multisig {
    /// Deserializes a byte buffer into a [Multisig](struct.State.html).
    pub fn deserialize(input: &mut [u8]) -> Result<&mut Self, ProgramError> {
        if input.len() < size_of::<Multisig>() {
            return Err(ProgramError::InvalidAccountData);
        }
        #[allow(clippy::cast_ptr_alignment)]
        Ok(unsafe { &mut *(&mut input[0] as *mut u8 as *mut Multisig) })
    }

    /// Serializes [Multisig](struct.State.html) into a byte buffer.
    pub fn serialize(self: &Self, output: &mut [u8]) -> ProgramResult {
        if output.len() < size_of::<Multisig>() {
            return Err(ProgramError::InvalidAccountData);
        }
        #[allow(clippy::cast_ptr_alignment)]
        let value = unsafe { &mut *(&mut output[0] as *mut u8 as *mut Multisig) };
        *value = *self;
        Ok(())
    }
}

/// Program states.
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// Unallocated state, may be initialized into another state.
    Unallocated,
    /// A mint.
    Mint(Token),
    /// An account that holds tokens
    Account(Account),
}
impl Default for State {
    fn default() -> Self {
        Self::Unallocated
    }
}
impl State {
    /// Processes an [InitializeMint](enum.TokenInstruction.html) instruction.
    pub fn process_initialize_mint(accounts: &[AccountInfo], info: TokenInfo) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let mint_info = next_account_info(account_info_iter)?;

        if State::Unallocated != State::deserialize(&mint_info.data.borrow())? {
            return Err(TokenError::AlreadyInUse.into());
        }

        let owner = if info.supply != 0 {
            let dest_account_info = next_account_info(account_info_iter)?;
            let mut dest_account_data = dest_account_info.data.borrow_mut();
            if let State::Account(mut dest_account) = State::deserialize(&dest_account_data)? {
                if mint_info.key != &dest_account.mint {
                    return Err(TokenError::MintMismatch.into());
                }

                dest_account.amount = info.supply;
                State::Account(dest_account).serialize(&mut dest_account_data)?;
            } else {
                return Err(ProgramError::InvalidArgument);
            }

            if let Ok(owner_info) = next_account_info(account_info_iter) {
                COption::Some(*owner_info.key)
            } else {
                COption::None
            }
        } else if let Ok(owner_info) = next_account_info(account_info_iter) {
            COption::Some(*owner_info.key)
        } else {
            return Err(TokenError::OwnerRequiredIfNoInitialSupply.into());
        };

        State::Mint(Token { info, owner }).serialize(&mut mint_info.data.borrow_mut())
    }

    /// Processes an [InitializeAccount](enum.TokenInstruction.html) instruction.
    pub fn process_initialize_account(accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let new_account_info = next_account_info(account_info_iter)?;
        let mint_info = next_account_info(account_info_iter)?;
        let owner_info = next_account_info(account_info_iter)?;

        let mut new_account_data = new_account_info.data.borrow_mut();
        if State::Unallocated != State::deserialize(&new_account_data)? {
            return Err(TokenError::AlreadyInUse.into());
        }

        let account = Account {
            mint: *mint_info.key,
            owner: *owner_info.key,
            amount: 0,
            delegate: COption::None,
            delegated_amount: 0,
        };

        State::Account(account).serialize(&mut new_account_data)
    }

    /// Processes a [InitializeMultisig](enum.TokenInstruction.html) instruction.
    pub fn process_initialize_multisig(accounts: &[AccountInfo], m: u8) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let multisig_info = next_account_info(account_info_iter)?;
        let mut multisig_account_data = multisig_info.data.borrow_mut();
        let mut multisig = Multisig::deserialize(&mut multisig_account_data)?;
        let signer_infos = account_info_iter.as_slice();

        multisig.m = m;
        multisig.n = signer_infos.len() as u8;
        if !is_valid_signer_index(multisig.n as usize) {
            return Err(TokenError::InvalidNumberOfProvidedSigners.into());
        }
        if !is_valid_signer_index(multisig.m as usize) {
            return Err(TokenError::InvalidNumberOfRequiredSigners.into());
        }
        for (i, signer_info) in signer_infos.iter().enumerate() {
            multisig.signers[i] = *signer_info.key;
        }
        Ok(())
    }

    /// Processes a [Transfer](enum.TokenInstruction.html) instruction.
    pub fn process_transfer(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let dest_account_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;

        let mut source_data = source_account_info.data.borrow_mut();
        let mut dest_data = dest_account_info.data.borrow_mut();
        if let (State::Account(mut source_account), State::Account(mut dest_account)) = (
            State::deserialize(&source_data)?,
            State::deserialize(&dest_data)?,
        ) {
            if source_account.amount < amount {
                return Err(TokenError::InsufficientFunds.into());
            }
            if source_account.mint != dest_account.mint {
                return Err(TokenError::MintMismatch.into());
            }

            match source_account.delegate {
                COption::Some(ref delegate) if authority_info.key == delegate => {
                    Self::validate_owner(
                        program_id,
                        delegate,
                        authority_info,
                        account_info_iter.as_slice(),
                    )?;
                    if source_account.delegated_amount < amount {
                        return Err(TokenError::InsufficientFunds.into());
                    }
                    source_account.delegated_amount -= amount;
                    if source_account.delegated_amount == 0 {
                        source_account.delegate = COption::None;
                    }
                }
                _ => Self::validate_owner(
                    program_id,
                    &source_account.owner,
                    authority_info,
                    account_info_iter.as_slice(),
                )?,
            };

            source_account.amount -= amount;
            dest_account.amount += amount;

            State::Account(source_account).serialize(&mut source_data)?;
            State::Account(dest_account).serialize(&mut dest_data)
        } else {
            Err(ProgramError::InvalidArgument)
        }
    }

    /// Processes an [Approve](enum.TokenInstruction.html) instruction.
    pub fn process_approve(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;

        let mut source_data = source_account_info.data.borrow_mut();
        if let State::Account(mut source_account) = State::deserialize(&source_data)? {
            source_account.delegate = if amount > 0 {
                let delegate_info = next_account_info(account_info_iter)?;
                COption::Some(*delegate_info.key)
            } else {
                COption::None
            };
            source_account.delegated_amount = amount;

            let owner_info = next_account_info(account_info_iter)?;
            Self::validate_owner(
                program_id,
                &source_account.owner,
                owner_info,
                account_info_iter.as_slice(),
            )?;

            State::Account(source_account).serialize(&mut source_data)
        } else {
            Err(ProgramError::InvalidArgument)
        }
    }

    /// Processes a [SetOwner](enum.TokenInstruction.html) instruction.
    pub fn process_set_owner(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let account_info = next_account_info(account_info_iter)?;
        let new_owner_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;

        let mut account_data = account_info.data.borrow_mut();
        match State::deserialize(&account_data)? {
            State::Account(mut account) => {
                Self::validate_owner(
                    program_id,
                    &account.owner,
                    authority_info,
                    account_info_iter.as_slice(),
                )?;

                account.owner = *new_owner_info.key;
                State::Account(account).serialize(&mut account_data)
            }
            State::Mint(mut token) => {
                match token.owner {
                    COption::Some(ref owner) => {
                        Self::validate_owner(
                            program_id,
                            owner,
                            authority_info,
                            account_info_iter.as_slice(),
                        )?;
                    }
                    COption::None => return Err(TokenError::FixedSupply.into()),
                }
                token.owner = COption::Some(*new_owner_info.key);
                State::Mint(token).serialize(&mut account_data)
            }
            _ => Err(ProgramError::InvalidArgument),
        }
    }

    /// Processes a [MintTo](enum.TokenInstruction.html) instruction.
    pub fn process_mint_to(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let mint_info = next_account_info(account_info_iter)?;
        let dest_account_info = next_account_info(account_info_iter)?;
        let owner_info = next_account_info(account_info_iter)?;

        let mut mint_data = mint_info.data.borrow_mut();
        if let State::Mint(mut token) = State::deserialize(&mint_data)? {
            match token.owner {
                COption::Some(owner) => {
                    Self::validate_owner(
                        program_id,
                        &owner,
                        owner_info,
                        account_info_iter.as_slice(),
                    )?;
                }
                COption::None => {
                    return Err(TokenError::FixedSupply.into());
                }
            }

            let mut dest_account_data = dest_account_info.data.borrow_mut();
            if let State::Account(mut dest_account) = State::deserialize(&dest_account_data)? {
                if mint_info.key != &dest_account.mint {
                    return Err(TokenError::MintMismatch.into());
                }

                token.info.supply += amount;
                State::Mint(token).serialize(&mut mint_data)?;

                dest_account.amount += amount;
                State::Account(dest_account).serialize(&mut dest_account_data)
            } else {
                Err(ProgramError::InvalidArgument)
            }
        } else {
            Err(ProgramError::InvalidArgument)
        }
    }

    /// Processes a [Burn](enum.TokenInstruction.html) instruction.
    pub fn process_burn(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let mint_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;

        let (mut source_account, mut source_data) = {
            let source_data = source_account_info.data.borrow_mut();
            match State::deserialize(&source_data)? {
                State::Account(source_account) => (source_account, source_data),
                _ => {
                    return Err(ProgramError::InvalidArgument);
                }
            }
        };

        let (mut token, mut mint_data) = {
            let mint_data = mint_info.data.borrow_mut();
            match State::deserialize(&mint_data)? {
                State::Mint(token) => (token, mint_data),
                _ => {
                    return Err(ProgramError::InvalidArgument);
                }
            }
        };

        if mint_info.key != &source_account.mint {
            return Err(TokenError::MintMismatch.into());
        }
        if source_account.amount < amount {
            return Err(TokenError::InsufficientFunds.into());
        }

        match source_account.delegate {
            COption::Some(ref delegate) if authority_info.key == delegate => {
                Self::validate_owner(
                    program_id,
                    delegate,
                    authority_info,
                    account_info_iter.as_slice(),
                )?;

                if source_account.delegated_amount < amount {
                    return Err(TokenError::InsufficientFunds.into());
                }
                source_account.delegated_amount -= amount;
                if source_account.delegated_amount == 0 {
                    source_account.delegate = COption::None;
                }
            }
            _ => Self::validate_owner(
                program_id,
                &source_account.owner,
                authority_info,
                account_info_iter.as_slice(),
            )?,
        }

        source_account.amount -= amount;
        token.info.supply -= amount;

        State::Account(source_account).serialize(&mut source_data)?;
        State::Mint(token).serialize(&mut mint_data)
    }

    /// Processes an [Instruction](enum.Instruction.html).
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        let instruction = TokenInstruction::deserialize(input)?;

        match instruction {
            TokenInstruction::InitializeMint(info) => {
                info!("Instruction: InitializeMint");
                Self::process_initialize_mint(accounts, info)
            }
            TokenInstruction::InitializeAccount => {
                info!("Instruction: InitializeAccount");
                Self::process_initialize_account(accounts)
            }
            TokenInstruction::InitializeMultisig(m) => {
                info!("Instruction: InitializeM<ultisig");
                Self::process_initialize_multisig(accounts, m)
            }
            TokenInstruction::Transfer(amount) => {
                info!("Instruction: Transfer");
                Self::process_transfer(program_id, accounts, amount)
            }
            TokenInstruction::Approve(amount) => {
                info!("Instruction: Approve");
                Self::process_approve(program_id, accounts, amount)
            }
            TokenInstruction::SetOwner => {
                info!("Instruction: SetOwner");
                Self::process_set_owner(program_id, accounts)
            }
            TokenInstruction::MintTo(amount) => {
                info!("Instruction: MintTo");
                Self::process_mint_to(program_id, accounts, amount)
            }
            TokenInstruction::Burn(amount) => {
                info!("Instruction: Burn");
                Self::process_burn(program_id, accounts, amount)
            }
        }
    }

    /// Validates owner(s) are present
    pub fn validate_owner(
        program_id: &Pubkey,
        expected_owner: &Pubkey,
        owner_account_info: &AccountInfo,
        signers: &[AccountInfo],
    ) -> ProgramResult {
        if expected_owner != owner_account_info.key {
            return Err(TokenError::OwnerMismatch.into());
        }
        if program_id == owner_account_info.owner
            && owner_account_info.data_len() == std::mem::size_of::<Multisig>()
        {
            let mut owner_data = owner_account_info.data.borrow_mut();
            let multisig = Multisig::deserialize(&mut owner_data).unwrap();
            let mut num_signers = 0;
            for signer in signers.iter() {
                if multisig.signers[0..multisig.n as usize].contains(signer.key) {
                    if !signer.is_signer {
                        return Err(ProgramError::MissingRequiredSignature);
                    }
                    num_signers += 1;
                }
            }
            if num_signers < multisig.m {
                return Err(ProgramError::MissingRequiredSignature);
            }
        } else if !owner_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        Ok(())
    }

    /// Deserializes a byte buffer into a Token Program [State](struct.State.html).
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
                Self::Mint(*token)
            }
            2 => {
                if input.len() < size_of::<u8>() + size_of::<Account>() {
                    return Err(ProgramError::InvalidAccountData);
                }
                #[allow(clippy::cast_ptr_alignment)]
                let account: &Account = unsafe { &*(&input[1] as *const u8 as *const Account) };
                Self::Account(*account)
            }
            _ => return Err(ProgramError::InvalidAccountData),
        })
    }

    /// Serializes Token Program [State](struct.State.html) into a byte buffer.
    pub fn serialize(self: &Self, output: &mut [u8]) -> ProgramResult {
        if output.len() < size_of::<u8>() {
            return Err(ProgramError::InvalidAccountData);
        }
        match self {
            Self::Unallocated => output[0] = 0,
            Self::Mint(token) => {
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
        }
        Ok(())
    }
}

// Pulls in the stubs required for `info!()`.
#[cfg(not(target_arch = "bpf"))]
solana_sdk_bpf_test::stubs!();

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::{
        approve, burn, initialize_account, initialize_mint, initialize_multisig, mint_to,
        set_owner, transfer,
    };
    use solana_sdk::{
        account::Account, account_info::create_is_signer_account_infos, clock::Epoch,
        instruction::Instruction,
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
    fn test_unique_account_sizes() {
        assert_ne!(size_of::<State>(), 0);
        assert_ne!(size_of::<Multisig>(), 0);
        assert_ne!(size_of::<State>(), size_of::<Multisig>());
    }

    #[test]
    fn test_initialize_mint() {
        let program_id = pubkey_rand();
        let account_key = pubkey_rand();
        let mut account_account = Account::new(0, size_of::<State>(), &program_id);
        let account2_key = pubkey_rand();
        let mut account2_account = Account::new(0, size_of::<State>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = Account::default();
        let mint_key = pubkey_rand();
        let mut mint_account = Account::new(0, size_of::<State>(), &program_id);
        let mint2_key = pubkey_rand();
        let mut mint2_account = Account::new(0, size_of::<State>(), &program_id);

        // account not created
        assert_eq!(
            Err(ProgramError::InvalidArgument),
            do_process_instruction(
                initialize_mint(
                    &program_id,
                    &mint_key,
                    Some(&account_key),
                    None,
                    TokenInfo {
                        supply: 1000,
                        decimals: 2,
                    }
                )
                .unwrap(),
                vec![&mut mint_account, &mut account_account]
            )
        );

        // create account
        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
            vec![&mut account_account, &mut owner_account, &mut mint_account],
        )
        .unwrap();

        // create new token
        do_process_instruction(
            initialize_mint(
                &program_id,
                &mint_key,
                Some(&account_key),
                None,
                TokenInfo {
                    supply: 1000,
                    decimals: 2,
                },
            )
            .unwrap(),
            vec![&mut mint_account, &mut account_account],
        )
        .unwrap();

        // create another account
        do_process_instruction(
            initialize_account(&program_id, &account2_key, &mint_key, &owner_key).unwrap(),
            vec![&mut account2_account, &mut owner_account, &mut mint_account],
        )
        .unwrap();

        // token mismatch
        assert_eq!(
            Err(TokenError::MintMismatch.into()),
            do_process_instruction(
                initialize_mint(
                    &program_id,
                    &mint2_key,
                    Some(&account2_key),
                    None,
                    TokenInfo {
                        supply: 1000,
                        decimals: 2,
                    },
                )
                .unwrap(),
                vec![&mut mint2_account, &mut account2_account]
            )
        );

        // create twice
        assert_eq!(
            Err(TokenError::AlreadyInUse.into()),
            do_process_instruction(
                initialize_mint(
                    &program_id,
                    &mint_key,
                    Some(&account_key),
                    None,
                    TokenInfo {
                        supply: 1000,
                        decimals: 2,
                    },
                )
                .unwrap(),
                vec![&mut mint_account, &mut account_account]
            )
        );
    }

    #[test]
    fn test_initialize_mint_account() {
        let program_id = pubkey_rand();
        let account_key = pubkey_rand();
        let mut account_account = Account::new(0, size_of::<State>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = Account::default();
        let mint_key = pubkey_rand();
        let mut mint_account = Account::new(0, size_of::<State>(), &program_id);

        // create account
        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
            vec![&mut account_account, &mut mint_account, &mut owner_account],
        )
        .unwrap();

        // create twice
        assert_eq!(
            Err(TokenError::AlreadyInUse.into()),
            do_process_instruction(
                initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
                vec![&mut account_account, &mut mint_account, &mut owner_account],
            )
        );
    }

    #[test]
    fn test_transfer() {
        let program_id = pubkey_rand();
        let account_key = pubkey_rand();
        let mut account_account = Account::new(0, size_of::<State>(), &program_id);
        let account2_key = pubkey_rand();
        let mut account2_account = Account::new(0, size_of::<State>(), &program_id);
        let account3_key = pubkey_rand();
        let mut account3_account = Account::new(0, size_of::<State>(), &program_id);
        let delegate_key = pubkey_rand();
        let mut delegate_account = Account::default();
        let mismatch_key = pubkey_rand();
        let mut mismatch_account = Account::new(0, size_of::<State>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = Account::default();
        let owner2_key = pubkey_rand();
        let mut owner2_account = Account::default();
        let mint_key = pubkey_rand();
        let mut mint_account = Account::new(0, size_of::<State>(), &program_id);
        let mint2_key = pubkey_rand();
        let mut mint2_account = Account::new(0, size_of::<State>(), &program_id);

        // create account
        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
            vec![&mut account_account, &mut mint_account, &mut owner_account],
        )
        .unwrap();

        // create another account
        do_process_instruction(
            initialize_account(&program_id, &account2_key, &mint_key, &owner_key).unwrap(),
            vec![&mut account2_account, &mut mint_account, &mut owner_account],
        )
        .unwrap();

        // create another account
        do_process_instruction(
            initialize_account(&program_id, &account3_key, &mint_key, &owner_key).unwrap(),
            vec![&mut account3_account, &mut mint_account, &mut owner_account],
        )
        .unwrap();

        // create mismatch account
        do_process_instruction(
            initialize_account(&program_id, &mismatch_key, &mint2_key, &owner_key).unwrap(),
            vec![
                &mut mismatch_account,
                &mut mint2_account,
                &mut owner_account,
            ],
        )
        .unwrap();

        // create new token
        do_process_instruction(
            initialize_mint(
                &program_id,
                &mint_key,
                Some(&account_key),
                None,
                TokenInfo {
                    supply: 1000,
                    decimals: 2,
                },
            )
            .unwrap(),
            vec![&mut mint_account, &mut account_account],
        )
        .unwrap();

        // missing signer
        let mut instruction = transfer(
            &program_id,
            &account_key,
            &account2_key,
            &owner_key,
            &[],
            1000,
        )
        .unwrap();
        instruction.accounts[2].is_signer = false;
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            do_process_instruction(
                instruction,
                vec![
                    &mut account_account,
                    &mut account2_account,
                    &mut owner_account,
                ],
            )
        );

        // mismatch token
        assert_eq!(
            Err(TokenError::MintMismatch.into()),
            do_process_instruction(
                transfer(
                    &program_id,
                    &account_key,
                    &mismatch_key,
                    &owner_key,
                    &[],
                    1000
                )
                .unwrap(),
                vec![
                    &mut account_account,
                    &mut mismatch_account,
                    &mut owner_account,
                ],
            )
        );

        // missing owner
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            do_process_instruction(
                transfer(
                    &program_id,
                    &account_key,
                    &account2_key,
                    &owner2_key,
                    &[],
                    1000
                )
                .unwrap(),
                vec![
                    &mut account_account,
                    &mut account2_account,
                    &mut owner2_account,
                ],
            )
        );

        // transfer
        do_process_instruction(
            transfer(
                &program_id,
                &account_key,
                &account2_key,
                &owner_key,
                &[],
                1000,
            )
            .unwrap(),
            vec![
                &mut account_account,
                &mut account2_account,
                &mut owner_account,
            ],
        )
        .unwrap();

        // insufficient funds
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            do_process_instruction(
                transfer(&program_id, &account_key, &account2_key, &owner_key, &[], 1).unwrap(),
                vec![
                    &mut account_account,
                    &mut account2_account,
                    &mut owner_account,
                ],
            )
        );

        // transfer half back
        do_process_instruction(
            transfer(
                &program_id,
                &account2_key,
                &account_key,
                &owner_key,
                &[],
                500,
            )
            .unwrap(),
            vec![
                &mut account2_account,
                &mut account_account,
                &mut owner_account,
            ],
        )
        .unwrap();

        // transfer rest
        do_process_instruction(
            transfer(
                &program_id,
                &account2_key,
                &account_key,
                &owner_key,
                &[],
                500,
            )
            .unwrap(),
            vec![
                &mut account2_account,
                &mut account_account,
                &mut owner_account,
            ],
        )
        .unwrap();

        // insufficient funds
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            do_process_instruction(
                transfer(&program_id, &account2_key, &account_key, &owner_key, &[], 1).unwrap(),
                vec![
                    &mut account2_account,
                    &mut account_account,
                    &mut owner_account,
                ],
            )
        );

        // approve delegate
        do_process_instruction(
            approve(
                &program_id,
                &account_key,
                &delegate_key,
                &owner_key,
                &[],
                100,
            )
            .unwrap(),
            vec![
                &mut account_account,
                &mut delegate_account,
                &mut owner_account,
            ],
        )
        .unwrap();

        // transfer via delegate
        do_process_instruction(
            transfer(
                &program_id,
                &account_key,
                &account2_key,
                &delegate_key,
                &[],
                100,
            )
            .unwrap(),
            vec![
                &mut account_account,
                &mut account2_account,
                &mut delegate_account,
            ],
        )
        .unwrap();

        // insufficient funds approved via delegate
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            do_process_instruction(
                transfer(
                    &program_id,
                    &account_key,
                    &account2_key,
                    &delegate_key,
                    &[],
                    100
                )
                .unwrap(),
                vec![
                    &mut account_account,
                    &mut account2_account,
                    &mut delegate_account,
                ],
            )
        );

        // transfer rest
        do_process_instruction(
            transfer(
                &program_id,
                &account_key,
                &account2_key,
                &owner_key,
                &[],
                900,
            )
            .unwrap(),
            vec![
                &mut account_account,
                &mut account2_account,
                &mut owner_account,
            ],
        )
        .unwrap();

        // approve delegate
        do_process_instruction(
            approve(
                &program_id,
                &account_key,
                &delegate_key,
                &owner_key,
                &[],
                100,
            )
            .unwrap(),
            vec![
                &mut account_account,
                &mut delegate_account,
                &mut owner_account,
            ],
        )
        .unwrap();

        // insufficient funds in source account via delegate
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            do_process_instruction(
                transfer(
                    &program_id,
                    &account_key,
                    &account2_key,
                    &delegate_key,
                    &[],
                    100
                )
                .unwrap(),
                vec![
                    &mut account_account,
                    &mut account2_account,
                    &mut delegate_account,
                ],
            )
        );
    }

    #[test]
    fn test_mintable_token_with_zero_supply() {
        let program_id = pubkey_rand();
        let account_key = pubkey_rand();
        let mut account_account = Account::new(0, size_of::<State>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = Account::default();
        let mint_key = pubkey_rand();
        let mut mint_account = Account::new(0, size_of::<State>(), &program_id);

        // create account
        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
            vec![&mut account_account, &mut owner_account, &mut mint_account],
        )
        .unwrap();

        // create mint-able token without owner
        let mut instruction = initialize_mint(
            &program_id,
            &mint_key,
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
            do_process_instruction(instruction, vec![&mut mint_account])
        );

        // create mint-able token with zero supply
        let info = TokenInfo {
            supply: 0,
            decimals: 2,
        };
        do_process_instruction(
            initialize_mint(&program_id, &mint_key, None, Some(&owner_key), info).unwrap(),
            vec![&mut mint_account, &mut account_account],
        )
        .unwrap();
        if let State::Mint(token) = State::deserialize(&mint_account.data).unwrap() {
            assert_eq!(
                token,
                Token {
                    info,
                    owner: COption::Some(owner_key)
                }
            );
        } else {
            panic!("not an account");
        }

        // mint to
        do_process_instruction(
            mint_to(&program_id, &mint_key, &account_key, &owner_key, &[], 42).unwrap(),
            vec![&mut mint_account, &mut account_account, &mut owner_account],
        )
        .unwrap();

        if let State::Mint(token) = State::deserialize(&mint_account.data).unwrap() {
            assert_eq!(token.info.supply, 42);
        } else {
            panic!("not an account");
        }
        if let State::Account(dest_account) = State::deserialize(&account_account.data).unwrap() {
            assert_eq!(dest_account.amount, 42);
        } else {
            panic!("not an account");
        }
    }

    #[test]
    fn test_approve() {
        let program_id = pubkey_rand();
        let account_key = pubkey_rand();
        let mut account_account = Account::new(0, size_of::<State>(), &program_id);
        let account2_key = pubkey_rand();
        let mut account2_account = Account::new(0, size_of::<State>(), &program_id);
        let delegate_key = pubkey_rand();
        let mut delegate_account = Account::default();
        let owner_key = pubkey_rand();
        let mut owner_account = Account::default();
        let owner2_key = pubkey_rand();
        let mut owner2_account = Account::default();
        let mint_key = pubkey_rand();
        let mut mint_account = Account::new(0, size_of::<State>(), &program_id);

        // create account
        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
            vec![&mut account_account, &mut owner_account, &mut mint_account],
        )
        .unwrap();

        // create another account
        do_process_instruction(
            initialize_account(&program_id, &account2_key, &mint_key, &owner_key).unwrap(),
            vec![&mut account2_account, &mut owner_account, &mut mint_account],
        )
        .unwrap();

        // create new token
        do_process_instruction(
            initialize_mint(
                &program_id,
                &mint_key,
                Some(&account_key),
                None,
                TokenInfo {
                    supply: 1000,
                    decimals: 2,
                },
            )
            .unwrap(),
            vec![&mut mint_account, &mut account_account],
        )
        .unwrap();

        // missing signer
        let mut instruction = approve(
            &program_id,
            &account_key,
            &delegate_key,
            &owner_key,
            &[],
            100,
        )
        .unwrap();
        instruction.accounts[2].is_signer = false;
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            do_process_instruction(
                instruction,
                vec![
                    &mut account_account,
                    &mut delegate_account,
                    &mut owner_account,
                ],
            )
        );

        // no owner
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            do_process_instruction(
                approve(
                    &program_id,
                    &account_key,
                    &delegate_key,
                    &owner2_key,
                    &[],
                    100
                )
                .unwrap(),
                vec![
                    &mut account_account,
                    &mut delegate_account,
                    &mut owner2_account,
                ],
            )
        );

        // approve delegate
        do_process_instruction(
            approve(
                &program_id,
                &account_key,
                &delegate_key,
                &owner_key,
                &[],
                100,
            )
            .unwrap(),
            vec![
                &mut account_account,
                &mut delegate_account,
                &mut owner_account,
            ],
        )
        .unwrap();
    }

    #[test]
    fn test_set_owner() {
        let program_id = pubkey_rand();
        let account_key = pubkey_rand();
        let mut account_account = Account::new(0, size_of::<State>(), &program_id);
        let account2_key = pubkey_rand();
        let mut account2_account = Account::new(0, size_of::<State>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = Account::default();
        let owner2_key = pubkey_rand();
        let mut owner2_account = Account::default();
        let owner3_key = pubkey_rand();
        let mut owner3_account = Account::default();
        let mint_key = pubkey_rand();
        let mut mint_account = Account::new(0, size_of::<State>(), &program_id);
        let mint2_key = pubkey_rand();
        let mut mint2_account = Account::new(0, size_of::<State>(), &program_id);

        // invalid account
        assert_eq!(
            Err(ProgramError::InvalidArgument),
            do_process_instruction(
                set_owner(&program_id, &account_key, &owner2_key, &owner_key, &[]).unwrap(),
                vec![
                    &mut account_account,
                    &mut owner2_account,
                    &mut owner_account,
                ],
            )
        );

        // create account
        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
            vec![&mut account_account, &mut mint_account, &mut owner_account],
        )
        .unwrap();

        // create token account
        do_process_instruction(
            initialize_account(&program_id, &account2_key, &mint2_key, &owner_key).unwrap(),
            vec![
                &mut account2_account,
                &mut mint2_account,
                &mut owner_account,
            ],
        )
        .unwrap();

        // missing owner
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            do_process_instruction(
                set_owner(&program_id, &account_key, &owner_key, &owner2_key, &[]).unwrap(),
                vec![
                    &mut account_account,
                    &mut owner_account,
                    &mut owner2_account,
                ],
            )
        );

        // owner did not sign
        let mut instruction =
            set_owner(&program_id, &account_key, &owner2_key, &owner_key, &[]).unwrap();
        instruction.accounts[2].is_signer = false;
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            do_process_instruction(
                instruction,
                vec![
                    &mut account_account,
                    &mut owner2_account,
                    &mut owner_account,
                ],
            )
        );

        // set owner
        do_process_instruction(
            set_owner(&program_id, &account_key, &owner2_key, &owner_key, &[]).unwrap(),
            vec![
                &mut account_account,
                &mut owner2_account,
                &mut owner_account,
            ],
        )
        .unwrap();

        // create new token with owner
        do_process_instruction(
            initialize_mint(
                &program_id,
                &mint_key,
                Some(&account_key),
                Some(&owner_key),
                TokenInfo {
                    supply: 1000,
                    decimals: 2,
                },
            )
            .unwrap(),
            vec![&mut mint_account, &mut account_account, &mut owner_account],
        )
        .unwrap();

        // wrong account
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            do_process_instruction(
                set_owner(&program_id, &mint_key, &owner3_key, &owner2_key, &[]).unwrap(),
                vec![&mut mint_account, &mut owner3_account, &mut owner2_account],
            )
        );

        // owner did not sign
        let mut instruction =
            set_owner(&program_id, &mint_key, &owner2_key, &owner_key, &[]).unwrap();
        instruction.accounts[2].is_signer = false;
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            do_process_instruction(
                instruction,
                vec![&mut mint_account, &mut owner2_account, &mut owner_account],
            )
        );

        // set owner
        do_process_instruction(
            set_owner(&program_id, &mint_key, &owner2_key, &owner_key, &[]).unwrap(),
            vec![&mut mint_account, &mut owner2_account, &mut owner_account],
        )
        .unwrap();

        // create new token without owner
        do_process_instruction(
            initialize_mint(
                &program_id,
                &mint2_key,
                Some(&account2_key),
                None,
                TokenInfo {
                    supply: 1000,
                    decimals: 2,
                },
            )
            .unwrap(),
            vec![&mut mint2_account, &mut account2_account],
        )
        .unwrap();

        // set owner for non-mint-able token
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            do_process_instruction(
                set_owner(&program_id, &mint2_key, &owner2_key, &owner_key, &[]).unwrap(),
                vec![&mut mint_account, &mut owner2_account, &mut owner_account],
            )
        );
    }

    #[test]
    fn test_mint_to() {
        let program_id = pubkey_rand();
        let account_key = pubkey_rand();
        let mut account_account = Account::new(0, size_of::<State>(), &program_id);
        let account2_key = pubkey_rand();
        let mut account2_account = Account::new(0, size_of::<State>(), &program_id);
        let account3_key = pubkey_rand();
        let mut account3_account = Account::new(0, size_of::<State>(), &program_id);
        let mismatch_key = pubkey_rand();
        let mut mismatch_account = Account::new(0, size_of::<State>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = Account::default();
        let owner2_key = pubkey_rand();
        let mut owner2_account = Account::default();
        let mint_key = pubkey_rand();
        let mut mint_account = Account::new(0, size_of::<State>(), &program_id);
        let mint2_key = pubkey_rand();
        let mut mint2_account = Account::new(0, size_of::<State>(), &program_id);
        let uninitialized_key = pubkey_rand();
        let mut uninitialized_account = Account::new(0, size_of::<State>(), &program_id);

        // create token account
        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
            vec![&mut account_account, &mut mint_account, &mut owner_account],
        )
        .unwrap();

        // create another token account
        do_process_instruction(
            initialize_account(&program_id, &account2_key, &mint_key, &owner_key).unwrap(),
            vec![&mut account2_account, &mut mint_account, &mut owner_account],
        )
        .unwrap();

        // create another token account
        do_process_instruction(
            initialize_account(&program_id, &account3_key, &mint_key, &owner_key).unwrap(),
            vec![&mut account3_account, &mut mint_account, &mut owner_account],
        )
        .unwrap();

        // create mismatch token account
        do_process_instruction(
            initialize_account(&program_id, &mismatch_key, &mint2_key, &owner_key).unwrap(),
            vec![
                &mut mismatch_account,
                &mut mint2_account,
                &mut owner_account,
            ],
        )
        .unwrap();

        // create new token with owner
        do_process_instruction(
            initialize_mint(
                &program_id,
                &mint_key,
                Some(&account_key),
                Some(&owner_key),
                TokenInfo {
                    supply: 1000,
                    decimals: 2,
                },
            )
            .unwrap(),
            vec![&mut mint_account, &mut account_account, &mut owner_account],
        )
        .unwrap();

        // mint to
        do_process_instruction(
            mint_to(&program_id, &mint_key, &account2_key, &owner_key, &[], 42).unwrap(),
            vec![&mut mint_account, &mut account2_account, &mut owner_account],
        )
        .unwrap();

        if let State::Mint(token) = State::deserialize(&mint_account.data).unwrap() {
            assert_eq!(token.info.supply, 1000 + 42);
        } else {
            panic!("not an account");
        }
        if let State::Account(dest_account) = State::deserialize(&account2_account.data).unwrap() {
            assert_eq!(dest_account.amount, 42);
        } else {
            panic!("not an account");
        }

        // missing signer
        let mut instruction =
            mint_to(&program_id, &mint_key, &account2_key, &owner_key, &[], 42).unwrap();
        instruction.accounts[2].is_signer = false;
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            do_process_instruction(
                instruction,
                vec![&mut mint_account, &mut account2_account, &mut owner_account],
            )
        );

        // mismatch token
        assert_eq!(
            Err(TokenError::MintMismatch.into()),
            do_process_instruction(
                mint_to(&program_id, &mint_key, &mismatch_key, &owner_key, &[], 42).unwrap(),
                vec![&mut mint_account, &mut mismatch_account, &mut owner_account,],
            )
        );

        // missing owner
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            do_process_instruction(
                mint_to(&program_id, &mint_key, &account2_key, &owner2_key, &[], 42).unwrap(),
                vec![
                    &mut mint_account,
                    &mut account2_account,
                    &mut owner2_account,
                ],
            )
        );

        // uninitialized destination account
        assert_eq!(
            Err(ProgramError::InvalidArgument),
            do_process_instruction(
                mint_to(
                    &program_id,
                    &mint_key,
                    &uninitialized_key,
                    &owner_key,
                    &[],
                    42
                )
                .unwrap(),
                vec![
                    &mut mint_account,
                    &mut uninitialized_account,
                    &mut owner_account,
                ],
            )
        );
    }

    #[test]
    fn test_burn() {
        let program_id = pubkey_rand();
        let account_key = pubkey_rand();
        let mut account_account = Account::new(0, size_of::<State>(), &program_id);
        let account2_key = pubkey_rand();
        let mut account2_account = Account::new(0, size_of::<State>(), &program_id);
        let account3_key = pubkey_rand();
        let mut account3_account = Account::new(0, size_of::<State>(), &program_id);
        let delegate_key = pubkey_rand();
        let mut delegate_account = Account::default();
        let mismatch_key = pubkey_rand();
        let mut mismatch_account = Account::new(0, size_of::<State>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = Account::default();
        let owner2_key = pubkey_rand();
        let mut owner2_account = Account::default();
        let mint_key = pubkey_rand();
        let mut mint_account = Account::new(0, size_of::<State>(), &program_id);
        let mint2_key = pubkey_rand();
        let mut mint2_account = Account::new(0, size_of::<State>(), &program_id);

        // create token account
        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
            vec![&mut account_account, &mut mint_account, &mut owner_account],
        )
        .unwrap();

        // create another token account
        do_process_instruction(
            initialize_account(&program_id, &account2_key, &mint_key, &owner_key).unwrap(),
            vec![&mut account2_account, &mut mint_account, &mut owner_account],
        )
        .unwrap();

        // create another token account
        do_process_instruction(
            initialize_account(&program_id, &account3_key, &mint_key, &owner_key).unwrap(),
            vec![&mut account3_account, &mut mint_account, &mut owner_account],
        )
        .unwrap();

        // create mismatch token account
        do_process_instruction(
            initialize_account(&program_id, &mismatch_key, &mint2_key, &owner_key).unwrap(),
            vec![
                &mut mismatch_account,
                &mut mint2_account,
                &mut owner_account,
            ],
        )
        .unwrap();

        // create new token
        do_process_instruction(
            initialize_mint(
                &program_id,
                &mint_key,
                Some(&account_key),
                None,
                TokenInfo {
                    supply: 1000,
                    decimals: 2,
                },
            )
            .unwrap(),
            vec![&mut mint_account, &mut account_account],
        )
        .unwrap();

        // missing signer
        let mut instruction =
            burn(&program_id, &account_key, &mint_key, &delegate_key, &[], 42).unwrap();
        instruction.accounts[2].is_signer = false;
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            do_process_instruction(
                instruction,
                vec![
                    &mut account_account,
                    &mut mint_account,
                    &mut delegate_account
                ],
            )
        );

        // mismatch token
        assert_eq!(
            Err(TokenError::MintMismatch.into()),
            do_process_instruction(
                burn(&program_id, &mismatch_key, &mint_key, &owner_key, &[], 42).unwrap(),
                vec![&mut mismatch_account, &mut mint_account, &mut owner_account,],
            )
        );

        // missing owner
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            do_process_instruction(
                burn(&program_id, &account_key, &mint_key, &owner2_key, &[], 42).unwrap(),
                vec![&mut account_account, &mut mint_account, &mut owner2_account],
            )
        );

        // burn
        do_process_instruction(
            burn(&program_id, &account_key, &mint_key, &owner_key, &[], 42).unwrap(),
            vec![&mut account_account, &mut mint_account, &mut owner_account],
        )
        .unwrap();

        if let State::Mint(token) = State::deserialize(&mint_account.data).unwrap() {
            assert_eq!(token.info.supply, 1000 - 42);
        } else {
            panic!("not a token account");
        }
        if let State::Account(account) = State::deserialize(&account_account.data).unwrap() {
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
                    &account_key,
                    &mint_key,
                    &owner_key,
                    &[],
                    100_000_000
                )
                .unwrap(),
                vec![&mut account_account, &mut mint_account, &mut owner_account],
            )
        );

        // approve delegate
        do_process_instruction(
            approve(
                &program_id,
                &account_key,
                &delegate_key,
                &owner_key,
                &[],
                84,
            )
            .unwrap(),
            vec![
                &mut account_account,
                &mut delegate_account,
                &mut owner_account,
            ],
        )
        .unwrap();

        // not a delegate of source account
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            do_process_instruction(
                burn(
                    &program_id,
                    &account_key,
                    &mint_key,
                    &owner_key,
                    &[],
                    100_000_000
                )
                .unwrap(),
                vec![&mut account_account, &mut mint_account, &mut owner_account],
            )
        );

        // burn via delegate
        do_process_instruction(
            burn(&program_id, &account_key, &mint_key, &delegate_key, &[], 84).unwrap(),
            vec![
                &mut account_account,
                &mut mint_account,
                &mut delegate_account,
            ],
        )
        .unwrap();

        if let State::Mint(token) = State::deserialize(&mint_account.data).unwrap() {
            assert_eq!(token.info.supply, 1000 - 42 - 84);
        } else {
            panic!("not a token account");
        }
        if let State::Account(account) = State::deserialize(&account_account.data).unwrap() {
            assert_eq!(account.amount, 1000 - 42 - 84);
        } else {
            panic!("not an account");
        }

        // insufficient funds approved via delegate
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            do_process_instruction(
                burn(
                    &program_id,
                    &account_key,
                    &mint_key,
                    &delegate_key,
                    &[],
                    100
                )
                .unwrap(),
                vec![
                    &mut account_account,
                    &mut mint_account,
                    &mut delegate_account,
                ],
            )
        );
    }

    #[test]
    fn test_multisig() {
        let program_id = pubkey_rand();
        let mint_key = pubkey_rand();
        let mut mint_account = Account::new(0, size_of::<State>(), &program_id);
        let account_key = pubkey_rand();
        let mut account = Account::new(0, size_of::<State>(), &program_id);
        let account2_key = pubkey_rand();
        let mut account2_account = Account::new(0, size_of::<State>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = Account::default();
        let multisig_key = pubkey_rand();
        let mut multisig_account = Account::new(0, size_of::<Multisig>(), &program_id);
        let multisig_delegate_key = pubkey_rand();
        let mut multisig_delegate_account = Account::new(0, size_of::<Multisig>(), &program_id);
        let signer_keys = vec![pubkey_rand(); MAX_SIGNERS];
        let signer_key_refs: Vec<&Pubkey> = signer_keys.iter().map(|key| key).collect();
        let mut signer_accounts = vec![Account::new(0, 0, &program_id); MAX_SIGNERS];

        // single signer
        let account_info_iter = &mut signer_accounts.iter_mut();
        do_process_instruction(
            initialize_multisig(&program_id, &multisig_key, &[&signer_keys[0]], 1).unwrap(),
            vec![
                &mut multisig_account,
                &mut account_info_iter.next().unwrap(),
            ],
        )
        .unwrap();

        // multiple signer
        let account_info_iter = &mut signer_accounts.iter_mut();
        do_process_instruction(
            initialize_multisig(
                &program_id,
                &multisig_delegate_key,
                &signer_key_refs,
                MAX_SIGNERS as u8,
            )
            .unwrap(),
            vec![
                &mut multisig_delegate_account,
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
            ],
        )
        .unwrap();

        // create token account with multisig owner
        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, &multisig_key).unwrap(),
            vec![&mut account, &mut mint_account, &mut multisig_account],
        )
        .unwrap();

        // create another token account with multisig owner
        do_process_instruction(
            initialize_account(
                &program_id,
                &account2_key,
                &mint_key,
                &multisig_delegate_key,
            )
            .unwrap(),
            vec![
                &mut account2_account,
                &mut mint_account,
                &mut multisig_account,
            ],
        )
        .unwrap();

        // create new token with multisig owner
        do_process_instruction(
            initialize_mint(
                &program_id,
                &mint_key,
                Some(&account_key),
                Some(&multisig_key),
                TokenInfo {
                    supply: 1000,
                    decimals: 2,
                },
            )
            .unwrap(),
            vec![&mut mint_account, &mut account, &mut multisig_account],
        )
        .unwrap();

        // approve
        let account_info_iter = &mut signer_accounts.iter_mut();
        do_process_instruction(
            approve(
                &program_id,
                &account_key,
                &multisig_delegate_key,
                &multisig_key,
                &[&signer_keys[0]],
                100,
            )
            .unwrap(),
            vec![
                &mut account,
                &mut multisig_delegate_account,
                &mut multisig_account,
                &mut account_info_iter.next().unwrap(),
            ],
        )
        .unwrap();

        // transfer
        let account_info_iter = &mut signer_accounts.iter_mut();
        do_process_instruction(
            transfer(
                &program_id,
                &account_key,
                &account2_key,
                &multisig_key,
                &[&signer_keys[0]],
                42,
            )
            .unwrap(),
            vec![
                &mut account,
                &mut account2_account,
                &mut multisig_account,
                &mut account_info_iter.next().unwrap(),
            ],
        )
        .unwrap();

        // transfer via delegate
        let account_info_iter = &mut signer_accounts.iter_mut();
        do_process_instruction(
            transfer(
                &program_id,
                &account_key,
                &account2_key,
                &multisig_delegate_key,
                &signer_key_refs,
                42,
            )
            .unwrap(),
            vec![
                &mut account,
                &mut account2_account,
                &mut multisig_delegate_account,
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
            ],
        )
        .unwrap();

        // mint to
        let account_info_iter = &mut signer_accounts.iter_mut();
        do_process_instruction(
            mint_to(
                &program_id,
                &mint_key,
                &account2_key,
                &multisig_key,
                &[&signer_keys[0]],
                42,
            )
            .unwrap(),
            vec![
                &mut mint_account,
                &mut account2_account,
                &mut multisig_account,
                &mut account_info_iter.next().unwrap(),
            ],
        )
        .unwrap();

        // burn
        let account_info_iter = &mut signer_accounts.iter_mut();
        do_process_instruction(
            burn(
                &program_id,
                &account_key,
                &mint_key,
                &multisig_key,
                &[&signer_keys[0]],
                42,
            )
            .unwrap(),
            vec![
                &mut account,
                &mut mint_account,
                &mut multisig_account,
                &mut account_info_iter.next().unwrap(),
            ],
        )
        .unwrap();

        // burn via delegate
        let account_info_iter = &mut signer_accounts.iter_mut();
        do_process_instruction(
            burn(
                &program_id,
                &account_key,
                &mint_key,
                &multisig_delegate_key,
                &signer_key_refs,
                42,
            )
            .unwrap(),
            vec![
                &mut account,
                &mut mint_account,
                &mut multisig_delegate_account,
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
                &mut account_info_iter.next().unwrap(),
            ],
        )
        .unwrap();

        // do SetOwner on mint
        let account_info_iter = &mut signer_accounts.iter_mut();
        do_process_instruction(
            set_owner(
                &program_id,
                &mint_key,
                &owner_key,
                &multisig_key,
                &[&signer_keys[0]],
            )
            .unwrap(),
            vec![
                &mut mint_account,
                &mut owner_account,
                &mut multisig_account,
                &mut account_info_iter.next().unwrap(),
            ],
        )
        .unwrap();

        // do SetOwner on account
        let account_info_iter = &mut signer_accounts.iter_mut();
        do_process_instruction(
            set_owner(
                &program_id,
                &account_key,
                &owner_key,
                &multisig_key,
                &[&signer_keys[0]],
            )
            .unwrap(),
            vec![
                &mut account,
                &mut owner_account,
                &mut multisig_account,
                &mut account_info_iter.next().unwrap(),
            ],
        )
        .unwrap();
    }

    #[test]
    fn test_validate_owner() {
        let program_id = pubkey_rand();
        let owner_key = pubkey_rand();
        let mut signer_keys = [Pubkey::default(); MAX_SIGNERS];
        for i in 0..MAX_SIGNERS {
            signer_keys[i] = pubkey_rand();
        }
        let mut signer_lamports = 0;
        let mut signer_data = vec![];
        let mut signers = vec![
            AccountInfo::new(
                &owner_key,
                true,
                false,
                &mut signer_lamports,
                &mut signer_data,
                &program_id,
                false,
                Epoch::default(),
            );
            MAX_SIGNERS + 1
        ];
        for (signer, key) in signers.iter_mut().zip(&signer_keys) {
            signer.key = key;
        }
        let mut lamports = 0;
        let mut data = vec![0; size_of::<Multisig>()];
        let mut multisig = Multisig::deserialize(&mut data).unwrap();
        multisig.m = MAX_SIGNERS as u8;
        multisig.n = MAX_SIGNERS as u8;
        multisig.signers = signer_keys.clone();
        let owner_account_info = AccountInfo::new(
            &owner_key,
            false,
            false,
            &mut lamports,
            &mut data,
            &program_id,
            false,
            Epoch::default(),
        );

        // full 11 of 11
        State::validate_owner(&program_id, &owner_key, &owner_account_info, &signers).unwrap();

        // 1 of 11
        {
            let mut data_ref_mut = owner_account_info.data.borrow_mut();
            let mut multisig = Multisig::deserialize(&mut data_ref_mut).unwrap();
            multisig.m = 1;
        }
        State::validate_owner(&program_id, &owner_key, &owner_account_info, &signers).unwrap();

        // 2:1
        {
            let mut data_ref_mut = owner_account_info.data.borrow_mut();
            let mut multisig = Multisig::deserialize(&mut data_ref_mut).unwrap();
            multisig.m = 2;
            multisig.n = 1;
        }
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            State::validate_owner(&program_id, &owner_key, &owner_account_info, &signers)
        );

        // 0:11
        {
            let mut data_ref_mut = owner_account_info.data.borrow_mut();
            let mut multisig = Multisig::deserialize(&mut data_ref_mut).unwrap();
            multisig.m = 0;
            multisig.n = 11;
        }
        State::validate_owner(&program_id, &owner_key, &owner_account_info, &signers).unwrap();

        // 2:11 but 0 provided
        {
            let mut data_ref_mut = owner_account_info.data.borrow_mut();
            let mut multisig = Multisig::deserialize(&mut data_ref_mut).unwrap();
            multisig.m = 2;
            multisig.n = 11;
        }
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            State::validate_owner(&program_id, &owner_key, &owner_account_info, &[])
        );
        // 2:11 but 1 provided
        {
            let mut data_ref_mut = owner_account_info.data.borrow_mut();
            let mut multisig = Multisig::deserialize(&mut data_ref_mut).unwrap();
            multisig.m = 2;
            multisig.n = 11;
        }
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            State::validate_owner(&program_id, &owner_key, &owner_account_info, &signers[0..1])
        );

        // 2:11, 2 from middle provided
        {
            let mut data_ref_mut = owner_account_info.data.borrow_mut();
            let mut multisig = Multisig::deserialize(&mut data_ref_mut).unwrap();
            multisig.m = 2;
            multisig.n = 11;
        }
        State::validate_owner(&program_id, &owner_key, &owner_account_info, &signers[5..7])
            .unwrap();

        // 11:11, one is not a signer
        {
            let mut data_ref_mut = owner_account_info.data.borrow_mut();
            let mut multisig = Multisig::deserialize(&mut data_ref_mut).unwrap();
            multisig.m = 2;
            multisig.n = 11;
        }
        signers[5].is_signer = false;
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            State::validate_owner(&program_id, &owner_key, &owner_account_info, &signers)
        );
        signers[5].is_signer = true;
    }
}
