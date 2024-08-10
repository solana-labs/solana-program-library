## Token-Group Interface

An interface describing the instructions required for a program to implement
to be considered a "token-group" program for SPL token mints. The interface can
be implemented by any program.

With a common interface, any wallet, dapp, or on-chain program can read the
group or member configurations, and any tool that creates or modifies group
or member configurations will just work with any program that implements the
interface.

This interface is compatible with any program that implements the SPL Token
interface. However, other program implementations that are not SPL Token
programs may still be compatible with an SPL Token Group program should that
program's token standard support the proper components such as mint and mint
authority accounts (see [Required Instructions](#required-instructions)).

There are also structs for `TokenGroup` and `TokenGroupMember` that may
optionally be implemented, but are not required.

### Example program

An example program demonstrating how to implement the SPL Token-Group Interface
can be found in the
[example](https://github.com/solana-labs/solana-program-library/tree/master/token-group/example)
directory alongside this interface's directory.

In addition to demonstrating what a token-group program might look like, it
also provides some reference examples for using the SPL Type Length Value
library to manage TLV-encoded data within account data.

For more information on SPL Type Length Value you can reference the library's
[source code](https://github.com/solana-labs/solana-program-library/tree/master/libraries/type-length-value).

### Motivation

As developers have engineered more creative ways to customize tokens and use
them to power applications, communities, and more, the reliance on tokens that
are intrinsically related through on-chain mapping has continued to strengthen.

Token-group provides developers with the minimum necessary interface components
required to create these relational mappings, allowing for reliable
composability as well as the freedom to customize these groups of tokens
however one might please.

By implementing token-group, on-chain programs can build brand-new kinds of
token groups, which can all overlap with each other and share common tooling.

### Required Instructions

All of the following instructions are listed in greater detail in the source code.

- [`InitializeGroup`](https://github.com/solana-labs/solana-program-library/blob/master/token-group/interface/src/instruction.rs#L22)
- [`UpdateGroupMaxSize`](https://github.com/solana-labs/solana-program-library/blob/master/token-group/interface/src/instruction.rs#L33)
- [`UpdateGroupAuthority`](https://github.com/solana-labs/solana-program-library/blob/master/token-group/interface/src/instruction.rs#L42)
- [`InitializeMember`](https://github.com/solana-labs/solana-program-library/blob/master/token-group/interface/src/instruction.rs#L51)

#### Initialize Group

Initializes a token-group TLV entry in an account for group configurations with
a provided maximum group size and update authority.

Must provide an SPL token mint and be signed by the mint authority.

#### Update Group Max Size

Updates the maximum size limit of a group.

Must be signed by the update authority.

#### Update Group Authority

Sets or unsets the token-group update authority, which signs any future updates
to the group configurations.

Must be signed by the update authority.

#### Initialize Member

Initializes a token-group TLV entry in an account for group member
configurations.

Must provide an SPL token mint for both the group and the group member.

Must be signed by the member mint's mint authority _and_ the group's update
authority.

### (Optional) State

A program that implements the interface may write the following data fields
into a type-length-value entry into an account. Note the type discriminants
for each.

For a group:

```rust
type OptionalNonZeroPubkey = Pubkey; // if all zeroes, interpreted as `None`
type PodU64 = [u8; 8];
type Pubkey = [u8; 32];

/// Type discriminant: [214, 15, 63, 132, 49, 119, 209, 40]
/// First 8 bytes of `hash("spl_token_group_interface:group")`
pub struct TokenGroup {
    /// The authority that can sign to update the group
    pub update_authority: OptionalNonZeroPubkey,
    /// The associated mint, used to counter spoofing to be sure that group
    /// belongs to a particular mint
    pub mint: Pubkey,
    /// The current number of group members
    pub size: PodU64,
    /// The maximum number of group members
    pub max_size: PodU64,
}
```

For a group member:

```rust
/// Type discriminant: [254, 50, 168, 134, 88, 126, 100, 186]
/// First 8 bytes of `hash("spl_token_group_interface:member")`
pub struct TokenGroupMember {
    /// The associated mint, used to counter spoofing to be sure that member
    /// belongs to a particular mint
    pub mint: Pubkey,
    /// The pubkey of the `TokenGroup`
    pub group: Pubkey,
    /// The member number
    pub member_number: PodU64,
}
```

By storing the configurations for either groups or group members in a TLV
structure, a developer who implements this interface can freely add any other
data fields in a different TLV entry.

As mentioned previously, you can find more information about
TLV / type-length-value structures at the
[spl-type-length-value repo](https://github.com/solana-labs/solana-program-library/tree/master/libraries/type-length-value).