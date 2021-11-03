from solana.keypair import Keypair
from solana.publickey import PublicKey
from solana.rpc.async_api import AsyncClient
from solana.rpc.commitment import Confirmed
from solana.rpc.types import TxOpts
from solana.transaction import Transaction
import solana.system_program as sys

from spl.token.constants import TOKEN_PROGRAM_ID

from stake_pool.constants import STAKE_POOL_PROGRAM_ID
from stake_pool.state import STAKE_POOL_LAYOUT, calculate_validator_list_size, Fee
import stake_pool.instructions as sp


async def create(client: AsyncClient, manager: Keypair,
                 stake_pool: Keypair, validator_list: Keypair,
                 pool_mint: PublicKey, reserve_stake: PublicKey,
                 manager_fee_account: PublicKey, fee: Fee, referral_fee: int):
    print(f"Creating stake pool {stake_pool.public_key}")
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
    print(f"Creating validator list {validator_list.public_key}")
    max_validators = 3950  # current supported max by the program, go big!
    validator_list_size = calculate_validator_list_size(max_validators)
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

    print("Initializing stake pool...")
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
