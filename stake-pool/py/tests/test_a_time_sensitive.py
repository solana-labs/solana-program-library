"""Time sensitive test, so run it first out of the bunch."""
import asyncio
import pytest
from solana.rpc.commitment import Confirmed
from spl.token.instructions import get_associated_token_address

from stake.constants import STAKE_LEN
from stake_pool.state import StakePool, ValidatorList
from stake_pool.actions import deposit_sol, decrease_validator_stake, increase_validator_stake, update_stake_pool


@pytest.mark.asyncio
async def test_increase_decrease_this_is_very_slow(async_client, validators, payer, stake_pool_addresses, waiter):
    (stake_pool_address, validator_list_address) = stake_pool_addresses
    resp = await async_client.get_minimum_balance_for_rent_exemption(STAKE_LEN)
    stake_rent_exemption = resp['result']
    increase_amount = 100_000_000
    decrease_amount = increase_amount // 2
    deposit_amount = (increase_amount + stake_rent_exemption) * len(validators)

    resp = await async_client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    stake_pool = StakePool.decode(data[0], data[1])
    token_account = get_associated_token_address(payer.public_key, stake_pool.pool_mint)
    await deposit_sol(async_client, payer, stake_pool_address, token_account, deposit_amount)

    # increase to all
    futures = [
        increase_validator_stake(async_client, payer, payer, stake_pool_address, validator, increase_amount)
        for validator in validators
    ]
    await asyncio.gather(*futures)

    resp = await async_client.get_account_info(validator_list_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    validator_list = ValidatorList.decode(data[0], data[1])
    for validator in validator_list.validators:
        assert validator.transient_stake_lamports == increase_amount + stake_rent_exemption
        assert validator.active_stake_lamports == 0

    print("Waiting for epoch to roll over")
    await waiter.wait_for_next_epoch(async_client)
    await update_stake_pool(async_client, payer, stake_pool_address)

    resp = await async_client.get_account_info(validator_list_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    validator_list = ValidatorList.decode(data[0], data[1])
    for validator in validator_list.validators:
        assert validator.last_update_epoch != 0
        assert validator.transient_stake_lamports == 0
        assert validator.active_stake_lamports == increase_amount  # rent exemption brought back to reserve

    # decrease from all
    futures = [
        decrease_validator_stake(async_client, payer, payer, stake_pool_address, validator, decrease_amount)
        for validator in validators
    ]
    await asyncio.gather(*futures)

    resp = await async_client.get_account_info(validator_list_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    validator_list = ValidatorList.decode(data[0], data[1])
    for validator in validator_list.validators:
        assert validator.transient_stake_lamports == decrease_amount
        assert validator.active_stake_lamports == increase_amount - decrease_amount

    print("Waiting for epoch to roll over")
    await waiter.wait_for_next_epoch(async_client)
    await update_stake_pool(async_client, payer, stake_pool_address)

    resp = await async_client.get_account_info(validator_list_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    validator_list = ValidatorList.decode(data[0], data[1])
    for validator in validator_list.validators:
        assert validator.transient_stake_lamports == 0
        assert validator.active_stake_lamports == increase_amount - decrease_amount
