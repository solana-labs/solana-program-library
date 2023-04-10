# Solana Perpetuals

## Introduction

Solana Perpetuals protocol is an open-source implementation of a non-custodial decentralized exchange that supports leveraged trading in a variety of assets.

## Quick start

### Setup Environment

1. Clone the repository from <https://github.com/askibin/perpetuals.git>.
2. Install the latest Solana tools from <https://docs.solana.com/cli/install-solana-cli-tools>. If you already have Solana tools, run `solana-install update` to get the latest compatible version.
3. Install the latest Rust stable from <https://rustup.rs/>. If you already have Rust, run `rustup update` to get the latest version.
4. Install the latest Anchor framework from <https://www.anchor-lang.com/docs/installation>. If you already have Anchor, run `avm update` to get the latest version.

Rustfmt is used to format the code. It requires `nightly` features to be activated:

5. Install `nightly` rust toolchain. <https://rust-lang.github.io/rustup/installation/index.html#installing-nightly>
6. Execute `git config core.hooksPath .githooks` to activate pre-commit hooks.

#### [Optional] Vscode setup

1. Install `rust-analyzer` extension
2. If formatting doesn't work, make sure that `rust-analyzer.rustfmt.extraArgs` is set to `+nightly`

### Build

First, generate a new key for the program address with `solana-keygen new -o <PROG_ID_JSON>`. Then replace the existing program ID with the newly generated address in `Anchor.toml` and `programs/perpetuals/src/lib.rs`.

Also, ensure the path to your wallet in `Anchor.toml` is correct. Alternatively, when running Anchor deploy or test commands, you can specify your wallet with `--provider.wallet` argument. The wallet's pubkey will be set as an upgrade authority upon initial deployment of the program. It is strongly recommended to make upgrade authority a multisig when deploying to the mainnet.

To build the program run `anchor build` command from the `perpetuals` directory:

```sh
cd perpetuals
anchor build
```

### Test

Unit tests are executed with the `cargo test` command:

```sh
cargo test -- --nocapture
```

Integration tests (Rust) can be started as follows:

```sh
cargo test-bpf -- --nocapture
```

Integration tests (Typescript) can be started as follows:

```sh
npm install
anchor test -- --features test
```

By default, integration tests are executed on a local validator, so it won't cost you any SOL.

### Deploy

To deploy the program to the devnet and upload the IDL use the following commands:

```sh
anchor deploy --provider.cluster devnet --program-keypair <PROG_ID_JSON>
anchor idl init --provider.cluster devnet --filepath ./target/idl/perpetuals.json <PROGRAM ID>
```

### Initialize

A small CLI Typescript client is included to help you initialize and manage the program. By default script uses devnet cluster. Add `-u https://api.mainnet-beta.solana.com` to all of the commands if you plan to execute them on mainnet.

To initialize deployed program, run the following commands:

```
cd app
npm install
npm install -g npx
npx ts-node src/cli.ts -k <ADMIN_WALLET> init --min-signatures <int> <ADMIN_PUBKEY1> <ADMIN_PUBKEY2> ...
```

Where `<ADMIN_WALLET>` is the file path to the wallet that was set as the upgrade authority of the program upon deployment. `<ADMIN_PUBKEY1>`, `<ADMIN_PUBKEY2>` etc., will be set as protocol admins, and `min-signatures` will be required to execute privileged instructions. To provide multiple signatures, just execute exactly the same command multiple times specifying different `<ADMIN_WALLET>` with `-k` option. The intermediate state is recorded on-chain so that commands can be executed on different computers.

To change protocol admins or minimum required signatures, run:

```
npx ts-node src/cli.ts -k <ADMIN_WALLET> set-authority --min-signatures <int> <ADMIN_PUBKEY1> <ADMIN_PUBKEY2> ...
```

To validate initialized program:

```
npx ts-node src/cli.ts -k <ADMIN_WALLET> get-multisig
npx ts-node src/cli.ts -k <ADMIN_WALLET> get-perpetuals
```

Before the program can accept any liquidity or open a trade, you need to create a token pool and add one or more token custodies to it:

```
npx ts-node src/cli.ts -k <ADMIN_WALLET> add-pool <POOL_NAME>
npx ts-node src/cli.ts -k <ADMIN_WALLET> add-custody <POOL_NAME> <TOKEN_MINT> <TOKEN_ORACLE> <IS_STABLE>
```

Where `<POOL_NAME>` is a random name you want to assign to the pool, `<TOKEN_MINT>` is the mint address of the token, and `<TOKEN_ORACLE>` is the corresponding Pyth price account that can be found on [this page](https://pyth.network/price-feeds?cluster=devnet). `<IS_STABLE>` specifies whether the custody is for a stablecoin. For example:

```
npx ts-node src/cli.ts -k <ADMIN_WALLET> add-pool TestPool1
npx ts-node src/cli.ts -k <ADMIN_WALLET> add-custody TestPool1 So11111111111111111111111111111111111111112 J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix false
```

To validate added pools and custodies, run:

```
npx ts-node src/cli.ts -k <ADMIN_WALLET> get-pool <POOL_NAME>
npx ts-node src/cli.ts -k <ADMIN_WALLET> get-custody <POOL_NAME> <TOKEN_MINT>
```

or

```
npx ts-node src/cli.ts -k <ADMIN_WALLET> get-pools
npx ts-node src/cli.ts -k <ADMIN_WALLET> get-custodies <POOL_NAME>
```

CLI offers other useful commands. You can get the list of all of them by running the following:

```
npx ts-node src/cli.ts --help
```

## UI

We have implemented a coressponding UI for the smartcontract, written in Typescript/Tailwind/Next. To quickly spin up a UI linked to the contract, first follow the previous directions to build the contract, and to init the exchange.

In the main directory, run `./migrations/migrate-target.sh` to copy over the target build directory to the ui.

Now, you can use the following CLI commands to quickly spin-up a `TestPool1` consisting of the three following tokens.

Sol Token: `J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix`

Test Token oracle: `BLArYBCUYhdWiY8PCUTpvFE21iaJq85dvxLk9bYMobcU`

USDC oracle: `5SSkXsEKQepHHAewytPVwdej4epN1nxgLVM84L4KXgy7`

```
cd app

npx ts-node src/cli.ts -k <ADMIN_WALLET> add-pool TestPool1

npx ts-node src/cli.ts -k <ADMIN_WALLET> add-custody TestPool1 So11111111111111111111111111111111111111112 J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix

npx ts-node src/cli.ts -k <ADMIN_WALLET> add-custody TestPool1 6QGdQbaZEgpXqqbGwXJZXwbZ9xJnthfyYNZ92ARzTdAX BLArYBCUYhdWiY8PCUTpvFE21iaJq85dvxLk9bYMobcU

npx ts-node src/cli.ts -k <ADMIN_WALLET> add-custody TestPool1 Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr 5SSkXsEKQepHHAewytPVwdej4epN1nxgLVM84L4KXgy7 true
```

Now, use the following commands to build and run the UI, (navigate to localhost:3000 to use the UI):

```
cd ../ui
yarn install
yarn dev
```

## Support

If you are experiencing technical difficulties while working with the Perpetuals codebase, ask your question on [StackExchange](https://solana.stackexchange.com) (tag your question with `perpetuals`).

If you find a bug in the code, you can raise an issue on [Github](https://github.com/askibin/perpetuals). But if this is a security issue, please don't disclose it on Github or in public channels. Send information to solana.farms@protonmail.com instead.

## Contributing

Contributions are very welcome. Please refer to the [Contributing](https://github.com/solana-labs/solana/blob/master/CONTRIBUTING.md) guidelines for more information.

## License

Solana Perpetuals codebase is released under [Apache License 2.0](LICENSE).

## Disclaimer

By accessing or using Solana Perpetuals or any of its components, you accept and agree with the [Disclaimer](DISCLAIMER.md).
