pub mod expr;
pub mod instruction;
pub mod processor;
pub mod state;

use crate::processor::process_instruction;

solana_sdk::declare_program!(
    "Budget1111111111111111111111111111111111111",
    solana_budget_program,
    process_instruction
);
