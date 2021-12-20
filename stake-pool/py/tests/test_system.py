import pytest
from solana.keypair import Keypair
from solana.rpc.commitment import Confirmed

import system.actions


@pytest.mark.asyncio
async def test_airdrop(async_client):
    manager = Keypair()
    airdrop_lamports = 1_000_000
    await system.actions.airdrop(async_client, manager.public_key, airdrop_lamports)
    resp = await async_client.get_balance(manager.public_key, commitment=Confirmed)
    assert resp['result']['value'] == airdrop_lamports
