# SPL Token Lending program command line interface

A basic CLI for initializing lending markets and reserves for SPL Token Lending.
See https://spl.solana.com/token-lending for more details

## Install the CLI
```shell
cargo install spl-token-lending-cli
```

## Deploy a lending program

Follow [this guide](../README.md#Deploy-a-lending-program)

## Create a lending market
```shell
spl-token-lending create-market \
  --program      PUBKEY \
  --fee-payer    SIGNER \
  --market-owner PUBKEY

# Creating lending market CyUJdNpYoAhnUeYk6kfFWbZnhuaPXW6KoAxNuhs2ssYN
# Signature: 262NEkpPMiBiTq2DUXd3G3TkkRqFZf4e5ebojzYDkP7XVaSRANK1ir5Gk8zr8XLW6CG2xGzNFvEcUrbnENwenEwa
```
- `--program` is your lending program ID.
- `--fee-payer` will sign to pay transaction fees.
- `--market-owner` is your lending market owner pubkey.

Note the lending market pubkey (e.g. `CyUJdNpYoAhnUeYk6kfFWbZnhuaPXW6KoAxNuhs2ssYN`).

Run `spl-token-lending create-market --help` for more details and options.

## Add a reserve to your market

```shell
spl-token-lending add-reserve \
  --program      PUBKEY \
  --fee-payer    SIGNER \
  --market-owner SIGNER \
  --token-owner  SIGNER \
  --market       PUBKEY \
  --source       PUBKEY \
  --amount       FLOAT  \
  --pyth-product 8yrQMUyJRnCJ72NWwMiPV9dNGw465Z8bKUvnUC8P5L6F \
  --pyth-price   BdgHsXrH1mXqhdosXavYxZgX6bGqTdj5mh2sxDhF8bJy
```
- `--program` is your lending program ID.
- `--fee-payer` will sign to pay transaction fees.
- `--market-owner` will sign as the lending market owner.
- `--token-owner` will sign as the token owner.
- `--market` is your lending market pubkey.
- `--source` is your SPL Token account pubkey.
- `--amount` is the amount of tokens to deposit.
- `--pyth-product` and `--pyth-price` are oracle
  accounts [provided by Pyth](https://pyth.network/developers/consumers/accounts) (SOL/USD shown).

Run `spl-token-lending add-reserve --help` for more details and options.