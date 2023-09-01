import type { TLVNumberSize } from '../../src/extensions/tlvData';
import { TLVData } from '../../src/extensions/tlvData';
import { expect } from 'chai';

describe('tlvData', () => {
    // typeLength 1, lengthSize 2
    const tlvData1 = Buffer.concat([
        Buffer.from([0]),
        Buffer.from([0, 0]),
        Buffer.from([]),
        Buffer.from([1]),
        Buffer.from([1, 0]),
        Buffer.from([1]),
        Buffer.from([2]),
        Buffer.from([2, 0]),
        Buffer.from([1, 2]),
    ]);

    // typeLength 2, lengthSize 1
    const tlvData2 = Buffer.concat([
        Buffer.from([0, 0]),
        Buffer.from([0]),
        Buffer.from([]),
        Buffer.from([1, 0]),
        Buffer.from([1]),
        Buffer.from([1]),
        Buffer.from([2, 0]),
        Buffer.from([2]),
        Buffer.from([1, 2]),
    ]);

    // typeLength 4, lengthSize 8
    const tlvData3 = Buffer.concat([
        Buffer.from([0, 0, 0, 0]),
        Buffer.from([0, 0, 0, 0, 0, 0, 0, 0]),
        Buffer.from([]),
        Buffer.from([1, 0, 0, 0]),
        Buffer.from([1, 0, 0, 0, 0, 0, 0, 0]),
        Buffer.from([1]),
        Buffer.from([2, 0, 0, 0]),
        Buffer.from([2, 0, 0, 0, 0, 0, 0, 0]),
        Buffer.from([1, 2]),
    ]);

    // typeLength 8, lengthSize 4
    const tlvData4 = Buffer.concat([
        Buffer.from([0, 0, 0, 0, 0, 0, 0, 0]),
        Buffer.from([0, 0, 0, 0]),
        Buffer.from([]),
        Buffer.from([1, 0, 0, 0, 0, 0, 0, 0]),
        Buffer.from([1, 0, 0, 0]),
        Buffer.from([1]),
        Buffer.from([2, 0, 0, 0, 0, 0, 0, 0]),
        Buffer.from([2, 0, 0, 0]),
        Buffer.from([1, 2]),
    ]);

    const testRawData = (tlvData: Buffer, typeSize: TLVNumberSize, lengthSize: TLVNumberSize) => {
        const tlv = new TLVData(tlvData, typeSize, lengthSize);
        expect(tlv.data).to.be.deep.equal(tlvData);
        const tlvWithOffset = new TLVData(tlvData, typeSize, lengthSize, typeSize + lengthSize);
        expect(tlvWithOffset.data).to.be.deep.equal(tlvData.subarray(typeSize + lengthSize));
    };

    it('should get the raw tlv data', () => {
        testRawData(tlvData1, 1, 2);
        testRawData(tlvData2, 2, 1);
        testRawData(tlvData3, 4, 8);
        testRawData(tlvData4, 8, 4);
    });

    const testIndividualEntries = (tlvData: Buffer, typeSize: TLVNumberSize, lengthSize: TLVNumberSize) => {
        const tlv = new TLVData(tlvData, typeSize, lengthSize);
        expect(tlv.entry(Number(0))).to.be.deep.equal(Buffer.alloc(0));
        expect(tlv.entry(Number(1))).to.be.deep.equal(Buffer.from([1]));
        expect(tlv.entry(Number(2))).to.be.deep.equal(Buffer.from([1, 2]));
        expect(tlv.entry(Number(3))).to.be.null;
        expect(tlv.entry(BigInt(0))).to.be.deep.equal(Buffer.alloc(0));
        expect(tlv.entry(BigInt(1))).to.be.deep.equal(Buffer.from([1]));
        expect(tlv.entry(BigInt(2))).to.be.deep.equal(Buffer.from([1, 2]));
        expect(tlv.entry(BigInt(3))).to.be.null;

        const type = Buffer.alloc(typeSize);
        type[0] = 0;
        expect(tlv.entry(type)).to.be.deep.equal(Buffer.alloc(0));
        type[0] = 1;
        expect(tlv.entry(type)).to.be.deep.equal(Buffer.from([1]));
        type[0] = 2;
        expect(tlv.entry(type)).to.be.deep.equal(Buffer.from([1, 2]));
        type[0] = 3;
        expect(tlv.entry(type)).to.be.null;
    };

    it('should get the entries individually', () => {
        testIndividualEntries(tlvData1, 1, 2);
        testIndividualEntries(tlvData2, 2, 1);
        testIndividualEntries(tlvData3, 4, 8);
        testIndividualEntries(tlvData4, 8, 4);
    });

    const testEntries = (tlvData: Buffer, typeSize: TLVNumberSize, lengthSize: TLVNumberSize) => {
        const tlv = new TLVData(tlvData, typeSize, lengthSize);
        const entries = tlv.entries();
        expect(entries).to.have.length(3);
        expect(entries.get(BigInt(0))).to.be.deep.equal(Buffer.alloc(0));
        expect(entries.get(BigInt(1))).to.be.deep.equal(Buffer.from([1]));
        expect(entries.get(BigInt(2))).to.be.deep.equal(Buffer.from([1, 2]));
        expect(entries.get(BigInt(3))).to.be.undefined;
    };

    it('should get the entries', () => {
        testEntries(tlvData1, 1, 2);
        testEntries(tlvData2, 2, 1);
        testEntries(tlvData3, 4, 8);
        testEntries(tlvData4, 8, 4);
    });
});
