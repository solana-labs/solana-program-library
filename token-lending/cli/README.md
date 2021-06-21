# SPL Token Lending program command line interface

A basic CLI for initializing lending markets and reserves for SPL Token Lending.
See https://spl.solana.com/token-lending for more details

## Install the CLI
```shell
cargo install spl-token-lending-cli
```

## Deploy a lending program (optional)

Follow [this guide](../README.md#Deploy-a-lending-program) and note the program ID.

## Create a lending market
```shell
spl-token-lending \
  --program      PUBKEY \
  --fee-payer    SIGNER \
  create-market \
  --market-owner PUBKEY

# Creating lending market CyUJdNpYoAhnUeYk6kfFWbZnhuaPXW6KoAxNuhs2ssYN
# Signature: 262NEkpPMiBiTq2DUXd3G3TkkRqFZf4e5ebojzYDkP7XVaSRANK1ir5Gk8zr8XLW6CG2xGzNFvEcUrbnENwenEwa
```
- `--program` is the lending program ID.
- `--fee-payer` will sign to pay transaction fees.
- `--market-owner` is the lending market owner pubkey.

Note the lending market pubkey (e.g. `CyUJdNpYoAhnUeYk6kfFWbZnhuaPXW6KoAxNuhs2ssYN`). You'll use this to add reserves.

Run `spl-token-lending create-market --help` for more details and options.

## Add a reserve to your market

```shell
spl-token-lending \
  --program      PUBKEY \
  --fee-payer    SIGNER \
  add-reserve \
  --market-owner SIGNER \
  --source-owner SIGNER \
  --market       PUBKEY \
  --source       PUBKEY \
  --amount       FLOAT  \
  --pyth-product 8yrQMUyJRnCJ72NWwMiPV9dNGw465Z8bKUvnUC8P5L6F \
  --pyth-price   BdgHsXrH1mXqhdosXavYxZgX6bGqTdj5mh2sxDhF8bJy
```
- `--program` is the lending program ID.
- `--fee-payer` will sign to pay transaction fees.
- `--market-owner` will sign as the lending market owner.
- `--source-owner` will sign as the source liquidity owner.
- `--market` is the lending market pubkey.
- `--source` is the SPL Token account pubkey (owned by `--source-owner`).
- `--amount` is the amount of tokens to deposit.
- `--pyth-product` and `--pyth-price` are oracle
  accounts [provided by Pyth](https://pyth.network/developers/consumers/accounts) (SOL/USD shown).

Run `spl-token-lending add-reserve --help` for more details and options.