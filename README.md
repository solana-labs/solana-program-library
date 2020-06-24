[![Build status][travis-image]][travis-url]

[travis-image]: https://travis-ci.org/solana-labs/solana-program-library.svg?branch=master
[travis-url]: https://travis-ci.org/solana-labs/solana-program-library

# Solana Program Library

The Solana Program Library (SPL) is a collection of on-chain programs targeting
the [Sealevel parallel runtime](https://medium.com/solana-labs/sealevel-parallel-processing-thousands-of-smart-contracts-d814b378192).
These programs are tested against Solana's implementation
of Sealevel, solana-runtime, and deployed to its mainnet.  As others implement
Sealevel, we will graciously accept patches to ensure the programs here are
portable across all implementations.

## Building

These programs cannot be built directly via cargo and instead require the build scripts located in Solana's BPF-SDK.

Download or update the BPF-SDK by running:
```bash
$ ./do.sh update
```

To build all programs, run:
```bash
$ ./do.sh build
```

Or choose a specific program:
```bash
$ ./do.sh build <program>
```

## Testing

Unit tests contained within all projects can be built via:
```bash
$ ./do.sh test
```

Or:
```bash
$ ./do.sh test <program>
```

End-to-end testing may be performed via the per-project .js bindings.  See the [token program's js project](token/js) for an example.

## Clippy

Clippy is also supported via:
```bash
$ ./do.sh clippy
```

Or:
```
$ ./do.sh clippy <program>
```
