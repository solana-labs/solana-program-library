from typing import Tuple

from solana.keypair import Keypair
from solana.publickey import PublicKey
from solana.rpc.async_api import AsyncClient
from solana.rpc.commitment import Confirmed
from solana.rpc.types import TxOpts
from solana.sysvar import SYSVAR_CLOCK_PUBKEY, SYSVAR_RENT_PUBKEY, SYSVAR_STAKE_HISTORY_PUBKEY
from solana.transaction import Transaction
import solana.system_program as sys

from spl.token.constants import TOKEN_PROGRAM_ID

from stake.constants import STAKE_PROGRAM_ID, STAKE_LEN, SYSVAR_STAKE_CONFIG_ID
import stake.instructions as st
from stake.state import StakeAuthorize
from stake_pool.constants import \
    MAX_VALIDATORS_TO_UPDATE, \
    STAKE_POOL_PROGRAM_ID, \
    find_stake_program_address, \
    find_transient_stake_program_address, \
    find_withdraw_authority_program_address
from stake_pool.state import STAKE_POOL_LAYOUT, ValidatorList, Fee, StakePool
import stake_pool.instructions as sp

from stake.actions import create_stake
from spl_token.actions import create_mint, create_associated_token_account


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
    max_validators = 2950  # current supported max by the program, go big!
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

    (withdraw_authority, seed) = find_withdraw_authority_program_address(
        STAKE_POOL_PROGRAM_ID, stake_pool.public_key)
    txn = Transaction()
    txn.add(
        sp.initialize(
            sp.InitializeParams(
                program_id=STAKE_POOL_PROGRAM_ID,
                stake_pool=stake_pool.public_key,
                manager=manager.public_key,
                staker=manager.public_key,
                withdraw_authority=withdraw_authority,
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
    await create_stake(client, manager, reserve_stake, pool_withdraw_authority, 1)

    pool_mint = Keypair()
    await create_mint(client, manager, pool_mint, pool_withdraw_authority)

    manager_fee_account = await create_associated_token_account(
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


async def deposit_sol(
    client: AsyncClient, funder: Keypair, stake_pool_address: PublicKey,
    destination_token_account: PublicKey, amount: int,
):
    resp = await client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    stake_pool = StakePool.decode(data[0], data[1])

    (withdraw_authority, seed) = find_withdraw_authority_program_address(STAKE_POOL_PROGRAM_ID, stake_pool_address)

    txn = Transaction()
    txn.add(
        sp.deposit_sol(
            sp.DepositSolParams(
                program_id=STAKE_POOL_PROGRAM_ID,
                stake_pool=stake_pool_address,
                withdraw_authority=withdraw_authority,
                reserve_stake=stake_pool.reserve_stake,
                funding_account=funder.public_key,
                destination_pool_account=destination_token_account,
                manager_fee_account=stake_pool.manager_fee_account,
                referral_pool_account=destination_token_account,
                pool_mint=stake_pool.pool_mint,
                system_program_id=sys.SYS_PROGRAM_ID,
                token_program_id=stake_pool.token_program_id,
                amount=amount,
                deposit_authority=None,
            )
        )
    )
    await client.send_transaction(
        txn, funder, opts=TxOpts(skip_confirmation=False, preflight_commitment=Confirmed))


async def withdraw_sol(
    client: AsyncClient, owner: Keypair, source_token_account: PublicKey,
    stake_pool_address: PublicKey, destination_system_account: PublicKey, amount: int,
):
    resp = await client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    stake_pool = StakePool.decode(data[0], data[1])

    (withdraw_authority, seed) = find_withdraw_authority_program_address(STAKE_POOL_PROGRAM_ID, stake_pool_address)

    txn = Transaction()
    txn.add(
        sp.withdraw_sol(
            sp.WithdrawSolParams(
                program_id=STAKE_POOL_PROGRAM_ID,
                stake_pool=stake_pool_address,
                withdraw_authority=withdraw_authority,
                source_transfer_authority=owner.public_key,
                source_pool_account=source_token_account,
                reserve_stake=stake_pool.reserve_stake,
                destination_system_account=destination_system_account,
                manager_fee_account=stake_pool.manager_fee_account,
                pool_mint=stake_pool.pool_mint,
                clock_sysvar=SYSVAR_CLOCK_PUBKEY,
                stake_history_sysvar=SYSVAR_STAKE_HISTORY_PUBKEY,
                stake_program_id=STAKE_PROGRAM_ID,
                token_program_id=stake_pool.token_program_id,
                amount=amount,
                sol_withdraw_authority=None,
            )
        )
    )
    await client.send_transaction(
        txn, owner, opts=TxOpts(skip_confirmation=False, preflight_commitment=Confirmed))


async def deposit_stake(
    client: AsyncClient,
    deposit_stake_authority: Keypair,
    stake_pool_address: PublicKey,
    validator_vote: PublicKey,
    deposit_stake: PublicKey,
    destination_pool_account: PublicKey,
):
    resp = await client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    stake_pool = StakePool.decode(data[0], data[1])

    (withdraw_authority, _) = find_withdraw_authority_program_address(STAKE_POOL_PROGRAM_ID, stake_pool_address)
    (validator_stake, _) = find_stake_program_address(
        STAKE_POOL_PROGRAM_ID,
        validator_vote,
        stake_pool_address,
    )

    txn = Transaction()
    txn.add(
        st.authorize(
            st.AuthorizeParams(
                stake=deposit_stake,
                clock_sysvar=SYSVAR_CLOCK_PUBKEY,
                authority=deposit_stake_authority.public_key,
                new_authority=stake_pool.stake_deposit_authority,
                stake_authorize=StakeAuthorize.STAKER,
            )
        )
    )
    txn.add(
        st.authorize(
            st.AuthorizeParams(
                stake=deposit_stake,
                clock_sysvar=SYSVAR_CLOCK_PUBKEY,
                authority=deposit_stake_authority.public_key,
                new_authority=stake_pool.stake_deposit_authority,
                stake_authorize=StakeAuthorize.WITHDRAWER,
            )
        )
    )
    txn.add(
        sp.deposit_stake(
            sp.DepositStakeParams(
                program_id=STAKE_POOL_PROGRAM_ID,
                stake_pool=stake_pool_address,
                validator_list=stake_pool.validator_list,
                deposit_authority=stake_pool.stake_deposit_authority,
                withdraw_authority=withdraw_authority,
                deposit_stake=deposit_stake,
                validator_stake=validator_stake,
                reserve_stake=stake_pool.reserve_stake,
                destination_pool_account=destination_pool_account,
                manager_fee_account=stake_pool.manager_fee_account,
                referral_pool_account=destination_pool_account,
                pool_mint=stake_pool.pool_mint,
                clock_sysvar=SYSVAR_CLOCK_PUBKEY,
                stake_history_sysvar=SYSVAR_STAKE_HISTORY_PUBKEY,
                token_program_id=stake_pool.token_program_id,
                stake_program_id=STAKE_PROGRAM_ID,
            )
        )
    )
    await client.send_transaction(
        txn, deposit_stake_authority, opts=TxOpts(skip_confirmation=False, preflight_commitment=Confirmed))


async def withdraw_stake(
    client: AsyncClient,
    payer: Keypair,
    source_transfer_authority: Keypair,
    destination_stake: Keypair,
    stake_pool_address: PublicKey,
    validator_vote: PublicKey,
    destination_stake_authority: PublicKey,
    source_pool_account: PublicKey,
    amount: int,
):
    resp = await client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    stake_pool = StakePool.decode(data[0], data[1])

    (withdraw_authority, _) = find_withdraw_authority_program_address(STAKE_POOL_PROGRAM_ID, stake_pool_address)
    (validator_stake, _) = find_stake_program_address(
        STAKE_POOL_PROGRAM_ID,
        validator_vote,
        stake_pool_address,
    )

    resp = await client.get_minimum_balance_for_rent_exemption(STAKE_LEN)
    stake_rent_exemption = resp['result']

    txn = Transaction()
    txn.add(
        sys.create_account(
            sys.CreateAccountParams(
                from_pubkey=payer.public_key,
                new_account_pubkey=destination_stake.public_key,
                lamports=stake_rent_exemption,
                space=STAKE_LEN,
                program_id=STAKE_PROGRAM_ID,
            )
        )
    )
    txn.add(
        sp.withdraw_stake(
            sp.WithdrawStakeParams(
                program_id=STAKE_POOL_PROGRAM_ID,
                stake_pool=stake_pool_address,
                validator_list=stake_pool.validator_list,
                withdraw_authority=withdraw_authority,
                validator_stake=validator_stake,
                destination_stake=destination_stake.public_key,
                destination_stake_authority=destination_stake_authority,
                source_transfer_authority=source_transfer_authority.public_key,
                source_pool_account=source_pool_account,
                manager_fee_account=stake_pool.manager_fee_account,
                pool_mint=stake_pool.pool_mint,
                clock_sysvar=SYSVAR_CLOCK_PUBKEY,
                token_program_id=stake_pool.token_program_id,
                stake_program_id=STAKE_PROGRAM_ID,
                amount=amount,
            )
        )
    )
    signers = [payer, source_transfer_authority, destination_stake] \
        if payer != source_transfer_authority else [payer, destination_stake]
    await client.send_transaction(
        txn, *signers, opts=TxOpts(skip_confirmation=False, preflight_commitment=Confirmed))


async def update_stake_pool(client: AsyncClient, payer: Keypair, stake_pool_address: PublicKey):
    """Create and send all instructions to completely update a stake pool after epoch change."""
    resp = await client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    stake_pool = StakePool.decode(data[0], data[1])
    resp = await client.get_account_info(stake_pool.validator_list, commitment=Confirmed)
    data = resp['result']['value']['data']
    validator_list = ValidatorList.decode(data[0], data[1])
    (withdraw_authority, seed) = find_withdraw_authority_program_address(STAKE_POOL_PROGRAM_ID, stake_pool_address)
    update_list_instructions = []
    validator_chunks = [
        validator_list.validators[i:i+MAX_VALIDATORS_TO_UPDATE]
        for i in range(0, len(validator_list.validators), MAX_VALIDATORS_TO_UPDATE)
    ]
    start_index = 0
    for validator_chunk in validator_chunks:
        validator_and_transient_stake_pairs = []
        for validator in validator_chunk:
            (validator_stake_address, _) = find_stake_program_address(
                STAKE_POOL_PROGRAM_ID,
                validator.vote_account_address,
                stake_pool_address,
            )
            validator_and_transient_stake_pairs.append(validator_stake_address)
            (transient_stake_address, _) = find_transient_stake_program_address(
                STAKE_POOL_PROGRAM_ID,
                validator.vote_account_address,
                stake_pool_address,
                validator.transient_seed_suffix_start,
            )
            validator_and_transient_stake_pairs.append(transient_stake_address)
        update_list_instructions.append(
            sp.update_validator_list_balance(
                sp.UpdateValidatorListBalanceParams(
                    program_id=STAKE_POOL_PROGRAM_ID,
                    stake_pool=stake_pool_address,
                    withdraw_authority=withdraw_authority,
                    validator_list=stake_pool.validator_list,
                    reserve_stake=stake_pool.reserve_stake,
                    clock_sysvar=SYSVAR_CLOCK_PUBKEY,
                    stake_history_sysvar=SYSVAR_STAKE_HISTORY_PUBKEY,
                    stake_program_id=STAKE_PROGRAM_ID,
                    validator_and_transient_stake_pairs=validator_and_transient_stake_pairs,
                    start_index=start_index,
                    no_merge=False,
                )
            )
        )
        start_index += MAX_VALIDATORS_TO_UPDATE
    if update_list_instructions:
        last_instruction = update_list_instructions.pop()
        for update_list_instruction in update_list_instructions:
            txn = Transaction()
            txn.add(update_list_instruction)
            await client.send_transaction(
                txn, payer, opts=TxOpts(skip_confirmation=True, preflight_commitment=Confirmed))
        txn = Transaction()
        txn.add(last_instruction)
        await client.send_transaction(
            txn, payer, opts=TxOpts(skip_confirmation=False, preflight_commitment=Confirmed))
    txn = Transaction()
    txn.add(
        sp.update_stake_pool_balance(
            sp.UpdateStakePoolBalanceParams(
                program_id=STAKE_POOL_PROGRAM_ID,
                stake_pool=stake_pool_address,
                withdraw_authority=withdraw_authority,
                validator_list=stake_pool.validator_list,
                reserve_stake=stake_pool.reserve_stake,
                manager_fee_account=stake_pool.manager_fee_account,
                pool_mint=stake_pool.pool_mint,
                token_program_id=stake_pool.token_program_id,
            )
        )
    )
    txn.add(
        sp.cleanup_removed_validator_entries(
            sp.CleanupRemovedValidatorEntriesParams(
                program_id=STAKE_POOL_PROGRAM_ID,
                stake_pool=stake_pool_address,
                validator_list=stake_pool.validator_list,
            )
        )
    )
    await client.send_transaction(
        txn, payer, opts=TxOpts(skip_confirmation=False, preflight_commitment=Confirmed))


async def increase_validator_stake(
    client: AsyncClient, payer: Keypair, staker: Keypair, stake_pool_address: PublicKey,
    validator_vote: PublicKey, lamports: int
):
    resp = await client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    stake_pool = StakePool.decode(data[0], data[1])

    resp = await client.get_account_info(stake_pool.validator_list, commitment=Confirmed)
    data = resp['result']['value']['data']
    validator_list = ValidatorList.decode(data[0], data[1])
    (withdraw_authority, seed) = find_withdraw_authority_program_address(STAKE_POOL_PROGRAM_ID, stake_pool_address)

    validator_info = next(x for x in validator_list.validators if x.vote_account_address == validator_vote)
    transient_stake_seed = validator_info.transient_seed_suffix_start + 1  # bump up by one to avoid reuse
    (transient_stake, _) = find_transient_stake_program_address(
        STAKE_POOL_PROGRAM_ID,
        validator_info.vote_account_address,
        stake_pool_address,
        transient_stake_seed,
    )

    txn = Transaction()
    txn.add(
        sp.increase_validator_stake(
            sp.IncreaseValidatorStakeParams(
                program_id=STAKE_POOL_PROGRAM_ID,
                stake_pool=stake_pool_address,
                staker=staker.public_key,
                withdraw_authority=withdraw_authority,
                validator_list=stake_pool.validator_list,
                reserve_stake=stake_pool.reserve_stake,
                transient_stake=transient_stake,
                validator_vote=validator_vote,
                clock_sysvar=SYSVAR_CLOCK_PUBKEY,
                rent_sysvar=SYSVAR_RENT_PUBKEY,
                stake_history_sysvar=SYSVAR_STAKE_HISTORY_PUBKEY,
                stake_config_sysvar=SYSVAR_STAKE_CONFIG_ID,
                system_program_id=sys.SYS_PROGRAM_ID,
                stake_program_id=STAKE_PROGRAM_ID,
                lamports=lamports,
                transient_stake_seed=transient_stake_seed,
            )
        )
    )

    signers = [payer, staker] if payer != staker else [payer]
    await client.send_transaction(
        txn, *signers, opts=TxOpts(skip_confirmation=False, preflight_commitment=Confirmed))


async def decrease_validator_stake(
    client: AsyncClient, payer: Keypair, staker: Keypair, stake_pool_address: PublicKey,
    validator_vote: PublicKey, lamports: int
):
    resp = await client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp['result']['value']['data']
    stake_pool = StakePool.decode(data[0], data[1])

    resp = await client.get_account_info(stake_pool.validator_list, commitment=Confirmed)
    data = resp['result']['value']['data']
    validator_list = ValidatorList.decode(data[0], data[1])
    (withdraw_authority, seed) = find_withdraw_authority_program_address(STAKE_POOL_PROGRAM_ID, stake_pool_address)

    validator_info = next(x for x in validator_list.validators if x.vote_account_address == validator_vote)
    (validator_stake, _) = find_stake_program_address(
        STAKE_POOL_PROGRAM_ID,
        validator_info.vote_account_address,
        stake_pool_address,
    )
    transient_stake_seed = validator_info.transient_seed_suffix_start + 1  # bump up by one to avoid reuse
    (transient_stake, _) = find_transient_stake_program_address(
        STAKE_POOL_PROGRAM_ID,
        validator_info.vote_account_address,
        stake_pool_address,
        transient_stake_seed,
    )

    txn = Transaction()
    txn.add(
        sp.decrease_validator_stake(
            sp.DecreaseValidatorStakeParams(
                program_id=STAKE_POOL_PROGRAM_ID,
                stake_pool=stake_pool_address,
                staker=staker.public_key,
                withdraw_authority=withdraw_authority,
                validator_list=stake_pool.validator_list,
                validator_stake=validator_stake,
                transient_stake=transient_stake,
                clock_sysvar=SYSVAR_CLOCK_PUBKEY,
                rent_sysvar=SYSVAR_RENT_PUBKEY,
                system_program_id=sys.SYS_PROGRAM_ID,
                stake_program_id=STAKE_PROGRAM_ID,
                lamports=lamports,
                transient_stake_seed=transient_stake_seed,
            )
        )
    )

    signers = [payer, staker] if payer != staker else [payer]
    await client.send_transaction(
        txn, *signers, opts=TxOpts(skip_confirmation=False, preflight_commitment=Confirmed))
