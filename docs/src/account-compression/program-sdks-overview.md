---
title: Account Compression SDKs
---

The [SPL-Account Compression Program](https://github.com/solana-labs/solana-program-library/tree/master/account-compression) implemented Rust Packages and a Typescript SDK in the current Beta version of the program. These enable developers to interact with the on-chain compression programs and concurrent Merkle trees.

## Rust Packages
| Name | Description | Program |
| --- | --- | --- |
| `spl-account-compression`| SDK for interacting with account compression program |[Rust Crate](https://crates.io/crates/spl-account-compression) and [Rust Docs](https://docs.rs/spl-account-compression)| 
| `spl-noop` | SDK for interacting with no op program, primarily for circumventing log truncation | [Rust Crate](https://crates.io/crates/spl-noop) and [Rust Docs](https://docs.rs/spl-noop)|
| `spl-concurrent-merkle-tree` | SDK for creating SPL ConcurrentMerkleTrees |[Rust Crate](https://crates.io/crates/spl-concurrent-merkle-tree) and [Rust Docs](https://docs.rs/spl-concurrent-merkle-tree)|
## TypeScript SDK
The [TypeScript SDK](https://github.com/solana-labs/solana-program-library/tree/master/account-compression/sdk) contains required methods and functions to interact with the Concurrent Merkle tree data structure, SPL Account Compression and SPL NoOp, to perform operations like append a leaf value, update a leaf, etc. This SDK is built on-top of the [Metaplex foundation's Solita framework](https://github.com/metaplex-foundation/solita/) which provides low-level interaction code to interact with Concurrent Merkle trees. 

## Example usage of the Typescript SDK
**Creating a Concurrent Merkle Tree using the SDK**
```ts
// Assume: known `payer` Keypair

// Generate a keypair for the ConcurrentMerkleTree
const cmtKeypair = Keypair.generate();

// Create a system instruction to allocate enough 
// space for the tree
const allocAccountIx = await createAllocTreeIx(
    connection,
    cmtKeypair.publicKey,
    payer.publicKey,
    { maxDepth, maxBufferSize },
    canopyDepth,
);

// Create an SPL compression instruction to initialize
// the newly created ConcurrentMerkleTree
const initTreeIx = createInitEmptyMerkleTreeIx(
    cmtKeypair.publicKey, 
    payer.publicKey, 
    { maxDepth, maxBufferSize }
);

const tx = new Transaction().add(allocAccountIx).add(initTreeIx);

await sendAndConfirmTransaction(connection, tx, [cmtKeypair, payer]);
```

**Appending a new leaf node to the tree**
```ts
// Create a new leaf
const newLeaf: Buffer = crypto.randomBytes(32);

// Add the new leaf to the existing tree
const appendIx = createAppendIx(cmtKeypair.publicKey, payer.publicKey, newLeaf);

const tx = new Transaction().add(appendIx);

await sendAndConfirmTransaction(connection, tx, [payer]);
```

**Replacing a leaf node**: Assuming that the indexer used in tandem with the Concurrent Merkle Tree is the  provided  `offChainTree` Merkle Tree itself (no 3rd-party indexer).
```ts
// Assume: `offChainTree` is a MerkleTree instance
// that has been indexing the `cmtKeypair.publicKey` transactions

// Get a new leaf
const newLeaf: Buffer = crypto.randomBytes(32);

// Query off-chain records for information about the leaf
// you wish to replace by its index in the tree
const leafIndex = 314;

// Replace the leaf at `leafIndex` with `newLeaf`
const replaceIx = createReplaceIx(
    cmtKeypair.publicKey,          
    payer.publicKey,
    newLeaf,
    offChainTree.getProof(leafIndex) 
);

const tx = new Transaction().add(replaceIx);

await sendAndConfirmTransaction(connection, tx, [payer]);
```

**Replacing a leaf node usin a 3rd-party indexer**: Assuming that the 3rd-party indexer is indexing the tree at: `cmtKeypair.publicKey` for the developer, and providing the following API endpoint to access the indexed data.: `getProofFromAnIndexer` .
```ts
// Get a new leaf
const newLeaf: Buffer = crypto.randomBytes(32);

// Query off-chain indexer for a MerkleProof
// possibly by executing GET request against a REST api
const proof = await getProofFromAnIndexer(myOldLeaf);

// Replace `myOldLeaf` with `newLeaf` at the same index in the tree
const replaceIx = createReplaceIx(
    cmtKeypair.publicKey,          
    payer.publicKey,
    newLeaf,
    proof
);

const tx = new Transaction().add(replaceIx);

await sendAndConfirmTransaction(connection, tx, [payer]);
```


## Testing and Deployment
The following Account Compression Program can be tested using either [`SPL tests`](https://github.com/solana-labs/solana-program-library/tree/master/account-compression/sdk/tests) or the[`Metaplex Program Library tests on compressed NFTs`](https://github.com/metaplex-foundation/metaplex-program-library/tree/master/bubblegum/js/tests).

The `SPL tests` can be performed by the following commands:

Installing the required dependencies after forking the SDK in the local environment
```sh
$ yarn
```

Generating the Solita SDK
```sh
$ yarn solita
```

Building the SDK
```sh
$ yarn build
```


Running the tests
```sh
$ yarn test
```
