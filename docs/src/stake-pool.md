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
to provide a market for their pool's tokens, through an AMM
like [Token Swap](token-swap.md).

## Operation

A stake pool manager creates a stake pool, and the staker includes validators that will
receive delegations from the pool by creating "validator stake accounts" and
activating a delegation on them. Once a validator stake account's delegation is
active, the staker adds it to the stake pool.

At this point, users can participate with deposits. They must delegate a stake
account to the one of the validators in the stake pool. Once it's active, the
user can deposit their stake into the pool in exchange for SPL tokens
representing their fractional ownership in pool. A percentage of the rewards
earned by the pool goes to the pool manager as a fee.

Over time, as the stakes in the stake pool accrue staking rewards, the user's fractional
ownership will be worth more than their initial deposit. Whenever the user chooses,
they can withdraw activated stake in exchange for their SPL pool tokens.

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
```console
$ cargo install spl-stake-pool-cli
```

Run `spl-stake-pool --help` for a full description of available commands.

### Configuration

The `spl-stake-pool` configuration is shared with the `solana` command-line tool.

#### Current Configuration

```console
solana config get
Config File: ${HOME}/.config/solana/cli/config.yml
RPC URL: https://api.mainnet-beta.solana.com
WebSocket URL: wss://api.mainnet-beta.solana.com/ (computed)
Keypair Path: ${HOME}/.config/solana/id.json
```

#### Cluster RPC URL

See [Solana clusters](https://docs.solana.com/clusters) for cluster-specific RPC URLs
```console
solana config set --url https://api.devnet.solana.com
```

#### Default Keypair

See [Keypair conventions](https://docs.solana.com/cli/conventions#keypair-conventions)
for information on how to setup a keypair if you don't already have one.

Keypair File
```console
solana config set --keypair ${HOME}/new-keypair.json
```

Hardware Wallet URL (See [URL spec](https://docs.solana.com/wallet-guide/hardware-wallets#specify-a-keypair-url))
```console
solana config set --keypair usb://ledger/
```

#### Run Locally

If you would like to test a stake pool locally without having to wait for stakes
to activate and deactivate, you can run the stake pool locally using the
`solana-test-validator` tool with shorter epochs, and pulling the current program
from devnet, testnet, or mainnet.

```console
$ solana-test-validator -c SPoo1xuN9wGpxNjGnPNbRPtpQ7mHgKM8d9BeFC549Jy -c GAm4m8ToXFMW6kT3DwYbT4QPDj2RMi744SjXUHm5szH3 --url devnet --slots-per-epoch 32
$ solana config set --url http://127.0.0.1:8899
```

### Stake Pool Manager Examples

#### Create a stake pool

The stake pool manager controls the stake pool from a high level, and in exchange
receives a fee in the form of SPL tokens. The manager
sets the fee on creation. Let's create a pool with a 3% fee and a maximum of 1000
validator stake accounts:

```console
$ spl-stake-pool create-pool --fee-numerator 3 --fee-denominator 100 --max-validators 1000
Creating reserve stake J5XB7mWpeaUZxZ6ogXT57qSCobczx27vLZYSgfSbZoBB
Creating mint BoNneHKDrX9BHjjvSpPfnQyRjsnc9WFH71v8wrgCd7LB
Creating associated token account DgyZrAq88bnG1TNRxpgDQzWXpzEurCvfY2ukKFWBvADQ to receive stake pool tokens of mint BoNneHKDrX9BHjjvSpPfnQyRjsnc9WFH71v8wrgCd7LB, owned by 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Creating pool fee collection account DgyZrAq88bnG1TNRxpgDQzWXpzEurCvfY2ukKFWBvADQ
Signature: gHdi9x16gmCbj9uiMGtVzpKqdqUT3ySNt78fkUvpM6mkDCzU6rh2VWu91P3Et7G4fN9yxKeZbFYJ6CX4UVGJjDF
Creating stake pool Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR
Signature: 2jg2qkX2Dk4BgtSNjSFxFSWTacuspCLkEq3xeuAsKrq7QGuoeKWt9bEaqCmTAG2J7kBa1sd4yZeDLWzZT2Ps3Q2f
```

The unique stake pool identifier is `Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR`.

The identifier for the stake pool's SPL token mint is
`BoNneHKDrX9BHjjvSpPfnQyRjsnc9WFH71v8wrgCd7LB`. The stake pool has full control
over the mint.

The pool creator's fee account identifier is
`DgyZrAq88bnG1TNRxpgDQzWXpzEurCvfY2ukKFWBvADQ`. Every epoch, as stake accounts
in the stake pool earn rewards, the program will mint SPL pool tokens
equal to 3% of the gains on that epoch into this account. If no gains were observed,
nothing will be deposited.

The reserve stake account identifier is `J5XB7mWpeaUZxZ6ogXT57qSCobczx27vLZYSgfSbZoBB`.
This account holds onto additional stake used when rebalancing between validators.

For a stake pool with 1000 validators, the cost to create a stake pool is less
than 0.5 SOL.

#### Set manager

The stake pool manager may pass their administrator privileges to another account.

```console
$ spl-stake-pool set-manager Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR --new-manager 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Signature: 39N5gkaqXuWm6JPEUWfenKXeG4nSa71p7iHb9zurvdZcsWmbjdmSXwLVYfhAVHWucTY77sJ8SkUNpVpVAhe4eZ53
```

At the same time, they may also change the SPL token account that receives fees
every epoch. The mint for the provided token account must be the SPL token mint,
`BoNneHKDrX9BHjjvSpPfnQyRjsnc9WFH71v8wrgCd7LB` in our example.

```console
$ spl-stake-pool set-manager Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR --new-fee-receiver HoCsh97wRxRXVjtG7dyfsXSwH9VxdDzC7GvAsBE1eqJz
Signature: 4aK8yzYvPBkP4PyuXTcCm529kjEH6tTt4ixc5D5ZyCrHwc4pvxAHj6wcr4cpAE1e3LddE87J1GLD466aiifcXoAY
```

#### Set fee

The stake pool manager may update the fee assessed every epoch, passing the
numerator and denominator for the fraction that make up the fee. For a fee of
10%, they could run:

```console
$ spl-stake-pool set-fee Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR 10 100
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

```console
$ spl-stake-pool set-staker Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
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

```console
$ spl-stake-pool create-validator-stake Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR 2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3
Creating stake account FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN, delegated to 2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3
Signature: 4pA2WKT6d2wkXEtSpiQswv22WyoFad2KX6FdPEzwBiEquvaUBEtzenys5Jh1ABPCh7yc4w8kzqMRRCwDj6ZSUV1K
```

In order to maximize censorship resistance, we want to distribute our SOL to as
many validators as possible, so let's add a few more.

```console
$ spl-stake-pool create-validator-stake Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR HJiC8iJ4Sj846SswQuauFJK93UvV6zp3c2T6jzGqzhhz
Creating stake account E5KBATUd21Dnjnh5sGFw5ngp9kdVXCcAAYMRe2WsVXie, delegated to HJiC8iJ4Sj846SswQuauFJK93UvV6zp3c2T6jzGqzhhz
Signature: 4pyRZzjsWG7jP3GRZeZCo2Eb2TPjHM4kAYRFMivimme6HAee1nhzoNJBe3VSt2sv7acp5fwT7J8omBM8o3niY8gu
$ spl-stake-pool create-validator-stake Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR AUCzCaGAGjL3uyjFBtJs7KuJcgQWvNZu1Z2S9G3pw77G
Creating stake account CrStLEWfme37kDc3nubK9HsmWR5dsuVUuqEKqTR4Mc5E, delegated to AUCzCaGAGjL3uyjFBtJs7KuJcgQWvNZu1Z2S9G3pw77G
Signature: 4ZUdZzUARgUCPuY8nVsJbN6vRDbVX8sYAQGYYXj2YVvjoJ2oevq2H8uzrhYApe419uoP7QYukqNstiti5p5DDukN
$ spl-stake-pool create-validator-stake Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR 8r1f8mwrUiYdg2Rx9sxTh4M3UAUcCBBrmRA3nxk3Z6Lm
Creating stake account FhFft7ArhZZkh6q4ir1JZMYFgXdH6wkT5M5nmDDb1Q13, delegated to 8r1f8mwrUiYdg2Rx9sxTh4M3UAUcCBBrmRA3nxk3Z6Lm
Signature: yQqXCbuA66wQsHtkziNg3XadfZF5aCmvjfentwbZJnSPeEjJwPka3M1QY5GmR1efprptqaePn71BTMSLscX8DLr
```

NOTE: These stake accounts have not been added to the stake pool yet. Stake pools
only accept deposits from fully active (warmed-up) delegated stake accounts.

We can see the status of stake account using the Solana command-line utility.

```console
$ solana stake-account FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN
Balance: 1.00228288 SOL
Rent Exempt Reserve: 0.00228288 SOL
Delegated Stake: 1 SOL
Active Stake: 0 SOL
Activating Stake: 1 SOL
Stake activates starting from epoch: 211
Stake Authority: 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Withdraw Authority: 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
```

The stake pool creates these special staking accounts with 1 SOL as the required
delegation amount. The stake and withdraw authorities are the keypair configured
with the `--config` flag, using the Solana CLI default key. More information
about the Solana CLI can be found on the
[Solana Docs](https://docs.solana.com/running-validator/validator-start#configure-solana-cli).

Now that we have created these delegated validator stake accounts, we need to
wait an epoch for the delegation to activate.

#### Add validator stake account

As mentioned in the last step, the stake pool only manages one stake account per
validator. Also, the stake pool only processes fully activated stake accounts.
We created new validator stake accounts in the last step and staked them. Once
the stake activates, we can add them to the stake pool.

Also, as mentioned in the last step, validator stake accounts must have exactly
1.00228288 SOL, 1 SOL for the delegation, and 0.00228288 SOL for the rent-exempt
reserve. After activation, the validator stake account may have already gained
some rewards, so we have to move those rewards off before adding the validator.
Let's check our stake account again:

```console
$ solana stake-account FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN
Balance: 1.00628288 SOL
Rent Exempt Reserve: 0.00228288 SOL
Delegated Stake: 1.004 SOL
Active Stake: 1.004 SOL
Stake activates starting from epoch: 211
Stake Authority: 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Withdraw Authority: 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
```

Since the delegated stake is now 1.004 SOL, we need to split that additional
amount into another stake account before adding.

```console
$ solana-keygen new -o split-stake.json
...
$ solana split-stake FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN split-stake.json 0.004
Signature: 5Pg385MpkWgXhrZZpwX34BZEexvsPSRw2kFZt1Sp3KS1jEFpAc8Vb1zDRqVLbDqJRx3b3gU6zpRc7mdHKvHbHXJH
```

The staker is free to do whatever they like with the new split stake account.
Most likely, they will want to deactivate it, wait an epoch, and then withdraw
the additional lamports back into their account.

```console
$ solana deactivate-stake split-stake.json
...
$ solana withdraw-stake split-stake.json 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn ALL
```

Now that the validator stake account has exactly 1 delegated SOL, we're ready to
add this validator to the stake pool!

```console
$ spl-stake-pool add-validator Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR 2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3
Adding stake account FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN, delegated to 2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3
Signature: 3N1K89rGV9gWueTTrPGTDBwKAp8BikQhKHMFoREw98Q1piXFeZSSxqfnRQexrfAZQfrpYH9qwsaPWRruwkVeBivV
```

Users can start depositing their activated stakes into the stake pool, as
long as they are delegated to the same vote account, which was
`FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN` in this example.  You can also
double-check that at any time using the Solana command-line utility.

```console
$ solana stake-account FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN
Balance: 1.00228288 SOL
Rent Exempt Reserve: 0.00228288 SOL
Delegated Stake: 1 SOL
Active Stake: 1 SOL
Stake activates starting from epoch: 211
Delegated Vote Account Address: 2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3
Stake Authority: 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Withdraw Authority: 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
```

#### Remove validator stake account

If the stake pool staker wants to stop delegating to a vote account, they can
totally remove the validator stake account from the stake pool.

As with adding a validator, the validator stake account must have exactly
1.00228288 SOL (1 SOL delegated, 0.00228288 SOL for rent exemption) to be removed.

If that is not the case, the staker must first decrease the stake to that minimum amount.
Let's assume that the validator stake account delegated to 
`AUCzCaGAGjL3uyjFBtJs7KuJcgQWvNZu1Z2S9G3pw77G` has a total delegated amount of
7.5 SOL. To reduce that number, the staker can run:

```console
$ spl-stake-pool decrease-validator-stake Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR AUCzCaGAGjL3uyjFBtJs7KuJcgQWvNZu1Z2S9G3pw77G 6.5
Signature: ZpQGwT85rJ8Y9afdkXhKo3TVv4xgTz741mmZj2vW7mihYseAkFsazWxza2y8eNGY4HDJm15c1cStwyiQzaM3RpH
```

Now, let's try to remove validator `AUCzCaGAGjL3uyjFBtJs7KuJcgQWvNZu1Z2S9G3pw77G`, with
stake account `CrStLEWfme37kDc3nubK9HsmWR5dsuVUuqEKqTR4Mc5E`.

```console
$ spl-stake-pool remove-validator Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR AUCzCaGAGjL3uyjFBtJs7KuJcgQWvNZu1Z2S9G3pw77G
Signature: 5rrQ3xhDWyiPkUTAQkNAeq31n6sMf1xsg2x9hVY8Vj1NonwBnhxuTv87nADLkwC8Xzc4CGTNCTX2Vph9esWnXk2d
```

Unlike a normal withdrawal, the validator stake account is totally
removed from the stake pool and now belongs to the administrator. The authority
for the withdrawn stake account can also be specified using the `--new-authority` flag:

```console
$ spl-stake-pool remove-validator Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR AUCzCaGAGjL3uyjFBtJs7KuJcgQWvNZu1Z2S9G3pw77G --new-authority 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Signature: 5rrQ3xhDWyiPkUTAQkNAeq31n6sMf1xsg2x9hVY8Vj1NonwBnhxuTv87nADLkwC8Xzc4CGTNCTX2Vph9esWnXk2d
```

We can check the removed stake account:

```console
$ solana stake-account CrStLEWfme37kDc3nubK9HsmWR5dsuVUuqEKqTR4Mc5E
Balance: 1.002282880 SOL
Rent Exempt Reserve: 0.00228288 SOL
Delegated Stake: 1.000000000 SOL
Active Stake: 1.000000000 SOL
Delegated Vote Account Address: AUCzCaGAGjL3uyjFBtJs7KuJcgQWvNZu1Z2S9G3pw77G
Stake Authority: 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Withdraw Authority: 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
```

#### Rebalance the stake pool

As time goes on, users will deposit to and withdraw from all of the stake accounts
managed by the pool, and the stake pool staker may want to rebalance the stakes.

For example, let's say the staker wants the same delegation to every validator
in the pool. When they look at the state of the pool, they see:

```console
$ spl-stake-pool list Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR
Stake Pool: Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR
Pool Token Mint: BoNneHKDrX9BHjjvSpPfnQyRjsnc9WFH71v8wrgCd7LB
Fee: 3/100 of epoch rewards
Reserve Account: J5XB7mWpeaUZxZ6ogXT57qSCobczx27vLZYSgfSbZoBB   Available Balance: ◎0.000000000
Vote Account: 8r1f8mwrUiYdg2Rx9sxTh4M3UAUcCBBrmRA3nxk3Z6Lm Balance: ◎1.002282881 Last Update Epoch: 212
Vote Account: 2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3 Balance: ◎3.410872673 Last Update Epoch: 212
Vote Account: HJiC8iJ4Sj846SswQuauFJK93UvV6zp3c2T6jzGqzhhz Balance: ◎11.436803652 Last Update Epoch: 212
Total Pool Stake: ◎15.849959206
Total Pool Tokens: 15.849959206
Current Number of Validators: 3
Max Number of Validators: 1000
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
$ spl-stake-pool decrease-validator-stake Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR HJiC8iJ4Sj846SswQuauFJK93UvV6zp3c2T6jzGqzhhz 6.153483916
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
$ spl-stake-pool increase-validator-stake Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR 8r1f8mwrUiYdg2Rx9sxTh4M3UAUcCBBrmRA3nxk3Z6Lm 4.281036854
Signature: 3GJACzjUGLPjcd9RLUW86AfBLWKapZRkxnEMc2yHT6erYtcKBgCapzyrVH6VN8Utxj7e2mtvzcigwLm6ZafXyTMw
```

And they add 1.872447062 SOL to `2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3`:

```sh
$ spl-stake-pool increase-validator-stake Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR 2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3 1.872447062
Signature: 4zaKYu3MQ3as8reLbuHKaXN8FNaHvpHuiZtsJeARo67UKMo6wUUoWE88Fy8N4EYQYicuwULTNffcUD3a9jY88PoU
```

Internally, this instruction also uses transient stake accounts.  This time, the
stake pool splits from the reserve stake, into the transient stake account,
then activates it to the appropriate validator.

One to two epochs later, once the transient stakes activate, the `update` command
automatically merges the transient stakes into the validator stake account, leaving
a fully rebalanced stake pool:

```console
$ spl-stake-pool list Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR
Stake Pool: Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR
Pool Token Mint: BoNneHKDrX9BHjjvSpPfnQyRjsnc9WFH71v8wrgCd7LB
Fee: 3/100 of epoch rewards
Reserve Account: J5XB7mWpeaUZxZ6ogXT57qSCobczx27vLZYSgfSbZoBB   Available Balance: ◎0.000000000
Vote Account: 8r1f8mwrUiYdg2Rx9sxTh4M3UAUcCBBrmRA3nxk3Z6Lm Balance: ◎5.283340235 Last Update Epoch: 212
Vote Account: 2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3 Balance: ◎5.283612231 Last Update Epoch: 212
Vote Account: HJiC8iJ4Sj846SswQuauFJK93UvV6zp3c2T6jzGqzhhz Balance: ◎5.284317422 Last Update Epoch: 212
Total Pool Stake: ◎15.851269888
Total Pool Tokens: 15.849959206
Current Number of Validators: 3
Max Number of Validators: 1000
```

Due to staking rewards that accrued during the rebalancing process, the pool is
not perfectly balanced. This is completely normal.

#### Set Preferred Deposit / Withdraw Validator

Since a stake pool accepts deposits to any of its stake accounts, and allows
withdrawals from any of its stake accounts, it could be used by malicious arbitrageurs
looking to maximize returns each epoch.

For example, if a stake pool has 1000 validators, an arbitrageur could stake to
any one of those validators. At the end of the epoch, they can check which
validator has the best performance, deposit their stake, and immediately withdraw
from the highest performing validator. Once rewards are paid out, they can take
their valuable stake, and deposit it back for more than they had.

To mitigate this arbitrage, a stake pool staker can set a preferred withdraw
or deposit validator. Any deposits or withdrawals must go to the corresponding
stake account, making this attack impossible without a lot of funds.

Let's set a preferred deposit validator stake account:

```console
$ spl-stake-pool set-preferred-validator Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR deposit --vote-account EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ
Signature: j6fbTqGJ8ehgKnSPns1adaSeFwg5M3wP1a32qYwZsQjymYoSejFUXLNGwvHSouJcFm4C78HUoC8xd7cvb5iActL
```

And then let's set the preferred withdraw validator stake account to the same one:

```console
$ spl-stake-pool set-preferred-validator Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR withdraw --vote-account EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ
Signature: 4MKdYLyFqU6H3311YZDeLtsoeGZMzswBHyBCRjHfkzuN1rB4LXJbPfkgUGLKkdbsxJvPRub7SqB1zNPTqDdwti2w
```

At any time, they may also unset the preferred validator:

```console
$ spl-stake-pool set-preferred-validator Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR withdraw --unset
Signature: 5Qh9FA3EXtJ7nKw7UyxmMWXnTMLRKQqcpvfEsEyBtxSPqzPAXp2vFXnPg1Pw8f37JFdvyzYay65CtA8Z1ewzVkvF
```

The preferred validators are marked in the `list` command:

```console
$ spl-stake-pool list Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR
Stake Pool: Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR
Pool Token Mint: BoNneHKDrX9BHjjvSpPfnQyRjsnc9WFH71v8wrgCd7LB
Preferred Deposit Validator: EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ
Preferred Withdraw Validator: EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ
...
```

### User Examples

#### List validator stake accounts

In order to deposit into the stake pool, a user must first delegate some stake
to one of the validator stake accounts associated with the stake pool. The
command-line utility has a special instruction for finding out which vote
accounts are already associated with the stake pool.

```sh
$ spl-stake-pool list Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR
Stake Pool: Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR
Pool Token Mint: BoNneHKDrX9BHjjvSpPfnQyRjsnc9WFH71v8wrgCd7LB
Fee: 3/100 of epoch rewards
Reserve Account: J5XB7mWpeaUZxZ6ogXT57qSCobczx27vLZYSgfSbZoBB   Available Balance: ◎0.000000000
Vote Account: 8r1f8mwrUiYdg2Rx9sxTh4M3UAUcCBBrmRA3nxk3Z6Lm Balance: ◎5.283340235 Last Update Epoch: 212
Vote Account: 2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3 Balance: ◎5.283612231 Last Update Epoch: 212
Vote Account: HJiC8iJ4Sj846SswQuauFJK93UvV6zp3c2T6jzGqzhhz Balance: ◎5.284317422 Last Update Epoch: 212
Total Pool Stake: ◎15.851269888
Total Pool Tokens: 15.849959206
Current Number of Validators: 3
Max Number of Validators: 1000
```

#### Deposit stake

Stake pools only accept deposits from active stake accounts, so we must first
create stake accounts and delegate them to one of the validators managed by the
stake pool. Using the `list` command from the previous section, we see that
`2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3` is a valid vote account, so let's
create a stake account and delegate our stake there.

```console
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

```console
$ spl-stake-pool deposit Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR 4F4AYKZbNtDnu7uQey2Vkz9VgkVtLE6XWLezYjc9yxZa
Update not required
Depositing stake 6oXrBEjuR9PDbQgeqkDGYZBR3TQ2UQz2ZDKtbeyqFL4Y into stake pool account FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN
Creating associated token account DgyZrAq88bnG1TNRxpgDQzWXpzEurCvfY2ukKFWBvADQ to receive stake pool tokens of mint BoNneHKDrX9BHjjvSpPfnQyRjsnc9WFH71v8wrgCd7LB, owned by 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Signature: 5QYuChKY4scRerDYwiSiSwFq1gotRz9k7bXNrGvc8sxU2vuN6ghKr6uiwMzPpb3yMb8KcQE5wRv6Zxsn
VhgMFiGK
```

The CLI will default to using the fee payer's
[Associated Token Account](associated-token-account.md) for stake pool tokens.
Alternatively, you can create an SPL token account yourself and pass it as the
`token-receiver` for the command.

```sh
$ spl-stake-pool deposit Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR 4F4AYKZbNtDnu7uQey2Vkz9VgkVtLE6XWLezYjc9yxZa --token-receiver 34XMHa3JUPv46ftU4dGHvemZ9oKVjnciRePYMcX3rjEF
Depositing into stake account FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN
Signature: 4AESGZzqBVfj5xQnMiPWAwzJnAtQDRFK1Ha6jqKKTs46Zm5fw3LqgU1mRAT6CKTywVfFMHZCLm1hcQNScSMwVvjQ
```

In return, the stake pool has minted us new pool tokens, representing our share
of ownership in the pool.  We can double-check our stake pool account using the
SPL token command-line utility.

```sh
$ spl-token balance 34XMHa3JUPv46ftU4dGHvemZ9oKVjnciRePYMcX3rjEF
0.024058966
```

#### Update

Every epoch, the network pays out rewards to stake accounts managed by the stake
pool, increasing the value of pool tokens minted on deposit.
In order to calculate the proper value of these stake pool tokens, we must update
the total value managed by the stake pool every epoch.

```sh
$ spl-stake-pool update Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR
Signature: 2rtPNGKFSSnXFCb6MKG5wHp34dkB5hJWNhro8EU2oGh1USafAgzu98EgoRnPLi7ojQfmTpvXk4S7DWXYGu5t85Ka
Signature: 5V2oCNvZCNJfC6QXHmR2UHGxVMip6nfZixYkVjFQBTyTf2Z9s9GJ9BjkxSFGvUsvW6zc2cCRv9Lqucu1cgHMFcVU
```

If another user already updated the stake pool balance for the current epoch, we
see a different output.

```sh
$ spl-stake-pool update Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR
Update not required
```

If no one updates the stake pool in the current epoch, all instructions, including
deposit and withdraw, will fail. The update instruction is permissionless, so any user
can run it before interacting with the pool. As a convenience, the CLI attempts
to update before running any instruction on the stake pool.

If the stake pool transient stakes are in an unexpected state, and merges are
not possible, there is the option to only update the stake pool balances without
performing merges using the `--no-merge` flag.

```sh
$ spl-stake-pool update Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR --no-merge
Signature: 5cjdZG727uzwnEEG3vJ1vskA9WsXibaEHh7imXSb2S1cwEYK4Q3btr2GEeAV8EffK4CEQ2WM6PQxawkJAHoZ4jsQ
Signature: EBHbSRstJ3HxKwYKak8vEwVMKr1UBxdbqs5KuX3XYt4ppPjhaziGEtvL2TJCm1HLokbrtMeTEv57Ef4xhByJtJP
```

Later on, whenever the transient stakes are ready to be merged, it is possible to
force another update in the same epoch using the `--force` flag.

```sh
$ spl-stake-pool update Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR --force
Signature: 5RneEBwJkFytBJaJdkvCTHFrG3QzE3SGf9vdBm9gteCcHV4HwaHzj3mjX1hZg4yCREQSgmo3H9bPF6auMmMFTSTo
Signature: 1215wJUY7vj82TQoGCacQ2VJZ157HnCTvfsUXkYph3nZzJNmeDaGmy1nCD7hkhFfxnQYYxVtec5TkDFGGB4e7EvG
```

#### Withdraw stake

Whenever the user wants to recover their SOL plus accrued rewards, they can provide their
pool tokens in exchange for an activated stake account.

Let's withdraw active staked SOL in exchange for 5 pool tokens.

```console
$ spl-stake-pool withdraw Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR 5
Withdrawing ◎5.000000000, or 5 pool tokens, from stake account Ef6yEz9qzQnuyhnRzmwwifix7wAezycSUWy6cE7JWEzR, delegated to EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ
Creating account to receive stake F8iaFJgNy9HS4UBodH8v7BLQA5xDDk25jp5Sf9v5TqhF
Signature: 4j4pHRg9VWSvayjes6zBvX8ok6nVjCqswrwtwBoKhohmpVx8CaFXgtP8JcC5dT1heKiDoTSeeEenqdp3PDUgRbYg
```

The stake pool took 5 pool tokens, and in exchange the user received a fully
active stake account, delegated to `EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ`.
Let's double-check the status of the stake account:

```console
$ solana stake-account F8iaFJgNy9HS4UBodH8v7BLQA5xDDk25jp5Sf9v5TqhF
Balance: 5.00228288 SOL
Rent Exempt Reserve: 0.00228288 SOL
Delegated Stake: 5 SOL
Active Stake: 5 SOL
Delegated Vote Account Address: EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ
Stake Authority: 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Withdraw Authority: 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
```

Alternatively, the user can specify an existing uninitialized stake account to
receive their stake using the `--stake-receiver` parameter.

```console
$ spl-stake-pool withdraw Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR  --amount 0.02 --vote-account EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ --stake-receiver CZF2z3JJoDmJRcVjtsrz1BKUUGNL3VPW5FPFqge1bzmQ
Withdrawing ◎5.000000000, or 5 pool tokens, from stake account Ef6yEz9qzQnuyhnRzmwwifix7wAezycSUWy6cE7JWEzR, delegated to EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ
Signature: 2xBPVPJ749AE4hHNCNYdjuHv1EdMvxm9uvvraWfTA7Urrvecwh9w64URCyLLroLQ2RKDGE2QELM2ZHd8qRkjavJM
```

By default, the withdraw command uses the `token-owner`'s associated token account to
source the pool tokens. It's possible to specify the SPL token account using
the `--pool-account` flag.

```console
$ spl-stake-pool withdraw Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR 5 --pool-account 34XMHa3JUPv46ftU4dGHvemZ9oKVjnciRePYMcX3rjEF
Withdrawing ◎5.000000000, or 5 pool tokens, from stake account Ef6yEz9qzQnuyhnRzmwwifix7wAezycSUWy6cE7JWEzR, delegated to EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ
Creating account to receive stake CZF2z3JJoDmJRcVjtsrz1BKUUGNL3VPW5FPFqge1bzmQ
Signature: 2xBPVPJ749AE4hHNCNYdjuHv1EdMvxm9uvvraWfTA7Urrvecwh9w64URCyLLroLQ2RKDGE2QELM2ZHd8qRkjavJM
```

By default, the withdraw command will withdraw from the largest validator stake
accounts in the pool. It's also possible to specify a specific vote account for
the withdraw using the `--vote-account` flag.

```console
$ spl-stake-pool withdraw Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR  --amount 5 --vote-account EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ
Withdrawing ◎5.000000000, or 5 pool tokens, from stake account Ef6yEz9qzQnuyhnRzmwwifix7wAezycSUWy6cE7JWEzR, delegated to EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ
Creating account to receive stake CZF2z3JJoDmJRcVjtsrz1BKUUGNL3VPW5FPFqge1bzmQ
Signature: 2xBPVPJ749AE4hHNCNYdjuHv1EdMvxm9uvvraWfTA7Urrvecwh9w64URCyLLroLQ2RKDGE2QELM2ZHd8qRkjavJM
```

Note that the associated validator stake account must have enough lamports to
satisfy the pool token amount requested.

##### Special case: exiting pool with a delinquent staker

With the reserve stake, it's possible for a delinquent or malicious staker to
move all stake into the reserve through `decrease-validator-stake`, so the
pool tokens will not gain rewards, and the stake pool users will not
be able to withdraw their funds.

To get around this case, it is also possible to withdraw from the stake pool's
reserve, but only if all of the validator stake accounts are at the minimum amount of
`1 SOL + stake account rent exemption`.

```console
$ spl-stake-pool withdraw Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR 5 --use-reserve
Withdrawing ◎5.000000000, or 5 pool tokens, from stake account J5XB7mWpeaUZxZ6ogXT57qSCobczx27vLZYSgfSbZoBB
Creating account to receive stake 51XdXiBSsVzeuY79xJwWAGZgeKzzgFKWajkwvWyrRiNE
Signature: yQH9n7Go6iCMEYXqWef38ZYBPwXDmbwKAJFJ4EHD6TusBpusKsfNuT3TV9TL8FmxR2N9ExZTZwbD9Njc3rMvUcf
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

### Safety of Funds

One of the primary aims of the stake pool program is to always allow pool token
holders to withdraw their funds at any time.

To that end, let's look at the three classes of stake accounts in the stake pool system:

* validator stake: active stake accounts, one per validator in the pool
* transient stake: activating or deactivating stake accounts, merged into the reserve after deactivation, or into the validator stake after activation, one per validator
* reserve stake: inactive stake, to be used by the staker for rebalancing

Additionally, the staker may set a "preferred withdraw account", which forces users
to withdraw from a particular stake account.  This is to prevent malicious
depositors from using the stake pool as a free conversion between validators.

When processing withdrawals, the order of priority goes:

* preferred withdraw validator stake account (if set)
* validator stake accounts
* transient stake accounts
* reserve stake account

If there is preferred withdraw validator, and that validator stake account has
any SOL, a user must withdraw from that account.

If that account is empty, or the preferred withdraw validator stake account is
not set, then the user must withdraw from any validator stake account.

If all validator stake accounts are empty, which may happen if the stake pool
staker decreases the stake on all validators at once, then the user must withdraw
from any transient stake account.

If all transient stake accounts are empty, then the user must withdraw from the
reserve.

In this way, a user's funds are never at risk, and always redeemable.

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
