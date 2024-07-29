from typing import Optional, Tuple

from solders.keypair import Keypair
from solders.pubkey import Pubkey
from solana.rpc.async_api import AsyncClient
from solana.rpc.commitment import Confirmed
from solana.rpc.types import TxOpts
from solders.sysvar import CLOCK, RENT, STAKE_HISTORY
from solana.transaction import Transaction
import solders.system_program as sys

from spl.token.constants import TOKEN_PROGRAM_ID

from stake.constants import STAKE_PROGRAM_ID, STAKE_LEN, SYSVAR_STAKE_CONFIG_ID
import stake.instructions as st
from stake.state import StakeAuthorize
from stake_pool.constants import \
    MAX_VALIDATORS_TO_UPDATE, \
    MINIMUM_RESERVE_LAMPORTS, \
    STAKE_POOL_PROGRAM_ID, \
    METADATA_PROGRAM_ID, \
    find_stake_program_address, \
    find_transient_stake_program_address, \
    find_withdraw_authority_program_address, \
    find_metadata_account, \
    find_ephemeral_stake_program_address
from stake_pool.state import STAKE_POOL_LAYOUT, ValidatorList, Fee, StakePool
import stake_pool.instructions as sp

from stake.actions import create_stake
from spl_token.actions import create_mint, create_associated_token_account


OPTS = TxOpts(skip_confirmation=False, preflight_commitment=Confirmed)


async def create(client: AsyncClient, manager: Keypair,
                 stake_pool: Keypair, validator_list: Keypair,
                 pool_mint: Pubkey, reserve_stake: Pubkey,
                 manager_fee_account: Pubkey, fee: Fee, referral_fee: int):
    resp = await client.get_minimum_balance_for_rent_exemption(STAKE_POOL_LAYOUT.sizeof())
    pool_balance = resp.value
    txn = Transaction(fee_payer=manager.pubkey())
    txn.add(
        sys.create_account(
            sys.CreateAccountParams(
                from_pubkey=manager.pubkey(),
                to_pubkey=stake_pool.pubkey(),
                lamports=pool_balance,
                space=STAKE_POOL_LAYOUT.sizeof(),
                owner=STAKE_POOL_PROGRAM_ID,
            )
        )
    )
    max_validators = 2950  # current supported max by the program, go big!
    validator_list_size = ValidatorList.calculate_validator_list_size(max_validators)
    resp = await client.get_minimum_balance_for_rent_exemption(validator_list_size)
    validator_list_balance = resp.value
    txn.add(
        sys.create_account(
            sys.CreateAccountParams(
                from_pubkey=manager.pubkey(),
                to_pubkey=validator_list.pubkey(),
                lamports=validator_list_balance,
                space=validator_list_size,
                owner=STAKE_POOL_PROGRAM_ID,
            )
        )
    )
    recent_blockhash = (await client.get_latest_blockhash()).value.blockhash
    await client.send_transaction(
        txn, manager, stake_pool, validator_list, recent_blockhash=recent_blockhash, opts=OPTS)

    (withdraw_authority, seed) = find_withdraw_authority_program_address(
        STAKE_POOL_PROGRAM_ID, stake_pool.pubkey())
    txn = Transaction(fee_payer=manager.pubkey())
    txn.add(
        sp.initialize(
            sp.InitializeParams(
                program_id=STAKE_POOL_PROGRAM_ID,
                stake_pool=stake_pool.pubkey(),
                manager=manager.pubkey(),
                staker=manager.pubkey(),
                withdraw_authority=withdraw_authority,
                validator_list=validator_list.pubkey(),
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
    recent_blockhash = (await client.get_latest_blockhash()).value.blockhash
    await client.send_transaction(txn, manager, recent_blockhash=recent_blockhash, opts=OPTS)


async def create_all(
        client: AsyncClient, manager: Keypair, fee: Fee, referral_fee: int
) -> Tuple[Pubkey, Pubkey, Pubkey]:
    stake_pool = Keypair()
    validator_list = Keypair()
    (pool_withdraw_authority, seed) = find_withdraw_authority_program_address(
        STAKE_POOL_PROGRAM_ID, stake_pool.pubkey())

    reserve_stake = Keypair()
    await create_stake(client, manager, reserve_stake, pool_withdraw_authority, MINIMUM_RESERVE_LAMPORTS)

    pool_mint = Keypair()
    await create_mint(client, manager, pool_mint, pool_withdraw_authority)

    manager_fee_account = await create_associated_token_account(
        client,
        manager,
        manager.pubkey(),
        pool_mint.pubkey(),
    )

    fee = Fee(numerator=1, denominator=1000)
    referral_fee = 20
    await create(
        client, manager, stake_pool, validator_list, pool_mint.pubkey(),
        reserve_stake.pubkey(), manager_fee_account, fee, referral_fee)
    return (stake_pool.pubkey(), validator_list.pubkey(), pool_mint.pubkey())


async def add_validator_to_pool(
    client: AsyncClient, staker: Keypair,
    stake_pool_address: Pubkey, validator: Pubkey
):
    resp = await client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    stake_pool = StakePool.decode(data)
    txn = Transaction(fee_payer=staker.pubkey())
    txn.add(
        sp.add_validator_to_pool_with_vote(
            STAKE_POOL_PROGRAM_ID,
            stake_pool_address,
            stake_pool.staker,
            stake_pool.validator_list,
            stake_pool.reserve_stake,
            validator,
            None,
        )
    )
    recent_blockhash = (await client.get_latest_blockhash()).value.blockhash
    await client.send_transaction(txn, staker, recent_blockhash=recent_blockhash, opts=OPTS)


async def remove_validator_from_pool(
    client: AsyncClient, staker: Keypair,
    stake_pool_address: Pubkey, validator: Pubkey
):
    resp = await client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    stake_pool = StakePool.decode(data)
    resp = await client.get_account_info(stake_pool.validator_list, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    validator_list = ValidatorList.decode(data)
    validator_info = next(x for x in validator_list.validators if x.vote_account_address == validator)
    txn = Transaction(fee_payer=staker.pubkey())
    txn.add(
        sp.remove_validator_from_pool_with_vote(
            STAKE_POOL_PROGRAM_ID,
            stake_pool_address,
            stake_pool.staker,
            stake_pool.validator_list,
            validator,
            validator_info.validator_seed_suffix or None,
            validator_info.transient_seed_suffix,
        )
    )
    recent_blockhash = (await client.get_latest_blockhash()).value.blockhash
    await client.send_transaction(txn, staker, recent_blockhash=recent_blockhash, opts=OPTS)


async def deposit_sol(
    client: AsyncClient, funder: Keypair, stake_pool_address: Pubkey,
    destination_token_account: Pubkey, amount: int,
):
    resp = await client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    stake_pool = StakePool.decode(data)

    (withdraw_authority, seed) = find_withdraw_authority_program_address(STAKE_POOL_PROGRAM_ID, stake_pool_address)

    txn = Transaction(fee_payer=funder.pubkey())
    txn.add(
        sp.deposit_sol(
            sp.DepositSolParams(
                program_id=STAKE_POOL_PROGRAM_ID,
                stake_pool=stake_pool_address,
                withdraw_authority=withdraw_authority,
                reserve_stake=stake_pool.reserve_stake,
                funding_account=funder.pubkey(),
                destination_pool_account=destination_token_account,
                manager_fee_account=stake_pool.manager_fee_account,
                referral_pool_account=destination_token_account,
                pool_mint=stake_pool.pool_mint,
                system_program_id=sys.ID,
                token_program_id=stake_pool.token_program_id,
                amount=amount,
                deposit_authority=None,
            )
        )
    )
    recent_blockhash = (await client.get_latest_blockhash()).value.blockhash
    await client.send_transaction(txn, funder, recent_blockhash=recent_blockhash, opts=OPTS)


async def withdraw_sol(
    client: AsyncClient, owner: Keypair, source_token_account: Pubkey,
    stake_pool_address: Pubkey, destination_system_account: Pubkey, amount: int,
):
    resp = await client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    stake_pool = StakePool.decode(data)

    (withdraw_authority, seed) = find_withdraw_authority_program_address(STAKE_POOL_PROGRAM_ID, stake_pool_address)

    txn = Transaction(fee_payer=owner.pubkey())
    txn.add(
        sp.withdraw_sol(
            sp.WithdrawSolParams(
                program_id=STAKE_POOL_PROGRAM_ID,
                stake_pool=stake_pool_address,
                withdraw_authority=withdraw_authority,
                source_transfer_authority=owner.pubkey(),
                source_pool_account=source_token_account,
                reserve_stake=stake_pool.reserve_stake,
                destination_system_account=destination_system_account,
                manager_fee_account=stake_pool.manager_fee_account,
                pool_mint=stake_pool.pool_mint,
                clock_sysvar=CLOCK,
                stake_history_sysvar=STAKE_HISTORY,
                stake_program_id=STAKE_PROGRAM_ID,
                token_program_id=stake_pool.token_program_id,
                amount=amount,
                sol_withdraw_authority=None,
            )
        )
    )
    recent_blockhash = (await client.get_latest_blockhash()).value.blockhash
    await client.send_transaction(txn, owner, recent_blockhash=recent_blockhash, opts=OPTS)


async def deposit_stake(
    client: AsyncClient,
    deposit_stake_authority: Keypair,
    stake_pool_address: Pubkey,
    validator_vote: Pubkey,
    deposit_stake: Pubkey,
    destination_pool_account: Pubkey,
):
    resp = await client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    stake_pool = StakePool.decode(data)

    resp = await client.get_account_info(stake_pool.validator_list, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    validator_list = ValidatorList.decode(data)

    validator_info = next(x for x in validator_list.validators if x.vote_account_address == validator_vote)

    (withdraw_authority, _) = find_withdraw_authority_program_address(STAKE_POOL_PROGRAM_ID, stake_pool_address)
    (validator_stake, _) = find_stake_program_address(
        STAKE_POOL_PROGRAM_ID,
        validator_vote,
        stake_pool_address,
        validator_info.validator_seed_suffix or None,
    )

    txn = Transaction(fee_payer=deposit_stake_authority.pubkey())
    txn.add(
        st.authorize(
            st.AuthorizeParams(
                stake=deposit_stake,
                clock_sysvar=CLOCK,
                authority=deposit_stake_authority.pubkey(),
                new_authority=stake_pool.stake_deposit_authority,
                stake_authorize=StakeAuthorize.STAKER,
            )
        )
    )
    txn.add(
        st.authorize(
            st.AuthorizeParams(
                stake=deposit_stake,
                clock_sysvar=CLOCK,
                authority=deposit_stake_authority.pubkey(),
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
                clock_sysvar=CLOCK,
                stake_history_sysvar=STAKE_HISTORY,
                token_program_id=stake_pool.token_program_id,
                stake_program_id=STAKE_PROGRAM_ID,
            )
        )
    )
    recent_blockhash = (await client.get_latest_blockhash()).value.blockhash
    await client.send_transaction(txn, deposit_stake_authority, recent_blockhash=recent_blockhash, opts=OPTS)


async def withdraw_stake(
    client: AsyncClient,
    payer: Keypair,
    source_transfer_authority: Keypair,
    destination_stake: Keypair,
    stake_pool_address: Pubkey,
    validator_vote: Pubkey,
    destination_stake_authority: Pubkey,
    source_pool_account: Pubkey,
    amount: int,
):
    resp = await client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    stake_pool = StakePool.decode(data)

    resp = await client.get_account_info(stake_pool.validator_list, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    validator_list = ValidatorList.decode(data)

    validator_info = next(x for x in validator_list.validators if x.vote_account_address == validator_vote)

    (withdraw_authority, _) = find_withdraw_authority_program_address(STAKE_POOL_PROGRAM_ID, stake_pool_address)
    (validator_stake, _) = find_stake_program_address(
        STAKE_POOL_PROGRAM_ID,
        validator_vote,
        stake_pool_address,
        validator_info.validator_seed_suffix or None,
    )

    rent_resp = await client.get_minimum_balance_for_rent_exemption(STAKE_LEN)
    stake_rent_exemption = rent_resp.value

    txn = Transaction(fee_payer=payer.pubkey())
    txn.add(
        sys.create_account(
            sys.CreateAccountParams(
                from_pubkey=payer.pubkey(),
                to_pubkey=destination_stake.pubkey(),
                lamports=stake_rent_exemption,
                space=STAKE_LEN,
                owner=STAKE_PROGRAM_ID,
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
                destination_stake=destination_stake.pubkey(),
                destination_stake_authority=destination_stake_authority,
                source_transfer_authority=source_transfer_authority.pubkey(),
                source_pool_account=source_pool_account,
                manager_fee_account=stake_pool.manager_fee_account,
                pool_mint=stake_pool.pool_mint,
                clock_sysvar=CLOCK,
                token_program_id=stake_pool.token_program_id,
                stake_program_id=STAKE_PROGRAM_ID,
                amount=amount,
            )
        )
    )
    signers = [payer, source_transfer_authority, destination_stake] \
        if payer != source_transfer_authority else [payer, destination_stake]
    recent_blockhash = (await client.get_latest_blockhash()).value.blockhash
    await client.send_transaction(txn, *signers, recent_blockhash=recent_blockhash, opts=OPTS)


async def update_stake_pool(client: AsyncClient, payer: Keypair, stake_pool_address: Pubkey):
    """Create and send all instructions to completely update a stake pool after epoch change."""
    resp = await client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    stake_pool = StakePool.decode(data)
    resp = await client.get_account_info(stake_pool.validator_list, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    validator_list = ValidatorList.decode(data)
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
                validator.validator_seed_suffix or None,
            )
            validator_and_transient_stake_pairs.append(validator_stake_address)
            (transient_stake_address, _) = find_transient_stake_program_address(
                STAKE_POOL_PROGRAM_ID,
                validator.vote_account_address,
                stake_pool_address,
                validator.transient_seed_suffix,
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
                    clock_sysvar=CLOCK,
                    stake_history_sysvar=STAKE_HISTORY,
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
            txn = Transaction(fee_payer=payer.pubkey())
            txn.add(update_list_instruction)
            recent_blockhash = (await client.get_latest_blockhash()).value.blockhash
            await client.send_transaction(txn, payer, recent_blockhash=recent_blockhash,
                                          opts=TxOpts(skip_confirmation=True, preflight_commitment=Confirmed))
        txn = Transaction(fee_payer=payer.pubkey())
        txn.add(last_instruction)
        recent_blockhash = (await client.get_latest_blockhash()).value.blockhash
        await client.send_transaction(txn, payer, recent_blockhash=recent_blockhash, opts=OPTS)
    txn = Transaction(fee_payer=payer.pubkey())
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
    recent_blockhash = (await client.get_latest_blockhash()).value.blockhash
    await client.send_transaction(txn, payer, recent_blockhash=recent_blockhash, opts=OPTS)


async def increase_validator_stake(
    client: AsyncClient,
    payer: Keypair,
    staker: Keypair,
    stake_pool_address: Pubkey,
    validator_vote: Pubkey,
    lamports: int,
    ephemeral_stake_seed: Optional[int] = None
):
    resp = await client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    stake_pool = StakePool.decode(data)

    resp = await client.get_account_info(stake_pool.validator_list, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    validator_list = ValidatorList.decode(data)
    (withdraw_authority, seed) = find_withdraw_authority_program_address(STAKE_POOL_PROGRAM_ID, stake_pool_address)

    validator_info = next(x for x in validator_list.validators if x.vote_account_address == validator_vote)

    if ephemeral_stake_seed is None:
        transient_stake_seed = validator_info.transient_seed_suffix + 1  # bump up by one to avoid reuse
    else:
        # we are updating an existing transient stake account, so we must use the same seed
        transient_stake_seed = validator_info.transient_seed_suffix

    validator_stake_seed = validator_info.validator_seed_suffix or None
    (transient_stake, _) = find_transient_stake_program_address(
        STAKE_POOL_PROGRAM_ID,
        validator_info.vote_account_address,
        stake_pool_address,
        transient_stake_seed,
    )
    (validator_stake, _) = find_stake_program_address(
        STAKE_POOL_PROGRAM_ID,
        validator_info.vote_account_address,
        stake_pool_address,
        validator_stake_seed
    )

    txn = Transaction(fee_payer=payer.pubkey())
    if ephemeral_stake_seed is not None:

        # We assume there is an existing transient account that we will update
        (ephemeral_stake, _) = find_ephemeral_stake_program_address(
            STAKE_POOL_PROGRAM_ID,
            stake_pool_address,
            ephemeral_stake_seed)

        txn.add(
            sp.increase_additional_validator_stake(
                sp.IncreaseAdditionalValidatorStakeParams(
                    program_id=STAKE_POOL_PROGRAM_ID,
                    stake_pool=stake_pool_address,
                    staker=staker.pubkey(),
                    withdraw_authority=withdraw_authority,
                    validator_list=stake_pool.validator_list,
                    reserve_stake=stake_pool.reserve_stake,
                    transient_stake=transient_stake,
                    validator_stake=validator_stake,
                    validator_vote=validator_vote,
                    clock_sysvar=CLOCK,
                    rent_sysvar=RENT,
                    stake_history_sysvar=STAKE_HISTORY,
                    stake_config_sysvar=SYSVAR_STAKE_CONFIG_ID,
                    system_program_id=sys.ID,
                    stake_program_id=STAKE_PROGRAM_ID,
                    lamports=lamports,
                    transient_stake_seed=transient_stake_seed,
                    ephemeral_stake=ephemeral_stake,
                    ephemeral_stake_seed=ephemeral_stake_seed
                )
            )
        )

    else:
        txn.add(
            sp.increase_validator_stake(
                sp.IncreaseValidatorStakeParams(
                    program_id=STAKE_POOL_PROGRAM_ID,
                    stake_pool=stake_pool_address,
                    staker=staker.pubkey(),
                    withdraw_authority=withdraw_authority,
                    validator_list=stake_pool.validator_list,
                    reserve_stake=stake_pool.reserve_stake,
                    transient_stake=transient_stake,
                    validator_stake=validator_stake,
                    validator_vote=validator_vote,
                    clock_sysvar=CLOCK,
                    rent_sysvar=RENT,
                    stake_history_sysvar=STAKE_HISTORY,
                    stake_config_sysvar=SYSVAR_STAKE_CONFIG_ID,
                    system_program_id=sys.ID,
                    stake_program_id=STAKE_PROGRAM_ID,
                    lamports=lamports,
                    transient_stake_seed=transient_stake_seed,
                )
            )
        )

    signers = [payer, staker] if payer != staker else [payer]
    recent_blockhash = (await client.get_latest_blockhash()).value.blockhash
    await client.send_transaction(txn, *signers, recent_blockhash=recent_blockhash, opts=OPTS)


async def decrease_validator_stake(
    client: AsyncClient,
    payer: Keypair,
    staker: Keypair,
    stake_pool_address: Pubkey,
    validator_vote: Pubkey,
    lamports: int,
    ephemeral_stake_seed: Optional[int] = None
):
    resp = await client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    stake_pool = StakePool.decode(data)

    resp = await client.get_account_info(stake_pool.validator_list, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    validator_list = ValidatorList.decode(data)
    (withdraw_authority, seed) = find_withdraw_authority_program_address(STAKE_POOL_PROGRAM_ID, stake_pool_address)

    validator_info = next(x for x in validator_list.validators if x.vote_account_address == validator_vote)
    validator_stake_seed = validator_info.validator_seed_suffix or None
    (validator_stake, _) = find_stake_program_address(
        STAKE_POOL_PROGRAM_ID,
        validator_info.vote_account_address,
        stake_pool_address,
        validator_stake_seed,
    )

    if ephemeral_stake_seed is None:
        transient_stake_seed = validator_info.transient_seed_suffix + 1  # bump up by one to avoid reuse
    else:
        # we are updating an existing transient stake account, so we must use the same seed
        transient_stake_seed = validator_info.transient_seed_suffix

    (transient_stake, _) = find_transient_stake_program_address(
        STAKE_POOL_PROGRAM_ID,
        validator_info.vote_account_address,
        stake_pool_address,
        transient_stake_seed,
    )

    txn = Transaction(fee_payer=payer.pubkey())

    if ephemeral_stake_seed is not None:

        # We assume there is an existing transient account that we will update
        (ephemeral_stake, _) = find_ephemeral_stake_program_address(
            STAKE_POOL_PROGRAM_ID,
            stake_pool_address,
            ephemeral_stake_seed)

        txn.add(
            sp.decrease_additional_validator_stake(
                sp.DecreaseAdditionalValidatorStakeParams(
                    program_id=STAKE_POOL_PROGRAM_ID,
                    stake_pool=stake_pool_address,
                    staker=staker.pubkey(),
                    withdraw_authority=withdraw_authority,
                    validator_list=stake_pool.validator_list,
                    reserve_stake=stake_pool.reserve_stake,
                    validator_stake=validator_stake,
                    transient_stake=transient_stake,
                    clock_sysvar=CLOCK,
                    rent_sysvar=RENT,
                    stake_history_sysvar=STAKE_HISTORY,
                    system_program_id=sys.ID,
                    stake_program_id=STAKE_PROGRAM_ID,
                    lamports=lamports,
                    transient_stake_seed=transient_stake_seed,
                    ephemeral_stake=ephemeral_stake,
                    ephemeral_stake_seed=ephemeral_stake_seed
                )
            )
        )

    else:

        txn.add(
            sp.decrease_validator_stake_with_reserve(
                sp.DecreaseValidatorStakeWithReserveParams(
                    program_id=STAKE_POOL_PROGRAM_ID,
                    stake_pool=stake_pool_address,
                    staker=staker.pubkey(),
                    withdraw_authority=withdraw_authority,
                    validator_list=stake_pool.validator_list,
                    reserve_stake=stake_pool.reserve_stake,
                    validator_stake=validator_stake,
                    transient_stake=transient_stake,
                    clock_sysvar=CLOCK,
                    stake_history_sysvar=STAKE_HISTORY,
                    system_program_id=sys.ID,
                    stake_program_id=STAKE_PROGRAM_ID,
                    lamports=lamports,
                    transient_stake_seed=transient_stake_seed,
                )
            )
        )

    signers = [payer, staker] if payer != staker else [payer]
    recent_blockhash = (await client.get_latest_blockhash()).value.blockhash
    await client.send_transaction(txn, *signers, recent_blockhash=recent_blockhash, opts=OPTS)


async def create_token_metadata(client: AsyncClient, payer: Keypair, stake_pool_address: Pubkey,
                                name: str, symbol: str, uri: str):
    resp = await client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    stake_pool = StakePool.decode(data)

    (withdraw_authority, _seed) = find_withdraw_authority_program_address(STAKE_POOL_PROGRAM_ID, stake_pool_address)
    (token_metadata, _seed) = find_metadata_account(stake_pool.pool_mint)

    txn = Transaction(fee_payer=payer.pubkey())
    txn.add(
        sp.create_token_metadata(
            sp.CreateTokenMetadataParams(
                program_id=STAKE_POOL_PROGRAM_ID,
                stake_pool=stake_pool_address,
                manager=stake_pool.manager,
                pool_mint=stake_pool.pool_mint,
                payer=payer.pubkey(),
                name=name,
                symbol=symbol,
                uri=uri,
                withdraw_authority=withdraw_authority,
                token_metadata=token_metadata,
                metadata_program_id=METADATA_PROGRAM_ID,
                system_program_id=sys.ID,
            )
        )
    )
    recent_blockhash = (await client.get_latest_blockhash()).value.blockhash
    await client.send_transaction(txn, payer, recent_blockhash=recent_blockhash, opts=OPTS)


async def update_token_metadata(client: AsyncClient, payer: Keypair, stake_pool_address: Pubkey,
                                name: str, symbol: str, uri: str):
    resp = await client.get_account_info(stake_pool_address, commitment=Confirmed)
    data = resp.value.data if resp.value else bytes()
    stake_pool = StakePool.decode(data)

    (withdraw_authority, _seed) = find_withdraw_authority_program_address(STAKE_POOL_PROGRAM_ID, stake_pool_address)
    (token_metadata, _seed) = find_metadata_account(stake_pool.pool_mint)

    txn = Transaction(fee_payer=payer.pubkey())
    txn.add(
        sp.update_token_metadata(
            sp.UpdateTokenMetadataParams(
                program_id=STAKE_POOL_PROGRAM_ID,
                stake_pool=stake_pool_address,
                manager=stake_pool.manager,
                pool_mint=stake_pool.pool_mint,
                name=name,
                symbol=symbol,
                uri=uri,
                withdraw_authority=withdraw_authority,
                token_metadata=token_metadata,
                metadata_program_id=METADATA_PROGRAM_ID,
            )
        )
    )
    recent_blockhash = (await client.get_latest_blockhash()).value.blockhash
    await client.send_transaction(txn, payer, recent_blockhash=recent_blockhash, opts=OPTS)
