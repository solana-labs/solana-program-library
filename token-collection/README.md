# SPL Token Collection

This program serves as a reference implementation for using the SPL Token Group
interface to create an on-chain program for managing token collections - such
as NFT Collections.

This program bears a lot of similarity to the example program found at
`token-group/example`, but with some additional implementations centered around
specifically token collections.

## How Collections Work in this Program

Strictly for demonstration purposes, this program is going to require the
following:

- Group tokens must be NFTs (0 decimals, 1 supply)
- Group tokens must have metadata
- Member tokens can be any SPL token, but must have metadata
- Member tokens can be part of multiple collections

## Demonstration

For a particularly fleshed-out example of this program in action, check out the
`token-collections.rs` test under `tests`!