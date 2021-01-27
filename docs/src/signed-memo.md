---
title: Signed Memo Program
---

The Signed Memo program is an expanded version of the original [memo](memo.md)
program. In addition to validating a string of UTF-8 encoded characters, the
program also verifies that any accounts provided are signers of the transaction.
An additional enhancement: the program logs the memo, as well as any signer
addresses and verification status, to the transaction log, so that anyone can
easily observe memos and know they were approved by zero or more addresses
by inspecting the transaction log from a trusted provider.

## Background

Solana's programming model and the definitions of the Solana terms used in this
document are available at:

- https://docs.solana.com/apps
- https://docs.solana.com/terminology

## Source

The Signed Memo Program's source is available on
[github](https://github.com/solana-labs/solana-program-library)

## Interface

The on-chain Signed Memo Program is written in Rust and available on crates.io as
[spl-signed-memo](https://crates.io/crates/spl-signed-memo) and
[docs.rs](https://docs.rs/spl-signed-memo).

The crate provides a `signed_memo()` method to easily create a properly
constructed Instruction.

## Operational Notes

If zero accounts are provided to the signed-memo instruction, the program
behaves like the original [memo](memo.md) program, succeeding when the memo is
valid UTF-8. It has the additional feature of logging the memo to the
transaction log.

If one or more accounts are provided to the signed-memo instruction, all must be
valid signers of the transaction for the instruction to succeed. To aid in
debugging, all account addresses and their signer status are logged to the
transaction log on failure as well as on success.
