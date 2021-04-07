---
title: Token Fractionalization Program
---

## Background

Solana's programming model and the definitions of the Solana terms used in this
document are available at:

- https://docs.solana.com/apps
- https://docs.solana.com/terminology

## Source

The Fraction Program's source is available on
[github](https://github.com/solana-labs/solana-program-library)

There is also an example Rust client located at
[github](https://github.com/solana-labs/solana-program-library/tree/master/token_metadata/test/src/main.rs)
that can be perused for learning and run if desired with `cargo run --bin spl-token-metadata-test-client`. It allows testing out a variety of scenarios.

## Interface

The on-chain Token Fraction program is written in Rust and available on crates.io as
[spl-metadata](https://crates.io/crates/spl-token-metadata) and
[docs.rs](https://docs.rs/spl-token-metadata).

The crate provides three instructions, `create_metadata_accounts()`, `update_metadata_accounts()` and `transfer_update_authority()`to easily create instructions for the program.

## Operational overview
