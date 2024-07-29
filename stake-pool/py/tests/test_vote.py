import pytest
from solders.keypair import Keypair
from solana.rpc.commitment import Confirmed

from vote.actions import create_vote
from vote.constants import VOTE_PROGRAM_ID


@pytest.mark.asyncio
async def test_create_vote(async_client, payer):
    vote = Keypair()
    node = Keypair()
    await create_vote(async_client, payer, vote, node, payer.pubkey(), payer.pubkey(), 10)
    resp = await async_client.get_account_info(vote.pubkey(), commitment=Confirmed)
    assert resp.value.owner == VOTE_PROGRAM_ID
