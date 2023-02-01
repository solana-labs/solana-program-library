import pytest
from stake_pool.actions import create_all, create_token_metadata
from stake_pool.state import Fee, StakePool
from solana.rpc.commitment import Confirmed
from solana.rpc.async_api import AsyncClient
from solana.keypair import Keypair
from stake_pool.constants import find_metadata_account



@pytest.mark.asyncio
async def test_create_metadata_success(async_client: AsyncClient, payer: Keypair):
    fee = Fee(numerator=1, denominator=1000)
    referral_fee = 20
    (stake_pool_address, _validator_list_address, _) = await create_all(async_client, payer, fee, referral_fee)
    resp = await async_client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    stake_pool = StakePool.decode(data[0], data[1])

    name = "test_name"
    symbol = "SYM"
    uri = "test_uri"
    await create_token_metadata(async_client, payer, stake_pool_address, name, symbol, uri)

    (metadata_program_address, _seed) = find_metadata_account(stake_pool.pool_mint)
    resp = await async_client.get_account_info(metadata_program_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    assert data[0] == name
    assert data[1] == symbol
    assert data[2] == uri