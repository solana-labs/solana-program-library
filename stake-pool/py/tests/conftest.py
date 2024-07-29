import asyncio
import pytest
import pytest_asyncio
import os
import shutil
import tempfile
from typing import AsyncIterator, List, Tuple
from subprocess import Popen

from solders.keypair import Keypair
from solders.pubkey import Pubkey
from solana.rpc.async_api import AsyncClient
from solana.rpc.commitment import Confirmed

from spl.token.instructions import get_associated_token_address

from vote.actions import create_vote
from system.actions import airdrop
from stake_pool.actions import deposit_sol, create_all, add_validator_to_pool
from stake_pool.state import Fee

NUM_SLOTS_PER_EPOCH: int = 32
AIRDROP_LAMPORTS: int = 30_000_000_000


@pytest.fixture(scope="session")
def solana_test_validator():
    old_cwd = os.getcwd()
    newpath = tempfile.mkdtemp()
    os.chdir(newpath)
    validator = Popen([
        "solana-test-validator",
        "--reset", "--quiet",
        "--bpf-program", "SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy",
        f"{old_cwd}/../../target/deploy/spl_stake_pool.so",
        "--bpf-program", "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s",
        f"{old_cwd}/../program/tests/fixtures/mpl_token_metadata.so",
        "--slots-per-epoch", str(NUM_SLOTS_PER_EPOCH),
    ],)
    yield
    validator.kill()
    os.chdir(old_cwd)
    shutil.rmtree(newpath)


@pytest_asyncio.fixture
async def validators(async_client, payer) -> List[Pubkey]:
    num_validators = 3
    validators = []
    for i in range(num_validators):
        vote = Keypair()
        node = Keypair()
        await create_vote(async_client, payer, vote, node, payer.pubkey(), payer.pubkey(), 10)
        validators.append(vote.pubkey())
    return validators


@pytest_asyncio.fixture
async def stake_pool_addresses(
    async_client, payer, validators, waiter
) -> Tuple[Pubkey, Pubkey, Pubkey]:
    fee = Fee(numerator=1, denominator=1000)
    referral_fee = 20
    await waiter.wait_for_next_epoch_if_soon(async_client)
    stake_pool_addresses = await create_all(async_client, payer, fee, referral_fee)
    stake_pool = stake_pool_addresses[0]
    pool_mint = stake_pool_addresses[2]
    token_account = get_associated_token_address(payer.pubkey(), pool_mint)
    await deposit_sol(async_client, payer, stake_pool, token_account, AIRDROP_LAMPORTS // 2)
    for validator in validators:
        await add_validator_to_pool(async_client, payer, stake_pool, validator)
    return stake_pool_addresses


@pytest_asyncio.fixture
async def async_client(solana_test_validator) -> AsyncIterator[AsyncClient]:
    async_client = AsyncClient(commitment=Confirmed)
    total_attempts = 20
    current_attempt = 0
    while not await async_client.is_connected():
        if current_attempt == total_attempts:
            raise Exception("Could not connect to test validator")
        else:
            current_attempt += 1
        await asyncio.sleep(1.0)
    yield async_client
    await async_client.close()


@pytest_asyncio.fixture
async def payer(async_client) -> Keypair:
    payer = Keypair()
    await airdrop(async_client, payer.pubkey(), AIRDROP_LAMPORTS)
    return payer


class Waiter:
    @staticmethod
    async def wait_for_next_epoch(async_client: AsyncClient):
        resp = await async_client.get_epoch_info(commitment=Confirmed)
        current_epoch = resp.value.epoch
        next_epoch = current_epoch
        while current_epoch == next_epoch:
            await asyncio.sleep(1.0)
            resp = await async_client.get_epoch_info(commitment=Confirmed)
            next_epoch = resp.value.epoch
        await asyncio.sleep(0.4)  # wait one more block to avoid reward payout time

    @staticmethod
    async def wait_for_next_epoch_if_soon(async_client: AsyncClient):
        resp = await async_client.get_epoch_info(commitment=Confirmed)
        if resp.value.slots_in_epoch - resp.value.slot_index < NUM_SLOTS_PER_EPOCH // 2:
            await Waiter.wait_for_next_epoch(async_client)
            return True
        else:
            return False


@pytest.fixture
def waiter() -> Waiter:
    return Waiter()
