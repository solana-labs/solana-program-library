# SPL Token program command-line utility

A basic command-line for creating and using SPL Tokens.  See https://spl.solana.com/token for more details

## Build

To build the CLI locally, simply run:

```sh
cargo build
```

## Testing

The tests require locally built programs for Token, Token-2022, and Associated
Token Account. To build these, you can run:

```sh
BUILD_DEPENDENT_PROGRAMS=1 cargo build
```

This method uses the local `build.rs` file, which can be error-prone, so alternatively,
you can build the programs by running the following commands from this directory:

```sh
cargo build-sbf --manifest-path ../program/Cargo.toml
cargo build-sbf --manifest-path ../program-2022/Cargo.toml
cargo build-sbf --manifest-path ../../associated-token-account/program/Cargo.toml
```

After that, you can run the tests as any other Rust project:

```sh
cargo test
```
