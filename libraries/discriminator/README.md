# SPL Discriminator

This library allows for easy management of 8-byte discriminators.

### The `Discriminator` Struct

With this crate, you can leverage the `Discriminator` type to manage an 8-byte discriminator for generic purposes.

```rust
let my_discriminator = Discriminator::new([8, 5, 1, 56, 10, 53, 9, 198]);
```

The `new(..)` function is also a **constant function**, so you can use `Discriminator` in constants as well.

```rust
const MY_DISCRIMINATOR: Discriminator = Discriminator::new([8, 5, 1, 56, 10, 53, 9, 198]);
```

The `Discriminator` struct also offers another constant function `as_slice(&self)`, so you can use `as_slice()` in constants as well.

```rust
const MY_DISCRIMINATOR_SLICE: &[u8] = MY_DISCRIMINATOR.as_slice();
```

### The `SplDiscriminator` Trait

A trait, `SplDiscriminator` is also available, which will give you the `Discriminator` constant type and also a slice representation of the discriminator. This can be particularly handy with match statements.

```rust
/// A trait for managing 8-byte discriminators in a slab of bytes
pub trait SplDiscriminator {
    /// The 8-byte discriminator as a `[u8; 8]`
    const SPL_DISCRIMINATOR: Discriminator;
    /// The 8-byte discriminator as a slice (`&[u8]`)
    const SPL_DISCRIMINATOR_SLICE: &'static [u8] = Self::SPL_DISCRIMINATOR.as_slice();
}
```

### The `SplDiscriminator` Derive Macro

The `SplDiscriminator` derive macro is a particularly useful tool for those who wish to derive their 8-byte discriminator from a particular string literal. Typically, you would have to run a hash function against the string literal, then copy the first 8 bytes, and then hard-code those bytes into a statement like the one above.

Instead, you can simply annotate a struct or enum with `SplDiscriminator` and provide a **namespace** via the `discriminator_namespace` attribute, and the macro will automatically derive the 8-byte discriminator for you!

```rust
#[derive(SplDiscriminator)] // Implements `SplDiscriminator` for your struct/enum using your declared string literal namespace
#[discriminator_namespace("some_discriminator_namespace")]
pub struct MyInstruction1 {
    arg1: String,
    arg2: u8,
}

let my_discriminator: Discriminator = MyInstruction1::SPL_DISCRIMINATOR;
let my_discriminator_slice: &[u8] = MyInstruction1::SPL_DISCRIMINATOR_SLICE;
```

Note: the 8-byte discriminator derived using the macro is always the **first 8 bytes** of the resulting hashed bytes.
