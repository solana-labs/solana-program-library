# Token JavaScript API

The Token JavaScript library comprises:

* A library to interact with the on-chain program
* A test client that exercises the program
* Scripts to facilitate building the program

## Getting Started

First fetch the npm dependencies, including `@solana/web3.js`, by running:
```bash
$ npm install
```

### Select a Network

The client connects to a local Solana cluster by default.

To enable on-chain program logs, set the `RUST_LOG` environment variable:

```bash
$ export RUST_LOG=solana_runtime::native_loader=trace,solana_runtime::system_instruction_processor=trace,solana_runtime::bank=debug,solana_bpf_loader=debug,solana_rbpf=debug
```

To start a local Solana cluster run:
```bash
$ solana-test-validator
```

Solana cluster logs are available with:
```bash
$ solana --url http://127.0.0.1:8899/ logs
```

### Build the on-chain program

```bash
$ npm run build:program
```

### Run the test client

```bash
$ npm run start
```

## Pointing to a public Solana cluster

Solana maintains three public clusters:
- `devnet` - Development cluster with airdrops enabled
- `testnet` - Tour De Sol test cluster without airdrops enabled
- `mainnet-beta` -  Main cluster

Use npm scripts to configure which cluster.

To point to `devnet`:
```bash
$ npm run cluster:devnet
```

To point back to the local cluster:
```bash
$ npm run cluster:localnet
```

## Releasing

1. (first-time only) Create your account on npmjs.com (with 2FA enabled!) and ask @mvines about granting the publish right and run `npm login`
3. Bump version in `package.json` and `npm install` (to update `package-lock.json`)
4. Create a PR for the version bump
5. Merge the PR and push new git tag on master branch
6. Create release on github.com from the pushed tag
7. Run `npm run build` and `npm publish`
