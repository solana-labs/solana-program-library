#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use serde_json::to_string;
use solana_program::program_error::ProgramError;

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

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
pub enum Protocol {
    Raydium,
    Saber,
    Orca,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct ProtocolInfo {
    pub protocol: Protocol,
    pub description: String,
    pub link: String,
    pub pools: u32,
    pub farms: u32,
    pub vaults: u32,
}

impl std::fmt::Display for ProgramIDType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            ProgramIDType::System => write!(f, "System"),
            ProgramIDType::ProgramsRef => write!(f, "ProgramsRef"),
            ProgramIDType::VaultsRef => write!(f, "VaultsRef"),
            ProgramIDType::Vault => write!(f, "Vault"),
            ProgramIDType::FarmsRef => write!(f, "FarmsRef"),
            ProgramIDType::Farm => write!(f, "Farm"),
            ProgramIDType::PoolsRef => write!(f, "PoolsRef"),
            ProgramIDType::Pool => write!(f, "Pool"),
            ProgramIDType::TokensRef => write!(f, "TokensRef"),
            ProgramIDType::Token => write!(f, "Token"),
            ProgramIDType::MainRouter => write!(f, "MainRouter"),
            ProgramIDType::Serum => write!(f, "Serum"),
            ProgramIDType::Raydium => write!(f, "Raydium"),
            ProgramIDType::Saber => write!(f, "Saber"),
            ProgramIDType::Orca => write!(f, "Orca"),
            ProgramIDType::FundsRef => write!(f, "FundsRef"),
            ProgramIDType::Fund => write!(f, "Fund"),
        }
    }
}

impl std::str::FromStr for ProgramIDType {
    type Err = ProgramError;

    fn from_str(s: &str) -> Result<Self, ProgramError> {
        match s.to_lowercase().as_str() {
            "system" => Ok(ProgramIDType::System),
            "programsref" => Ok(ProgramIDType::ProgramsRef),
            "vaultsref" => Ok(ProgramIDType::VaultsRef),
            "vault" => Ok(ProgramIDType::Vault),
            "farmsref" => Ok(ProgramIDType::FarmsRef),
            "farm" => Ok(ProgramIDType::Farm),
            "poolsref" => Ok(ProgramIDType::PoolsRef),
            "pool" => Ok(ProgramIDType::Pool),
            "tokensref" => Ok(ProgramIDType::TokensRef),
            "token" => Ok(ProgramIDType::Token),
            "mainrouter" => Ok(ProgramIDType::MainRouter),
            "serum" => Ok(ProgramIDType::Serum),
            "raydium" => Ok(ProgramIDType::Raydium),
            "saber" => Ok(ProgramIDType::Saber),
            "orca" => Ok(ProgramIDType::Orca),
            "fundsref" => Ok(ProgramIDType::FundsRef),
            "fund" => Ok(ProgramIDType::Fund),
            _ => Err(ProgramError::InvalidArgument),
        }
    }
}

impl Protocol {
    pub fn id(&self) -> &'static str {
        match *self {
            Protocol::Raydium => "RDM",
            Protocol::Saber => "SBR",
            Protocol::Orca => "ORC",
        }
    }
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Protocol::Raydium => write!(f, "Raydium"),
            Protocol::Saber => write!(f, "Saber"),
            Protocol::Orca => write!(f, "Orca"),
        }
    }
}

impl std::str::FromStr for Protocol {
    type Err = ProgramError;

    fn from_str(s: &str) -> Result<Self, ProgramError> {
        match s.to_lowercase().as_str() {
            "rdm" | "raydium" => Ok(Protocol::Raydium),
            "sbr" | "saber" => Ok(Protocol::Saber),
            "orc" | "orca" => Ok(Protocol::Orca),
            _ => Err(ProgramError::InvalidArgument),
        }
    }
}

impl std::fmt::Display for ProtocolInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", to_string(&self).unwrap())
    }
}
