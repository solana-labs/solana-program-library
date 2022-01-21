import pytest
from solana.keypair import Keypair

from spl_token.actions import create_mint, create_associated_token_account


@pytest.mark.asyncio
async def test_create_mint(async_client, payer):
    pool_mint = Keypair()
    await create_mint(async_client, payer, pool_mint, payer.public_key)
    await create_associated_token_account(
        async_client,
        payer,
        payer.public_key,
        pool_mint.public_key,
    )
