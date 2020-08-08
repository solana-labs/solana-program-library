//! Program state processor

#![cfg(feature = "program")]

use crate::{
    error::TokenError,
    instruction::{is_valid_signer_index, TokenInstruction},
    option::COption,
    state::{self, Account, Mint, Multisig},
};
use num_traits::FromPrimitive;
use solana_sdk::{
    account_info::{next_account_info, AccountInfo},
    decode_error::DecodeError,
    entrypoint::ProgramResult,
    info,
    program_error::{PrintProgramError, ProgramError},
    pubkey::Pubkey,
};
use std::mem::size_of;

/// Program state handler.
pub struct Processor {}
impl Processor {
    /// Processes an [InitializeMint](enum.TokenInstruction.html) instruction.
    pub fn process_initialize_mint(
        accounts: &[AccountInfo],
        amount: u64,
        decimals: u8,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let mint_info = next_account_info(account_info_iter)?;

        let mut mint_info_data = mint_info.data.borrow_mut();
        let mut mint: &mut Mint = state::unpack_unchecked(&mut mint_info_data)?;
        if mint.is_initialized {
            return Err(TokenError::AlreadyInUse.into());
        }

        let owner = if amount != 0 {
            let dest_account_info = next_account_info(account_info_iter)?;
            let mut dest_account_data = dest_account_info.data.borrow_mut();
            let mut dest_account: &mut Account = state::unpack(&mut dest_account_data)?;

            if mint_info.key != &dest_account.mint {
                return Err(TokenError::MintMismatch.into());
            }

            dest_account.amount = amount;

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

        mint.owner = owner;
        mint.decimals = decimals;
        mint.is_initialized = true;

        Ok(())
    }

    /// Processes an [InitializeAccount](enum.TokenInstruction.html) instruction.
    pub fn process_initialize_account(accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let new_account_info = next_account_info(account_info_iter)?;
        let mint_info = next_account_info(account_info_iter)?;
        let owner_info = next_account_info(account_info_iter)?;

        let mut new_account_data = new_account_info.data.borrow_mut();
        let mut account: &mut Account = state::unpack_unchecked(&mut new_account_data)?;
        if account.is_initialized {
            return Err(TokenError::AlreadyInUse.into());
        }

        account.mint = *mint_info.key;
        account.owner = *owner_info.key;
        account.delegate = COption::None;
        account.delegated_amount = 0;
        account.is_initialized = true;
        if *mint_info.key == crate::native_mint::id() {
            account.is_native = true;
            account.amount = new_account_info.lamports();
        } else {
            account.is_native = false;
            account.amount = 0;
        };

        Ok(())
    }

    /// Processes a [InitializeMultisig](enum.TokenInstruction.html) instruction.
    pub fn process_initialize_multisig(accounts: &[AccountInfo], m: u8) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let multisig_info = next_account_info(account_info_iter)?;
        let mut multisig_account_data = multisig_info.data.borrow_mut();
        let mut multisig: &mut Multisig = state::unpack_unchecked(&mut multisig_account_data)?;
        if multisig.is_initialized {
            return Err(TokenError::AlreadyInUse.into());
        }

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
        multisig.is_initialized = true;

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

        if source_account_info.key == dest_account_info.key {
            return Ok(());
        }

        let mut source_data = source_account_info.data.borrow_mut();
        let mut source_account: &mut Account = state::unpack(&mut source_data)?;
        let mut dest_data = dest_account_info.data.borrow_mut();
        let mut dest_account: &mut Account = state::unpack(&mut dest_data)?;

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

        if source_account.is_native {
            **source_account_info.lamports.borrow_mut() -= amount;
            **dest_account_info.lamports.borrow_mut() += amount;
        }

        Ok(())
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
        let mut source_account: &mut Account = state::unpack(&mut source_data)?;
        let delegate_info = next_account_info(account_info_iter)?;
        let owner_info = next_account_info(account_info_iter)?;

        Self::validate_owner(
            program_id,
            &source_account.owner,
            owner_info,
            account_info_iter.as_slice(),
        )?;

        source_account.delegate = COption::Some(*delegate_info.key);
        source_account.delegated_amount = amount;

        Ok(())
    }

    /// Processes an [Revoke](enum.TokenInstruction.html) instruction.
    pub fn process_revoke(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;

        let mut source_data = source_account_info.data.borrow_mut();
        let mut source_account: &mut Account = state::unpack(&mut source_data)?;
        let owner_info = next_account_info(account_info_iter)?;

        Self::validate_owner(
            program_id,
            &source_account.owner,
            owner_info,
            account_info_iter.as_slice(),
        )?;

        source_account.delegate = COption::None;
        source_account.delegated_amount = 0;

        Ok(())
    }

    /// Processes a [SetOwner](enum.TokenInstruction.html) instruction.
    pub fn process_set_owner(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let account_info = next_account_info(account_info_iter)?;
        let new_owner_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;

        if account_info.data_len() == size_of::<Account>() {
            let mut account_data = account_info.data.borrow_mut();
            let mut account: &mut Account = state::unpack(&mut account_data)?;

            Self::validate_owner(
                program_id,
                &account.owner,
                authority_info,
                account_info_iter.as_slice(),
            )?;

            account.owner = *new_owner_info.key;
        } else if account_info.data_len() == size_of::<Mint>() {
            let mut account_data = account_info.data.borrow_mut();
            let mut mint: &mut Mint = state::unpack(&mut account_data)?;

            match mint.owner {
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
            mint.owner = COption::Some(*new_owner_info.key);
        } else {
            return Err(ProgramError::InvalidArgument);
        }

        Ok(())
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

        let mut dest_account_data = dest_account_info.data.borrow_mut();
        let mut dest_account: &mut Account = state::unpack(&mut dest_account_data)?;

        if dest_account.is_native {
            return Err(TokenError::NativeNotSupported.into());
        }
        if mint_info.key != &dest_account.mint {
            return Err(TokenError::MintMismatch.into());
        }

        let mut mint_info_data = mint_info.data.borrow_mut();
        let mint: &mut Mint = state::unpack(&mut mint_info_data)?;

        match mint.owner {
            COption::Some(owner) => {
                Self::validate_owner(program_id, &owner, owner_info, account_info_iter.as_slice())?;
            }
            COption::None => {
                return Err(TokenError::FixedSupply.into());
            }
        }

        dest_account.amount += amount;

        Ok(())
    }

    /// Processes a [Burn](enum.TokenInstruction.html) instruction.
    pub fn process_burn(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;

        let mut source_data = source_account_info.data.borrow_mut();
        let source_account: &mut Account = state::unpack(&mut source_data)?;

        if source_account.is_native {
            return Err(TokenError::NativeNotSupported.into());
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

        Ok(())
    }

    /// Processes a [CloseAccount](enum.TokenInstruction.html) instruction.
    pub fn process_close_account(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let dest_account_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;

        let mut source_data = source_account_info.data.borrow_mut();
        let source_account: &mut Account = state::unpack(&mut source_data)?;

        if !source_account.is_native {
            return Err(TokenError::NonNativeNotSupported.into());
        }

        Self::validate_owner(
            program_id,
            &source_account.owner,
            authority_info,
            account_info_iter.as_slice(),
        )?;

        **dest_account_info.lamports.borrow_mut() += source_account_info.lamports();
        **source_account_info.lamports.borrow_mut() = 0;
        source_account.amount = 0;

        Ok(())
    }

    /// Processes an [Instruction](enum.Instruction.html).
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        let instruction = TokenInstruction::unpack(input)?;

        match instruction {
            TokenInstruction::InitializeMint { amount, decimals } => {
                info!("Instruction: InitializeMint");
                Self::process_initialize_mint(accounts, amount, decimals)
            }
            TokenInstruction::InitializeAccount => {
                info!("Instruction: InitializeAccount");
                Self::process_initialize_account(accounts)
            }
            TokenInstruction::InitializeMultisig { m } => {
                info!("Instruction: InitializeMultisig");
                Self::process_initialize_multisig(accounts, m)
            }
            TokenInstruction::Transfer { amount } => {
                info!("Instruction: Transfer");
                Self::process_transfer(program_id, accounts, amount)
            }
            TokenInstruction::Approve { amount } => {
                info!("Instruction: Approve");
                Self::process_approve(program_id, accounts, amount)
            }
            TokenInstruction::Revoke => {
                info!("Instruction: Revoke");
                Self::process_revoke(program_id, accounts)
            }
            TokenInstruction::SetOwner => {
                info!("Instruction: SetOwner");
                Self::process_set_owner(program_id, accounts)
            }
            TokenInstruction::MintTo { amount } => {
                info!("Instruction: MintTo");
                Self::process_mint_to(program_id, accounts, amount)
            }
            TokenInstruction::Burn { amount } => {
                info!("Instruction: Burn");
                Self::process_burn(program_id, accounts, amount)
            }
            TokenInstruction::CloseAccount => {
                info!("Instruction: CloseAccount");
                Self::process_close_account(program_id, accounts)
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
            let multisig: &mut Multisig = state::unpack(&mut owner_data)?;
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
}

impl PrintProgramError for TokenError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            TokenError::InsufficientFunds => info!("Error: insufficient funds"),
            TokenError::MintMismatch => info!("Error: Account not associated with this Mint"),
            TokenError::OwnerMismatch => info!("Error: owner does not match"),
            TokenError::FixedSupply => info!("Error: the total supply of this token is fixed"),
            TokenError::AlreadyInUse => info!("Error: account or token already in use"),
            TokenError::OwnerRequiredIfNoInitialSupply => {
                info!("Error: An owner is required if supply is zero")
            }
            TokenError::InvalidNumberOfProvidedSigners => {
                info!("Error: Invalid number of provided signers")
            }
            TokenError::InvalidNumberOfRequiredSigners => {
                info!("Error: Invalid number of required signers")
            }
            TokenError::UninitializedState => info!("Error: State is uninitialized"),
            TokenError::NativeNotSupported => {
                info!("Error: Instruction does not support native tokens")
            }
            TokenError::NonNativeNotSupported => {
                info!("Error: Instruction does not support non-native tokens")
            }
            TokenError::InvalidInstruction => info!("Error: Invalid instruction"),
        }
    }
}

// Pull in syscall stubs when building for non-BPF targets
#[cfg(not(target_arch = "bpf"))]
solana_sdk::program_stubs!();

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::{
        approve, burn, close_account, initialize_account, initialize_mint, initialize_multisig,
        mint_to, revoke, set_owner, transfer, MAX_SIGNERS,
    };
    use solana_sdk::{
        account::Account as SolanaAccount, account_info::create_is_signer_account_infos,
        clock::Epoch, instruction::Instruction,
    };

    fn pubkey_rand() -> Pubkey {
        Pubkey::new(&rand::random::<[u8; 32]>())
    }

    fn do_process_instruction(
        instruction: Instruction,
        accounts: Vec<&mut SolanaAccount>,
    ) -> ProgramResult {
        let mut meta = instruction
            .accounts
            .iter()
            .zip(accounts)
            .map(|(account_meta, account)| (&account_meta.pubkey, account_meta.is_signer, account))
            .collect::<Vec<_>>();

        let account_infos = create_is_signer_account_infos(&mut meta);
        Processor::process(&instruction.program_id, &account_infos, &instruction.data)
    }

    fn return_token_error_as_program_error() -> ProgramError {
        TokenError::MintMismatch.into()
    }

    #[test]
    fn test_print_error() {
        let error = return_token_error_as_program_error();
        error.print::<TokenError>();
    }

    #[test]
    #[should_panic(expected = "Custom(1)")]
    fn test_error_unwrap() {
        Err::<(), ProgramError>(return_token_error_as_program_error()).unwrap();
    }

    #[test]
    fn test_unique_account_sizes() {
        assert_ne!(size_of::<Mint>(), 0);
        assert_ne!(size_of::<Mint>(), size_of::<Account>());
        assert_ne!(size_of::<Mint>(), size_of::<Multisig>());
        assert_ne!(size_of::<Account>(), 0);
        assert_ne!(size_of::<Account>(), size_of::<Multisig>());
        assert_ne!(size_of::<Multisig>(), 0);
    }

    #[test]
    fn test_initialize_mint() {
        let program_id = pubkey_rand();
        let account_key = pubkey_rand();
        let mut account_account = SolanaAccount::new(0, size_of::<Account>(), &program_id);
        let account2_key = pubkey_rand();
        let mut account2_account = SolanaAccount::new(0, size_of::<Account>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = SolanaAccount::default();
        let mint_key = pubkey_rand();
        let mut mint_account = SolanaAccount::new(0, size_of::<Mint>(), &program_id);
        let mint2_key = pubkey_rand();
        let mut mint2_account = SolanaAccount::new(0, size_of::<Mint>(), &program_id);

        // account not created
        assert_eq!(
            Err(TokenError::UninitializedState.into()),
            do_process_instruction(
                initialize_mint(&program_id, &mint_key, Some(&account_key), None, 1000, 2).unwrap(),
                vec![&mut mint_account, &mut account_account]
            )
        );

        // create account
        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
            vec![&mut account_account, &mut owner_account, &mut mint_account],
        )
        .unwrap();

        // create new mint
        do_process_instruction(
            initialize_mint(&program_id, &mint_key, Some(&account_key), None, 1000, 2).unwrap(),
            vec![&mut mint_account, &mut account_account],
        )
        .unwrap();

        // create another account
        do_process_instruction(
            initialize_account(&program_id, &account2_key, &mint_key, &owner_key).unwrap(),
            vec![&mut account2_account, &mut owner_account, &mut mint_account],
        )
        .unwrap();

        // mismatch account
        assert_eq!(
            Err(TokenError::MintMismatch.into()),
            do_process_instruction(
                initialize_mint(&program_id, &mint2_key, Some(&account2_key), None, 1000, 2)
                    .unwrap(),
                vec![&mut mint2_account, &mut account2_account]
            )
        );

        // create twice
        assert_eq!(
            Err(TokenError::AlreadyInUse.into()),
            do_process_instruction(
                initialize_mint(&program_id, &mint_key, Some(&account_key), None, 1000, 2).unwrap(),
                vec![&mut mint_account, &mut account_account]
            )
        );
    }

    #[test]
    fn test_initialize_mint_account() {
        let program_id = pubkey_rand();
        let account_key = pubkey_rand();
        let mut account_account = SolanaAccount::new(0, size_of::<Account>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = SolanaAccount::default();
        let mint_key = pubkey_rand();
        let mut mint_account = SolanaAccount::new(0, size_of::<Mint>(), &program_id);

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
        let mut account_account = SolanaAccount::new(0, size_of::<Account>(), &program_id);
        let account2_key = pubkey_rand();
        let mut account2_account = SolanaAccount::new(0, size_of::<Account>(), &program_id);
        let account3_key = pubkey_rand();
        let mut account3_account = SolanaAccount::new(0, size_of::<Account>(), &program_id);
        let delegate_key = pubkey_rand();
        let mut delegate_account = SolanaAccount::default();
        let mismatch_key = pubkey_rand();
        let mut mismatch_account = SolanaAccount::new(0, size_of::<Account>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = SolanaAccount::default();
        let owner2_key = pubkey_rand();
        let mut owner2_account = SolanaAccount::default();
        let mint_key = pubkey_rand();
        let mut mint_account = SolanaAccount::new(0, size_of::<Mint>(), &program_id);
        let mint2_key = pubkey_rand();
        let mut mint2_account = SolanaAccount::new(0, size_of::<Mint>(), &program_id);

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

        // create new mint
        do_process_instruction(
            initialize_mint(&program_id, &mint_key, Some(&account_key), None, 1000, 2).unwrap(),
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

        // mismatch mint
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

        // transfer to self
        {
            let instruction = transfer(
                &program_id,
                &account_key,
                &account_key,
                &owner_key,
                &[],
                500,
            )
            .unwrap();
            let account_account_info = AccountInfo::from((
                &instruction.accounts[0].pubkey,
                instruction.accounts[0].is_signer,
                &mut account_account,
            ));
            let owner_account_info = AccountInfo::from((
                &instruction.accounts[2].pubkey,
                instruction.accounts[2].is_signer,
                &mut owner_account,
            ));
            Processor::process(
                &instruction.program_id,
                &[
                    account_account_info.clone(),
                    account_account_info,
                    owner_account_info,
                ],
                &instruction.data,
            )
            .unwrap()
        }
        let account: &mut Account = state::unpack(&mut account_account.data).unwrap();
        assert_eq!(account.amount, 1000);

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
        let mut account_account = SolanaAccount::new(0, size_of::<Account>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = SolanaAccount::default();
        let mint_key = pubkey_rand();
        let mut mint_account = SolanaAccount::new(0, size_of::<Mint>(), &program_id);

        // create account
        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
            vec![&mut account_account, &mut owner_account, &mut mint_account],
        )
        .unwrap();

        // create mint-able token without owner
        let mut instruction =
            initialize_mint(&program_id, &mint_key, None, Some(&owner_key), 0, 2).unwrap();
        instruction.accounts.pop();
        assert_eq!(
            Err(TokenError::OwnerRequiredIfNoInitialSupply.into()),
            do_process_instruction(instruction, vec![&mut mint_account])
        );

        // create mint-able token with zero supply
        let amount = 0;
        let decimals = 2;
        do_process_instruction(
            initialize_mint(
                &program_id,
                &mint_key,
                None,
                Some(&owner_key),
                amount,
                decimals,
            )
            .unwrap(),
            vec![&mut mint_account, &mut account_account],
        )
        .unwrap();
        let mint: &mut Mint = state::unpack(&mut mint_account.data).unwrap();
        assert_eq!(
            *mint,
            Mint {
                owner: COption::Some(owner_key),
                decimals,
                is_initialized: true,
            }
        );

        // mint to
        do_process_instruction(
            mint_to(&program_id, &mint_key, &account_key, &owner_key, &[], 42).unwrap(),
            vec![&mut mint_account, &mut account_account, &mut owner_account],
        )
        .unwrap();

        let _: &mut Mint = state::unpack(&mut mint_account.data).unwrap();
        let dest_account: &mut Account = state::unpack(&mut account_account.data).unwrap();
        assert_eq!(dest_account.amount, 42);
    }

    #[test]
    fn test_approve() {
        let program_id = pubkey_rand();
        let account_key = pubkey_rand();
        let mut account_account = SolanaAccount::new(0, size_of::<Account>(), &program_id);
        let account2_key = pubkey_rand();
        let mut account2_account = SolanaAccount::new(0, size_of::<Account>(), &program_id);
        let delegate_key = pubkey_rand();
        let mut delegate_account = SolanaAccount::default();
        let owner_key = pubkey_rand();
        let mut owner_account = SolanaAccount::default();
        let owner2_key = pubkey_rand();
        let mut owner2_account = SolanaAccount::default();
        let mint_key = pubkey_rand();
        let mut mint_account = SolanaAccount::new(0, size_of::<Mint>(), &program_id);

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

        // create new mint
        do_process_instruction(
            initialize_mint(&program_id, &mint_key, Some(&account_key), None, 1000, 2).unwrap(),
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

        // revoke delegate
        do_process_instruction(
            revoke(&program_id, &account_key, &owner_key, &[]).unwrap(),
            vec![&mut account_account, &mut owner_account],
        )
        .unwrap();
    }

    #[test]
    fn test_set_owner() {
        let program_id = pubkey_rand();
        let account_key = pubkey_rand();
        let mut account_account = SolanaAccount::new(0, size_of::<Account>(), &program_id);
        let account2_key = pubkey_rand();
        let mut account2_account = SolanaAccount::new(0, size_of::<Account>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = SolanaAccount::default();
        let owner2_key = pubkey_rand();
        let mut owner2_account = SolanaAccount::default();
        let owner3_key = pubkey_rand();
        let mut owner3_account = SolanaAccount::default();
        let mint_key = pubkey_rand();
        let mut mint_account = SolanaAccount::new(0, size_of::<Mint>(), &program_id);
        let mint2_key = pubkey_rand();
        let mut mint2_account = SolanaAccount::new(0, size_of::<Mint>(), &program_id);

        // invalid account
        assert_eq!(
            Err(TokenError::UninitializedState.into()),
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

        // create another account
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

        // create new mint with owner
        do_process_instruction(
            initialize_mint(
                &program_id,
                &mint_key,
                Some(&account_key),
                Some(&owner_key),
                1000,
                2,
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

        // create new mint without owner
        do_process_instruction(
            initialize_mint(&program_id, &mint2_key, Some(&account2_key), None, 1000, 2).unwrap(),
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
        let mut account_account = SolanaAccount::new(0, size_of::<Account>(), &program_id);
        let account2_key = pubkey_rand();
        let mut account2_account = SolanaAccount::new(0, size_of::<Account>(), &program_id);
        let account3_key = pubkey_rand();
        let mut account3_account = SolanaAccount::new(0, size_of::<Account>(), &program_id);
        let mismatch_key = pubkey_rand();
        let mut mismatch_account = SolanaAccount::new(0, size_of::<Account>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = SolanaAccount::default();
        let owner2_key = pubkey_rand();
        let mut owner2_account = SolanaAccount::default();
        let mint_key = pubkey_rand();
        let mut mint_account = SolanaAccount::new(0, size_of::<Mint>(), &program_id);
        let mint2_key = pubkey_rand();
        let mut mint2_account = SolanaAccount::new(0, size_of::<Mint>(), &program_id);
        let uninitialized_key = pubkey_rand();
        let mut uninitialized_account = SolanaAccount::new(0, size_of::<Account>(), &program_id);

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

        // create new mint with owner
        do_process_instruction(
            initialize_mint(
                &program_id,
                &mint_key,
                Some(&account_key),
                Some(&owner_key),
                1000,
                2,
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

        let _: &mut Mint = state::unpack(&mut mint_account.data).unwrap();
        let dest_account: &mut Account = state::unpack(&mut account2_account.data).unwrap();
        assert_eq!(dest_account.amount, 42);

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

        // mismatch account
        assert_eq!(
            Err(TokenError::MintMismatch.into()),
            do_process_instruction(
                mint_to(&program_id, &mint_key, &mismatch_key, &owner_key, &[], 42).unwrap(),
                vec![&mut mint_account, &mut mismatch_account, &mut owner_account],
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
            Err(TokenError::UninitializedState.into()),
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
        let mut account_account = SolanaAccount::new(0, size_of::<Account>(), &program_id);
        let account2_key = pubkey_rand();
        let mut account2_account = SolanaAccount::new(0, size_of::<Account>(), &program_id);
        let account3_key = pubkey_rand();
        let mut account3_account = SolanaAccount::new(0, size_of::<Account>(), &program_id);
        let delegate_key = pubkey_rand();
        let mut delegate_account = SolanaAccount::default();
        let mismatch_key = pubkey_rand();
        let mut mismatch_account = SolanaAccount::new(0, size_of::<Account>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = SolanaAccount::default();
        let owner2_key = pubkey_rand();
        let mut owner2_account = SolanaAccount::default();
        let mint_key = pubkey_rand();
        let mut mint_account = SolanaAccount::new(0, size_of::<Mint>(), &program_id);
        let mint2_key = pubkey_rand();
        let mut mint2_account = SolanaAccount::new(0, size_of::<Mint>(), &program_id);

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

        // create new mint
        do_process_instruction(
            initialize_mint(&program_id, &mint_key, Some(&account_key), None, 1000, 2).unwrap(),
            vec![&mut mint_account, &mut account_account],
        )
        .unwrap();

        // missing signer
        let mut instruction = burn(&program_id, &account_key, &delegate_key, &[], 42).unwrap();
        instruction.accounts[1].is_signer = false;
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            do_process_instruction(
                instruction,
                vec![&mut account_account, &mut delegate_account],
            )
        );

        // missing owner
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            do_process_instruction(
                burn(&program_id, &account_key, &owner2_key, &[], 42).unwrap(),
                vec![&mut account_account, &mut owner2_account],
            )
        );

        // burn
        do_process_instruction(
            burn(&program_id, &account_key, &owner_key, &[], 42).unwrap(),
            vec![&mut account_account, &mut owner_account],
        )
        .unwrap();

        let _: &mut Mint = state::unpack(&mut mint_account.data).unwrap();
        let account: &mut Account = state::unpack(&mut account_account.data).unwrap();
        assert_eq!(account.amount, 1000 - 42);

        // insufficient funds
        assert_eq!(
            Err(TokenError::InsufficientFunds.into()),
            do_process_instruction(
                burn(&program_id, &account_key, &owner_key, &[], 100_000_000).unwrap(),
                vec![&mut account_account, &mut owner_account],
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
                burn(&program_id, &account_key, &owner_key, &[], 100_000_000).unwrap(),
                vec![&mut account_account, &mut owner_account],
            )
        );

        // burn via delegate
        do_process_instruction(
            burn(&program_id, &account_key, &delegate_key, &[], 84).unwrap(),
            vec![&mut account_account, &mut delegate_account],
        )
        .unwrap();

        // match
        let _: &mut Mint = state::unpack(&mut mint_account.data).unwrap();
        let account: &mut Account = state::unpack(&mut account_account.data).unwrap();
        assert_eq!(account.amount, 1000 - 42 - 84);

        // insufficient funds approved via delegate
        assert_eq!(
            Err(TokenError::OwnerMismatch.into()),
            do_process_instruction(
                burn(&program_id, &account_key, &delegate_key, &[], 100).unwrap(),
                vec![&mut account_account, &mut delegate_account],
            )
        );
    }

    #[test]
    fn test_multisig() {
        let program_id = pubkey_rand();
        let mint_key = pubkey_rand();
        let mut mint_account = SolanaAccount::new(0, size_of::<Mint>(), &program_id);
        let account_key = pubkey_rand();
        let mut account = SolanaAccount::new(0, size_of::<Account>(), &program_id);
        let account2_key = pubkey_rand();
        let mut account2_account = SolanaAccount::new(0, size_of::<Account>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = SolanaAccount::default();
        let multisig_key = pubkey_rand();
        let mut multisig_account = SolanaAccount::new(0, size_of::<Multisig>(), &program_id);
        let multisig_delegate_key = pubkey_rand();
        let mut multisig_delegate_account =
            SolanaAccount::new(0, size_of::<Multisig>(), &program_id);
        let signer_keys = vec![pubkey_rand(); MAX_SIGNERS];
        let signer_key_refs: Vec<&Pubkey> = signer_keys.iter().map(|key| key).collect();
        let mut signer_accounts = vec![SolanaAccount::new(0, 0, &program_id); MAX_SIGNERS];

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

        // create account with multisig owner
        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, &multisig_key).unwrap(),
            vec![&mut account, &mut mint_account, &mut multisig_account],
        )
        .unwrap();

        // create another account with multisig owner
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

        // create new m int with multisig owner
        do_process_instruction(
            initialize_mint(
                &program_id,
                &mint_key,
                Some(&account_key),
                Some(&multisig_key),
                1000,
                2,
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
                &multisig_key,
                &[&signer_keys[0]],
                42,
            )
            .unwrap(),
            vec![
                &mut account,
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
                &multisig_delegate_key,
                &signer_key_refs,
                42,
            )
            .unwrap(),
            vec![
                &mut account,
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
        for signer_key in signer_keys.iter_mut().take(MAX_SIGNERS) {
            *signer_key = pubkey_rand();
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
        let mut multisig: &mut Multisig = state::unpack_unchecked(&mut data).unwrap();
        multisig.m = MAX_SIGNERS as u8;
        multisig.n = MAX_SIGNERS as u8;
        multisig.signers = signer_keys;
        multisig.is_initialized = true;
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
        Processor::validate_owner(&program_id, &owner_key, &owner_account_info, &signers).unwrap();

        // 1 of 11
        {
            let mut data_ref_mut = owner_account_info.data.borrow_mut();
            let mut multisig: &mut Multisig = state::unpack(&mut data_ref_mut).unwrap();
            multisig.m = 1;
        }
        Processor::validate_owner(&program_id, &owner_key, &owner_account_info, &signers).unwrap();

        // 2:1
        {
            let mut data_ref_mut = owner_account_info.data.borrow_mut();
            let mut multisig: &mut Multisig = state::unpack(&mut data_ref_mut).unwrap();
            multisig.m = 2;
            multisig.n = 1;
        }
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            Processor::validate_owner(&program_id, &owner_key, &owner_account_info, &signers)
        );

        // 0:11
        {
            let mut data_ref_mut = owner_account_info.data.borrow_mut();
            let mut multisig: &mut Multisig = state::unpack(&mut data_ref_mut).unwrap();
            multisig.m = 0;
            multisig.n = 11;
        }
        Processor::validate_owner(&program_id, &owner_key, &owner_account_info, &signers).unwrap();

        // 2:11 but 0 provided
        {
            let mut data_ref_mut = owner_account_info.data.borrow_mut();
            let mut multisig: &mut Multisig = state::unpack(&mut data_ref_mut).unwrap();
            multisig.m = 2;
            multisig.n = 11;
        }
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            Processor::validate_owner(&program_id, &owner_key, &owner_account_info, &[])
        );
        // 2:11 but 1 provided
        {
            let mut data_ref_mut = owner_account_info.data.borrow_mut();
            let mut multisig: &mut Multisig = state::unpack(&mut data_ref_mut).unwrap();
            multisig.m = 2;
            multisig.n = 11;
        }
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            Processor::validate_owner(&program_id, &owner_key, &owner_account_info, &signers[0..1])
        );

        // 2:11, 2 from middle provided
        {
            let mut data_ref_mut = owner_account_info.data.borrow_mut();
            let mut multisig: &mut Multisig = state::unpack(&mut data_ref_mut).unwrap();
            multisig.m = 2;
            multisig.n = 11;
        }
        Processor::validate_owner(&program_id, &owner_key, &owner_account_info, &signers[5..7])
            .unwrap();

        // 11:11, one is not a signer
        {
            let mut data_ref_mut = owner_account_info.data.borrow_mut();
            let mut multisig: &mut Multisig = state::unpack(&mut data_ref_mut).unwrap();
            multisig.m = 2;
            multisig.n = 11;
        }
        signers[5].is_signer = false;
        assert_eq!(
            Err(ProgramError::MissingRequiredSignature),
            Processor::validate_owner(&program_id, &owner_key, &owner_account_info, &signers)
        );
        signers[5].is_signer = true;
    }

    #[test]
    fn test_close_account() {
        let program_id = pubkey_rand();
        let mint_key = pubkey_rand();
        let mut mint_account = SolanaAccount::new(0, size_of::<Mint>(), &program_id);
        let account_key = pubkey_rand();
        let mut account_account = SolanaAccount::new(42, size_of::<Account>(), &program_id);
        let account2_key = pubkey_rand();
        let mut account2_account = SolanaAccount::new(2, size_of::<Account>(), &program_id);
        let account3_key = pubkey_rand();
        let mut account3_account = SolanaAccount::new(2, size_of::<Account>(), &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = SolanaAccount::default();
        let owner2_key = pubkey_rand();
        let mut owner2_account = SolanaAccount::default();

        // uninitialized
        assert_eq!(
            Err(TokenError::UninitializedState.into()),
            do_process_instruction(
                close_account(&program_id, &account_key, &account3_key, &owner2_key, &[]).unwrap(),
                vec![
                    &mut account_account,
                    &mut account3_account,
                    &mut owner2_account,
                ],
            )
        );

        // initialize non-native account
        do_process_instruction(
            initialize_account(&program_id, &account_key, &mint_key, &owner_key).unwrap(),
            vec![&mut account_account, &mut mint_account, &mut owner_account],
        )
        .unwrap();

        // initialize native account
        do_process_instruction(
            initialize_account(
                &program_id,
                &account2_key,
                &crate::native_mint::id(),
                &owner_key,
            )
            .unwrap(),
            vec![&mut account2_account, &mut mint_account, &mut owner_account],
        )
        .unwrap();
        let account: &mut Account = state::unpack(&mut account2_account.data).unwrap();
        assert!(account.is_native);
        assert_eq!(account.amount, 2);

        // close non-native account
        assert_eq!(
            Err(TokenError::NonNativeNotSupported.into()),
            do_process_instruction(
                close_account(&program_id, &account_key, &account3_key, &owner_key, &[]).unwrap(),
                vec![
                    &mut account_account,
                    &mut account3_account,
                    &mut owner_account,
                ],
            )
        );
        assert_eq!(account_account.lamports, 42);

        // close native account
        do_process_instruction(
            close_account(&program_id, &account2_key, &account3_key, &owner_key, &[]).unwrap(),
            vec![
                &mut account2_account,
                &mut account3_account,
                &mut owner_account,
            ],
        )
        .unwrap();
        let account: &mut Account = state::unpack_unchecked(&mut account2_account.data).unwrap();
        assert!(account.is_native);
        assert_eq!(account.amount, 0);
        assert_eq!(account3_account.lamports, 4);
    }

    #[test]
    fn test_native_token() {
        let program_id = pubkey_rand();
        let mut mint_account = SolanaAccount::new(0, size_of::<Mint>(), &program_id);
        let account_key = pubkey_rand();
        let mut account_account = SolanaAccount::new(42, size_of::<Account>(), &program_id);
        let account2_key = pubkey_rand();
        let mut account2_account = SolanaAccount::new(2, size_of::<Account>(), &program_id);
        let account3_key = pubkey_rand();
        let mut account3_account = SolanaAccount::new(2, 0, &program_id);
        let owner_key = pubkey_rand();
        let mut owner_account = SolanaAccount::default();

        // initialize native account
        do_process_instruction(
            initialize_account(
                &program_id,
                &account_key,
                &crate::native_mint::id(),
                &owner_key,
            )
            .unwrap(),
            vec![&mut account_account, &mut mint_account, &mut owner_account],
        )
        .unwrap();
        let account: &mut Account = state::unpack(&mut account_account.data).unwrap();
        assert!(account.is_native);
        assert_eq!(account.amount, 42);

        // initialize native account
        do_process_instruction(
            initialize_account(
                &program_id,
                &account2_key,
                &crate::native_mint::id(),
                &owner_key,
            )
            .unwrap(),
            vec![&mut account2_account, &mut mint_account, &mut owner_account],
        )
        .unwrap();
        let account: &mut Account = state::unpack(&mut account2_account.data).unwrap();
        assert!(account.is_native);
        assert_eq!(account.amount, 2);

        // mint_to unsupported
        assert_eq!(
            Err(TokenError::NativeNotSupported.into()),
            do_process_instruction(
                mint_to(
                    &program_id,
                    &crate::native_mint::id(),
                    &account_key,
                    &owner_key,
                    &[],
                    42
                )
                .unwrap(),
                vec![&mut mint_account, &mut account_account, &mut owner_account],
            )
        );

        // burn unsupported
        assert_eq!(
            Err(TokenError::NativeNotSupported.into()),
            do_process_instruction(
                burn(&program_id, &account_key, &owner_key, &[], 42).unwrap(),
                vec![&mut account_account, &mut owner_account],
            )
        );

        // initialize native account
        do_process_instruction(
            transfer(
                &program_id,
                &account_key,
                &account2_key,
                &owner_key,
                &[],
                40,
            )
            .unwrap(),
            vec![
                &mut account_account,
                &mut account2_account,
                &mut owner_account,
            ],
        )
        .unwrap();

        let account: &mut Account = state::unpack(&mut account_account.data).unwrap();
        assert!(account.is_native);
        assert_eq!(account_account.lamports, 2);
        assert_eq!(account.amount, 2);
        let account: &mut Account = state::unpack(&mut account2_account.data).unwrap();
        assert!(account.is_native);
        assert_eq!(account2_account.lamports, 42);
        assert_eq!(account.amount, 42);

        // close native account
        do_process_instruction(
            close_account(&program_id, &account_key, &account3_key, &owner_key, &[]).unwrap(),
            vec![
                &mut account_account,
                &mut account3_account,
                &mut owner_account,
            ],
        )
        .unwrap();
        let account: &mut Account = state::unpack_unchecked(&mut account_account.data).unwrap();
        assert!(account.is_native);
        assert_eq!(account_account.lamports, 0);
        assert_eq!(account.amount, 0);
        assert_eq!(account3_account.lamports, 4);
    }
}
