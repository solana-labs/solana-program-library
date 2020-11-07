//! Program state processor

//#![cfg(feature = "program")]

use crate::{
    error::IdentityError,
    instruction::{IdentityInstruction, MAX_SIGNERS},
    state::{IdentityAccount, AccountState, Multisig, Attestation, SerializablePubkey},
};
use num_traits::FromPrimitive;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    decode_error::DecodeError,
    entrypoint::ProgramResult,
    info,
    program_error::{PrintProgramError, ProgramError},
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
};

/// Program state handler.
pub struct Processor {}
impl Processor {

    /// Processes an [InitializeIdentity](enum.IdentityInstruction.html) instruction.
    pub fn process_initialize_identity(accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let new_subject_info = next_account_info(account_info_iter)?;
        let owner_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(next_account_info(account_info_iter)?)?;

        info!("Instruction: InitializeIdentity, extracted inputs");

        let new_account_info_data_len = new_subject_info.data_len();
        info!(new_subject_info.data_len(), 0, 0, 0, 0);
        let mut subject = IdentityAccount::deserialize2(&new_subject_info.data.borrow())?;

        info!("Instruction: InitializeIdentity, deserialized subject");

        if subject.is_initialized() {
            return Err(IdentityError::AlreadyInUse.into());
        }

        if !rent.is_exempt(new_subject_info.lamports(), new_account_info_data_len) {
            return Err(IdentityError::NotRentExempt.into());
        }

        info!("Instruction: InitializeIdentity, checks complete");

        subject.owner = SerializablePubkey::from(*owner_info.key);
        subject.state = AccountState::Initialized;
        subject.serialize(&mut new_subject_info.data.borrow_mut())?;

        Ok(())
    }

    /// Processes an [Attest](enum.IdentityInstruction.html) instruction.
    pub fn process_attest(accounts: &[AccountInfo], attestation_data: &[u8; 32]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let subject_info = next_account_info(account_info_iter)?;
        let idv_info = next_account_info(account_info_iter)?;

        let mut subject = IdentityAccount::deserialize2(&subject_info.data.borrow())?;

        info!("Instruction: Attest, extracted inputs");

        let serializable_idv_pubkey = SerializablePubkey::from(*idv_info.key);
        let new_attestation = Attestation { idv: serializable_idv_pubkey, attestation_data: *attestation_data };

        // TODO replace this with attestations.push() when more than one attestation is allowed
        subject.num_attestations = 1;
        subject.attestation = new_attestation;

        subject.serialize(&mut subject_info.data.borrow_mut())?;

        Ok(())
    }


    /// Processes an [Instruction](enum.Instruction.html).
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        info!("process");
        let instruction = IdentityInstruction::deserialize(input)?;

        match instruction {
            IdentityInstruction::InitializeIdentity => {
                info!("Instruction: InitializeIdentity");
                Self::process_initialize_identity(accounts)
            }
            IdentityInstruction::Attest { attestation_data } => {
                info!("Instruction: Attest");
                Self::process_attest(accounts, &attestation_data)
            }
        }
    }

    /// Verifies than Identity belongs to the correct owner
    /// and is signed by the expected IdV
    pub fn verify(
        identity: &IdentityAccount,
        expected_owner: &Pubkey,
        expected_idv: &Pubkey,
    ) -> Result<(), IdentityError> {
        if *expected_owner != identity.owner.to_pubkey() {
            return Err(IdentityError::OwnerMismatch.into());
        }
        if identity.num_attestations < 1 {
            info!("No attestations for identity");
            return Err(IdentityError::UnauthorizedIdentity.into());
        }

        if *expected_idv != identity.attestation.idv.to_pubkey() {
            info!("Identity not attested by correct IDV");
            return Err(IdentityError::UnauthorizedIdentity.into());
        }

        Ok(())
    }


    /// Validates owner(s) are present
    pub fn validate_owner(
        program_id: &Pubkey,
        expected_owner: &Pubkey,
        owner_account_info: &AccountInfo,
        signers: &[AccountInfo],
    ) -> ProgramResult {
        if expected_owner != owner_account_info.key {
            return Err(IdentityError::OwnerMismatch.into());
        }
        if program_id == owner_account_info.owner
            && owner_account_info.data_len() == Multisig::get_packed_len()
        {
            let multisig = Multisig::unpack(&owner_account_info.data.borrow())?;
            let mut num_signers = 0;
            let mut matched = [false; MAX_SIGNERS];
            for signer in signers.iter() {
                for (position, key) in multisig.signers[0..multisig.n as usize].iter().enumerate() {
                    if key == signer.key && !matched[position] {
                        if !signer.is_signer {
                            return Err(ProgramError::MissingRequiredSignature);
                        }
                        matched[position] = true;
                        num_signers += 1;
                    }
                }
            }
            if num_signers < multisig.m {
                return Err(ProgramError::MissingRequiredSignature);
            }
            return Ok(());
        } else if !owner_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        Ok(())
    }
}

impl PrintProgramError for IdentityError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            IdentityError::NotRentExempt => {
                info!("Error: Lamport balance below rent-exempt threshold")
            }
            IdentityError::InsufficientFunds => info!("Error: insufficient funds"),
            IdentityError::OwnerMismatch => info!("Error: owner does not match"),
            IdentityError::AlreadyInUse => info!("Error: account already in use"),
            IdentityError::InvalidInstruction => info!("Error: Invalid instruction"),
            IdentityError::UnauthorizedIdentity => info!("Error: Unauthorized identity"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::*;
    use solana_program::{
        account::Account as SolanaAccount, account_info::create_is_signer_account_infos,
        clock::Epoch, instruction::Instruction, sysvar::rent,
    };

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

    fn do_process_instruction_dups(
        instruction: Instruction,
        account_infos: Vec<AccountInfo>,
    ) -> ProgramResult {
        Processor::process(&instruction.program_id, &account_infos, &instruction.data)
    }

    fn return_identity_error_as_program_error() -> ProgramError {
        IdentityError::MintMismatch.into()
    }

    fn rent_sysvar() -> SolanaAccount {
        rent::create_account(42, &Rent::default())
    }

    fn account_minimum_balance() -> u64 {
        Rent::default().minimum_balance(Account::get_packed_len())
    }

    fn multisig_minimum_balance() -> u64 {
        Rent::default().minimum_balance(Multisig::get_packed_len())
    }

    #[test]
    fn test_print_error() {
        let error = return_identity_error_as_program_error();
        error.print::<IdentityError>();
    }

    #[test]
    #[should_panic(expected = "Custom(3)")]
    fn test_error_unwrap() {
        Err::<(), ProgramError>(return_identity_error_as_program_error()).unwrap();
    }

    #[test]
    fn test_unique_account_sizes() {
        assert_ne!(Mint::get_packed_len(), 0);
        assert_ne!(Mint::get_packed_len(), Account::get_packed_len());
        assert_ne!(Mint::get_packed_len(), Multisig::get_packed_len());
        assert_ne!(Account::get_packed_len(), 0);
        assert_ne!(Account::get_packed_len(), Multisig::get_packed_len());
        assert_ne!(Multisig::get_packed_len(), 0);
    }

    #[test]
    fn test_pack_unpack() {
        // Mint
        let check = Mint {
            mint_authority: COption::Some(Pubkey::new(&[1; 32])),
            supply: 42,
            decimals: 7,
            is_initialized: true,
            freeze_authority: COption::Some(Pubkey::new(&[2; 32])),
        };
        let mut packed = vec![0; Mint::get_packed_len() + 1];
        assert_eq!(
            Err(ProgramError::InvalidAccountData),
            Mint::pack(check, &mut packed)
        );
        let mut packed = vec![0; Mint::get_packed_len() - 1];
        assert_eq!(
            Err(ProgramError::InvalidAccountData),
            Mint::pack(check, &mut packed)
        );
        let mut packed = vec![0; Mint::get_packed_len()];
        Mint::pack(check, &mut packed).unwrap();
        let expect = vec![
            1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 42, 0, 0, 0, 0, 0, 0, 0, 7, 1, 1, 0, 0, 0, 2, 2, 2, 2, 2, 2, 2, 2,
            2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
        ];
        assert_eq!(packed, expect);
        let unpacked = Mint::unpack(&packed).unwrap();
        assert_eq!(unpacked, check);

        // Account
        let check = Account {
            mint: Pubkey::new(&[1; 32]),
            owner: Pubkey::new(&[2; 32]),
            amount: 3,
            delegate: COption::Some(Pubkey::new(&[4; 32])),
            state: AccountState::Frozen,
            is_native: COption::Some(5),
            delegated_amount: 6,
            close_authority: COption::Some(Pubkey::new(&[7; 32])),
        };
        let mut packed = vec![0; Account::get_packed_len() + 1];
        assert_eq!(
            Err(ProgramError::InvalidAccountData),
            Account::pack(check, &mut packed)
        );
        let mut packed = vec![0; Account::get_packed_len() - 1];
        assert_eq!(
            Err(ProgramError::InvalidAccountData),
            Account::pack(check, &mut packed)
        );
        let mut packed = vec![0; Account::get_packed_len()];
        Account::pack(check, &mut packed).unwrap();
        let expect = vec![
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
            2, 2, 2, 2, 2, 2, 3, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
            4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 2, 1, 0, 0, 0, 5, 0, 0,
            0, 0, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
            7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
        ];
        assert_eq!(packed, expect);
        let unpacked = Account::unpack(&packed).unwrap();
        assert_eq!(unpacked, check);

        // Multisig
        let check = Multisig {
            m: 1,
            n: 2,
            is_initialized: true,
            signers: [Pubkey::new(&[3; 32]); MAX_SIGNERS],
        };
        let mut packed = vec![0; Multisig::get_packed_len() + 1];
        assert_eq!(
            Err(ProgramError::InvalidAccountData),
            Multisig::pack(check, &mut packed)
        );
        let mut packed = vec![0; Multisig::get_packed_len() - 1];
        assert_eq!(
            Err(ProgramError::InvalidAccountData),
            Multisig::pack(check, &mut packed)
        );
        let mut packed = vec![0; Multisig::get_packed_len()];
        Multisig::pack(check, &mut packed).unwrap();
        let expect = vec![
            1, 2, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, 3, 3, 3, 3,
        ];
        assert_eq!(packed, expect);
        let unpacked = Multisig::unpack(&packed).unwrap();
        assert_eq!(unpacked, check);
    }
}
