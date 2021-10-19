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

It accepts the number of vote accounts to create and validator list file path to output
vote accounts, e.g.:

```bash
$ ./setup-local.sh 100 validator_list.txt
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

* epoch fee numerator
* epoch fee denominator
* withdrawal fee numerator
* withdrawal fee denominator
* deposit fee numerator
* deposit fee denominator
* referral fee
* manager
* staker
* maximum number of validators
* list of validator vote accounts
* (Optional) deposit authority, for restricted pools

Modify the parameters to suit your needs, and your pool will be created!

```bash
$ ./setup-stake-pool.sh
```

### deposit-withdraw.sh

Creates new stakes to deposit into each of the stake accounts in the pool, given
the stake pool, mint, and validator list.

```bash
$ ./deposit-withdraw.sh keys/stake-pool.json validator_list.txt
```
