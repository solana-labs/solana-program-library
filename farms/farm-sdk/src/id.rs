//! Official accounts and program ids

use serde::{Deserialize, Serialize};

pub mod main_router {
    solana_program::declare_id!("RepLaceThisWithVaLidMainRouterProgramPubkey");
}

pub mod main_router_admin {
    solana_program::declare_id!("RepLaceThisWithCorrectMainRouterAdminPubkey");
}

pub mod zero {
    // ID that represents the unset Pubkey. This is to avoid passing Pubkey::default() which
    // is equal to system_program::id().
    // [14, 196, 109, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    solana_program::declare_id!("zeRosMEYuuABXv5y2LNUbgmPp62yFD5CULW5soHS9HR");
}

pub const DAO_TOKEN_NAME: &str = "FARM_DAO";
pub const DAO_PROGRAM_NAME: &str = "FarmGovernance";
pub const DAO_MINT_NAME: &str = "FarmMintGovernance";
pub const DAO_CUSTODY_NAME: &str = "FarmCustodyGovernance";

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProgramIDType {
    System,
    ProgramsRef,
    VaultsRef,
    Vault,
    FarmsRef,
    Farm,
    PoolsRef,
    Pool,
    TokensRef,
    Token,
    MainRouter,
    Serum,
    Raydium,
    Saber,
    Orca,
}
