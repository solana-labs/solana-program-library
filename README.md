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

## Development

### Environment Setup

1. Install the latest Rust stable from https://rustup.rs/
2. Install Solana v1.6.1 or later from https://docs.solana.com/cli/install-solana-cli-tools
3. Install the `libudev` development package for your distribution (`libudev-dev` on Debian-derived distros, `libudev-devel` on Redhat-derived).

### Build

The normal cargo build is available for building programs against your host machine:
```
$ cargo build
```

To build a specific program, such as SPL Token, for the Solana BPF target:
```
$ cd token/program
$ cargo build-bpf
```

### Test

Unit tests contained within all projects can be run with:
```bash
$ cargo test      # <-- runs host-based tests
$ cargo test-bpf  # <-- runs BPF program tests
```

To run a specific program's tests, such as SPL Token:
```
$ cd token/program
$ cargo test      # <-- runs host-based tests
$ cargo test-bpf  # <-- runs BPF program tests
```

Integration testing may be performed via the per-project .js bindings.  See the
[token program's js project](token/js) for an example.

### Clippy
```bash
$ cargo clippy
```

### Coverage
```bash
$ ./coverage.sh  # Please help! Coverage build currently fails on MacOS due to an XCode `grcov` mismatch...
```

#### MacOS

You may need to pin your grcov version, and then rustup with the apple-darwin nightly toolchain:
```bash
$ cargo install grcov --version 0.6.1
$ rustup toolchain install nightly-x86_64-apple-darwin
```


## Release Process
SPL programs are currently tagged and released manually. Each program is
versioned independently of the others, with all new development occurring on
master. Once a program is tested and deemed ready for release:

### Bump Version

  * Increment the version number in the program's Cargo.toml
  * Generate a new program ID and replace in `<program>/program-id.md` and `<program>/src/lib.rs`
  * Run `cargo build-bpf <program>` to update relevant C bindings. (Note the
    location of the generated `spl_<program>.so` for attaching to the Github
    release.)
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
to test the build. Then run `cargo publish`.
 
 # Disclaimer

All claims, content, designs, algorithms, estimates, roadmaps,
specifications, and performance measurements described in this project
are done with the Solana Foundation's ("SF") best efforts. It is up to
the reader to check and validate their accuracy and truthfulness.
Furthermore nothing in this project constitutes a solicitation for
investment.

Any content produced by SF or developer resources that SF provides, are
for educational and inspiration purposes only. SF does not encourage,
induce or sanction the deployment, integration or use of any such
applications (including the code comprising the Solana blockchain
protocol) in violation of applicable laws or regulations and hereby
prohibits any such deployment, integration or use. This includes use of
any such applications by the reader (a) in violation of export control
or sanctions laws of the United States or any other applicable
jurisdiction, (b) if the reader is located in or ordinarily resident in
a country or territory subject to comprehensive sanctions administered
by the U.S. Office of Foreign Assets Control (OFAC), or (c) if the
reader is or is working on behalf of a Specially Designated National
(SDN) or a person subject to similar blocking or denied party
prohibitions.

The reader should be aware that U.S. export control and sanctions laws
prohibit U.S. persons (and other persons that are subject to such laws)
from transacting with persons in certain countries and territories or
that are on the SDN list. As a project based primarily on open-source
software, it is possible that such sanctioned persons may nevertheless
bypass prohibitions, obtain the code comprising the Solana blockchain
protocol (or other project code or applications) and deploy, integrate,
or otherwise use it. Accordingly, there is a risk to individuals that
other persons using the Solana blockchain protocol may be sanctioned
persons and that transactions with such persons would be a violation of
U.S. export controls and sanctions law. This risk applies to
individuals, organizations, and other ecosystem participants that
deploy, integrate, or use the Solana blockchain protocol code directly
(e.g., as a node operator), and individuals that transact on the Solana
blockchain through light clients, third party interfaces, and/or wallet
software.

