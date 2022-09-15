# Fund

The Fund program implements a decentralized, non-custodial, and trustless capital management protocol. It is built on top of supported liquidity protocols, including Farm Vaults. Fund Managers are responsible for selecting a portfolio of assets their Fund will hold. These assets can be in various forms: individual tokens, liquidity invested into different Pools, staked into Farms, or deposited into Farm Vaults.

Fund Managers are allowed to perform specific operations with tokens: swap, add/remove liquidity, stake/unstake/harvest, etc., and only in approved Pools, while all other actions are forbidden. It is enforced by the Fund program and allows investors to earn passive returns while maintaining custody of their assets (by holding Fund tokens that are minted upon each deposit and can be withdrawn for underlying assets).

There could be any number of Funds, each implementing its own assets management strategy. Because of the blockchain nature, historical performance and current allocations are always transparent, so investors can make an informed decision on which Funds they want to invest in.

## Build & Deploy

To run the Fund, first, build and deploy Farm programs and upload metadata as described in the [Quick Start Guide](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/quick_start.md).

To build and deploy the Fund program, run:

```sh
cd solana-program-library/farms/fund
cargo build-bpf
solana program deploy ../target/deploy/solana_fund.so
```

Integration tests are located in the `fund/tests` directory and can be started as follows:

```sh
cargo test -- --nocapture --test-threads=1 --ignored
```

These tests execute transactions on mainnet, which will cost you some SOL.

The next step is to generate Fund metadata:

```sh
solana-farm-ctrl --keypair main_admin.json generate Fund [FUND_PROGRAM_ADDRESS] [FUND_NAME]
```

Generated metadata should be manually saved to JSON files (tokens and funds separately) using a format similar to other tokens and vault files. And then uploaded with:

```sh
solana-farm-ctrl --keypair main_admin.json load token fund_tokens.json
solana-farm-ctrl --keypair main_admin.json load fund funds.json
```

To verify metadata and installation, run `solana-farm-ctrl get-all fund` and `solana-farm-client fund-info [FUND_NAME]`.

## Initialization

Before a Fund can be used it, must be initialized with `solana-farm-ctrl fund-init [FUNDNAME]`. Instead of executing commands with `Farm Ctrl CLI`, alternatively, you can create and send corresponding instructions with [HTTP Client](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/http_client.md) or [Rust Client](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/rust_client.md).

Each Fund has three types of configuration settings. First is Fund Assets Tracking Config, which can be set with `fund-set-assets-tracking-config`. It defines parameters such as assets limit, oracle price minimum quality, and whether or not to issue Fund tokens to depositors. Only admins are allowed to alter these settings. The other configs are Deposit and Withdrawal Schedules, which can be set with `fund-set-deposit-schedule` and `fund-set-withdrawal-schedule`. They define when deposits/withdrawals can be made, whether they will require the approval of the Fund Manager, fees, and amount limits. Deposit/Withdrawal configs can be set or modified by admins or Fund managers.

Funds keep all assets in custodies. There are two types of custodies - DepositWithdraw and Trading. The former is used to accept initial deposits or process withdrawals and the latter to perform trading operations. Custodies can be added with `fund-add-custody`. Users will be allowed to make deposits and withdrawals only in tokens for which DepositWithdraw custodies have been created. Similarly, trading operations (like swaps, adding liquidity to a Farm or Vault, etc.) require corresponding Trading custodies to be pre-created, including the ones for holding LP tokens. Fund Manager can move tokens from DepositWithdraw custody to Trading or vice versa using `fund-lock-assets` and `fund-unlock-assets`.

Liquidity Pools, Farms, and Vaults that particular Fund will be allowed to trade in or deposit liquidity to must be explicitly whitelisted. This can be done with `fund-add-vault` and require admin privileges.

Fund Managers perform trading operations, deposit and withdrawal approvals (if enabled). New Fund Manager can be set with `fund-set-manager`.

Whether the Fund is properly initialized can be verified with client tools, e.g. `solana-farm-client fund-info [FUND_NAME]`.
