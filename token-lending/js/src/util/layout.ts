import { AccountInfo, PublicKey } from '@solana/web3.js';

export type Parser<T> = (
    pubkey: PublicKey,
    info: AccountInfo<Uint8Array>,
) =>
    | {
          pubkey: PublicKey;
          info: AccountInfo<Uint8Array>;
          data: T;
      }
    | undefined;
