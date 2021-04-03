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
[github](https://github.com/solana-labs/solana-program-library/tree/master/token_metadata/test/src/main.rs)
that can be perused for learning and run if desired with `cargo run --bin spl-token-metadata-test-client`. It allows testing out a variety of scenarios.

## Interface

The on-chain Token Metadata program is written in Rust and available on crates.io as
[spl-metadata](https://crates.io/crates/spl-token-metadata) and
[docs.rs](https://docs.rs/spl-token-metadata).

The crate provides three instructions, `create_metadata_accounts()`, `update_metadata_accounts()` and `transfer_update_authority()`to easily create instructions for the program.

## Operational overview

This is a very simple program designed to allow metadata tagging to a given mint, with an update authority
that can change that metadata going forward. The app is composed of 3 actions, one which will create the accounts, one which will update some of the fields on them, and a third action which for a special subset
of accounts will allow swapping of the authorities.

### Permissioning and Architecture

The Metadata app creates two different kinds of Metadata: Unique metadata and Non-Unique metadata. These are
toggled via the `allow_duplicates` boolean in the `create_metadata_accounts` call.

Only the minting authority on a mint can create metadata accounts. A Metadata account holds the name, symbol,
and uri of the mint, as well as the mint id. to ensure the uniqueness of
a mint's metadata, the address of a Metadata account is a program derived address composed of seeds:

```rust
["metadata".as_bytes(), program_id.as_ref(), mint_key.as_ref()]
```

If the caller is alright with having other people potentially duplicate their name/symbol combination
(ie `allow_duplicates` is `true`) then the additional field `non_unique_specific_update_authority`,
which is a `Option<Pubkey>` will be set to the update authority. If the caller prefers to reserve their
name/symbol combination for unique use, they can join the pool of unique Metadata by
setting `allow_duplicates` to `false`. When this is done, a second account called a NameSymbolTuple is created.

The NameSymbolTuple address is a program derived address composed of seeds:

```rust
["metadata".as_bytes(), program_id.as_ref(), name_as_bytes, symbol_as_bytes]
```

This ensures easy lookups by those interested - they can simply look up the metadata account by mint address, then
look up NameSymbolTuple with the name and symbol if they want to. This means a client who is interested in NFTs
can do RPC calls against Metadata only in the unique space by searching for Metadatas with the first bit of 0,
because the first bit in Metadata is always 0 for unique Metadata and always 1 for non-unique Metadata.

Also users who wish to look up a particular set of Metadata for a unique name-symbol combo can look up a NameSymbolTuple by it's program-derived address, and because it has both the metadata key and update authority,
they can easily learn who the owner of that name/symbol is and what mint backs it.

For metadatas that are part of the unique pool, they will need to use the separate `set_update_authority` call,
to change update authorities. For those that aren't, they can do so via the normal update call.

Due to the nature of the addresses on these accounts, name and symbol are immutable.

### create_metadata_accounts

(Mint authority must be signer)

This action creates the NameSymbolTuple(only if `allow_duplicates` is `true`) and Metadata accounts.
This can also be used for an existing NameSymbolTuple, if the update authority that is passed in is also signer,
to reset the metadata account it is pointing at to a newly created metadata. This effectively transfers the
NameSymbolTuple from one Metadata account to a new one.

### update_metadata_accounts

(Update authority must be signer)

This call can be called at any time by the update authority to update the URI on any metadata or
update authority on non-unique metadata, and later other fields.

### transfer_update_authority

(Update authority must be signer)

For unique metadatas, this transfers the ownership of NameSymbolTuple to a different person.

### Further extensions

This program is designed to be extended with further account buckets.

If say, we wanted to add metadata for youtube metadata, we could create a new struct called Youtube
and seed it with the seed

```rust
["metadata".as_bytes(), program_id.as_ref(), mint_key.as_ref(), "youtube".as_bytes()]
```

And then only those interested in that metadata need search for it, and its uniqueness is ensured. It can also
have it's own update action that follows a similar pattern to the original update action.
