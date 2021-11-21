import {PublicKey, Connection, clusterApiUrl} from '@solana/web3.js';
import * as index from '../src/index';
import {getFirstStakePoolAccount} from "./utils";

describe('Integration test', () => {
  it.skip('should successfully decode all validators from devnet', async () => {
    /**
     * Full integration test:
     * Makes a connection to devnet, gets all stake pool accounts there,
     * decodes them, and prints their details.
     */
    const connection = new Connection(
      clusterApiUrl('devnet'),
      'confirmed',
    );

    const STAKE_POOL_PROGRAM_ADDR = new PublicKey(
      'SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy',
    );

    const accounts = await index.getStakePoolAccounts(
      connection,
      STAKE_POOL_PROGRAM_ADDR,
    );

    console.log('Number of stake pool accounts in devnet: ', accounts!.length);

    accounts!.map(account => {
      index.prettyPrintAccount(account);
      console.log('\n');
    });
  });

  it.skip('should successfully decode all validators from testnet', async () => {
    /**
     * Full integration test:
     * Makes a connection to testnet, gets all stake pool accounts there,
     * decodes them, and prints their details.
     * Testnet presents a greater challenge due to the presence of old stake pool program accounts
     */
    const connection = new Connection(
      clusterApiUrl('testnet'),
      'confirmed',
    );

    const STAKE_POOL_PROGRAM_ADDR = new PublicKey(
      'poo1B9L9nR3CrcaziKVYVpRX6A9Y1LAXYasjjfCbApj',
    );

    const accounts = await index.getStakePoolAccounts(
      connection,
      STAKE_POOL_PROGRAM_ADDR,
    );

    console.log('Number of stake pool accounts in testnet: ', accounts!.length);

    accounts!.map(account => {
      index.prettyPrintAccount(account);
      console.log('\n');
    });
  });

  it('should successfully get pool info from first pool in devnet', async () => {

    const connection = new Connection(
      clusterApiUrl('devnet'),
      // 'http://localhost:8899',
      'confirmed',
    );

    const STAKE_POOL_PROGRAM_ADDR = new PublicKey(
      'SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy',
      // 'poo1B9L9nR3CrcaziKVYVpRX6A9Y1LAXYasjjfCbApj',
    );

    const first = await getFirstStakePoolAccount(connection, STAKE_POOL_PROGRAM_ADDR);

    console.log('\n');
    console.log('\n');
    console.log('\n');
    console.log('\n');

    index.prettyPrintAccount(first!);

    console.log('first: ' + index.prettyPrintPubKey(first!.pubkey));

  });

});
