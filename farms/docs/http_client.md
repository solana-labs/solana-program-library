# HTTP Client

An easy way to interact with liquidity Pools, Farms, and Vaults or query on-chain information is by using Solana Farms HTTP RPC service. Under the hood, it wraps [Rust Client](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/rust_client.md) and serves its methods over HTTP.

HTTP RPC service is a part of Farms suite, and to use it, existing deployment of Farm programs and metadata must be present per [Quick Start Guide](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/quick_start.md).

To communicate with the service, use any tool or language that supports HTTP requests, for example:

```sh
curl 'http://127.0.0.1:9090/api/v1/token_account_balance?wallet_address=9wsC5hx5JopG5VwoDiUGrvgM2NaYz6tS3uyhuneRKgcN&token_name=RAY'
```

You can also use [SwaggerHub](https://app.swaggerhub.com/apis-docs/ska22/SolanaFarms/0.1) to call any method interactively. Swagger schema is available in `solana-program-library/farms/farm-rpc/swagger.yaml`.

The default endpoint port is `9000` and route prefix `/api/v1/`, e.g. `http://localhost:9000/api/v1`. A static page with descriptions and links to all methods will be displayed if the endpoint is opened in a browser without a particular method.

## Signing Transactions

While HTTP RPC service supports POST requests to interact with liquidity Pools, Farms, and Vaults, it is recommended for internal use or testing only because you will have to include a keypair with your request. Instead, you can sign a transaction locally with a wallet app and send it as usual via @solana/web3.js. To do so, you need to call an instruction building method (anything that starts with `new_instruction_` or `all_instructions_`) to receive a plain instruction in JSON, convert it, sign and send. Here is how it can be done in Javascript and Phantom:

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

## Methods

### Get

[protocols](#get-protocols)  
[admins](#get-admins)  
[program_admins](#get-program_admins)  
[git_token](#get-git_token)  
[git_tokens](#get-git_tokens)  
[fund](#get-fund)  
[funds](#get-funds)  
[fund_ref](#get-fund_ref)  
[fund_refs](#get-fund_refs)  
[fund_by_ref](#get-fund_by_ref)  
[fund_name](#get-fund_name)  
[find_funds](#get-find_funds)  
[vault](#get-vault)  
[vaults](#get-vaults)  
[vault_ref](#get-vault_ref)  
[vault_refs](#get-vault_refs)  
[vault_by_ref](#get-vault_by_ref)  
[vault_name](#get-vault_name)  
[find_vaults](#get-find_vaults)  
[find_vaults_with_vt](#get-find_vaults_with_vt)  
[pool](#get-pool)  
[pools](#get-pools)  
[pool_ref](#get-pool_ref)  
[pool_refs](#get-pool_refs)  
[pool_by_ref](#get-pool_by_ref)  
[pool_name](#get-pool_name)  
[find_pools](#get-find_pools)  
[find_pools_with_lp](#get-find_pools_with_lp)  
[pool_price](#get-pool_price)  
[oracle](#get-oracle)  
[oracle_price](#get-oracle_price)  
[farm](#get-farm)  
[farms](#get-farms)  
[farm_ref](#get-farm_ref)  
[farm_refs](#get-farm_refs)  
[farm_by_ref](#get-farm_by_ref)  
[farm_name](#get-farm_name)  
[find_farms_with_lp](#get-find_farms_with_lp)  
[token](#get-token)  
[tokens](#get-tokens)  
[token_ref](#get-token_ref)  
[token_refs](#get-token_refs)  
[token_by_ref](#get-token_by_ref)  
[token_name](#get-token_name)  
[token_with_mint](#get-token_with_mint)  
[token_with_account](#get-token_with_account)  
[program_id](#get-program_id)  
[program_ids](#get-program_ids)  
[program_name](#get-program_name)  
[is_official_id](#get-is_official_id)  
[is_fund_manager](#get-is_fund_manager)  
[managed_funds](#get-managed_funds)  
[token_supply](#get-token_supply)  
[associated_token_address](#get-associated_token_address)  
[wallet_tokens](#get-wallet_tokens)  
[token_account_data](#get-token_account_data)  
[account_balance](#get-account_balance)  
[token_account_balance](#get-token_account_balance)  
[token_account_balance_with_address](#get-token_account_balance_with_address)  
[has_active_token_account](#get-has_active_token_account)  
[fund_admins](#get-fund_admins)  
[fund_user_info](#get-fund_user_info)  
[all_fund_user_infos](#get-all_fund_user_infos)  
[fund_user_requests](#get-fund_user_requests)  
[all_fund_user_requests](#get-all_fund_user_requests)  
[fund_info](#get-fund_info)  
[all_fund_infos](#get-all_fund_infos)  
[fund_assets](#get-fund_assets)  
[fund_custody](#get-fund_custody)  
[fund_custody_with_balance](#get-fund_custody_with_balance)  
[fund_custodies](#get-fund_custodies)  
[fund_custodies_with_balance](#get-fund_custodies_with_balance)  
[fund_vault](#get-fund_vault)  
[fund_vaults](#get-fund_vaults)  
[fund_stats](#get-fund_stats)  
[user_stake_balance](#get-user_stake_balance)  
[vault_stake_balance](#get-vault_stake_balance)  
[vault_admins](#get-vault_admins)  
[vault_user_info](#get-vault_user_info)  
[vault_info](#get-vault_info)  
[all_vault_infos](#get-all_vault_infos)  
[vault_token_decimals](#get-vault_token_decimals)  
[pool_tokens_decimals](#get-pool_tokens_decimals)  
[new_instruction_create_system_account](#get-new_instruction_create_system_account)  
[new_instruction_create_system_account_with_seed](#get-new_instruction_create_system_account_with_seed)  
[new_instruction_close_system_account](#get-new_instruction_close_system_account)  
[new_instruction_transfer](#get-new_instruction_transfer)  
[new_instruction_token_transfer](#get-new_instruction_token_transfer)  
[new_instruction_sync_token_balance](#get-new_instruction_sync_token_balance)  
[new_instruction_create_token_account](#get-new_instruction_create_token_account)  
[new_instruction_close_token_account](#get-new_instruction_close_token_account)  
[new_instruction_user_init_vault](#get-new_instruction_user_init_vault)  
[new_instruction_add_liquidity_vault](#get-new_instruction_add_liquidity_vault)  
[new_instruction_lock_liquidity_vault](#get-new_instruction_lock_liquidity_vault)  
[new_instruction_unlock_liquidity_vault](#get-new_instruction_unlock_liquidity_vault)  
[new_instruction_remove_liquidity_vault](#get-new_instruction_remove_liquidity_vault)  
[new_instruction_add_liquidity_pool](#get-new_instruction_add_liquidity_pool)  
[new_instruction_remove_liquidity_pool](#get-new_instruction_remove_liquidity_pool)  
[new_instruction_wrap_token](#get-new_instruction_wrap_token)  
[new_instruction_unwrap_token](#get-new_instruction_unwrap_token)  
[new_instruction_swap](#get-new_instruction_swap)  
[new_instruction_user_init](#get-new_instruction_user_init)  
[new_instruction_stake](#get-new_instruction_stake)  
[new_instruction_unstake](#get-new_instruction_unstake)  
[new_instruction_harvest](#get-new_instruction_harvest)  
[new_instruction_crank_vault](#get-new_instruction_crank_vault)  
[new_instruction_user_init_fund](#get-new_instruction_user_init_fund)  
[new_instruction_request_deposit_fund](#get-new_instruction_request_deposit_fund)  
[new_instruction_cancel_deposit_fund](#get-new_instruction_cancel_deposit_fund)  
[new_instruction_request_withdrawal_fund](#get-new_instruction_request_withdrawal_fund)  
[new_instruction_cancel_withdrawal_fund](#get-new_instruction_cancel_withdrawal_fund)  
[new_instruction_start_liquidation_fund](#get-new_instruction_start_liquidation_fund)  
[new_instruction_disable_deposits_fund](#get-new_instruction_disable_deposits_fund)  
[new_instruction_approve_deposit_fund](#get-new_instruction_approve_deposit_fund)  
[new_instruction_deny_deposit_fund](#get-new_instruction_deny_deposit_fund)  
[new_instruction_disable_withdrawals_fund](#get-new_instruction_disable_withdrawals_fund)  
[new_instruction_approve_withdrawal_fund](#get-new_instruction_approve_withdrawal_fund)  
[new_instruction_deny_withdrawal_fund](#get-new_instruction_deny_withdrawal_fund)  
[new_instruction_lock_assets_fund](#get-new_instruction_lock_assets_fund)  
[new_instruction_unlock_assets_fund](#get-new_instruction_unlock_assets_fund)  
[new_instruction_update_fund_assets_with_custody](#get-new_instruction_update_fund_assets_with_custody)  
[new_instruction_update_fund_assets_with_vault](#get-new_instruction_update_fund_assets_with_vault)  
[new_instruction_fund_add_liquidity_pool](#get-new_instruction_fund_add_liquidity_pool)  
[new_instruction_fund_remove_liquidity_pool](#get-new_instruction_fund_remove_liquidity_pool)  
[new_instruction_fund_user_init_farm](#get-new_instruction_fund_user_init_farm)  
[new_instruction_fund_stake](#get-new_instruction_fund_stake)  
[new_instruction_fund_unstake](#get-new_instruction_fund_unstake)  
[new_instruction_fund_harvest](#get-new_instruction_fund_harvest)  
[new_instruction_fund_user_init_vault](#get-new_instruction_fund_user_init_vault)  
[new_instruction_fund_add_liquidity_vault](#get-new_instruction_fund_add_liquidity_vault)  
[new_instruction_fund_lock_liquidity_vault](#get-new_instruction_fund_lock_liquidity_vault)  
[new_instruction_fund_unlock_liquidity_vault](#get-new_instruction_fund_unlock_liquidity_vault)  
[new_instruction_fund_remove_liquidity_vault](#get-new_instruction_fund_remove_liquidity_vault)  
[all_instructions_token_transfer](#get-all_instructions_token_transfer)  
[all_instructions_wrap_sol](#get-all_instructions_wrap_sol)  
[all_instructions_unwrap_sol](#get-all_instructions_unwrap_sol)  
[all_instructions_add_liquidity_vault](#get-all_instructions_add_liquidity_vault)  
[all_instructions_add_locked_liquidity_vault](#get-all_instructions_add_locked_liquidity_vault)  
[all_instructions_remove_liquidity_vault](#get-all_instructions_remove_liquidity_vault)  
[all_instructions_remove_unlocked_liquidity_vault](#get-all_instructions_remove_unlocked_liquidity_vault)  
[all_instructions_add_liquidity_pool](#get-all_instructions_add_liquidity_pool)  
[all_instructions_remove_liquidity_pool](#get-all_instructions_remove_liquidity_pool)  
[all_instructions_swap](#get-all_instructions_swap)  
[all_instructions_stake](#get-all_instructions_stake)  
[all_instructions_unstake](#get-all_instructions_unstake)  
[all_instructions_harvest](#get-all_instructions_harvest)  
[all_instructions_request_deposit_fund](#get-all_instructions_request_deposit_fund)  
[all_instructions_request_withdrawal_fund](#get-all_instructions_request_withdrawal_fund)  
[all_instructions_fund_add_liquidity_pool](#get-all_instructions_fund_add_liquidity_pool)  
[all_instructions_fund_remove_liquidity_pool](#get-all_instructions_fund_remove_liquidity_pool)  
[all_instructions_fund_stake](#get-all_instructions_fund_stake)  
[all_instructions_fund_unstake](#get-all_instructions_fund_unstake)  
[all_instructions_fund_harvest](#get-all_instructions_fund_harvest)  
[all_instructions_fund_add_liquidity_vault](#get-all_instructions_fund_add_liquidity_vault)  
[all_instructions_fund_add_locked_liquidity_vault](#get-all_instructions_fund_add_locked_liquidity_vault)  
[all_instructions_fund_remove_liquidity_vault](#get-all_instructions_fund_remove_liquidity_vault)  
[all_instructions_fund_remove_unlocked_liquidity_vault](#get-all_instructions_fund_remove_unlocked_liquidity_vault)

### Post

[create_system_account](#post-create_system_account)  
[create_system_account_with_seed](#post-create_system_account_with_seed)  
[assign_system_account](#post-assign_system_account)  
[close_system_account](#post-close_system_account)  
[transfer](#post-transfer)  
[token_transfer](#post-token_transfer)  
[wrap_sol](#post-wrap_sol)  
[unwrap_sol](#post-unwrap_sol)  
[sync_token_balance](#post-sync_token_balance)  
[create_token_account](#post-create_token_account)  
[close_token_account](#post-close_token_account)  
[user_init_vault](#post-user_init_vault)  
[add_liquidity_vault](#post-add_liquidity_vault)  
[add_locked_liquidity_vault](#post-add_locked_liquidity_vault)  
[remove_liquidity_vault](#post-remove_liquidity_vault)  
[remove_unlocked_liquidity_vault](#post-remove_unlocked_liquidity_vault)  
[add_liquidity_pool](#post-add_liquidity_pool)  
[remove_liquidity_pool](#post-remove_liquidity_pool)  
[swap](#post-swap)  
[user_init](#post-user_init)  
[stake](#post-stake)  
[unstake](#post-unstake)  
[harvest](#post-harvest)  
[crank_vault](#post-crank_vault)  
[crank_vaults](#post-crank_vaults)  
[reset_cache](#post-reset_cache)  
[user_init_fund](#post-user_init_fund)  
[request_deposit_fund](#post-request_deposit_fund)  
[cancel_deposit_fund](#post-cancel_deposit_fund)  
[request_withdrawal_fund](#post-request_withdrawal_fund)  
[cancel_withdrawal_fund](#post-cancel_withdrawal_fund)  
[start_liquidation_fund](#post-start_liquidation_fund)  
[disable_deposits_fund](#post-disable_deposits_fund)  
[approve_deposit_fund](#post-approve_deposit_fund)  
[deny_deposit_fund](#post-deny_deposit_fund)  
[disable_withdrawals_fund](#post-disable_withdrawals_fund)  
[approve_withdrawal_fund](#post-approve_withdrawal_fund)  
[deny_withdrawal_fund](#post-deny_withdrawal_fund)  
[lock_assets_fund](#post-lock_assets_fund)  
[unlock_assets_fund](#post-unlock_assets_fund)  
[update_fund_assets_with_custody](#post-update_fund_assets_with_custody)  
[update_fund_assets_with_custodies](#post-update_fund_assets_with_custodies)  
[update_fund_assets_with_vault](#post-update_fund_assets_with_vault)  
[update_fund_assets_with_vaults](#post-update_fund_assets_with_vaults)  
[fund_add_liquidity_pool](#post-fund_add_liquidity_pool)  
[fund_remove_liquidity_pool](#post-fund_remove_liquidity_pool)  
[fund_user_init_farm](#post-fund_user_init_farm)  
[fund_stake](#post-fund_stake)  
[fund_unstake](#post-fund_unstake)  
[fund_harvest](#post-fund_harvest)  
[fund_user_init_vault](#post-fund_user_init_vault)  
[fund_add_liquidity_vault](#post-fund_add_liquidity_vault)  
[fund_add_locked_liquidity_vault](#post-fund_add_locked_liquidity_vault)  
[fund_remove_liquidity_vault](#post-fund_remove_liquidity_vault)  
[fund_remove_unlocked_liquidity_vault](#post-fund_remove_unlocked_liquidity_vault)

---

## (GET) protocols

Returns description and stats of all supported protocols

### Parameters:

No parameters

### Results:

The result will be an array of ProtocolInfo objects in Json or 404 status code with error description.

---

## (GET) admins

Returns current admin signers for the Main Router

### Parameters:

No parameters

### Results:

The result will be a Multisig object in Json or 404 status code with error description.

---

## (GET) program_admins

Returns program upgrade signers

### Parameters:

`program_id`: `Pubkey`

### Results:

The result will be a Multisig object in Json or 404 status code with error description.

---

## (GET) git_token

Returns Token metadata from Github

### Parameters:

`name`: `String`

### Results:

The result will be a GitToken object in Json or 404 status code with error description.

---

## (GET) git_tokens

Returns all Tokens from Github

### Parameters:

No parameters

### Results:

The result will be a GitTokens object in Json or 404 status code with error description.

---

## (GET) fund

Returns the Fund struct for the given name

### Parameters:

`name`: `String`

### Results:

The result will be a Fund object in Json or 404 status code with error description.

---

## (GET) funds

Returns all Funds available

### Parameters:

No parameters

### Results:

The result will be a FundMap object in Json or 404 status code with error description.

---

## (GET) fund_ref

Returns the Fund metadata address for the given name

### Parameters:

`name`: `String`

### Results:

The result will be a String object or 404 status code with error description.

---

## (GET) fund_refs

Returns Fund refs: a map of Fund name to account address with metadata

### Parameters:

No parameters

### Results:

The result will be a PubkeyMap object in Json or 404 status code with error description.

---

## (GET) fund_by_ref

Returns the Fund metadata at the specified address

### Parameters:

`fund_ref`: `Pubkey`

### Results:

The result will be a Fund object in Json or 404 status code with error description.

---

## (GET) fund_name

Returns the Fund name for the given metadata address

### Parameters:

`fund_ref`: `Pubkey`

### Results:

The result will be a String object or 404 status code with error description.

---

## (GET) find_funds

Returns all Funds that have Vaults with the name matching the pattern sorted by version

### Parameters:

`vault_name_pattern`: `String`

### Results:

The result will be an array of Fund objects in Json or 404 status code with error description.

---

## (GET) vault

Returns the Vault struct for the given name

### Parameters:

`name`: `String`

### Results:

The result will be a Vault object in Json or 404 status code with error description.

---

## (GET) vaults

Returns all Vaults available

### Parameters:

No parameters

### Results:

The result will be a VaultMap object in Json or 404 status code with error description.

---

## (GET) vault_ref

Returns the Vault metadata address for the given name

### Parameters:

`name`: `String`

### Results:

The result will be a String object or 404 status code with error description.

---

## (GET) vault_refs

Returns Vault refs: a map of Vault name to account address with metadata

### Parameters:

No parameters

### Results:

The result will be a PubkeyMap object in Json or 404 status code with error description.

---

## (GET) vault_by_ref

Returns the Vault metadata at the specified address

### Parameters:

`vault_ref`: `Pubkey`

### Results:

The result will be a Vault object in Json or 404 status code with error description.

---

## (GET) vault_name

Returns the Vault name for the given metadata address

### Parameters:

`vault_ref`: `Pubkey`

### Results:

The result will be a String object or 404 status code with error description.

---

## (GET) find_vaults

Returns all Vaults with tokens A and B sorted by version

### Parameters:

`token_a`: `String`  
`token_b`: `String`

### Results:

The result will be an array of Vault objects in Json or 404 status code with error description.

---

## (GET) find_vaults_with_vt

Returns all Vaults with tokens A and B sorted by version

### Parameters:

`vt_token_name`: `String`

### Results:

The result will be an array of Vault objects in Json or 404 status code with error description.

---

## (GET) pool

Returns the Pool struct for the given name

### Parameters:

`name`: `String`

### Results:

The result will be a Pool object in Json or 404 status code with error description.

---

## (GET) pools

Returns all Pools available

### Parameters:

No parameters

### Results:

The result will be a PoolMap object in Json or 404 status code with error description.

---

## (GET) pool_ref

Returns the Pool metadata address for the given name

### Parameters:

`name`: `String`

### Results:

The result will be a String object or 404 status code with error description.

---

## (GET) pool_refs

Returns Pool refs: a map of Pool name to account address with metadata

### Parameters:

No parameters

### Results:

The result will be a PubkeyMap object in Json or 404 status code with error description.

---

## (GET) pool_by_ref

Returns the Pool metadata at the specified address

### Parameters:

`pool_ref`: `Pubkey`

### Results:

The result will be a Pool object in Json or 404 status code with error description.

---

## (GET) pool_name

Returns the Pool name for the given metadata address

### Parameters:

`pool_ref`: `Pubkey`

### Results:

The result will be a String object or 404 status code with error description.

---

## (GET) find_pools

Returns all Pools with tokens A and B sorted by version for the given protocol

### Parameters:

`protocol`: `String`  
`token_a`: `String`  
`token_b`: `String`

### Results:

The result will be an array of Pool objects in Json or 404 status code with error description.

---

## (GET) find_pools_with_lp

Returns all Pools sorted by version for the given LP token

### Parameters:

`lp_token`: `String`

### Results:

The result will be an array of Pool objects in Json or 404 status code with error description.

---

## (GET) pool_price

Returns pair's price based on the ratio of tokens in the pool

### Parameters:

`name`: `String`

### Results:

The result will be a String object or 404 status code with error description.

---

## (GET) oracle

Returns oracle address for the given token

### Parameters:

`symbol`: `String`

### Results:

The result will be a Pubkey object in Json or 404 status code with error description.

---

## (GET) oracle_price

Returns the price in USD for the given token

### Parameters:

`symbol`: `String`  
`max_price_age_sec`: `u64`  
`max_price_error`: `f64`

### Results:

The result will be a String object or 404 status code with error description.

---

## (GET) farm

Returns the Farm struct for the given name

### Parameters:

`name`: `String`

### Results:

The result will be a Farm object in Json or 404 status code with error description.

---

## (GET) farms

Returns all Farms available

### Parameters:

No parameters

### Results:

The result will be a FarmMap object in Json or 404 status code with error description.

---

## (GET) farm_ref

Returns the Farm metadata address for the given name

### Parameters:

`name`: `String`

### Results:

The result will be a String object or 404 status code with error description.

---

## (GET) farm_refs

Returns Farm refs: a map of Farm name to account address with metadata

### Parameters:

No parameters

### Results:

The result will be a PubkeyMap object in Json or 404 status code with error description.

---

## (GET) farm_by_ref

Returns the Farm metadata at the specified address

### Parameters:

`farm_ref`: `Pubkey`

### Results:

The result will be a Farm object in Json or 404 status code with error description.

---

## (GET) farm_name

Returns the Farm name for the given metadata address

### Parameters:

`farm_ref`: `Pubkey`

### Results:

The result will be a String object or 404 status code with error description.

---

## (GET) find_farms_with_lp

Returns all Farms for the given LP token

### Parameters:

`lp_token`: `String`

### Results:

The result will be an array of Farm objects in Json or 404 status code with error description.

---

## (GET) token

Returns the Token struct for the given name

### Parameters:

`name`: `String`

### Results:

The result will be a Token object in Json or 404 status code with error description.

---

## (GET) tokens

Returns all Tokens available

### Parameters:

No parameters

### Results:

The result will be a TokenMap object in Json or 404 status code with error description.

---

## (GET) token_ref

Returns the Token metadata address for the given name

### Parameters:

`name`: `String`

### Results:

The result will be a String object or 404 status code with error description.

---

## (GET) token_refs

Returns Token refs: a map of Token name to account address with metadata

### Parameters:

No parameters

### Results:

The result will be a PubkeyMap object in Json or 404 status code with error description.

---

## (GET) token_by_ref

Returns the Token metadata at the specified address

### Parameters:

`token_ref`: `Pubkey`

### Results:

The result will be a Token object in Json or 404 status code with error description.

---

## (GET) token_name

Returns the Token name for the given metadata address

### Parameters:

`token_ref`: `Pubkey`

### Results:

The result will be a String object or 404 status code with error description.

---

## (GET) token_with_mint

Returns the Token metadata for the specified mint

### Parameters:

`token_mint`: `Pubkey`

### Results:

The result will be a Token object in Json or 404 status code with error description.

---

## (GET) token_with_account

Returns the Token metadata for the specified token account

### Parameters:

`token_account`: `Pubkey`

### Results:

The result will be a Token object in Json or 404 status code with error description.

---

## (GET) program_id

Returns the official Program ID for the given name

### Parameters:

`name`: `String`

### Results:

The result will be a String object or 404 status code with error description.

---

## (GET) program_ids

Returns all official Program IDs available

### Parameters:

No parameters

### Results:

The result will be a PubkeyMap object in Json or 404 status code with error description.

---

## (GET) program_name

Returns the official program name for the given Program ID

### Parameters:

`prog_id`: `Pubkey`

### Results:

The result will be a String object or 404 status code with error description.

---

## (GET) is_official_id

Checks if the given address is the official Program ID

### Parameters:

`prog_id`: `Pubkey`

### Results:

The result will be a bool object in Json or 404 status code with error description.

---

## (GET) is_fund_manager

Checks if the given address is the Fund manager

### Parameters:

`wallet_address`: `Pubkey`

### Results:

The result will be a bool object in Json or 404 status code with error description.

---

## (GET) managed_funds

Returns all Funds managed by the given address

### Parameters:

`wallet_address`: `Pubkey`

### Results:

The result will be an array of Fund objects in Json or 404 status code with error description.

---

## (GET) token_supply

Returns token supply as UI amount

### Parameters:

`token_name`: `String`

### Results:

The result will be a String object or 404 status code with error description.

---

## (GET) associated_token_address

Returns the associated token account address for the given token name

### Parameters:

`wallet_address`: `Pubkey`  
`token_name`: `String`

### Results:

The result will be a String object or 404 status code with error description.

---

## (GET) wallet_tokens

Returns all tokens with active account in the wallet

### Parameters:

`wallet_address`: `Pubkey`

### Results:

The result will be an array of String objects in Json or 404 status code with error description.

---

## (GET) token_account_data

Returns UiTokenAccount struct data for the associated token account address

### Parameters:

`wallet_address`: `Pubkey`  
`token_name`: `String`

### Results:

The result will be a UiTokenAccount object in Json or 404 status code with error description.

---

## (GET) account_balance

Returns native SOL balance

### Parameters:

`wallet_address`: `Pubkey`

### Results:

The result will be a String object or 404 status code with error description.

---

## (GET) token_account_balance

Returns token balance for the associated token account address

### Parameters:

`wallet_address`: `Pubkey`  
`token_name`: `String`

### Results:

The result will be a String object or 404 status code with error description.

---

## (GET) token_account_balance_with_address

Returns token balance for the specified token account address

### Parameters:

`token_account`: `Pubkey`

### Results:

The result will be a String object or 404 status code with error description.

---

## (GET) has_active_token_account

Returns true if the associated token account exists and is initialized

### Parameters:

`wallet_address`: `Pubkey`  
`token_name`: `String`

### Results:

The result will be a bool object in Json or 404 status code with error description.

---

## (GET) fund_admins

Returns current admin signers for the Fund

### Parameters:

`name`: `String`

### Results:

The result will be a Multisig object in Json or 404 status code with error description.

---

## (GET) fund_user_info

Returns user stats for specific Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`

### Results:

The result will be a FundUserInfo object in Json or 404 status code with error description.

---

## (GET) all_fund_user_infos

Returns user stats for all Funds

### Parameters:

`wallet_address`: `Pubkey`

### Results:

The result will be an array of FundUserInfo objects in Json or 404 status code with error description.

---

## (GET) fund_user_requests

Returns user requests for specific Fund and token

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`token_name`: `String`

### Results:

The result will be a FundUserRequests object in Json or 404 status code with error description.

---

## (GET) all_fund_user_requests

Returns user requests for all tokens accepted by the Fund

### Parameters:

`fund_name`: `String`

### Results:

The result will be an array of FundUserRequests objects in Json or 404 status code with error description.

---

## (GET) fund_info

Returns Fund stats and config

### Parameters:

`fund_name`: `String`

### Results:

The result will be a FundInfo object in Json or 404 status code with error description.

---

## (GET) fund_assets

Returns the Fund assets info

### Parameters:

`fund_name`: `String`  
`asset_type`: `String`

### Results:

The result will be a FundAssets object in Json or 404 status code with error description.

---

## (GET) fund_custody

Returns the Fund custody info

### Parameters:

`fund_name`: `String`  
`token_name`: `String`  
`custody_type`: `String`

### Results:

The result will be a FundCustody object in Json or 404 status code with error description.

---

## (GET) fund_custody_with_balance

Returns the Fund custody extended info

### Parameters:

`fund_name`: `String`  
`token_name`: `String`  
`custody_type`: `String`

### Results:

The result will be a FundCustodyWithBalance object in Json or 404 status code with error description.

---

## (GET) fund_custodies

Returns all custodies belonging to the Fund sorted by custody_id

### Parameters:

`fund_name`: `String`

### Results:

The result will be an array of FundCustody objects in Json or 404 status code with error description.

---

## (GET) fund_custodies_with_balance

Returns all custodies belonging to the Fund with extended info

### Parameters:

`fund_name`: `String`

### Results:

The result will be an array of FundCustodyWithBalance objects in Json or 404 status code with error description.

---

## (GET) fund_vault

Returns the Fund Vault info

### Parameters:

`fund_name`: `String`  
`vault_name`: `String`  
`vault_type`: `String`

### Results:

The result will be a FundVault object in Json or 404 status code with error description.

---

## (GET) fund_vaults

Returns all Vaults belonging to the Fund sorted by vault_id

### Parameters:

`fund_name`: `String`

### Results:

The result will be an array of FundVault objects in Json or 404 status code with error description.

---

## (GET) fund_stats

Returns Fund's historical performance

### Parameters:

`fund_name`: `String`  
`timeframe`: `String`  
`start_time`: `i64`  
`limit`: `u32`

### Results:

The result will be an array of FundStatsRecord objects in Json or 404 status code with error description.

---

## (GET) user_stake_balance

Returns User's stacked balance

### Parameters:

`wallet_address`: `Pubkey`  
`farm_name`: `String`

### Results:

The result will be a String object or 404 status code with error description.

---

## (GET) vault_stake_balance

Returns Vault's stacked balance

### Parameters:

`vault_name`: `String`

### Results:

The result will be a String object or 404 status code with error description.

---

## (GET) vault_admins

Returns current admin signers for the Vault

### Parameters:

`name`: `String`

### Results:

The result will be a Multisig object in Json or 404 status code with error description.

---

## (GET) vault_user_info

Returns user stats for specific Vault

### Parameters:

`wallet_address`: `Pubkey`  
`vault_name`: `String`

### Results:

The result will be a VaultUserInfo object in Json or 404 status code with error description.

---

## (GET) vault_info

Returns Vault stats

### Parameters:

`vault_name`: `String`

### Results:

The result will be a VaultInfo object in Json or 404 status code with error description.

---

## (GET) vault_token_decimals

Returns number of decimal digits of the Vault token

### Parameters:

`vault_name`: `String`

### Results:

The result will be a String object or 404 status code with error description.

---

## (GET) pool_tokens_decimals

Returns number of decimal digits of the Vault token

### Parameters:

`pool_name`: `String`

### Results:

The result will be an array of u8 objects in Json or 404 status code with error description.

---

## (GET) new_instruction_create_system_account

Returns a new Instruction for creating system account

### Parameters:

`wallet_address`: `Pubkey`  
`new_address`: `Pubkey`  
`lamports`: `u64`  
`space`: `u64`  
`owner`: `Pubkey`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_create_system_account_with_seed

Returns a new Instruction for creating system account with seed

### Parameters:

`wallet_address`: `Pubkey`  
`base_address`: `Pubkey`  
`seed`: `String`  
`lamports`: `u64`  
`space`: `u64`  
`owner`: `Pubkey`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_close_system_account

Returns a new Instruction for closing system account

### Parameters:

`wallet_address`: `Pubkey`  
`target_address`: `Pubkey`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_transfer

Returns the native SOL transfer instruction

### Parameters:

`wallet_address`: `Pubkey`  
`destination_wallet`: `Pubkey`  
`sol_ui_amount`: `f64`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_token_transfer

Returns a tokens transfer instruction

### Parameters:

`wallet_address`: `Pubkey`  
`token_name`: `String`  
`destination_wallet`: `Pubkey`  
`ui_amount`: `f64`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_sync_token_balance

Returns a new Instruction for syncing token balance for the specified account

### Parameters:

`wallet_address`: `Pubkey`  
`token_name`: `String`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_create_token_account

Returns a new Instruction for creating associated token account

### Parameters:

`wallet_address`: `Pubkey`  
`token_name`: `String`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_close_token_account

Returns a new Instruction for closing associated token account

### Parameters:

`wallet_address`: `Pubkey`  
`token_name`: `String`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_user_init_vault

Returns a new Instruction for initializing a new User for the Vault

### Parameters:

`wallet_address`: `Pubkey`  
`vault_name`: `String`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_add_liquidity_vault

Returns a new Instruction for adding liquidity to the Vault

### Parameters:

`wallet_address`: `Pubkey`  
`vault_name`: `String`  
`max_token_a_ui_amount`: `f64`  
`max_token_b_ui_amount`: `f64`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_lock_liquidity_vault

Returns a new Instruction for locking liquidity in the Vault

### Parameters:

`wallet_address`: `Pubkey`  
`vault_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_unlock_liquidity_vault

Returns a new Instruction for unlocking liquidity from the Vault

### Parameters:

`wallet_address`: `Pubkey`  
`vault_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_remove_liquidity_vault

Returns a new Instruction for removing liquidity from the Vault

### Parameters:

`wallet_address`: `Pubkey`  
`vault_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_add_liquidity_pool

Returns a new Instruction for adding liquidity to the Pool

### Parameters:

`wallet_address`: `Pubkey`  
`pool_name`: `String`  
`max_token_a_ui_amount`: `f64`  
`max_token_b_ui_amount`: `f64`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_remove_liquidity_pool

Returns a new Instruction for removing liquidity from the Pool

### Parameters:

`wallet_address`: `Pubkey`  
`pool_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_wrap_token

Returns a new Instruction for wrapping the token into protocol specific token

### Parameters:

`wallet_address`: `Pubkey`  
`pool_name`: `String`  
`token_to_wrap`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_unwrap_token

Returns a new Instruction for unwrapping the token from protocol specific token

### Parameters:

`wallet_address`: `Pubkey`  
`pool_name`: `String`  
`token_to_unwrap`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_swap

Returns a new Instruction for tokens swap

### Parameters:

`wallet_address`: `Pubkey`  
`protocol`: `String`  
`from_token`: `String`  
`to_token`: `String`  
`ui_amount_in`: `f64`  
`min_ui_amount_out`: `f64`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_user_init

Returns a new Instruction for initializing a new User in the Farm

### Parameters:

`wallet_address`: `Pubkey`  
`farm_name`: `String`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_stake

Returns a new Instruction for tokens staking

### Parameters:

`wallet_address`: `Pubkey`  
`farm_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_unstake

Returns a new Instruction for tokens unstaking

### Parameters:

`wallet_address`: `Pubkey`  
`farm_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_harvest

Returns a new Instruction for rewards harvesting

### Parameters:

`wallet_address`: `Pubkey`  
`farm_name`: `String`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_crank_vault

Returns a new Vault Crank Instruction

### Parameters:

`wallet_address`: `Pubkey`  
`vault_name`: `String`  
`step`: `u64`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_user_init_fund

Returns a new Instruction for initializing a new User for the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`token_name`: `String`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_request_deposit_fund

Returns a new Instruction for requesting deposit to the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`token_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_cancel_deposit_fund

Returns a new Instruction for canceling pending deposit to the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`token_name`: `String`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_request_withdrawal_fund

Returns a new Instruction for requesting withdrawal from the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`token_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_cancel_withdrawal_fund

Returns a new Instruction for canceling pending withdrawal from the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`token_name`: `String`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_start_liquidation_fund

Returns a new Instruction for initiating liquidation of the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_disable_deposits_fund

Returns a new Instruction for disabling deposits to the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_approve_deposit_fund

Returns a new Instruction for approving deposit to the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`user_address`: `Pubkey`  
`fund_name`: `String`  
`token_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_deny_deposit_fund

Returns a new Instruction for denying deposit to the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`user_address`: `Pubkey`  
`fund_name`: `String`  
`token_name`: `String`  
`deny_reason`: `String`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_disable_withdrawals_fund

Returns a new Instruction for disabling withdrawals from the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_approve_withdrawal_fund

Returns a new Instruction for approving withdrawal from the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`user_address`: `Pubkey`  
`fund_name`: `String`  
`token_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_deny_withdrawal_fund

Returns a new Instruction for denying withdrawal from the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`user_address`: `Pubkey`  
`fund_name`: `String`  
`token_name`: `String`  
`deny_reason`: `String`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_lock_assets_fund

Returns a new Instruction for moving deposited assets to the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`token_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_unlock_assets_fund

Returns a new Instruction for releasing assets from the Fund to Deposit/Withdraw custody

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`token_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_update_fund_assets_with_custody

Returns a new Instruction for updating Fund assets based on custody holdings

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`custody_id`: `u32`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_update_fund_assets_with_vault

Returns a new Instruction for updating Fund assets with Vault holdings

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`vault_id`: `u32`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_fund_add_liquidity_pool

Returns a new Instruction for adding liquidity to the Pool in the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`pool_name`: `String`  
`max_token_a_ui_amount`: `f64`  
`max_token_b_ui_amount`: `f64`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_fund_remove_liquidity_pool

Returns a new Instruction for removing liquidity from the Pool in the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`pool_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_fund_user_init_farm

Returns a new Instruction for initializing a new User for the Farm in the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`farm_name`: `String`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_fund_stake

Returns a new Instruction for tokens staking to the Farm in the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`farm_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_fund_unstake

Returns a new Instruction for tokens unstaking from the Farm in the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`farm_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_fund_harvest

Returns a new Instruction for rewards harvesting from the Farm in the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`farm_name`: `String`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_fund_user_init_vault

Returns a new Instruction for initializing a new User for the Vault in the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`vault_name`: `String`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_fund_add_liquidity_vault

Returns a new Instruction for adding liquidity to the Vault in the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`vault_name`: `String`  
`max_token_a_ui_amount`: `f64`  
`max_token_b_ui_amount`: `f64`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_fund_lock_liquidity_vault

Returns a new Instruction for locking liquidity in the Vault in the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`vault_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_fund_unlock_liquidity_vault

Returns a new Instruction for unlocking liquidity from the Vault in the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`vault_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) new_instruction_fund_remove_liquidity_vault

Returns a new Instruction for removing liquidity from the Vault in the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`vault_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an Instruction object in Json or 404 status code with error description.

---

## (GET) all_instructions_token_transfer

Returns a new complete set of instructions for tokens transfer

### Parameters:

`wallet_address`: `Pubkey`  
`token_name`: `String`  
`destination_wallet`: `Pubkey`  
`ui_amount`: `f64`

### Results:

The result will be an array of Instruction objects in Json or 404 status code with error description.

---

## (GET) all_instructions_wrap_sol

Returns a new complete set of instructions for SOL wrapping

### Parameters:

`wallet_address`: `Pubkey`  
`ui_amount`: `f64`

### Results:

The result will be an array of Instruction objects in Json or 404 status code with error description.

---

## (GET) all_instructions_unwrap_sol

Returns a new complete set of instructions for SOL unwrapping

### Parameters:

`wallet_address`: `Pubkey`

### Results:

The result will be an array of Instruction objects in Json or 404 status code with error description.

---

## (GET) all_instructions_add_liquidity_vault

Returns a new complete set of instructions for adding liquidity to the Vault

### Parameters:

`wallet_address`: `Pubkey`  
`vault_name`: `String`  
`max_token_a_ui_amount`: `f64`  
`max_token_b_ui_amount`: `f64`

### Results:

The result will be an array of Instruction objects in Json or 404 status code with error description.

---

## (GET) all_instructions_add_locked_liquidity_vault

Returns a new complete set of instructions for adding locked liquidity to the Vault

### Parameters:

`wallet_address`: `Pubkey`  
`vault_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an array of Instruction objects in Json or 404 status code with error description.

---

## (GET) all_instructions_remove_liquidity_vault

Returns a new complete set of Instructions for removing liquidity from the Vault

### Parameters:

`wallet_address`: `Pubkey`  
`vault_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an array of Instruction objects in Json or 404 status code with error description.

---

## (GET) all_instructions_remove_unlocked_liquidity_vault

Returns a new complete set of Instructions for removing unlocked liquidity from the Vault

### Parameters:

`wallet_address`: `Pubkey`  
`vault_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an array of Instruction objects in Json or 404 status code with error description.

---

## (GET) all_instructions_add_liquidity_pool

Returns a new complete set of Instructions for adding liquidity to the Pool

### Parameters:

`wallet_address`: `Pubkey`  
`pool_name`: `String`  
`max_token_a_ui_amount`: `f64`  
`max_token_b_ui_amount`: `f64`

### Results:

The result will be an array of Instruction objects in Json or 404 status code with error description.

---

## (GET) all_instructions_remove_liquidity_pool

Returns a new complete set of Instructions for removing liquidity from the Pool

### Parameters:

`wallet_address`: `Pubkey`  
`pool_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an array of Instruction objects in Json or 404 status code with error description.

---

## (GET) all_instructions_swap

Returns a new complete set of Instructions for swapping tokens

### Parameters:

`wallet_address`: `Pubkey`  
`protocol`: `String`  
`from_token`: `String`  
`to_token`: `String`  
`ui_amount_in`: `f64`  
`min_ui_amount_out`: `f64`

### Results:

The result will be an array of Instruction objects in Json or 404 status code with error description.

---

## (GET) all_instructions_stake

Returns a new complete set of Instructions for staking tokens to the Farm

### Parameters:

`wallet_address`: `Pubkey`  
`farm_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an array of Instruction objects in Json or 404 status code with error description.

---

## (GET) all_instructions_unstake

Returns a new complete set of Instructions for unstaking tokens from the Farm

### Parameters:

`wallet_address`: `Pubkey`  
`farm_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an array of Instruction objects in Json or 404 status code with error description.

---

## (GET) all_instructions_harvest

Returns a new complete set of Instructions for harvesting rewards from the Farm

### Parameters:

`wallet_address`: `Pubkey`  
`farm_name`: `String`

### Results:

The result will be an array of Instruction objects in Json or 404 status code with error description.

---

## (GET) all_instructions_request_deposit_fund

Returns a new complete set of Instructions for requesting a new deposit to the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`token_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an array of Instruction objects in Json or 404 status code with error description.

---

## (GET) all_instructions_request_withdrawal_fund

Returns a new complete set of Instructions for requesting a new withdrawal from the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`token_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an array of Instruction objects in Json or 404 status code with error description.

---

## (GET) all_instructions_fund_add_liquidity_pool

Returns a new complete set of Instructions for adding liquidity to the Pool in the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`pool_name`: `String`  
`max_token_a_ui_amount`: `f64`  
`max_token_b_ui_amount`: `f64`

### Results:

The result will be an array of Instruction objects in Json or 404 status code with error description.

---

## (GET) all_instructions_fund_remove_liquidity_pool

Returns a new complete set of Instructions for removing liquidity from the Pool in the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`pool_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an array of Instruction objects in Json or 404 status code with error description.

---

## (GET) all_instructions_fund_stake

Returns a new complete set of Instructions for staking tokens to the Farm in the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`farm_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an array of Instruction objects in Json or 404 status code with error description.

---

## (GET) all_instructions_fund_unstake

Returns a new complete set of Instructions for unstaking tokens from the Farm in the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`farm_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an array of Instruction objects in Json or 404 status code with error description.

---

## (GET) all_instructions_fund_harvest

Returns a new complete set of Instructions for harvesting rewards from the Farm in the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`farm_name`: `String`

### Results:

The result will be an array of Instruction objects in Json or 404 status code with error description.

---

## (GET) all_instructions_fund_add_liquidity_vault

Returns a new complete set of instructions for adding liquidity to the Vault in the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`vault_name`: `String`  
`max_token_a_ui_amount`: `f64`  
`max_token_b_ui_amount`: `f64`

### Results:

The result will be an array of Instruction objects in Json or 404 status code with error description.

---

## (GET) all_instructions_fund_add_locked_liquidity_vault

Returns a new complete set of instructions for adding locked liquidity to the Vault in the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`vault_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an array of Instruction objects in Json or 404 status code with error description.

---

## (GET) all_instructions_fund_remove_liquidity_vault

Returns a new complete set of Instructions for removing liquidity from the Vault in the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`vault_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an array of Instruction objects in Json or 404 status code with error description.

---

## (GET) all_instructions_fund_remove_unlocked_liquidity_vault

Returns a new complete set of Instructions for removing unlocked liquidity from the Vault in the Fund

### Parameters:

`wallet_address`: `Pubkey`  
`fund_name`: `String`  
`vault_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be an array of Instruction objects in Json or 404 status code with error description.

---

## (POST) create_system_account

Creates a new system account

### Parameters:

`wallet_keypair`: `Keypair`  
`new_account_keypair`: `Keypair`  
`lamports`: `u64`  
`space`: `u64`  
`owner`: `Pubkey`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) create_system_account_with_seed

Creates a new system account with seed

### Parameters:

`wallet_keypair`: `Keypair`  
`base_address`: `Pubkey`  
`seed`: `String`  
`lamports`: `u64`  
`space`: `u64`  
`owner`: `Pubkey`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) assign_system_account

Assigns system account to a program

### Parameters:

`wallet_keypair`: `Keypair`  
`program_address`: `Pubkey`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) close_system_account

Closes existing system account

### Parameters:

`wallet_keypair`: `Keypair`  
`target_account_keypair`: `Keypair`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) transfer

Transfers native SOL from the wallet to the destination

### Parameters:

`wallet_keypair`: `Keypair`  
`destination_wallet`: `Pubkey`  
`sol_ui_amount`: `f64`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) token_transfer

Transfers tokens from the wallet to the destination

### Parameters:

`wallet_keypair`: `Keypair`  
`token_name`: `String`  
`destination_wallet`: `Pubkey`  
`ui_amount`: `f64`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) wrap_sol

Transfers native SOL from the wallet to the associated Wrapped SOL account

### Parameters:

`wallet_keypair`: `Keypair`  
`ui_amount`: `f64`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) unwrap_sol

Transfers Wrapped SOL back to SOL by closing the associated Wrapped SOL account

### Parameters:

`wallet_keypair`: `Keypair`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) sync_token_balance

Updates token balance of the account, usefull after transfer SOL to WSOL account

### Parameters:

`wallet_keypair`: `Keypair`  
`token_name`: `String`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) create_token_account

Returns the associated token account for the given user's main account or creates one

### Parameters:

`wallet_keypair`: `Keypair`  
`token_name`: `String`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) close_token_account

Closes existing token account associated with the given user's main account

### Parameters:

`wallet_keypair`: `Keypair`  
`token_name`: `String`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) user_init_vault

Initializes a new User for the Vault

### Parameters:

`wallet_keypair`: `Keypair`  
`vault_name`: `String`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) add_liquidity_vault

Adds liquidity to the Vault

### Parameters:

`wallet_keypair`: `Keypair`  
`vault_name`: `String`  
`max_token_a_ui_amount`: `f64`  
`max_token_b_ui_amount`: `f64`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) add_locked_liquidity_vault

Adds locked liquidity to the Vault

### Parameters:

`wallet_keypair`: `Keypair`  
`vault_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) remove_liquidity_vault

Removes liquidity from the Vault

### Parameters:

`wallet_keypair`: `Keypair`  
`vault_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) remove_unlocked_liquidity_vault

Removes unlocked liquidity from the Vault

### Parameters:

`wallet_keypair`: `Keypair`  
`vault_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) add_liquidity_pool

Adds liquidity to the Pool

### Parameters:

`wallet_keypair`: `Keypair`  
`pool_name`: `String`  
`max_token_a_ui_amount`: `f64`  
`max_token_b_ui_amount`: `f64`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) remove_liquidity_pool

Removes liquidity from the Pool

### Parameters:

`wallet_keypair`: `Keypair`  
`pool_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) swap

Swaps tokens

### Parameters:

`wallet_keypair`: `Keypair`  
`protocol`: `String`  
`from_token`: `String`  
`to_token`: `String`  
`ui_amount_in`: `f64`  
`min_ui_amount_out`: `f64`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) user_init

Initializes a new User for the Farm

### Parameters:

`wallet_keypair`: `Keypair`  
`farm_name`: `String`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) stake

Stakes tokens to the Farm

### Parameters:

`wallet_keypair`: `Keypair`  
`farm_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) unstake

Unstakes tokens from the Farm

### Parameters:

`wallet_keypair`: `Keypair`  
`farm_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) harvest

Harvests rewards from the Farm

### Parameters:

`wallet_keypair`: `Keypair`  
`farm_name`: `String`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) crank_vault

Cranks single Vault

### Parameters:

`wallet_keypair`: `Keypair`  
`vault_name`: `String`  
`step`: `u64`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) crank_vaults

Cranks all Vaults

### Parameters:

`wallet_keypair`: `Keypair`  
`step`: `u64`

### Results:

The result will be a String object or 404 status code with error description.

---

## (POST) reset_cache

Clears cache records to force re-pull from blockchain

### Parameters:

No parameters

### Results:

The result will be a String object or 404 status code with error description.

---

## (POST) user_init_fund

Initializes a new User for the Fund

### Parameters:

`wallet_keypair`: `Keypair`  
`fund_name`: `String`  
`token_name`: `String`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) request_deposit_fund

Requests a new deposit to the Fund

### Parameters:

`wallet_keypair`: `Keypair`  
`fund_name`: `String`  
`token_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) cancel_deposit_fund

Cancels pending deposit to the Fund

### Parameters:

`wallet_keypair`: `Keypair`  
`fund_name`: `String`  
`token_name`: `String`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) request_withdrawal_fund

Requests a new withdrawal from the Fund

### Parameters:

`wallet_keypair`: `Keypair`  
`fund_name`: `String`  
`token_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) cancel_withdrawal_fund

Cancels pending deposit to the Fund

### Parameters:

`wallet_keypair`: `Keypair`  
`fund_name`: `String`  
`token_name`: `String`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) start_liquidation_fund

Starts the Fund liquidation

### Parameters:

`wallet_keypair`: `Keypair`  
`fund_name`: `String`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) disable_deposits_fund

Disables deposits to the Fund

### Parameters:

`wallet_keypair`: `Keypair`  
`fund_name`: `String`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) approve_deposit_fund

Approves pending deposit to the Fund

### Parameters:

`wallet_keypair`: `Keypair`  
`fund_name`: `String`  
`user_address`: `Pubkey`  
`token_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) deny_deposit_fund

Denies pending deposit to the Fund

### Parameters:

`wallet_keypair`: `Keypair`  
`fund_name`: `String`  
`user_address`: `Pubkey`  
`token_name`: `String`  
`deny_reason`: `String`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) disable_withdrawals_fund

Disables withdrawals from the Fund

### Parameters:

`wallet_keypair`: `Keypair`  
`fund_name`: `String`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) approve_withdrawal_fund

Approves pending withdrawal from the Fund

### Parameters:

`wallet_keypair`: `Keypair`  
`fund_name`: `String`  
`user_address`: `Pubkey`  
`token_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) deny_withdrawal_fund

Denies pending withdrawal from the Fund

### Parameters:

`wallet_keypair`: `Keypair`  
`fund_name`: `String`  
`user_address`: `Pubkey`  
`token_name`: `String`  
`deny_reason`: `String`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) lock_assets_fund

Moves deposited assets from Deposit/Withdraw custody to the Fund

### Parameters:

`wallet_keypair`: `Keypair`  
`fund_name`: `String`  
`token_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) unlock_assets_fund

Releases assets from the Fund to Deposit/Withdraw custody

### Parameters:

`wallet_keypair`: `Keypair`  
`fund_name`: `String`  
`token_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) update_fund_assets_with_custody

Update Fund assets info based on custody holdings

### Parameters:

`wallet_keypair`: `Keypair`  
`fund_name`: `String`  
`custody_id`: `u32`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) update_fund_assets_with_custodies

Update Fund assets info based on all custodies

### Parameters:

`wallet_keypair`: `Keypair`  
`fund_name`: `String`

### Results:

The result will be a String object or 404 status code with error description.

---

## (POST) update_fund_assets_with_vault

Update Fund assets info based on Vault holdings

### Parameters:

`wallet_keypair`: `Keypair`  
`fund_name`: `String`  
`vault_id`: `u32`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) update_fund_assets_with_vaults

Update Fund assets info based on Vault holdings

### Parameters:

`wallet_keypair`: `Keypair`  
`fund_name`: `String`

### Results:

The result will be a String object or 404 status code with error description.

---

## (POST) fund_add_liquidity_pool

Adds liquidity to the Pool in the Fund

### Parameters:

`wallet_keypair`: `Keypair`  
`fund_name`: `String`  
`pool_name`: `String`  
`max_token_a_ui_amount`: `f64`  
`max_token_b_ui_amount`: `f64`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) fund_remove_liquidity_pool

Removes liquidity from the Pool in the Fund

### Parameters:

`wallet_keypair`: `Keypair`  
`fund_name`: `String`  
`pool_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) fund_user_init_farm

Initializes a new User for the Farm in the Fund

### Parameters:

`wallet_keypair`: `Keypair`  
`fund_name`: `String`  
`farm_name`: `String`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) fund_stake

Stakes tokens to the Farm in the Fund

### Parameters:

`wallet_keypair`: `Keypair`  
`fund_name`: `String`  
`farm_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) fund_unstake

Unstakes tokens from the Farm in the Fund

### Parameters:

`wallet_keypair`: `Keypair`  
`fund_name`: `String`  
`farm_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) fund_harvest

Harvests rewards from the Farm in the Fund

### Parameters:

`wallet_keypair`: `Keypair`  
`fund_name`: `String`  
`farm_name`: `String`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) fund_user_init_vault

Initializes a new User for the Vault in the Fund

### Parameters:

`wallet_keypair`: `Keypair`  
`fund_name`: `String`  
`vault_name`: `String`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) fund_add_liquidity_vault

Adds liquidity to the Vault in the Fund

### Parameters:

`wallet_keypair`: `Keypair`  
`fund_name`: `String`  
`vault_name`: `String`  
`max_token_a_ui_amount`: `f64`  
`max_token_b_ui_amount`: `f64`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) fund_add_locked_liquidity_vault

Adds locked liquidity to the Vault in the Fund

### Parameters:

`wallet_keypair`: `Keypair`  
`fund_name`: `String`  
`vault_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) fund_remove_liquidity_vault

Removes liquidity from the Vault in the Fund

### Parameters:

`wallet_keypair`: `Keypair`  
`fund_name`: `String`  
`vault_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be a Signature object or 404 status code with error description.

---

## (POST) fund_remove_unlocked_liquidity_vault

Removes unlocked liquidity from the Vault in the Fund

### Parameters:

`wallet_keypair`: `Keypair`  
`fund_name`: `String`  
`vault_name`: `String`  
`ui_amount`: `f64`

### Results:

The result will be a Signature object or 404 status code with error description.
