# transfer-tokens example program

A simple program using a program-derived address to transfer SPL tokens, AKA "The Giver"

## Motivation

One of the most widely-used actions on-chain is an SPL token transfer. It's used
for escrows, trading, staking, DAO voting, NFT marketplaces, and many other surprising
and novel use-cases.

Most of these on-chain programs must use a program-derived address to authorize
token transfers from their reserves.

By showing both of these in a simple example program, on-chain program developers
can copy this code whenever needed.

## Concept

When invoked, the program transfers any tokens owned by a program-derived
address, defined by:

```rust
Pubkey::find_program_address(&[b"authority"], program_id);
```

## How to use

* Choose the token mint address you'd like to use for the transfer

* Build and deploy the program, and save the new program id

```console
$ cargo build-sbf --manifest-path program/Cargo.toml
$ solana program deploy ../../../target/deploy/spl_example_transfer_tokens.so
```

* Get the address for the program

```console
$ cargo run --manifest-path cli/Cargo.toml -- pda <PROGRAM_ID>
```

* Create the associated token account for the program-derived address on that mint

```console
$ spl-token create-account <MINT_ADDRESS> --owner <PROGRAM_DERIVED_ADDRESS> --fee-payer <YOUR_KEY>
```

* Transfer tokens into the new account

```console
$ spl-token transfer <MINT_ADDRESS> <AMOUNT> <PROGRAM_DERIVED_ADDRESS>
```

* Invoke the program to retrieve the tokens

```console
$ cargo run --manifest-path cli/Cargo.toml -- give <PROGRAM_ID> <MINT_ADDRESS> <DESTINATION_ACCOUNT>
```
