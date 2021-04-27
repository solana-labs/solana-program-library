#![cfg(feature = "test-bpf")]

use {
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        sysvar::{self},
    },
    solana_program_test::*,
    solana_sdk::{signature::Signer, transaction::Transaction},
    std::str::FromStr,
};

use spl_perpetual::{error::*, instruction::*, processor::*, state::*};

#[tokio::test]
async fn test_deposit() {
    let mut test = ProgramTest::new(
        "spl_perpetual",
        spl_perpetual::id(),
        processor!(Processor::process),
    );
    // TODO Testing is quite tough...
}

#[tokio::test]
async fn test_withdraw() {
    let mut test = ProgramTest::new(
        "spl_perpetual",
        spl_perpetual::id(),
        processor!(Processor::process),
    );
    // TODO Testing is quite tough...
}
