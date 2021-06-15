---
title: Stake Pool Program
---

A program for pooling together SOL to be staked by an off-chain agent running
a Delegation Bot which redistributes the stakes across the network and tries
to maximize censorship resistance and rewards.

## Overview

SOL token holders can earn rewards and help secure the network by staking tokens
to one or more validators. Rewards for staked tokens are based on the current
inflation rate, total number of SOL staked on the network, and an individual
validator’s uptime and commission (fee).

Stake pools are an alternative method of earning staking rewards. This on-chain
program pools together SOL to be staked by a staker, allowing SOL holders to
stake and earn rewards without managing stakes.

Additional information regarding staking and stake programming is available at:

- https://solana.com/staking
- https://docs.solana.com/staking/stake-programming

## Motivation

This document is intended for the main actors of the stake pool system:

* manager: creates and manages the stake pool, earns fees, can update the fee, staker, and manager
* staker: adds and removes validators to the pool, rebalances stake among validators
* user: provides staked SOL into an existing stake pool

In its current iteration, the stake pool only processes totally active stakes.
Deposits must come from fully active stakes, and withdrawals return a fully
active stake account.

This means that stake pool managers, stakers, and users must be comfortable with
creating and delegating stakes, which are more advanced operations than sending and
receiving SPL tokens and SOL. Additional information on stake operations are
available at:

- https://docs.solana.com/cli/delegate-stake
- https://docs.solana.com/cli/manage-stake-accounts

To reach a wider audience of users, stake pool managers are encouraged
to provide a market for their pool's staking derivatives, through an AMM
like [Token Swap](token-swap.md).

## Operation

A stake pool manager creates a stake pool, and the staker includes validators that will
receive delegations from the pool by creating "validator stake accounts" and
activating a delegation on them. Once a validator stake account's delegation is
active, the staker adds it to the stake pool.

At this point, users can participate with deposits. They must delegate a stake
account to the one of the validators in the stake pool. Once it's active, the
user can deposit their stake into the pool in exchange for SPL staking derivatives
representing their fractional ownership in pool. A percentage of the rewards
earned by the pool goes to the pool manager as a fee.

Over time, as the stakes in the stake pool accrue staking rewards, the user's fractional
ownership will be worth more than their initial deposit. Whenever the user chooses,
they can withdraw their SPL staking derivatives in exchange for an activated stake.

The stake pool staker can add and remove validators, or rebalance the pool by
decreasing the stake on a validator, waiting an epoch to move it into the stake
pool's reserve account, then increasing the stake on another validator.

The staker operation to add a new validator requires roughly 1.003 SOL to create
the stake account on a validator, so the stake pool staker will need liquidity
on hand to fully manage the pool stakes.

## Background

Solana's programming model and the definitions of the Solana terms used in this
document are available at:

- https://docs.solana.com/apps
- https://docs.solana.com/terminology

## Source

The Stake Pool Program's source is available on
[github](https://github.com/solana-labs/solana-program-library).

## Command-line Utility

The following explains the instructions available in the Stake Pool Program along
with examples using the command-line utility.

The `spl-stake-pool` command-line utility can be used to experiment with SPL
tokens.  Once you have [Rust installed](https://rustup.rs/), run:
```sh
$ cargo install spl-stake-pool-cli
```

Run `spl-stake-pool --help` for a full description of available commands.

### Configuration

The `spl-stake-pool` configuration is shared with the `solana` command-line tool.

#### Current Configuration

```sh
solana config get
Config File: ${HOME}/.config/solana/cli/config.yml
RPC URL: https://api.mainnet-beta.solana.com
WebSocket URL: wss://api.mainnet-beta.solana.com/ (computed)
Keypair Path: ${HOME}/.config/solana/id.json
```

#### Cluster RPC URL

See [Solana clusters](https://docs.solana.com/clusters) for cluster-specific RPC URLs
```sh
solana config set --url https://api.devnet.solana.com
```

#### Default Keypair

See [Keypair conventions](https://docs.solana.com/cli/conventions#keypair-conventions)
for information on how to setup a keypair if you don't already have one.

Keypair File
```sh
solana config set --keypair ${HOME}/new-keypair.json
```

Hardware Wallet URL (See [URL spec](https://docs.solana.com/wallet-guide/hardware-wallets#specify-a-keypair-url))
```sh
solana config set --keypair usb://ledger/
```

#### Run Locally

If you would like to test a stake pool locally without having to wait for stakes
to activate and deactivate, you can run the stake pool locally using the
`solana-test-validator` tool with shorter epochs, and pulling the current program
from devnet, testnet, or mainnet.

```sh
$ solana-test-validator -c poo1B9L9nR3CrcaziKVYVpRX6A9Y1LAXYasjjfCbApj -c 5TfMPP2zwrXWTUvkg5AG54QWpEkwjeBUhpP7x99kkvEj --url devnet --slots-per-epoch 32
$ solana config set --url http://127.0.0.1:8899
```

### Stake Pool Manager Examples

#### Create a stake pool

The stake pool manager controls the stake pool from a high level, and in exchange
receives a fee in the form of SPL token staking derivatives. The manager
sets the fee on creation. Let's create a pool with a 3% fee and a maximum of 1000
validator stake accounts:

```sh
$ spl-stake-pool create-pool --fee-numerator 3 --fee-denominator 100 --max-validators 1000
Creating reserve stake 33Hg3bvYrAwfqCzTMjAWZNAWC6H96qJNEdzGamfFjG4J
Creating mint D5yiK1tE1yAXBnrV9ZrSUJCw8WiQctZ8ekbv1U6ATVZ
Creating pool fee collection account 5gpuSdutGY98KKbgmR5CfLK7toFcQD69JzKDwseegzXE
Signature: 2dvCtHMcqxibckhvVgFQeFCRb7VcHbuFLRf71Aqd9PtzFzdbG3gAkNpxYznfpKDx2vTRrVtwW81sZAx5U3Frb5Uu
Creating stake pool EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1
Signature: 2kYDVyJp8FVrLmEZyW9ivMYcXEsgWm4hFyhp5omxVtonjhYG6WS1S85sPTCdsQWe3idof6ZqsY8F3oaMXwrEkAYK
```

The unique stake pool identifier is `EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1`.

The identifier for the SPL token for staking derivatives is
`D5yiK1tE1yAXBnrV9ZrSUJCw8WiQctZ8ekbv1U6ATVZ`. The stake pool has full control
over the mint.

The pool creator's fee account identifier is
`5gpuSdutGY98KKbgmR5CfLK7toFcQD69JzKDwseegzXE`. Every epoch, as stake accounts
in the stake pool earn rewards, the program will mint SPL token staking derivatives
equal to 3% of the gains on that epoch into this account. If no gains were observed,
nothing will be deposited.

The reserve stake account identifier is `33Hg3bvYrAwfqCzTMjAWZNAWC6H96qJNEdzGamfFjG4J`.
This account holds onto additional stake used when rebalancing between validators.

For a stake pool with 1000 validators, the cost to create a stake pool is less
than 0.5 SOL.

#### Set manager

The stake pool manager may pass their administrator privileges to another account.

```sh
$ spl-stake-pool set-manager EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1 --new-manager 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Signature: 39N5gkaqXuWm6JPEUWfenKXeG4nSa71p7iHb9zurvdZcsWmbjdmSXwLVYfhAVHWucTY77sJ8SkUNpVpVAhe4eZ53
```

At the same time, they may also change the SPL token account that receives fees
every epoch. The mint for the provided token account must be the SPL token mint,
`D5yiK1tE1yAXBnrV9ZrSUJCw8WiQctZ8ekbv1U6ATVZ` in our example.

```sh
$ spl-stake-pool set-manager EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1 --new-fee-receiver HoCsh97wRxRXVjtG7dyfsXSwH9VxdDzC7GvAsBE1eqJz
Signature: 4aK8yzYvPBkP4PyuXTcCm529kjEH6tTt4ixc5D5ZyCrHwc4pvxAHj6wcr4cpAE1e3LddE87J1GLD466aiifcXoAY
```

#### Set fee

The stake pool manager may update the fee assessed every epoch, passing the
numerator and denominator for the fraction that make up the fee. For a fee of
10%, they could run:

```sh
$ spl-stake-pool set-fee EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1 10 100
Signature: 5yPXfVj5cbKBfZiEVi2UR5bXzVDuc2c3ruBwSjkAqpvxPHigwGHiS1mXQVE4qwok5moMWT5RNYAMvkE9bnfQ1i93
```

In order to protect stake pool depositors from malicious managers, the program
applies the new fee for the following epoch. For example, if the fee is 1% at
epoch 100, and the manager sets it to 10%, the manager will still gain 1% for
the rewards earned during epoch 100. Starting with epoch 101, the manager will
earn 10%.

#### Set staker

In order to manage the stake accounts, the stake pool manager or
staker can set the staker authority of the stake pool's managed accounts.

```sh
$ spl-stake-pool set-staker EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Signature: 39N5gkaqXuWm6JPEUWfenKXeG4nSa71p7iHb9zurvdZcsWmbjdmSXwLVYfhAVHWucTY77sJ8SkUNpVpVAhe4eZ53
```

Now, the new staker can perform any normal stake pool operations, including
adding and removing validators and rebalancing stake.

Important security note: the stake pool program only gives staking authority to
the pool staker and always retains withdraw authority. Therefore, a malicious
stake pool staker cannot steal funds from the stake pool.

Note: to avoid "disturbing the manager", the staker can also reassign their stake
authority.

### Stake Pool Staker Examples

#### Create a validator stake account

In order to accommodate large numbers of user deposits into the stake pool, the
stake pool only manages one stake account per validator. To add a new validator
to the stake pool, we first create a validator-associated stake account.

Looking at [validators.app](https://www.validators.app/) or other Solana validator
lists, we choose some validators at random and start with identity
`8SQEcP4FaYQySktNQeyxF3w8pvArx3oMEh7fPrzkN9pu` on vote account
`2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3`. Let's create a validator stake account
delegated to that vote account.

```sh
$ spl-stake-pool create-validator-stake EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1 2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3
Creating stake account FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN
Signature: 4pA2WKT6d2wkXEtSpiQswv22WyoFad2KX6FdPEzwBiEquvaUBEtzenys5Jh1ABPCh7yc4w8kzqMRRCwDj6ZSUV1K
```

In order to maximize censorship resistance, we want to distribute our SOL to as
many validators as possible, so let's add a few more.

```sh
$ spl-stake-pool create-validator-stake EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1 HJiC8iJ4Sj846SswQuauFJK93UvV6zp3c2T6jzGqzhhz
Creating stake account E5KBATUd21Dnjnh5sGFw5ngp9kdVXCcAAYMRe2WsVXie
Signature: 4pyRZzjsWG7jP3GRZeZCo2Eb2TPjHM4kAYRFMivimme6HAee1nhzoNJBe3VSt2sv7acp5fwT7J8omBM8o3niY8gu
$ spl-stake-pool create-validator-stake EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1 AUCzCaGAGjL3uyjFBtJs7KuJcgQWvNZu1Z2S9G3pw77G
Creating stake account CrStLEWfme37kDc3nubK9HsmWR5dsuVUuqEKqTR4Mc5E
Signature: 4ZUdZzUARgUCPuY8nVsJbN6vRDbVX8sYAQGYYXj2YVvjoJ2oevq2H8uzrhYApe419uoP7QYukqNstiti5p5DDukN
$ spl-stake-pool create-validator-stake EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1 8r1f8mwrUiYdg2Rx9sxTh4M3UAUcCBBrmRA3nxk3Z6Lm
Creating stake account FhFft7ArhZZkh6q4ir1JZMYFgXdH6wkT5M5nmDDb1Q13
Signature: yQqXCbuA66wQsHtkziNg3XadfZF5aCmvjfentwbZJnSPeEjJwPka3M1QY5GmR1efprptqaePn71BTMSLscX8DLr
```

NOTE: These stake accounts have not been added to the stake pool yet. Stake pools
only accept deposits from fully delegated (warmed-up) stake accounts, so we must
first delegate these stakes.

We can see the status of stake account using the Solana command-line utility.

```sh
$ solana stake-account FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN
Balance: 1.002282880 SOL
Rent Exempt Reserve: 0.00228288 SOL
Stake account is undelegated
Stake Authority: 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Withdraw Authority: 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
```

The stake pool creates these special staking accounts with 1 SOL as a minimum
delegation. The stake and withdraw authorities are the keypair configured
with the `--config` flag, using the Solana CLI default key. More information
about the Solana CLI can be found on the
[Solana Docs](https://docs.solana.com/running-validator/validator-start#configure-solana-cli).

We must delegate these stake accounts to the vote account specified on creation.

```sh
$ solana delegate-stake FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN 2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3
Signature: 2H9oiPJQ2fRihPqvjc62pHwBi8VcK1LQFJTLvdJR2pAhGWQcLXQpMoHiDgCLPE78Kxy9JPbvihsGeC8yCZHCdpWG
$ solana delegate-stake E5KBATUd21Dnjnh5sGFw5ngp9kdVXCcAAYMRe2WsVXie HJiC8iJ4Sj846SswQuauFJK93UvV6zp3c2T6jzGqzhhz
Signature: 3NZfYFMheSVZJxuLMvW9QsqdVJxsBj5Aa8huGfCTzojQeP2nCuGGYGn81pPiumpKefcjKRSz2LSsnzJQN3aCUG77
$ solana delegate-stake CrStLEWfme37kDc3nubK9HsmWR5dsuVUuqEKqTR4Mc5E AUCzCaGAGjL3uyjFBtJs7KuJcgQWvNZu1Z2S9G3pw77G
Signature: Jhj5wgbn6rvmkZRdfNS2uEwRyAnZS3LUpANyCvFXgeVigw3L5gumZTyvpPvE6nyN7MfPLqnX9yfYcAFN8i8NJmT
$ solana delegate-stake FhFft7ArhZZkh6q4ir1JZMYFgXdH6wkT5M5nmDDb1Q13 8r1f8mwrUiYdg2Rx9sxTh4M3UAUcCBBrmRA3nxk3Z6Lm
Signature: 5Xg7d5v2bjgVc4o1T8dU9JBHTssb8CR9J4XW1oXxuAPJ72F7ANFcxuB81r9ky7GbyKwUPJbbF7Gvpgch6623wjFA
```

Now that we have delegated the stakes, we need to wait an epoch for the delegation
to activate.

#### Add validator stake account

As mentioned in the last step, the stake pool only manages one stake account per
validator. Also, the stake pool only processes fully activated stake accounts.
We created new validator stake accounts in the last step and staked them. Once
the stake activates, we can add them to the stake pool.

```sh
$ spl-stake-pool add-validator EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1 2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3
Signature: 3N1K89rGV9gWueTTrPGTDBwKAp8BikQhKHMFoREw98Q1piXFeZSSxqfnRQexrfAZQfrpYH9qwsaPWRruwkVeBivV
```

Users can start depositing their activated stakes into the stake pool, as
long as they are delegated to the same vote account, which was
`FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN` in this example.  You can also
double-check that at any time using the Solana command-line utility.

```sh
$ solana stake-account FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN
Balance: 0.002282881 SOL
Rent Exempt Reserve: 0.00228288 SOL
Delegated Stake: 1.000000000 SOL
Active Stake: 1.000000000 SOL
Activating Stake: 0 SOL
Stake activates starting from epoch: 161
Delegated Vote Account Address: 2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3
Stake Authority: 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Withdraw Authority: 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
```

#### Remove validator stake account

If the stake pool staker wants to stop delegating to a vote account, they can
totally remove the validator stake account from the stake pool.

```sh
$ spl-stake-pool remove-validator EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1 AUCzCaGAGjL3uyjFBtJs7KuJcgQWvNZu1Z2S9G3pw77G
Signature: 5rrQ3xhDWyiPkUTAQkNAeq31n6sMf1xsg2x9hVY8Vj1NonwBnhxuTv87nADLkwC8Xzc4CGTNCTX2Vph9esWnXk2d
```

The difference with `withdraw` is that the validator stake account is totally
removed from the stake pool and now belongs to the administrator. The authority
for the withdrawn stake account can also be specified using the `--new-authority` flag:

```sh
$ spl-stake-pool remove-validator EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1 AUCzCaGAGjL3uyjFBtJs7KuJcgQWvNZu1Z2S9G3pw77G --new-authority 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Signature: 5rrQ3xhDWyiPkUTAQkNAeq31n6sMf1xsg2x9hVY8Vj1NonwBnhxuTv87nADLkwC8Xzc4CGTNCTX2Vph9esWnXk2d
```

We can check the removed stake account:

```sh
$ solana stake-account CrStLEWfme37kDc3nubK9HsmWR5dsuVUuqEKqTR4Mc5E
Balance: 1.002282880 SOL
Rent Exempt Reserve: 0.00228288 SOL
Delegated Stake: 1.000000000 SOL
Active Stake: 1.000000000 SOL
Delegated Vote Account Address: AUCzCaGAGjL3uyjFBtJs7KuJcgQWvNZu1Z2S9G3pw77G
Stake Authority: 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Withdraw Authority: 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
```

The administrator's SPL token account has been debited to accommodate the
removal of staked SOL from the pool.

We can also double-check that the stake pool no longer shows the stake account:

```sh
$ spl-stake-pool list EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1
Pubkey: FhFft7ArhZZkh6q4ir1JZMYFgXdH6wkT5M5nmDDb1Q13    Vote: 8r1f8mwrUiYdg2Rx9sxTh4M3UAUcCBBrmRA3nxk3Z6Lm ◎1.002282881
Pubkey: FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN    Vote: 2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3 ◎3.410872673
Pubkey: E5KBATUd21Dnjnh5sGFw5ngp9kdVXCcAAYMRe2WsVXie    Vote: HJiC8iJ4Sj846SswQuauFJK93UvV6zp3c2T6jzGqzhhz ◎11.436803652
Total: ◎15.849959206
```

#### Rebalance the stake pool

As time goes on, users will deposit to and withdraw from all of the stake accounts
managed by the pool, and the stake pool staker may want to rebalance the stakes.

For example, let's say the staker wants the same delegation to every validator
in the pool. When they look at the state of the pool, they see:

```sh
$ spl-stake-pool list EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1
Pubkey: FhFft7ArhZZkh6q4ir1JZMYFgXdH6wkT5M5nmDDb1Q13    Vote: 8r1f8mwrUiYdg2Rx9sxTh4M3UAUcCBBrmRA3nxk3Z6Lm ◎1.002282881
Pubkey: FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN    Vote: 2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3 ◎3.410872673
Pubkey: E5KBATUd21Dnjnh5sGFw5ngp9kdVXCcAAYMRe2WsVXie    Vote: HJiC8iJ4Sj846SswQuauFJK93UvV6zp3c2T6jzGqzhhz ◎11.436803652
Total: ◎15.849959206
```

This isn't great! The last stake account, `E5KBATUd21Dnjnh5sGFw5ngp9kdVXCcAAYMRe2WsVXie`
has too much allocated. For their strategy, the staker wants the `15.849959206`
SOL to be distributed evenly, meaning around `5.283319735` in each account. They need
to move `4.281036854` to `FhFft7ArhZZkh6q4ir1JZMYFgXdH6wkT5M5nmDDb1Q13` and
`1.872447062` to `FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN`.

##### Decrease validator stake

First, they need to decrease the amount on stake account
`E5KBATUd21Dnjnh5sGFw5ngp9kdVXCcAAYMRe2WsVXie`, delegated to
`HJiC8iJ4Sj846SswQuauFJK93UvV6zp3c2T6jzGqzhhz`, by total of `6.153483916` SOL.

They decrease that amount of SOL:

```sh
$ spl-stake-pool decrease-validator-stake EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1 HJiC8iJ4Sj846SswQuauFJK93UvV6zp3c2T6jzGqzhhz 6.153483916
Signature: ZpQGwT85rJ8Y9afdkXhKo3TVv4xgTz741mmZj2vW7mihYseAkFsazWxza2y8eNGY4HDJm15c1cStwyiQzaM3RpH
```

Internally, this instruction splits and deactivates 6.153483916 SOL from the
validator stake account `E5KBATUd21Dnjnh5sGFw5ngp9kdVXCcAAYMRe2WsVXie` into a
transient stake account, owned and managed entirely by the stake pool.

Once the stake is deactivated during the next epoch, the `update` command will
automatically merge the transient stake account into a reserve stake account,
also entirely owned and managed by the stake pool.

##### Increase validator stake

Now that the reserve stake account has enough to perform the rebalance, the staker
can increase the stake on the two other validators,
`8r1f8mwrUiYdg2Rx9sxTh4M3UAUcCBBrmRA3nxk3Z6Lm` and
`2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3`.

They add 4.281036854 SOL to `8r1f8mwrUiYdg2Rx9sxTh4M3UAUcCBBrmRA3nxk3Z6Lm`:

```sh
$ spl-stake-pool increase-validator-stake EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1 8r1f8mwrUiYdg2Rx9sxTh4M3UAUcCBBrmRA3nxk3Z6Lm 4.281036854
Signature: 3GJACzjUGLPjcd9RLUW86AfBLWKapZRkxnEMc2yHT6erYtcKBgCapzyrVH6VN8Utxj7e2mtvzcigwLm6ZafXyTMw
```

And they add 1.872447062 SOL to `2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3`:

```sh
$ spl-stake-pool increase-validator-stake EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1 2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3 1.872447062
Signature: 4zaKYu3MQ3as8reLbuHKaXN8FNaHvpHuiZtsJeARo67UKMo6wUUoWE88Fy8N4EYQYicuwULTNffcUD3a9jY88PoU
```

Internally, this instruction also uses transient stake accounts.  This time, the
stake pool splits from the reserve stake, into the transient stake account,
then activates it to the appropriate validator.

One to two epochs later, once the transient stakes activate, the `update` command
automatically merges the transient stakes into the validator stake account, leaving
a fully rebalanced stake pool:

```sh
$ spl-stake-pool list EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1
Pubkey: FhFft7ArhZZkh6q4ir1JZMYFgXdH6wkT5M5nmDDb1Q13    Vote: 8r1f8mwrUiYdg2Rx9sxTh4M3UAUcCBBrmRA3nxk3Z6Lm ◎5.283340235
Pubkey: FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN    Vote: 2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3 ◎5.283612231
Pubkey: E5KBATUd21Dnjnh5sGFw5ngp9kdVXCcAAYMRe2WsVXie    Vote: HJiC8iJ4Sj846SswQuauFJK93UvV6zp3c2T6jzGqzhhz ◎5.284317422
Total: ◎15.851269888
```

Due to staking rewards that accrued during the rebalancing process, the pool is
not perfectly balanced. This is completely normal.

### User Examples

#### List validator stake accounts

In order to deposit into the stake pool, a user must first delegate some stake
to one of the validator stake accounts associated with the stake pool. The
command-line utility has a special instruction for finding out which vote
accounts are already associated with the stake pool.

```sh
$ spl-stake-pool list EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1
CrStLEWfme37kDc3nubK9HsmWR5dsuVUuqEKqTR4Mc5E    1.002282880 SOL
E5KBATUd21Dnjnh5sGFw5ngp9kdVXCcAAYMRe2WsVXie    1.002282880 SOL
FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN    1.002282880 SOL
FhFft7ArhZZkh6q4ir1JZMYFgXdH6wkT5M5nmDDb1Q13    1.002282880 SOL
Total: 4.009131520 SOL
```

If the manager has recently created the stake pool, and there are no stake
accounts present yet, the command-line utility will inform us.

```sh
$ spl-stake-pool list EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1
No accounts found.
```

#### Deposit stake

Stake pools only accept deposits from active accounts, so we must first
create stake accounts and delegate them to one of the validators managed by the
stake pool. Using the `list` command from the previous section, we see that
`2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3` is a valid vote account, so let's
create a stake account and delegate our stake there.

```sh
$ solana-keygen new --no-passphrase -o stake-account.json
Generating a new keypair
Wrote new keypair to stake-account.json
============================================================================
pubkey: 4F4AYKZbNtDnu7uQey2Vkz9VgkVtLE6XWLezYjc9yxZa
============================================================================
Save this seed phrase to recover your new keypair:
++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
============================================================================
$ solana create-stake-account stake-account.json 10
Signature: 5Y9r6MNoqJzVX8TWryAJbdp8i2DvintfxbYWoY6VcLEPgphK2tdydhtJTd3o3dF7QdM2Pg8sBFDZuyNcMag3nPvj
$ solana delegate-stake 4F4AYKZbNtDnu7uQey2Vkz9VgkVtLE6XWLezYjc9yxZa 2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3
Signature: 2cDjHXSHjuadGQf1NQpPi43A8R19aCifsY16yTcictKPHcSAXN5TvXZ58nDJwkYs12tuZfTh5WVgAMSvptfrKdPP
```

Two epochs later, when the stake is fully active and has received one epoch of
rewards, we can deposit the stake into the stake pool.

```sh
$ spl-stake-pool deposit EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1 4F4AYKZbNtDnu7uQey2Vkz9VgkVtLE6XWLezYjc9yxZa
Depositing into stake account FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN
Creating account to receive tokens 34XMHa3JUPv46ftU4dGHvemZ9oKVjnciRePYMcX3rjEF
Signature: 4AESGZzqBVfj5xQnMiPWAwzJnAtQDRFK1Ha6jqKKTs46Zm5fw3LqgU1mRAT6CKTywVfFMHZCLm1hcQNScSMwVvjQ
```

The CLI will default to using the fee payer's
[Associated Token Account](associated-token-account.md) for stake pool tokens.
Alternatively, you can create an SPL token account yourself and pass it as the
`token-receiver` for the command.

```sh
$ spl-stake-pool deposit EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1 4F4AYKZbNtDnu7uQey2Vkz9VgkVtLE6XWLezYjc9yxZa --token-receiver 34XMHa3JUPv46ftU4dGHvemZ9oKVjnciRePYMcX3rjEF
Depositing into stake account FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN
Signature: 4AESGZzqBVfj5xQnMiPWAwzJnAtQDRFK1Ha6jqKKTs46Zm5fw3LqgU1mRAT6CKTywVfFMHZCLm1hcQNScSMwVvjQ
```

In return, the stake pool has sent us staking derivatives in the form of SPL
tokens.  We can double-check our stake pool account using the SPL token
command-line utility.

```sh
$ spl-token balance 34XMHa3JUPv46ftU4dGHvemZ9oKVjnciRePYMcX3rjEF
0.024058966
```

#### Update

Every epoch, the network pays out rewards to stake accounts managed by the stake
pool, increasing the value of staking derivative SPL tokens minted on deposit.
In order to calculate the proper value of these stake pool tokens, we must update
the total value managed by the stake pool every epoch.

```sh
$ spl-stake-pool update EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1
Updating stake pool...
Signature: 3Yx1RH3Afqj5ckX8YvPCRt1DudVP4HuRPkh1dBPvTM9GqGxcB9ZXHGZPADVSZiaqKi166fevMG232EWxrRWswPtt
```

If another user already updated the stake pool balance for the current epoch, we
see a different output.

```sh
$ spl-stake-pool update EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1
Update not required
```

If no one updates the stake pool in the current epoch, the deposit and withdraw
instructions will fail. The update instruction is permissionless, so any user
can run it before depositing or withdrawing. As a convenience, the CLI attempts
to update before running any instruction on the stake pool.

If the stake pool transient stakes are in an unexpected state, and merges are
not possible, there is the option to only update the stake pool balances without
performing merges using the `--no-merge` flag.

```sh
$ spl-stake-pool update EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1 --no-merge
Updating stake pool...
Signature: 3Yx1RH3Afqj5ckX8YvPCRt1DudVP4HuRPkh1dBPvTM9GqGxcB9ZXHGZPADVSZiaqKi166fevMG232EWxrRWswPtt
```

Later on, whenever the transient stakes are ready to be merged, it is possible to
force another update in the same epoch using the `--force` flag.

```sh
$ spl-stake-pool update EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1 --force
Updating stake pool...
Signature: 3Yx1RH3Afqj5ckX8YvPCRt1DudVP4HuRPkh1dBPvTM9GqGxcB9ZXHGZPADVSZiaqKi166fevMG232EWxrRWswPtt
```

#### Withdraw stake

Whenever the user wants to recover SOL plus accrued rewards, they can provide their
staking derivative SPL tokens in exchange for an activated stake account.

Let's withdraw 0.02 staking derivative tokens from the stake pool.

```sh
$ spl-stake-pool withdraw EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1 0.02
Withdrawing from account FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN, amount 8.867176377 SOL, 0.02 pool tokens
Creating account to receive stake CZF2z3JJoDmJRcVjtsrz1BKUUGNL3VPW5FPFqge1bzmQ
Signature: 2xBPVPJ749AE4hHNCNYdjuHv1EdMvxm9uvvraWfTA7Urrvecwh9w64URCyLLroLQ2RKDGE2QELM2ZHd8qRkjavJM
```

The stake pool took 0.02 pool tokens, and in exchange the user received a fully
active stake account, delegated to `2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3`.
Let's double-check the status of the stake account:

```sh
$ solana stake-account CZF2z3JJoDmJRcVjtsrz1BKUUGNL3VPW5FPFqge1bzmQ
Balance: 8.869459257 SOL
Rent Exempt Reserve: 0.00228288 SOL
Delegated Stake: 8.867176377 SOL
Active Stake: 8.867176377 SOL
Delegated Vote Account Address: 2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3
Stake Authority: 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Withdraw Authority: 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
```

Alternatively, the user can specify an existing uninitialized stake account to
receive their stake using the `--stake-receiver` parameter.

```sh
$ spl-stake-pool withdraw EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1  --amount 0.02 --withdraw-from 34XMHa3JUPv46ftU4dGHvemZ9oKVjnciRePYMcX3rjEF --stake-receiver CZF2z3JJoDmJRcVjtsrz1BKUUGNL3VPW5FPFqge1bzmQ
Withdrawing from account FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN, amount 8.867176377 SOL, 0.02 pool tokens
Signature: 2xBPVPJ749AE4hHNCNYdjuHv1EdMvxm9uvvraWfTA7Urrvecwh9w64URCyLLroLQ2RKDGE2QELM2ZHd8qRkjavJM
```

By default, the withdraw command uses the fee payer's associated token account to
source the derivative tokens. It's possible to specify the SPL token account using
the `--pool-account` flag.

```sh
$ spl-stake-pool withdraw EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1 0.02 --pool-account 34XMHa3JUPv46ftU4dGHvemZ9oKVjnciRePYMcX3rjEF
Withdrawing from account FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN, amount 8.867176377 SOL, 0.02 pool tokens
Creating account to receive stake CZF2z3JJoDmJRcVjtsrz1BKUUGNL3VPW5FPFqge1bzmQ
Signature: 2xBPVPJ749AE4hHNCNYdjuHv1EdMvxm9uvvraWfTA7Urrvecwh9w64URCyLLroLQ2RKDGE2QELM2ZHd8qRkjavJM
```

By default, the withdraw command will withdraw from the largest validator stake
accounts in the pool. It's also possible to specify a specific vote account for
the withdraw using the `--vote-account` flag.

```sh
$ spl-stake-pool withdraw EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1 0.02 --vote-account 2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3
Withdrawing from account FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN, amount 8.867176377 SOL, 0.02 pool tokens
Creating account to receive stake CZF2z3JJoDmJRcVjtsrz1BKUUGNL3VPW5FPFqge1bzmQ
Signature: 2xBPVPJ749AE4hHNCNYdjuHv1EdMvxm9uvvraWfTA7Urrvecwh9w64URCyLLroLQ2RKDGE2QELM2ZHd8qRkjavJM
```

Note that the associated validator stake account must have enough lamports to
satisfy the pool token amount requested.

##### Special case: exiting pool with a delinquent staker

With the reserve stake, it's possible for a delinquent or malicious staker to
move all stake into the reserve through `decrease-validator-stake`, so the
staking derivatives will not gain rewards, and the stake pool users will not
be able to withdraw their funds.

To get around this case, it is also possible to withdraw from the stake pool's
reserve, but only if all of the validator stake accounts are at the minimum amount of
`1 SOL + stake account rent exemption`.

```sh
$ spl-stake-pool withdraw EjspffVUi2Tivszzs2JVj4GiSiMNYKyqZpgP3NeefBU1 0.02 --use-reserve
Withdrawing from account 33Hg3bvYrAwfqCzTMjAWZNAWC6H96qJNEdzGamfFjG4J, amount 8.867176377 SOL, 0.02 pool tokens
Creating account to receive stake 9E5YzXXu9NDhtMxWJKCwe2M8Sdz6vL6bcBS92U76PVtE
Signature: 4aZaeT9Azcq23PdKcjbQLseNveZVAQ4xMabBGQspfX316cE62Q2hoES373ExbT9y2JUhug7SgdybNaCjuZ6uqNYf
```

## Appendix

### Activated stakes

As mentioned earlier, the stake pool only processes active stakes. This feature
maintains fungibility of stake pool tokens. Fully activated stakes
are not equivalent to inactive, activating, or deactivating stakes due to the
time cost of staking. Otherwise, malicious actors can deposit stake in one state
and withdraw it in another state without waiting.

### Transient stake accounts

Each validator gets one transient stake account, so the staker can only
perform one action at a time on a validator. It's impossible to increase
and decrease the stake on a validator at the same time. The staker must wait for
the existing transient stake account to get merged during an `update` instruction
before performing a new action.

### Reserve stake account

Every stake pool is initialized with an undelegated reserve stake account, used
to hold undelegated stake in process of rebalancing. After the staker decreases
the stake on a validator, one epoch later, the update operation will merge the
decreased stake into the reserve. Conversely, whenever the staker increases the
stake on a validator, the lamports are drawn from the reserve stake account.

### Staking Credits Observed on Deposit

A deposited stake account's "credits observed" must match the destination
account's "credits observed". Typically, this means you must wait an additional
epoch after activation for your stake account to match up with the stake pool's account.

### Transaction sizes

The Solana transaction processor has two important limitations:

* size of the overall transaction, limited to roughly 1 MTU / packet
* computation budget per instruction

A stake pool may manage hundreds of staking accounts, so it is impossible to
update the total value of the stake pool in one instruction. Thankfully, the
command-line utility breaks up transactions to avoid this issue for large pools.
