import pytest
from solana.rpc.commitment import Confirmed
from solana.keypair import Keypair
from spl.token.instructions import get_associated_token_address

from stake.actions import create_stake, delegate_stake
from stake.constants import STAKE_LEN
from stake_pool.actions import deposit_stake, withdraw_stake
from stake_pool.state import StakePool


@pytest.mark.asyncio
async def test_deposit_withdraw_stake(async_client, validators, payer, stake_pool_addresses):
    (stake_pool_address, validator_list_address) = stake_pool_addresses
    resp = await async_client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    stake_pool = StakePool.decode(data[0], data[1])
    token_account = get_associated_token_address(payer.public_key, stake_pool.pool_mint)

    resp = await async_client.get_minimum_balance_for_rent_exemption(STAKE_LEN)
    stake_rent_exemption = resp['result']

    stake_amount = 1_000_000
    for validator in validators:
        stake = Keypair()
        await create_stake(async_client, payer, stake, payer.public_key, stake_amount)
        await delegate_stake(async_client, payer, payer, stake.public_key, validator)
        await deposit_stake(async_client, payer, stake_pool_address, validator, stake.public_key, token_account)

    pool_token_balance = await async_client.get_token_account_balance(token_account, Confirmed)
    pool_token_balance = pool_token_balance['result']['value']['amount']
    assert pool_token_balance == str((stake_amount + stake_rent_exemption) * len(validators))

    for validator in validators:
        destination_stake = Keypair()
        await withdraw_stake(
            async_client, payer, payer, destination_stake, stake_pool_address, validator,
            payer.public_key, token_account, stake_amount
        )

    pool_token_balance = await async_client.get_token_account_balance(token_account, Confirmed)
    pool_token_balance = pool_token_balance['result']['value']['amount']
    assert pool_token_balance == str(stake_rent_exemption * len(validators))
