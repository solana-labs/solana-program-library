import * as index from './index.js';
import * as schema from './schema.js';
import BN from 'bn.js';
import assert, {deepStrictEqual} from 'assert';
import {SOLANA_SCHEMA, PublicKey, Connection} from '@solana/web3.js';

// First populate schema
schema.addStakePoolSchema(SOLANA_SCHEMA);

describe('Integration test', () => {
  it('should successfully decode all validators from devnet', async () => {
    /**
     * Full integration test:
     * Makes a connection to devnet, gets all stake pool accounts there,
     * decodes them, and prints their details.
     */
    const connection = new Connection(
      'https://api.devnet.solana.com/',
      'confirmed',
    );
    const STAKE_POOL_PROGRAM_ADDR = new PublicKey(
      'poo1B9L9nR3CrcaziKVYVpRX6A9Y1LAXYasjjfCbApj',
    );

    const accounts = await index.getStakePoolAccounts(
      connection,
      STAKE_POOL_PROGRAM_ADDR,
    );

    console.log('Number of stake pool accounts in devnet: ', accounts.length);

    accounts.map(account => {
      index.prettyPrintAccount(account);
      console.log('\n');
    });
  });

  it('should successfully decode all validators from testnet', async () => {
    /**
     * Full integration test:
     * Makes a connection to testnet, gets all stake pool accounts there,
     * decodes them, and prints their details.
     * Testnet presents a greater challenge due to the presence of old stake pool program accounts
     */
    const connection = new Connection(
      'https://api.testnet.solana.com/',
      'confirmed',
    );
    const STAKE_POOL_PROGRAM_ADDR = new PublicKey(
      'poo1B9L9nR3CrcaziKVYVpRX6A9Y1LAXYasjjfCbApj',
    );

    const accounts = await index.getStakePoolAccounts(
      connection,
      STAKE_POOL_PROGRAM_ADDR,
    );

    console.log('Number of stake pool accounts in testnet: ', accounts.length);

    accounts.map(account => {
      index.prettyPrintAccount(account);
      console.log('\n');
    });
  });
});
