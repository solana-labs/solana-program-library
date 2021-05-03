import * as schema from './schema.js';
import borsh from 'borsh';
import solanaWeb3 from '@solana/web3.js';

export class StakePool {
  /**
   * Wrapper class for a stake pool.
   * Each stake pool has a stake pool account and a validator list account.
   * (Optionally) a stake pool can also have a name and ticker.
   */
  name: string;
  ticker: string;
  stakePool: {
    pubkey: solanaWeb3.PublicKey;
    accountInfo: solanaWeb3.AccountInfo<schema.StakePoolAccount>;
  };
  validatorList: {
    pubkey: solanaWeb3.PublicKey;
    accountInfo: solanaWeb3.AccountInfo<schema.ValidatorListAccount>;
  };
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
): Promise<
  {
    account: solanaWeb3.AccountInfo<
      schema.StakePoolAccount | schema.ValidatorListAccount
    >;
    pubkey: solanaWeb3.PublicKey;
  }[]
> {
  try {
    let response = await connection.getProgramAccounts(STAKE_POOL_ADDR);

    const stakePoolAccounts = response.map(a => {
      let b = {
        pubkey: a.pubkey,
        account: {
          data: null,
          executable: a.account.executable,
          lamports: a.account.lamports,
          owner: a.account.owner,
        },
      };

      if (a.account.data.length === STAKE_POOL_ACCT_LENGTH) {
        b.account.data = decodeSerializedStakePool(
          a.account.data,
          schema.StakePoolAccount,
        );
      } else {
        b.account.data = decodeSerializedStakePool(
          a.account.data,
          schema.ValidatorListAccount,
        );
      }
      return b;
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
