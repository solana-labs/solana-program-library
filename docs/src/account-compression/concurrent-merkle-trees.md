---
title: Concurrent Merkle Trees
---

The [Account Compression Program](https://github.com/solana-labs/solana-program-library/tree/master/account-compression) is implemented in the Solana Program Library to allow developers levarage the off-chain database solutions' network speed, and "concurrently" provide users with complete on-chain verification and security to store digital assets like Non-Fungible tokens.

## Concept

Merkle trees have three main components:

1.  The root node: The topmost node of the tree.
    
2.  The leaf nodes: The end nodes of the tree.
    
3.  The branch nodes: All other nodes in the tree.
    

Merkle trees store the transaction hash values in the leaf nodes of the tree and propagate the combined hash of each sub-tree toward the root node. This results in a robust mechanism where any change in the leaf node’s value results in a mismatch with the previous root node hash.

  

Merkle Proof of a particular target node is the set of nodes in the Merkle tree required to generate the hash signature of the root node with respect to the target node. As the value of the target node changes, the Merkle proof becomes invalid. To make it valid we need to update the values or nodes in the Merkle proof, with the new hash signatures of each node.

  

Now as we have multiple leaf nodes, there is a possibility of updating two or more leaf nodes concurrently, which may result in a proof collision. To avoid this the Solana Blockchain implemented Concurrent Merkle Trees, which allows users to append and update any number (up to D, depth of the Merkle tree) of write requests onto the on-chain program. More about this implementation can be explored [here](https://drive.google.com/file/d/1BOpa5OFmara50fTvL0VIVYjtg-qzHCVc/view).

  

This robust implementation of Concurrent Merkle trees can be used to store digital assets like NFTs in a compressed format. Where each parameter of an NFT like:

| Property/Seed | Type | Description |
| --- | --- | --- |
| `Owner`| PublicKey |Public key of the asset owner| 
| `Delegate` | PublicKey | Public Key of the asset delegate |
| `Name` | String | Name of the asset |
| `URI` | String | Link to the asset metadata |
| `Asset ID` | UUID | Unique asset identifier |
| `Creator` | PublicKey | creator of the asset (entitiled to royalties) |
| `Royalty Percent` | Integer | Percentage of the sale transferred to the creator |
are considered as leaf nodes, and as transacting or minting an NFT requires concurrent changes in all of the above, the Concurrent Merkle tree data structure allows updating all the parameters on-chain concurrently. At the same time, Concurrent Merkle trees store these NFTs in the form of a hash signature, thus compressing the data to a much smaller size, to save network bandwidth and cost to store each unit.

As this SPL-implementation of Concurrent Merkle Trees aim to support the Metaplex-based compressed NFT standard, a look into their implementation of the Concurrent Merkle Tree application [Bubblegum](https://github.com/metaplex-foundation/metaplex-program-library/tree/master/bubblegum), shows that this efficient data strucuture is capable of **storing upto 1 Billion NFTs under a single account** on-chain, **this reduced the storage fee per unit by more than 10,000 times**. Also this implementation allowed 2048 concurrent updates and ammends per slot.

More details about the implementation of Concurrent Merkle trees and their usage in storing compessed NFT data can be explored in this initial draft of the [whitepaper on SPL Concurrent Merkle trees](https://drive.google.com/file/d/1BOpa5OFmara50fTvL0VIVYjtg-qzHCVc/view).
