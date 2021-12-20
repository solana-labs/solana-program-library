import pytest
from solana.keypair import Keypair
from solana.publickey import PublicKey
from solana.rpc.commitment import Confirmed

from vote.actions import create_vote
from vote.constants import VOTE_PROGRAM_ID


@pytest.mark.asyncio
async def test_create_vote(async_client, payer):
    vote = Keypair()
    node = Keypair()
    await create_vote(async_client, payer, vote, node, payer.public_key, payer.public_key, 10)
    resp = await async_client.get_account_info(vote.public_key, commitment=Confirmed)
    assert PublicKey(resp['result']['value']['owner']) == VOTE_PROGRAM_ID
