# Token Group Interface

This interface aims to serve the general purpose of linking tokens together
into groups. A group is defined by the group itself and any members of that
group.

This may sound extremely vauge and generic, and that's the intention!
With this interface, it's possible to create some familiar token groupings
such as NFT Collections and Editions - or new, never before seen relationships
among tokens!

## Motivation

Solana developers and users across the ecosystem are not only familiar with
NFT collections and editions, but they've built successful protocols on these
concepts, making use of these relational structures to build creative
communities and products.

In a world of interface-based programs, this interface aims to provide the
scaffold for creating such relational models, while also being generic enough
and simple enough to not inhibit the potential to scale beyond this mere
framework.

## Structure

### Instructions

#### Initialize Group

This instruction initializes a group. You can see the instruction is generic
over whatever type of group you're intending to set up. This means you can
provide some additional `meta` under the generic argument `G`. You can also
skip the additional `meta` and provide `None`.

#### Update Group Max Size

Update the `max_size` of a group. The interface defines an optional maximum
size for a group, which puts a limit on the total number of members that can
be added to a group.

#### Update Group Authority

Update the update authority for a group. The update authority has the
authorization to modify the group's configurations and thus must also
sign to change the authority. If set to `None`, the group becomes
**immutable**.

#### Initialize Member

Initialize a member of a group.
(Note on generics for members?)

#### Emit

Emits the data of any underlying assets to the program's return data,
thus effectively serving as a view function. Note that you can provide
an enum value for `Group` or `Member` to identify which type of asset is
being emitted using the enum's `u8` value.

### (Optional) State

A program that implements the interface may write the following data fields
into a type-length-value entry into an account:

```rust
type Pubkey = [u8; 32];
type OptionalNonZeroPubkey = Pubkey; // if all zeroes, interpreted as `None`

pub struct Group<G>
where
    G: SplTokenGroup,
{
    /// The authority that can sign to update the group
    pub update_authority: OptionalNonZeroPubkey,
    /// The current number of group members
    pub size: u64,
    /// The maximum number of group members
    pub max_size: Option<u64>,
    /// Additional state
    pub meta: Option<G>,
}

pub struct Member {
    /// The pubkey of the `Group`
    pub group: Pubkey,
    /// The member number
    pub member_number: u64,
}
```

(Note on non-repeating TLV entries?)
