from solana.publickey import PublicKey
from solana.rpc.async_api import AsyncClient
from solana.rpc.commitment import Confirmed


async def airdrop(client: AsyncClient, receiver: PublicKey, lamports: int):
    print(f"Airdropping {lamports} lamports to {receiver}...")
    resp = await client.request_airdrop(receiver, lamports, Confirmed)
    await client.confirm_transaction(resp['result'], Confirmed)
