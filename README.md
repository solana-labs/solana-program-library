[![Build status][travis-image]][travis-url]

[travis-image]:
https://travis-ci.org/solana-labs/solana-program-library.svg?branch=master
[travis-url]: https://travis-ci.org/solana-labs/solana-program-library

# Solana Program Library

The Solana Program Library (SPL) is a collection of on-chain programs targeting
the [Sealevel parallel
runtime](https://medium.com/solana-labs/sealevel-parallel-processing-thousands-of-smart-contracts-d814b378192).
These programs are tested against Solana's implementation of Sealevel,
solana-runtime, and deployed to its mainnet.  As others implement Sealevel, we
will graciously accept patches to ensure the programs here are portable across
all implementations.

Full documentation is available at https://spl.solana.com

## Building

These programs cannot be built directly via cargo and instead require the build
scripts located in Solana's BPF-SDK.

Download or update the BPF-SDK by running:
```bash
$ ./do.sh update
```

To build all programs, run:
```bash
$ ./do.sh build all
```

Or choose a specific program:
```bash
$ ./do.sh build <program>
```

## Testing

Unit tests contained within all projects can be built via:
```bash
$ ./do.sh test all
```

Or:
```bash
$ ./do.sh test <program>
```

End-to-end testing may be performed via the per-project .js bindings.  See the
[token program's js project](token/js) for an example.

## Clippy

Clippy is also supported via:
```bash
$ ./do.sh clippy all
```

Or:
```
$ ./do.sh clippy <program>
```

## Release Process

SPL programs are currently tagged and released manually. Each program is
versioned independently of the others, with all new development occurring on
master. Once a program is tested and deemed ready for release:

### Bump Version

  * Increment the version number in the program's Cargo.toml
  * Generate a new program ID and replace in `<program>/program-id.md` and `<program>/src/lib.rs`
  * Run `./do.sh build <program>` to update relevant C bindings. (Note the location of the generated `spl_<program>.so` for attaching to the Github release.)
  * Open a PR with these version changes and merge after passing CI.

### Create Github tag

Program tags are of the form `<program>-vX.Y.Z`.
Create the new tag at the version-bump commit and push to the
solana-program-library repository, eg:

```
$ git tag token-v1.0.0 b24bfe7
$ git push upstream --tags
```

### Publish Github release

  * Go to [GitHub Releases UI](https://github.com/solana-labs/solana-program-library/releases)
  * Click "Draft new release", and enter the new tag in the "Tag version" box.
  * Title the release "SPL <Program> vX.Y.Z", complete the description, and attach the `spl_<program>.so` binary
  * Click "Publish release"

### Publish to Crates.io

Navigate to the program directory and run `cargo package`
to test the build. Then run `cargo publish`. (Currently, programs with generated
C bindings fail the final step of verification due to the location of the
generated file. Once you are certain the crate is ready, run `cargo publish --no-verify`
to bypass this error.)
