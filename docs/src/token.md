---
title: Token Program
---

A Token program on the Solana blockchain.

This program defines a common implementation for Fungible and Non Fungible tokens.

## Background

Solana's programming model and the definitions of the Solana terms used in this
document are available at:

- https://docs.solana.com/apps
- https://docs.solana.com/terminology

## Source

The Token Program's source is available on
[github](https://github.com/solana-labs/solana-program-library)

## Interface

The Token Program is written in Rust and available on [crates.io](https://crates.io/crates/spl-token) and [docs.rs](https://docs.rs/spl-token).

Auto-generated C bindings are also available
[here](https://github.com/solana-labs/solana-program-library/blob/master/token/program/inc/token.h)

[JavaScript
bindings](https://github.com/solana-labs/solana-program-library/blob/master/token/js/client/token.js)
are available that support loading the Token Program on to a chain and issue
instructions.

See the [SPL Associated Token Account](associated-token-account.md) program for
convention around wallet address to token account mapping and funding.

## Command-line Utility

The `spl-token` command-line utility can be used to experiment with SPL
tokens.  Once you have [Rust installed](https://rustup.rs/), run:
```console
$ cargo install spl-token-cli
```

Run `spl-token --help` for a full description of available commands.

### Configuration

The `spl-token` configuration is shared with the `solana` command-line tool.

#### Current Configuration

```console
$ solana config get
Config File: ${HOME}/.config/solana/cli/config.yml
RPC URL: https://api.mainnet-beta.solana.com
WebSocket URL: wss://api.mainnet-beta.solana.com/ (computed)
Keypair Path: ${HOME}/.config/solana/id.json
```

#### Cluster RPC URL

See [Solana clusters](https://docs.solana.com/clusters) for cluster-specific RPC URLs
```console
$ solana config set --url https://api.devnet.solana.com
```

#### Default Keypair

See [Keypair conventions](https://docs.solana.com/cli/conventions#keypair-conventions)
for information on how to setup a keypair if you don't already have one.

Keypair File
```console
$ solana config set --keypair ${HOME}/new-keypair.json
```

Hardware Wallet URL (See [URL spec](https://docs.solana.com/wallet-guide/hardware-wallets#specify-a-keypair-url))
```console
$ solana config set --keypair usb://ledger/
```

#### Airdrop SOL

Creating tokens and accounts requires SOL for account rent deposits and
transaction fees. If the cluster you are targeting offers a faucet, you can get
a little SOL for testing:
```console
$ solana airdrop 1
```

### Example: Creating your own fungible token

```console
$ spl-token create-token
Creating token AQoKYV7tYpTrFZN6P5oUufbQKAUr9mNYGe1TTJC9wajM
Signature: 47hsLFxWRCg8azaZZPSnQR8DNTRsGyPNfUK7jqyzgt7wf9eag3nSnewqoZrVZHKm8zt3B6gzxhr91gdQ5qYrsRG4
```

The unique identifier of the token is `AQoKYV7tYpTrFZN6P5oUufbQKAUr9mNYGe1TTJC9wajM`.

Tokens when initially created by `spl-token` have no supply:
```console
$ spl-token supply AQoKYV7tYpTrFZN6P5oUufbQKAUr9mNYGe1TTJC9wajM
0
```

Let's mint some.  First create an account to hold a balance of the new
`AQoKYV7tYpTrFZN6P5oUufbQKAUr9mNYGe1TTJC9wajM` token:
```console
$ spl-token create-account AQoKYV7tYpTrFZN6P5oUufbQKAUr9mNYGe1TTJC9wajM
Creating account 7UX2i7SucgLMQcfZ75s3VXmZZY4YRUyJN9X1RgfMoDUi
Signature: 42Sa5eK9dMEQyvD9GMHuKxXf55WLZ7tfjabUKDhNoZRAxj9MsnN7omriWMEHXLea3aYpjZ862qocRLVikvkHkyfy
```

`7UX2i7SucgLMQcfZ75s3VXmZZY4YRUyJN9X1RgfMoDUi` is now an empty account:
```console
$ spl-token balance AQoKYV7tYpTrFZN6P5oUufbQKAUr9mNYGe1TTJC9wajM
0
```

Mint 100 tokens into the account:
```console
$ spl-token mint AQoKYV7tYpTrFZN6P5oUufbQKAUr9mNYGe1TTJC9wajM 100
Minting 100 tokens
  Token: AQoKYV7tYpTrFZN6P5oUufbQKAUr9mNYGe1TTJC9wajM
  Recipient: 7UX2i7SucgLMQcfZ75s3VXmZZY4YRUyJN9X1RgfMoDUi
Signature: 41mARH42fPkbYn1mvQ6hYLjmJtjW98NXwd6pHqEYg9p8RnuoUsMxVd16RkStDHEzcS2sfpSEpFscrJQn3HkHzLaa
```

The token `supply` and account `balance` now reflect the result of minting:
```console
$ spl-token supply AQoKYV7tYpTrFZN6P5oUufbQKAUr9mNYGe1TTJC9wajM
100
```

```console
$ spl-token balance AQoKYV7tYpTrFZN6P5oUufbQKAUr9mNYGe1TTJC9wajM
100
```

### Example: View all Tokens that you own

```console
$ spl-token accounts
Token                                         Balance
------------------------------------------------------------
7e2X5oeAAJyUTi4PfSGXFLGhyPw2H8oELm1mx87ZCgwF  84
AQoKYV7tYpTrFZN6P5oUufbQKAUr9mNYGe1TTJC9wajM  100
AQoKYV7tYpTrFZN6P5oUufbQKAUr9mNYGe1TTJC9wajM  0    (Aux-1*)
AQoKYV7tYpTrFZN6P5oUufbQKAUr9mNYGe1TTJC9wajM  1    (Aux-2*)
```

### Example: Wrapping SOL in a Token

```console
$ spl-token wrap 1
Wrapping 1 SOL into GJTxcnA5Sydy8YRhqvHxbQ5QNsPyRKvzguodQEaShJje
Signature: 4f4s5QVMKisLS6ihZcXXPbiBAzjnvkBcp2A7KKER7k9DwJ4qjbVsQBKv2rAyBumXC1gLn8EJQhwWkybE4yJGnw2Y
```

To unwrap the Token back to SOL:
```console
$ spl-token unwrap GJTxcnA5Sydy8YRhqvHxbQ5QNsPyRKvzguodQEaShJje
Unwrapping GJTxcnA5Sydy8YRhqvHxbQ5QNsPyRKvzguodQEaShJje
  Amount: 1 SOL
  Recipient: vines1vzrYbzLMRdu58ou5XTby4qAqVRLmqo36NKPTg
Signature: f7opZ86ZHKGvkJBQsJ8Pk81v8F3v1VUfyd4kFs4CABmfTnSZK5BffETznUU3tEWvzibgKJASCf7TUpDmwGi8Rmh
```

### Example: Transferring tokens to another user
First the receiver uses `spl-token create-account` to create their associated
token account for the Token type.  Then the receiver obtains their wallet
address by running `solana address` and provides it to the sender.

The sender then runs:
```console
$ spl-token transfer AQoKYV7tYpTrFZN6P5oUufbQKAUr9mNYGe1TTJC9wajM 50 vines1vzrYbzLMRdu58ou5XTby4qAqVRLmqo36NKPTg
Transfer 50 tokens
  Sender: 7UX2i7SucgLMQcfZ75s3VXmZZY4YRUyJN9X1RgfMoDUi
  Recipient: vines1vzrYbzLMRdu58ou5XTby4qAqVRLmqo36NKPTg
  Recipient associated token account: F59618aQB8r6asXeMcB9jWuY6NEx1VduT9yFo1GTi1ks

Signature: 5a3qbvoJQnTAxGPHCugibZTbSu7xuTgkxvF4EJupRjRXGgZZrnWFmKzfEzcqKF2ogCaF4QKVbAtuFx7xGwrDUcGd
```

### Example: Transferring tokens to another user, with sender-funding
If the receiver does not yet have an associated token account, the sender may
choose to fund the receiver's account.

The receiver obtains their wallet address by running `solana address` and provides it to the sender.

The sender then runs to fund the receiver's associated token account, at the
sender's expense, and then transfers 50 tokens into it:
```console
$ spl-token transfer --fund-recipient AQoKYV7tYpTrFZN6P5oUufbQKAUr9mNYGe1TTJC9wajM 50 vines1vzrYbzLMRdu58ou5XTby4qAqVRLmqo36NKPTg
Transfer 50 tokens
  Sender: 7UX2i7SucgLMQcfZ75s3VXmZZY4YRUyJN9X1RgfMoDUi
  Recipient: vines1vzrYbzLMRdu58ou5XTby4qAqVRLmqo36NKPTg
  Recipient associated token account: F59618aQB8r6asXeMcB9jWuY6NEx1VduT9yFo1GTi1ks
  Funding recipient: F59618aQB8r6asXeMcB9jWuY6NEx1VduT9yFo1GTi1ks (0.00203928 SOL)

Signature: 5a3qbvoJQnTAxGPHCugibZTbSu7xuTgkxvF4EJupRjRXGgZZrnWFmKzfEzcqKF2ogCaF4QKVbAtuFx7xGwrDUcGd
```

### Example: Transferring tokens to an explicit recipient token account
Tokens may be transferred to a specific recipient token account.  The recipient
token account must already exist and be of the same Token type.

```console
$ spl-token create-account AQoKYV7tYpTrFZN6P5oUufbQKAUr9mNYGe1TTJC9wajM /path/to/auxiliary_keypair.json
Creating account CqAxDdBRnawzx9q4PYM3wrybLHBhDZ4P6BTV13WsRJYJ
Signature: 4yPWj22mbyLu5mhfZ5WATNfYzTt5EQ7LGzryxM7Ufu7QCVjTE7czZdEBqdKR7vjKsfAqsBdjU58NJvXrTqCXvfWW
```

```console
$ spl-token accounts AQoKYV7tYpTrFZN6P5oUufbQKAUr9mNYGe1TTJC9wajM -v
Account                                       Token                                         Balance
--------------------------------------------------------------------------------------------------------
7UX2i7SucgLMQcfZ75s3VXmZZY4YRUyJN9X1RgfMoDUi  AQoKYV7tYpTrFZN6P5oUufbQKAUr9mNYGe1TTJC9wajM  100
CqAxDdBRnawzx9q4PYM3wrybLHBhDZ4P6BTV13WsRJYJ  AQoKYV7tYpTrFZN6P5oUufbQKAUr9mNYGe1TTJC9wajM  0    (Aux-1*)
```

```console
$ spl-token transfer 7UX2i7SucgLMQcfZ75s3VXmZZY4YRUyJN9X1RgfMoDUi 50 CqAxDdBRnawzx9q4PYM3wrybLHBhDZ4P6BTV13WsRJYJ
Transfer 50 tokens
  Sender: 7UX2i7SucgLMQcfZ75s3VXmZZY4YRUyJN9X1RgfMoDUi
  Recipient: CqAxDdBRnawzx9q4PYM3wrybLHBhDZ4P6BTV13WsRJYJ

Signature: 5a3qbvoJQnTAxGPHCugibZTbSu7xuTgkxvF4EJupRjRXGgZZrnWFmKzfEzcqKF2ogCaF4QKVbAtuFx7xGwrDUcGd
```

```console
$ spl-token accounts AQoKYV7tYpTrFZN6P5oUufbQKAUr9mNYGe1TTJC9wajM -v
Account                                       Token                                         Balance
--------------------------------------------------------------------------------------------------------
7UX2i7SucgLMQcfZ75s3VXmZZY4YRUyJN9X1RgfMoDUi  AQoKYV7tYpTrFZN6P5oUufbQKAUr9mNYGe1TTJC9wajM  50
CqAxDdBRnawzx9q4PYM3wrybLHBhDZ4P6BTV13WsRJYJ  AQoKYV7tYpTrFZN6P5oUufbQKAUr9mNYGe1TTJC9wajM  50  (Aux-1*)
```

### Example: Create a non-fungible token

Create the token type with zero decimal place,
```console
$ spl-token create-token --decimals 0
Creating token 559u4Tdr9umKwft3yHMsnAxohhzkFnUBPAFtibwuZD9z
Signature: 4kz82JUey1B9ki1McPW7NYv1NqPKCod6WNptSkYqtuiEsQb9exHaktSAHJJsm4YxuGNW4NugPJMFX9ee6WA2dXts
```

then create an account to hold tokens of this new type:
```console
$ spl-token create-account 559u4Tdr9umKwft3yHMsnAxohhzkFnUBPAFtibwuZD9z
Creating account 7KqpRwzkkeweW5jQoETyLzhvs9rcCj9dVQ1MnzudirsM
Signature: sjChze6ecaRtvuQVZuwURyg6teYeiH8ZwT6UTuFNKjrdayQQ3KNdPB7d2DtUZ6McafBfEefejHkJ6MWQEfVHLtC
```

Now mint only one token into the account,
```console
$ spl-token mint 559u4Tdr9umKwft3yHMsnAxohhzkFnUBPAFtibwuZD9z 1 7KqpRwzkkeweW5jQoETyLzhvs9rcCj9dVQ1MnzudirsM
Minting 1 tokens
  Token: 559u4Tdr9umKwft3yHMsnAxohhzkFnUBPAFtibwuZD9z
  Recipient: 7KqpRwzkkeweW5jQoETyLzhvs9rcCj9dVQ1MnzudirsM
Signature: 2Kzg6ZArQRCRvcoKSiievYy3sfPqGV91Whnz6SeimhJQXKBTYQf3E54tWg3zPpYLbcDexxyTxnj4QF69ucswfdY
```

and disable future minting:
```console
$ spl-token authorize 559u4Tdr9umKwft3yHMsnAxohhzkFnUBPAFtibwuZD9z mint --disable
Updating 559u4Tdr9umKwft3yHMsnAxohhzkFnUBPAFtibwuZD9z
  Current mint authority: vines1vzrYbzLMRdu58ou5XTby4qAqVRLmqo36NKPTg
  New mint authority: disabled
Signature: 5QpykLzZsceoKcVRRFow9QCdae4Dp2zQAcjebyEWoezPFg2Np73gHKWQicHG1mqRdXu3yiZbrft3Q8JmqNRNqhwU
```

Now the `7KqpRwzkkeweW5jQoETyLzhvs9rcCj9dVQ1MnzudirsM` account holds the
one and only `559u4Tdr9umKwft3yHMsnAxohhzkFnUBPAFtibwuZD9z` token:

```console
$ spl-token account-info 559u4Tdr9umKwft3yHMsnAxohhzkFnUBPAFtibwuZD9z

Address: 7KqpRwzkkeweW5jQoETyLzhvs9rcCj9dVQ1MnzudirsM
Balance: 1
Mint: 559u4Tdr9umKwft3yHMsnAxohhzkFnUBPAFtibwuZD9z
Owner: vines1vzrYbzLMRdu58ou5XTby4qAqVRLmqo36NKPTg
State: Initialized
Delegation: (not set)
Close authority: (not set)
```

```console
$ spl-token supply 559u4Tdr9umKwft3yHMsnAxohhzkFnUBPAFtibwuZD9z
1
```

### Multisig usage

The main difference in `spl-token` command line usage when referencing multisig
accounts is in specifying the `--owner` argument. Typically the signer specified
by this argument directly provides a signature granting its authority, but in
the multisig case it just points to the address of the multisig account.
Signatures are then provided by the multisig signer-set members specified by the
`--multisig-signer` argument.

Multisig accounts can be used for any authority on an SPL Token mint or token
account.
- Mint account mint authority: `spl-token mint ...`, `spl-token authorize ... mint ...`
- Mint account freeze authority: `spl-token freeze ...`, `spl-token thaw ...`,
`spl-token authorize ... freeze ...`
- Token account owner authority: `spl-token transfer ...`, `spl-token approve ...`,
`spl-token revoke ...`, `spl-token burn ...`, `spl-token wrap ...`,
`spl-token unwrap ...`, `spl-token authorize ... owner ...`
- Token account close authority: `spl-token close ...`, `spl-token authorize ... close ...`

### Example: Mint with multisig authority

First create keypairs to act as the multisig signer-set. In reality, these can
be any supported signer, like: a Ledger hardware wallet, a keypair file, or
a paper wallet. For convenience, keypair files will be used in this example.
```console
$ for i in $(seq 3); do solana-keygen new --no-passphrase -so "signer-${i}.json"; done
Wrote new keypair to signer-1.json
Wrote new keypair to signer-2.json
Wrote new keypair to signer-3.json
```

In order to create the multisig account, the public keys of the signer-set must
be collected.
```console
$ for i in $(seq 3); do SIGNER="signer-${i}.json"; echo "$SIGNER: $(solana-keygen pubkey "$SIGNER")"; done
signer-1.json: BzWpkuRrwXHq4SSSFHa8FJf6DRQy4TaeoXnkA89vTgHZ
signer-2.json: DhkUfKgfZ8CF6PAGKwdABRL1VqkeNrTSRx8LZfpPFVNY
signer-3.json: D7ssXHrZJjfpZXsmDf8RwfPxe1BMMMmP1CtmX3WojPmG
```

Now the multisig account can be created with the `spl-token create-multisig`
subcommand. Its first positional argument is the minimum number of signers (`M`)
that must sign a transaction affecting a token/mint account that is controlled
by this multisig account. The remaining positional arguments are the public keys
of all keypairs allowed (`N`) to sign for the multisig account. This example
will use a "2 of 3" multisig account.  That is, two of the three allowed keypairs
must sign all transactions.

NOTE: SPL Token Multisig accounts are limited to a signer-set of eleven signers
(1 <= `N` <= 11) and minimum signers must be no more than `N` (1 <= `M` <= `N`)
```
$ spl-token create-multisig 2 BzWpkuRrwXHq4SSSFHa8FJf6DRQy4TaeoXnkA89vTgHZ \
DhkUfKgfZ8CF6PAGKwdABRL1VqkeNrTSRx8LZfpPFVNY D7ssXHrZJjfpZXsmDf8RwfPxe1BMMMmP1CtmX3WojPmG
```
```console
Creating 2/3 multisig 46ed77fd4WTN144q62BwjU2B3ogX3Xmmc8PT5Z3Xc2re
Signature: 2FN4KXnczAz33SAxwsuevqrD1BvikP6LUhLie5Lz4ETt594X8R7yvMZzZW2zjmFLPsLQNHsRuhQeumExHbnUGC9A
```

Next create the token mint and receiving accounts
[as previously described](#example-creating-your-own-fungible-token)
```console
$ spl-token create-token
Creating token 4VNVRJetwapjwYU8jf4qPgaCeD76wyz8DuNj8yMCQ62o
Signature: 3n6zmw3hS5Hyo5duuhnNvwjAbjzC42uzCA3TTsrgr9htUonzDUXdK1d8b8J77XoeSherqWQM8mD8E1TMYCpksS2r

$ spl-token create-account 4VNVRJetwapjwYU8jf4qPgaCeD76wyz8DuNj8yMCQ62o
Creating account EX8zyi2ZQUuoYtXd4MKmyHYLTjqFdWeuoTHcsTdJcKHC
Signature: 5mVes7wjE7avuFqzrmSCWneKBQyPAjasCLYZPNSkmqmk2YFosYWAP9hYSiZ7b7NKpV866x5gwyKbbppX3d8PcE9s
```

Then set the mint account's minting authority to the multisig account
```console
$ spl-token authorize 4VNVRJetwapjwYU8jf4qPgaCeD76wyz8DuNj8yMCQ62o mint 46ed77fd4WTN144q62BwjU2B3ogX3Xmmc8PT5Z3Xc2re
Updating 4VNVRJetwapjwYU8jf4qPgaCeD76wyz8DuNj8yMCQ62o
  Current mint authority: 5hbZyJ3KRuFvdy5QBxvE9KwK17hzkAUkQHZTxPbiWffE
  New mint authority: 46ed77fd4WTN144q62BwjU2B3ogX3Xmmc8PT5Z3Xc2re
Signature: yy7dJiTx1t7jvLPCRX5RQWxNRNtFwvARSfbMJG94QKEiNS4uZcp3GhhjnMgZ1CaWMWe4jVEMy9zQBoUhzomMaxC
```

To demonstrate that the mint account is now under control of the multisig
account, attempting to mint with one multisig signer fails
```
$ spl-token mint 4VNVRJetwapjwYU8jf4qPgaCeD76wyz8DuNj8yMCQ62o 1 EX8zyi2ZQUuoYtXd4MKmyHYLTjqFdWeuoTHcsTdJcKHC \
--owner 46ed77fd4WTN144q62BwjU2B3ogX3Xmmc8PT5Z3Xc2re \
--multisig-signer signer-1.json
```
```console
Minting 1 tokens
  Token: 4VNVRJetwapjwYU8jf4qPgaCeD76wyz8DuNj8yMCQ62o
  Recipient: EX8zyi2ZQUuoYtXd4MKmyHYLTjqFdWeuoTHcsTdJcKHC
RPC response error -32002: Transaction simulation failed: Error processing Instruction 0: missing required signature for instruction
```

But repeating with a second multisig signer, succeeds
```
$ spl-token mint 4VNVRJetwapjwYU8jf4qPgaCeD76wyz8DuNj8yMCQ62o 1 EX8zyi2ZQUuoYtXd4MKmyHYLTjqFdWeuoTHcsTdJcKHC \
--owner 46ed77fd4WTN144q62BwjU2B3ogX3Xmmc8PT5Z3Xc2re \
--multisig-signer signer-1.json \
--multisig-signer signer-2.json
```
```console
Minting 1 tokens
  Token: 4VNVRJetwapjwYU8jf4qPgaCeD76wyz8DuNj8yMCQ62o
  Recipient: EX8zyi2ZQUuoYtXd4MKmyHYLTjqFdWeuoTHcsTdJcKHC
Signature: 2ubqWqZb3ooDuc8FLaBkqZwzguhtMgQpgMAHhKsWcUzjy61qtJ7cZ1bfmYktKUfnbMYWTC1S8zdKgU6m4THsgspT
```

### Example: Offline signing with multisig 

Sometimes online signing is not possible or desireable. Such is the case for example when signers are not in the same geographic location  
or when they use air-gapped devices not connected to the network.  In this case, we use offline signing which combines the   
previous examples of [multisig](#example-mint-with-multisig-authority) with [offline signing](https://docs.solana.com/offline-signing)
and a [nonce account](https://docs.solana.com/offline-signing/durable-nonce).

This example will use the same mint account, token account, multisig account,
and multisig signer-set keypair filenames as the online example, as well as a nonce
account that we create here:

```console
$ solana-keygen new -o nonce-keypair.json
...
======================================================================
pubkey: Fjyud2VXixk2vCs4DkBpfpsq48d81rbEzh6deKt7WvPj
======================================================================
```

```console
$ solana create-nonce-account nonce-keypair.json 1
Signature: 3DALwrAAmCDxqeb4qXZ44WjpFcwVtgmJKhV4MW5qLJVtWeZ288j6Pzz1F4BmyPpnGLfx2P8MEJXmqPchX5y2Lf3r
```

```console
$ solana nonce-account Fjyud2VXixk2vCs4DkBpfpsq48d81rbEzh6deKt7WvPj
Balance: 0.01 SOL
Minimum Balance Required: 0.00144768 SOL
Nonce blockhash: 6DPt2TfFBG7sR4Hqu16fbMXPj8ddHKkbU4Y3EEEWrC2E
Fee: 5000 lamports per signature
Authority: 5hbZyJ3KRuFvdy5QBxvE9KwK17hzkAUkQHZTxPbiWffE
```

For the fee-payer and nonce-authority roles, a local hot wallet at
`5hbZyJ3KRuFvdy5QBxvE9KwK17hzkAUkQHZTxPbiWffE` will be used.

First a template command is built by specifying all signers by their public
key. Upon running this command, all signers will be listed as "Absent Signers"
in the output. This command will be run by each offline signer to generate the
corresponding signature.

NOTE: The argument to the `--blockhash` parameter is the "Nonce blockhash:" field from
the designated durable nonce account.

```
$ spl-token mint 4VNVRJetwapjwYU8jf4qPgaCeD76wyz8DuNj8yMCQ62o 1 EX8zyi2ZQUuoYtXd4MKmyHYLTjqFdWeuoTHcsTdJcKHC \
--owner 46ed77fd4WTN144q62BwjU2B3ogX3Xmmc8PT5Z3Xc2re \
--multisig-signer BzWpkuRrwXHq4SSSFHa8FJf6DRQy4TaeoXnkA89vTgHZ \
--multisig-signer DhkUfKgfZ8CF6PAGKwdABRL1VqkeNrTSRx8LZfpPFVNY \
--blockhash 6DPt2TfFBG7sR4Hqu16fbMXPj8ddHKkbU4Y3EEEWrC2E \
--fee-payer 5hbZyJ3KRuFvdy5QBxvE9KwK17hzkAUkQHZTxPbiWffE \
--nonce Fjyud2VXixk2vCs4DkBpfpsq48d81rbEzh6deKt7WvPj \
--nonce-authority 5hbZyJ3KRuFvdy5QBxvE9KwK17hzkAUkQHZTxPbiWffE \
--sign-only \
--mint-decimals 9
```

```console
Minting 1 tokens
  Token: 4VNVRJetwapjwYU8jf4qPgaCeD76wyz8DuNj8yMCQ62o
  Recipient: EX8zyi2ZQUuoYtXd4MKmyHYLTjqFdWeuoTHcsTdJcKHC

Blockhash: 6DPt2TfFBG7sR4Hqu16fbMXPj8ddHKkbU4Y3EEEWrC2E
Absent Signers (Pubkey):
 5hbZyJ3KRuFvdy5QBxvE9KwK17hzkAUkQHZTxPbiWffE
 BzWpkuRrwXHq4SSSFHa8FJf6DRQy4TaeoXnkA89vTgHZ
 DhkUfKgfZ8CF6PAGKwdABRL1VqkeNrTSRx8LZfpPFVNY
```

Next each offline signer executes the template command, replacing each instance
of their public key with the corresponding keypair.
```
$ spl-token mint 4VNVRJetwapjwYU8jf4qPgaCeD76wyz8DuNj8yMCQ62o 1 EX8zyi2ZQUuoYtXd4MKmyHYLTjqFdWeuoTHcsTdJcKHC \
--owner 46ed77fd4WTN144q62BwjU2B3ogX3Xmmc8PT5Z3Xc2re \
--multisig-signer signer-1.json \
--multisig-signer DhkUfKgfZ8CF6PAGKwdABRL1VqkeNrTSRx8LZfpPFVNY \
--blockhash 6DPt2TfFBG7sR4Hqu16fbMXPj8ddHKkbU4Y3EEEWrC2E \
--fee-payer 5hbZyJ3KRuFvdy5QBxvE9KwK17hzkAUkQHZTxPbiWffE \
--nonce Fjyud2VXixk2vCs4DkBpfpsq48d81rbEzh6deKt7WvPj \
--nonce-authority 5hbZyJ3KRuFvdy5QBxvE9KwK17hzkAUkQHZTxPbiWffE \
--sign-only \
--mint-decimals 9
```
```console
Minting 1 tokens
  Token: 4VNVRJetwapjwYU8jf4qPgaCeD76wyz8DuNj8yMCQ62o
  Recipient: EX8zyi2ZQUuoYtXd4MKmyHYLTjqFdWeuoTHcsTdJcKHC

Blockhash: 6DPt2TfFBG7sR4Hqu16fbMXPj8ddHKkbU4Y3EEEWrC2E
Signers (Pubkey=Signature):
 BzWpkuRrwXHq4SSSFHa8FJf6DRQy4TaeoXnkA89vTgHZ=2QVah9XtvPAuhDB2QwE7gNaY962DhrGP6uy9zeN4sTWvY2xDUUzce6zkQeuT3xg44wsgtUw2H5Rf8pEArPSzJvHX
Absent Signers (Pubkey):
 5hbZyJ3KRuFvdy5QBxvE9KwK17hzkAUkQHZTxPbiWffE
 DhkUfKgfZ8CF6PAGKwdABRL1VqkeNrTSRx8LZfpPFVNY
```

```
$ spl-token mint 4VNVRJetwapjwYU8jf4qPgaCeD76wyz8DuNj8yMCQ62o 1 EX8zyi2ZQUuoYtXd4MKmyHYLTjqFdWeuoTHcsTdJcKHC \
--owner 46ed77fd4WTN144q62BwjU2B3ogX3Xmmc8PT5Z3Xc2re \
--multisig-signer BzWpkuRrwXHq4SSSFHa8FJf6DRQy4TaeoXnkA89vTgHZ \
--multisig-signer signer-2.json \
--blockhash 6DPt2TfFBG7sR4Hqu16fbMXPj8ddHKkbU4Y3EEEWrC2E \
--fee-payer 5hbZyJ3KRuFvdy5QBxvE9KwK17hzkAUkQHZTxPbiWffE \
--nonce Fjyud2VXixk2vCs4DkBpfpsq48d81rbEzh6deKt7WvPj \
--nonce-authority 5hbZyJ3KRuFvdy5QBxvE9KwK17hzkAUkQHZTxPbiWffE \
--sign-only \
--mint-decimals 9
```
```console
Minting 1 tokens
  Token: 4VNVRJetwapjwYU8jf4qPgaCeD76wyz8DuNj8yMCQ62o
  Recipient: EX8zyi2ZQUuoYtXd4MKmyHYLTjqFdWeuoTHcsTdJcKHC

Blockhash: 6DPt2TfFBG7sR4Hqu16fbMXPj8ddHKkbU4Y3EEEWrC2E
Signers (Pubkey=Signature):
 DhkUfKgfZ8CF6PAGKwdABRL1VqkeNrTSRx8LZfpPFVNY=2brZbTiCfyVYSCp6vZE3p7qCDeFf3z1JFmJHPBrz8SnWSDZPjbpjsW2kxFHkktTNkhES3y6UULqS4eaWztLW7FrU
Absent Signers (Pubkey):
 5hbZyJ3KRuFvdy5QBxvE9KwK17hzkAUkQHZTxPbiWffE
 BzWpkuRrwXHq4SSSFHa8FJf6DRQy4TaeoXnkA89vTgHZ
```

Finally, the offline signers communicate the `Pubkey=Signature` pair from the
output of their command to the party who will broadcast the transaction to the
cluster. The broadcasting party then runs the template command after modifying
it as follows:
1. Replaces any corresponding public keys with their keypair (`--fee-payer ...`
and `--nonce-authority ...` in this example)
1. Removes the `--sign-only` argument, and in the case of the `mint` subcommand,
the `--mint-decimals ...` argument as it will be queried from the cluster
1. Adds the offline signatures to the template command via the `--signer` argument
```
$ spl-token mint 4VNVRJetwapjwYU8jf4qPgaCeD76wyz8DuNj8yMCQ62o 1 EX8zyi2ZQUuoYtXd4MKmyHYLTjqFdWeuoTHcsTdJcKHC \
--owner 46ed77fd4WTN144q62BwjU2B3ogX3Xmmc8PT5Z3Xc2re \
--multisig-signer BzWpkuRrwXHq4SSSFHa8FJf6DRQy4TaeoXnkA89vTgHZ \
--multisig-signer DhkUfKgfZ8CF6PAGKwdABRL1VqkeNrTSRx8LZfpPFVNY \
--blockhash 6DPt2TfFBG7sR4Hqu16fbMXPj8ddHKkbU4Y3EEEWrC2E \
--fee-payer hot-wallet.json \
--nonce Fjyud2VXixk2vCs4DkBpfpsq48d81rbEzh6deKt7WvPj \
--nonce-authority hot-wallet.json \
--signer BzWpkuRrwXHq4SSSFHa8FJf6DRQy4TaeoXnkA89vTgHZ=2QVah9XtvPAuhDB2QwE7gNaY962DhrGP6uy9zeN4sTWvY2xDUUzce6zkQeuT3xg44wsgtUw2H5Rf8pEArPSzJvHX \
--signer DhkUfKgfZ8CF6PAGKwdABRL1VqkeNrTSRx8LZfpPFVNY=2brZbTiCfyVYSCp6vZE3p7qCDeFf3z1JFmJHPBrz8SnWSDZPjbpjsW2kxFHkktTNkhES3y6UULqS4eaWztLW7FrU
```
```console
Minting 1 tokens
  Token: 4VNVRJetwapjwYU8jf4qPgaCeD76wyz8DuNj8yMCQ62o
  Recipient: EX8zyi2ZQUuoYtXd4MKmyHYLTjqFdWeuoTHcsTdJcKHC
Signature: 2AhZXVPDBVBxTQLJohyH1wAhkkSuxRiYKomSSXtwhPL9AdF3wmhrrJGD7WgvZjBPLZUFqWrockzPp9S3fvzbgicy
```

## JSON RPC methods

There is a rich set of JSON RPC methods available for use with SPL Token:
* `getTokenAccountBalance`
* `getTokenAccountsByDelegate`
* `getTokenAccountsByOwner`
* `getTokenLargestAccounts`
* `getTokenSupply`

See https://docs.solana.com/apps/jsonrpc-api for more details.

Additionally the versatile `getProgramAccounts` JSON RPC method can be employed in various ways to fetch SPL Token accounts of interest.

### Finding all token accounts for a specific mint

To find all token accounts for the `TESTpKgj42ya3st2SQTKiANjTBmncQSCqLAZGcSPLGM` mint:
```
curl http://api.mainnet-beta.solana.com -X POST -H "Content-Type: application/json" -d '
  {
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getProgramAccounts",
    "params": [
      "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
      {
        "encoding": "jsonParsed",
        "filters": [
          {
            "dataSize": 165
          },
          {
            "memcmp": {
              "offset": 0,
              "bytes": "TESTpKgj42ya3st2SQTKiANjTBmncQSCqLAZGcSPLGM"
            }
          }
        ]
      }
    ]
  }
'
```

The `"dataSize": 165` filter selects all [Token
Account](https://github.com/solana-labs/solana-program-library/blob/08d9999f997a8bf38719679be9d572f119d0d960/token/program/src/state.rs#L86-L106)s,
and then the `"memcmp": ...` filter selects based on the
[mint](https://github.com/solana-labs/solana-program-library/blob/08d9999f997a8bf38719679be9d572f119d0d960/token/program/src/state.rs#L88)
address within each token account.

### Finding all token accounts for a wallet

Find all token accounts owned by the `vines1vzrYbzLMRdu58ou5XTby4qAqVRLmqo36NKPTg` user:
```
curl http://api.mainnet-beta.solana.com -X POST -H "Content-Type: application/json" -d '
  {
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getProgramAccounts",
    "params": [
      "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
      {
        "encoding": "jsonParsed",
        "filters": [
          {
            "dataSize": 165
          },
          {
            "memcmp": {
              "offset": 32,
              "bytes": "vines1vzrYbzLMRdu58ou5XTby4qAqVRLmqo36NKPTg"
            }
          }
        ]
      }
    ]
  }
```

The `"dataSize": 165` filter selects all [Token
Account](https://github.com/solana-labs/solana-program-library/blob/08d9999f997a8bf38719679be9d572f119d0d960/token/program/src/state.rs#L86-L106)s,
and then the `"memcmp": ...` filter selects based on the
[owner](https://github.com/solana-labs/solana-program-library/blob/08d9999f997a8bf38719679be9d572f119d0d960/token/program/src/state.rs#L90)
address within each token account.

## Operational overview

### Creating a new token type

A new token type can be created by initializing a new Mint with the
`InitializeMint` instruction. The Mint is used to create or "mint" new tokens,
and these tokens are stored in Accounts. A Mint is associated with each
Account, which means that the total supply of a particular token type is equal
to the balances of all the associated Accounts.

It's important to note that the `InitializeMint` instruction does not require
the Solana account being initialized also be a signer. The `InitializeMint`
instruction should be atomically processed with the system instruction that
creates the Solana account by including both instructions in the same
transaction.

Once a Mint is initialized, the `mint_authority` can create new tokens using the
`MintTo` instruction.  As long as a Mint contains a valid `mint_authority`, the
Mint is considered to have a non-fixed supply, and the `mint_authority` can
create new tokens with the `MintTo` instruction at any time.  The `SetAuthority`
instruction can be used to irreversibly set the Mint's authority to `None`,
rendering the Mint's supply fixed.  No further tokens can ever be Minted.

Token supply can be reduced at any time by issuing a `Burn` instruction which
removes and discards tokens from an Account.

### Creating accounts

Accounts hold token balances and are created using the `InitializeAccount`
instruction. Each Account has an owner who must be present as a signer in some
instructions.

An Account's owner may transfer ownership of an account to another using the
`SetAuthority` instruction.

It's important to note that the `InitializeAccount` instruction does not require
the Solana account being initialized also be a signer. The `InitializeAccount`
instruction should be atomically processed with the system instruction that
creates the Solana account by including both instructions in the same
transaction.

### Transferring tokens
Balances can be transferred between Accounts using the `Transfer` instruction.
The owner of the source Account must be present as a signer in the `Transfer`
instruction when the source and destination accounts are different.

It's important to note that when the source and destination of a `Transfer` are
the **same**, the `Transfer` will _always_ succeed. Therefore, a successful `Transfer`
does not necessarily imply that the involved Accounts were valid SPL Token
accounts, that any tokens were moved, or that the source Account was present as
a signer. We strongly recommend that developers are careful about checking that
the source and destination are **different** before invoking a `Transfer`
instruction from within their program.

### Burning

The `Burn` instruction decreases an Account's token balance without transferring
to another Account, effectively removing the token from circulation permanently.

There is no other way to reduce supply on chain. This is similar to transferring
to an account with unknown private key or destroying a private key. But the act
of burning by using `Burn` instructions is more explicit and can be confirmed on
chain by any parties.

### Authority delegation

Account owners may delegate authority over some or all of their token balance
using the `Approve` instruction. Delegated authorities may transfer or burn up
to the amount they've been delegated. Authority delegation may be revoked by the
Account's owner via the `Revoke` instruction.

### Multisignatures

M of N multisignatures are supported and can be used in place of Mint
authorities or Account owners or delegates. Multisignature authorities must be
initialized with the `InitializeMultisig` instruction. Initialization specifies
the set of N public keys that are valid and the number M of those N that must be
present as instruction signers for the authority to be legitimate.

It's important to note that the `InitializeMultisig` instruction does not
require the Solana account being initialized also be a signer. The
`InitializeMultisig` instruction should be atomically processed with the system
instruction that creates the Solana account by including both instructions in
the same transaction.

### Freezing accounts

The Mint may also contain a `freeze_authority` which can be used to issue
`FreezeAccount` instructions that will render an Account unusable.  Token
instructions that include a frozen account will fail until the Account is thawed
using the `ThawAccount` instruction.  The `SetAuthority` instruction can be used
to change a Mint's `freeze_authority`.  If a Mint's `freeze_authority` is set to
`None` then account freezing and thawing is permanently disabled and all
currently frozen accounts will also stay frozen permanently.

### Wrapping SOL

The Token Program can be used to wrap native SOL. Doing so allows native SOL to
be treated like any other Token program token type and can be useful when being
called from other programs that interact with the Token Program's interface.

Accounts containing wrapped SOL are associated with a specific Mint called the
"Native Mint" using the public key
`So11111111111111111111111111111111111111112`.

These accounts have a few unique behaviors

- `InitializeAccount` sets the balance of the initialized Account to the SOL
  balance of the Solana account being initialized, resulting in a token balance
  equal to the SOL balance.
- Transfers to and from not only modify the token balance but also transfer an
  equal amount of SOL from the source account to the destination account.
- Burning is not supported
- When closing an Account the balance may be non-zero.

The Native Mint supply will always report 0, regardless of how much SOL is currently wrapped.

### Rent-exemption

To ensure a reliable calculation of supply, a consistency valid Mint, and
consistently valid Multisig accounts all Solana accounts holding an Account,
Mint, or Multisig must contain enough SOL to be considered [rent
exempt](https://docs.solana.com/implemented-proposals/rent)

### Closing accounts

An account may be closed using the `CloseAccount` instruction. When closing an
Account, all remaining SOL will be transferred to another Solana account
(doesn't have to be associated with the Token Program). Non-native Accounts must
have a balance of zero to be closed.

### Non-Fungible tokens
An NFT is simply a token type where only a single token has been minted.

## Wallet Integration Guide
This section describes how to integrate SPL Token support into an existing
wallet supporting native SOL.  It assumes a model whereby the user has a single
system account as their **main wallet address** that they send and receive SOL
from.

Although all SPL Token accounts do have their own address on-chain, there's no
need to surface these additional addresses to the user.

There are two programs that are used by the wallet:
* SPL Token program: generic program that is used by all SPL Tokens
* [SPL Associated Token Account](associated-token-account.md) program: defines
  the convention and provides the mechanism for mapping the user's wallet
  address to the associated token accounts they hold.

### How to fetch and display token holdings
The [getTokenAccountsByOwner](https://docs.solana.com/apps/jsonrpc-api#gettokenaccountsbyowner)
JSON RPC method can be used to fetch all token accounts for a wallet address.

For each token mint, the wallet could have multiple token accounts: the
associated token account and/or other ancillary token accounts

By convention it is suggested that wallets roll up the balances from all token
accounts of the same token mint into a single balance for the user to shield the
user from this complexity.

See the [Garbage Collecting Ancillary Token Accounts](#garbage-collecting-ancillary-token-accounts)
section for suggestions on how the wallet should clean up ancillary token accounts on the user's behalf.

### Associated Token Account
Before the user can receive tokens, their associated token account must be created
on-chain, requiring a small amount of SOL to mark the account as rent-exempt.

There's no restriction on who can create a user's associated token account.  It
could either be created by the wallet on behalf of the user or funded by a 3rd
party through an airdrop campaign.

The creation process is described [here](associated-token-account.md#creating-an-associated-token-account).

It's highly recommended that the wallet create the associated token account for
a given SPL Token itself before indicating to the user that they are able to
receive that SPL Tokens type (typically done by showing the user their receiving
address). A wallet that chooses to not perform this step may limit its user's ability
to receive SPL Tokens from other wallets.

#### Sample "Add Token" workflow
The user should first fund their associated token account when they want to
receive SPL Tokens of a certain type to:
1. Maximize interoperability with other wallet implementations
2. Avoid pushing the cost of creating their associated token account on the first sender

The wallet should provide a UI that allow the users to "add a token".
The user selects the kind of token, and is presented with information about how
much SOL it will cost to add the token.

Upon confirmation, the wallet creates the associated token type as the described
[here](associated-token-account.md#creating-an-associated-token-account).

#### Sample "Airdrop campaign" workflow
For each recipient wallet addresses, send a transaction containing:
1. Create the associated token account on the recipient's behalf.
2. Use `TokenInstruction::Transfer` to complete the transfer

#### Associated Token Account Ownership
⚠️ The wallet should never use `TokenInstruction::SetAuthority` to set the
`AccountOwner` authority of the associated token account to another address.

### Ancillary Token Accounts
At any time ownership of an existing SPL Token account may be assigned to the
user.  One way to accomplish this is with the
`spl-token authorize <TOKEN_ADDRESS> owner <USER_ADDRESS>` command.  Wallets
should be prepared to gracefully manage token accounts that they themselves did
not create for the user.

### Transferring Tokens Between Wallets
The preferred method of transferring tokens between wallets is to transfer into
associated token account of the recipient.

The recipient must provide their main wallet address to the sender.  The sender
then:
1. Derives the associated token account for the recipient
1. Fetches the recipient's associated token account over RPC and checks that it exists
1. If the recipient's associated token account does not yet exist, the sender
   wallet should create the recipient's associated token account as described
   [here](associated-token-account.md#creating-an-associated-token-account).
   The sender's wallet may choose to inform the user that as a result of account
   creation the transfer will require more SOL than normal.
   However a wallet that chooses to not support creating the recipient's
   associated token account at this time should present a message to the user with enough
   information to permit them to find a workaround (such as transferring the
   token through a fully compliant intermediary wallet such as https://sollet.io)
   to allow the users to accomplish their goal
1. Use `TokenInstruction::Transfer` to complete the transfer

The sender's wallet must not require that the recipient's main wallet address
hold a balance before allowing the transfer.

### Registry for token details
At the moment there exist two solutions for Token Mint registries:

* hard coded addresses in the wallet or dapp
* [spl-token-registry](https://www.npmjs.com/package/@solana/spl-token-registry)
package, maintained at [https://github.com/solana-labs/token-list](https://github.com/solana-labs/token-list)

**A decentralized solution is in progress.**

### Garbage Collecting Ancillary Token Accounts
Wallets should empty ancillary token accounts as quickly as practical by
transferring into the user's associated token account.  This effort serves two
purposes:
* If the user is the close authority for the ancillary account, the wallet can
  reclaim SOL for the user by closing the account.
* If the ancillary account was funded by a 3rd party, once the account is
  emptied that 3rd party may close the account and reclaim the SOL.

One natural time to garbage collect ancillary token accounts is when the user
next sends tokens.  The additional instructions to do so can be added to the
existing transaction, and will not require an additional fee.

Cleanup Pseudo Steps:
1. For all non-empty ancillary token accounts, add a
   `TokenInstruction::Transfer` instruction to the transfer the full token
   amount to the user's associated token account.
2. For all empty ancillary token accounts where the user is the close authority,
   add a `TokenInstruction::CloseAccount` instruction

If adding one or more of clean up instructions cause the transaction to exceed
the maximum allowed transaction size, remove those extra clean up instructions.
They can be cleaned up during the next send operation.

The `spl-token gc` command provides an example implementation of this cleanup process.


### Token Vesting Contract:
This program allows you to lock arbitrary SPL tokens and release the locked tokens with a determined unlock schedule. An `unlock schedule` is made of a `unix timestamp` and a token `amount`, when initializing a vesting contract, the creator can pass an array of `unlock schedule` with an arbitrary size giving the creator of the contract complete control of how the tokens unlock over time.

Unlocking works by pushing a permissionless crank on the contract that moves the tokens to the pre-specified address. The recipient address of a vesting contract can be modified by the owner of the current recipient key, meaning that vesting contract locked tokens can be traded.

- Code: [https://github.com/Bonfida/token-vesting](https://github.com/Bonfida/token-vesting)
- UI: [https://vesting.bonfida.com/#/](https://vesting.bonfida.com/#/)
- Audit: The audit was conducted by Kudelski, the report can be found [here](https://github.com/Bonfida/token-vesting/blob/master/audit/Bonfida_SecurityAssessment_Vesting_Final050521.pdf)
