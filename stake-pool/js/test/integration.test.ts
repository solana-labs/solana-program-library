import { PublicKey, Connection, clusterApiUrl } from '@solana/web3.js';
import { getFirstStakePoolAccount } from "./utils";
import { getStakePoolAccounts, prettyPrintAccount, prettyPrintPubKey } from "../src";

/**
 * @joncinque:
 *  These tests could be extremely flaky because of the devnet connection, so we could probably just remove them.
 *  It doesn't need to be done in this PR, but eventually we should have tests that create a stake pool / deposit / withdraw,
 *  all only accessing a local test validator. Same as with the token and token-swap js tests.
 */
describe('Integration test', () => {

  it.skip('should successfully decode all validators from devnet', (done) => {
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

    getStakePoolAccounts(
      connection,
      STAKE_POOL_PROGRAM_ADDR,
    ).then((accounts) => {
      console.log('Number of stake pool accounts in devnet: ', accounts!.length);
      accounts!.map(account => {
        prettyPrintAccount(account);
        console.log('\n');
      });
      done();
    });

  });

  it.skip('should successfully decode all validators from testnet', (done) => {
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

    getStakePoolAccounts(
      connection,
      STAKE_POOL_PROGRAM_ADDR,
    ).then((accounts) => {
      console.log('Number of stake pool accounts in testnet: ', accounts!.length);
      accounts!.map(account => {
        prettyPrintAccount(account);
        console.log('\n');
      });
      done();
    });

  });

  it('should successfully get pool info from first pool in devnet', (done) => {

    const connection = new Connection(
      clusterApiUrl('devnet'),
      // 'http://localhost:8899',
      'confirmed',
    );

    const STAKE_POOL_PROGRAM_ADDR = new PublicKey(
      'SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy',
      // 'poo1B9L9nR3CrcaziKVYVpRX6A9Y1LAXYasjjfCbApj',
    );

    getFirstStakePoolAccount(connection, STAKE_POOL_PROGRAM_ADDR).then((first) => {
      console.log('\n');
      console.log('\n');
      console.log('\n');
      console.log('\n');
      prettyPrintAccount(first!);
      console.log('first: ' + prettyPrintPubKey(first!.pubkey));
      done();
    })

  });

});
