from solders.pubkey import Pubkey
from solders.keypair import Keypair
from solana.rpc.async_api import AsyncClient
from solana.rpc.commitment import Confirmed
from solana.rpc.types import TxOpts
from solana.transaction import Transaction
import solders.system_program as sys

from spl.token.constants import TOKEN_PROGRAM_ID
from spl.token.async_client import AsyncToken
from spl.token._layouts import MINT_LAYOUT
import spl.token.instructions as spl_token


OPTS = TxOpts(skip_confirmation=False, preflight_commitment=Confirmed)


async def create_associated_token_account(
    client: AsyncClient,
    payer: Keypair,
    owner: Pubkey,
    mint: Pubkey
) -> Pubkey:
    txn = Transaction(fee_payer=payer.pubkey())
    create_txn = spl_token.create_associated_token_account(
        payer=payer.pubkey(), owner=owner, mint=mint
    )
    txn.add(create_txn)
    recent_blockhash = (await client.get_latest_blockhash()).value.blockhash
    await client.send_transaction(txn, payer, recent_blockhash=recent_blockhash, opts=OPTS)
    return create_txn.accounts[1].pubkey


async def create_mint(client: AsyncClient, payer: Keypair, mint: Keypair, mint_authority: Pubkey):
    mint_balance = await AsyncToken.get_min_balance_rent_for_exempt_for_mint(client)
    print(f"Creating pool token mint {mint.pubkey()}")
    txn = Transaction(fee_payer=payer.pubkey())
    txn.add(
        sys.create_account(
            sys.CreateAccountParams(
                from_pubkey=payer.pubkey(),
                to_pubkey=mint.pubkey(),
                lamports=mint_balance,
                space=MINT_LAYOUT.sizeof(),
                owner=TOKEN_PROGRAM_ID,
            )
        )
    )
    txn.add(
        spl_token.initialize_mint(
            spl_token.InitializeMintParams(
                program_id=TOKEN_PROGRAM_ID,
                mint=mint.pubkey(),
                decimals=9,
                mint_authority=mint_authority,
                freeze_authority=None,
            )
        )
    )
    recent_blockhash = (await client.get_latest_blockhash()).value.blockhash
    await client.send_transaction(
        txn, payer, mint, recent_blockhash=recent_blockhash, opts=OPTS)
