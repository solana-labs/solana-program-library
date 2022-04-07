import {TransactionInstruction, PublicKey} from '@solana/web3.js';

/**
 * Address of the memo program.
 */
export const MEMO_CONFIG = new PublicKey(
  'MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr',
);

/**
 *  Initialize build memo params
 */

export type BuildMemoParams = {
  /** memo input */
  memo: String;
  /** signers of the transaction*/
  signer_public_keys?: PublicKey[];
};

export class MemoProgram {
  constructor() {}

  static programId: PublicKey = new PublicKey(
    'MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr',
  );

  /** Returns the public key of program id */
  static id(): PublicKey {
    return MemoProgram.programId;
  }

  /** Checks whether the given public key is same as the program id */
  static checkId(id: PublicKey): boolean {
    return id.equals(MemoProgram.programId);
  }

  /** Returns a transaction to the memo program*/

  static buildMemo(params: BuildMemoParams): TransactionInstruction {
    const {memo, signer_public_keys} = params;

    let data = Buffer.from(memo);
    let keys = [];
    if (signer_public_keys) {
      for (const key of signer_public_keys) {
        keys.push({pubkey: key, isSigner: true, isWritable: true});
      }
    }

    return new TransactionInstruction({
      keys: keys,
      programId: MemoProgram.id(),
      data,
    });
  }
}
