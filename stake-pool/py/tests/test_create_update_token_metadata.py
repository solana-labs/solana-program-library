import pytest
from stake_pool.actions import create_all, create_token_metadata, update_token_metadata
from stake_pool.state import Fee, StakePool
from solana.rpc.commitment import Confirmed
from stake_pool.constants import find_metadata_account


@pytest.mark.asyncio
async def test_create_metadata_success(async_client, waiter, payer):
    fee = Fee(numerator=1, denominator=1000)
    referral_fee = 20
    await waiter.wait_for_next_epoch_if_soon(async_client)
    (stake_pool_address, _validator_list_address, _) = await create_all(async_client, payer, fee, referral_fee)
    resp = await async_client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    stake_pool = StakePool.decode(data)

    name = "test_name"
    symbol = "SYM"
    uri = "test_uri"
    await create_token_metadata(async_client, payer, stake_pool_address, name, symbol, uri)

    (metadata_program_address, _seed) = find_metadata_account(stake_pool.pool_mint)
    resp = await async_client.get_account_info(metadata_program_address, commitment=Confirmed)
    raw_data = resp.value.data if resp.value else bytes()
    assert name == str(raw_data[69:101], "utf-8")[:len(name)]
    assert symbol == str(raw_data[105:115], "utf-8")[:len(symbol)]
    assert uri == str(raw_data[119:319], "utf-8")[:len(uri)]


@pytest.mark.asyncio
async def test_update_metadata_success(async_client, waiter, payer):
    fee = Fee(numerator=1, denominator=1000)
    referral_fee = 20
    await waiter.wait_for_next_epoch_if_soon(async_client)
    (stake_pool_address, _validator_list_address, _) = await create_all(async_client, payer, fee, referral_fee)
    resp = await async_client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    stake_pool = StakePool.decode(data)

    name = "test_name"
    symbol = "SYM"
    uri = "test_uri"
    await create_token_metadata(async_client, payer, stake_pool_address, name, symbol, uri)

    (metadata_program_address, _seed) = find_metadata_account(stake_pool.pool_mint)
    resp = await async_client.get_account_info(metadata_program_address, commitment=Confirmed)
    raw_data = resp.value.data if resp.value else bytes()
    assert name == str(raw_data[69:101], "utf-8")[:len(name)]
    assert symbol == str(raw_data[105:115], "utf-8")[:len(symbol)]
    assert uri == str(raw_data[119:319], "utf-8")[:len(uri)]

    updated_name = "updated_name"
    updated_symbol = "USM"
    updated_uri = "updated_uri"
    await update_token_metadata(async_client, payer, stake_pool_address, updated_name, updated_symbol, updated_uri)

    (metadata_program_address, _seed) = find_metadata_account(stake_pool.pool_mint)
    resp = await async_client.get_account_info(metadata_program_address, commitment=Confirmed)
    raw_data = resp.value.data if resp.value else bytes()
    assert updated_name == str(raw_data[69:101], "utf-8")[:len(updated_name)]
    assert updated_symbol == str(raw_data[105:115], "utf-8")[:len(updated_symbol)]
    assert updated_uri == str(raw_data[119:319], "utf-8")[:len(updated_uri)]
