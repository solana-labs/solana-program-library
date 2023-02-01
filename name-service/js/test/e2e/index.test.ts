import {
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  sendAndConfirmTransaction,
  Transaction,
} from '@solana/web3.js';
import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';

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

chai.use(chaiAsPromised);
const url = 'http://localhost:8899';

describe('Name Service Program', () => {
  let connection: Connection;
  let payer: Keypair;
  let owner: Keypair;
  let nameKey: PublicKey;
  const name = '.sol';
  before(async () => {
    connection = new Connection(url, 'confirmed');
    payer = Keypair.generate();
    const airdropSignature = await connection.requestAirdrop(
      payer.publicKey,
      4 * LAMPORTS_PER_SOL
    );
    await connection.confirmTransaction(airdropSignature, 'confirmed');
    nameKey = await getNameKey(name);
    owner = Keypair.generate();
    const space = 20;
    const lamports = await connection.getMinimumBalanceForRentExemption(
      space + NameRegistryState.HEADER_LEN
    );
    const inst = await createNameRegistry(
      connection,
      name,
      space,
      payer.publicKey,
      owner.publicKey,
      lamports
    );
    const tx = new Transaction().add(inst);
    await sendAndConfirmTransaction(connection, tx, [payer]);
  });

  it('Create Name Registery', async () => {
    const nameAccount = await NameRegistryState.retrieve(connection, nameKey);
    nameAccount.owner.equals(owner.publicKey);
    expect(nameAccount.data?.length).to.eql(20);
  });
  it('Update Name Registery', async () => {
    const data = Buffer.from('@Dudl');
    const inst = await updateNameRegistryData(connection, name, 0, data);
    const tx = new Transaction().add(inst);
    await sendAndConfirmTransaction(connection, tx, [payer, owner]);
    const nameAccount = await NameRegistryState.retrieve(connection, nameKey);
    nameAccount.data?.equals(data);
  });
  it('Transfer Name Ownership', async () => {
    const newOwner = Keypair.generate();
    const inst = await transferNameOwnership(
      connection,
      name,
      newOwner.publicKey
    );
    const tx = new Transaction().add(inst);
    await sendAndConfirmTransaction(connection, tx, [payer, owner]);
    const nameAccount = await NameRegistryState.retrieve(connection, nameKey);
    nameAccount.owner.equals(newOwner.publicKey);
    owner = newOwner;
  });
  it('Realloc Name Account to bigger space', async () => {
    const inst = await reallocNameAccount(
      connection,
      name,
      30,
      payer.publicKey
    );
    const tx = new Transaction().add(inst);
    await sendAndConfirmTransaction(connection, tx, [payer, owner]);
    const nameAccount = await NameRegistryState.retrieve(connection, nameKey);
    expect(nameAccount.data?.length).to.eql(30);
  });
  it('Realloc Name Account to smaller space', async () => {
    const inst = await reallocNameAccount(
      connection,
      name,
      10,
      payer.publicKey
    );
    const tx = new Transaction().add(inst);
    await sendAndConfirmTransaction(connection, tx, [payer, owner]);
    const nameAccount = await NameRegistryState.retrieve(connection, nameKey);
    expect(nameAccount.data?.length).to.eql(10);
  });
  it('Delete Name Registry', async () => {
    const inst = await deleteNameRegistry(connection, name, payer.publicKey);
    const tx = new Transaction().add(inst);
    await sendAndConfirmTransaction(connection, tx, [payer, owner]);
    const nameAccount = await connection.getAccountInfo(nameKey);
    expect(nameAccount).to.be.null;
  });
});

const getNameKey = async (
  name: string,
  nameClass?: PublicKey,
  parentName?: PublicKey
) => {
  const hashed_name = await getHashedName(name);
  const nameAccountKey = await getNameAccountKey(
    hashed_name,
    nameClass,
    parentName
  );
  return nameAccountKey;
};
