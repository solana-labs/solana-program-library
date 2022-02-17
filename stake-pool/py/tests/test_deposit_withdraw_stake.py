import pytest
from solana.rpc.commitment import Confirmed
from solana.keypair import Keypair
from spl.token.instructions import get_associated_token_address

from stake.actions import create_stake, delegate_stake
from stake.constants import STAKE_LEN
from stake.state import StakeState
from stake_pool.actions import deposit_stake, withdraw_stake, update_stake_pool
from stake_pool.state import StakePool


@pytest.mark.asyncio
async def test_deposit_withdraw_stake(async_client, validators, payer, stake_pool_addresses, waiter):
    (stake_pool_address, validator_list_address) = stake_pool_addresses
    resp = await async_client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    stake_pool = StakePool.decode(data[0], data[1])
    validator = next(iter(validators))
    stake_amount = 1_000_000
    stake = Keypair()
    await create_stake(async_client, payer, stake, payer.public_key, stake_amount)
    stake = stake.public_key
    await delegate_stake(async_client, payer, payer, stake, validator)
    resp = await async_client.get_account_info(stake, commitment=Confirmed)
    data = resp['result']['value']['data']
    stake_state = StakeState.decode(data[0], data[1])
    print(stake_state)

    await waiter.wait_for_next_epoch(async_client)

    await update_stake_pool(async_client, payer, stake_pool_address)
    token_account = get_associated_token_address(payer.public_key, stake_pool.pool_mint)
    await deposit_stake(async_client, payer, stake_pool_address, validator, stake, token_account)
    pool_token_balance = await async_client.get_token_account_balance(token_account, Confirmed)
    pool_token_balance = pool_token_balance['result']['value']['amount']
    resp = await async_client.get_minimum_balance_for_rent_exemption(STAKE_LEN)
    stake_rent_exemption = resp['result']
    assert pool_token_balance == str(stake_amount + stake_rent_exemption)

    destination_stake = Keypair()
    await withdraw_stake(
        async_client, payer, payer, destination_stake, stake_pool_address, validator,
        payer.public_key, token_account, stake_amount
    )

    pool_token_balance = await async_client.get_token_account_balance(token_account, Confirmed)
    pool_token_balance = pool_token_balance['result']['value']['amount']
    assert pool_token_balance == str(stake_rent_exemption)
