#![deny(missing_docs)]

//! A simple program that accepts a string of encoded characters and verifies that it parses. Currently handles UTF-8.

pub mod processor;

/// The spl-memo program's on-chain program id
pub const PROGRAM_ID: &str = "Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo";
