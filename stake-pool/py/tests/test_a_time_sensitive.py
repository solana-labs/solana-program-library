"""Time sensitive test, so run it first out of the bunch."""
import asyncio
import pytest
from solana.rpc.commitment import Confirmed
from spl.token.instructions import get_associated_token_address

from stake.constants import STAKE_LEN
from stake_pool.actions import deposit_sol, decrease_validator_stake, increase_validator_stake, update_stake_pool
from stake_pool.constants import MINIMUM_ACTIVE_STAKE
from stake_pool.state import StakePool, ValidatorList


@pytest.mark.asyncio
async def test_increase_decrease_this_is_very_slow(async_client, validators, payer, stake_pool_addresses, waiter):
    (stake_pool_address, validator_list_address, _) = stake_pool_addresses

    resp = await async_client.get_minimum_balance_for_rent_exemption(STAKE_LEN)
    stake_rent_exemption = resp.value
    minimum_amount = MINIMUM_ACTIVE_STAKE + stake_rent_exemption
    increase_amount = MINIMUM_ACTIVE_STAKE * 4
    decrease_amount = increase_amount // 2
    deposit_amount = (increase_amount + stake_rent_exemption) * len(validators)

    resp = await async_client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    stake_pool = StakePool.decode(data)
    token_account = get_associated_token_address(payer.pubkey(), stake_pool.pool_mint)
    await deposit_sol(async_client, payer, stake_pool_address, token_account, deposit_amount)

    # increase to all
    futures = [
        increase_validator_stake(async_client, payer, payer, stake_pool_address, validator, increase_amount // 2)
        for validator in validators
    ]
    await asyncio.gather(*futures)

    # validate the increase is now on the transient account
    resp = await async_client.get_account_info(validator_list_address, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    validator_list = ValidatorList.decode(data)
    for validator in validator_list.validators:
        assert validator.transient_stake_lamports == increase_amount // 2 + stake_rent_exemption
        assert validator.active_stake_lamports == minimum_amount

    # increase the same amount to test the increase additional instruction
    futures = [
        increase_validator_stake(async_client, payer, payer, stake_pool_address, validator, increase_amount // 2,
                                 ephemeral_stake_seed=0)
        for validator in validators
    ]
    await asyncio.gather(*futures)

    # validate the additional increase is now on the transient account
    resp = await async_client.get_account_info(validator_list_address, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    validator_list = ValidatorList.decode(data)
    for validator in validator_list.validators:
        assert validator.transient_stake_lamports == increase_amount + stake_rent_exemption * 2
        assert validator.active_stake_lamports == minimum_amount

    print("Waiting for epoch to roll over")
    await waiter.wait_for_next_epoch(async_client)
    await update_stake_pool(async_client, payer, stake_pool_address)

    resp = await async_client.get_account_info(validator_list_address, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    validator_list = ValidatorList.decode(data)
    for validator in validator_list.validators:
        assert validator.last_update_epoch != 0
        assert validator.transient_stake_lamports == 0
        assert validator.active_stake_lamports == increase_amount + minimum_amount + stake_rent_exemption

    # decrease from all
    futures = [
        decrease_validator_stake(async_client, payer, payer, stake_pool_address, validator, decrease_amount)
        for validator in validators
    ]
    await asyncio.gather(*futures)

    # validate the decrease is now on the transient account
    resp = await async_client.get_account_info(validator_list_address, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    validator_list = ValidatorList.decode(data)
    for validator in validator_list.validators:
        assert validator.transient_stake_lamports == decrease_amount + stake_rent_exemption
        assert validator.active_stake_lamports == increase_amount - decrease_amount + minimum_amount + \
            stake_rent_exemption

    # DO NOT test decrese additional instruction as it is confirmed NOT to be working as advertised

    # roll over one epoch and verify we have the balances that we expect
    expected_active_stake_lamports = increase_amount - decrease_amount + minimum_amount + stake_rent_exemption

    print("Waiting for epoch to roll over")
    await waiter.wait_for_next_epoch(async_client)
    await update_stake_pool(async_client, payer, stake_pool_address)

    resp = await async_client.get_account_info(validator_list_address, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    validator_list = ValidatorList.decode(data)
    for validator in validator_list.validators:
        assert validator.transient_stake_lamports == 0
        assert validator.active_stake_lamports == expected_active_stake_lamports
