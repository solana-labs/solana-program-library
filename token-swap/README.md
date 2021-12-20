# Token Swap Program

A Uniswap-like exchange for the Token program on the Solana blockchain, deployed
to `SwaPpA9LAaLfeLi3a68M4DjnLqgtticKg6CnyNwgAC8` on all networks.

Full documentation is available at https://spl.solana.com/token-swap

JavaScript bindings are available in the `./js` directory.

## Building

To build a development version of the Token Swap program, you can use the normal
build command for Solana programs:

```sh
cargo build-bpf
```

For production versions, the Token Swap Program contains a `production` feature
to fix constraints on fees and fee account owner. A developer can
deploy the program, allow others to create pools, and earn a "protocol fee" on
all activity.

Since Solana programs cannot contain any modifiable state, we must hard-code
all constraints into the program.  `SwapConstraints` in `program/src/constraints.rs`
contains all hard-coded fields for fees.  Additionally the
`SWAP_PROGRAM_OWNER_FEE_ADDRESS` environment variable specifies the public key
that must own all fee accounts.

You can build the production version of Token Swap running on devnet, testnet, and
mainnet-beta using the following command:

```sh
SWAP_PROGRAM_OWNER_FEE_ADDRESS=HfoTxFR1Tm6kGmWgYWD6J7YHVy1UwqSULUGVLXkJqaKN cargo build-bpf --features=production
```

## Testing

### Unit tests

Run unit tests from `./program/` using:

```sh
cargo test
```

### Fuzz tests

Using the Rust version of `honggfuzz`, we "fuzz" the Token Swap program every night.
Install `honggfuzz` with:

```sh
cargo install honggfuzz
```

From there, run fuzzing from `./program/fuzz` with:

```sh
cargo hfuzz run token-swap-instructions
```

If the program crashes or errors, `honggfuzz` dumps a `.fuzz` file in the workspace,
so you can debug the failing input using:

```sh
cargo hfuzz run-debug token-swap-instructions hfuzz_workspace/token-swap-instructions/*fuzz
```

This command attaches a debugger to the test, allowing you to easily see the
exact problem.

### Integration tests

You can test the JavaScript bindings and on-chain interactions using
`solana-test-validator`, included in the Solana Tool Suite.  See the
[CLI installation instructions](https://docs.solana.com/cli/install-solana-cli-tools).

From `./js`, install the required modules:

```sh
npm i
```

Then run all tests:

```sh
npm run start-with-test-validator
```

If you are testing a production build, use:

```sh
SWAP_PROGRAM_OWNER_FEE_ADDRESS="HfoTxFR1Tm6kGmWgYWD6J7YHVy1UwqSULUGVLXkJqaKN" npm run start-with-test-validator
```
