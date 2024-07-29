from solders.pubkey import Pubkey
from solana.rpc.async_api import AsyncClient


async def airdrop(client: AsyncClient, receiver: Pubkey, lamports: int):
    print(f"Airdropping {lamports} lamports to {receiver}...")
    resp = await client.request_airdrop(receiver, lamports)
    await client.confirm_transaction(resp.value)
