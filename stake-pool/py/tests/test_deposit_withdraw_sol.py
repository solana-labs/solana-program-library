import pytest
from solana.rpc.commitment import Confirmed, Processed
from solders.keypair import Keypair
from spl.token.instructions import get_associated_token_address

from stake_pool.state import Fee, StakePool
from stake_pool.actions import create_all, deposit_sol, withdraw_sol


@pytest.mark.asyncio
async def test_deposit_withdraw_sol(async_client, waiter, payer):
    fee = Fee(numerator=1, denominator=1000)
    referral_fee = 20
    await waiter.wait_for_next_epoch(async_client)
    (stake_pool_address, validator_list_address, _) = await create_all(async_client, payer, fee, referral_fee)
    resp = await async_client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    stake_pool = StakePool.decode(data)
    token_account = get_associated_token_address(payer.pubkey(), stake_pool.pool_mint)
    deposit_amount = 100_000_000
    await deposit_sol(async_client, payer, stake_pool_address, token_account, deposit_amount)
    pool_token_balance = await async_client.get_token_account_balance(token_account, Confirmed)
    assert pool_token_balance.value.amount == str(deposit_amount)
    recipient = Keypair()
    await withdraw_sol(async_client, payer, token_account, stake_pool_address, recipient.pubkey(), deposit_amount)
    # for some reason, this is not always in sync when running all tests
    pool_token_balance = await async_client.get_token_account_balance(token_account, Processed)
    assert pool_token_balance.value.amount == str('0')
