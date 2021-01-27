---
title: Memo Program
---

The Memo program is a simple program that validates a string of UTF-8 encoded
characters and verifies that any accounts provided are signers of the
transaction. The program also logs the memo, as well as any verified signer
addresses, to the transaction log, so that anyone can easily observe memos and
know they were approved by zero or more addresses by inspecting the transaction
log from a trusted provider.

## Background

Solana's programming model and the definitions of the Solana terms used in this
document are available at:

- https://docs.solana.com/apps
- https://docs.solana.com/terminology

## Source

The Memo Program's source is available on
[github](https://github.com/solana-labs/solana-program-library)

## Interface

The on-chain Memo Program is written in Rust and available on crates.io as
[spl-memo](https://crates.io/crates/spl-memo) and
[docs.rs](https://docs.rs/spl-memo).

The crate provides a `build_memo()` method to easily create a properly
constructed Instruction.

## Operational Notes

If zero accounts are provided to the signed-memo instruction, the program
succeeds when the memo is valid UTF-8, and logs the memo to the transaction log.

If one or more accounts are provided to the signed-memo instruction, all must be
valid signers of the transaction for the instruction to succeed.
