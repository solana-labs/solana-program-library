# Solana Yield Farming

## Introduction

Solana Yield Farming is a set of easy-to-use tools and blockchain contracts for yield optimization strategies.

It is powered by Solana blockchain to allow for frequent automatic compounding, staking, and rebalancing.

One of the distinct features of this platform is the On-chain Reference Database. Metadata for all objects: Tokens, Pools, Farms, Vaults, etc., is stored in the blockchain, so clients don't need any state or hard-coded data.

Solana Yield Farming provides an unified interface to Vaults, regular AMM Pools, Farms, and basic operations on tokens and accounts. Currently, Raydium, Saber, and Orca protocols are supported, but others are under development.

This source code is an example that third parties can utilize to create and use their own version of a yield farming or aggregation service.

### Farm Client

A Rust library that can be used by off-chain programs to interact with Routers, Vaults, perform admin operations, metadata queries, and some common operations with wallets and accounts.

The Client's methods accept human-readable names (tokens, polls, etc.) and UI (decimal) amounts, so you can simply call:

```rust
client.swap(&keypair, "RDM", "SOL", "RAY", 0.1);
client.add_liquidity(&keypair, "RDM.RAY-SOL", 0.0, 0.1);
client.stake(&keypair, "RDM.RAY-SOL", 0.0);
client.harvest(&keypair, "RDM.RAY-SOL");
```

to swap 0.1 SOL for RAY, deposit RAY and SOL to a Raydium pool, stake LP tokens, and harvest rewards. `RDM` above is a route or protocol, use `SBR` for Saber pools, and `ORC` for Orca. All metadata required to lookup account addresses, decimals, etc., is stored on-chain. The Client also allows building raw unsigned instructions to be integrated into more complex workflows. See `farms/farm-client/src/client.rs` for examples.

The Client caches metadata to make subsequent calls faster, most noticeably queries that list a large number of objects, like `get_pools()`.

In addition to the library, there is also a command-line tool that sets an example for basic usage. See `solana-farm-client --help` for the details.

### Farm RPC

A JSON RPC service that can be used to export Farm Client's functionality over HTTP. Most of the Client's functions can then be called from any language that supports HTTP requests, used with the SwaggerHub or tools like curl:

```sh
curl 'http://127.0.0.1:9090/api/v1/token_account_balance?wallet_address=9wsC5hx5JopG5VwoDiUGrvgM2NaYz6tS3uyhuneRKgcN&token_name=RAY'
```

It is also possible to perform POST requests to interact with liquidity pools, farms, and vaults, but you must include a keypair. To avoid that and sign a transaction locally with a wallet app, you can query for a plain instruction in JSON, convert it, sign and send. Here is how it can be done in Javascript with Phantom:

```js
const json_data = await (
  await fetch(
    "http://127.0.0.1:9090/api/v1/new_instruction_add_liquidity_vault?wallet_address=9wsC5hx5JopG5VwoDiUGrvgM2NaYz6tS3uyhuneRKgcN&vault_name=RDM.STC.RAY-SRM&max_token_a_ui_amount=0.1&max_token_b_ui_amount=0.0"
  )
).json();

json_data.accounts.forEach(function (item, index) {
  let acc = {
    isSigner: item.is_signer ? true : false,
    isWritable: item.is_writable ? true : false,
    pubkey: new PublicKey(item.pubkey),
  };
  accounts.push(acc);
});

const instruction = new TransactionInstruction({
  programId: new PublicKey(json_data.program_id),
  data: json_data.data,
  keys: accounts,
});

let transaction = new Transaction({
  recentBlockhash: (await this.connection.getRecentBlockhash()).blockhash,
  feePayer: this.state.provider.publicKey,
});
transaction.add(instruction);

let signed = await this.state.provider.signTransaction(transaction);
let signature = await this.connection.sendRawTransaction(signed.serialize());
```

No additional JS bindings or other dependencies are required for the above to work besides standard @solana/web3.js.

Note that RPC service should be adequately scaled and put behind a load balancer and HTTPS proxy for production use.

### Farm SDK

A Rust library with a common code that is used by all Yield Farming tools and contracts. In addition to account management functionsб it includes definitions for all native and external protocols instructions and metadata objects.

### Farm Ctrl

A command-line tool for on-chain data management (init/upload/delete/lookup) and vaults control (init/enable/disable/set parameters etc). It can also generate metadata for Vaults and Vault tokens. Metadata for external protocols, like Raydium, needs to be extracted from relative sources. While such tools are not included, you can find target format examples in the `farm-ctrl/src/metadata` folder.

### Vaults

A Vault contract implementation. Individual yield farming strategies are stored under the `strategies` sub-folder. `RDM-STAKE-LP-COMPOUND` strategy works as follows:

- User deposits tokens into a Vault with add_liquidity transaction. For example, Vault `RDM.STC.RAY-SRM` takes RAY and SRM tokens. To get a list of available Vaults, one can use the `client.get_vaults()` function or `api/v1/vaults` RPC call. Vault tokens are minted back to the user to represent their share in the Vault.
- Vault sends user tokens to the corresponding Raydium Pool, receives LP tokens, and stakes them to the Raydium Farm.
- Vaults should be cranked on a periodic basis. Crank operation is permissionless and can be done by anyone. And it is executed for the entire Vault, not per individual user. Crank consists of three steps: 1. Harvest Farm rewards (in one or both tokens); 2. Rebalance rewards to get proper amounts of each token; 3. Place rewards back into the Pool and stake received LP tokens. A small Vault fee is taken from rewards, and it can be used to incentivize Crank operations.
- Upon liquidity removal, the user gets original tokens back in amounts proportional to Vault tokens they hold. Vault tokens are then burned.

`SBR-STAKE-LP-COMPOUND` is a similar strategy, but it uses Saber Pools and Farms.

### Main Router

An on-chain program that handles the creation, updates, and deletion of all metadata objects: tokens, pools, farms, vaults, program IDs, and generic key-value records, such as user or vault stats.

### Protocol Routers (Raydium, Saber, and Orca)

An on-chain programs that demonstrates interaction with Raydium, Saber, and Orca pools and farms. They performs in and out amounts calculations and safety checks for tokens spent and received. They don't hold user funds but validate, wrap, and send instructions to the AMMs and farms.

## Build

Before starting the build check `main_router` and `main_router_admin` pubkeys in `farm-sdk/src/id.rs`. They should point to existing main router program and admin account or generate a new set of keys if you plan to maintain your own version of the reference database:

```
solana-keygen new -o main_admin.json
solana-keygen new -o main_router.json
```

These keys must be used for main router deployment.

To build the off-chain library or program, run the `cargo build` command from each project directory, for example:

```sh
cd farms/farm-client
cargo build --release
```

To build on-chain programs, use the standard build command for Solana programs:

```sh
cargo build-bpf
```

To build Vaults, specify an additional argument that tells the compiler which strategy needs to be built:

```sh
cd farms/vaults
cargo build-bpf --no-default-features --features SBR-STAKE-LP-COMPOUND
```

## Test

Tests are executed with the `cargo test` command:

```sh
cd farms/farm-sdk
cargo test
```

Integration tests are located in the `farm-client/tests` directory and can be started as following:

```sh
cd farms/farm-client
cargo test -- --nocapture --test-threads=1 --ignored
```

Bear in mind that integration tests execute transactions, and it will cost you some SOL.

## Deploy & Run

To deploy on-chain programs, use the standard `solana program deploy`:

```sh
solana program deploy --commitment finalized target/deploy/solana_farm_vaults.so
solana program deploy --commitment finalized target/deploy/solana_farm_router_raydium.so
solana program deploy --commitment finalized target/deploy/solana_farm_router_saber.so
solana program deploy --commitment finalized target/deploy/solana_farm_router_orca.so
solana program deploy --commitment finalized --upgrade-authority main_admin.json --program-id main_router.json target/deploy/solana_farm_router_main.so
```

To start JSON RPC service:

```sh
target/release/solana-farm-rpc --farm-client-url https://api.mainnet-beta.solana.com --json-rpc-url http://0.0.0.0:9090
```

Open http://127.0.0.1:9090 in a browser to see available endpoints or check provided swagger schema in farms/farm-rpc/swagger.yaml.

## On-chain Reference Database

This project uses on-chain reference database to store required metadata. If you plan to maintain your own copy of the database you need to build and deploy main router and initialize the storage, otherwise skip this step.

First, generate PDA addresses for the RefDB indexes:

```sh
solana-farm-ctrl print-pda-all
```

Update `farm-ctrl/src/metadata/programs/programs.json` with newly generated addresses.

Initialize the storage:

```sh
solana-farm-ctrl --keypair main_admin.json init-all
```

And upload metadata:

```sh
solana-farm-ctrl --keypair main_admin.json load --skip-existing Program src/metadata/programs/programs.json
solana-farm-ctrl --keypair main_admin.json load --skip-existing Token src/metadata/tokens/solana_token_list/tokens.json
solana-farm-ctrl --keypair main_admin.json load --skip-existing Token src/metadata/tokens/raydium/lp_tokens.json
solana-farm-ctrl --keypair main_admin.json load --skip-existing Pool src/metadata/pools/raydium/pools.json
solana-farm-ctrl --keypair main_admin.json load --skip-existing Farm src/metadata/farms/raydium/farms.json
solana-farm-ctrl --keypair main_admin.json load --skip-existing Token src/metadata/tokens/saber/tokens.json
solana-farm-ctrl --keypair main_admin.json load --skip-existing Pool src/metadata/pools/saber/pools_and_farms.json
solana-farm-ctrl --keypair main_admin.json load --skip-existing Farm src/metadata/pools/saber/pools_and_farms.json
solana-farm-ctrl --keypair main_admin.json load --skip-existing Pool src/metadata/pools/orca/pools.json
solana-farm-ctrl --keypair main_admin.json load --skip-existing Farm src/metadata/pools/orca/farms.json
```

To generate metadata for Vaults run:

```sh
solana-farm-ctrl --keypair main_admin.json generate Vault [VAULT_PROGRAM_ADDRESS] [VAULT_NAME] [VAULT_TOKEN_NAME]
```

And then upload it:

```sh
solana-farm-ctrl --keypair main_admin.json load Token src/metadata/tokens/vault_tokens/vault_tokens.json
solana-farm-ctrl --keypair main_admin.json load Vault src/metadata/vaults/stc_saber/vaults.json
```

## Governance

To initialize the DAO first build and deploy governance program:

```sh
cd solana-program-library/governance/program
cargo build-bpf
solana program deploy --commitment finalized target/deploy/spl_governance.so
```

Then initialize the DAO using main router admin account with:

```sh
solana-farm-ctrl governance init [DAO_PROGRAM_ADDRESS] [DAO_TOKENS_TO_MINT]
```

It will take over on-chain programs upgrade authorities (including the DAO program itself) and DAO mint. Realm authority will also be removed. DAO tokens will be deposited to the admin account for further distribution.

Farm client can be used to perform all DAO operations: create proposals, deposit tokens, sign-off, add or execute instructions, vote, etc. See help for details:

```sh
solana-farm-client governance help
```

As part of DAO initialization, SOL token custody will be created (and more tokens can be added permissionless). Custody can be used to govern all interactions with pools, farms, or vaults. It is useful if a third party manages funds, and every operation must be voted on first. Farm client simplifies instruction creation and verification process, here is a workflow example for already initialized DAO:

```sh
solana-farm-client governance proposal-new FarmCustodyGovernance SwapTokens http://description.com 0
solana-farm-client governance signatory-add FarmCustodyGovernance 0 J7paVZ8axBfUaGFDNknc7XF3GHjVLZzvL57FaCuxjJo7
solana-farm-client governance instruction-insert-swap FarmCustodyGovernance 0 0 RDM RAY SRM 1.0 0.0
solana-farm-client -k signer.json governance sign-off FarmCustodyGovernance 0
solana-farm-client -k voter.json governance instruction-verify-swap FarmCustodyGovernance 0 0 RDM RAY SRM 1.0 0.0
solana-farm-client -k voter.json governance vote-cast FarmCustodyGovernance 0 1
solana-farm-client governance vote-finalize FarmCustodyGovernance 0
solana-farm-client -k anyone.json governance instruction-execute FarmCustodyGovernance 0 0
```

## Disclaimer

All claims, content, designs, algorithms, estimates, roadmaps, specifications, and performance measurements described in this project are done with the good faith efforts Solana Labs, Inc. and its affiliates ("SL"). It is up to the reader to check and validate their accuracy and truthfulness. Furthermore nothing in this project constitutes a solicitation for investment.
Any content produced by SL or developer resources that SL provides have not been subject to audit and are for educational and inspiration purposes only. SL does not encourage, induce or sanction the deployment, integration or use of any such applications (including the code comprising the Solana blockchain protocol) in violation of applicable laws or regulations and hereby prohibits any such deployment, integration or use. This includes use of any such applications by the reader (a) in violation of export control or sanctions laws of the United States or any other applicable jurisdiction, (b) if the reader is located in or ordinarily resident in a country or territory subject to comprehensive sanctions administered by the U.S. Office of Foreign Assets Control (OFAC), or (c) if the reader is or is working on behalf of a Specially Designated National (SDN) or a person subject to similar blocking or denied party prohibitions.
The reader should be aware that U.S. export control and sanctions laws prohibit U.S. persons (and other persons that are subject to such laws) from transacting with persons in certain countries and territories or that are on the SDN list. As a project based primarily on open-source software, it is possible that such sanctioned persons may nevertheless bypass prohibitions, obtain the code comprising the Solana blockchain protocol (or other project code or applications) and deploy, integrate, or otherwise use it. Accordingly, there is a risk to individuals that other persons using the Solana blockchain protocol may be sanctioned persons and that transactions with such persons would be a violation of U.S. export controls and sanctions law. This risk applies to individuals, organizations, and other ecosystem participants that deploy, integrate, or use the Solana blockchain protocol code directly (e.g., as a node operator), and individuals that transact on the Solana blockchain through light clients, third party interfaces, and/or wallet software.
