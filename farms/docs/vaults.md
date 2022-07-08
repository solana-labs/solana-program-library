# Vaults

Vaults are on-chain programs that implement various yield farming strategies. Under the hood, they interact with underlying Liquidity Pools and other Defi protocols to generate an extra yield for users that wouldn't be possible with passive investments. Individual yield farming strategies are stored under the `strategies` sub-folder.

`RDM-STAKE-LP-COMPOUND` strategy works as follows:

- User deposits tokens into a Vault with add_liquidity transaction. For example, Vault `RDM.STC.RAY-SRM` takes RAY and SRM tokens. To get a list of available Vaults, one can use the `client.get_vaults()` function or `api/v1/vaults` RPC call. Vault tokens are minted back to the user to represent their share in the Vault.
- Vault sends user tokens to the corresponding Raydium Pool, receives LP tokens, and stakes them to the Raydium Farm.
- Vaults should be cranked on a periodic basis. Crank operation is permissionless and can be done by anyone. And it is executed for the entire Vault, not per individual user. Crank consists of three steps:
  1. Harvest Farm rewards (in one or both tokens);
  2. Rebalance rewards to get proper amounts of each token;
  3. Place rewards back into the Pool and stake received LP tokens. A small Vault fee is taken from rewards, and it can be used to incentivize Crank operations.
- Upon liquidity removal, the user gets original tokens back in amounts proportional to Vault tokens they hold. Vault tokens are then burned.

`SBR-STAKE-LP-COMPOUND` and `ORC-STAKE-LP-COMPOUND` are similar strategies but use Saber and Orca protocols.

## Initialization

In order to run Vaults, first, build and deploy Farm programs and upload metadata as described in the [Quick Start Guide](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/quick_start.md).

To initialize and enable Vaults, run:

```sh
solana-farm-ctrl vault-init all
solana-farm-ctrl vault-enable-deposits all
solana-farm-ctrl vault-enable-withdrawals all
```

You can set Vault fees and minimum allowed crank interval with `vault-set-fee` and `vault-set-min-crank-interval`. Cranks need to be executed periodically using `solana-farm-ctrl vault-crank all` or `solana-farm-client crank-vaults`. Alternatively you can use [HTTP Client](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/http_client.md) or [Rust Client](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/rust_client.md) or send raw instructions.

Whether the Vault is properly initialized can be verified with client tools, e.g., `solana-farm-client vault-info [VAULT_NAME]`.
