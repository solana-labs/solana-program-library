import { Connection, PublicKey } from '@solana/web3.js';
import { deserialize, Schema } from 'borsh';

type InitArgs = {
    parentName: Uint8Array;
    owner: Uint8Array;
    class: Uint8Array;
};

export class NameRegistryState {
    static HEADER_LEN = 96;
    parentName: PublicKey;
    owner: PublicKey;
    class: PublicKey;
    data: Buffer | undefined;

    static schema: Schema = {
        struct: {
            parentName: { array: { type: 'u8', len: 32 } },
            owner: { array: { type: 'u8', len: 32 } },
            class: { array: { type: 'u8', len: 32 } },
        },
    };
    constructor(obj: InitArgs) {
        this.parentName = new PublicKey(obj.parentName);
        this.owner = new PublicKey(obj.owner);
        this.class = new PublicKey(obj.class);
    }

    public static async retrieve(connection: Connection, nameAccountKey: PublicKey): Promise<NameRegistryState> {
        const nameAccount = await connection.getAccountInfo(nameAccountKey, 'processed');
        if (!nameAccount) {
            throw new Error('Invalid name account provided');
        }

        const deserialized = deserialize(this.schema, nameAccount.data) as InitArgs;
        const res = new NameRegistryState(deserialized);

        res.data = nameAccount.data?.slice(this.HEADER_LEN);

        return res;
    }
}
