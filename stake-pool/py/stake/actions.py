from solana.publickey import PublicKey
from solana.keypair import Keypair
from solana.rpc.async_api import AsyncClient
from solana.rpc.commitment import Confirmed
from solana.rpc.types import TxOpts
from solana.transaction import Transaction
import solana.system_program as sys

from stake.constants import STAKE_LEN, STAKE_PROGRAM_ID
from stake.state import Authorized, Lockup
import stake.instructions as st


async def create_stake(client: AsyncClient, payer: Keypair, stake: Keypair, authority: PublicKey):
    print(f"Creating stake {stake.public_key}")
    resp = await client.get_minimum_balance_for_rent_exemption(STAKE_LEN)
    txn = Transaction()
    txn.add(
        sys.create_account(
            sys.CreateAccountParams(
                from_pubkey=payer.public_key,
                new_account_pubkey=stake.public_key,
                lamports=resp['result'] + 1,  # add one more lamport
                space=STAKE_LEN,
                program_id=STAKE_PROGRAM_ID,
            )
        )
    )
    txn.add(
        st.initialize(
            st.InitializeParams(
                stake=stake.public_key,
                authorized=Authorized(
                    staker=authority,
                    withdrawer=authority,
                ),
                lockup=Lockup(
                    unix_timestamp=0,
                    epoch=0,
                    custodian=sys.SYS_PROGRAM_ID,
                )
            )
        )
    )
    await client.send_transaction(
        txn, payer, stake, opts=TxOpts(skip_confirmation=False, preflight_commitment=Confirmed))
