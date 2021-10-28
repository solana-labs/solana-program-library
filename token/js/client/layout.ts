import * as BufferLayout from '@solana/buffer-layout';

/**
 * Layout for a public key
 */
export const publicKey = (property: string = 'publicKey'): BufferLayout.Layout => {
  return BufferLayout.blob(32, property);
};

/**
 * Layout for a 64bit unsigned value
 */
export const uint64 = (property: string = 'uint64'): BufferLayout.Layout => {
  return BufferLayout.blob(8, property);
};
