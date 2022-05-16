#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

pub mod error;
pub mod farm;
pub mod fund;
pub mod id;
pub mod instruction;
pub mod log;
pub mod math;
pub mod pack;
pub mod pool;
pub mod program;
pub mod refdb;
pub mod string;
pub mod token;
pub mod traits;
pub mod vault;

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
    FundsRef,
    Fund,
}
