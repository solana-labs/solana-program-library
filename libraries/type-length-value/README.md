# Type-Length-Value

Library with utilities for working with Type-Length-Value structures.

## Example usage

This simple examples defines a zero-copy type with its discriminator.

```rust
use {
    borsh::{BorshSerialize, BorshDeserialize},
    bytemuck::{Pod, Zeroable},
    spl_type_length_value::{
        discriminator::{Discriminator, TlvDiscriminator},
        state::{TlvState, TlvStateBorrowed, TlvStateMut}
    },
};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
struct MyPodValue {
    data: [u8; 32],
}
impl TlvDiscriminator for MyPodValue {
    // Give it a unique discriminator, can also be generated using a hash function
    const TLV_DISCRIMINATOR: Discriminator = Discriminator::new([1; Discriminator::LENGTH]);
}
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
struct MyOtherPodValue {
    data: u8,
}
// Give this type a non-derivable implementation of `Default` to write some data
impl Default for MyOtherPodValue {
    fn default() -> Self {
        Self {
            data: 10,
        }
    }
}
impl TlvDiscriminator for MyOtherPodValue {
    // Some other unique discriminator
    const TLV_DISCRIMINATOR: Discriminator = Discriminator::new([2; Discriminator::LENGTH]);
}

// Account will have two sets of `get_base_len()` (8-byte discriminator and 4-byte length),
// and enough room for a `MyPodValue` and a `MyOtherPodValue`
let account_size = TlvState::get_base_len() + std::mem::size_of::<MyPodValue>() + \
    TlvState::get_base_len() + std::mem::size_of::<MyOtherPodValue>();

// Buffer likely comes from a Solana `solana_program::account_info::AccountInfo`,
// but this example just uses a vector.
let mut buffer = vec![0; account_size];

// Unpack the base buffer as a TLV structure
let mut state = TlvStateMut::unpack(&mut buffer).unwrap();

// Init and write default value
let value = state.init_value::<MyPodValue>().unwrap();
// Update it in-place
value.data[0] = 1;

// Init and write another default value
let other_value = state.init_value::<MyOtherPodValue>().unwrap();
assert_eq!(other_value.data, 10);
// Update it in-place
other_value.data = 2;

// Later on, to work with it again
let value = state.get_value_mut::<MyPodValue>().unwrap();

// Or fetch it from an immutable buffer
let state = TlvStateBorrowed::unpack(&buffer).unwrap();
let value = state.get_value::<MyOtherPodValue>().unwrap();
```

## Motivation

The Solana blockchain exposes slabs of bytes to on-chain programs, allowing program
writers to intepret these bytes and change them however they wish. Currently,
programs interpet account bytes as being only of one type. For example, an token
mint account is only ever a token mint, an AMM pool account is only ever an AMM pool,
a token metadata account can only hold token metadata, etc.

In a world of interfaces, a program will likely implement multiple interfaces.
As a concrete and important example, imagine a token program where mints hold
their own metadata. This means that a single account can be both a mint and
metadata.

To allow easy implementation of multiple interfaces, accounts must be able to
hold multiple different types within one opaque slab of bytes. The
[type-length-value](https://en.wikipedia.org/wiki/Type%E2%80%93length%E2%80%93value)
scheme facilitates this exact case.

## How it works

This library allows for holding multiple disparate types within the same account
by encoding the type, then length, then value.

The type is an 8-byte `Discriminator`, which can be set to anything.

The length is a little-endian `u32`.

The value is a slab of `length` bytes that can be used however a program desires.

When searching through the buffer for a particular type, the library looks at
the first 8-byte discriminator. If it's all zeroes, this means it's uninitialized.
If not, it reads the next 4-byte length. If the discriminator matches, it returns
the next `length` bytes. If not, it jumps ahead `length` bytes and reads the
next 8-byte discriminator.

## Borsh integration

The initial example works using the `bytemuck` crate for zero-copy serialization
and deserialization. It's possible to use Borsh by activating the `borsh` feature.

```rust
use {
    borsh::{BorshDeserialize, BorshSerialize},
    spl_type_length_value::state::{TlvState, TlvStateMut},
};
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize)]
struct MyBorsh {
    data: String, // variable length type
}
impl TlvDiscriminator for MyBorsh {
    const TLV_DISCRIMINATOR: Discriminator = Discriminator::new([5; Discriminator::LENGTH]);
}
let initial_data = "This is a pretty cool test!";
// Allocate exactly the right size for the string, can go bigger if desired
let tlv_size = 4 + initial_data.len();
let account_size = TlvState::get_base_len() + tlv_size;

// Buffer likely comes from a Solana `solana_program::account_info::AccountInfo`,
// but this example just uses a vector.
let mut buffer = vec![0; account_size];
let mut state = TlvStateMut::unpack(&mut buffer).unwrap();

// No need to hold onto the bytes since we'll serialize back into the right place
let _ = state.allocate::<MyBorsh>(tlv_size).unwrap();
let my_borsh = MyBorsh {
    data: initial_data.to_string()
};
state.borsh_serialize(&my_borsh).unwrap();
let deser = state.borsh_deserialize::<MyBorsh>().unwrap();
assert_eq!(deser, my_borsh);
```
