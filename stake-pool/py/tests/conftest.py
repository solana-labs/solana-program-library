import asyncio
import pytest
import os
import shutil
import tempfile
import time
from typing import Iterator
from subprocess import run, Popen

from solana.publickey import PublicKey
from solana.rpc.async_api import AsyncClient
from solana.rpc.commitment import Confirmed


@pytest.fixture(scope="session")
def solana_test_validator():
    old_cwd = os.getcwd()
    newpath = tempfile.mkdtemp()
    os.chdir(newpath)
    validator = Popen(["solana-test-validator", "--reset", "--quiet",
                       "--bpf-program", "SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy",
                       f"{old_cwd}/../../target/deploy/spl_stake_pool.so"],)
    yield
    validator.kill()
    os.chdir(old_cwd)
    shutil.rmtree(newpath)


@pytest.fixture
def validators(async_client):
    num_validators = 3
    validators = []
    for i in range(num_validators):
        tf = tempfile.NamedTemporaryFile()
        identity = f"{tf.name}-identity-{i}.json"
        run(["solana-keygen", "new", "-s", "-o", identity])
        vote = f"{tf.name}-vote-{i}.json"
        run(["solana-keygen", "new", "-s", "-o", vote])
        withdrawer = f"{tf.name}-withdrawer-{i}.json"
        run(["solana-keygen", "new", "-s", "-o", withdrawer])
        run(["solana", "create-vote-account",
             vote, identity, withdrawer,
             "--commission", "1",
             "--commitment", "confirmed",
             "-ul"])
        output = run(["solana-keygen", "pubkey", vote], capture_output=True)
        validators.append(PublicKey(output.stdout.decode('utf-8').strip()))
    return validators


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
