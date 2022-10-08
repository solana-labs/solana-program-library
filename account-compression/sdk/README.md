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