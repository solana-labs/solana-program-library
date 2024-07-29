"""Time sensitive test, so run it first out of the bunch."""
import pytest
from solana.rpc.commitment import Confirmed
from spl.token.instructions import get_associated_token_address

from stake.constants import STAKE_LEN, LAMPORTS_PER_SOL
from stake_pool.actions import deposit_sol
from stake_pool.constants import MINIMUM_ACTIVE_STAKE, MINIMUM_RESERVE_LAMPORTS
from stake_pool.state import StakePool, ValidatorList

from bot.rebalance import rebalance


ENDPOINT: str = "http://127.0.0.1:8899"


@pytest.mark.asyncio
async def test_rebalance_this_is_very_slow(async_client, validators, payer, stake_pool_addresses, waiter):
    (stake_pool_address, validator_list_address, _) = stake_pool_addresses
    resp = await async_client.get_minimum_balance_for_rent_exemption(STAKE_LEN)
    stake_rent_exemption = resp.value
    # With minimum delegation at MINIMUM_DELEGATION + rent-exemption, when
    # decreasing, we'll need rent exemption + minimum delegation delegated to
    # cover all movements
    minimum_amount = MINIMUM_ACTIVE_STAKE + stake_rent_exemption
    increase_amount = MINIMUM_ACTIVE_STAKE + stake_rent_exemption
    deposit_amount = (increase_amount + stake_rent_exemption) * len(validators)

    resp = await async_client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    stake_pool = StakePool.decode(data)
    total_lamports = stake_pool.total_lamports + deposit_amount
    token_account = get_associated_token_address(payer.pubkey(), stake_pool.pool_mint)
    await deposit_sol(async_client, payer, stake_pool_address, token_account, deposit_amount)

    # Test case 1: Increase everywhere
    await rebalance(ENDPOINT, stake_pool_address, payer, 0.0)

    # should only have minimum left
    resp = await async_client.get_account_info(stake_pool.reserve_stake, commitment=Confirmed)
    assert resp.value.lamports == stake_rent_exemption + MINIMUM_RESERVE_LAMPORTS

    # should all be the same
    resp = await async_client.get_account_info(validator_list_address, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    validator_list = ValidatorList.decode(data)
    for validator in validator_list.validators:
        assert validator.active_stake_lamports == minimum_amount
        assert validator.transient_stake_lamports == total_lamports / len(validators) - minimum_amount

    # Test case 2: Decrease everything back to reserve
    print('Waiting for next epoch')
    await waiter.wait_for_next_epoch(async_client)
    max_in_reserve = total_lamports - minimum_amount * len(validators)
    await rebalance(ENDPOINT, stake_pool_address, payer, max_in_reserve / LAMPORTS_PER_SOL)

    # should still only have minimum left
    resp = await async_client.get_account_info(stake_pool.reserve_stake, commitment=Confirmed)
    reserve_lamports = resp.value.lamports
    assert reserve_lamports == stake_rent_exemption + MINIMUM_RESERVE_LAMPORTS

    # should all be decreasing now
    resp = await async_client.get_account_info(validator_list_address, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    validator_list = ValidatorList.decode(data)
    for validator in validator_list.validators:
        assert validator.active_stake_lamports == minimum_amount
        assert validator.transient_stake_lamports == max_in_reserve / len(validators)

    # Test case 3: Do nothing
    print('Waiting for next epoch')
    await waiter.wait_for_next_epoch(async_client)
    await rebalance(ENDPOINT, stake_pool_address, payer, max_in_reserve / LAMPORTS_PER_SOL)

    # should still only have minimum left + rent exemptions from increase
    resp = await async_client.get_account_info(stake_pool.reserve_stake, commitment=Confirmed)
    reserve_lamports = resp.value.lamports
    assert reserve_lamports == stake_rent_exemption + max_in_reserve + MINIMUM_RESERVE_LAMPORTS

    # should all be decreased now
    resp = await async_client.get_account_info(validator_list_address, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    validator_list = ValidatorList.decode(data)
    for validator in validator_list.validators:
        assert validator.active_stake_lamports == minimum_amount
        assert validator.transient_stake_lamports == 0
