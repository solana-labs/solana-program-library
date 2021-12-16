import asyncio
import pytest
import os
import shutil
import tempfile
import time
from typing import Iterator, List, Tuple
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


@pytest.fixture
def validators(event_loop, async_client, payer) -> List[PublicKey]:
    num_validators = 3
    validators = []
    futures = []
    for i in range(num_validators):
        vote = Keypair()
        node = Keypair()
        futures.append(create_vote(async_client, payer, vote, node, payer.public_key, payer.public_key, 10))
        validators.append(vote.public_key)
    event_loop.run_until_complete(asyncio.gather(*futures))
    return validators


@pytest.fixture
def stake_pool_addresses(event_loop, async_client, payer, validators) -> Tuple[PublicKey, PublicKey]:
    fee = Fee(numerator=1, denominator=1000)
    referral_fee = 20
    stake_pool_addresses = event_loop.run_until_complete(
        create_all(async_client, payer, fee, referral_fee)
    )
    futures = [
        add_validator_to_pool(async_client, payer, stake_pool_addresses[0], validator)
        for validator in validators
    ]
    event_loop.run_until_complete(asyncio.gather(*futures))
    return stake_pool_addresses


@pytest.fixture
def event_loop():
    loop = asyncio.get_event_loop()
    yield loop
    loop.close()


@pytest.fixture
def async_client(event_loop, solana_test_validator) -> Iterator[AsyncClient]:
    async_client = AsyncClient(commitment=Confirmed)
    total_attempts = 10
    current_attempt = 0
    while not event_loop.run_until_complete(async_client.is_connected()):
        if current_attempt == total_attempts:
            raise Exception("Could not connect to test validator")
        else:
            current_attempt += 1
        time.sleep(1)
    yield async_client
    event_loop.run_until_complete(async_client.close())


@pytest.fixture
def payer(event_loop, async_client) -> Keypair:
    payer = Keypair()
    airdrop_lamports = 10_000_000_000
    event_loop.run_until_complete(airdrop(async_client, payer.public_key, airdrop_lamports))
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
