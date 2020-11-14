# Ouroboros

[![Ouroboros on Crates.IO](https://img.shields.io/crates/v/ouroboros)](https://crates.io/crates/ouroboros)
[![Documentation](https://img.shields.io/badge/documentation-link-success)](https://docs.rs/ouroboros)


Easy self-referential struct generation for Rust. 
Dual licensed under MIT / Apache 2.0.

Note: as of September 2019, there is a [limitation in Rust's type checker](https://users.rust-lang.org/t/why-does-this-not-compile-box-t-target-t/49027/7?u=aaaaa)
which prevents structs with chained references from compiling properly. (E.G. you cannot have a 
struct where field C refers to field B which refers to field A.) Refer to the documentation on
[chain_hack](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#using-chain_hack) 
for a workaround for this problem.

Tests are located in the examples/ folder because they need to be in a crate outside of `ouroboros`
for the `self_referencing` macro to work properly.

```rust
use ouroboros::self_referencing;

#[self_referencing]
struct MyStruct {
    int_data: Box<i32>,
    float_data: Box<f32>,
    #[borrows(int_data)]
    int_reference: &'this i32,
    #[borrows(mut float_data)]
    float_reference: &'this mut f32,
}

fn main() {
    let mut my_value = MyStructBuilder {
        int_data: Box::new(42),
        float_data: Box::new(3.14),
        int_reference_builder: |int_data: &i32| int_data,
        float_reference_builder: |float_data: &mut f32| float_data,
    }.build();

    // Prints 42
    println!("{:?}", my_value.with_int_data_contents(|int_data| *int_data));
    // Prints 3.14
    println!("{:?}", my_value.with_float_reference(|float_reference| **float_reference));
    // Sets the value of float_data to 84.0
    my_value.with_mut(|fields| {
        **fields.float_reference = (**fields.int_reference as f32) * 2.0;
    });

    // We can hold on to this reference...
    let int_ref = my_value.with_int_reference(|int_ref| *int_ref);
    println!("{:?}", *int_ref);
    // As long as the struct is still alive.
    drop(my_value);
    // This will cause an error!
    // println!("{:?}", *int_ref);
}
```
