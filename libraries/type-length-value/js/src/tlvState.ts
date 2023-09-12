import { TlvInvalidAccountSizeError, TlvInvalidDiscriminatorError } from './errors.js';

export type TlvNumberSize = 1 | 2 | 4 | 8;

export interface TlvDiscriminator {
    bytes: Buffer;
}

export type TlvType = Buffer | Uint8Array | number | bigint | TlvDiscriminator;

export class TlvState {
    private readonly tlvData: Buffer;
    private readonly typeSize: TlvNumberSize;
    private readonly lengthSize: TlvNumberSize;

    public constructor(buffer: Buffer, typeSize: TlvNumberSize = 2, lengthSize: TlvNumberSize = 2, offset: number = 0) {
        this.tlvData = buffer.subarray(offset);
        this.typeSize = typeSize;
        this.lengthSize = lengthSize;
    }

    /**
     * Get the raw tlv data
     *
     * @return the raw tlv data
     */
    public get data(): Buffer {
        return this.tlvData;
    }

    private readTlvNumberSize<T>(size: TlvNumberSize, offset: number, constructor: (x: number | bigint) => T): T {
        switch (size) {
            case 1:
                return constructor(this.tlvData.readUInt8(offset));
            case 2:
                return constructor(this.tlvData.readUInt16LE(offset));
            case 4:
                return constructor(this.tlvData.readUInt32LE(offset));
            case 8:
                return constructor(this.tlvData.readBigUInt64LE(offset));
        }
    }

    private tlvDiscriminatorMatches(type: TlvType, offset: number): boolean {
        switch (typeof type) {
            case 'number':
                return this.readTlvNumberSize(this.typeSize, offset, Number) === type;
            case 'bigint':
                return this.readTlvNumberSize(this.typeSize, offset, BigInt) === type;
            case 'object':
                if (type instanceof Buffer) {
                    return this.tlvData.subarray(offset, offset + this.typeSize).equals(type);
                }
                if ('bytes' in type) {
                    return this.tlvData.subarray(offset, offset + this.typeSize).equals(type.bytes);
                }
                throw new TlvInvalidDiscriminatorError();
            default:
                throw new TlvInvalidDiscriminatorError();
        }
    }

    /**
     * Get a single entry from the tlv data. This function returns the first entry with the given type.
     *
     * @param type the type of the entry to get
     *
     * @return the entry from the tlv data or null
     */
    public firstBytes(type: TlvType): Buffer | null {
        const entries = this.bytesRepeating(type, 1);
        return entries.length > 0 ? entries[0] : null;
    }

    /**
     * Get a multiple entries from the tlv data. This function returns `count` or less entries with the given type.
     *
     * @param type the type of the entry to get
     * @param count the number of entries to get (0 for all entries)
     *
     * @return the entry from the tlv data or null
     */
    public bytesRepeating(type: TlvType, count = 0): Buffer[] {
        const entries: Buffer[] = [];
        let offset = 0;
        while (offset < this.tlvData.length) {
            if (offset + this.typeSize + this.lengthSize > this.tlvData.length) {
                throw new TlvInvalidAccountSizeError();
            }
            const typeMatches = this.tlvDiscriminatorMatches(type, offset);
            offset += this.typeSize;
            const entryLength = this.readTlvNumberSize(this.lengthSize, offset, Number);
            offset += this.lengthSize;
            if (offset + entryLength > this.tlvData.length) {
                throw new TlvInvalidAccountSizeError();
            }
            if (typeMatches) {
                entries.push(this.tlvData.subarray(offset, offset + entryLength));
            }
            if (count > 0 && entries.length >= count) {
                break;
            }
            offset += entryLength;
        }
        return entries;
    }

    /**
     * Get all the discriminators from the tlv data. This function will return a type multiple times if it occurs multiple times in the tlv data.
     *
     * @return a list of the discriminators.
     */
    public discriminators(): Buffer[] {
        const types: Buffer[] = [];
        let offset = 0;
        while (offset < this.tlvData.length) {
            if (offset + this.typeSize + this.lengthSize > this.tlvData.length) {
                throw new TlvInvalidAccountSizeError();
            }
            const type = this.tlvData.subarray(offset, offset + this.typeSize);
            types.push(type);
            offset += this.typeSize;
            const entryLength = this.readTlvNumberSize(this.lengthSize, offset, Number);
            offset += this.lengthSize;
            if (offset + entryLength > this.tlvData.length) {
                throw new TlvInvalidAccountSizeError();
            }
            offset += entryLength;
        }
        return types;
    }
}
