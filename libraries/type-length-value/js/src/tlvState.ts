import { TlvInvalidAccountDataError } from './errors.js';

export type LengthSize = 1 | 2 | 4 | 8;

export type Discriminator = Uint8Array;

export class TlvState {
    private readonly tlvData: Buffer;
    private readonly discriminatorSize: number;
    private readonly lengthSize: LengthSize;

    public constructor(buffer: Buffer, discriminatorSize = 2, lengthSize: LengthSize = 2, offset: number = 0) {
        this.tlvData = buffer.subarray(offset);
        this.discriminatorSize = discriminatorSize;
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

    private readEntryLength<T>(size: LengthSize, offset: number, constructor: (x: number | bigint) => T): T {
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

    /**
     * Get a single entry from the tlv data. This function returns the first entry with the given type.
     *
     * @param type the type of the entry to get
     *
     * @return the entry from the tlv data or null
     */
    public firstBytes(discriminator: Discriminator): Buffer | null {
        const entries = this.bytesRepeating(discriminator, 1);
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
    public bytesRepeating(discriminator: Discriminator, count = 0): Buffer[] {
        const entries: Buffer[] = [];
        let offset = 0;
        while (offset < this.tlvData.length) {
            if (offset + this.discriminatorSize + this.lengthSize > this.tlvData.length) {
                throw new TlvInvalidAccountDataError();
            }
            const type = this.tlvData.subarray(offset, offset + this.discriminatorSize);
            offset += this.discriminatorSize;
            const entryLength = this.readEntryLength(this.lengthSize, offset, Number);
            offset += this.lengthSize;
            if (offset + entryLength > this.tlvData.length) {
                throw new TlvInvalidAccountDataError();
            }
            if (type.equals(discriminator)) {
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
        const discriminators: Buffer[] = [];
        let offset = 0;
        while (offset < this.tlvData.length) {
            if (offset + this.discriminatorSize + this.lengthSize > this.tlvData.length) {
                throw new TlvInvalidAccountDataError();
            }
            const type = this.tlvData.subarray(offset, offset + this.discriminatorSize);
            discriminators.push(type);
            offset += this.discriminatorSize;
            const entryLength = this.readEntryLength(this.lengthSize, offset, Number);
            offset += this.lengthSize;
            if (offset + entryLength > this.tlvData.length) {
                throw new TlvInvalidAccountDataError();
            }
            offset += entryLength;
        }
        return discriminators;
    }
}
