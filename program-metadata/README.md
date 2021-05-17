
---
title: SPL Program Metadata
---

## Summary

This program allows for programs to be associated with metadata. For metadata, this includes arbitrary key / value pairs and a special type of metadata for describing how to interact with a program -- an IDL (Interface Definition Language). Only those with authority over a given program are allowed to add or remove metadata. All data is stored through the SPL Name Service program with SPL Program Metadata as the class (see [SPL Name Service](https://spl.solana.com/name-service)).

[SPL Program Metadata Proposal]()

## Background

Solana's programming model and the definitions of the Solana terms used in this
document are available at:

- https://docs.solana.com/apps
- https://docs.solana.com/terminology

## Source

The Program Metadata program source is available on [github](https://github.com/solana-labs/solana-program-library)

There is also a JavaScript client located at [github](https://github.com/solana-labs/solana-program-library/tree/master/program_metadata/ts).

## Interface

The on-chain Token Metadata program is written in Rust and available on crates.io as [spl-program-metadata](https://crates.io/crates/spl-program-metadata) and [docs.rs](https://docs.rs/spl-program-metadata).

The crate provides five instructions, `create_metadata_entry()`, `update_metadata_entry()`, `delete_metadata_entry()`, `create_versioned_idl()`, `update_versioned_idl()` to create instructions for the program.

## Operational Overview

All program metadata operations require two program derived addresses:

1. A `Class` address for SPL Program Metadata, seeded with:
`["program_metadata", target_program_key, program_metadata_key]`
2. A `Name Record` address for SPL Name Service, seeded with:
`[SHA256(HASH_PREFIX + "Create::name"), class_account_key, Pubkey::default()]`

They also require the following accounts:
3. The Program account (the program metadata will be associated with)
4. The Program's ProgramData account (the program data account associated with the Program)
5. The Program Authority (the account with update authority on the Program as a signer)

### create_metadata_entry
```
///   0. `[]` Class account (seed: ['program_metadata', target_program_key, program_metadata_key])
///   1. `[writable]` Name record account (seed: [SHA256(HASH_PREFIX, 'Create::name'), class_key, ])
///   2. `[]` Target program
///   3. `[]` Target program ProgramData
///   4. `[signer]` Target program update authority
///   5. `[signer]` Payer
///   6. `[]` System program
///   7. `[]` Rent info
///   8. `[]` Name service
CreateMetadataEntry {
    name: String,
    value: String,
    hashed_name: Vec<u8>,
},
```

### update_metadata_entry
```
///   0. `[]` Class account (seed: ['program_metadata', target_program_key, program_metadata_key])
///   1. `[writable]` Name record account (seed: [SHA256(HASH_PREFIX, 'Create::name')])
///   2. `[]` Target program
///   3. `[]` Target program ProgramData
///   4. `[signer]` Target program update authority
///   5. `[]` Name service UpdateMetadataEntry { value: String },
```
### delete_metadata_entry
```
///   0. `[]` Class account (seed: ['program_metadata', target_program_key, program_metadata_key])
///   1. `[writable]` Name record PDA (seed: [SHA256(HASH_PREFIX, 'Create::name')])
///   2. `[]` Target program
///   3. `[]` Target program ProgramData
///   4. `[signer]` Target program update authority
///   5. `[]` Refund account
///   6. `[]` Name service
DeleteMetadataEntry,
```

### create_versioned_idl
```
///   0. `[]` Class account (seed: ['program_metadata', target_program_key, program_metadata_key])
///   1. `[writable]` Name record account (seed: [SHA256(HASH_PREFIX, 'Create::name')])
///   2. `[]` Target program
///   3. `[]` Target program ProgramData
///   4. `[signer]` Target program update authority
///   5. `[signer]` Payer
///   6. `[]` System program
///   7. `[]` Rent info
///   8. `[]` Name service
CreateVersionedIdl {
    effective_slot: u64,
    idl_url: String,
    idl_hash: [u8; 32],
    source_url: String,
    serialization: SerializationMethod,
    custom_layout_url: Option<String>,
    hashed_name: [u8; 32],
},
```

### update_versioned_dl
```
///   0. `[writable]` Class account (seed: ['program_metadata', target_program_key, program_metadata_key])
///   1. `[writable]` Name record account (seed: [SHA256(HASH_PREFIX, 'Create::name')])
///   2. `[]` Target program
///   3. `[]` Target program ProgramData
///   4. `[signer]` Target program update authority
///   5. `[]` Name service
UpdateVersionedIdl {
    idl_url: String,
    idl_hash: [u8; 32],
    source_url: String,
    serialization: SerializationMethod,
    custom_layout_url: Option<String>,
},
```