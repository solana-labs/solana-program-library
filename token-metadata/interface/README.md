## Token-Metadata Interface

An interface describing the instructions required for a program to implement
to be considered a "token-metadata" program for SPL token mints. The interface
can be implemented by any program.

With a common interface, any wallet, dapp, or on-chain program can read the metadata,
and any tool that creates or modifies metadata will just work with any program
that implements the interface.

There is also a `TokenMetadata` struct that may optionally be implemented, but
is not required because of the `Emit` instruction, which indexers and other off-chain
users can call to get metadata.

### Example program

Coming soon!

### Motivation

Token creators on Solana need all sorts of functionality for their token-metadata,
and the Metaplex Token-Metadata program has been the one place for all metadata
needs, leading to a feature-rich program that still might not serve all needs.

At its base, token-metadata is a set of data fields associated to a particular token
mint, so we propose an interface that serves the simplest base case with some
compatibility with existing solutions.

With this proposal implemented, fungible and non-fungible token creators will
have two options:

* implement the interface in their own program, so they can eventually extend it
with new functionality or even other interfaces
* use a reference program that implements the simplest case

### Required Instructions

All of the following instructions are listed in greater detail in the source code.
Once the interface is decided, the information in the source code will be copied
here.

#### Initialize

Initializes the token-metadata TLV entry in an account with an update authority,
name, symbol, and URI.

Must provide an SPL token mint and be signed by the mint authority.

#### Update Field

Updates a field in a token-metadata account. This may be an existing or totally
new field.

Must be signed by the update authority.

#### Remove Key

Unsets a key-value pair, clearing an existing entry.

Must be signed by the update authority.

#### Update Authority

Sets or unsets the token-metadata update authority, which signs any future updates
to the metadata.

Must be signed by the update authority.

#### Emit

Emits token-metadata in the expected `TokenMetadata` state format. Although
implementing a struct that uses the exact state is optional, this instruction is
required.

### (Optional) State

A program that implements the interface may write the following data fields
into a type-length-value entry into an account:

```rust
type Pubkey = [u8; 32];
type OptionalNonZeroPubkey = Pubkey; // if all zeroes, interpreted as `None`

pub struct TokenMetadata {
    /// The authority that can sign to update the metadata
    pub update_authority: OptionalNonZeroPubkey,
    /// The associated mint, used to counter spoofing to be sure that metadata
    /// belongs to a particular mint
    pub mint: Pubkey,
    /// The longer name of the token
    pub name: String,
    /// The shortened symbol for the token
    pub symbol: String,
    /// The URI pointing to richer metadata
    pub uri: String,
    /// Any additional metadata about the token as key-value pairs. The program
    /// must avoid storing the same key twice.
    pub additional_metadata: Vec<(String, String)>,
}
```

By storing the metadata in a TLV structure, a developer who implements this
interface in their program can freely add any other data fields in a different
TLV entry.

You can find more information about TLV / type-length-value structures at the
[spl-type-length-value repo](https://github.com/solana-labs/solana-program-library/tree/master/libraries/type-length-value).
