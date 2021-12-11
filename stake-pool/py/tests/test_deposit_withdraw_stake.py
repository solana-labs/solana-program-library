import asyncio
import pytest
from typing import Tuple
from solana.rpc.async_api import AsyncClient
from solana.rpc.commitment import Confirmed
from solana.keypair import Keypair
from solana.publickey import PublicKey
from spl.token.instructions import get_associated_token_address

from stake.actions import create_stake, delegate_stake
from stake.constants import STAKE_LEN
from stake_pool.actions import deposit_stake, withdraw_stake, update_stake_pool
from stake_pool.state import StakePool


async def prepare_stake(
    async_client: AsyncClient, payer: Keypair, stake_pool_address: PublicKey,
    validator: PublicKey, token_account: PublicKey, stake_amount: int
) -> Tuple[PublicKey, PublicKey]:
    stake = Keypair()
    await create_stake(async_client, payer, stake, payer.public_key, stake_amount)
    await delegate_stake(async_client, payer, payer, stake.public_key, validator)
    return (stake.public_key, validator)


@pytest.mark.asyncio
async def test_deposit_withdraw_stake(async_client, validators, payer, stake_pool_addresses, waiter):
    (stake_pool_address, validator_list_address) = stake_pool_addresses
    resp = await async_client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    stake_pool = StakePool.decode(data[0], data[1])
    token_account = get_associated_token_address(payer.public_key, stake_pool.pool_mint)

    resp = await async_client.get_minimum_balance_for_rent_exemption(STAKE_LEN)
    stake_rent_exemption = resp['result']
    await waiter.wait_for_next_epoch_if_soon(async_client)
    await update_stake_pool(async_client, payer, stake_pool_address)

    stake_amount = 1_000_000
    futures = [
        prepare_stake(async_client, payer, stake_pool_address, validator, token_account, stake_amount)
        for validator in validators
    ]
    stakes = await asyncio.gather(*futures)
    await waiter.wait_for_next_epoch(async_client)
    await update_stake_pool(async_client, payer, stake_pool_address)
    futures = [
        deposit_stake(async_client, payer, stake_pool_address, validator, stake, token_account)
        for (stake, validator) in stakes
    ]
    stakes = await asyncio.gather(*futures)

    pool_token_balance = await async_client.get_token_account_balance(token_account, Confirmed)
    pool_token_balance = pool_token_balance['result']['value']['amount']
    assert pool_token_balance == str((stake_amount + stake_rent_exemption) * len(validators))

    futures = []
    for validator in validators:
        destination_stake = Keypair()
        futures.append(withdraw_stake(
            async_client, payer, payer, destination_stake, stake_pool_address, validator,
            payer.public_key, token_account, stake_amount
        ))
    await asyncio.gather(*futures)

    pool_token_balance = await async_client.get_token_account_balance(token_account, Confirmed)
    pool_token_balance = pool_token_balance['result']['value']['amount']
    assert pool_token_balance == str(stake_rent_exemption * len(validators))
