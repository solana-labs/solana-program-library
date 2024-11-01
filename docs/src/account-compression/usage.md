---
title: Example usage of the TS SDK
---



## Install

```shell
npm install --save @solana/spl-account-compression @solana/web3.js@1
```

__OR__

```shell
yarn add @solana/spl-account-compression @solana/web3.js@1
```

### Examples

1. Create a tree

```typescript
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

2. Add a leaf to the tree

```typescript
// Create a new leaf
const newLeaf: Buffer = crypto.randomBytes(32);
// Add the new leaf to the existing tree
const appendIx = createAppendIx(cmtKeypair.publicKey, payer.publicKey, newLeaf);
const tx = new Transaction().add(appendIx);
await sendAndConfirmTransaction(connection, tx, [payer]);
```

3. Replace a leaf in the tree, using the provided `MerkleTree` as an indexer

This example assumes that `offChainTree` has been indexing all previous modifying transactions
involving this tree. 
It is okay for the indexer to be behind by a maximum of `maxBufferSize` transactions.


```typescript
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

4. Replace a leaf in the tree, using a 3rd party indexer

This example assumes that some 3rd party service is indexing the tree at `cmtKeypair.publicKey` for you, and providing MerkleProofs via some REST endpoint.
The `getProofFromAnIndexer` function is a **placeholder** to exemplify this relationship.

```typescript
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

## Reference examples

Here are some examples using account compression in the wild:

* Solana Program Library [tests](https://github.com/solana-labs/solana-program-library/tree/master/account-compression/sdk/tests)

* Metaplex Program Library Compressed NFT [tests](https://github.com/metaplex-foundation/metaplex-program-library/tree/master/bubblegum/js/tests)
