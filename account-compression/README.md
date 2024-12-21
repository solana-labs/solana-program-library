# SPL Account Compression (Beta)

## Overview

SPL Account Compression is an on-chain program that enables smart contracts to work with SPL ConcurrentMerkleTrees. This technology allows for efficient on-chain verification of off-chain data modifications, primarily designed to support compressed NFTs on Solana.

The main benefits include:
- Reduced storage costs for NFT collections
- Efficient on-chain verification of off-chain data
- Optimal for large-scale NFT operations

> **Important**: Implementation requires an indexer service to monitor transactions and maintain an off-chain database of relevant data.

## Primary Use Case

The program's main application is supporting [Metaplex Compressed NFTs](https://github.com/metaplex-foundation/mpl-bubblegum), allowing creators to mint and manage NFTs at a fraction of the usual cost. The implementation may evolve as the technology matures.

## Technical Documentation

A preliminary whitepaper detailing SPL ConcurrentMerkleTrees can be found [here](https://drive.google.com/file/d/1BOpa5OFmara50fTvL0VIVYjtg-qzHCVc/view).

## Available Packages

### Rust SDKs

The program provides three main Rust packages:

1. `spl-account-compression`
   - Core SDK for interacting with the account compression program
   - Handles primary compression functionality

2. `spl-noop`
   - Utility SDK for the no-op program
   - Primarily used to handle log truncation issues

3. `spl-concurrent-merkle-tree`
   - SDK for creating and managing SPL ConcurrentMerkleTrees
   - Provides core merkle tree functionality

### TypeScript Support

The `@solana/spl-account-compression` package provides TypeScript bindings, generated using [Solita](https://github.com/metaplex-foundation/solita/) by the Metaplex Foundation.

## Development Guide

### Setting Up the Test Environment

To run the test suite locally, follow these steps:

1. Build the local SDK
2. Link the account compression package:
   ```bash
   pnpm link @solana/spl-account-compression
   ```
3. Install dependencies:
   ```bash
   pnpm i
   ```
4. Run the test suite:
   ```bash
   pnpm test
   ```

## Security

The program has undergone security audits. For detailed information about the audit status and findings, please refer to the main [Solana Program Library README](https://github.com/solana-labs/solana-program-library#audits).