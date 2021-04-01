---
title: Stake Pool Program
---

A program for pooling together SOL to be staked by an off-chain agent running
a Delegation bot which redistributes the stakes across the network and tries
to maximize censorship resistance and rewards.

## Overview

SOL token holders can earn rewards and help secure the network by staking tokens
to one or more validators. Rewards for staked tokens are based on the current
inflation rate, total number of SOL staked on the network, and an individual
validator’s uptime and commission (fee).

Stake pools are an alternative method of earning staking rewards. This on-chain
program pools together SOL to be staked by a manager, allowing SOL holders to
stake and earn rewards without managing stakes.

Additional information regarding staking and stake programming is available at:

- https://solana.com/staking
- https://docs.solana.com/staking/stake-programming

## Motivation

This document is intended for stake pool managers who want to create or manage
stake pools, and users who want to provide staked SOL into an existing stake
pool.

In its current iteration, the stake pool only processes totally active stakes.
Deposits must come from fully active stakes, and withdrawals return a fully
active stake account.

This means that stake pool managers and users must be comfortable with creating
and delegating stakes, which are more advanced operations than sending and
receiving SPL tokens and SOL. Additional information on stake operations are
available at:

- https://docs.solana.com/cli/delegate-stake
- https://docs.solana.com/cli/manage-stake-accounts

To reach a wider audience of users, stake pool managers are encouraged
to provide a market for their pool's staking derivatives, through an AMM
like [Token Swap](token-swap.md).

## Operation

A stake pool manager creates a stake pool and includes validators that will
receive delegations from the pool by creating "validator stake accounts" and
activating a delegation on them. Once a validator stake account's delegation is
active, the stake pool manager adds it to the stake pool.

At this point, users can participate with deposits. They must delegate a stake
account to the one of the validators in the stake pool. Once it's active, the
user can deposit their stake into the pool in exchange for SPL staking derivatives
representing their fractional ownership in pool. A percentage of the user's
deposit goes to the pool manager as a fee.

Over time, as the stake pool accrues staking rewards, the user's fractional
ownership will be worth more than their initial deposit. Whenever the user chooses,
they can withdraw their SPL staking derivatives in exchange for an activated stake.

The stake pool manager can add and remove validators, or rebalance the pool by
withdrawing stakes from the pool, deactivating them, reactivating them on another
validator, then depositing back into the pool.

These manager operations require SPL staking derivatives and staked SOL, so the
stake pool manager will need liquidity on hand to properly manage the pool.

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
solana config set --url https://devnet.solana.com
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

### Stake Pool Administrator Examples

#### Create a stake pool

The pool administrator manages the stake accounts in a stake pool, and in exchange
receives a fee in the form of SPL token staking derivatives. The administrator
sets the fee on creation. Let's create a pool with a 3% fee:

```sh
$ spl-stake-pool create-pool --fee-numerator 3 --fee-denominator 100
Creating mint Gmk71cM7j2RMorRsQrsyysM4HsByQx5PuDGtDdqGLWCS
Creating pool fee collection account 3xvXPfQi2SaTkqPV9A7BQwh4GyTe2ZPasfoaCBCnTAJ5
Creating stake pool 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC
Signature: 5HdDoPssqwyLjt2QvhRbnSATZqFLGKha92zMuJiBUpKeKYKGURRV41N5ydCQxqnFjCud3xv85Z6ghErppNJzaYM8
```

The unique stake pool identifier is `3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC`.

The identifier for the SPL token for staking derivatives is
`Gmk71cM7j2RMorRsQrsyysM4HsByQx5PuDGtDdqGLWCS`. The stake pool has full control
over the mint.

The pool creator's fee account identifier is
`3xvXPfQi2SaTkqPV9A7BQwh4GyTe2ZPasfoaCBCnTAJ5`. When users deposit warmed up
stake accounts into the stake pool, the program will transfer 3% of their
contribution into this account in the form of SPL token staking derivatives.

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
$ spl-stake-pool create-validator-stake 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC 2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3
Creating stake account FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN
Signature: 4pA2WKT6d2wkXEtSpiQswv22WyoFad2KX6FdPEzwBiEquvaUBEtzenys5Jh1ABPCh7yc4w8kzqMRRCwDj6ZSUV1K
```

In order to maximize censorship resistance, we want to distribute our SOL to as
many validators as possible, so let's add a few more.

```sh
$ spl-stake-pool create-validator-stake 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC HJiC8iJ4Sj846SswQuauFJK93UvV6zp3c2T6jzGqzhhz
Creating stake account E5KBATUd21Dnjnh5sGFw5ngp9kdVXCcAAYMRe2WsVXie
Signature: 4pyRZzjsWG7jP3GRZeZCo2Eb2TPjHM4kAYRFMivimme6HAee1nhzoNJBe3VSt2sv7acp5fwT7J8omBM8o3niY8gu
$ spl-stake-pool create-validator-stake 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC AUCzCaGAGjL3uyjFBtJs7KuJcgQWvNZu1Z2S9G3pw77G
Creating stake account CrStLEWfme37kDc3nubK9HsmWR5dsuVUuqEKqTR4Mc5E
Signature: 4ZUdZzUARgUCPuY8nVsJbN6vRDbVX8sYAQGYYXj2YVvjoJ2oevq2H8uzrhYApe419uoP7QYukqNstiti5p5DDukN
$ spl-stake-pool create-validator-stake 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC 8r1f8mwrUiYdg2Rx9sxTh4M3UAUcCBBrmRA3nxk3Z6Lm
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
$ spl-stake-pool add-validator 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN
Creating account to receive tokens Gu8xqzYFg2sPHWHhUivKNBeF9uikiauihLs9hLzziKu7
Signature: 3N1K89rGV9gWueTTrPGTDBwKAp8BikQhKHMFoREw98Q1piXFeZSSxqfnRQexrfAZQfrpYH9qwsaPWRruwkVeBivV
```

Users can start depositing their activated stakes into the stake pool, as
long as they are delegated to the same vote account, which was
`FhFft7ArhZZkh6q4ir1JZMYFgXdH6wkT5M5nmDDb1Q13` in this example.  You can also
double-check that at any time using the Solana command-line utility.

```sh
$ solana stake-account FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN
Balance: 0.002282881 SOL
Rent Exempt Reserve: 0.00228288 SOL
Delegated Stake: 0.000000001 SOL
Active Stake: 0.000000001 SOL
Activating Stake: 0 SOL
Stake activates starting from epoch: 161
Delegated Vote Account Address: 2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3
Stake Authority: 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Withdraw Authority: 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
```

#### Remove validator stake account

If the stake pool manager wants to stop delegating to a vote account, they can
totally remove the validator stake account from the stake pool by providing
staking derivatives, just like `withdraw`.

```sh
$ spl-stake-pool remove-validator 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC CrStLEWfme37kDc3nubK9HsmWR5dsuVUuqEKqTR4Mc5E --withdraw-from 34XMHa3JUPv46ftU4dGHvemZ9oKVjnciRePYMcX3rjEF
Signature: 5rrQ3xhDWyiPkUTAQkNAeq31n6sMf1xsg2x9hVY8Vj1NonwBnhxuTv87nADLkwC8Xzc4CGTNCTX2Vph9esWnXk2d
```

The difference with `withdraw` is that the validator stake account is totally
removed from the stake pool and now belongs to the administrator.

We can check the removed stake account:

```sh
$ solana stake-account CrStLEWfme37kDc3nubK9HsmWR5dsuVUuqEKqTR4Mc5E
Balance: 1.002282881 SOL
Rent Exempt Reserve: 0.00228288 SOL
Delegated Stake: 1.000000001 SOL
Active Stake: 1.000000001 SOL
Delegated Vote Account Address: AUCzCaGAGjL3uyjFBtJs7KuJcgQWvNZu1Z2S9G3pw77G
Stake Authority: 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Withdraw Authority: 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
```

The administrator's SPL token account has been debited to accommodate the
removal of staked SOL from the pool.

We can also double-check that the stake pool no longer shows the stake account:

```sh
$ spl-stake-pool list 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC
Pubkey: FhFft7ArhZZkh6q4ir1JZMYFgXdH6wkT5M5nmDDb1Q13    Vote: 8r1f8mwrUiYdg2Rx9sxTh4M3UAUcCBBrmRA3nxk3Z6Lm ◎1.002282881
Pubkey: FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN    Vote: 2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3 ◎3.410872673
Pubkey: E5KBATUd21Dnjnh5sGFw5ngp9kdVXCcAAYMRe2WsVXie    Vote: HJiC8iJ4Sj846SswQuauFJK93UvV6zp3c2T6jzGqzhhz ◎11.436803652
Total: ◎15.849959206
```

#### Rebalance the stake pool

As time goes on, deposits and withdrawals will happen to all of the stake accounts
managed by the pool, and the stake pool manager may want to rebalance the stakes.

For example, let's say the manager wants the same delegation to every validator
in the pool. When they look at the state of the pool, they see:

```sh
$ spl-stake-pool list 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC
Pubkey: FhFft7ArhZZkh6q4ir1JZMYFgXdH6wkT5M5nmDDb1Q13    Vote: 8r1f8mwrUiYdg2Rx9sxTh4M3UAUcCBBrmRA3nxk3Z6Lm ◎1.002282881
Pubkey: FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN    Vote: 2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3 ◎3.410872673
Pubkey: E5KBATUd21Dnjnh5sGFw5ngp9kdVXCcAAYMRe2WsVXie    Vote: HJiC8iJ4Sj846SswQuauFJK93UvV6zp3c2T6jzGqzhhz ◎11.436803652
Total: ◎15.849959206
```

This isn't great! The last stake account, `E5KBATUd21Dnjnh5sGFw5ngp9kdVXCcAAYMRe2WsVXie`
has too much allocated. For their strategy, the manager wants the `15.849959206`
SOL to be distributed evenly, meaning around `5.283319735` in each account. They need
to move `4.281036854` to `FhFft7ArhZZkh6q4ir1JZMYFgXdH6wkT5M5nmDDb1Q13` and
`1.872447062` to `FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN`.

First, they need to withdraw a total of `6.153483916` from
`E5KBATUd21Dnjnh5sGFw5ngp9kdVXCcAAYMRe2WsVXie`. Using the `spl-token` utility,
let's check the total supply of pool tokens:

```sh
$ spl-token supply Gmk71cM7j2RMorRsQrsyysM4HsByQx5PuDGtDdqGLWCS
0.034692168
```

Given a total pool token supply of `0.034692168` and total staked SOL amount of
`15.849959206`, let's calculate how many pool tokens to withdraw from the pool:

```
sol_to_withdraw * total_pool_tokens / total_sol_staked = pool_tokens_to_withdraw
6.153483916 * 0.034692168 / 15.849959206 ~ 0.013468659
```

They withdraw that amount of pool tokens:

```sh
$ spl-stake-pool withdraw 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC --amount 0.013468659 --withdraw-from 34XMHa3JUPv46ftU4dGHvemZ9oKVjnciRePYMcX3rjEF
Withdrawing from account E5KBATUd21Dnjnh5sGFw5ngp9kdVXCcAAYMRe2WsVXie, amount ◎6.153483855, 0.013468659 pool tokens
Creating account to receive stake 8ykyY7maA9HUfUphZHBkhsnydY5gFfyHFSfxCA7imqrk
Signature: z8a5ZRfWdj8Fcsr3ttCJ731wFKyhZNcqoKEdV1RBCkzr3tHGQNCC56qvRVJ6oxyCVDqWZ3KL1Bkyn3sDpjYPDku
```

Because of rounding in the calculation a few lines above, it looks like we receive
less than we should.  If we play that back the other way, we'll see that all is well:

```
pool_tokens_to_withdraw * total_sol_staked / total_pool_tokens = sol_to_withdraw
0.013468659 * 15.849959206 / 0.034692168 ~ 6.153483855
```

Next, they deactivate the new received stake:

```sh
$ solana deactivate-stake 8ykyY7maA9HUfUphZHBkhsnydY5gFfyHFSfxCA7imqrk
Signature: 4SuwZK5JvYkYVkM5yfu2x8x6iou6558teMwzphGECLmstMVoWbSvngUH48Ra24PrxtgUDyVDA8SXYS1qMyx3fjMj
```

Once the stake is deactivated during the next epoch, they split the stake
and activate it on the other two validator vote accounts. For brevity, those
commands are omitted.

Eventually, we are left with stake account `4zppED2kFodUS2hBf8Fzeepu6yZ6QuyeNPBXCT9VU6fK`
with `4.281036854` delegated to `8r1f8mwrUiYdg2Rx9sxTh4M3UAUcCBBrmRA3nxk3Z6Lm`
and stake account `GCJnuFGCDzaToPwJtG5GiK4g3DJBfuhQy6388NyGcfwf` with `1.872447062`
delegated to `2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3`.

Once the new stakes are ready, the manager deposits them back into the stake pool:
```sh
$ spl-stake-pool deposit 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC GCJnuFGCDzaToPwJtG5GiK4g3DJBfuhQy6388NyGcfwf --token-receiver 34XMHa3JUPv46ftU4dGHvemZ9oKVjnciRePYMcX3rjEF
Depositing into stake account FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN
Signature: jKsdEr3zxF2zZs78rmrP3PmQiTwE7v15ieEuxp4db1VQe9owXVGM8nM3dJqVRHXPsS4frQW4gJ6xBfTTk2HvKDX
$ spl-stake-pool deposit 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC 4zppED2kFodUS2hBf8Fzeepu6yZ6QuyeNPBXCT9VU6fK --token-receiver 34XMHa3JUPv46ftU4dGHvemZ9oKVjnciRePYMcX3rjEF
Depositing into stake account FhFft7ArhZZkh6q4ir1JZMYFgXdH6wkT5M5nmDDb1Q13
Signature: 3JXvTvea6F4Epd2krSxnTRZPB4gLZ8GqisFE58Z4ocV92fDN1HRMVPoPhJtYcfuF12vyQZUueKwVmkvL6Wgf2evc
```

Leaving them with a rebalanced stake pool!

```sh
$ spl-stake-pool list 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC
Pubkey: FhFft7ArhZZkh6q4ir1JZMYFgXdH6wkT5M5nmDDb1Q13    Vote: 8r1f8mwrUiYdg2Rx9sxTh4M3UAUcCBBrmRA3nxk3Z6Lm ◎5.283340235
Pubkey: FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN    Vote: 2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3 ◎5.283612231
Pubkey: E5KBATUd21Dnjnh5sGFw5ngp9kdVXCcAAYMRe2WsVXie    Vote: HJiC8iJ4Sj846SswQuauFJK93UvV6zp3c2T6jzGqzhhz ◎5.284317422
Total: ◎15.851269888
```

Due to staking rewards that accrued during the rebalancing process, the pool is
not prefectly balanced. This is completely normal.

#### Set staking authority

In order to manage the stake accounts more directly, the stake pool owner can
set the stake authority of the stake pool's managed accounts.

```sh
$ spl-stake-pool set-staking-auth 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC --stake-account FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN --new-staker 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Signature: 39N5gkaqXuWm6JPEUWfenKXeG4nSa71p7iHb9zurvdZcsWmbjdmSXwLVYfhAVHWucTY77sJ8SkUNpVpVAhe4eZ53
```

Now, the new staking authority can perform any normal staking operations,
including deactivating or re-staking.

Important security note: the stake pool program only gives staking authority to
the pool owner and always retains withdraw authority. Therefore, a malicious
stake pool manager cannot steal funds from the stake pool.

#### Set owner

The stake pool owner may pass their administrator privileges to another account.

```sh
$ spl-stake-pool 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC --new-owner 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Signature: 39N5gkaqXuWm6JPEUWfenKXeG4nSa71p7iHb9zurvdZcsWmbjdmSXwLVYfhAVHWucTY77sJ8SkUNpVpVAhe4eZ53
```

### User Examples

#### List validator stake accounts

In order to deposit into the stake pool, a user must first delegate some stake
to one of the validator stake accounts associated with the stake pool. The
command-line utility has a special instruction for finding out which vote
accounts are already associated with the stake pool.

```sh
$ spl-stake-pool list 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC
CrStLEWfme37kDc3nubK9HsmWR5dsuVUuqEKqTR4Mc5E    1.002282880 SOL
E5KBATUd21Dnjnh5sGFw5ngp9kdVXCcAAYMRe2WsVXie    1.002282880 SOL
FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN    1.002282880 SOL
FhFft7ArhZZkh6q4ir1JZMYFgXdH6wkT5M5nmDDb1Q13    1.002282880 SOL
Total: 4.009131520 SOL
```

If the manager has recently created the stake pool, and there are no stake
accounts present yet, the command-line utility will inform us.

```sh
$ spl-stake-pool list 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC
No accounts found.
```

#### Deposit stake

Stake pools only accept deposits from fully staked accounts, so we must first
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
$ spl-stake-pool deposit 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC 4F4AYKZbNtDnu7uQey2Vkz9VgkVtLE6XWLezYjc9yxZa
Depositing into stake account FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN
Creating account to receive tokens 34XMHa3JUPv46ftU4dGHvemZ9oKVjnciRePYMcX3rjEF
Signature: 4AESGZzqBVfj5xQnMiPWAwzJnAtQDRFK1Ha6jqKKTs46Zm5fw3LqgU1mRAT6CKTywVfFMHZCLm1hcQNScSMwVvjQ
```

Alternatively, you can create an SPL token account yourself and pass it as the
`token-receiver` for the command.

```sh
$ spl-stake-pool deposit 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC 4F4AYKZbNtDnu7uQey2Vkz9VgkVtLE6XWLezYjc9yxZa --token-receiver 34XMHa3JUPv46ftU4dGHvemZ9oKVjnciRePYMcX3rjEF
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
$ spl-stake-pool update 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC
Signature: 3Yx1RH3Afqj5ckX8YvPCRt1DudVP4HuRPkh1dBPvTM9GqGxcB9ZXHGZPADVSZiaqKi166fevMG232EWxrRWswPtt
```

If another user already updated the stake pool balance for the current epoch, we
see a different output.

```sh
$ spl-stake-pool update 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC
Stake pool balances are up to date, no update required.
```

If no one updates the stake pool in the current epoch, the deposit and withdraw
instructions will fail. The update instruction is permissionless, so any user
can run it before depositing or withdrawing.

#### Withdraw stake

Whenever the user wants to recover SOL plus accrued rewards, they can provide their
staking derivative SPL tokens in exchange for an activated stake account.

Let's withdraw 0.02 staking derivative tokens from the stake pool.

```sh
$ spl-stake-pool withdraw 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC --amount 0.02 --withdraw-from 34XMHa3JUPv46ftU4dGHvemZ9oKVjnciRePYMcX3rjEF
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

Alternatively, the user can specify an existing stake account to receive their
stake using the `stake-receiver` parameter.

```sh
$ spl-stake-pool withdraw 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC  --amount 0.02 --withdraw-from 34XMHa3JUPv46ftU4dGHvemZ9oKVjnciRePYMcX3rjEF --stake-receiver CZF2z3JJoDmJRcVjtsrz1BKUUGNL3VPW5FPFqge1bzmQ
Withdrawing from account FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN, amount 8.867176377 SOL, 0.02 pool tokens
Signature: 2xBPVPJ749AE4hHNCNYdjuHv1EdMvxm9uvvraWfTA7Urrvecwh9w64URCyLLroLQ2RKDGE2QELM2ZHd8qRkjavJM
```

## Appendix

### Activated stakes

As mentioned earlier, the stake pool only processes active stakes. This feature
maintains fungibility of stake pool tokens. Fully activated stakes
are not equivalent to inactive, activating, or deactivating stakes due to the
time cost of staking. Otherwise, malicious actors can deposit stake in one state
and withdraw it in another state without waiting.

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
