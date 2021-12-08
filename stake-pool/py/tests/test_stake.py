import pytest
from solana.keypair import Keypair

from stake.actions import create_stake


@pytest.mark.asyncio
async def test_create_stake(async_client, payer):
    reserve_stake = Keypair()
    await create_stake(async_client, payer, reserve_stake, payer.public_key)
