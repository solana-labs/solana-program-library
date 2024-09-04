# Type-Length-Value

Library with utilities for working with Type-Length-Value structures.

## Example usage

This simple examples defines a zero-copy type with its discriminator.

```rust
use {
    bytemuck::{Pod, Zeroable},
    spl_discriminator::{ArrayDiscriminator, SplDiscriminate},
    spl_type_length_value::{
        state::{TlvState, TlvStateBorrowed, TlvStateMut}
    },
};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
struct MyPodValue {
    data: [u8; 32],
}
impl SplDiscriminate for MyPodValue {
    // Give it a unique discriminator, can also be generated using a hash function
    const SPL_DISCRIMINATOR: ArrayDiscriminator = ArrayDiscriminator::new([1; ArrayDiscriminator::LENGTH]);
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
impl SplDiscriminate for MyOtherPodValue {
    // Some other unique discriminator
    const SPL_DISCRIMINATOR: ArrayDiscriminator = ArrayDiscriminator::new([2; ArrayDiscriminator::LENGTH]);
}

// Account will have two sets of `get_base_len()` (8-byte discriminator and 4-byte length),
// and enough room for a `MyPodValue` and a `MyOtherPodValue`
let account_size = TlvStateMut::get_base_len()
    + std::mem::size_of::<MyPodValue>()
    + TlvStateMut::get_base_len()
    + std::mem::size_of::<MyOtherPodValue>()
    + TlvStateMut::get_base_len()
    + std::mem::size_of::<MyOtherPodValue>();

// Buffer likely comes from a Solana `solana_program::account_info::AccountInfo`,
// but this example just uses a vector.
let mut buffer = vec![0; account_size];

// Unpack the base buffer as a TLV structure
let mut state = TlvStateMut::unpack(&mut buffer).unwrap();

// Init and write default value
// Note: you'll need to provide a boolean whether or not to allow repeating
// values with the same TLV discriminator.
// If set to false, this function will error when an existing entry is detected.
// Note the function also returns the repetition number, which can be used to
// fetch the value again.
let (value, _repetition_number) = state.init_value::<MyPodValue>(false).unwrap();
// Update it in-place
value.data[0] = 1;

// Init and write another default value
// This time, we're going to allow repeating values.
let (other_value1, other_value1_repetition_number) =
    state.init_value::<MyOtherPodValue>(true).unwrap();
assert_eq!(other_value1.data, 10);
// Update it in-place
other_value1.data = 2;

// Let's do it again, since we can now have repeating values!
let (other_value2, other_value2_repetition_number) =
    state.init_value::<MyOtherPodValue>(true).unwrap();
assert_eq!(other_value2.data, 10);
// Update it in-place
other_value2.data = 4;

// Later on, to work with it again, we can just get the first value we
// encounter, because we did _not_ allow repeating entries for `MyPodValue`.
let value = state.get_first_value_mut::<MyPodValue>().unwrap();

// Or fetch it from an immutable buffer
let state = TlvStateBorrowed::unpack(&buffer).unwrap();
let value1 = state.get_first_value::<MyOtherPodValue>().unwrap();

// Since we used repeating entries for `MyOtherPodValue`, we can grab either one by
// its repetition number
let value1 = state
    .get_value_with_repetition::<MyOtherPodValue>(other_value1_repetition_number)
    .unwrap();
let value2 = state
    .get_value_with_repetition::<MyOtherPodValue>(other_value2_repetition_number)
    .unwrap();

```

## Motivation

The Solana blockchain exposes slabs of bytes to on-chain programs, allowing program
writers to interpret these bytes and change them however they wish. Currently,
programs interpret account bytes as being only of one type. For example, a token
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

The type is an 8-byte `ArrayDiscriminator`, which can be set to anything.

The length is a little-endian `u32`.

The value is a slab of `length` bytes that can be used however a program desires.

When searching through the buffer for a particular type, the library looks at
the first 8-byte discriminator. If it's all zeroes, this means it's uninitialized.
If not, it reads the next 4-byte length. If the discriminator matches, it returns
the next `length` bytes. If not, it jumps ahead `length` bytes and reads the
next 8-byte discriminator.

## Serialization of variable-length types

The initial example works using the `bytemuck` crate for zero-copy serialization
and deserialization. It's possible to use Borsh by implementing the `VariableLenPack`
trait on your type.

```rust
use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        borsh1::{get_instance_packed_len, try_from_slice_unchecked},
        program_error::ProgramError,
    },
    spl_discriminator::{ArrayDiscriminator, SplDiscriminate},
    spl_type_length_value::{
        state::{TlvState, TlvStateMut},
        variable_len_pack::VariableLenPack
    },
};
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize)]
struct MyVariableLenType {
    data: String, // variable length type
}
impl SplDiscriminate for MyVariableLenType {
    const SPL_DISCRIMINATOR: ArrayDiscriminator = ArrayDiscriminator::new([5; ArrayDiscriminator::LENGTH]);
}
impl VariableLenPack for MyVariableLenType {
    fn pack_into_slice(&self, dst: &mut [u8]) -> Result<(), ProgramError> {
        borsh::to_writer(&mut dst[..], self).map_err(Into::into)
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        try_from_slice_unchecked(src).map_err(Into::into)
    }

    fn get_packed_len(&self) -> Result<usize, ProgramError> {
        get_instance_packed_len(self).map_err(Into::into)
    }
}
let initial_data = "This is a pretty cool test!";
// Allocate exactly the right size for the string, can go bigger if desired
let tlv_size = 4 + initial_data.len();
let account_size = TlvStateMut::get_base_len() + tlv_size;

// Buffer likely comes from a Solana `solana_program::account_info::AccountInfo`,
// but this example just uses a vector.
let mut buffer = vec![0; account_size];
let mut state = TlvStateMut::unpack(&mut buffer).unwrap();

// No need to hold onto the bytes since we'll serialize back into the right place
// For this example, let's _not_ allow repeating entries.
let _ = state.alloc::<MyVariableLenType>(tlv_size, false).unwrap();
let my_variable_len = MyVariableLenType {
    data: initial_data.to_string()
};
state.pack_first_variable_len_value(&my_variable_len).unwrap();
let deser = state.get_first_variable_len_value::<MyVariableLenType>().unwrap();
assert_eq!(deser, my_variable_len);
```
