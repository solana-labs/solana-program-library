# Solana Program Library

The Solana Program Library (SPL) is a collection of on-chain programs targeting
the [Sealevel parallel
runtime](https://medium.com/solana-labs/sealevel-parallel-processing-thousands-of-smart-contracts-d814b378192).
These programs are tested against Solana's implementation of Sealevel,
solana-runtime, and some are deployed to Mainnet Beta.  As others implement
Sealevel, we will graciously accept patches to ensure the programs here are
portable across all implementations.

For more information see the [SPL documentation](https://spl.solana.com) and the [Token TypeDocs](https://solana-labs.github.io/solana-program-library/token/js/).

## Deployments

Only a subset of programs within the Solana Program Library repo are deployed to
the Solana Mainnet Beta. Currently, this includes:

| Program | Version |
| --- | --- |
| [token](https://github.com/solana-labs/solana-program-library/tree/master/token/program) | [3.4.0](https://github.com/solana-labs/solana-program-library/releases/tag/token-v3.4.0) |
| [associated-token-account](https://github.com/solana-labs/solana-program-library/tree/master/associated-token-account/program) | [1.1.0](https://github.com/solana-labs/solana-program-library/releases/tag/associated-token-account-v1.1.0) |
| [token-2022](https://github.com/solana-labs/solana-program-library/tree/master/token/program-2022) | [1.0.0](https://github.com/solana-labs/solana-program-library/releases/tag/token-2022-v1.0.0) |
| [governance](https://github.com/solana-labs/solana-program-library/tree/master/governance/program) | [3.1.0](https://github.com/solana-labs/solana-program-library/releases/tag/governance-v3.1.0) |
| [stake-pool](https://github.com/solana-labs/solana-program-library/tree/master/stake-pool/program) | [1.0.0](https://github.com/solana-labs/solana-program-library/releases/tag/stake-pool-v1.0.0) |
| [account-compression](https://github.com/solana-labs/solana-program-library/tree/master/account-compression/programs/account-compression) | [0.1.3](https://github.com/solana-labs/solana-program-library/releases/tag/account-compression-v0.1.3) |
| [shared-memory](https://github.com/solana-labs/solana-program-library/tree/master/shared-memory/program) | [1.0.0](https://github.com/solana-labs/solana-program-library/commit/b40e0dd3fd6c0e509dc1e8dd3da0a6d609035bbd) |
| [feature-proposal](https://github.com/solana-labs/solana-program-library/tree/master/feature-proposal/program) | [1.0.0](https://github.com/solana-labs/solana-program-library/releases/tag/feature-proposal-v1.0.0) |
| [name-service](https://github.com/solana-labs/solana-program-library/tree/master/name-service/program) | [0.3.0](https://github.com/solana-labs/solana-program-library/releases/tag/name-service-v0.3.0) |
| [memo](https://github.com/solana-program/memo/tree/master/program) | [3.0.0](https://github.com/solana-labs/solana-program-library/releases/tag/memo-v3.0.0) |

In addition, one program is planned for deployment to Solana Mainnet Beta:

| Program | Version |
| --- | --- |
| [single-pool](https://github.com/solana-labs/solana-program-library/tree/master/single-pool/program) | [1.0.1](https://github.com/solana-labs/solana-program-library/releases/tag/single-pool-v1.0.1) |

## Audits

Only a subset of programs within the Solana Program Library repo are audited. Currently, this includes:

| Program | Last Audit Date | Version |
| --- | --- | --- |
| [token](https://github.com/solana-labs/solana-program-library/tree/master/token/program) | 2022-08-04 (Peer review) | [4fadd55](https://github.com/solana-labs/solana-program-library/commit/4fadd553e1c549afd1d62aeb5ffa7ef31d1999d1) |
| [associated-token-account](https://github.com/solana-labs/solana-program-library/tree/master/associated-token-account/program) | 2022-08-04 (Peer review) | [c00194d](https://github.com/solana-labs/solana-program-library/commit/c00194d2257302f028f44a403c6dee95c0f9c3bc) |
| [token-2022](https://github.com/solana-labs/solana-program-library/tree/master/token/program-2022) | [2023-11-03](https://github.com/solana-labs/security-audits/blob/master/spl/OtterSecToken2022Audit-2023-11-03.pdf) | [e924132](https://github.com/solana-labs/solana-program-library/tree/e924132d65ba0896249fb4983f6f97caff15721a) |
| [stake-pool](https://github.com/solana-labs/solana-program-library/tree/master/stake-pool/program) | [2023-12-31](https://github.com/solana-labs/security-audits/blob/master/spl/HalbornStakePoolAudit-2023-12-31.pdf) | [a17fffe](https://github.com/solana-labs/solana-program-library/commit/a17fffe70d6cc13742abfbc4a4a375b087580bc1) |
| [account-compression](https://github.com/solana-labs/solana-program-library/tree/master/account-compression/programs/account-compression) | [2022-12-05](https://github.com/solana-labs/security-audits/blob/master/spl/OtterSecAccountCompressionAudit-2022-12-03.pdf) | [6e81794](https://github.com/solana-labs/solana-program-library/commit/6e81794) |
| [shared-memory](https://github.com/solana-labs/solana-program-library/tree/master/shared-memory/program) | [2021-02-25](https://github.com/solana-labs/security-audits/blob/master/spl/KudelskiTokenSwapSharedMemAudit-2021-02-25.pdf) | [b40e0dd](https://github.com/solana-labs/solana-program-library/commit/b40e0dd3fd6c0e509dc1e8dd3da0a6d609035bbd) |
| [single-pool](https://github.com/solana-labs/solana-program-library/tree/master/single-pool/program) | [2024-01-02](https://github.com/solana-labs/security-audits/blob/master/spl/ZellicSinglePoolAudit-2024-01-02.pdf) | [ef44df9](https://github.com/solana-labs/solana-program-library/commit/ef44df985e76a697ee9a8aabb3a223610e4cf1dc) |

All other programs may be updated from time to time. These programs are not
audited, so fork and deploy them at your own risk. Here is the full list of
unaudited programs:

* [binary-option](https://github.com/solana-labs/solana-program-library/tree/master/binary-option/program)
* [binary-oracle-pair](https://github.com/solana-labs/solana-program-library/tree/master/binary-oracle-pair/program)
* [feature-proposal](https://github.com/solana-labs/solana-program-library/tree/master/feature-proposal/program)
* [instruction-padding](https://github.com/solana-labs/solana-program-library/tree/master/instruction-padding/program)
* [managed-token](https://github.com/solana-labs/solana-program-library/tree/master/managed-token/program)
* [name-service](https://github.com/solana-labs/solana-program-library/tree/master/name-service/program)
* [record](https://github.com/solana-labs/solana-program-library/tree/master/record/program)
* [stateless-asks](https://github.com/solana-labs/solana-program-library/tree/master/stateless-asks/program)
* [token-lending](https://github.com/solana-labs/solana-program-library/tree/master/token-lending/program)
* [token-swap](https://github.com/solana-labs/solana-program-library/tree/master/token-swap/program)
* [token-upgrade](https://github.com/solana-labs/solana-program-library/tree/master/token-upgrade/program)

More information about the repository's security policy at
[SECURITY.md](https://github.com/solana-labs/solana-program-library/tree/master/SECURITY.md).

The [security-audits repo](https://github.com/solana-labs/security-audits) contains
all past and present program audits.

## Migrated Packages

The Solana Program Library repository is being broken up into separate repos for
each program and set of clients. The following programs have been moved:

* [Memo](https://github.com/solana-program/memo)

## Program Packages

| Package | Description | Version | Docs |
| :-- | :-- | :--| :-- |
| `spl-token` | ERC20-like token program on Solana | [![Crates.io](https://img.shields.io/crates/v/spl-token)](https://crates.io/crates/spl-token) | [![Docs.rs](https://docs.rs/spl-token/badge.svg)](https://docs.rs/spl-token) |
| `spl-token-2022` | Token program compatible with `spl-token`, with extensions | [![Crates.io](https://img.shields.io/crates/v/spl-token-2022)](https://crates.io/crates/spl-token-2022) | [![Docs.rs](https://docs.rs/spl-token-2022/badge.svg)](https://docs.rs/spl-token-2022) |
| `spl-associated-token-account` | Stateless protocol defining a canonical "associated" token account for a wallet | [![Crates.io](https://img.shields.io/crates/v/spl-associated-token-account)](https://crates.io/crates/spl-associated-token-account) | [![Docs.rs](https://docs.rs/spl-associated-token-account/badge.svg)](https://docs.rs/spl-associated-token-account) |
| `spl-governance` | DAO program using tokens for voting | [![Crates.io](https://img.shields.io/crates/v/spl-governance)](https://crates.io/crates/spl-governance) | [![Docs.rs](https://docs.rs/spl-governance/badge.svg)](https://docs.rs/spl-governance) |
| `spl-account-compression` | Program for managing compressed accounts stored in an off-chain merkle tree | [![Crates.io](https://img.shields.io/crates/v/spl-account-compression)](https://crates.io/crates/spl-account-compression) | [![Docs.rs](https://docs.rs/spl-account-compression/badge.svg)](https://docs.rs/spl-account-compression) |
| `spl-feature-proposal` | Program using tokens to vote on enabling Solana network features | [![Crates.io](https://img.shields.io/crates/v/spl-feature-proposal)](https://crates.io/crates/spl-feature-proposal) | [![Docs.rs](https://docs.rs/spl-feature-proposal/badge.svg)](https://docs.rs/spl-feature-proposal) |
| `spl-noop` | Program that does nothing, used for logging instruction data | [![Crates.io](https://img.shields.io/crates/v/spl-noop)](https://crates.io/crates/spl-noop) | [![Docs.rs](https://docs.rs/spl-noop/badge.svg)](https://docs.rs/spl-noop) |
| `spl-name-service` | Program for managing ownership of data on-chain | [![Crates.io](https://img.shields.io/crates/v/spl-name-service)](https://crates.io/crates/spl-name-service) | [![Docs.rs](https://docs.rs/spl-name-service/badge.svg)](https://docs.rs/spl-name-service) |
| `spl-shared-memory` | Program for sharing data between programs | [![Crates.io](https://img.shields.io/crates/v/spl-shared-memory)](https://crates.io/crates/spl-shared-memory) | [![Docs.rs](https://docs.rs/spl-shared-memory/badge.svg)](https://docs.rs/spl-shared-memory) |
| `spl-stake-pool` | Program for pooling stake accounts, managed by another entity | [![Crates.io](https://img.shields.io/crates/v/spl-stake-pool)](https://crates.io/crates/spl-stake-pool) | [![Docs.rs](https://docs.rs/spl-stake-pool/badge.svg)](https://docs.rs/spl-stake-pool) |
| `spl-instruction-padding` | Program to padding to other instructions | [![Crates.io](https://img.shields.io/crates/v/spl-instruction-padding)](https://crates.io/crates/spl-instruction-padding) | [![Docs.rs](https://docs.rs/spl-instruction-padding/badge.svg)](https://docs.rs/spl-instruction-padding) |
| `spl-concurrent-merkle-tree` | Library for on-chain representation of merkle tree | [![Crates.io](https://img.shields.io/crates/v/spl-concurrent-merkle-tree)](https://crates.io/crates/spl-concurrent-merkle-tree) | [![Docs.rs](https://docs.rs/spl-concurrent-merkle-tree/badge.svg)](https://docs.rs/spl-concurrent-merkle-tree) |
| `spl-math` | Library for on-chain math | [![Crates.io](https://img.shields.io/crates/v/spl-math)](https://crates.io/crates/spl-math) | [![Docs.rs](https://docs.rs/spl-math/badge.svg)](https://docs.rs/spl-math) |
| `spl-token-lending` | Over-collateralized lending program for tokens | [![Crates.io](https://img.shields.io/crates/v/spl-token-lending)](https://crates.io/crates/spl-token-lending) | [![Docs.rs](https://docs.rs/spl-token-lending/badge.svg)](https://docs.rs/spl-token-lending) |
| `spl-token-swap` | AMM for trading tokens | [![Crates.io](https://img.shields.io/crates/v/spl-token-swap)](https://crates.io/crates/spl-token-swap) | [![Docs.rs](https://docs.rs/spl-token-swap/badge.svg)](https://docs.rs/spl-token-swap) |
| `spl-token-upgrade` | Protocol for burning one token type in exchange for another | [![Crates.io](https://img.shields.io/crates/v/spl-token-upgrade)](https://crates.io/crates/spl-token-upgrade) | [![Docs.rs](https://docs.rs/spl-token-upgrade/badge.svg)](https://docs.rs/spl-token-upgrade) |

## CLI Packages

| Package | Description | Version |
| :-- | :-- | :--|
| `spl-token-cli` | CLI for the token, token-2022, and associated-token-account programs | [![Crates.io](https://img.shields.io/crates/v/spl-token-cli)](https://crates.io/crates/spl-token-cli) |
| `spl-stake-pool-cli` | CLI for the stake-pool program | [![Crates.io](https://img.shields.io/crates/v/spl-stake-pool-cli)](https://crates.io/crates/spl-stake-pool-cli) |
| `spl-feature-proposal-cli` | CLI for the feature-proposal program | [![Crates.io](https://img.shields.io/crates/v/spl-feature-proposal-cli)](https://crates.io/crates/spl-feature-proposal-cli) |
| `spl-token-lending-cli` | CLI for the token-lending program | [![Crates.io](https://img.shields.io/crates/v/spl-token-lending-cli)](https://crates.io/crates/spl-token-lending-cli) |
| `spl-token-upgrade-cli` | CLI for the token-upgrade program | [![Crates.io](https://img.shields.io/crates/v/spl-token-upgrade-cli)](https://crates.io/crates/spl-token-upgrade-cli) |

## JavaScript Packages

| Package | Description | Version | Docs |
| :-- | :-- | :--| :-- |
| `@solana/spl-token` | Bindings for the token, token-2022, and associated-token-account programs | [![npm](https://img.shields.io/npm/v/@solana/spl-token.svg)](https://www.npmjs.com/package/@solana/spl-token) | [![Docs](https://img.shields.io/badge/docs-typedoc-blue)](https://solana-labs.github.io/solana-program-library/token/js) |
| `@solana/spl-governance` | Bindings for the governance program | [![npm](https://img.shields.io/npm/v/@solana/spl-governance.svg)](https://www.npmjs.com/package/@solana/spl-governance) | N/A |
| `@solana/spl-account-compression` | Bindings for the account-compression program | [![npm](https://img.shields.io/npm/v/@solana/spl-account-compression.svg)](https://www.npmjs.com/package/@solana/spl-account-compression) | [![Docs](https://img.shields.io/badge/docs-typedoc-blue)](https://solana-labs.github.io/solana-program-library/account-compression/sdk/docs) |
| `@solana/spl-name-service` | Bindings for the name-service program | [![npm](https://img.shields.io/npm/v/@solana/spl-name-service.svg)](https://www.npmjs.com/package/@solana/spl-name-service) | N/A |
| `@solana/spl-stake-pool` | Bindings for the stake-pool program | [![npm](https://img.shields.io/npm/v/@solana/spl-stake-pool.svg)](https://www.npmjs.com/package/@solana/spl-stake-pool) | N/A |
| `@solana/spl-token-lending` | Bindings for the token-lending program | [![npm](https://img.shields.io/npm/v/@solana/spl-token-lending.svg)](https://www.npmjs.com/package/@solana/spl-token-lending) | N/A |
| `@solana/spl-token-swap` | Bindings for the token-swap program | [![npm](https://img.shields.io/npm/v/@solana/spl-token-swap.svg)](https://www.npmjs.com/package/@solana/spl-token-swap) | N/A |

## Development

### Environment Setup

1. Install the latest [Solana tools](https://docs.solana.com/cli/install-solana-cli-tools).
2. Install the latest [Rust stable](https://rustup.rs/). If you already have Rust, run `rustup update` to get the latest version.
3. Install the `libudev` development package for your distribution (`libudev-dev` on Debian-derived distros, `libudev-devel` on Redhat-derived).

### Build

### Build on-chain programs

```bash
# To build all on-chain programs
$ cargo build-sbf

# To build a specific on-chain program
$ cd <program_name>/program
$ cargo build-sbf
```

### Build clients

```bash
# To build all clients
$ cargo build

# To build a specific client
$ cd <program_name>/cli
$ cargo build
```

### Test

Unit tests contained within all projects can be run with:
```bash
$ cargo test      # <-- runs host-based tests
$ cargo test-sbf  # <-- runs BPF program tests
```

To run a specific program's tests, such as SPL Token:
```bash
$ cd token/program
$ cargo test      # <-- runs host-based tests
$ cargo test-sbf  # <-- runs BPF program tests
```

Integration testing may be performed via the per-project .js bindings.  See the
[token program's js project](token/js) for an example.

### Common Issues

Solutions to a few issues you might run into are mentioned here.

1. `Failed to open: ../../deploy/spl_<program-name>.so`

    Update your Rust and Cargo to the latest versions and re-run `cargo build-sbf` in the relevant `<program-name>` directory,
    or run it at the repository root to rebuild all on-chain programs.

2. [Error while loading shared libraries. (libssl.so.1.1)](https://solana.stackexchange.com/q/3029/36)

    A working solution was mentioned [here](https://solana.stackexchange.com/q/3029/36).
    Install libssl.
    ```bash
    wget http://nz2.archive.ubuntu.com/ubuntu/pool/main/o/openssl/libssl1.1_1.1.1l-1ubuntu1.2_amd64.deb
    sudo dpkg -i libssl1.1_1.1.1l-1ubuntu1.2_amd64.deb
    ```

3.  CPU or Memory usage at 100%

    This is to be expected while building some of the programs in this library.
    The simplest solution is to add the `--jobs 1` flag to the build commands to limit the number of parallel jobs to 1 and check if that fixes the issue. Although this will mean much longer build times.


### Clippy
```bash
$ cargo clippy
```

### Coverage
```bash
$ ./coverage.sh  # Help wanted! Coverage build currently fails on MacOS due to an XCode `grcov` mismatch...
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
  * Run `cargo build-sbf <program>` to build binary. Note the
    location of the generated `spl_<program>.so` for attaching to the GitHub
    release.
  * Open a PR with these version changes and merge after passing CI.

### Create GitHub tag

Program tags are of the form `<program>-vX.Y.Z`.
Create the new tag at the version-bump commit and push to the
solana-program-library repository, eg:

```
$ git tag token-v1.0.0 b24bfe7
$ git push upstream --tags
```

### Publish GitHub release

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
are done with the Solana Labs, Inc. (“SL”) best efforts. It is up to
the reader to check and validate their accuracy and truthfulness.
Furthermore nothing in this project constitutes a solicitation for
investment.

Any content produced by SL or developer resources that SL provides, are
for educational and inspiration purposes only. SL does not encourage,
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
that are on the SDN list. Accordingly, there is a risk to individuals 
that other persons using any of the code contained in this repo, or a 
derivation thereof, may be sanctioned persons and that transactions with 
such persons would be a violation of U.S. export controls and sanctions law.
