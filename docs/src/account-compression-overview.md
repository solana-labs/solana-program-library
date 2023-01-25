---
title: Account Compression Program Overview
---

The [Account Compression Program](https://github.com/solana-labs/solana-program-library/tree/master/account-compression) is implemented in the Solana Program Library to allow developers levarage the off-chain database solutions' network speed, and "concurrently" provide users with complete on-chain verification and security to store digital assets like Non-Fungible tokens.

## Motivation

The primary motivation of the [Account Compression standard](https://github.com/solana-labs/solana-program-library/tree/master/account-compression) being implemented (currently in Beta) by Solana as part of the Solana Program Library is to enable storing of compressed digital assets, specifically, Non-Fungible-Tokens on-chain using their digital fingerprints and Concurrent Merkle trees, thus allowing users to edit the coupled off-chain assets, stored on conventional databases and verify the same on the on-chain Concurrent Merkle trees.

Account Compression allows users to:

1.  Store compressed digital data on-chain at a price as close as possible to 0, with a complete on-chain verification property and the ability to reconstruct the uncompressed data from the on-chain fingerprints, by processing the ledger sequentially.
    
2.  Use the network speeds of current traditional (web2) databases to fetch the digital data, at the same time leverage the security, on-chain verification of data on the Solana Blockchain.

## Source

The Account Compression Program's source is available on [github](https://github.com/solana-labs/solana-program-library).

## Background
The Account Compression standard is built on top of the Concurrent Merkle trees concept, currently, the aim is to support the [Metaplex-based Compressed Non-Fungible Tokens standard](https://github.com/metaplex-foundation/metaplex-program-library/tree/master/bubblegum).

  

Merkle trees are one of the fundamental data structure units of a blockchain, which are used to combine the individual hash fingerprints of each transaction to derive a combined hash fingerprint of a single block, this allows the blockchain to verify all transactions in a single block with a single hash fingerprint.

  

Concurrent Merkle trees (more about them in the next section) are an advanced implementation of the current Merkle tree architecture, to allow concurrent write requests on a single Merkle tree. This allows a single Merkle tree to hold all the metadata properties of an NFT and append or update them concurrently.

  

To run this program we need indexers to parse through the current state of transactions on the ConcurrentMerkleTree and store the Merkle proof for the upcoming write requests. We also need indexers to update the off-chain databases with the updated or executed transaction information on-chain.

