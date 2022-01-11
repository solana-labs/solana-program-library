---
title: Operation
---

Stake pools are an alternative method of earning staking rewards. This on-chain
program pools together SOL to be staked by a staker, allowing SOL holders to
stake and earn rewards without managing stakes.

## Staking

SOL token holders can earn rewards and help secure the network by staking tokens
to one or more validators. Rewards for staked tokens are based on the current
inflation rate, total number of SOL staked on the network, and an individual
validator’s uptime and commission (fee).

Additional information regarding staking and stake programming is available at:

- https://solana.com/staking
- https://docs.solana.com/staking/stake-programming

## Background

Solana's programming model and the definitions of the Solana terms used in this
document are available at:

- https://docs.solana.com/apps
- https://docs.solana.com/terminology

## Motivation

This document is intended for the main actors of the stake pool system:

* manager: creates and manages the stake pool, earns fees, can update the fee, staker, and manager
* staker: adds and removes validators to the pool, rebalances stake among validators
* user: provides staked SOL into an existing stake pool

In its current iteration, the stake pool accepts active stakes or SOL, so
deposits may come from either an active stake or SOL wallet. Withdrawals
can return a fully active stake account from one of the stake pool's accounts,
or SOL from the reserve.

This means that stake pool managers and stakers must be comfortable with
creating and delegating stakes, which are more advanced operations than sending and
receiving SPL tokens and SOL. Additional information on stake operations are
available at:

- https://docs.solana.com/cli/delegate-stake
- https://docs.solana.com/cli/manage-stake-accounts

To reach a wider audience of users, stake pool managers are encouraged
to provide a market for their pool's tokens, through an AMM
like [Token Swap](../token-swap.md).

Alternatively, stake pool managers can partner with wallet and stake account
providers for direct SOL deposits.

## Operation

A stake pool manager creates a stake pool, and the staker includes validators that will
receive delegations from the pool by adding "validator stake accounts" to the pool
using the `add-validator` instruction. In this command, the stake pool creates
a new stake account and delegates it to the desired validator.

At this point, users can participate with deposits. They can directly deposit
SOL into the stake pool using the `deposit-sol` instruction. Within this instruction,
the stake pool will move SOL into the pool's reserve account, to be redistributed
by the staker.

Alternatively, users can deposit a stake account into the pool.  To do this,
they must delegate a stake account to the one of the validators in the stake pool.
If the stake pool has a preferred deposit validator, the user must delegate their
stake to that validator's vote account.

Once the stake becomes active, which happens at the following epoch boundary
(maximum 2 days), the user can deposit their stake into the pool using the
`deposit-stake` instruction.

In exchange for their deposit (SOL or stake), the user receives SPL tokens
representing their fractional ownership in pool. A percentage of the rewards
earned by the pool goes to the pool manager as an epoch fee.

Over time, as the stakes in the pool accrue rewards, the user's fractional
ownership will be worth more than their initial deposit.

Whenever they wish to exit the pool, the user may use the `withdraw-sol` instruction
to receive SOL from the stake pool's reserve in exchange for stake pool tokens.
Note that this operation will fail if there is not enough SOL in the stake pool's
reserve, which is normal if the stake pool manager stakes all of the SOL in the pool.

Alternatively, they can use the `withdraw-stake` instruction to withdraw an
activated stake account in exchange for their SPL pool tokens. The user will get
back a SOL stake account immediately. The ability to withdraw stake is always
possible, under all circumstances.

Note: when withdrawing stake, if the user wants to withdraw the SOL in the stake
account, they must first deactivate the stake account and wait until the next
epoch boundary (maximum 2 days).  Once the stake is inactive, they can freely
withdraw the SOL.

The stake pool staker can add and remove validators, or rebalance the pool by
decreasing the stake on a validator, waiting an epoch to move it into the stake
pool's reserve account, then increasing the stake on another validator.

The staker operation to add a new validator requires 0.00328288 SOL to create
the stake account on a validator, so the stake pool staker will need liquidity
on hand to fully manage the pool stakes.  The SOL used to add a new validator
is recovered when removing the validator.

### Funding restrictions

To give the manager more control over funds entering the pool, stake pools allow
deposit and withdrawal restrictions on SOL and stakes through three different
"funding authorities":

* SOL deposit
* Stake deposit
* SOL withdrawal

If the field is set, that authority must sign the associated instruction.

For example, if the manager sets a stake deposit authority, then that address
must sign every stake deposit instruction.

This can also be useful in a few situations:

* Control who deposits into the stake pool
* Prohibit a form of deposit. For example, the manager only wishes to have SOL
  deposits, so they set a stake deposit authority, making it only possible to
  deposit a stake account if that authority signs the transaction.
* Maintenance mode. If the pool needs time to reset fees or otherwise, the
  manager can temporarily restrict new deposits by setting deposit authorities.

Note: in order to keep user funds safe, stake withdrawals are always permitted.

## Safety of Funds

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

## Appendix

### Active stakes

As mentioned earlier, the stake pool works with active stakes to
maintains fungibility of stake pool tokens. Fully activated stakes
are not equivalent to inactive, activating, or deactivating stakes due to the
time cost of staking.

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

### Validator list account

Every stake pool contains two data accounts: the stake pool and the validator list.

The stake pool contains overall information about the pool, including fees,
pool token mint, amount under management, etc.

The validator list contains specific information about each of the validator
stake accounts in the pool. This information includes the amount of SOL staked on
the validator by the pool, and the amount of SOL being activated / deactivated
on the validator.

Every stake pool must have its own validator list account, otherwise it will
fail on initialization.

### Transaction sizes

The Solana transaction processor has two important limitations:

* size of the overall transaction, limited to roughly 1 MTU / packet
* computation budget per instruction

A stake pool may manage hundreds of staking accounts, so it is impossible to
update the total value of the stake pool in one instruction. Thankfully, the
command-line utility breaks up transactions to avoid this issue for large pools.
