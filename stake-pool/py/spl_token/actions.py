from solana.publickey import PublicKey
from solana.keypair import Keypair
from solana.rpc.async_api import AsyncClient
from solana.rpc.commitment import Confirmed
from solana.rpc.types import TxOpts
from solana.transaction import Transaction
import solana.system_program as sys

from spl.token.constants import TOKEN_PROGRAM_ID
from spl.token.async_client import AsyncToken
from spl.token._layouts import MINT_LAYOUT
import spl.token.instructions as spl_token


async def create_associated_token_account(
    client: AsyncClient,
    payer: Keypair,
    owner: PublicKey,
    mint: PublicKey
) -> PublicKey:
    txn = Transaction()
    create_txn = spl_token.create_associated_token_account(
        payer=payer.public_key, owner=owner, mint=mint
    )
    txn.add(create_txn)
    await client.send_transaction(txn, payer, opts=TxOpts(skip_confirmation=False, preflight_commitment=Confirmed))
    return create_txn.keys[1].pubkey


async def create_mint(client: AsyncClient, payer: Keypair, mint: Keypair, mint_authority: PublicKey):
    mint_balance = await AsyncToken.get_min_balance_rent_for_exempt_for_mint(client)
    print(f"Creating pool token mint {mint.public_key}")
    txn = Transaction()
    txn.add(
        sys.create_account(
            sys.CreateAccountParams(
                from_pubkey=payer.public_key,
                new_account_pubkey=mint.public_key,
                lamports=mint_balance,
                space=MINT_LAYOUT.sizeof(),
                program_id=TOKEN_PROGRAM_ID,
            )
        )
    )
    txn.add(
        spl_token.initialize_mint(
            spl_token.InitializeMintParams(
                program_id=TOKEN_PROGRAM_ID,
                mint=mint.public_key,
                decimals=9,
                mint_authority=mint_authority,
                freeze_authority=None,
            )
        )
    )
    await client.send_transaction(
        txn, payer, mint, opts=TxOpts(skip_confirmation=False, preflight_commitment=Confirmed))
