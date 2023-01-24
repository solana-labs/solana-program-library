# Account Compression Program
 This on-chain program provides an interface for composing smart contracts to create and use SPL ConcurrentMerkleTrees and also this program is targetted towards [Metaplex Compressed NFTs](https://github.com/metaplex-foundation/metaplex-program-library/tree/master/bubblegum)

The primary application of using SPL ConcurrentMerkleTrees is to make edits to off-chain data with on-chain verification.

# Background

This program comes from the usage of SPL ConcurrentMerkleTrees. We can find the white paper of 
ConcurrentMerkleTrees will be available at

-  [SPL ConcurrentMerkleTree WhitePaper](https://drive.google.com/file/d/1BOpa5OFmara50fTvL0VIVYjtg-qzHCVc/view)

While SPL ConcurrentMerkleTrees can generically store arbitrary information, one exemplified use case is the Bubblegum contract, which uses SPL-Compression to store encoded information about NFTs. The use of SPL-Compression within [Bubblegum](https://github.com/metaplex-foundation/metaplex-program-library/tree/master/bubblegum) allows for:

- up to 1 billion NFTs to be stored in a single account on-chain (>10,000x decrease in on-chain cost)
- up to 2048 concurrent updates per slot

Operationally, SPL ConcurrentMerkleTrees must be supplemented by off-chain indexers to cache information about leaf and to power an API that can supply up-to-date proofs to allow updates to the tree. All modifications to SPL ConcurrentMerkleTrees are settled on the Solana ledger via instructions against the SPL Compression contract.

A production-ready indexer (Plerkle) can be found in the [Metaplex Program Library](https://github.com/metaplex-foundation/digital-asset-validator-plugin)

# Source

The Account Compression program's code is available on [github](https://github.com/solana-labs/solana-program-library)

# Interface

The Account Compression Program is written in rust and available on [crates.io](https://crates.io/crates/spl-account-compression) and [doc.rs](https://docs.rs/spl-account-compression/latest/spl_account_compression/).

Account compression has the following functions are written in rust:

- [append](https://docs.rs/spl-account-compression/latest/spl_account_compression/spl_account_compression/fn.append.html)

```
pub fn append(
    ctx: Context<'_, '_, '_, '_, Modify<'_>>,
    leaf: [u8; 32]
) -> Result<()> 
```
This instruction allows the tree’s authority to append a new leaf to the tree without having to supply a proof.

- [close empty tree](https://docs.rs/spl-account-compression/latest/spl_account_compression/spl_account_compression/fn.close_empty_tree.html)

```
pub fn close_empty_tree(
    ctx: Context<'_, '_, '_, '_, CloseTree<'_>>
) -> Result<()>

```

This instruction allows to close Merkle tree account.

- [init empty merkle tree](https://docs.rs/spl-account-compression/latest/spl_account_compression/spl_account_compression/fn.init_empty_merkle_tree.html)

```
pub fn init_empty_merkle_tree(
    ctx: Context<'_, '_, '_, '_, Initialize<'_>>,
    max_depth: u32,
    max_buffer_size: u32
) -> Result<()>

```
Creates a new merkle tree with maximum leaf capacity of power(2, max_depth) and a minimum concurrency limit of max_buffer_size.

Concurrency limit represents the # of replace instructions that can be successfully executed with proofs dated for the same root. For example, a maximum buffer size of 1024 means that a minimum of 1024 replaces can be executed before a new proof must be generated for the next replace instruction.

Concurrency limit should be determined by empirically testing the demand for state built on top of SPL Compression.

- [insert or append](https://docs.rs/spl-account-compression/latest/spl_account_compression/spl_account_compression/fn.insert_or_append.html)

```
pub fn insert_or_append(
    ctx: Context<'_, '_, '_, '_, Modify<'_>>,
    root: [u8; 32],
    leaf: [u8; 32],
    index: u32
) -> Result<()>
```
This instruction takes a proof, and will attempt to write the given leaf to the specified index in the tree. If the insert operation fails, the leaf will be append-ed to the tree. It is up to the indexer to parse the final location of the leaf from the emitted changelog.

- [replace leaf](https://docs.rs/spl-account-compression/latest/spl_account_compression/spl_account_compression/fn.replace_leaf.html)

```
pub fn replace_leaf(
    ctx: Context<'_, '_, '_, '_, Modify<'_>>,
    root: [u8; 32],
    previous_leaf: [u8; 32],
    new_leaf: [u8; 32],
    index: u32
) -> Result<()>

```
This instruction has been deemed unusable for publicly indexed compressed NFTs. Indexing batched data in this way requires indexers to read in the uris onto physical storage and then into their database. This opens up a DOS attack vector, whereby this instruction is repeatedly invoked, causing indexers to fail.

- [transfer authority](https://docs.rs/spl-account-compression/latest/spl_account_compression/spl_account_compression/fn.transfer_authority.html)

```
pub fn transfer_authority(
    ctx: Context<'_, '_, '_, '_, TransferAuthority<'_>>,
    new_authority: Pubkey
) -> Result<()>

```
This instruction require authority to sign.

- [verify leaf](https://docs.rs/spl-account-compression/latest/spl_account_compression/spl_account_compression/fn.verify_leaf.html)

```
pub fn verify_leaf(
    ctx: Context<'_, '_, '_, '_, VerifyLeaf<'_>>,
    root: [u8; 32],
    leaf: [u8; 32],
    index: u32
) -> Result<()>

```
This instruction does Verify a provided proof and leaf. If invalid throws an error.

# Packages

- [spl-account-compression](https://github.com/solana-labs/solana-program-library/tree/master/account-compression) available in [crates.io](https://crates.io/crates/spl-account-compression) and [docs.rs](https://docs.rs/spl-account-compression/latest/spl_account_compression/)

- [spl-noop](https://github.com/solana-labs/solana-program-library/tree/master/account-compression/programs/noop) available in [crates.io](https://crates.io/crates/spl-noop) and [docs.rs](https://docs.rs/spl-noop/latest/spl_noop/)

- [bubblegum](https://github.com/metaplex-foundation/metaplex-program-library/tree/master/bubblegum) available in [crates.io](https://crates.io/crates/bubblegum) and [docs.rs](https://docs.rs/crate/bubblegum/1.0.1)


# Testing

Testing locally requires the [SPL account compression SDK](https://www.npmjs.com/package/@solana/spl-account-compression) to be built.

with a built SDK , the test suite can be run with

- `yarn link @solana/spl-account-compression`
- `yarn`
- `yarn test`
















