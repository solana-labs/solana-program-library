---
title: Single Validator Stake Pool Manager Program
---

A work-in-progress program for permissionless management of stake pools that are
only delegated to one validator.

## Operation

The single-validator manager program defines a canonical stake pool for any
validator on the network, which can be created by anyone.

The program manages the "delegation strategy" of the pool, which is to maximize
the amount of stake on one validator, while maintaining a small amount of SOL
in the reserve for small stakers.

### Create Pool

The `CreateStakePool` instruction allows anyone to create the canonical stake
pool for a validator, at the cost of ~0.0124 SOL for account rent-exemption of
the stake pool, validator list, reserve, mint, and fee account.

After the pool is created, the reserve requires at least stake rent-exemption plus
the minimum delegation amount to actually add the validator to the pool, which
costs 1.0028288 SOL. The user receives pool tokens for this deposit.

After the SOL deposit, the `AddValidatorToPool` instruction on the pool management
program allows anyone to add the stake account for that validator.

Once the added stake account is active, anyone may deposit stake accounts.

### Depositing and withdrawing stake

If stakers deposit or withdraw stake accounts from the pool, everything works
seamlessly. Stakers receive pool tokens for their deposited stake account, and
may use the pool tokens to withdraw a stake account.

Activated stake accounts on the same validator are all fungible, so there's no
other economics to worry about.

### Depositing and withdrawing SOL

As mentioned in the fees section, only SOL deposits have a fee, and everything
else can be done freely.

To maximize returns for pool depositors, the program allows anyone to maximally
deploy the reserve to the validator, with important limits.

Single-validator stake pools are an option for small stakers to earn inflation
rewards by staking their SOL, but if stakers have less than the minimum delegation
amount, they cannot withdraw stake accounts, so they need a way to withdraw SOL
from the reserve.

To give a way out for these users, the reserve aims to maintain the minimum delegation
amount plus stake rent-exemption, or 1.0028288 SOL. If the reserve has less than
this amount, any user can call `DecreaseValidatorStake` to decrease the minimum
delegation amount plus rent-exemption.

On the other hand, if the reserve receives enough SOL deposits equalling at least
two times this amount, then anyone can call `IncreaseValidatorStake` on the manager
program to activate everything in the reserve, minus the minimum delegation plus
rent-exemption.

### Griefing

The permissionless increase instruction allows an attack to make the pool less
performant.

For example, at the start of the epoch, a malicious user can deposit enough SOL to get
just above the limit for increasing, and immediate run the increase. This prevents
the activation of subsequent SOL deposits until the next epoch.

However, this attack only works for one epoch, since at the start of the next
epoch, anyone can activate all of the remaining stake. For this reason, SOL
depositors are charged 2 epochs of rewards in fees. This inflates the value of
pool tokens, covering for any economic attacks.

With the permissionless decrease instruction, a similar attack is possible.

For example, if the reserve is at its target amount of 1.0028288 SOL, an attacker
can withdraw 1 lamport from the reserve, then decrease another 1.0028288 SOL.

Question: should the threshold for allowing decreases be 1 / 2 minimum delegation
in the reserve? That way, the previous attack is more costly, but it means delegators
with more than 1 / 2 minimum delegation need to wait longer to get out of the pool.
That might be an acceptable tradeoff.

## Fees

Pools are configured with a SOL deposit fee of 8 basis points, which corresponds
to the rewards received over two epochs. This way, no user can deposit SOL and
immediately withdraw a stake account at the end of an epoch to steal rewards
from the pool.

All other fees are 0 so that pools mimic the reward structure of native staking.

The program also exposes a permissionless `BurnFees` instruction to allow anyone
to burn the pool tokens in the fee account, which increases the value of all
other pool tokens.

## Token Metadata

Since each pool is guaranteed to only include one validator, the validator operator
should be allowed to upload whatever token metadata they wish. The `CreateTokenMetadata`
and `UpdateTokenMetadata` instructions allow the authorized voter on the validator
to work with the token metadata.

Question: should the information be pulled from the validator-info? Or should
some other authority than the authorized voter do it?

## Existing pools

Since the program is completely permissionless and stateless, existing stake pools
may transition the pool to program management by setting the manager and staker
to the correct program-derived address.

The management program exposes more instructions to make sure that all single-validator
pools are equal:

* `ResetFees`: put all the fees to the amounts described in the fees section
* `RemoveFundingAuthorities`: open the pool to all deposits and withdrawals of SOL and stake
* `RemoveValidatorFromPool`: get rid of any other validator in the pool
