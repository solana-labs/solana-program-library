import pytest
import asyncio
from solana.publickey import PublicKey
from solana.rpc.commitment import Confirmed

from vote.constants import VOTE_PROGRAM_ID
from stake_pool.state import Fee, ValidatorList
from stake_pool.actions import create_all, add_validator_to_pool, update_stake_pool


@pytest.mark.asyncio
async def test_add_remove_validators(async_client, validators, payer):
    fee = Fee(numerator=1, denominator=1000)
    referral_fee = 20
    (stake_pool, validator_list_address) = await create_all(async_client, payer, fee, referral_fee)
    for validator in validators:
        resp = await async_client.get_account_info(validator, commitment=Confirmed)
        assert PublicKey(resp['result']['value']['owner']) == VOTE_PROGRAM_ID
        await add_validator_to_pool(async_client, payer, stake_pool, validator)

    resp = await async_client.get_account_info(validator_list_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    validator_list = ValidatorList.decode(data[0], data[1])
    print("Waiting for epoch to roll over, 12 seconds")
    await asyncio.sleep(12.0)

    await update_stake_pool(async_client, payer, stake_pool)

    resp = await async_client.get_account_info(validator_list_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    validator_list = ValidatorList.decode(data[0], data[1])
    for validator_info in validator_list.validators:
        assert validator_info.last_update_epoch != 0
