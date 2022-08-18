import asyncio
import pytest
from solana.keypair import Keypair

from stake.actions import authorize, create_stake, delegate_stake
from stake.constants import MINIMUM_DELEGATION
from stake.state import StakeAuthorize


@pytest.mark.asyncio
async def test_create_stake(async_client, payer):
    stake = Keypair()
    await create_stake(async_client, payer, stake, payer.public_key, 1)


@pytest.mark.asyncio
async def test_delegate_stake(async_client, validators, payer):
    validator = validators[0]
    stake = Keypair()
    await create_stake(async_client, payer, stake, payer.public_key, MINIMUM_DELEGATION)
    await delegate_stake(async_client, payer, payer, stake.public_key, validator)


@pytest.mark.asyncio
async def test_authorize_stake(async_client, payer):
    stake = Keypair()
    new_authority = Keypair()
    await create_stake(async_client, payer, stake, payer.public_key, MINIMUM_DELEGATION)
    await asyncio.gather(
        authorize(async_client, payer, payer, stake.public_key, new_authority.public_key, StakeAuthorize.STAKER),
        authorize(async_client, payer, payer, stake.public_key, new_authority.public_key, StakeAuthorize.WITHDRAWER)
    )
    await authorize(async_client, payer, new_authority, stake.public_key, payer.public_key, StakeAuthorize.WITHDRAWER)
