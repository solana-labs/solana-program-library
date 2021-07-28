// Mark this test as BPF-only due to current `ProgramTest` limitations when CPIing into the system program
#![cfg(feature = "test-bpf")]

use {
    solana_program::{
        feature::{self, Feature},
        program_option::COption,
        pubkey::Pubkey,
        system_program,
    },
    solana_program_test::*,
    solana_sdk::{
        signature::{Keypair, Signer},
        transaction::Transaction,
    },
    spl_token_swap::{instruction::*, state::*, processor::*, *},
};

fn program_test() -> ProgramTest {
    ProgramTest::new(
        "spl_token_swap",
        id(),
        processor!(Processor::process),
    )
}

#[tokio::test]
async fn test_basic() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
}
