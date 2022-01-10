---
title: Fees
---

Operators of stake pools should take time to understand the purpose of each fee
and think about them carefully to ensure that the pool cannot be abused.

There are five different sources of fees.

### Epoch Fee

Every epoch (roughly 2 days), the stake accounts in the pool earn 
inflation rewards, so the stake pool mints pool tokens into the manager's fee
account as a proportion of the earned rewards.

For example, if the pool earns 10 SOL in rewards, and the fee is set to 2%, the
manager will earn pool tokens worth 0.2 SOL.

Note that the epoch fee is charged after normal validator
commissions are assessed. For example, if a validator charges 8% commission,
and the stake pool charges 2%, and a stake in the pool earns 100 SOL pre-commission,
then that stake will actually enrich the pool by 90.16 SOL. The total rewards
on that validator will be reduced by ~9.84%.

### SOL Withdraw Fee

Sends a proportion of the desired withdrawal amount to the manager.

For example, if a user wishes to withdraw 100 pool tokens, and the fee is set
to 3%, 3 pool tokens go to the manager, and the remaining 97 tokens are converted
to SOL and sent to the user.

### Stake Withdraw Fee

Sends a proportion of the desired withdrawal amount to the manager before
creating a new stake for the user.

For example, if a user wishes to withdraw 100 pool tokens, and the fee is set
to 0.5%, 0.5 pool tokens go to the manager, and the remaining 99.5 tokens are
converted to SOL then sent to the user as an activated stake account.

### SOL Deposit Fee

Converts the entire SOL deposit into pool tokens, then sends a proportion of
the pool tokens to the manager, and the rest to the user.

For example, if a user deposits 100 SOL, which converts to 90 pool tokens,
and the fee is 1%, then the user receives 89.1 pool tokens, and the manager receives
0.9 pool tokens.

### Stake Deposit Fee

Converts the stake account's delegation plus rent-exemption to pool tokens,
sends a proportion of those to the manager, and the rest to the user. The rent-
exempt portion of the stake account is converted at the SOL deposit rate, and
the stake is converted at the stake deposit rate.

For example, let's say the pool token to SOL exchange rate is 1:1, the SOL deposit rate
is 10%, and the stake deposit rate is 5%. If a user deposits a stake account with
100 SOL staked and 0.00228288 SOL for the rent exemption. The fee from the stake
is worth 5 pool tokens, and the fee from the rent exemption is worth 0.000228288
pool tokens, so the user receives 95.002054592 pool tokens, and the manager
receives 5.000228288 pool tokens.

## Referral Fees

For partner applications, the manager may set a referral fee on deposits.
During SOL or stake deposits, the stake pool redistributes a percentage of
the pool token fees to another address as a referral fee.

This option is particularly attractive for wallet providers. When a wallet
integrates a stake pool, the wallet developer will have the option to earn
additional tokens anytime a user deposits into the stake pool. Stake pool
managers can use this feature to create strategic partnerships and entice
greater adoption of stake pools!

## Best Practices

Outside of monetization, fees are a crucial tool for avoiding economic attacks
on the stake pool and keeping it running. For this reason, the stake pool CLI
will prevent managers from creating a pool with no fees, unless they also provide
the `--yolo` flag.

### Epoch

If a stake pool with 1000 validators runs a rebalancing script every epoch, the
staker needs to send roughly 200 transactions to update the stake pool balances,
followed by up to 1000 transactions to increase or decrease the stake on every
validator.

At the time of writing, the current transaction fee is 5,000 lamports per signature,
so the minimum cost for these 1,200 transactions is 6,000,000 lamports, or 0.006 SOL.
For the stake pool manager to break even, they must earn 0.006 SOL per epoch in
fees.

For example, let's say we have a stake pool with 10,000 SOL staked, whose stakes
are earning 6% APY / ~3.3 basis points per epoch, yielding roughly 3.3 SOL per epoch
in rewards.  The minimal break-even epoch fee for this stake pool is 0.18%.

### Stake Deposit / Withdraw

If a stake pool has no deposit or withdraw fees, a malicious pool token holder
can easily leech value from the stake pool.

In the simplest attack, right before the end of every epoch, the malicious pool
token holder finds the highest performing validator in the pool for that epoch,
withdraws an active stake worth all of their pool tokens, waits until the epoch
rolls over, earns the maximum stake rewards, and then deposits right back into
the stake pool.

Practically speaking, the malicious depositor is always delegated to the best
performing validator in the stake pool, without ever actually committing a stake
to that validator. On top of that, the malicious depositor goes around any
epoch fees.

To render this attack unviable, the stake pool manager can set a deposit or withdraw
fee. If the stake pool has an overall performance of 6% APY / ~3.3 basis points
per epoch, and the best validator has a performance of 6.15% APY / ~3.37 basis
points per epoch, then the minimum stake deposit / withdrawal fee would be 
0.07 basis points.

For total safety, in case a delinquent validator in the pool brings down
performance, a manager may want to go much higher.

### SOL Deposit / Withdrawal

If a stake pool has 0 SOL deposit / withdrawal fee, then a malicious SOL holder
can perform a similar attack to extract even more value from the pool.

If they deposit SOL into a stake pool, withdraw a stake account on the top
validator in the pool, wait until the epoch rolls over, then deposit that stake
back into the pool, then withdraw SOL, they have essentially earned free instant
rewards without any time commitment of their SOL.  In the meantime, the stake
pool performance has decreased because the deposited liquid SOL does not earn
rewards.

For example, if the best performing validator in the stake pool earns 6.15%
APY / ~3.37 basis points per epoch, then the minimum SOL deposit / withdrawal
fee should be 3.37 basis points.

## Final thoughts

The attacks outlined in the previous sections are the simplest attacks that anyone
can easily perform with a couple of scripts running a few times per epoch. There are
likely more complex attacks possible for zero or very low fee stake pools, so be
sure to protect your depositors with fees!
