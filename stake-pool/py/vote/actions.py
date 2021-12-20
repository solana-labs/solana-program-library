from solana.publickey import PublicKey
from solana.keypair import Keypair
from solana.rpc.async_api import AsyncClient
from solana.rpc.commitment import Confirmed
from solana.rpc.types import TxOpts
from solana.sysvar import SYSVAR_CLOCK_PUBKEY, SYSVAR_RENT_PUBKEY
from solana.transaction import Transaction
import solana.system_program as sys

from vote.constants import VOTE_PROGRAM_ID, VOTE_STATE_LEN
from vote.instructions import initialize, InitializeParams


async def create_vote(
        client: AsyncClient, payer: Keypair, vote: Keypair, node: Keypair,
        voter: PublicKey, withdrawer: PublicKey, commission: int):
    print(f"Creating vote account {vote.public_key}")
    resp = await client.get_minimum_balance_for_rent_exemption(VOTE_STATE_LEN)
    txn = Transaction()
    txn.add(
        sys.create_account(
            sys.CreateAccountParams(
                from_pubkey=payer.public_key,
                new_account_pubkey=vote.public_key,
                lamports=resp['result'],
                space=VOTE_STATE_LEN,
                program_id=VOTE_PROGRAM_ID,
            )
        )
    )
    txn.add(
        initialize(
            InitializeParams(
                vote=vote.public_key,
                rent_sysvar=SYSVAR_RENT_PUBKEY,
                clock_sysvar=SYSVAR_CLOCK_PUBKEY,
                node=node.public_key,
                authorized_voter=voter,
                authorized_withdrawer=withdrawer,
                commission=commission,
            )
        )
    )
    await client.send_transaction(
        txn, payer, vote, node, opts=TxOpts(skip_confirmation=False, preflight_commitment=Confirmed))
