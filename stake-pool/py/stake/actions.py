from solana.publickey import PublicKey
from solana.keypair import Keypair
from solana.rpc.async_api import AsyncClient
from solana.rpc.commitment import Confirmed
from solana.rpc.types import TxOpts
from solana.sysvar import SYSVAR_CLOCK_PUBKEY, SYSVAR_STAKE_HISTORY_PUBKEY
from solana.transaction import Transaction
import solana.system_program as sys

from stake.constants import STAKE_LEN, STAKE_PROGRAM_ID, SYSVAR_STAKE_CONFIG_ID
from stake.state import Authorized, Lockup, StakeAuthorize
import stake.instructions as st


async def create_stake(client: AsyncClient, payer: Keypair, stake: Keypair, authority: PublicKey, lamports: int):
    print(f"Creating stake {stake.public_key}")
    resp = await client.get_minimum_balance_for_rent_exemption(STAKE_LEN)
    txn = Transaction()
    txn.add(
        sys.create_account(
            sys.CreateAccountParams(
                from_pubkey=payer.public_key,
                new_account_pubkey=stake.public_key,
                lamports=resp['result'] + lamports,
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


async def delegate_stake(client: AsyncClient, payer: Keypair, staker: Keypair, stake: PublicKey, vote: PublicKey):
    txn = Transaction()
    txn.add(
        st.delegate_stake(
            st.DelegateStakeParams(
                stake=stake,
                vote=vote,
                clock_sysvar=SYSVAR_CLOCK_PUBKEY,
                stake_history_sysvar=SYSVAR_STAKE_HISTORY_PUBKEY,
                stake_config_id=SYSVAR_STAKE_CONFIG_ID,
                staker=staker.public_key,
            )
        )
    )
    signers = [payer, staker] if payer != staker else [payer]
    await client.send_transaction(
        txn, *signers, opts=TxOpts(skip_confirmation=False, preflight_commitment=Confirmed))


async def authorize(
    client: AsyncClient, payer: Keypair, authority: Keypair, stake: PublicKey,
    new_authority: PublicKey, stake_authorize: StakeAuthorize
):
    txn = Transaction()
    txn.add(
        st.authorize(
            st.AuthorizeParams(
                stake=stake,
                clock_sysvar=SYSVAR_CLOCK_PUBKEY,
                authority=authority.public_key,
                new_authority=new_authority,
                stake_authorize=stake_authorize,
            )
        )
    )
    signers = [payer, authority] if payer != authority else [payer]
    await client.send_transaction(
        txn, *signers, opts=TxOpts(skip_confirmation=False, preflight_commitment=Confirmed))
