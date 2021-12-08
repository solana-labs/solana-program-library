import pytest
from solana.keypair import Keypair

from vote.actions import create_vote


@pytest.mark.asyncio
async def test_create_vote(async_client, payer):
    vote = Keypair()
    node = Keypair()
    await create_vote(async_client, payer, vote, node, payer.public_key, payer.public_key, 10)
