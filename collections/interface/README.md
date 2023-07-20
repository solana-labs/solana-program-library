# Collections

This interface is designed to allow on-chain programs to group tokens
into a `Collection`.

A `Collection` serves to identify members of a published group based on
token mint and metadata.

## Metaplex Protocol

Below is an overview of how Metaplex's Collections work.

In summary, a unique Collection NFT marks a collection, while any members of
the collection (or group) store a pointer within their token metadata that
points to that unique collection.

### The Collection NFT

Metaplex makes use of a "Collection NFT" - which is specified to be an NFT
regardless of whether or not the tokens in the collection are NFTs. This is
because of the unique non-fungible nature of NFTs, which serves as a great
way to make sure Collection accounts are also non-fungible.

- Mint a new SPL Token.
- Create a `Metadata` account for the token.
  - Inside the `Metadata` data schema, populate the `CollectionDetails` field

As we can see, metadata for a mint that serves as a Collection (an NFT) is
the same as any other metadata schema, but makes use of one specific field.

```rust
pub struct Metadata {
    ...
    pub collection_details: Option<CollectionDetails>,  // Configurations marking an NFT as a Collection NFT
    ...
}
```

Taking a closer look at the `CollectionDetails`, we can see it contains
minimal data, but it's presence as `Some()` value instead of `None` is enough
to mark the NFT as a collection - plus the spefication of a `u64` as the
**collection size**, which is the current number of collection members.

```rust
pub enum CollectionDetails {
    V1 {
        size: u64, // Number of collection members
    },
}
```

### A Collection Member

Taking a look at the metadata schema again, we know that if we're working
with a token who's a member of a collection, the configurations will instead
live within `Collection`.

```rust
pub struct Metadata {
    ...
    pub collection: Option<Collection>,     // Configurations marking a token as a member of a collection
    ...
}
```

The data within `Collection` simply points back to the Collection NFT itself
and also specifies whether or not this token is a **verified member** of the
collection it claims to be a member of.

```rust
pub struct Collection {
    pub verified: bool,
    pub key: Pubkey,
}
```

### Verifying a Collection Member

Verifying a token as a member of a collection simply requires the signature
of the mint authority of the Collection NFT to authorize the token in question
as a verified member of the collection.

### Using Both Configs at the Same Time

It's possible for a token (specifically an NFT for Metaplex) to populate
both the `CollectionDetails` and the `Collection` fields in its metadata.
This means you're working with a **nested collection**, where the Collection
NFT with both configs is a member of a **root collection** but also is a
collection itself, with it's own members. Those members would then be sub-members
of the root collection. This can be chained even more.

## SPL Interface

The SPL Token Collections interface works quite similarly to the Metaplex
layout described above, only the configurations don't need to be stored
within a token metadata schema, and the interface itself can't enforce a
collection as an NFT only.

Other than that, the concept of "One-to-Many" relationships works essentially
the same.

> Note: The SPL Collections interface is quite similar to the SPL Editions
> interface. In fact, they only differ in a few subtle ways. More details
> under [Overview](#overview)

### Overview

The SPL Collections and SPL Editions interfaces share much of the same state
and instruction architecture.

- Both interfaces employ a One-to-Many relationship from a parent to its
children.
- Both interfaces leverage the concept of a maximum "supply" or "size"
(`u64`).
- This maximum value, and any other values within the parent state, can only
be changed by an `update_authority`.
- The Emit instruction (view function) can be used to emit either kind of
asset.

However, here's where these interfaces differ:

| SPL Editions | SPL Collections |
| :----------- | :-------------- |
| When a `Reprint` is created from an `Original`, the token metadata of the `Original` is copied and becomes the token metadata of the `Reprint`. | When a `Member` is created (or "registered"), nothing happens with the member's metadata. It can look completely different than the collection's token metadata. |
| SPL discriminators are **the same** for both `Original` and `Reprint` state, thus preventing any account using unique TLV entries from storing both in the same data buffer. | SPL discriminators are **different** for `Collection` and `Member`, thus allowing the inclusion of both state types within a TLV-encoded data buffer (nested collections).

### The Collection

A `Collection` in the SPL Token Collections interface is simply a parent in
the parent-child relationship between collections and members, that's it!
A collection is not enforced to be any NFT or specific token. It can be
any token. Ultimately it's up to the on-chain program implementing the interface
to determine how to create unique collection parents (or whether or not they
want to).

A `Collection` is also fairly decoupled from token metadata. Unlike the SPL
Editions interface, creating members of a collection (children in the parent-
child relationships) does not require the collection's (parent's) token
metadata. This means that **a metadata account is not required to create a
collection**.

Although not enforced by the interface, it's _recommended_ to make the
update authority of a collection the same as the collection's metadata
update authority.

### The Member

The collection `Member` data simply points back to the collection (parent)
that it's associated with.

You'll notice there's no concept of "verified" collection members. Let's
explore why.

Metaplex's protocol ties collection details directly into metadata, which means
anyone can create metadata for their token, and just insert the pointer to
whatever collection they want, since all you need to create metadata for a
token is the signature of the token's mint authority.

With the SPL Token Collections interface, the state data that marks a token
as a member of a collection is merely an arbitrary piece of state with a
pointer, and it's completely decoupled from token metadata. One can choose
to include this bit of state in the same account as a token's metadata, using
TLV-encoded entries, or they can choose to store this in a separate account
altogether.

For this reason, the SPL Token Collections interface has no concept of
"verified" and "unverified" collection members. Instead, the **collection's
mint authority must also sign** any instruction that intends to create a member
of its collection.

### Nested Collections

Since the SPL discriminators for both states (`Collection` and `Member`) are
different, nested collections leveraging the SPL Collections interface work
just the same as Metaplex's nested collections described above.

However, one noteworthy feature of the SPL Collections interface's nested
collections is that one on-chain program can implement the interface, and
another on-chain program can _also_ implement the interface, but create
collections that are sub-collections of the first on-chain program's
collections! (I know, confusing, but awesome).
