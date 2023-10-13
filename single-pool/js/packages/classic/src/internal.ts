import { Connection, Transaction, TransactionInstruction, PublicKey } from '@solana/web3.js';
import { Buffer } from 'buffer';

export function rpc(connection: Connection) {
  return {
    getAccountInfo(address: string) {
      return {
        async send() {
          const pubkey = new PublicKey(address);
          return await connection.getAccountInfo(pubkey);
        },
      };
    },
    getMinimumBalanceForRentExemption(size: bigint) {
      return {
        async send() {
          return BigInt(await connection.getMinimumBalanceForRentExemption(Number(size)));
        },
      };
    },
    getStakeMinimumDelegation() {
      return {
        async send() {
          const minimumDelegation = await connection.getStakeMinimumDelegation();
          return { value: BigInt(minimumDelegation.value) };
        },
      };
    },
  };
}

export function modernInstructionToLegacy(modernInstruction: any): TransactionInstruction {
  const keys = [];
  for (const account of modernInstruction.accounts) {
    keys.push({
      pubkey: new PublicKey(account.address),
      isSigner: !!(account.role & 2),
      isWritable: !!(account.role & 1),
    });
  }

  return new TransactionInstruction({
    programId: new PublicKey(modernInstruction.programAddress),
    keys,
    data: Buffer.from(modernInstruction.data),
  });
}

export function modernTransactionToLegacy(modernTransaction: any): Transaction {
  const legacyTransaction = new Transaction();
  legacyTransaction.add(...modernTransaction.instructions.map(modernInstructionToLegacy));

  return legacyTransaction;
}

export function paramsToModern(params: any) {
  const modernParams = {} as any;
  for (const k of Object.keys(params)) {
    if (k == 'connection') {
      modernParams.rpc = rpc(params[k]);
    } else if (params[k] instanceof PublicKey || params[k].constructor.name == 'PublicKey') {
      modernParams[k] = params[k].toBase58();
    } else if (typeof params[k] == 'number') {
      modernParams[k] = BigInt(params[k]);
    } else {
      modernParams[k] = params[k];
    }
  }

  return modernParams;
}
