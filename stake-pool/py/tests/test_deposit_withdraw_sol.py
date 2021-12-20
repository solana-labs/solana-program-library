import pytest
from solana.rpc.commitment import Confirmed
from solana.keypair import Keypair
from spl.token.instructions import get_associated_token_address

from stake_pool.state import Fee, StakePool
from stake_pool.actions import create_all, deposit_sol, withdraw_sol


@pytest.mark.asyncio
async def test_deposit_withdraw_sol(async_client, payer):
    fee = Fee(numerator=1, denominator=1000)
    referral_fee = 20
    (stake_pool_address, validator_list_address) = await create_all(async_client, payer, fee, referral_fee)
    resp = await async_client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    stake_pool = StakePool.decode(data[0], data[1])
    token_account = get_associated_token_address(payer.public_key, stake_pool.pool_mint)
    deposit_amount = 100_000_000
    await deposit_sol(async_client, payer, stake_pool_address, token_account, deposit_amount)
    pool_token_balance = await async_client.get_token_account_balance(token_account, Confirmed)
    assert pool_token_balance['result']['value']['amount'] == str(deposit_amount)
    recipient = Keypair()
    await withdraw_sol(async_client, payer, token_account, stake_pool_address, recipient.public_key, deposit_amount)
    pool_token_balance = await async_client.get_token_account_balance(token_account, Confirmed)
    assert pool_token_balance['result']['value']['amount'] == str('0')
