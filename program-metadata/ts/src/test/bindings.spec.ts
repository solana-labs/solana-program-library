require('dotenv').config();

import { Connection, Keypair, PublicKey, Transaction } from '@solana/web3.js';
import { ProgramMetadata } from "../index";
import { expect } from 'chai';
import { v4 as uuid } from 'uuid';

const timeout = (ms) => new Promise(resolve => setTimeout(resolve, ms));

const privateSecretKey = require(process.env.PRIVATE_KEY_PATH);
const privateKeypair = Keypair.fromSecretKey(new Uint8Array(privateSecretKey));
const connection = new Connection('http://localhost:8899');
const targetProgramKey = new PublicKey(process.env.TARGET_PROGRAM_KEY);
const programMetadata = new ProgramMetadata(connection,
  {
    programMetadataKey: new PublicKey(process.env.PROGRAM_METADATA_KEY),
    nameServiceKey: new PublicKey(process.env.NAME_SERVICE_KEY)
  }
);

describe('ProgramMetadata', async () => {

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
    });
  });

  describe('update metadata entry', async () => {

  });

  describe('delete metadata entry', async () => {

  });
});