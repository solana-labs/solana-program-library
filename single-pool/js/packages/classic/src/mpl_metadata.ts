import { PublicKey } from '@solana/web3.js';
import { Buffer } from 'buffer';

export const MPL_METADATA_PROGRAM_ID = new PublicKey('metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s');

export function findMplMetadataAddress(poolMintAddress: PublicKey) {
  const [publicKey] = PublicKey.findProgramAddressSync(
    [Buffer.from('metadata'), MPL_METADATA_PROGRAM_ID.toBuffer(), poolMintAddress.toBuffer()],
    MPL_METADATA_PROGRAM_ID,
  );
  return publicKey;
}
