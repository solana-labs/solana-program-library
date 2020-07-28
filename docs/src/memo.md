---
title: Memo Program
---

A simple program that validates a string of UTF-8 encoded characters.  It can be
used to record a string on-chain, stored in the instruction data of a successful
transaction.

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
[spl-memo](https://crates.io/crates/spl-memo).

## Operational overview

The Memo program attempts to UTF-8 decode the instruction data; if successfully
decoded, the instruction is successful.