---
title: Token Metadata Program
---

## Background

Solana's programming model and the definitions of the Solana terms used in this
document are available at:

- https://docs.solana.com/apps
- https://docs.solana.com/terminology

## Source

The Metadata Program's source is available on
[github](https://github.com/solana-labs/solana-program-library)

There is also an example Rust client located at 
[github](https://github.com/solana-labs/solana-program-library/tree/master/metadata/test/src/main.rs)
that can be perused learning and run if desired with `cargo run --bin spl-metadata-test-client`.

## Interface

The on-chain Token Metadata program is written in Rust and available on crates.io as
[spl-metadata](https://crates.io/crates/spl-token-metadata) and
[docs.rs](https://docs.rs/spl-metadata).

The crate provides two instructions, `create_metadata_accounts()`, and `update_metadata_accounts()` to easily create instructions for the program

## Operational overview

This is a very simple program designed to allow unique metadata tagging to a given mint, with a unique owner
that can change that metadata going forward. The app is composed of 2 actions, one which will create the accounts, and one which will update some of the fields on them.

### Permissioning and Architecture

Only the authority on a mint can create the unique metadata accounts. The two metadata accounts created are the Metadata account, which holds the Name, Symbol, and URI and the Owner account, which holds a key to the Owner of
the metadata. The authority on a mint is responsible for calling create_metadata_accounts to create
allocated accounts on the chain.

To ensure the uniqueness of a mint's metadata, the address of a Metadata account is a PDA composed of seeds:

```rust
["metadata".as_bytes(), program_id.as_ref(), mint_key.as_ref()]
```

While the Owner address is a PDA composed of seeds:

```rust
["metadata".as_bytes(), program_id.as_ref(), name_as_bytes, symbol_as_bytes]
```

This ensures easy lookups by those interested - they can simply look up the metadata account by mint id, then
look up owner with the name and symbol if they need it.

The owner can only call the update_metadata_accounts command, which right now can only update the URI.

Due to the nature of the addresses on these accounts, name and symbol are immutable.

### create_metadata_accounts

(Mint authority must be signer)

This action creates the Owner and Metadata accounts. 

### update_metadata_accounts

(Owner must be signer)

This call can be called at any time by the owner to update the URI, and later other fields.

### Further extensions

This app is designed to be extended with further account buckets. If say, we wanted to add metadata for youtube
metadata, we could create a new struct called Youtube and seed it with the seed

```rust
["metadata".as_bytes(), program_id.as_ref(), mint_key.as_ref(), "youtube".as_bytes()]
```

And then only those interested in that metadata need search for it, and it's uniqueness is ensured. It can also
have it's own update action that follows a similar pattern to the original update action.
