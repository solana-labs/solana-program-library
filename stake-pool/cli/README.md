# SPL Stake Pool program command-line utility

A basic command-line for creating and using SPL Stake Pools.  See https://spl.solana.com/stake-pool for more details.

## Scripts for setting up a stake pool

Under `./scripts`, this repo also contains some bash scripts that are useful for
setting up your own stake pool. These scripts require the Solana CLI tool suite,
which can be downloaded by following the instructions at
(https://docs.solana.com/cli/install-solana-cli-tools). Additionally, you must
have a usable keypair, created at the default location using `solana-keygen new`.

### setup-local.sh

Builds the stake pool program and sets up a `solana-test-validator` with some
new validator vote accounts.

The only input it accepts is a number, for the number of vote accounts to create, e.g.:

```bash
$ ./setup-local.sh 100
```

#### Important notes on local network

If you are using epochs of 32 slots, there is a good chance
that you will pass an epoch while using one of the stake pool commands, causing
it to fail with: `Custom program error: 0x11`. This is totally normal, and will
not happen on a normal network.

Since there is no voting activity on the test validator network, you will
need to use the `--force` flag with `solana delegate-stake`, ie:

```bash
$ solana delegate-stake --force stake.json CzDy6uxLTko5Jjcdm46AozMmrARY6R2aDBagdemiBuiT
```

### setup-stake-pool.sh

Creates a new stake pool with the parameters hardcoded in the script:

* fee numerator
* fee denominator
* maximum number of validators
* list of validator vote accounts

```bash
$ ./setup-stake-pool.sh 100 validator_list.txt
```

### deposit-withdraw.sh

Creates new stakes to deposit into each of the stake accounts in the pool, given
the stake pool, mint, and validator list.

```bash
$ ./deposit-withdraw.sh keys/stake-pool.json validator_list.txt
```
