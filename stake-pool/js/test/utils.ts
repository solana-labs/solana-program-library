import {assert} from 'chai';
import {Connection, PublicKey} from "@solana/web3.js";
import * as index from "../src";
import BN from "bn.js";

export function isStakePoolAccount(account: any): account is index.StakePoolAccount {
  return (account !== undefined) &&
    (account.account !== undefined) &&
    (account.account.data !== undefined) &&
    ('manager' in account.account.data);
}

export async function getFirstStakePoolAccount(
  connection: Connection,
  stakePoolProgramAddress: PublicKey,
): Promise<index.StakePoolAccount | undefined> {
  const accounts = await index.getStakePoolAccounts(connection, stakePoolProgramAddress);

  return accounts!
    // .filter(accounts => accounts !== undefined)
    .filter(account => isStakePoolAccount(account))
    .pop() as index.StakePoolAccount;
}

/**
 * Helper function to do deep equality check because BNs are not equal.
 * TODO: write this function recursively. For now, sufficient.
 */
export function deepStrictEqualBN(decodedData: any, expectedData: any) {
  for (const key in decodedData) {
    if (expectedData[key] instanceof BN) {
      assert.ok(expectedData[key].eq(decodedData[key]));
    } else {
      if (decodedData[key] instanceof Object) {
        for (const subkey in decodedData[key]) {
          if (decodedData[key][subkey] instanceof Object) {
            if (decodedData[key][subkey] instanceof BN) {
              assert.ok(decodedData[key][subkey].eq(expectedData[key][subkey]));
            } else {
              for (const subsubkey in decodedData[key][subkey]) {
                if (decodedData[key][subkey][subsubkey] instanceof BN) {
                  assert.ok(
                    decodedData[key][subkey][subsubkey].eq(
                      expectedData[key][subkey][subsubkey],
                    ),
                  );
                } else {
                  assert.deepStrictEqual(
                    expectedData[key][subkey][subsubkey],
                    decodedData[key][subkey][subsubkey],
                  );
                }
              }
            }
          } else {
            assert.strictEqual(
              decodedData[key][subkey],
              expectedData[key][subkey],
            );
          }
        }
      } else {
        assert.strictEqual(decodedData[key], expectedData[key]);
      }
    }
  }
}
