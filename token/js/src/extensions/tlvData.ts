export type TLVNumberSize = 1 | 2 | 4 | 8;

function readTLVNumberSize<T>(
    buffer: Buffer,
    size: TLVNumberSize,
    offset: number,
    constructor: (x: number | bigint) => T
): T {
    switch (size) {
        case 1:
            return constructor(buffer.readUInt8(offset));
        case 2:
            return constructor(buffer.readUInt16LE(offset));
        case 4:
            return constructor(buffer.readUInt32LE(offset));
        case 8:
            return constructor(buffer.readBigUInt64LE(offset));
    }
}

export class TLVData {
    private readonly tlvData: Buffer;
    private readonly typeSize: TLVNumberSize;
    private readonly lengthSize: TLVNumberSize;

    public constructor(buffer: Buffer, typeSize: TLVNumberSize = 2, lengthSize: TLVNumberSize = 2, offset: number = 0) {
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

    /**
     * Get a single entry from the tlv data. This function performs a greedy search returning the first entry with the given type.
     *
     * @param type the type of the entry to get
     *
     * @return the entry from the tlv data or null
     */
    public entry(type: Buffer | Uint8Array | number | bigint): Buffer | null {
        let offset = 0;
        while (offset + this.typeSize + this.lengthSize <= this.tlvData.length) {
            let typeMatches = false;
            switch (typeof type) {
                case 'number':
                    typeMatches = readTLVNumberSize(this.tlvData, this.typeSize, offset, Number) === type;
                    break;
                case 'bigint':
                    typeMatches = readTLVNumberSize(this.tlvData, this.typeSize, offset, BigInt) === type;
                    break;
                default:
                    typeMatches = this.tlvData.subarray(offset, offset + this.typeSize).equals(type);
                    break;
            }
            offset += this.typeSize;
            const entryLength = readTLVNumberSize(this.tlvData, this.lengthSize, offset, Number);
            offset += this.lengthSize;
            if (typeMatches && offset + entryLength <= this.tlvData.length) {
                return this.tlvData.subarray(offset, offset + entryLength);
            }
            offset += entryLength;
        }
        return null;
    }

    /**
     * Get all entries from the tlv data. If you need to get a single entry, use the entry function instead.
     *
     * @return a map of all entries in the tlv data
     */
    public entries(): Map<bigint, Buffer> {
        let offset = 0;
        const entries = new Map<bigint, Buffer>();
        while (offset + this.typeSize + this.lengthSize <= this.tlvData.length) {
            const entryType = readTLVNumberSize(this.tlvData, this.typeSize, offset, BigInt);
            offset += this.typeSize;
            const entryLength = readTLVNumberSize(this.tlvData, this.lengthSize, offset, Number);
            offset += this.lengthSize;
            if (offset + entryLength <= this.tlvData.length) {
                const entryData = this.tlvData.subarray(offset, offset + entryLength);
                entries.set(entryType, entryData);
            }
            offset += entryLength;
        }
        return entries;
    }
}
