import { Connection, PublicKey } from '@solana/web3.js';
import { deserializeUnchecked, Schema } from 'borsh';

export class NameRegistryState {
  parentName: PublicKey;
  owner: PublicKey;
  class: PublicKey;
  data: Buffer;

  static schema: Schema = new Map([
    [
      NameRegistryState,
      {
        kind: 'struct',
        fields: [
          ['parentName', [32]],
          ['owner', [32]],
          ['class', [32]],
          ['data', ['u8']],
        ],
      },
    ],
  ]);
  constructor(obj: {
    parentName: Uint8Array;
    owner: Uint8Array;
    class: Uint8Array;
    data: Uint8Array;
  }) {
    this.parentName = new PublicKey(obj.parentName);
    this.owner = new PublicKey(obj.owner);
    this.class = new PublicKey(obj.class);
    this.data = Buffer.from(obj.data);
  }

  static async retrieve(
    connection: Connection,
    nameAccountKey: PublicKey
  ): Promise<NameRegistryState> {
    const nameAccount = await connection.getAccountInfo(
      nameAccountKey,
      'processed'
    );
    if (!nameAccount) {
      throw new Error('Invalid name account provided');
    }

    const res: NameRegistryState = deserializeUnchecked(
      this.schema,
      NameRegistryState,
      nameAccount.data
    );
    return res;
  }
}
