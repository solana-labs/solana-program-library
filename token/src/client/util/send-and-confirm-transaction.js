// @flow

import {sendAndConfirmTransaction as realSendAndConfirmTransaction} from '@solana/web3.js';
import type {Account, Connection, Transaction} from '@solana/web3.js';
import YAML from 'json-to-pretty-yaml';

import {newSystemAccountWithAirdrop} from './new-system-account-with-airdrop';

type TransactionNotification = (string, string) => void;

let notify: TransactionNotification = () => undefined;

export function onTransaction(callback: TransactionNotification) {
  notify = callback;
}

let payerAccount: Account | null = null;
export async function sendAndConfirmTransaction(
  title: string,
  connection: Connection,
  transaction: Transaction,
  ...signers: Array<Account>
): Promise<void> {
  const when = Date.now();

  if (!payerAccount) {
    const {feeCalculator} = await connection.getRecentBlockhash();
    const fees = feeCalculator.lamportsPerSignature * 1000; // wag
    const newPayerAccount = await newSystemAccountWithAirdrop(connection, fees);
    // eslint-disable-next-line require-atomic-updates
    payerAccount = payerAccount || newPayerAccount;
  }

  const signature = await realSendAndConfirmTransaction(
    connection,
    transaction,
    payerAccount,
    ...signers,
  );

  const body = {
    time: new Date(when).toString(),
    from: signers[0].publicKey.toBase58(),
    signature,
    instructions: transaction.instructions.map(i => {
      return {
        keys: i.keys.map(keyObj => keyObj.pubkey.toBase58()),
        programId: i.programId.toBase58(),
        data: '0x' + i.data.toString('hex'),
      };
    }),
  };

  notify(title, YAML.stringify(body).replace(/"/g, ''));
}
