import pytest
from solders.keypair import Keypair

from spl_token.actions import create_mint, create_associated_token_account


@pytest.mark.asyncio
async def test_create_mint(async_client, payer):
    pool_mint = Keypair()
    await create_mint(async_client, payer, pool_mint, payer.pubkey())
    await create_associated_token_account(
        async_client,
        payer,
        payer.pubkey(),
        pool_mint.pubkey(),
    )
