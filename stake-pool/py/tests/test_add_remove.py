import asyncio
import pytest
from solana.rpc.commitment import Confirmed

from stake.constants import STAKE_LEN
from stake_pool.actions import remove_validator_from_pool
from stake_pool.constants import MINIMUM_ACTIVE_STAKE
from stake_pool.state import ValidatorList, StakeStatus


@pytest.mark.asyncio
async def test_add_remove_validators(async_client, validators, payer, stake_pool_addresses):
    (stake_pool_address, validator_list_address, _) = stake_pool_addresses
    resp = await async_client.get_account_info(validator_list_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    validator_list = ValidatorList.decode(data[0], data[1])
    assert len(validator_list.validators) == len(validators)
    resp = await async_client.get_minimum_balance_for_rent_exemption(STAKE_LEN)
    stake_rent_exemption = resp['result']
    futures = []
    for validator_info in validator_list.validators:
        assert validator_info.vote_account_address in validators
        assert validator_info.active_stake_lamports == stake_rent_exemption + MINIMUM_ACTIVE_STAKE
        assert validator_info.transient_stake_lamports == 0
        assert validator_info.status == StakeStatus.ACTIVE
        futures.append(
            remove_validator_from_pool(async_client, payer, stake_pool_address, validator_info.vote_account_address)
        )
    await asyncio.gather(*futures)

    resp = await async_client.get_account_info(validator_list_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    validator_list = ValidatorList.decode(data[0], data[1])
    for validator_info in validator_list.validators:
        assert validator_info.status == StakeStatus.DEACTIVATING_VALIDATOR
