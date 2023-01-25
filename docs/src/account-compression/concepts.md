---
title: Core Concepts
---

## Concurrent Merkle Trees
To understand concurrent merkle trees we must first briefly understand merkle trees.

### Merkle Trees

A merkle tree is a hash based data structure that encodes data into a tree.
The tree has nodes that are hashes of it's children and each leaf node is a hash of the data.

Each node has a 256 bit (32 byte) string represented by X<sub>i</sub> ∈ {0,1}^256 which is hashed using `H: {0, 1}^256 × {0, 1}^256 → {0, 1}^256`, meaning two child nodes with their 256 bit strings are hashed into one parent node with a 256 bit string. You can use can use any hash function that satisfies this property but we use SHA256.

Important properties of merkle trees:
- The tree must be a fully balanced binary tree
- Each Node *X<sub>i</sub> = H(X<sub>2i</sub>, X<sub>2i+1</sub>) for all i < 2^D*
- Each Leaf Node *X<sub>i</sub> for all i <= 2^D*. X<sub>i</sub> is the hash of the data.

Because of these properties we can verify if certain data exists in tree while compressing all the data into a single 256 bit string called the root hash.

Example of a merkle tree of depth 2:
```txt
        X1
      /    \
    X2      X3
   / \     / \
 X4  X5   X6  X7
```
You can verify that X5 computes to X1 by doing X1 = H(H(X4,X5),X3)) where {X4,X5,X3} are the proof.
If you change X5 to X5' then you will have to recompute the root hash in the following steps:
- X2' = H(X4,X5')
- X1' = H(X2',X3)

### Concurrent leaf replacement
We know that there can be multiple concurrent requests to write to the same state, however when the root changes while the first write is happenning the second write will generate an invalid root, in other words everytime a root is modified all modifications in progress will be invalid.
```txt
          X1              C1'
        /    \          /    \
      X2      X3      C2'      C3'
     / \     / \      / \     / \
   X4  X5   X6  X7   C4'  C5'  C6' C7'
```
In the above example