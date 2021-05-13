require('dotenv').config();

import { Connection, Keypair, PublicKey, Transaction } from '@solana/web3.js';
import { ProgramMetadata } from "../index";
import bs58 from 'bs58';
import { expect } from 'chai';
import { v4 as uuid } from 'uuid';
import { createHash } from 'crypto';
import { SerializationMethod } from '../instruction';

const timeout = (ms) => new Promise(resolve => setTimeout(resolve, ms));

const privateSecretKey = require(process.env.PRIVATE_KEY_PATH);
const privateKeypair = Keypair.fromSecretKey(new Uint8Array(privateSecretKey));
const connection = new Connection('http://localhost:8899', 'single');
const targetProgramKey = new PublicKey(process.env.TARGET_PROGRAM_KEY);
const programMetadata = new ProgramMetadata(connection,
  {
    programMetadataKey: new PublicKey(process.env.PROGRAM_METADATA_KEY),
    nameServiceKey: new PublicKey(process.env.NAME_SERVICE_KEY)
  }
);

describe('ProgramMetadata: metadata entries', async () => {
  describe('create metadata entry', async () => {
    it('should create a metadata entry', async () => {
      const name = uuid();

      const ix = await programMetadata.createMetadataEntry(
        targetProgramKey,
        privateKeypair.publicKey,
        privateKeypair.publicKey,
        name,
        'some metadata'
      );

      let tx = new Transaction();
      const signers: Keypair[] = [];
      signers.push(privateKeypair);
      tx.add(ix);

      let res = await connection.sendTransaction(tx, signers, {
        preflightCommitment: 'single'
      });

      await timeout(5000);

      const accts = await connection.getProgramAccounts(
        programMetadata.nameServiceKey,
        {
          filters: [
            {
              memcmp: {
                bytes: bs58.encode(Buffer.from(name)),
                offset: 101
              }
            }
          ]
        }
      );

      expect(accts.length).to.equal(1);

      // clean up
      const deleteIx = await programMetadata.deleteMetadataEntry(
        targetProgramKey,
        privateKeypair.publicKey,
        privateKeypair.publicKey,
        name,
      );

      tx = new Transaction();
      tx.add(deleteIx);

      res = await connection.sendTransaction(tx, signers, {
        preflightCommitment: 'single'
      });

      await timeout(5000);
    });
  });

  describe('update metadata entry', async () => {
    it('should update metadata entry', async () => {

      const name = uuid();

      const ix = await programMetadata.createMetadataEntry(
        targetProgramKey,
        privateKeypair.publicKey,
        privateKeypair.publicKey,
        name,
        'some metadata'
      );

      let tx = new Transaction();
      const signers: Keypair[] = [];
      signers.push(privateKeypair);
      tx.add(ix);

      let res = await connection.sendTransaction(tx, signers, {
        preflightCommitment: 'single'
      });

      await timeout(5000);

      const updateIx = await programMetadata.updateMetadataEntry(
        targetProgramKey,
        privateKeypair.publicKey,
        name,
        "new metadata"
      );

      tx = new Transaction();
      tx.add(updateIx);

      res = await connection.sendTransaction(tx, signers, {
        preflightCommitment: 'single'
      });

      await timeout(5000);

      const accts = await connection.getProgramAccounts(
        programMetadata.nameServiceKey,
        {
          filters: [
            {
              memcmp: {
                bytes: bs58.encode(Buffer.from(name)),
                offset: 101
              }
            }
          ]
        }
      );

      expect(accts.length).to.equal(1);
      expect(!!accts[0].account.data.toString().match(/new metadata/));

      // clean up
      const deleteIx = await programMetadata.deleteMetadataEntry(
        targetProgramKey,
        privateKeypair.publicKey,
        privateKeypair.publicKey,
        name,
      );

      tx = new Transaction();
      tx.add(deleteIx);

      res = await connection.sendTransaction(tx, signers, {
        preflightCommitment: 'single'
      });

      await timeout(5000);
    });
  });

  describe('delete metadata entry', async () => {
    it('should delete a metadata entry', async () => {
      const name = uuid();

      const ix = await programMetadata.createMetadataEntry(
        targetProgramKey,
        privateKeypair.publicKey,
        privateKeypair.publicKey,
        name,
        'some metadata'
      );

      let tx = new Transaction();
      const signers: Keypair[] = [];
      signers.push(privateKeypair);
      tx.add(ix);

      let res = await connection.sendTransaction(tx, signers, {
        preflightCommitment: 'single'
      });

      await timeout(5000);

      // clean up
      const deleteIx = await programMetadata.deleteMetadataEntry(
        targetProgramKey,
        privateKeypair.publicKey,
        privateKeypair.publicKey,
        name,
      );

      tx = new Transaction();
      tx.add(deleteIx);

      res = await connection.sendTransaction(tx, signers, {
        preflightCommitment: 'single'
      });

      await timeout(5000);

      const accts = await connection.getProgramAccounts(
        programMetadata.nameServiceKey,
        {
          filters: [
            {
              memcmp: {
                bytes: bs58.encode(Buffer.from(name)),
                offset: 101
              }
            }
          ]
        }
      );

      expect(accts.length).to.equal(0);
    });
  });
});

describe('ProgramMetadata: IDL entries', async () => {
  describe('create versioned idl', async () => {
    it('should create versioned idl', async () => {
      const effectiveSlot = 3000;
      const idlHash = createHash('sha256').update('some idl', 'utf8').digest();
      const ix = await programMetadata.createVersionedIdl(
        targetProgramKey,
        privateKeypair.publicKey,
        privateKeypair.publicKey,
        effectiveSlot,
        'http://www.test.com',
        idlHash,
        'https://github.com/source',
        SerializationMethod.Borsh,
        null
      );

      let tx = new Transaction();
      const signers: Keypair[] = [];
      signers.push(privateKeypair);
      tx.add(ix);

      let res = await connection.sendTransaction(tx, signers, {
        preflightCommitment: 'single'
      });

      expect(!!res);

      await timeout(5000);

         // clean up
         const deleteIx = await programMetadata.deleteMetadataEntry(
          targetProgramKey,
          privateKeypair.publicKey,
          privateKeypair.publicKey,
          `idl_${effectiveSlot}`,
        );

        tx = new Transaction();
        tx.add(deleteIx);

        res = await connection.sendTransaction(tx, signers, {
          preflightCommitment: 'single'
        });

        await timeout(5000);
    });
  });

  describe('update versioned idl', async () => {

  });

  describe('delete versioned idl', async () => {

  });
});