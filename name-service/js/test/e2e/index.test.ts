import {
    Connection,
    Keypair,
    LAMPORTS_PER_SOL,
    PublicKey,
    sendAndConfirmTransaction,
    Transaction,
} from '@solana/web3.js';
import { beforeAll, beforeEach, describe, expect, test } from '@jest/globals';

import {
    createNameRegistry,
    deleteNameRegistry,
    getHashedName,
    getNameAccountKey,
    NameRegistryState,
    reallocNameAccount,
    transferNameOwnership,
    updateNameRegistryData,
} from '../../src';

const url = 'http://127.0.0.1:8899';
const connection = new Connection(url, 'confirmed');
const payer = Keypair.generate();
const owner = Keypair.generate();
const space = 20;
let nameKey: PublicKey;
let name: string;
beforeAll(async () => {
    const airdropSignature = await connection.requestAirdrop(payer.publicKey, LAMPORTS_PER_SOL);
    await connection.confirmTransaction({
        signature: airdropSignature,
        ...(await connection.getLatestBlockhash()),
    });
});

beforeEach(async () => {
    name = Math.random().toString() + '.sol';
    nameKey = await getNameKey(name);
    const lamports = await connection.getMinimumBalanceForRentExemption(space + NameRegistryState.HEADER_LEN);
    const inst = await createNameRegistry(connection, name, space, payer.publicKey, owner.publicKey, lamports);
    const tx = new Transaction().add(inst);
    await sendAndConfirmTransaction(connection, tx, [payer]);
});

test('Create Name Registery', async () => {
    const nameAccount = await NameRegistryState.retrieve(connection, nameKey);
    nameAccount.owner.equals(owner.publicKey);
    expect(nameAccount.data?.length).toEqual(space);
});
test('Update Name Registery', async () => {
    const data = Buffer.from('@Dudl');
    const inst = await updateNameRegistryData(connection, name, 0, data);
    const tx = new Transaction().add(inst);
    await sendAndConfirmTransaction(connection, tx, [payer, owner]);
    const nameAccount = await NameRegistryState.retrieve(connection, nameKey);
    nameAccount.data?.equals(data);
});
test('Transfer Name Ownership', async () => {
    const newOwner = Keypair.generate();
    const inst = await transferNameOwnership(connection, name, newOwner.publicKey);
    const tx = new Transaction().add(inst);
    await sendAndConfirmTransaction(connection, tx, [payer, owner]);
    const nameAccount = await NameRegistryState.retrieve(connection, nameKey);
    nameAccount.owner.equals(newOwner.publicKey);
});
test('Realloc Name Account to bigger space', async () => {
    const inst = await reallocNameAccount(connection, name, space + 10, payer.publicKey);
    const tx = new Transaction().add(inst);
    await sendAndConfirmTransaction(connection, tx, [payer, owner]);
    const nameAccount = await NameRegistryState.retrieve(connection, nameKey);
    expect(nameAccount.data?.length).toEqual(space + 10);
});
test('Realloc Name Account to smaller space', async () => {
    const inst = await reallocNameAccount(connection, name, space - 10, payer.publicKey);
    const tx = new Transaction().add(inst);
    await sendAndConfirmTransaction(connection, tx, [payer, owner]);
    const nameAccount = await NameRegistryState.retrieve(connection, nameKey);
    expect(nameAccount.data?.length).toEqual(space - 10);
});
test('Delete Name Registry', async () => {
    const inst = await deleteNameRegistry(connection, name, payer.publicKey);
    const tx = new Transaction().add(inst);
    await sendAndConfirmTransaction(connection, tx, [payer, owner]);
    const nameAccount = await connection.getAccountInfo(nameKey);
    expect(nameAccount).toBeNull;
});

const getNameKey = async (name: string, nameClass?: PublicKey, parentName?: PublicKey) => {
    const hashedName = await getHashedName(name);
    const nameAccountKey = await getNameAccountKey(hashedName, nameClass, parentName);
    return nameAccountKey;
};
