# Rust Client

Rust Client is a library that provides an easy way for off-chain Rust programs to interact with Routers, Pools, Funds, Vaults, and Funds, perform admin operations, metadata queries, and some common operations with wallets and accounts.

Rust Client is a part of Farms suite, and to use it, existing deployment of Farm programs and metadata must be present per [Quick Start Guide](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/quick_start.md).

The library needs to be specified as a dependency in your Cargo.toml:

```
[dependencies]
solana-farm-client = "1.1.2"
```

And declared in a program:

```rust
use solana_farm_client::client::FarmClient;

let client = FarmClient::new("https://api.mainnet-beta.solana.com");
let keypair = FarmClient::read_keypair_from_file(
    &(std::env::var("HOME").unwrap().to_string() + "/.config/solana/id.json"),
);
```

Client's methods accept human-readable names (tokens, polls, etc.) and UI (decimal) amounts, so you can simply call:

```rust
client.swap(&keypair, "RDM", "SOL", "RAY", 0.1);
client.add_liquidity(&keypair, "RDM.RAY-SOL", 0.0, 0.1);
client.stake(&keypair, "RDM.RAY-SOL", 0.0);
client.harvest(&keypair, "RDM.RAY-SOL");
```

to swap 0.1 SOL for RAY, deposit RAY and SOL to a Raydium pool, stake LP tokens, and harvest rewards. `RDM` above is a route or protocol. Use `SBR` for Saber pools and `ORC` for Orca. All metadata required to lookup account addresses, decimals, etc., is stored on-chain.

Rust Client caches metadata to make subsequent calls faster, most noticeably queries that list a large number of objects, like `get_tokens()`. It also leverages RefDB to detect if any metadata objects have changed (e.g., any of the tokens) and reloads them if that is the case.

Under the hood Client uses the official Solana RPC Client which can be accessed with
client.rpc_client, for example: `client.rpc_client.get_latest_blockhash()`.

The naming convention for Pools and Farms is `[PROTOCOL].[TOKEN_A]-[TOKEN_B]-[VERSION]`.
Naming convention for Vaults is `[PROTOCOL].[STRATEGY].[TOKEN_A]-[TOKEN_B]-[VERSION]`.
There are single token pools where `[TOKEN_B]` is not present.
A list of supported protocols can be obtained with `get_protocols()`.
If `[VERSION]` is omitted, then Pool, Farm, or Vault with the latest version will be used.

A few examples:

```rust
// get SOL account balance
client.get_account_balance(&keypair.pubkey());

// get SPL token account balance
client.get_token_account_balance(&keypair.pubkey(), "SRM");

// get token metadata
client.get_token("SRM");

// find Raydium pools with RAY and SRM tokens
client.find_pools(Protocol::Raydium, "RAY", "SRM");

// find Saber pools with USDC and USDT tokens
client.find_pools(Protocol::Saber, "USDC", "USDT");

// get pool metadata
client.get_pool("RDM.RAY-SRM");

// get farm metadata
client.get_farm("RDM.RAY-SRM");

// find all vaults with RAY and SRM tokens
client.find_vaults("RAY", "SRM");

// get vault metadata
client.get_vault("RDM.STC.RAY-SRM");

// get fund metadata
client.get_fund("TestFund");

// get the list of all pools
client.get_pools();

// find farms for specific LP token
client.find_farms_with_lp("LP.RDM.RAY-SRM-V4");

// get Raydium pool price
client.get_pool_price("RDM.RAY-SRM");
// or specify version for specific pool
client.get_pool_price("RDM.RAY-SRM-V4");

// get oracle price
client.get_oracle_price("SOL", 0, 0.0);

// list official program IDs
client.get_program_ids();

// swap in the Raydium pool
client.swap(&keypair, Protocol::Raydium, "SOL", "RAY", 0.01, 0.0);

// swap in the Saber pool
client.swap(&keypair, Protocol::Saber, "USDC", "USDT", 0.01, 0.0);

// deposit liquidity to the Raydium pool (zero second token amount means calculate it automatically)
client.add_liquidity_pool(&keypair, "RDM.GRAPE-USDC", 0.1, 0.0);

// withdraw your liquidity from the Raydium pool (zero amount means remove all tokens)
client.remove_liquidity_pool(&keypair, "RDM.GRAPE-USDC", 0.0);

// stake LP tokens to the Raydium farm (zero amount means stake all)
client.stake(&keypair, "RDM.GRAPE-USDC", 0.0);

// get staked balance
client.get_user_stake_balance(&keypair.pubkey(), "RDM.GRAPE-USDC");

// harvest rewards
client.harvest(&keypair, "RDM.GRAPE-USDC");

// unstake LP tokens from the farm (zero amount means unstake all)
client.unstake(&keypair, "RDM.GRAPE-USDC", 0.0);

// deposit liquidity to the vault (zero second token amount means calculate it automatically)
client.add_liquidity_vault(&keypair, "RDM.STC.RAY-SRM", 0.01, 0.0);

// withdraw liquidity from the vault (zero amount means remove all tokens)
client.remove_liquidity_vault(&keypair, "RDM.STC.RAY-SRM", 0.0);

// request liquidity deposit to the fund
client.request_deposit_fund(&keypair, "TestFund", "USDC", 0.01);

// request liquidity withdrawal from the fund (zero amount means withdraw everything)
client.request_withdrawal_fund(&keypair, "TestFund", "USDC", 0.0);

// list all vaults that belong to particular fund
client.get_fund_vaults("TestFund");

// transfer SOL to another wallet
client.transfer(&keypair, &Pubkey::new_unique(), 0.001);

// transfer SPL tokens to another wallet
client.token_transfer(&keypair, "SRM", &Pubkey::new_unique(), 0.001);

// create associated token account for the wallet
client.get_or_create_token_account(&keypair, "SRM");

// list all active token accounts for the wallet
client.get_wallet_tokens(&keypair.pubkey());

// get vault stats
client.get_vault_info("RDM.STC.RAY-SRM");

// get user stats for particular vault
client.get_vault_user_info(&keypair.pubkey(), "RDM.STC.RAY-SRM");

// get fund stats and parameters
client.get_fund_info("TestFund");

// get fund custody info
client.get_fund_custody("TestFund", "USDC", FundCustodyType::DepositWithdraw);

// get information about fund assets
client.get_fund_assets(&fund_name, FundAssetType::Vault);
client.get_fund_assets(&fund_name, FundAssetType::Custody);
```

The Client also allows for building raw unsigned instructions that can be integrated into more complex workflows:

```rust
// create a new instruction for cranking a Vault, neither sign nor send it
let inst = client.new_instruction_crank_vault(&keypair.pubkey(), "RDM.STC.RAY-SRM");
```

You can then sign and send it with:

```rust
client.sign_and_send_instructions(&[keypair], &[inst]);
```

Some actions may require multiple instructions to be executed. To handle such cases, there are methods with names starting with `all_instructions_`, and they return a vector of instructions. For example:

```rust
// create a single instruction for depositing liquidity to the vault assuming all prerequisites are met
client.new_instruction_add_liquidity_vault(&keypair.pubkey(), "RDM.STC.RAY-SRM", 0.1, 0.0);

// create potentially multiple instructions that would handle new token accounts creation, user initialization, token wrap/unwrap, etc.
client.all_instructions_add_liquidity_vault(&keypair.pubkey(), "RDM.STC.RAY-SRM", 0.1, 0.0);
```

In addition to the library, there is also a command-line tool that sets an example for basic usage of the library: [Farm Client CLI](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/farm_client_cli.md).
