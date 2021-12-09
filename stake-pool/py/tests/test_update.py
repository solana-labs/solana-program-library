import pytest
import asyncio
from solana.rpc.commitment import Confirmed

from stake_pool.state import ValidatorList
from stake_pool.actions import update_stake_pool


@pytest.mark.asyncio
async def test_update_this_is_very_slow(async_client, validators, payer, stake_pool_addresses):
    (stake_pool_address, validator_list_address) = stake_pool_addresses
    print("Waiting for epoch to roll over, 12 seconds")
    await asyncio.sleep(12.0)

    await update_stake_pool(async_client, payer, stake_pool_address)

    resp = await async_client.get_account_info(validator_list_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    validator_list = ValidatorList.decode(data[0], data[1])
    for validator_info in validator_list.validators:
        assert validator_info.last_update_epoch != 0
