import {
  ConfirmOptions,
  sendAndConfirmTransaction as realSendAndConfirmTransaction,
} from '@solana/web3.js';
import type {
  Account,
  Connection,
  Transaction,
  TransactionSignature,
} from '@solana/web3.js';

export function sendAndConfirmTransaction(
  title: string,
  connection: Connection,
  transaction: Transaction,
  options: ConfirmOptions,
  ...signers: Array<Account>
): Promise<TransactionSignature> {
  return realSendAndConfirmTransaction(connection, transaction, signers, {
    skipPreflight: options.skipPreflight,
    commitment: 'recent',
    preflightCommitment: 'recent',
  });
}
