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


## Background

Solana's programming model and the definitions of the Solana terms used in this
document are available at:

- https://docs.solana.com/apps
- https://docs.solana.com/terminology

## Source

The Stake Pool Program's source is available on
[github](https://github.com/solana-labs/solana-program-library).

## Operational overview

The following explains the instructions available in the Stake Pool Program along
with examples using the command-line utility.

## Very important note concerning activated stakes

In its current iteration, the stake pool only processes fully activated stakes.
Deposits must come from fully active stakes, and withdrawals return a fully 
active stake account.

This feature maintains fungibility of stake pool tokens. Fully activated stakes
are not equivalent to inactive, activating, or deactivating stakes due to the
time cost of staking. Otherwise, malicious actors can deposit stake in one state
and withdraw it in another state without waiting.

Future iterations of the stake pool program aim to improve on this limitation.

## Command-line Utility

The `spl-stake-pool` command-line utility can be used to experiment with SPL
tokens.  Once you have [Rust installed](https://rustup.rs/), run:
```sh
$ cargo install spl-stake-pool-cli
```

Run `spl-stake-pool --help` for a full description of available commands.

### Configuration

The `spl-stake-pool` configuration is shared with the `solana` command-line tool.

#### Current Configuration

```
solana config get
```

```
Config File: ${HOME}/.config/solana/cli/config.yml
RPC URL: https://api.mainnet-beta.solana.com
WebSocket URL: wss://api.mainnet-beta.solana.com/ (computed)
Keypair Path: ${HOME}/.config/solana/id.json
```

#### Cluster RPC URL

See [Solana clusters](https://docs.solana.com/clusters) for cluster-specific RPC URLs
```
solana config set --url https://devnet.solana.com
```

#### Default Keypair

See [Keypair conventions](https://docs.solana.com/cli/conventions#keypair-conventions)
for information on how to setup a keypair if you don't already have one.

Keypair File
```
solana config set --keypair ${HOME}/new-keypair.json
```

Hardware Wallet URL (See [URL spec](https://docs.solana.com/wallet-guide/hardware-wallets#specify-a-keypair-url))
```
solana config set --keypair usb://ledger/
```

### Example: Creating your own stake pool with 3% fee

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
`3xvXPfQi2SaTkqPV9A7BQwh4GyTe2ZPasfoaCBCnTAJ5`. When users deposit SOL into the
stake pool, the program will transfer 3% of their contribution into this account.

### Example: Create validator stake account

In order to accommodate large numbers of user deposits into the stake pool, the
stake pool only manages one stake account per validator. To add a new validator
to the stake pool, we first create a validator-associated stake account.

Looking at [validators.app](https://www.validators.app/) or other Solana validator
lists, we choose some validators at random and start with
identity `8SQEcP4FaYQySktNQeyxF3w8pvArx3oMEh7fPrzkN9pu` on vote account 
`2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3`. Let's create a validator stake account
delegated to that vote account.

```sh
$ spl-stake-pool create-validator-stake --pool 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC --validator 2HUKQz7W2nXZSwrdX5RkfS2rLU4j1QZLjdGCHcoUKFh3
Creating stake account FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN
Signature: 4pA2WKT6d2wkXEtSpiQswv22WyoFad2KX6FdPEzwBiEquvaUBEtzenys5Jh1ABPCh7yc4w8kzqMRRCwDj6ZSUV1K
```

In order to maximize censorship resistance, we want to distribute our SOL to as
many validators as possible, so let's add a few more.

```sh
$ spl-stake-pool create-validator-stake --pool 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC --validator HJiC8iJ4Sj846SswQuauFJK93UvV6zp3c2T6jzGqzhhz
Creating stake account E5KBATUd21Dnjnh5sGFw5ngp9kdVXCcAAYMRe2WsVXie
Signature: 4pyRZzjsWG7jP3GRZeZCo2Eb2TPjHM4kAYRFMivimme6HAee1nhzoNJBe3VSt2sv7acp5fwT7J8omBM8o3niY8gu
$ spl-stake-pool create-validator-stake --pool 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC --validator AUCzCaGAGjL3uyjFBtJs7KuJcgQWvNZu1Z2S9G3pw77G
Creating stake account CrStLEWfme37kDc3nubK9HsmWR5dsuVUuqEKqTR4Mc5E
Signature: 4ZUdZzUARgUCPuY8nVsJbN6vRDbVX8sYAQGYYXj2YVvjoJ2oevq2H8uzrhYApe419uoP7QYukqNstiti5p5DDukN
$ spl-stake-pool create-validator-stake --pool 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC --validator 8r1f8mwrUiYdg2Rx9sxTh4M3UAUcCBBrmRA3nxk3Z6Lm
Creating stake account FhFft7ArhZZkh6q4ir1JZMYFgXdH6wkT5M5nmDDb1Q13
Signature: yQqXCbuA66wQsHtkziNg3XadfZF5aCmvjfentwbZJnSPeEjJwPka3M1QY5GmR1efprptqaePn71BTMSLscX8DLr
```

NOTE: These stake accounts have not been added to the stake pool yet. Stake pools
only accept deposits from fully delegated (warmed-up) stake accounts, so we must
first delegate these stakes.

We can see the status of stake account using the Solana command-line utility.

```sh
$ solana stake-account FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN
Balance: 0.002282881 SOL
Rent Exempt Reserve: 0.00228288 SOL
Stake account is undelegated
Stake Authority: 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Withdraw Authority: 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
```

The stake pool creates these special staking accounts with 1 lamport as a
minimum delegation.  We must delegate them ourselves to the vote account specified
on the creation.

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

### Example: (Admin only) Add validator stake account

As mentioned in the last step, the stake pool only manages one stake account per
validator. Also, the stake pool only processes fully activated stake accounts.
We created new validator stake accounts in the last step and staked them. Once 
the stake activates, we can add them to the stake pool.

```sh
$ spl-stake-pool add-validator-stake --pool 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC --stake FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN
Creating account to receive tokens Gu8xqzYFg2sPHWHhUivKNBeF9uikiauihLs9hLzziKu7
Signature: 3N1K89rGV9gWueTTrPGTDBwKAp8BikQhKHMFoREw98Q1piXFeZSSxqfnRQexrfAZQfrpYH9qwsaPWRruwkVeBivV
```

Users can start depositing their activated stakes into the stake pool, as
long as they are delegated to the same vote account, which was
`FhFft7ArhZZkh6q4ir1JZMYFgXdH6wkT5M5nmDDb1Q13` in this example.  You can also
double-check that at any time using the base Solana command-line utility.

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

### Example: List validator stake accounts

In order to deposit into the stake pool, a user must first delegate some stake
to one of the validator stake accounts associated with the stake pool. The
command-line utility has a special instruction for finding out which vote
accounts are already associated with the stake pool.

```sh
$ spl-stake-pool list --pool 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC
CrStLEWfme37kDc3nubK9HsmWR5dsuVUuqEKqTR4Mc5E    1.002282881 SOL
E5KBATUd21Dnjnh5sGFw5ngp9kdVXCcAAYMRe2WsVXie    1.002282881 SOL
FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN    1.002282881 SOL
FhFft7ArhZZkh6q4ir1JZMYFgXdH6wkT5M5nmDDb1Q13    1.002282881 SOL
Total: 4.009131524 SOL
```

If the manager has recently created the stake pool, and there are no stake
accounts present yet, the command-line utility will inform us.

```sh
$ spl-stake-pool list --pool 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC
No accounts found.
```

### Example: Deposit stake

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

One epoch later, when the stake is fully active, we can deposit the stake into 
the stake pool.

```sh
$ spl-stake-pool deposit --pool 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC --stake 4F4AYKZbNtDnu7uQey2Vkz9VgkVtLE6XWLezYjc9yxZa
TODO
```

In return, the stake pool has sent us staking derivatives in the form of SPL
tokens.  We can double-check our stake pool account using the SPL token
command-line utility.

### Example: Update

Every epoch, the network pays out rewards to stake accounts managed by the stake
pool, increasing the value of staking derivative SPL tokens given on deposit.
In order to calculate the proper value of these stake pool tokens, we must update
the total value managed by the stake pool.

The Solana transaction processor has two important limitations:

* size of the overall transaction, limited to roughly 1 MTU / packet
* computation budget per instruction

A stake pool may manage hundreds of staking accounts, so it is impossible to
update the total value of the stake pool in one instruction. Thankfully, the
command-line utility does all of the work of breaking up transactions.

```sh
$ spl-stake-pool update --pool 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC 
Signature: 3Yx1RH3Afqj5ckX8YvPCRt1DudVP4HuRPkh1dBPvTM9GqGxcB9ZXHGZPADVSZiaqKi166fevMG232EWxrRWswPtt
```

If another user already updated the stake pool balance for the current epoch, we
see a different output.

```sh
$ spl-stake-pool update --pool 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC 
Stake pool balances are up to date, no update required.
```

### Example: Withdraw stake

Whenever we want to recover SOL plus accrued rewards, we can provide our
staking derivative SPL tokens in exchange for an activated stake account.

Let's withdraw 10 of our staking derivative tokens from the stake pool.

```sh
$ spl-stake-pool withdraw --pool 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC  --amount 10 --burn-from 111111111111111111
TODO
```

Our 10 tokens were taken, and in exchange we received a fully active stake
account with X tokens, delegated to X. We can leave this stake account as it is,
or we can deactivate it as a normal stake account.

```sh
$ solana deactivate-stake XXXXXXXXX
TODO
```

Once the stake is no longer active, we can use it as normal fungible SOL.

### Example: (Admin only) Remove validator stake account

If the stake pool manager wants to stop delegating to a vote account, they can
totally remove the validator stake account from the stake pool.

```sh
$ spl-stake-pool remove-validator-stake --pool 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC --stake FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN --burn-from XXXXXXX
TODO
```

This operation works just like `withdraw`, in that the stake pool manager provides
SPL staking derivatives in exchange for an activated stake account. The difference
is that the validator stake account is totally removed from the stake pool.

### Example: (Admin only) Set staking authority

In order to manage the stake accounts more directly, the stake pool owner can 
set the stake authority of the stake pool's managed accounts.

```sh
$ spl-stake-pool set-staking-auth --pool 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC --stake-account FYQB64aEzSmECvnG8RVvdAXBxRnzrLvcA3R22aGH2hUN --new-staker 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Signature: 39N5gkaqXuWm6JPEUWfenKXeG4nSa71p7iHb9zurvdZcsWmbjdmSXwLVYfhAVHWucTY77sJ8SkUNpVpVAhe4eZ53
```

Now, the new staker can perform any normal staking operations, including deactivating
or re-staking.

Important security note: the stake pool program only gives staking authority to
the pool owner and always retains withdraw authority. Therefore, a malicious 
stake pool manager cannot steal funds from the stake pool.

### Example: (Admin only) Set owner

The stake pool owner may pass their admin privileges to another account.

```sh
$ spl-stake-pool --pool 3CLwo9CntMi4D1enHEFBe3pRJQzGJBCAYe66xFuEbmhC --new-owner 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Signature: 39N5gkaqXuWm6JPEUWfenKXeG4nSa71p7iHb9zurvdZcsWmbjdmSXwLVYfhAVHWucTY77sJ8SkUNpVpVAhe4eZ53
```
