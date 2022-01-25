"""Time sensitive test, so run it first out of the bunch."""
import pytest
from solana.rpc.commitment import Confirmed
from spl.token.instructions import get_associated_token_address

from stake.constants import STAKE_LEN
from stake_pool.state import StakePool, ValidatorList
from stake_pool.actions import deposit_sol

from bot.rebalance import rebalance


ENDPOINT: str = "http://127.0.0.1:8899"


@pytest.mark.asyncio
async def test_rebalance_this_is_very_slow(async_client, validators, payer, stake_pool_addresses, waiter):
    (stake_pool_address, validator_list_address) = stake_pool_addresses
    resp = await async_client.get_minimum_balance_for_rent_exemption(STAKE_LEN)
    stake_rent_exemption = resp['result']
    increase_amount = 100_000_000
    deposit_amount = (increase_amount + stake_rent_exemption) * len(validators)

    resp = await async_client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    stake_pool = StakePool.decode(data[0], data[1])
    token_account = get_associated_token_address(payer.public_key, stake_pool.pool_mint)
    await deposit_sol(async_client, payer, stake_pool_address, token_account, deposit_amount)

    # Test case 1: Increase
    await rebalance(ENDPOINT, stake_pool_address, payer, 0.0)

    # should only have minimum left
    resp = await async_client.get_account_info(stake_pool.reserve_stake, commitment=Confirmed)
    assert resp['result']['value']['lamports'] == stake_rent_exemption + 1

    # should all be the same
    resp = await async_client.get_account_info(validator_list_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    validator_list = ValidatorList.decode(data[0], data[1])
    for validator in validator_list.validators:
        assert validator.active_stake_lamports == 0
        assert validator.transient_stake_lamports == increase_amount + stake_rent_exemption

    # Test case 2: Decrease
    print('Waiting for next epoch')
    await waiter.wait_for_next_epoch(async_client)
    await rebalance(ENDPOINT, stake_pool_address, payer, deposit_amount / 1_000_000_000)

    # should still only have minimum left + rent exemptions from increase
    resp = await async_client.get_account_info(stake_pool.reserve_stake, commitment=Confirmed)
    reserve_lamports = resp['result']['value']['lamports']
    assert reserve_lamports == stake_rent_exemption * (1 + len(validator_list.validators)) + 1

    # should all be decreasing now
    resp = await async_client.get_account_info(validator_list_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    validator_list = ValidatorList.decode(data[0], data[1])
    for validator in validator_list.validators:
        assert validator.active_stake_lamports == 0
        assert validator.transient_stake_lamports == increase_amount

    # Test case 3: Do nothing
    print('Waiting for next epoch')
    await waiter.wait_for_next_epoch(async_client)
    await rebalance(ENDPOINT, stake_pool_address, payer, deposit_amount / 1_000_000_000)

    # should still only have minimum left + rent exemptions from increase
    resp = await async_client.get_account_info(stake_pool.reserve_stake, commitment=Confirmed)
    reserve_lamports = resp['result']['value']['lamports']
    assert reserve_lamports == stake_rent_exemption + deposit_amount + 1

    # should all be decreasing now
    resp = await async_client.get_account_info(validator_list_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    validator_list = ValidatorList.decode(data[0], data[1])
    for validator in validator_list.validators:
        assert validator.active_stake_lamports == 0
        assert validator.transient_stake_lamports == 0
