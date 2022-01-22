---
title: Quick Start Guide
---

This quick start guide is meant for managers who want to start running a pool
right away.

## Prerequisites

This guide requires the Solana CLI tool suite and Stake Pool CLI tool.

- [Install the Solana Tools](https://docs.solana.com/cli/install-solana-cli-tools)
- [Install the Stake Pool CLI](cli.md)

You must also have an account with SOL. The guide will assume that you
are using the default keypair created at the default location using `solana-keygen new`.
Note that it is possible to override the default keypair with every command if
needed.

If you are running on localhost using `solana-test-validator`, the default keypair
will automatically start with 500,000,000 SOL.

If you are running on devnet or testnet, you can airdrop funds using `solana airdrop 1`.

If you are running on mainnet-beta, you must purchase funds some other way, from
an exchange, a friend, etc.

## Sample scripts

This guide uses the
[sample scripts on GitHub](https://github.com/solana-labs/solana-program-library/tree/master/stake-pool/cli/scripts)
to run everything quickly and easily.

You'll see the following scripts:

* `setup-test-validator.sh`: sets up a local test validator with validator vote accounts
* `setup-stake-pool.sh`: creates a new stake pool with hardcoded parameters
* `add-validators.sh`: adds validators to the stake pool
* `deposit.sh`: performs stake and SOL deposits
* `rebalance.sh`: rebalances the stake pool
* `withdraw.sh`: performs some withdrawals

This guide will use most of these scripts to setup a stake pool on a local
network.

## (Optional) Step 0: Setup a local network for testing

All of these scripts can be run against devnet, testnet, or mainnet-beta, but
to allow for more experimentation, we will setup a local validator with some
validator vote accounts using `setup-test-validator.sh`.

The script accepts the number of vote accounts to create and file path to output
validator vote accounts, e.g.:

```bash
$ ./setup-test-validator.sh 10 local_validators.txt
```

This will take roughly 10 seconds, eventually outputting a file with list of
base58-encoded public keys. These represent validator vote accounts on the
local network, e.g.:

```
EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ
J3xu64PWShcMen99kU3igxtwbke2Nwfo8pkZNRgrq66H
38DYMkwYCvsj8TC6cNaEvFHHVDYeWDp1qUgMgyjNqZXk
7q371UZcYJTMmFPeijUJ6RBr6jHE9t4mDd2gnDs7wpje
7ffftyketRJrmCcczhSnWatxB32SzAG3dhDpnyRdm91d
HtqJXQNWr4E1qxftAxxqNnHbpSYnokayHSxurzS9vKKF
4e6EmSSmExdRM6tF1osYiAq9HxXN5oVvDqS78FcT6F4P
DrT6VGqqJT1GRVaZmuEjNim4ie7ecmNixjiycd67jyJy
71vNo5HBuAtejbcQYp9CdBeT7npVdbJqjmuWbXbNeudq
7FMebvnWnWN45KF5Fa3Y7kAJZReKU6WLzribtWDJybax
```

Note: this will fail if another `solana-test-validator` is already running.

#### Important notes on local network

If you are using epochs of 32 slots, there is a good chance
that you will pass an epoch while using one of the stake pool commands, causing
it to fail with: `Custom program error: 0x11`. This is totally normal, and will
not happen on the other networks. You simply need to re-run the command.

Since there is no voting activity on the test validator network, you will
need to use the secret `--force` flag with `solana delegate-stake`, ie:

```bash
$ solana delegate-stake --force stake.json CzDy6uxLTko5Jjcdm46AozMmrARY6R2aDBagdemiBuiT
```

## Step 1: Create the stake pool

Our next script is `setup-stake-pool.sh`. In it, you will see a large section
in which you can modify parameters for your stake pool. These parameters are used
to create a new stake pool, and include:

* epoch fee, expressed as two different flags, numerator and denominator
* withdrawal fee, expressed as two different flags, numerator and denominator
* deposit fee, expressed as two different flags, numerator and denominator
* referral fee, expressed as a number between 0 and 100, inclusive
* maximum number of validators (highest possible is 2,950 currently)
* (Optional) deposit authority, for restricted pools

Although fees may seem uninteresting or scammy at this point, consider the costs
of running your stake pool, and potential malicious actors that may abuse your pool
if it has no fees.

Each of these parameters is modifiable after pool creation, so there's no need
to worry about being locked in to any choices.

Modify the parameters to suit your needs. The fees are especially important to
avoid abuse, so please take the time to review and calculate fees that work best
for your pool.

Carefully read through the [Fees](fees.md) for more information about fees and
best practices.

In our example, we will use fees of 0.3%, a referral fee of 50%, opt to *not*
set a deposit authority, and have the maximum number of validators (2,950).  Next,
run the script:

```bash
$ ./setup-stake-pool.sh
Creating pool
+ spl-stake-pool create-pool --epoch-fee-numerator 3 --epoch-fee-denominator 1000 --withdrawal-fee-numerator 3 --withdrawal-fee-denominator 1000 --deposit-fee-numerator 3 --deposit-fee-denominator 1000 --referral-fee 50 --max-validators 2950 --pool-keypair keys/stake-pool.json --validator-list-keypair keys/validator-list.json --mint-keypair keys/mint.json --reserve-keypair keys/reserve.json
Creating reserve stake 4tvTkLB4X7ahUYZ2NaTohkG3mud4UBBvu9ZEGD4Wk9mt
Creating mint BoNneHKDrX9BHjjvSpPfnQyRjsnc9WFH71v8wrgCd7LB
Creating associated token account DgyZrAq88bnG1TNRxpgDQzWXpzEurCvfY2ukKFWBvADQ to receive stake pool tokens of mint BoNneHKDrX9BHjjvSpPfnQyRjsnc9WFH71v8wrgCd7LB, owned by 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Creating pool fee collection account DgyZrAq88bnG1TNRxpgDQzWXpzEurCvfY2ukKFWBvADQ
Signature: 51yf2J6dSGAx42KPs2oTMTV4ufEm1ncAHyLPQ6PNf4sbeMHGqno7BGn2tHkUnrd7PRXiWBbGzCWpJNevYjmoLgn2
Creating stake pool Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR with validator list 86VZZCuqiz7sDJpFKjQy9c9dZQN9vwDKbYgY8pcwHuaF
Signature: 47QHcWMEa5Syg13C3SQRA4n88Y8iLx1f39wJXQAStRUxpt2VD5t6pYgAdruNRHUQt1ZBY8QwbvEC1LX9j3nPrAzn
```

Your stake pool now exists! For the largest number of validators, the cost for
this phase is ~2.02 SOL.

## Step 2: Add validators to the pool

Now that the pool exists, we need to add validators to it.

Using `add-validators.sh`, we'll add each of the validators created during step 0
to the stake pool. If you are running on another network, you can create your own
file with validator vote accounts.

```bash
$ ./add-validators.sh keys/stake-pool.json local_validators.txt
Adding validator stake accounts to the pool
Adding stake account 3k7Nwu9jUSc6SNG11wzufKYoZXRFgxWamheGLYWp5Rvx, delegated to EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ
Signature: 5Vm2n3umPXFzQgDiaib1B42k7GqsNYHZWrauoe4DUyFszczB7Hjv9r1DKWKrypc8KDiUccdWmJhHBqM1fdP6WiCm
Signature: 3XtmYu9msqnMeKJs9BopYjn5QTc5hENMXXiBwvEw6HYzU5w6z1HUkGwNW24io4Vu9WRKFFN6SAtrfkZBLK4fYjv4
... (something similar repeated 9 more times)
```

This operation costs 0.00328288 SOL per validator. This amount is totally recoverable
by removing the validator from the stake pool.

## Step 3: Deposit into the pool

Now that your pool has validators, it needs some SOL or stake accounts for you
to manage. There are two possible sources of deposits: SOL or stake accounts.

### a) Depositing SOL

This will likely be the most attractive form of deposit, since it's the easiest
for everyone to use. Normally, this will likely be done from a DeFi app or
wallet, but in our example, we'll do it straight from the command line.  Let's
deposit 10 SOL into our pool:

```
$ spl-stake-pool deposit-sol Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR 100
Using existing associated token account DgyZrAq88bnG1TNRxpgDQzWXpzEurCvfY2ukKFWBvADQ to receive stake pool tokens of mint BoNneHKDrX9BHjjvSpPfnQyRjsnc9WFH71v8wrgCd7LB, owned by 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Signature: 4AJv6hSznYoMGnaQvjWXSBjKqtjYpjBx2MLezmRRjWRDa8vUaBLQfPNGd3kamZNs1JeWSvnzczwtzsMD5WkgKamA
```

Now there will be some SOL for us to work with.

### b) Depositing stake accounts

Alternatively, users can deposit stake accounts into the pool. This option is
particularly attractive for users that already have a stake account, and either
want stake pool tokens in return, or to diversify their stake more.

The `deposit.sh` script gives an idea of how this works with the CLI.

Creates new stakes to deposit a given amount into each of the stake accounts in
the pool, given the stake pool and validator file.

```bash
$ ./deposit.sh keys/stake-pool.json local_validators.txt 10
```

Note: This is a bit more finnicky on a local network because of the short epochs, and
may fail. No problem, you simply need to retry.

## Step 4: Rebalance stake in the pool

Over time, as people deposit SOL into the reserve, or as validator performance
varies, you will want to move stake around. The best way to do this will be
through an automated system to collect information about the stake pool and the
network, and decide how much stake to allocate to each validator.

The Solana Foundation maintains an open-source bot for its delegation program,
which can be adapated for your stake pool. The source code is part of the
[stake-o-matic GitHub repo](https://github.com/solana-labs/stake-o-matic/tree/master/bot).

Additionally, there is a work-in-progress Python stake pool bot, found at the
[stake-pool-py on GitHub](https://github.com/solana-labs/solana-program-library/tree/master/stake-pool/py).

For our example, we will run a simple pool rebalancer, which increases the stake
on each validator in the list by the given amount. There are no checks or logic
to make sure that this is valid.

```bash
$ ./rebalance.sh keys/stake-pool.json local_validators.txt 1
```

## Step 5: Withdraw from the stake pool

Finally, if a user wants to withdraw from the stake pool, they can choose to
withdraw SOL from the reserve if it has enough SOL, or to withdraw from one of
the stake accounts in the pool.

The `withdraw.sh` script removes stakes and SOL from each of the stake accounts
in the pool, given the stake pool, validator file, and amount.

```bash
$ ./withdraw.sh keys/stake-pool.json local_validators.txt 1
```
