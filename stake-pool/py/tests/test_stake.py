import asyncio
import pytest
from solana.keypair import Keypair

from stake.state import StakeAuthorize
from stake.actions import authorize, create_stake, delegate_stake


@pytest.mark.asyncio
async def test_create_stake(async_client, payer):
    stake = Keypair()
    await create_stake(async_client, payer, stake, payer.public_key, 100_000)


@pytest.mark.asyncio
async def test_delegate_stake(async_client, validators, payer):
    validator = validators[0]
    stake = Keypair()
    await create_stake(async_client, payer, stake, payer.public_key, 1)
    await delegate_stake(async_client, payer, payer, stake.public_key, validator)


@pytest.mark.asyncio
async def test_authorize_stake(async_client, payer):
    stake = Keypair()
    new_authority = Keypair()
    await create_stake(async_client, payer, stake, payer.public_key, 1_000)
    await asyncio.gather(
        authorize(async_client, payer, payer, stake.public_key, new_authority.public_key, StakeAuthorize.STAKER),
        authorize(async_client, payer, payer, stake.public_key, new_authority.public_key, StakeAuthorize.WITHDRAWER)
    )
    await authorize(async_client, payer, new_authority, stake.public_key, payer.public_key, StakeAuthorize.WITHDRAWER)
