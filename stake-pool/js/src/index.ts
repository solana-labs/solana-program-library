import * as schema from './schema.js';
import borsh from 'borsh';
import solanaWeb3 from '@solana/web3.js';

export class StakePool {
  name: string;
  ticker: string;
  stakePool: solanaWeb3.AccountInfo<schema.StakePoolAccount>;
  validatorList: solanaWeb3.AccountInfo<schema.ValidatorListAccount>;
}

function decodeSerializedStakePool(
  serializedStakePool: Buffer,
  accountType,
): schema.StakePoolAccount | schema.ValidatorListAccount {
  return accountType.decode(serializedStakePool);
}

async function getStakePoolAccounts(
  connection: solanaWeb3.Connection,
  stakePoolAddress: solanaWeb3.PublicKey,
): Promise<(schema.StakePoolAccount | schema.ValidatorListAccount)[]> {
  try {
    let response = await connection.getProgramAccounts(STAKE_POOL_ADDR);

    const stakePoolAccounts = response.map(a => {
      if (a.account.data.length === STAKE_POOL_ACCT_LENGTH) {
        return decodeSerializedStakePool(
          a.account.data,
          schema.StakePoolAccount,
        );
      } else {
        return decodeSerializedStakePool(
          a.account.data,
          schema.ValidatorListAccount,
        );
      }
    });

    return stakePoolAccounts;
  } catch (error) {
    console.log(error);
  }
}

/* Test function on devnet: get accounts, deserialize them, then log them */

const STAKE_POOL_ACCT_LENGTH = 298;
const connection = new solanaWeb3.Connection(
  'https://devnet.solana.com/',
  'confirmed',
);
const STAKE_POOL_ADDR = new solanaWeb3.PublicKey(
  'poo1B9L9nR3CrcaziKVYVpRX6A9Y1LAXYasjjfCbApj',
);

getStakePoolAccounts(connection, STAKE_POOL_ADDR).then(accounts => {
  accounts.map(sp => {
    if (sp) {
      for (const val in sp) {
        if (sp[val] instanceof schema.PublicKey) {
          console.log(
            val,
            new solanaWeb3.PublicKey(
              new solanaWeb3.PublicKey(sp[val].value).toBytes().reverse(),
            ).toString(),
          );
        } else {
          console.log(val, sp[val]);
        }
      }
    }
    console.log('\n');
  });
});
