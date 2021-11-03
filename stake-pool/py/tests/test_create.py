import pytest
from solana.keypair import Keypair
from solana.rpc.commitment import Confirmed
from spl.token.constants import TOKEN_PROGRAM_ID

from stake_pool.constants import find_withdraw_authority_program_address, STAKE_POOL_PROGRAM_ID
from stake_pool.state import StakePool, Fee

import actions.system
import actions.stake
import actions.stake_pool
import actions.token


@pytest.mark.asyncio
async def test_airdrop(async_client):
    manager = Keypair()
    airdrop_lamports = 1_000_000
    await actions.system.airdrop(async_client, manager.public_key, airdrop_lamports)
    resp = await async_client.get_balance(manager.public_key, commitment=Confirmed)
    assert resp['result']['value'] == airdrop_lamports


@pytest.mark.asyncio
async def test_create_stake(async_client):
    owner = Keypair()
    reserve_stake = Keypair()
    airdrop_lamports = 1_000_000_000
    await actions.system.airdrop(async_client, owner.public_key, airdrop_lamports)
    await actions.stake.create_stake(async_client, owner, reserve_stake, owner.public_key)


@pytest.mark.asyncio
async def test_create_mint(async_client):
    owner = Keypair()
    airdrop_lamports = 1_000_000_000
    await actions.system.airdrop(async_client, owner.public_key, airdrop_lamports)
    pool_mint = Keypair()
    await actions.token.create_mint(async_client, owner, pool_mint, owner.public_key)
    await actions.token.create_associated_token_account(
        async_client,
        owner,
        owner.public_key,
        pool_mint.public_key,
    )


@pytest.mark.asyncio
async def test_create_stake_pool(async_client):
    manager = Keypair()
    airdrop_lamports = 1_000_000_000_000
    await actions.system.airdrop(async_client, manager.public_key, airdrop_lamports)

    stake_pool = Keypair()
    validator_list = Keypair()
    (pool_withdraw_authority, seed) = find_withdraw_authority_program_address(
        STAKE_POOL_PROGRAM_ID, stake_pool.public_key)

    reserve_stake = Keypair()
    await actions.stake.create_stake(async_client, manager, reserve_stake, pool_withdraw_authority)

    pool_mint = Keypair()
    await actions.token.create_mint(async_client, manager, pool_mint, pool_withdraw_authority)

    manager_fee_account = await actions.token.create_associated_token_account(
        async_client,
        manager,
        manager.public_key,
        pool_mint.public_key,
    )

    fee = Fee(numerator=1, denominator=1000)
    referral_fee = 20
    await actions.stake_pool.create(
        async_client, manager, stake_pool, validator_list, pool_mint.public_key,
        reserve_stake.public_key, manager_fee_account, fee, referral_fee)
    resp = await async_client.get_account_info(stake_pool.public_key, commitment=Confirmed)
    assert resp['result']['value']['owner'] == str(STAKE_POOL_PROGRAM_ID)
    data = resp['result']['value']['data']
    pool_data = StakePool.decode(data[0], data[1])
    assert pool_data.manager == manager.public_key
    assert pool_data.staker == manager.public_key
    assert pool_data.stake_withdraw_bump_seed == seed
    assert pool_data.validator_list == validator_list.public_key
    assert pool_data.reserve_stake == reserve_stake.public_key
    assert pool_data.pool_mint == pool_mint.public_key
    assert pool_data.manager_fee_account == manager_fee_account
    assert pool_data.token_program_id == TOKEN_PROGRAM_ID
    assert pool_data.total_lamports == 0
    assert pool_data.pool_token_supply == 0
    assert pool_data.epoch_fee == fee
    assert pool_data.next_epoch_fee is None
    assert pool_data.preferred_deposit_validator is None
    assert pool_data.preferred_withdraw_validator is None
    assert pool_data.stake_deposit_fee == fee
    assert pool_data.stake_withdrawal_fee == fee
    assert pool_data.next_stake_withdrawal_fee is None
    assert pool_data.stake_referral_fee == referral_fee
    assert pool_data.sol_deposit_authority is None
    assert pool_data.sol_deposit_fee == fee
    assert pool_data.sol_referral_fee == referral_fee
    assert pool_data.sol_withdraw_authority is None
    assert pool_data.sol_withdrawal_fee == fee
    assert pool_data.next_sol_withdrawal_fee is None
    assert pool_data.last_epoch_pool_token_supply == 0
    assert pool_data.last_epoch_total_lamports == 0
