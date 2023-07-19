# Editions

This interface is designed to allow on-chain programs to make use of printed "copies" of token metadata, with parent-child relationships.

Metaplex introduced the idea of [Editions](https://docs.metaplex.com/programs/token-metadata/overview#printing-editions) for tokens, a concept most projects on Solana are readily familiar with.

In short, editions allow you to create an original "copy" of a token and then print new "copies" of that token. Although we're talking about "copies", the token itself is not copied. All that's being copied with Editions is the **token metadata**.

## Metaplex Protocol

Below is an overview of how Metaplex's Editions work.

### Create the Original

Create a `MasterEdition` account for the the original token.

- Mint a new SPL Token.
- Create a `Metadata` account for the token.
- Create a `MasterEdition` account for the token.

As described in [Metaplex's docs](https://docs.metaplex.com/programs/token-metadata/overview#printing-editions):
> "The Master Edition NFT, a.k.a. Original NFT, acts as the master record that one can use to print copies, a.k.a. Print NFTs."

We can infer how a `MasterEdition` account dictates the printing of copies by examining its schema below:

```rust
pub struct MasterEditionV2 {
    pub key: Key,       // Enum value `MasterEdition`
    pub supply: u64,
    pub max_supply: Option<u64>,
}
```

With Metaplex's protocol, the `MasterEdition` determines the max supply and also incrementally records the current supply, so we know **this account must always be involved with the printing of new editions**.

### Print Copies

Create one or more copies of the original token by creating an `Edition`, by creating new tokens that inherently "copy" the original token's metadata.

- Mint a new SPL Token.
- Create a `Metadata` account for the token with the data _copied from the `Metadata` account referred to by the `MasterEdition`_.
- Create an `Edition` account for the token pointing to the `MasterEdition` account.

We can immediately see the parent-child relationship made evident within an `Edition` schema:

```rust
pub struct Edition {
    pub key: Key,       // Enum value `Edition`
    pub parent: Pubkey,
    pub edition: u64,
}
```

### Seeds

The PDA seeds used to derive a `MasterEdition` and an `Edition` are the same, so that a token can only have one or the other. They are as follows:

```text
"metadata" + <token metadata program ID> + <mint address> + "edition"
```

## SPL Interface

The interface - albeit strongly inspired by and largely based upon Metaplex's original engineering - should be simple enough to not inundate or force anyone to comply with any preconcieved ideas of how these parent-child relationships amongst editions should interact.

> Note: Changing the nomenclature of these state objects may potentially confuse people familiar with Metaplex's model, but serve to acknowledge the fact that this interface is something entirely different.

### The Original

This interface will dub the original token metadata simply as an `Original`.

Similar to `MasterEdition`, this state will store information pertaining to supply and maximum supply.

```rust
pub struct Original {
    pub update_authority: OptionalNonZeroPubkey,
    pub supply: u64,
    pub max_supply: Option<u64>,
}
```

### The Reprint

This interface will dub copies of the original as a `Reprint`.

Similar to `Edition`, this state will store information pointing back to the parent.

```rust
pub struct Reprint {
    pub original: Pubkey,
    pub copy: u64,
}
```

### Seeds (SPL)

It's worth noting that, as Metaplex has designed, making the seeds for these types of accounts the same is a good idea. However, this interface cannot enforce this.

In the case of the interface, both states share the same **SPL discriminator**, thus mimicking the conflicting seed pattern of Metaplex.

> Note: Although the naming conventions `Original` and `Reprint` exist in the instruction nomonclature, this interface cannot enforce state.

### Working with Metadata

Editions work closely with token metadata, so it makes sense to implement _both_ the SPL Token Metadata interface as well as the SPL Token Editions interface within the same on-chain program.

However, this is not required! As long as the mint authority is correct, a program that implements the SPL Token Editions interface can create `Original` prints and simply CPI into the proper Token Metadata program to create `Reprint` copies!

To see this concept in action, and more details on implementing the SPL Token Editions interface, see the `example` program in this repository.
