import pytest
from solana.keypair import Keypair
from solana.publickey import PublicKey
from solana.rpc.commitment import Confirmed

import actions.system
import actions.stake_pool
from vote.constants import VOTE_PROGRAM_ID
from stake_pool.state import Fee, ValidatorList, StakeStatus


@pytest.mark.asyncio
async def test_add_validator(async_client, validators):
    manager = Keypair()
    airdrop_lamports = 1_000_000_000_000
    await actions.system.airdrop(async_client, manager.public_key, airdrop_lamports)

    fee = Fee(numerator=1, denominator=1000)
    referral_fee = 20
    (stake_pool, validator_list_address) = await actions.stake_pool.create_all(async_client, manager, fee, referral_fee)
    for validator in validators:
        resp = await async_client.get_account_info(validator, commitment=Confirmed)
        assert PublicKey(resp['result']['value']['owner']) == VOTE_PROGRAM_ID
        await actions.stake_pool.add_validator_to_pool(async_client, manager, stake_pool, validator)

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
        await actions.stake_pool.remove_validator_from_pool(async_client, manager, stake_pool, validator)

    resp = await async_client.get_account_info(validator_list_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    validator_list = ValidatorList.decode(data[0], data[1])
    for validator_info in validator_list.validators:
        assert validator_info.status == StakeStatus.READY_FOR_REMOVAL
