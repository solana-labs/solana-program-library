import { Connection, PublicKey } from '@solana/web3.js';

export class NameRegistryState {
  parentName: PublicKey;
  owner: PublicKey;
  class: PublicKey;
  data: Buffer;

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

  static deserialize(buffer: Buffer): NameRegistryState {
    return new NameRegistryState({
      parentName: buffer.slice(0, 32),
      owner: buffer.slice(32, 64),
      class: buffer.slice(64, 96),
      data: buffer.slice(96, buffer.length),
    });
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

    const res: NameRegistryState = NameRegistryState.deserialize(
      nameAccount.data
    );
    return res;
  }
}
