---
title: Core Concepts
---

## Concurrent Merkle Trees

To understand concurrent merkle trees we must first briefly understand merkle trees.

### Merkle Trees

A merkle tree is a hash based data structure that encodes data into a tree.
The tree has nodes that are hashes of its children and each leaf node is a hash of the data.

Each node has a 256 bit (32 byte) string represented by X<sub>i</sub> ∈ \{0,1\}^256 which is hashed using `H: {0, 1}^256 × {0, 1}^256 → {0, 1}^256`, meaning two child nodes with their 256 bit strings are hashed into one parent node with a 256 bit string. You can use any hash function that satisfies this property but we use SHA256.

Important properties of merkle trees:

-   The tree must be a fully balanced binary tree
-   Each Node _X<sub>i</sub> = H(X<sub>2i</sub>, X<sub>2i+1</sub>) for all i < 2^D_
-   Each Leaf Node _X<sub>i</sub> for all i {'<='} 2^D_. X<sub>i</sub> is the hash of the data.

Because of these properties we can verify if certain data exists in tree while compressing all the data into a single 256 bit string called the root hash.

Example of a merkle tree of depth 2:

```txt
        X1
      /    \
    X2      X3
   / \     / \
 X4  X5   X6  X7
```

You can verify that X5 computes to X1 by doing X1 = H(H(X4,X5),X3)) where \{X4,X5,X3\} are the proof.
If you change X5 to X5' then you will have to recompute the root hash in the following steps:

-   X2' = H(X4,X5')
-   X1' = H(X2',X3)

### Concurrent leaf replacement

We know that there can be multiple concurrent requests to write to the same state, however when the root changes while the first write is happening the second write will generate an invalid root, in other words every time a root is modified all modifications in progress will be invalid.

```txt
          X1'              X1''
        /    \           /    \
      X2'      X3       X2      X3''
     / \     / \       / \     / \
   X4  X5'   X6  X7   X4  X5  X6'' X7
```

In the above example let's say we try to modify `X5 -> X5'` and make another request to modify X6 -> X6''. For the first change we get root `X1'` computed using `X1' = H(H(X4,X5'),X3)`. For the second change we get root X1'' computed using `X1'' = H(H(X6'',X7),X2`). However `X1''` is not valid as `X1' != H(H(X6, X7), X2)` because the new root is actually `X1'`.

The reason this happens is because the change in the first trees path actually changes the proofs required by the second trees change. To circumvent this problem we maintain a changelog of updates that have been made to the tree, so when `X5 -> X5'` the second mutation can actually use X2' instead of X2 which would compute to the correct root.

To swap the nodes when adding a new leaf in the second tree we do the following:

-   Take XOR of the leaf indices of the change log path and the new leaf in base 2
-   The depth at which you have to make the swap is the number of leading zeroes in the result(we also add one to it because the swap node is one below the intersection node)
-   At that depth change the node in the proof to the node in the changelog

Example with the previous trees:

```txt
             2   1
Changelog: [X5',X2']
New Leaf: X6'' at leaf index 2

                         2   1
Old proof for new leaf: [X7,X2]

1 XOR 2 = 001 XOR 010 = 011 (no leading zeroes)
depth to swap at = 0 + 1 = 1

                          2   1
New proof for new leaf: [X7,X2']
```

**Note:** We use XOR here because changelogs can get large as there can be many concurrent writes so using XOR is more efficient than a simple array search algorithm.

**Note**: Solana imposes a transactions size restriction of 1232 bytes hence the program also provides the ability to cache the upper most part of the concurrent merkle tree called a "canopy" which is stored at the end of the account.
