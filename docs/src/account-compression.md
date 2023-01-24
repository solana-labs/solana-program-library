---
title: Account Compression Program
---

This program provides an interface for composing smart-contracts to create and use [SPL Concurrent Merkle Trees](https://drive.google.com/file/d/1BOpa5OFmara50fTvL0VIVYjtg-qzHCVc/view). 

The primary application of using SPL Concurrent Merkle Trees is to make edits to off-chain data with on-chain verification.

## Background

The Account Compression Program is targeted towards supporting [Metaplex Compressed NFTs](https://github.com/metaplex-foundation/metaplex-program-library/tree/master/bubblegum) and may be subject to change.

**Note**: Using this program requires an indexer to parse transaction information and write relevant information to an off-chain database.

## Source

- `account-compression` can be found in the [solana-program-library](https://github.com/solana-labs/solana-program-library/tree/master/account-compression)
- `spl-noop` can be found in the [solana-program-library](https://github.com/solana-labs/solana-program-library/tree/master/account-compression/programs/noop)
- `bubblegum` can be found in the [metaplex-program-library](https://github.com/metaplex-foundation/metaplex-program-library/tree/master/bubblegum)


## Interface

The supporting Programs are written in Rust and available as follows: 


| Name | Description | Program |
| --- | --- | --- |
| `spl-account-compression`| SDK for interacting with account compression program |[crates.io](https://crates.io/crates/spl-account-compression) and [docs.rs](https://docs.rs/spl-account-compression).| 
| `spl-noop` | SDK for interacting with no op program, primarily for circumventing log truncation | [crates.io](https://crates.io/crates/spl-noop) and [docs.rs](https://docs.rs/spl-noop). |
| `spl-concurrent-merkle-tree` | SDK for creating SPL ConcurrentMerkleTrees | [crates.io](https://crates.io/crates/spl-concurrent-merkle-tree) and [docs.rs](https://docs.rs/spl-concurrent-merkle-tree). |


## Testing and Development

Testing Account Compression Program requires the `@solana/spl-account-compression` SDK to be built locally.

With a built local SDK, the test suite can be ran with these steps:

 1. `yarn link @solana/spl-account-compression`
 2. `yarn`
 3. `yarn test`

Example code references related to creating Merkle Trees can be found over [here](https://github.com/solana-labs/solana-program-library/tree/master/account-compression/sdk#examples)

Example code implementations of Account compression:
- Solana Program Library [tests](https://github.com/solana-labs/solana-program-library/tree/master/account-compression/sdk/tests)
- Metaplex Program Library [tests](https://github.com/metaplex-foundation/metaplex-program-library/tree/master/bubblegum/js/tests)
- Metaplex compression read API [example](https://github.com/metaplex-foundation/compression-read-api-js-examples)
