# Quick Start

## Setup Environment

1. Clone the repository from https://github.com/solana-labs/solana-program-library.git
2. Install the latest Solana tools from https://docs.solana.com/cli/install-solana-cli-tools. If you already have Solana tools, run `solana-install update` to get the latest compatible version.
3. Install the latest Rust stable from https://rustup.rs/. If you already have Rust, run `rustup update` to get the latest version.
4. Install the `libudev` development package for your distribution (`libudev-dev` on Debian-derived distros, `libudev-devel` on Redhat-derived).
5. Install the `libsqlite3` development package for your distribution (`libsqlite3-dev` on Debian-derived distros, `sqlite-devel` on Redhat-derived).
6. If you get an error about missing `libssl.so.1.1` file while building on-chain programs, install libssl1.1 package. It is missing in the standard repositories for the most recent versions of Ubuntu, so you might need to install it manually like this:

```
wget http://nz2.archive.ubuntu.com/ubuntu/pool/main/o/openssl/libssl1.1_1.1.1l-1ubuntu1.5_amd64.deb
sudo dpkg -i ./libssl1.1_1.1.1l-1ubuntu1.5_amd64.deb
```

## Build

Before starting the build, set `MAIN_ROUTER_ID` and `MAIN_ROUTER_ADMIN` environment variables. They should point to the existing Main Router program and admin account or generate a new set of keys if you plan to maintain your own version of the reference database:

```
solana-keygen new -o main_admin.json
solana-keygen new -o main_router.json
```

To build all of the off-chain libraries and programs, run the `cargo build` command from the `farms` directory:

```sh
export MAIN_ROUTER_ID="replace this with main router id (solana main_router.json address)"
export MAIN_ROUTER_ADMIN="replace this with admin pubkey (solana main_admin.json address)"
cd solana-program-library/farms
cargo build --release
```

Binaries will be placed under `solana-program-library/farms/target/release` directory. If you plan on using `solana-farm-ctrl` or `solana-farm-client` tools per the examples below, you might want to add the release path to your environment's `PATH` variable:

```sh
pushd target/release
export PATH=$PATH:$(pwd)
popd
```

Alternatively, you can execute instructions directly using [HTTP Client](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/http_client.md) or [Rust Client](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/rust_client.md).

To build on-chain programs, use the standard `cargo build-bpf` command for Solana programs:

```sh
for program in router-*; do
    pushd $program &>/dev/null
    cargo build-bpf
    popd &>/dev/null
done
```

To build Vaults, specify an additional argument that tells the compiler which strategy needs to be built:

```sh
pushd vaults
cargo build-bpf --no-default-features --features SBR-STAKE-LP-COMPOUND
popd
```

Every time you re-build the vault program with another strategy, it overwrites the same file (target/deploy/solana_vaults.so), so you should build and deploy multiple strategies one by one and specify which address to deploy to, e.g.:

```sh
pushd vaults &>/dev/null
for strategy in RDM SBR ORC; do
    solana-keygen new -o vault_$strategy.json
    cargo build-bpf --no-default-features --features $strategy-STAKE-LP-COMPOUND
    solana program deploy --program-id vault_$strategy.json target/deploy/solana_vaults.so
done
popd &>/dev/null
```

## Test

Tests are executed with the `cargo test` command:

```sh
cargo test
```

Integration tests are located in `farm-client/tests` directory and can be started as follows:

```sh
cargo test -- --nocapture --test-threads=1 --ignored
```

Remember that integration tests execute transactions on mainnet, which will cost you some SOL, and require on-chain programs and reference database to be deployed beforehand.

## Deploy

To deploy on-chain programs, use the standard `solana program deploy`:

```sh
solana program deploy target/deploy/solana_router_raydium.so
solana program deploy target/deploy/solana_router_saber.so
solana program deploy target/deploy/solana_router_orca.so
```

If you generated your own set of keys for Main Router ID and Main Router admin, you need to deploy the Main Router program while specifying the corresponding upgrade authority and program id:

```sh
solana program deploy --upgrade-authority main_admin.json --program-id main_router.json target/deploy/solana_router_main.so
```

## Upload Metadata

This project uses an on-chain reference database to store the required metadata. If you plan to maintain your own copy of the database (i.e., generated a new pair of Main Router keys and deployed Main Router), you need to initialize the storage and upload metadata. Otherwise, skip this step.

First, generate PDA addresses for the RefDB indexes:

```sh
solana-farm-ctrl print-pda-all
```

Update `farm-ctrl/metadata/programs/programs.json` with newly generated addresses and addresses of your deployed programs (anything that has a blank address in that file needs to be updated or deleted if not needed).

Initialize the storage (Note: it will cost you about 5 SOL):

```sh
solana-farm-ctrl --keypair main_admin.json init-all
```

Metadata for external protocols, like Raydium, needs to be extracted from relative sources. For convenience, scripts for data download and data itself are available in the `farms/farm-ctrl/metadata` directory.
To upload metadata, run:

```sh
solana-farm-ctrl  -k main_admin.json load --skip-existing Token farm-ctrl/metadata/tokens/solana_token_list/filtered_tokens.json
solana-farm-ctrl  -k main_admin.json load --skip-existing Token farm-ctrl/metadata/pools/raydium/pools.json
solana-farm-ctrl  -k main_admin.json load --skip-existing Token farm-ctrl/metadata/pools/saber/pools.json
solana-farm-ctrl  -k main_admin.json load --skip-existing Token farm-ctrl/metadata/pools/orca/pools.json
solana-farm-ctrl  -k main_admin.json load --skip-existing Token farm-ctrl/metadata/farms/orca/farms.json
solana-farm-ctrl  -k main_admin.json load --skip-existing Pool farm-ctrl/metadata/pools/raydium/pools.json
solana-farm-ctrl  -k main_admin.json load --skip-existing Farm farm-ctrl/metadata/farms/raydium/farms.json
solana-farm-ctrl  -k main_admin.json load --skip-existing Pool farm-ctrl/metadata/pools/saber/pools_and_farms.json
solana-farm-ctrl  -k main_admin.json load --skip-existing Farm farm-ctrl/metadata/farms/saber/pools_and_farms.json
solana-farm-ctrl  -k main_admin.json load --skip-existing Pool farm-ctrl/metadata/pools/orca/pools.json
solana-farm-ctrl  -k main_admin.json load --skip-existing Farm farm-ctrl/metadata/farms/orca/farms.json
```

Metadata has interdependencies, so it needs to be uploaded sequentially as per the list above, don't run it in parallel even for the data of the same type. Metadata upload will cost you some SOL, depending on the number of records. You can get the price per record (Target) and max number of records (Target Max) by running this command:

```sh
solana-farm-ctrl print-size-all
```

To generate metadata for all Raydium Vaults, run:

```sh
./farm-ctrl/metadata/vaults/generate_vaults.py vaults_rdm.json tokens_rdm.json [VAULT_PROG_ID] RDM
```

And then upload it:

```sh
solana-farm-ctrl --keypair main_admin.json load token tokens_rdm.json
solana-farm-ctrl --keypair main_admin.json load vault vaults_rdm.json
```

Similarly, metadata can be generated and uploaded for Orca and Saber Vaults. Also, it is possible to generate metadata only for a single Vault with:

```sh
solana-farm-ctrl --keypair main_admin.json generate Vault [VAULT_PROGRAM_ADDRESS] [VAULT_NAME] [VAULT_TOKEN_NAME]
```

To verify metadata you can run `solana-farm-ctrl list-all vault` or `solana-farm-ctrl get-all vault`.

## Run

After metadata for Vaults and Vault tokens have been uploaded, Vaults need to be initialized with:

```sh
solana-farm-ctrl vault-init all
solana-farm-ctrl vault-enable-deposits all
solana-farm-ctrl vault-enable-withdrawals all
```

And then, you can try one of the client commands to verify the installation:

```sh
solana-farm-client deposit-vault [VAULT_NAME] [TOKEN_A_AMOUNT] 0
solana-farm-client crank-vault [VAULT_NAME] 1
solana-farm-client vault-info [VAULT_NAME]
solana-farm-client swap RDM SOL USDC 0.1
```

For more information see [Vaults](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/vaults.md), for more usage examples see [Rust Client](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/rust_client.md) or [HTTP Client](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/http_client.md).

To use HTTP RPC service, start it with:

```sh
solana-farm-rpc --farm-client-url https://solana-api.projectserum.com --http-rpc-url http://0.0.0.0:9090
```

Note that RPC service should be adequately scaled and put behind a load balancer and HTTPS proxy for production use.

Open http://127.0.0.1:9090 in a browser to see available methods. You can also use [SwaggerHub](https://app.swaggerhub.com/apis-docs/ska22/SolanaFarms/0.1) to call any method interactively. Swagger schema is available in `solana-program-library/farms/farm-rpc/swagger.yaml`.

## Further Steps

Now, when everything is up and running, you may also consider:

- Write [HTTP Client](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/http_client.md) or [Rust Client](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/rust_client.md)
- Initialize a decentralized [Fund](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/fund.md)
- Enable [Multisig](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/multisig.md) for admin operations
- Enable [Governance / DAO](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/governance.md)
