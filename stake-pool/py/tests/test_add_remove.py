import pytest
from solana.publickey import PublicKey
from solana.rpc.commitment import Confirmed

from vote.constants import VOTE_PROGRAM_ID
from stake_pool.state import Fee, ValidatorList, StakeStatus
from stake_pool.actions import create_all, add_validator_to_pool, remove_validator_from_pool


@pytest.mark.asyncio
async def test_add_validator(async_client, validators, payer):
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
    assert len(validator_list.validators) == len(validators)
    for (validator_info, validator) in zip(validator_list.validators, validators):
        assert validator_info.vote_account_address == validator
        assert validator_info.active_stake_lamports == 0
        assert validator_info.transient_stake_lamports == 0
        assert validator_info.last_update_epoch == 0
        assert validator_info.status == StakeStatus.ACTIVE
        await remove_validator_from_pool(async_client, payer, stake_pool, validator)

    resp = await async_client.get_account_info(validator_list_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    validator_list = ValidatorList.decode(data[0], data[1])
    for validator_info in validator_list.validators:
        assert validator_info.status == StakeStatus.READY_FOR_REMOVAL
