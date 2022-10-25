# `@solana/spl-account-compression`

A TypeScript library for interacting with SPL Account Compression and SPL NoOp.

## Install

```shell
npm install --save @solana/spl-account-compression @solana/web3.js
```

__OR__

```shell
yarn add @solana/spl-account-compression @solana/web3.js
```


## Examples

* Solana Program Library [tests](https://github.com/solana-labs/solana-program-library/tree/master/account-compression/sdk/tests)

* Metaplex Program Library Compressed NFT [tests](https://github.com/metaplex-foundation/metaplex-program-library/tree/master/bubblegum/js/tests)

## Information

This on-chain program provides an interface for composing smart-contracts to create and use SPL ConcurrentMerkleTrees. The primary application of using SPL ConcurrentMerkleTrees is to make edits to off-chain data with on-chain verification.

This program is targeted towards supporting [Metaplex Compressed NFTs](https://github.com/metaplex-foundation/metaplex-program-library/tree/master/bubblegum) and may be subject to change.

Note: Using this program requires an indexer to parse transaction information and write relevant information to an off-chain database.

A **rough draft** of the whitepaper for SPL ConcurrentMerkleTree's can be found [here](https://drive.google.com/file/d/1BOpa5OFmara50fTvL0VIVYjtg-qzHCVc/view).

## Build from Source

0. Install dependencies with `yarn`.

1. Generate the Solita SDK with `yarn solita`.

2. Then build the SDK with `yarn build`.

3. Run tests with `yarn test`. (Expect `jest` to detect an open handle that prevents it from exiting naturally)


#### Examples

1. Create a tree

```typescript
// Generate a keypair for the ConcurrentMerkleTree
const cmtKeypair = Keypair.generate();

// Create a system instruction to allocate enough 
// space for the tree
const allocAccountIx = await createAllocTreeIx(
    connection,
    cmtKeypair.publicKey,
    payer.publicKey,
    maxSize,
    maxDepth,
    canopyDepth,
);

// Create an SPL compression instruction to initialize
// the newly created ConcurrentMerkleTree
const initTreeIx = createInitEmptyMerkleTreeIx(
    cmtKeypair.publicKey, 
    payer.publicKey, 
    maxDepth, 
    maxSize
);

const tx = new Transaction().add(allocAccountIx).add(initTreeIx);

await sendAndConfirmTransaction(connection, tx, [cmtKeypair, payer]);
```

2. Add a leaf to the tree

```typescript
// Create a new leaf
const newLeaf: Buffer = crypto.randomBytes(32);

// Add it to an existing tree
const appendIx = createAppendIx(cmt, payer, newLeaf);

const tx = new Transaction().add(appendIx);
await sendAndConfirmTransaction(connection, tx, [payer]);
```

3. Replace a leaf in the tree

```typescript
// Get a new leaf
const newLeaf: Buffer = crypto.randomBytes(32);

// Query off-chain records for information about the leaf
// you wish to replace
const leafIndex = 314;

let proof: Buffer[] = getProofOfLeaf(offChainTree, leafIndex)
    .map((n) => n.node);

const replaceIx = createReplaceIx(
    cmt,            // ConcurrentMerkleTree
    payer,          // Authority of the Tree
    root,           // Buffer 
    previousLeaf,   // Buffer
    newLeaf,        
    314,            // Index of the leaf in the tree, 0-indexed
    proof             
);
const tx = new Transaction().add(replaceIx);
await sendAndConfirmTransaction(connection, tx, [payer]);
```