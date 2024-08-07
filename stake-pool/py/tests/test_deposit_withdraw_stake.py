import pytest
from solana.rpc.commitment import Confirmed
from solders.keypair import Keypair
from spl.token.instructions import get_associated_token_address

from stake.actions import create_stake, delegate_stake
from stake.constants import STAKE_LEN
from stake.state import StakeStake
from stake_pool.actions import deposit_stake, withdraw_stake, update_stake_pool
from stake_pool.constants import MINIMUM_ACTIVE_STAKE
from stake_pool.state import StakePool


@pytest.mark.asyncio
async def test_deposit_withdraw_stake(async_client, validators, payer, stake_pool_addresses, waiter):
    (stake_pool_address, validator_list_address, _) = stake_pool_addresses
    resp = await async_client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    stake_pool = StakePool.decode(data)
    validator = next(iter(validators))
    stake_amount = MINIMUM_ACTIVE_STAKE
    stake = Keypair()
    await create_stake(async_client, payer, stake, payer.pubkey(), stake_amount)
    stake = stake.pubkey()
    await delegate_stake(async_client, payer, payer, stake, validator)
    resp = await async_client.get_account_info(stake, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    stake_state = StakeStake.decode(data)
    token_account = get_associated_token_address(payer.pubkey(), stake_pool.pool_mint)
    pre_pool_token_balance = await async_client.get_token_account_balance(token_account, Confirmed)
    pre_pool_token_balance = int(pre_pool_token_balance.value.amount)
    print(stake_state)

    await waiter.wait_for_next_epoch(async_client)

    await update_stake_pool(async_client, payer, stake_pool_address)
    await deposit_stake(async_client, payer, stake_pool_address, validator, stake, token_account)
    pool_token_balance = await async_client.get_token_account_balance(token_account, Confirmed)
    pool_token_balance = pool_token_balance.value.amount
    resp = await async_client.get_minimum_balance_for_rent_exemption(STAKE_LEN)
    stake_rent_exemption = resp.value
    assert pool_token_balance == str(stake_amount + stake_rent_exemption + pre_pool_token_balance)

    destination_stake = Keypair()
    await withdraw_stake(
        async_client, payer, payer, destination_stake, stake_pool_address, validator,
        payer.pubkey(), token_account, stake_amount
    )

    pool_token_balance = await async_client.get_token_account_balance(token_account, Confirmed)
    pool_token_balance = pool_token_balance.value.amount
    assert pool_token_balance == str(stake_rent_exemption + pre_pool_token_balance)
