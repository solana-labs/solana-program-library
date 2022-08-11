//! Vaults traits and common features.

use {
    crate::vault_info::VaultInfo,
    solana_farm_sdk::vault::Vault,
    solana_program::{account_info::AccountInfo, entrypoint::ProgramResult},
};

pub trait VaultParams {
    fn default_min_crank_interval() -> u64;
    fn default_fee() -> f64;
    fn default_external_fee() -> f64;
}

pub trait UserInit {
    fn user_init(vault: &Vault, accounts: &[AccountInfo]) -> ProgramResult;
}

pub trait AddLiquidity {
    fn add_liquidity(
        vault: &Vault,
        accounts: &[AccountInfo],
        max_token_a_amount: u64,
        max_token_b_amount: u64,
    ) -> ProgramResult;
}

pub trait LockLiquidity {
    fn lock_liquidity(vault: &Vault, accounts: &[AccountInfo], amount: u64) -> ProgramResult;
}

pub trait UnlockLiquidity {
    fn unlock_liquidity(vault: &Vault, accounts: &[AccountInfo], amount: u64) -> ProgramResult;
}

pub trait RemoveLiquidity {
    fn remove_liquidity(vault: &Vault, accounts: &[AccountInfo], amount: u64) -> ProgramResult;
}

pub trait Features {
    fn set_min_crank_interval(
        vault: &Vault,
        vault_info: &mut VaultInfo,
        accounts: &[AccountInfo],
        min_crank_interval_sec: u64,
    ) -> ProgramResult;

    fn set_fee(
        vault: &Vault,
        vault_info: &mut VaultInfo,
        accounts: &[AccountInfo],
        fee: f64,
    ) -> ProgramResult;

    fn set_external_fee(
        vault: &Vault,
        vault_info: &mut VaultInfo,
        accounts: &[AccountInfo],
        external_fee: f64,
    ) -> ProgramResult;

    fn enable_deposits(
        vault: &Vault,
        vault_info: &mut VaultInfo,
        accounts: &[AccountInfo],
    ) -> ProgramResult;

    fn disable_deposits(
        vault: &Vault,
        vault_info: &mut VaultInfo,
        accounts: &[AccountInfo],
    ) -> ProgramResult;

    fn enable_withdrawals(
        vault: &Vault,
        vault_info: &mut VaultInfo,
        accounts: &[AccountInfo],
    ) -> ProgramResult;

    fn disable_withdrawals(
        vault: &Vault,
        vault_info: &mut VaultInfo,
        accounts: &[AccountInfo],
    ) -> ProgramResult;
}

pub trait Crank {
    fn crank(vault: &Vault, accounts: &[AccountInfo], step: u64) -> ProgramResult;
}

pub trait Init {
    fn init(vault: &Vault, accounts: &[AccountInfo], step: u64) -> ProgramResult;
}

pub trait Shutdown {
    fn shutdown(vault: &Vault, accounts: &[AccountInfo]) -> ProgramResult;
}

pub trait WithdrawFees {
    fn withdraw_fees(vault: &Vault, accounts: &[AccountInfo], amount: u64) -> ProgramResult;
}

pub trait SetAdminSigners {
    fn set_admin_signers(
        vault: &Vault,
        accounts: &[AccountInfo],
        min_signatures: u8,
    ) -> ProgramResult;
}

pub trait RemoveMultisig {
    fn remove_multisig(vault: &Vault, accounts: &[AccountInfo]) -> ProgramResult;
}
