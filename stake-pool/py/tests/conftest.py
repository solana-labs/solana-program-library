import asyncio
import pytest
import pytest_asyncio
import os
import shutil
import tempfile
from typing import AsyncIterator, List, Tuple
from subprocess import Popen

from solana.keypair import Keypair
from solana.publickey import PublicKey
from solana.rpc.async_api import AsyncClient
from solana.rpc.commitment import Confirmed

from vote.actions import create_vote
from system.actions import airdrop
from stake_pool.actions import create_all, add_validator_to_pool
from stake_pool.state import Fee

NUM_SLOTS_PER_EPOCH: int = 32


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
        "--slots-per-epoch", str(NUM_SLOTS_PER_EPOCH),
    ],)
    yield
    validator.kill()
    os.chdir(old_cwd)
    shutil.rmtree(newpath)


@pytest_asyncio.fixture
async def validators(async_client, payer) -> List[PublicKey]:
    num_validators = 3
    validators = []
    for i in range(num_validators):
        vote = Keypair()
        node = Keypair()
        await create_vote(async_client, payer, vote, node, payer.public_key, payer.public_key, 10)
        validators.append(vote.public_key)
    return validators


@pytest_asyncio.fixture
async def stake_pool_addresses(async_client, payer, validators, waiter) -> Tuple[PublicKey, PublicKey]:
    fee = Fee(numerator=1, denominator=1000)
    referral_fee = 20
    # Change back to `wait_for_next_epoch_if_soon` once https://github.com/solana-labs/solana/pull/26851 is available
    await waiter.wait_for_next_epoch(async_client)
    stake_pool_addresses = await create_all(async_client, payer, fee, referral_fee)
    for validator in validators:
        await add_validator_to_pool(async_client, payer, stake_pool_addresses[0], validator)
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
    airdrop_lamports = 20_000_000_000
    await airdrop(async_client, payer.public_key, airdrop_lamports)
    return payer


class Waiter:
    @staticmethod
    async def wait_for_next_epoch(async_client: AsyncClient):
        resp = await async_client.get_epoch_info(commitment=Confirmed)
        current_epoch = resp['result']['epoch']
        next_epoch = current_epoch
        while current_epoch == next_epoch:
            await asyncio.sleep(1.0)
            resp = await async_client.get_epoch_info(commitment=Confirmed)
            next_epoch = resp['result']['epoch']

    @staticmethod
    async def wait_for_next_epoch_if_soon(async_client: AsyncClient):
        resp = await async_client.get_epoch_info(commitment=Confirmed)
        if resp['result']['slotsInEpoch'] - resp['result']['slotIndex'] < NUM_SLOTS_PER_EPOCH // 2:
            await Waiter.wait_for_next_epoch(async_client)
            return True
        else:
            return False


@pytest.fixture
def waiter() -> Waiter:
    return Waiter()
