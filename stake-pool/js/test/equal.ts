import BN from 'bn.js';
import {StakePoolAccount} from '../src';

export function isStakePoolAccount(account: any): account is StakePoolAccount {
  return (
    account !== undefined &&
    account.account !== undefined &&
    account.account.data !== undefined &&
    'manager' in account.account.data
  );
}

/**
 * Helper function to do deep equality check because BNs are not equal.
 * TODO: write this function recursively. For now, sufficient.
 */
export function deepStrictEqualBN(a: any, b: any) {
  for (const key in a) {
    if (b[key] instanceof BN) {
      expect(b[key].toString()).toEqual(a[key].toString());
    } else {
      if (a[key] instanceof Object) {
        for (const subkey in a[key]) {
          if (a[key][subkey] instanceof Object) {
            if (a[key][subkey] instanceof BN) {
              expect(b[key][subkey].toString()).toEqual(
                a[key][subkey].toString(),
              );
            } else {
              for (const subsubkey in a[key][subkey]) {
                if (a[key][subkey][subsubkey] instanceof BN) {
                  expect(b[key][subkey][subsubkey].toString()).toEqual(
                    a[key][subkey][subsubkey].toString(),
                  );
                } else {
                  expect(b[key][subkey][subsubkey]).toStrictEqual(
                    a[key][subkey][subsubkey],
                  );
                }
              }
            }
          } else {
            expect(b[key][subkey]).toStrictEqual(a[key][subkey]);
          }
        }
      } else {
        expect(b[key]).toStrictEqual(a[key]);
      }
    }
  }
}
