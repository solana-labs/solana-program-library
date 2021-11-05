---
title: Command-line Interface
---

The following explains the instructions available in the Stake Pool Program along
with examples using the command-line utility.

## Installation

The `spl-stake-pool` command-line utility can be used to experiment with SPL
tokens.  Once you have [Rust installed](https://rustup.rs/), run:
```console
$ cargo install spl-stake-pool-cli
```

Run `spl-stake-pool --help` for a full description of available commands.

## Configuration

The `spl-stake-pool` configuration is shared with the `solana` command-line tool.

### Current Configuration

```console
$ solana config get
Config File: ${HOME}/.config/solana/cli/config.yml
RPC URL: https://api.mainnet-beta.solana.com
WebSocket URL: wss://api.mainnet-beta.solana.com/ (computed)
Keypair Path: ${HOME}/.config/solana/id.json
```

### Cluster RPC URL

See [Solana clusters](https://docs.solana.com/clusters) for cluster-specific RPC URLs
```console
$ solana config set --url https://api.devnet.solana.com
```

### Default Keypair

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

### Running Locally

If you would like to test a stake pool locally without having to wait for stakes
to activate and deactivate, you can run the stake pool locally using the
`solana-test-validator` tool with shorter epochs, and pulling the current program
from devnet.

```console
$ solana-test-validator -c SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy -c EmiU8AQkB2sswTxVB6aCmsAJftoowZGGDXuytm6X65R3 --url devnet --slots-per-epoch 32
$ solana config set --url http://127.0.0.1:8899
```

## Stake Pool Manager Examples

### Create a stake pool

The stake pool manager controls the stake pool from a high level, and in exchange
receives a fee in the form of SPL tokens. The manager
sets the fee on creation. Let's create a pool with a 3% fee and a maximum of 1000
validator stake accounts:

```console
$ spl-stake-pool create-pool --epoch-fee-numerator 3 --epoch-fee-denominator 100 --max-validators 1000
Creating reserve stake DVwDn4LTRztuai4QeenM6fyzgiwUGpVXVNZ1mgKE1Pyc
Creating mint BoNneHKDrX9BHjjvSpPfnQyRjsnc9WFH71v8wrgCd7LB
Creating associated token account DgyZrAq88bnG1TNRxpgDQzWXpzEurCvfY2ukKFWBvADQ to receive stake pool tokens of mint BoNneHKDrX9BHjjvSpPfnQyRjsnc9WFH71v8wrgCd7LB, owned by 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Creating pool fee collection account DgyZrAq88bnG1TNRxpgDQzWXpzEurCvfY2ukKFWBvADQ
Signature: qQwqahLuC24wPwVdgVXtd7v5htSSPDAH3JxFNmXCv9aDwjjqygQ64VMg3WdPCiNzc4Bn8vtS3qcnUVHVP5MbKgL
Creating stake pool Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR
Signature: 5z6uH3EuPcujeWGpAjBtciSUR3TxtMBgWYU4ULagUso4QGzE9JenhYHwYthJ4b3rS57ByUNEXTr2BFyF5PjWC42Y
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

The `create-pool` command allows setting all of the accounts and keypairs to
pre-generated values, including:

* stake pool, through the `--pool-keypair` flag
* validator list, through the `--validator-list-keypair` flag
* pool token mint, through the `--mint-keypair` flag
* pool reserve stake account, through the `--reserve-keypair` flag

Otherwise, these will all default to newly-generated keypairs.

You can always check out the available options by running `spl-stake-pool create-pool -h`.

### Create a restricted stake pool

If a manager would like to restrict deposits (stake and SOL) to one key in
particular, they can set a deposit authority at creation:

```console
$ spl-stake-pool create-pool --epoch-fee-numerator 3 --epoch-fee-denominator 100 --max-validators 1000 --deposit-authority authority_keypair.json
Creating reserve stake DVwDn4LTRztuai4QeenM6fyzgiwUGpVXVNZ1mgKE1Pyc
Creating mint BoNneHKDrX9BHjjvSpPfnQyRjsnc9WFH71v8wrgCd7LB
Creating associated token account DgyZrAq88bnG1TNRxpgDQzWXpzEurCvfY2ukKFWBvADQ to receive stake pool tokens of mint BoNneHKDrX9BHjjvSpPfnQyRjsnc9WFH71v8wrgCd7LB, owned by 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Creating pool fee collection account DgyZrAq88bnG1TNRxpgDQzWXpzEurCvfY2ukKFWBvADQ
Signature: qQwqahLuC24wPwVdgVXtd7v5htSSPDAH3JxFNmXCv9aDwjjqygQ64VMg3WdPCiNzc4Bn8vtS3qcnUVHVP5MbKgL
Creating stake pool Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR
Deposits will be restricted to 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn only, this can be changed using the set-funding-authority command.
Signature: 5z6uH3EuPcujeWGpAjBtciSUR3TxtMBgWYU4ULagUso4QGzE9JenhYHwYthJ4b3rS57ByUNEXTr2BFyF5PjWC42Y
```

As the output says, the `set-funding-authority` can be used to modify or remove
the deposit authority.

As long as the deposit authority is set, SOL and stake deposits must be signed
by `4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn`, so no one else can participate
in the pool. As mentioned earlier, this feature does not prohibit withdrawals,
so anyone with pool tokens will still be able to withdraw from the pool.

### Set manager

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

### Set fee

The stake pool manager may update any of the fees associated with the stake pool,
passing the numerator and denominator for the fraction that make up the fee.

For an epoch fee of 10%, they could run:

```console
$ spl-stake-pool set-fee Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR epoch 10 100
Signature: 5yPXfVj5cbKBfZiEVi2UR5bXzVDuc2c3ruBwSjkAqpvxPHigwGHiS1mXQVE4qwok5moMWT5RNYAMvkE9bnfQ1i93
```

In order to protect stake pool depositors from malicious managers, the program
applies the new fee for the following epoch.

For example, if the fee is 1% at epoch 100, and the manager sets it to 10%, the
manager will still gain 1% for the rewards earned during epoch 100. Starting
with epoch 101, the manager will earn 10%.

Additionally, to prevent a malicious manager from immediately setting the withdrawal
fee to a very high amount, making it practically impossible for users to withdraw,
the stake pool program currently enforces a limit of 1.5x increase per epoch.

For example, if the current withdrawal fee is 2.5%, the maximum that can be set
for the next epoch is 3.75%.

The possible options for the fee type are `epoch`, `sol-withdrawal`,
`stake-withdrawal`, `sol-deposit`, and `stake-deposit`.

### Set referral fee

The stake pool manager may update the referral fee on deposits at any time, passing
in a percentage amount.

To set a stake deposit referral fee of 80%, they may run:

```console
$ spl-stake-pool set-referral-fee Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR stake 80
Signature: 4vhaBEDhuKkVwMxy7TpyfHEk3Z5kGZKerD1AgajQBdiMRQLZuNZKVR3KQaqbUYZM7UyfRXgkZNdAeP1NfvmwKdqb
```

For 80%, this means that 20% of the stake deposit fee goes to the manager, and
80% goes to the referrer.

### Set staker

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

### Set Funding Authority

To restrict who can interact with the pool, the stake pool manager may require
a particular signature on stake deposits, SOL deposits, or SOL withdrawals. This
does not make the pool private, since all information is available on-chain, but
it restricts who can use the pool.

As an example, let's say a pool wants to restrict all SOL withdrawals.

```console
$ spl-stake-pool set-funding-authority Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR sol-withdraw AZ1PgxWSxw4ezX8gvpNgGsr39jJHCwtkaXr1mNMwWWeK
Signature: 3gx7ckGNSL7gUUyxh4CU3RH3Lyt88hiCvYQ4QRKtnmrZHvAS93ebP6bf39WYGTeKDMVSJUuwBEmk9VFSaWtXsHVV
```

After running this command, `AZ1PgxWSxw4ezX8gvpNgGsr39jJHCwtkaXr1mNMwWWeK` must
sign all SOL withdrawals, otherwise the operation fails.

After some time, if the manager wishes to enable SOL withdrawals, they can remove
the restriction:

```console
$ spl-stake-pool set-funding-authority Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR sol-withdraw --unset
Signature: 5kWeBqoxyvANMHCP4ydsZRf8QU4hMotLnKkFbTEdvqEVywo4F3MpZtay7D57FbjJZpdp72fc3vrbxJi9qDLfLCnD
```

Now, anyone can withdraw SOL from the stake pool, provided there is enough SOL left
in the reserve.

The options for funding authorities are `sol-withdraw`, `sol-deposit`, and `stake-deposit`.

Note: it is impossible to restrict stake withdrawals. This would create an opportunity
for malicious pool managers to effectively lock user funds.

## Stake Pool Staker Examples

### Add a validator to the pool

In order to accommodate large numbers of user deposits into the stake pool, the
stake pool only manages one stake account per validator. To add a new validator
to the stake pool, the staker must use the `add-validator` command.

Let's add some random validators to the stake pool.

```console
$ spl-stake-pool add-validator Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR 38DYMkwYCvsj8TC6cNaEvFHHVDYeWDp1qUgMgyjNqZXk
Adding stake account F8e8Ympp4MkDSPZdvRxdQUZXRkMBDdyqgHa363GShAPt, delegated to 38DYMkwYCvsj8TC6cNaEvFHHVDYeWDp1qUgMgyjNqZXk
Signature: 5tdpsx64mVcSHBK8vMbBzFDHnEZB6GUmVpqSXXE5hezMAzPYwZbJCBtAHakDAiuWNcrMongGrmwDaeywhVz4i8pi
```

In order to maximize censorship resistance, we want to distribute our SOL to as
many validators as possible, so let's add a few more.

```console
$ spl-stake-pool add-validator Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR J3xu64PWShcMen99kU3igxtwbke2Nwfo8pkZNRgrq66H
Adding stake account 5AaobwjccyHnXhFCd24uiX6VqPjXE3Ry4o92fJjqqjAr, delegated to J3xu64PWShcMen99kU3igxtwbke2Nwfo8pkZNRgrq66H
Signature: 4xeve6gWuiffqBLAMcqa8s7dCMvBmSVdKbDu5WQhigLiXHdCjSNEwoZRexTZji786qgEjXg3nrUh4HcTt3RauZV5
$ spl-stake-pool add-validator Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ
Adding stake account 3k7Nwu9jUSc6SNG11wzufKYoZXRFgxWamheGLYWp5Rvx, delegated to EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ
Signature: 4VJYHpPmWkP99TdgYUTgLYixmhqmqsEkWtg4j7zvGZFjYbnLgryu48aV6ub8bqDyULzKckUhb6tvcmZmMX5AFf5G
```

We can see the status of a stake account using the Solana command-line utility.

```console
$ solana stake-account 5AaobwjccyHnXhFCd24uiX6VqPjXE3Ry4o92fJjqqjAr
Balance: 0.00328288 SOL
Rent Exempt Reserve: 0.00228288 SOL
Delegated Stake: 0.001 SOL
Active Stake: 0 SOL
Activating Stake: 0.001 SOL
Stake activates starting from epoch: 5
Delegated Vote Account Address: J3xu64PWShcMen99kU3igxtwbke2Nwfo8pkZNRgrq66H
Stake Authority: DS3AyFN9dF1ruNBcSeo8XXQR8UyVMhcCPcnjU5GnY18S
Withdraw Authority: DS3AyFN9dF1ruNBcSeo8XXQR8UyVMhcCPcnjU5GnY18S
```

The stake pool creates these special staking accounts with 0.001 SOL as the required
minimum delegation amount. The stake and withdraw authorities are the stake pool
withdraw authority, program addresses derived from the stake pool's address.

We can also see the status of the stake pool.

```console
$ spl-stake-pool list Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR
Stake Pool: Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR
Pool Token Mint: BoNneHKDrX9BHjjvSpPfnQyRjsnc9WFH71v8wrgCd7LB
Epoch Fee: 3/100 of epoch rewards
Withdrawal Fee: none
Stake Deposit Fee: none
SOL Deposit Fee: none
SOL Deposit Referral Fee: none
Stake Deposit Referral Fee: none
Reserve Account: EN4px2h4gFkYtsQUi4yeCYBrdRM4DoRxCVJyavMXEAm5   Available Balance: ◎0.000000000
Vote Account: EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ      Balance: ◎0.000000000 Last Update Epoch: 4
Vote Account: J3xu64PWShcMen99kU3igxtwbke2Nwfo8pkZNRgrq66H      Balance: ◎0.000000000  Last Update Epoch: 4
Vote Account: 38DYMkwYCvsj8TC6cNaEvFHHVDYeWDp1qUgMgyjNqZXk      Balance: ◎0.000000000  Last Update Epoch: 4
Total Pool Stake: ◎0.000000000
Total Pool Tokens: 0.00000000
Current Number of Validators: 3
Max Number of Validators: 1000
```

To make reading easier, the tool will not show balances that cannot be touched by
the stake pool. The stake account `5AaobwjccyHnXhFCd24uiX6VqPjXE3Ry4o92fJjqqjAr`,
delegated to `J3xu64PWShcMen99kU3igxtwbke2Nwfo8pkZNRgrq66H`, actually has a balance
of 0.00328288 SOL, but since this is the minimum required amount, it is
not shown by the CLI.

### Remove validator stake account

If the stake pool staker wants to stop delegating to a vote account, they can
totally remove the validator stake account from the stake pool.

As with adding a validator, the validator stake account must have exactly
0.00328288 SOL (0.001 SOL delegated, 0.00228288 SOL for rent exemption) to be removed.

If that is not the case, the staker must first decrease the stake to that minimum amount.
Let's assume that the validator stake account delegated to 
`J3xu64PWShcMen99kU3igxtwbke2Nwfo8pkZNRgrq66H` has a total delegated amount of
7.5 SOL. To reduce that number, the staker can run:

```console
$ spl-stake-pool decrease-validator-stake Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR J3xu64PWShcMen99kU3igxtwbke2Nwfo8pkZNRgrq66H 6.5
Signature: ZpQGwT85rJ8Y9afdkXhKo3TVv4xgTz741mmZj2vW7mihYseAkFsazWxza2y8eNGY4HDJm15c1cStwyiQzaM3RpH
```

Now, let's try to remove validator `J3xu64PWShcMen99kU3igxtwbke2Nwfo8pkZNRgrq66H`, with
stake account `5AaobwjccyHnXhFCd24uiX6VqPjXE3Ry4o92fJjqqjAr`.

```console
$ spl-stake-pool remove-validator Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR J3xu64PWShcMen99kU3igxtwbke2Nwfo8pkZNRgrq66H
Removing stake account 5AaobwjccyHnXhFCd24uiX6VqPjXE3Ry4o92fJjqqjAr, delegated to J3xu64PWShcMen99kU3igxtwbke2Nwfo8pkZNRgrq66H
Creating account to receive stake nHEEyey8KkgHuVRAUDzkH5Q4PkA4veSHuTxgG6C8L2G
Signature: 4XprnR768Ch6LUvqUVLTjMCiqdYvtjNfECh4izErqwbsASTGjUBz7NtLZHAiraTqhs7b9PoSAazetdsgXa6J4wVu
```

Unlike a normal withdrawal, the validator stake account is totally moved from
the stake pool and into a new account belonging to the administrator.

Note: since removal is only possible when the validator stake is at the minimum
amount of 0.00328288, the administrator does not get any control of user funds,
and only recovers the amount contributed during `add-validator`.

The authority for the withdrawn stake account can also be specified using the
`--new-authority` flag:

```console
$ spl-stake-pool remove-validator Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR J3xu64PWShcMen99kU3igxtwbke2Nwfo8pkZNRgrq66H --new-authority 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Signature: 5rrQ3xhDWyiPkUTAQkNAeq31n6sMf1xsg2x9hVY8Vj1NonwBnhxuTv87nADLkwC8Xzc4CGTNCTX2Vph9esWnXk2d
```

We can check the removed stake account:

```console
$ solana stake-account nHEEyey8KkgHuVRAUDzkH5Q4PkA4veSHuTxgG6C8L2G
Balance: 0.003282880 SOL
Rent Exempt Reserve: 0.00328288 SOL
Delegated Stake: 0.001000000 SOL
Active Stake: 0.001000000 SOL
Delegated Vote Account Address: J3xu64PWShcMen99kU3igxtwbke2Nwfo8pkZNRgrq66H
Stake Authority: 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Withdraw Authority: 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
```

### Rebalance the stake pool

As time goes on, users will deposit to and withdraw from all of the stake accounts
managed by the pool, and the stake pool staker may want to rebalance the stakes.

For example, let's say the staker wants the same delegation to every validator
in the pool. When they look at the state of the pool, they see:

```console
$ spl-stake-pool list Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR
Stake Pool: Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR
Pool Token Mint: BoNneHKDrX9BHjjvSpPfnQyRjsnc9WFH71v8wrgCd7LB
Epoch Fee: 3/100 of epoch rewards
Withdrawal Fee: none
Stake Deposit Fee: none
SOL Deposit Fee: none
SOL Deposit Referral Fee: none
Stake Deposit Referral Fee: none
Reserve Account: EN4px2h4gFkYtsQUi4yeCYBrdRM4DoRxCVJyavMXEAm5   Available Balance: ◎10.006848640
Vote Account: EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ      Balance: ◎100.000000000 Last Update Epoch: 4
Vote Account: J3xu64PWShcMen99kU3igxtwbke2Nwfo8pkZNRgrq66H      Balance: ◎10.000000000  Last Update Epoch: 4
Vote Account: 38DYMkwYCvsj8TC6cNaEvFHHVDYeWDp1qUgMgyjNqZXk      Balance: ◎10.000000000  Last Update Epoch: 4
Total Pool Stake: ◎130.006848640
Total Pool Tokens: 130.00684864
Current Number of Validators: 3
Max Number of Validators: 1000
```

This isn't great! The first stake account, `EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ`
has too much allocated. For their strategy, the staker wants the `100`
SOL to be distributed evenly, meaning `40` in each account. They need
to move `30` to `J3xu64PWShcMen99kU3igxtwbke2Nwfo8pkZNRgrq66H` and
`38DYMkwYCvsj8TC6cNaEvFHHVDYeWDp1qUgMgyjNqZXk`.

#### Decrease validator stake

First, they need to decrease the amount on stake account
`3k7Nwu9jUSc6SNG11wzufKYoZXRFgxWamheGLYWp5Rvx`, delegated to
`EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ`, by a total of `60` SOL.

They decrease that amount of SOL:

```sh
$ spl-stake-pool decrease-validator-stake Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ 60
Signature: ZpQGwT85rJ8Y9afdkXhKo3TVv4xgTz741mmZj2vW7mihYseAkFsazWxza2y8eNGY4HDJm15c1cStwyiQzaM3RpH
```

Internally, this instruction splits and deactivates 60 SOL from the
validator stake account `3k7Nwu9jUSc6SNG11wzufKYoZXRFgxWamheGLYWp5Rvx` into a
transient stake account, owned and managed entirely by the stake pool.

Once the stake is deactivated during the next epoch, the `update` command will
automatically merge the transient stake account into a reserve stake account,
also entirely owned and managed by the stake pool.

#### Increase validator stake

Now that the reserve stake account has enough to perform the rebalance, the staker
can increase the stake on the two other validators,
`J3xu64PWShcMen99kU3igxtwbke2Nwfo8pkZNRgrq66H` and
`38DYMkwYCvsj8TC6cNaEvFHHVDYeWDp1qUgMgyjNqZXk`.

They add 30 SOL to `J3xu64PWShcMen99kU3igxtwbke2Nwfo8pkZNRgrq66H`:

```sh
$ spl-stake-pool increase-validator-stake Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR J3xu64PWShcMen99kU3igxtwbke2Nwfo8pkZNRgrq66H 30
Signature: 3GJACzjUGLPjcd9RLUW86AfBLWKapZRkxnEMc2yHT6erYtcKBgCapzyrVH6VN8Utxj7e2mtvzcigwLm6ZafXyTMw
```

And they add 30 SOL to `38DYMkwYCvsj8TC6cNaEvFHHVDYeWDp1qUgMgyjNqZXk`:

```sh
$ spl-stake-pool increase-validator-stake Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR 38DYMkwYCvsj8TC6cNaEvFHHVDYeWDp1qUgMgyjNqZXk 30
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
Preferred Deposit Validator: 38DYMkwYCvsj8TC6cNaEvFHHVDYeWDp1qUgMgyjNqZXk
Epoch Fee: 3/100 of epoch rewards
Withdrawal Fee: none
Stake Deposit Fee: none
SOL Deposit Fee: none
SOL Deposit Referral Fee: none
Stake Deposit Referral Fee: none
Reserve Account: EN4px2h4gFkYtsQUi4yeCYBrdRM4DoRxCVJyavMXEAm5   Available Balance: ◎10.006848640
Vote Account: EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ      Balance: ◎40.000000000  Last Update Epoch: 8
Vote Account: J3xu64PWShcMen99kU3igxtwbke2Nwfo8pkZNRgrq66H      Balance: ◎40.000000000  Last Update Epoch: 8
Vote Account: 38DYMkwYCvsj8TC6cNaEvFHHVDYeWDp1qUgMgyjNqZXk      Balance: ◎40.000000000  Last Update Epoch: 8
Total Pool Stake: ◎130.006848640
Total Pool Tokens: 130.00684864
Current Number of Validators: 3
Max Number of Validators: 1000
```

Due to staking rewards that accrued during the rebalancing process, the pool may
not perfectly balanced. This is completely normal.

### Set Preferred Deposit / Withdraw Validator

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

## User Examples

### List validator stake accounts

In order to deposit into the stake pool, a user must first delegate some stake
to one of the validator stake accounts associated with the stake pool. The
command-line utility has a special instruction for finding out which vote
accounts are already associated with the stake pool.

```console
$ spl-stake-pool list Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR
Stake Pool: Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR
Pool Token Mint: BoNneHKDrX9BHjjvSpPfnQyRjsnc9WFH71v8wrgCd7LB
Preferred Deposit Validator: 38DYMkwYCvsj8TC6cNaEvFHHVDYeWDp1qUgMgyjNqZXk
Epoch Fee: 3/100 of epoch rewards
Withdrawal Fee: none
Stake Deposit Fee: none
SOL Deposit Fee: none
SOL Deposit Referral Fee: none
Stake Deposit Referral Fee: none
Reserve Account: EN4px2h4gFkYtsQUi4yeCYBrdRM4DoRxCVJyavMXEAm5   Available Balance: ◎10.006848640
Vote Account: EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ      Balance: ◎35.000000000  Last Update Epoch: 8
Vote Account: J3xu64PWShcMen99kU3igxtwbke2Nwfo8pkZNRgrq66H      Balance: ◎35.000000000  Last Update Epoch: 8
Vote Account: 38DYMkwYCvsj8TC6cNaEvFHHVDYeWDp1qUgMgyjNqZXk      Balance: ◎35.000000000  Last Update Epoch: 8
Total Pool Stake: ◎115.006848640
Total Pool Tokens: 115.00684864
Current Number of Validators: 3
Max Number of Validators: 1000
```

### Deposit SOL

Stake pools accept SOL deposits directly from a normal SOL wallet account, and
in exchange mint the appropriate amount of pool tokens.

```console
$ spl-stake-pool deposit-sol Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR 100
Using existing associated token account DgyZrAq88bnG1TNRxpgDQzWXpzEurCvfY2ukKFWBvADQ to receive stake pool tokens of mint BoNneHKDrX9BHjjvSpPfnQyRjsnc9WFH71v8wrgCd7LB, owned by 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Signature: 23CptpZaq33njCpJPAvk8XS53xXwpfqF1sGxChk3VDB5mzz7XPKQqwsreun3iwZ6b51AyHqGBaUyc6tx9fqvF9JK
```

In return, the stake pool has minted us new pool tokens, representing our share
of ownership in the pool.  We can double-check our stake pool account using the
SPL token command-line utility.

```console
$ spl-token balance BoNneHKDrX9BHjjvSpPfnQyRjsnc9WFH71v8wrgCd7LB
100.00000000
```

### Withdraw SOL

Stake pools allow SOL withdrawals directly from the reserve and into a normal
SOL wallet account, and in exchange burns the provided pool tokens.

```console
$ spl-stake-pool withdraw-sol Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR 7VXPpSxneL6JLj18Naw2gkukXtjBZfbmPh18cnoUCMD8 2
Signature: 4bqZKUUrjVspqTGqGqX4zxnHnJB67WbeukKUZRmxJ2yFmr275CtHPjZNzQJD9Pe7Q6mSxnUpcVv9FUdAbGP9RyBc
```

The stake pool has burned 2 pool tokens, and in return, sent SOL to
`7VXPpSxneL6JLj18Naw2gkukXtjBZfbmPh18cnoUCMD8`.

You can check that the pool tokens have been burned:

```console
$ spl-token balance BoNneHKDrX9BHjjvSpPfnQyRjsnc9WFH71v8wrgCd7LB
98.00000000
```

And you can check that the recipient has been credited:

```console
$ solana balance 7VXPpSxneL6JLj18Naw2gkukXtjBZfbmPh18cnoUCMD8
2 SOL
```

### Deposit stake

Stake pools also accept deposits from active stake accounts, so we must first
create stake accounts and delegate them to one of the validators managed by the
stake pool. Using the `list` command from the previous section, we see that
`38DYMkwYCvsj8TC6cNaEvFHHVDYeWDp1qUgMgyjNqZXk` is a valid vote account, so let's
create a stake account and delegate our stake there.

```console
$ solana-keygen new --no-passphrase -o stake-account.json
Generating a new keypair
Wrote new keypair to stake-account.json
============================================================================
pubkey: 97wBBiLVA7fUViEew8yV8R6tTdKithZDVz8LHLfF9sTJ
============================================================================
Save this seed phrase to recover your new keypair:
++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
============================================================================
$ solana create-stake-account stake-account.json 10
Signature: 5Y9r6MNoqJzVX8TWryAJbdp8i2DvintfxbYWoY6VcLEPgphK2tdydhtJTd3o3dF7QdM2Pg8sBFDZuyNcMag3nPvj
$ solana delegate-stake 97wBBiLVA7fUViEew8yV8R6tTdKithZDVz8LHLfF9sTJ 38DYMkwYCvsj8TC6cNaEvFHHVDYeWDp1qUgMgyjNqZXk
Signature: 2cDjHXSHjuadGQf1NQpPi43A8R19aCifsY16yTcictKPHcSAXN5TvXZ58nDJwkYs12tuZfTh5WVgAMSvptfrKdPP
```

Two epochs later, when the stake is fully active and has received one epoch of
rewards, we can deposit the stake into the stake pool.

```console
$ spl-stake-pool deposit-stake Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR 97wBBiLVA7fUViEew8yV8R6tTdKithZDVz8LHLfF9sTJ
Depositing stake 97wBBiLVA7fUViEew8yV8R6tTdKithZDVz8LHLfF9sTJ into stake pool account F8e8Ympp4MkDSPZdvRxdQUZXRkMBDdyqgHa363GShAPt
Using existing associated token account DgyZrAq88bnG1TNRxpgDQzWXpzEurCvfY2ukKFWBvADQ to receive stake pool tokens of mint BoNneHKDrX9BHjjvSpPfnQyRjsnc9WFH71v8wrgCd7LB, owned by 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Signature: 45x2UtA1b49eBPtRHdkvA3k8JneZzfwjptNN1kKQZaPABYiJ4hSA8qwi7qLNN5b3Fr4Z6vXhJprrTCpkk3f8UqgD
```

The CLI will default to using the fee payer's
[Associated Token Account](associated-token-account.md) for stake pool tokens
and the withdraw authority on the deposited stake account.

Alternatively, you can create an SPL token account yourself and pass it as the
`token-receiver` for the command, and specify the withdraw authority on the
stake account using the `withdraw-authority` flag.

```console
$ spl-stake-pool deposit-stake Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR 97wBBiLVA7fUViEew8yV8R6tTdKithZDVz8LHLfF9sTJ --token-receiver 34XMHa3JUPv46ftU4dGHvemZ9oKVjnciRePYMcX3rjEF --withdraw-authority authority.json
Depositing stake 97wBBiLVA7fUViEew8yV8R6tTdKithZDVz8LHLfF9sTJ into stake pool account F8e8Ympp4MkDSPZdvRxdQUZXRkMBDdyqgHa363GShAPt
Signature: 4AESGZzqBVfj5xQnMiPWAwzJnAtQDRFK1Ha6jqKKTs46Zm5fw3LqgU1mRAT6CKTywVfFMHZCLm1hcQNScSMwVvjQ
```

In return, the stake pool has minted us new pool tokens, representing our share
of ownership in the pool.  We can double-check our stake pool account using the
SPL token command-line utility.

```console
$ spl-token balance BoNneHKDrX9BHjjvSpPfnQyRjsnc9WFH71v8wrgCd7LB
10.00000000
```

#### Note on stake deposit fee

Stake pools have separate fees for stake and SOL, so the total fee from depositing
a stake account is calculated from the rent-exempt reserve as SOL, and the delegation
as stake.

For example, if a stake pool has a stake deposit fee of 1%, and a SOL deposit fee
of 5%, and you deposit a stake account with 10 SOL in stake, and .00228288 SOL
in rent-exemption, the total fee charged is:

```
total_fee = stake_delegation * stake_deposit_fee + rent_exemption * sol_deposit_fee
total_fee = 10 * 1% + .00228288 * 5%
total_fee = 0.100114144
```

### Update

Every epoch, the network pays out rewards to stake accounts managed by the stake
pool, increasing the value of pool tokens minted on deposit.
In order to calculate the proper value of these stake pool tokens, we must update
the total value managed by the stake pool every epoch.

```console
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

### Withdraw stake

Whenever the user wants to recover their SOL plus accrued rewards, they can provide their
pool tokens in exchange for an activated stake account.

Let's withdraw active staked SOL in exchange for 5 pool tokens.

```console
$ spl-stake-pool withdraw-stake Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR 5
Withdrawing ◎5.000000000, or 5 pool tokens, from stake account 3k7Nwu9jUSc6SNG11wzufKYoZXRFgxWamheGLYWp5Rvx, delegated to EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ
Creating account to receive stake 5GuAyPAt6577HoGhSVRNBv6aHohVtjQ8q7q5i3X1p4tB
Signature: 5fzaKt5MU8bLjJRgNZyEktKsgweSQzFRpubCGKPeuk9shNQb4CtTkbgZ2X5MmC1VRDZ3YcCTPdtL9sFpXYfoqaeV
```

The stake pool took 5 pool tokens, and in exchange the user received a fully
active stake account, delegated to `EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ`.
Let's double-check the status of the stake account:

```console
$ solana stake-account 5GuAyPAt6577HoGhSVRNBv6aHohVtjQ8q7q5i3X1p4tB
Balance: 5.00228288 SOL
Rent Exempt Reserve: 0.00228288 SOL
Delegated Stake: 5 SOL
Active Stake: 5 SOL
Delegated Vote Account Address: EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ
Stake Authority: 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
Withdraw Authority: 4SnSuUtJGKvk2GYpBwmEsWG53zTurVM8yXGsoiZQyMJn
```

Note: this operation cost the user some funds, as they needed to create a new
stake account with the minimum rent exemption in order to receive the funds. This
allows the user to withdraw any amount of stake pool tokens, even if it is not
enough to cover the stake account rent-exemption.

Alternatively, the user can specify an existing uninitialized stake account to
receive their stake using the `--stake-receiver` parameter.

```console
$ spl-stake-pool withdraw-stake Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR  --amount 0.02 --vote-account EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ --stake-receiver CZF2z3JJoDmJRcVjtsrz1BKUUGNL3VPW5FPFqge1bzmQ
Withdrawing ◎5.000000000, or 5 pool tokens, from stake account 3k7Nwu9jUSc6SNG11wzufKYoZXRFgxWamheGLYWp5Rvx, delegated to EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ
Signature: 2xBPVPJ749AE4hHNCNYdjuHv1EdMvxm9uvvraWfTA7Urrvecwh9w64URCyLLroLQ2RKDGE2QELM2ZHd8qRkjavJM
```

By default, the withdraw command uses the `token-owner`'s associated token account to
source the pool tokens. It's possible to specify the SPL token account using
the `--pool-account` flag.

```console
$ spl-stake-pool withdraw-stake Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR 5 --pool-account 34XMHa3JUPv46ftU4dGHvemZ9oKVjnciRePYMcX3rjEF
Withdrawing ◎5.000000000, or 5 pool tokens, from stake account 3k7Nwu9jUSc6SNG11wzufKYoZXRFgxWamheGLYWp5Rvx, delegated to EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ
Creating account to receive stake CZF2z3JJoDmJRcVjtsrz1BKUUGNL3VPW5FPFqge1bzmQ
Signature: 2xBPVPJ749AE4hHNCNYdjuHv1EdMvxm9uvvraWfTA7Urrvecwh9w64URCyLLroLQ2RKDGE2QELM2ZHd8qRkjavJM
```

By default, the withdraw command will withdraw from the largest validator stake
accounts in the pool. It's also possible to specify a specific vote account for
the withdraw using the `--vote-account` flag.

```console
$ spl-stake-pool withdraw-stake Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR  --amount 5 --vote-account EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ
Withdrawing ◎5.000000000, or 5 pool tokens, from stake account 3k7Nwu9jUSc6SNG11wzufKYoZXRFgxWamheGLYWp5Rvx, delegated to EhRbKi4Vhm1oUCGWHiLEMYZqDrHwEd7Jgzgi26QJKvfQ
Creating account to receive stake CZF2z3JJoDmJRcVjtsrz1BKUUGNL3VPW5FPFqge1bzmQ
Signature: 2xBPVPJ749AE4hHNCNYdjuHv1EdMvxm9uvvraWfTA7Urrvecwh9w64URCyLLroLQ2RKDGE2QELM2ZHd8qRkjavJM
```

Note that the associated validator stake account must have enough lamports to
satisfy the pool token amount requested.

#### Special case: exiting pool with a delinquent staker

With the reserve stake, it's possible for a delinquent or malicious staker to
move all stake into the reserve through `decrease-validator-stake`, so the
pool tokens will not gain rewards, and the stake pool users will not
be able to withdraw their funds.

To get around this case, it is also possible to withdraw from the stake pool's
reserve, but only if all of the validator stake accounts are at the minimum amount of
`0.001 SOL + stake account rent exemption`.

```console
$ spl-stake-pool withdraw-stake Zg5YBPAk8RqBR9kaLLSoN5C8Uv7nErBz1WC63HTsCPR 5 --use-reserve
Withdrawing ◎5.000000000, or 5 pool tokens, from stake account J5XB7mWpeaUZxZ6ogXT57qSCobczx27vLZYSgfSbZoBB
Creating account to receive stake 51XdXiBSsVzeuY79xJwWAGZgeKzzgFKWajkwvWyrRiNE
Signature: yQH9n7Go6iCMEYXqWef38ZYBPwXDmbwKAJFJ4EHD6TusBpusKsfNuT3TV9TL8FmxR2N9ExZTZwbD9Njc3rMvUcf
```
