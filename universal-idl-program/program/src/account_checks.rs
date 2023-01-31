use std::fmt::Display;

use lazy_format::lazy_format;
use solana_program::account_info::AccountInfo;
use solana_program::bpf_loader_upgradeable::{self, UpgradeableLoaderState};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;
use solana_program::{bpf_loader, bpf_loader_deprecated, msg};

#[inline(always)]
pub fn assert_with_msg(v: bool, err: impl Into<ProgramError>, msg: impl Display) -> ProgramResult {
    if v {
        Ok(())
    } else {
        let caller = std::panic::Location::caller();
        msg!("{} {}", msg, caller);
        Err(err.into())
    }
}

#[inline(always)]
pub fn assert_mut(account: &AccountInfo, name: &str) -> ProgramResult {
    assert_with_msg(
        account.is_writable,
        ProgramError::InvalidInstructionData,
        lazy_format!("{} must be mutable", name),
    )
}

#[inline(always)]
pub fn assert_executable(account: &AccountInfo, name: &str) -> ProgramResult {
    assert_with_msg(
        account.executable,
        ProgramError::InvalidAccountData,
        lazy_format!("{} must be executable", name),
    )
}

#[inline(always)]
pub fn assert_signer(account: &AccountInfo, name: &str) -> ProgramResult {
    assert_with_msg(
        account.is_signer,
        ProgramError::InvalidInstructionData,
        lazy_format!("{} must be signer", name),
    )
}

#[inline(always)]
pub fn assert_owner(account: &AccountInfo, owner: &Pubkey, name: &str) -> ProgramResult {
    assert_with_msg(
        account.owner == owner,
        ProgramError::IllegalOwner,
        lazy_format!("{} must be owned by {}", name, owner),
    )
}

#[inline(always)]
pub fn assert_empty(account: &AccountInfo, name: &str) -> ProgramResult {
    assert_with_msg(
        account.data_is_empty(),
        ProgramError::InvalidInstructionData,
        lazy_format!("{} must be empty", name),
    )
}

#[inline(always)]
pub fn assert_address(address_one: &Pubkey, address_two: &Pubkey, name: &str) -> ProgramResult {
    assert_with_msg(
        address_one == address_two,
        ProgramError::InvalidInstructionData,
        lazy_format!("{} must equal {}", name, address_two),
    )
}

#[inline(always)]
pub fn assert_program_account(account: &AccountInfo, discriminator: [u8; 8]) -> ProgramResult {
    let data = &account.data.borrow_mut();
    assert_with_msg(
        is_correct_account_type(data, discriminator) && *account.owner == crate::id(),
        ProgramError::InvalidInstructionData,
        lazy_format!("Invalid account type for {}", account.key),
    )
}

pub fn assert_account_owned_by_loader(account: &AccountInfo, name: &str) -> ProgramResult {
    assert_owner(account, &bpf_loader_upgradeable::id(), name)
}

pub fn assert_system_program(account: &AccountInfo) -> ProgramResult {
    assert_address(account.key, &system_program::id(), "system_program")
}

pub fn assert_program_data_matches_program(
    program: &UpgradeableLoaderState,
    program_data_key: &Pubkey,
) -> ProgramResult {
    if let UpgradeableLoaderState::Program {
        programdata_address,
    } = program
    {
        assert_with_msg(
            program_data_key == programdata_address,
            ProgramError::InvalidInstructionData,
            lazy_format!("Invalid program data for {}", program_data_key),
        )?;
        Ok(())
    } else {
        Err(ProgramError::InvalidInstructionData)
    }
}

pub fn assert_program_authority_matches_program(
    program_data: &UpgradeableLoaderState,
    program_authority_key: &Pubkey,
) -> ProgramResult {
    if let UpgradeableLoaderState::ProgramData {
        upgrade_authority_address,
        ..
    } = program_data
    {
        assert_with_msg(
            upgrade_authority_address.is_some(),
            ProgramError::InvalidInstructionData,
            lazy_format!("The given program is frozen, and has no authority"),
        )?;

        assert_with_msg(
            upgrade_authority_address.unwrap() == *program_authority_key,
            ProgramError::InvalidInstructionData,
            lazy_format!("Invalid program authority: {}", program_authority_key),
        )?;
        Ok(())
    } else {
        Err(ProgramError::InvalidInstructionData)
    }
}

pub fn is_correct_account_type(data: &[u8], discriminator: [u8; 8]) -> bool {
    data[..8] == discriminator
}

pub fn assert_program_authority_in_allowlist(account: &AccountInfo, name: &str) -> ProgramResult {
    let allowlist_frozen_authorities: Vec<&str> =
        ["6tgR1upn2bsMdiprpfUAWmoniEJ8E1XVF8f9gLmWqyTS"].to_vec();

    let auth_str: &str = &account.key.to_string();
    assert_with_msg(
        allowlist_frozen_authorities.contains(&auth_str),
        ProgramError::InvalidInstructionData,
        lazy_format!("{} must be in allowlist", name),
    )
}

pub fn is_frozen_program(account: &AccountInfo) -> bool {
    *account.owner == bpf_loader::id() || *account.owner == bpf_loader_deprecated::id()
}

pub fn is_upgradeable_program(account: &AccountInfo) -> bool {
    *account.owner == bpf_loader_upgradeable::id()
}

pub fn assert_owned_by_frozen_loader(account: &AccountInfo, name: &str) -> ProgramResult {
    assert_with_msg(
        is_frozen_program(account),
        ProgramError::InvalidAccountData,
        lazy_format!("{} must be owned by a frozen loader", name),
    )
}
