use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::ErrorKind;

use borsh::maybestd::io::Error as BorshError;
use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankAccount;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, hash::hash, msg,
    program_error::ProgramError, pubkey::Pubkey,
};

use crate::account_checks::{assert_owner, assert_with_msg, is_correct_account_type};
use crate::error::ErrorCode;

/// Account Type
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum AccountType {
    Idl = 0,
    FrozenProgramAuthority = 1,
    Unrecognized = 7,
}

impl From<u8> for AccountType {
    fn from(orig: u8) -> Self {
        match orig {
            0 => AccountType::Idl,
            _ => AccountType::Unrecognized,
        }
    }
}

impl Display for AccountType {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            AccountType::Idl => write!(f, "Idl"),
            AccountType::FrozenProgramAuthority => write!(f, "FrozenProgramAuthority"),
            AccountType::Unrecognized => write!(f, "Unrecognized"),
        }
    }
}

/// Solana Account
pub trait SolanaAccount {
    fn account_type() -> AccountType;
    fn save(&self, account: &AccountInfo) -> ProgramResult;
    fn new() -> Self;
    fn hash() -> [u8; 8];

    fn safe_deserialize<T: BorshDeserialize>(mut data: &[u8]) -> Result<T, BorshError> {
        if !is_correct_account_type(data, Self::hash()) {
            return Err(BorshError::new(ErrorKind::Other, "InvalidAccountType"));
        }

        let result: Result<T, std::io::Error> = T::deserialize(&mut data);
        if result.is_err() {
            return Err(BorshError::new(ErrorKind::Other, "FailToDeserialize"));
        }

        Ok(result.unwrap())
    }

    fn from_account_info<T: BorshDeserialize>(account: &AccountInfo) -> Result<T, ProgramError> {
        // check that account belongs in the program`
        assert_owner(account, &crate::id(), "account")?;

        let account: T = Self::safe_deserialize(&account.data.borrow_mut())
            .map_err(|_| ErrorCode::DataTypeMismatch)?;

        Ok(account)
    }
}

/// IDL seeds
pub const IDL_SEED: &str = "idl";
#[inline]
pub fn idl_seeds(program_key: &Pubkey) -> (Pubkey, Vec<Vec<u8>>) {
    let mut seeds = vec![IDL_SEED.as_bytes().to_vec(), program_key.as_ref().to_vec()];
    let (key, bump) = Pubkey::find_program_address(
        &seeds.iter().map(|s| s.as_slice()).collect::<Vec<&[u8]>>(),
        &crate::id(),
    );
    seeds.push(vec![bump]);
    (key, seeds)
}

#[inline]
pub fn assert_idl_seeds(
    program_key: &Pubkey,
    expected_key: &Pubkey,
) -> Result<Vec<Vec<u8>>, ProgramError> {
    let (key, seeds) = idl_seeds(program_key);
    assert_with_msg(
        expected_key == &key,
        ProgramError::InvalidInstructionData,
        "Invalid IDL seeds".to_string(),
    )?;
    Ok(seeds)
}

/// Buffer seeds
// There can be at most 1 other buffer that exists so developers can write a fully functional
// IDL to it, before publishing it to the IDL account.
pub const BUFFER_SEED: &str = "buffer";
#[inline]
pub fn buffer_seeds(program_key: &Pubkey) -> (Pubkey, Vec<Vec<u8>>) {
    let mut seeds = vec![
        BUFFER_SEED.as_bytes().to_vec(),
        program_key.as_ref().to_vec(),
    ];
    let (key, bump) = Pubkey::find_program_address(
        &seeds.iter().map(|s| s.as_slice()).collect::<Vec<&[u8]>>(),
        &crate::id(),
    );
    seeds.push(vec![bump]);
    (key, seeds)
}

#[inline]
pub fn assert_buffer_seeds(
    program_key: &Pubkey,
    expected_key: &Pubkey,
) -> Result<Vec<Vec<u8>>, ProgramError> {
    let (key, seeds) = buffer_seeds(program_key);
    assert_with_msg(
        expected_key == &key,
        ProgramError::InvalidInstructionData,
        "Invalid buffer seeds".to_string(),
    )?;
    Ok(seeds)
}

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, ShankAccount)]
pub struct Idl {
    pub account_type: [u8; 8],
    // Address that can modify the IDL
    pub authority: Pubkey,
    // Slot of the last time this was updated
    pub slot: u64,
    // Compressed idl bytes
    pub data: Vec<u8>,
}

impl Idl {
    pub fn init_deserialize(account: &AccountInfo) -> Result<Self, ProgramError> {
        let buf = &account.data.borrow();
        let authority = Pubkey::try_from_slice(buf)?;
        let slot = u64::try_from_slice(buf)?;
        let vec_len = u32::try_from_slice(buf)?;
        if vec_len != 0 {
            msg!("You cannot init-deserialize an initialized IDL");
            Err(ProgramError::InvalidAccountData)
        } else {
            Ok(Self {
                account_type: Self::hash(),
                authority,
                slot,
                data: vec![],
            })
        }
    }
}

impl SolanaAccount for Idl {
    fn hash() -> [u8; 8] {
        let discriminator_preimage = format!("account:{}", "idl");
        let mut discriminator = [0u8; 8];
        discriminator.copy_from_slice(&hash(discriminator_preimage.as_bytes()).to_bytes()[..8]);
        discriminator
    }

    fn new() -> Self {
        Self {
            account_type: Self::hash(),
            authority: Pubkey::default(),
            slot: 0,
            data: vec![],
        }
    }

    fn account_type() -> AccountType {
        AccountType::Idl
    }

    fn save(&self, account: &AccountInfo) -> ProgramResult {
        BorshSerialize::serialize(&self, &mut *account.data.borrow_mut())?;
        Ok(())
    }
}

/// Frozen program authority seeds
pub const FROZEN_AUTHORITY_SEEDS: &str = "frozen_auth";
pub fn frozen_authority_seeds(program_key: &Pubkey) -> (Pubkey, Vec<Vec<u8>>) {
    let mut seeds = vec![
        FROZEN_AUTHORITY_SEEDS.as_bytes().to_vec(),
        program_key.as_ref().to_vec(),
    ];
    let (key, bump) = Pubkey::find_program_address(
        &seeds.iter().map(|s| s.as_slice()).collect::<Vec<&[u8]>>(),
        &crate::id(),
    );
    seeds.push(vec![bump]);
    (key, seeds)
}

pub fn assert_frozen_authority_seeds(
    program_id: &Pubkey,
    expected_key: &Pubkey,
) -> Result<(Pubkey, Vec<Vec<u8>>), ProgramError> {
    let (key, seeds) = frozen_authority_seeds(program_id);
    assert_with_msg(
        expected_key == &key,
        ProgramError::InvalidInstructionData,
        "Invalid frozen program authority seeds".to_string(),
    )?;
    Ok((key, seeds))
}

#[derive(Debug, PartialEq, Clone, BorshDeserialize, BorshSerialize)]
pub struct FrozenProgramAuthority {
    pub account_type: [u8; 8],
    pub authority: Pubkey,
    pub meta_authority: Pubkey,
    pub slot: u64,
}

impl SolanaAccount for FrozenProgramAuthority {
    fn hash() -> [u8; 8] {
        let discriminator_preimage = format!("account:{}", "frozen_program_authority");
        let mut discriminator = [0u8; 8];
        discriminator.copy_from_slice(&hash(discriminator_preimage.as_bytes()).to_bytes()[..8]);
        discriminator
    }

    fn new() -> Self {
        Self {
            account_type: Self::hash(),
            authority: Pubkey::default(),
            meta_authority: Pubkey::default(),
            slot: 0,
        }
    }

    fn account_type() -> AccountType {
        AccountType::FrozenProgramAuthority
    }

    fn save(&self, account: &AccountInfo) -> ProgramResult {
        BorshSerialize::serialize(&self, &mut *account.data.borrow_mut())?;
        Ok(())
    }
}
