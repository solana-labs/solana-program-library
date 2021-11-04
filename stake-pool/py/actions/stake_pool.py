from typing import Tuple

from solana.keypair import Keypair
from solana.publickey import PublicKey
from solana.rpc.async_api import AsyncClient
from solana.rpc.commitment import Confirmed
from solana.rpc.types import TxOpts
from solana.transaction import Transaction
import solana.system_program as sys

from spl.token.constants import TOKEN_PROGRAM_ID

from stake.constants import STAKE_PROGRAM_ID, STAKE_LEN
from stake_pool.constants import STAKE_POOL_PROGRAM_ID, find_withdraw_authority_program_address
from stake_pool.state import STAKE_POOL_LAYOUT, ValidatorList, Fee, StakePool
import stake_pool.instructions as sp

import actions.stake
import actions.token


async def create(client: AsyncClient, manager: Keypair,
                 stake_pool: Keypair, validator_list: Keypair,
                 pool_mint: PublicKey, reserve_stake: PublicKey,
                 manager_fee_account: PublicKey, fee: Fee, referral_fee: int):
    resp = await client.get_minimum_balance_for_rent_exemption(STAKE_POOL_LAYOUT.sizeof())
    pool_balance = resp['result']
    txn = Transaction()
    txn.add(
        sys.create_account(
            sys.CreateAccountParams(
                from_pubkey=manager.public_key,
                new_account_pubkey=stake_pool.public_key,
                lamports=pool_balance,
                space=STAKE_POOL_LAYOUT.sizeof(),
                program_id=STAKE_POOL_PROGRAM_ID,
            )
        )
    )
    max_validators = 3950  # current supported max by the program, go big!
    validator_list_size = ValidatorList.calculate_validator_list_size(max_validators)
    resp = await client.get_minimum_balance_for_rent_exemption(validator_list_size)
    validator_list_balance = resp['result']
    txn.add(
        sys.create_account(
            sys.CreateAccountParams(
                from_pubkey=manager.public_key,
                new_account_pubkey=validator_list.public_key,
                lamports=validator_list_balance,
                space=validator_list_size,
                program_id=STAKE_POOL_PROGRAM_ID,
            )
        )
    )
    await client.send_transaction(
        txn, manager, stake_pool, validator_list, opts=TxOpts(skip_confirmation=False, preflight_commitment=Confirmed))

    txn = Transaction()
    txn.add(
        sp.initialize(
            sp.InitializeParams(
                program_id=STAKE_POOL_PROGRAM_ID,
                stake_pool=stake_pool.public_key,
                manager=manager.public_key,
                staker=manager.public_key,
                validator_list=validator_list.public_key,
                reserve_stake=reserve_stake,
                pool_mint=pool_mint,
                manager_fee_account=manager_fee_account,
                token_program_id=TOKEN_PROGRAM_ID,
                epoch_fee=fee,
                withdrawal_fee=fee,
                deposit_fee=fee,
                referral_fee=referral_fee,
                max_validators=max_validators,
            )
        )
    )
    await client.send_transaction(
        txn, manager, validator_list, opts=TxOpts(skip_confirmation=False, preflight_commitment=Confirmed))


async def create_all(client: AsyncClient, manager: Keypair, fee: Fee, referral_fee: int) -> Tuple[PublicKey, PublicKey]:
    stake_pool = Keypair()
    validator_list = Keypair()
    (pool_withdraw_authority, seed) = find_withdraw_authority_program_address(
        STAKE_POOL_PROGRAM_ID, stake_pool.public_key)

    reserve_stake = Keypair()
    await actions.stake.create_stake(client, manager, reserve_stake, pool_withdraw_authority)

    pool_mint = Keypair()
    await actions.token.create_mint(client, manager, pool_mint, pool_withdraw_authority)

    manager_fee_account = await actions.token.create_associated_token_account(
        client,
        manager,
        manager.public_key,
        pool_mint.public_key,
    )

    fee = Fee(numerator=1, denominator=1000)
    referral_fee = 20
    await create(
        client, manager, stake_pool, validator_list, pool_mint.public_key,
        reserve_stake.public_key, manager_fee_account, fee, referral_fee)
    return (stake_pool.public_key, validator_list.public_key)


async def add_validator_to_pool(
    client: AsyncClient, funder: Keypair,
    stake_pool_address: PublicKey, validator: PublicKey
):
    resp = await client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    stake_pool = StakePool.decode(data[0], data[1])
    txn = Transaction()
    txn.add(
        sp.add_validator_to_pool_with_vote(
            STAKE_POOL_PROGRAM_ID,
            stake_pool_address,
            stake_pool.staker,
            stake_pool.validator_list,
            funder.public_key,
            validator,
        )
    )
    await client.send_transaction(
        txn, funder, opts=TxOpts(skip_confirmation=False, preflight_commitment=Confirmed))


async def remove_validator_from_pool(
    client: AsyncClient, staker: Keypair,
    stake_pool_address: PublicKey, validator: PublicKey
):
    resp = await client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    stake_pool = StakePool.decode(data[0], data[1])
    resp = await client.get_account_info(stake_pool.validator_list, commitment=Confirmed)
    data = resp['result']['value']['data']
    validator_list = ValidatorList.decode(data[0], data[1])
    validator_info = next(x for x in validator_list.validators if x.vote_account_address == validator)
    destination_stake = Keypair()
    txn = Transaction()
    txn.add(
        sys.create_account(
            sys.CreateAccountParams(
                from_pubkey=staker.public_key,
                new_account_pubkey=destination_stake.public_key,
                lamports=0,  # will get filled by split
                space=STAKE_LEN,
                program_id=STAKE_PROGRAM_ID,
            )
        )
    )
    txn.add(
        sp.remove_validator_from_pool_with_vote(
            STAKE_POOL_PROGRAM_ID,
            stake_pool_address,
            stake_pool.staker,
            stake_pool.validator_list,
            staker.public_key,
            validator,
            validator_info.transient_seed_suffix_start,
            destination_stake.public_key
        )
    )
    await client.send_transaction(
        txn, staker, destination_stake,
        opts=TxOpts(skip_confirmation=False, preflight_commitment=Confirmed))
