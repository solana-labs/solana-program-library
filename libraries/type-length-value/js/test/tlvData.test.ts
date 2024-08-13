import type { LengthSize } from '../src/tlvState';
import { TlvState } from '../src/tlvState';
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
        Buffer.from([0]),
        Buffer.from([3, 0]),
        Buffer.from([1, 2, 3]),
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
        Buffer.from([0, 0]),
        Buffer.from([3]),
        Buffer.from([1, 2, 3]),
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
        Buffer.from([0, 0, 0, 0]),
        Buffer.from([3, 0, 0, 0, 0, 0, 0, 0]),
        Buffer.from([1, 2, 3]),
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
        Buffer.from([0, 0, 0, 0, 0, 0, 0, 0]),
        Buffer.from([3, 0, 0, 0]),
        Buffer.from([1, 2, 3]),
    ]);

    const testRawData = (tlvData: Buffer, discriminatorSize: number, lengthSize: LengthSize) => {
        const tlv = new TlvState(tlvData, discriminatorSize, lengthSize);
        expect(tlv.data).to.be.deep.equal(tlvData);
        const tlvWithOffset = new TlvState(tlvData, discriminatorSize, lengthSize, discriminatorSize + lengthSize);
        expect(tlvWithOffset.data).to.be.deep.equal(tlvData.subarray(discriminatorSize + lengthSize));
    };

    it('should get the raw tlv data', () => {
        testRawData(tlvData1, 1, 2);
        testRawData(tlvData2, 2, 1);
        testRawData(tlvData3, 4, 8);
        testRawData(tlvData4, 8, 4);
    });

    const testIndividualEntries = (tlvData: Buffer, discriminatorSize: number, lengthSize: LengthSize) => {
        const tlv = new TlvState(tlvData, discriminatorSize, lengthSize);

        const type = Buffer.alloc(discriminatorSize);
        type[0] = 0;
        expect(tlv.firstBytes(type)).to.be.deep.equal(Buffer.from([]));
        type[0] = 1;
        expect(tlv.firstBytes(type)).to.be.deep.equal(Buffer.from([1]));
        type[0] = 2;
        expect(tlv.firstBytes(type)).to.be.deep.equal(Buffer.from([1, 2]));
        type[0] = 3;
        expect(tlv.firstBytes(type)).to.equal(null);
    };

    it('should get the entries individually', () => {
        testIndividualEntries(tlvData1, 1, 2);
        testIndividualEntries(tlvData2, 2, 1);
        testIndividualEntries(tlvData3, 4, 8);
        testIndividualEntries(tlvData4, 8, 4);
    });

    const testRepeatingEntries = (tlvData: Buffer, discriminatorSize: number, lengthSize: LengthSize) => {
        const tlv = new TlvState(tlvData, discriminatorSize, lengthSize);

        const bufferDiscriminator = tlv.bytesRepeating(Buffer.alloc(discriminatorSize));
        expect(bufferDiscriminator).to.have.length(2);
        expect(bufferDiscriminator[0]).to.be.deep.equal(Buffer.from([]));
        expect(bufferDiscriminator[1]).to.be.deep.equal(Buffer.from([1, 2, 3]));

        const bufferDiscriminatorWithCount = tlv.bytesRepeating(Buffer.alloc(discriminatorSize), 1);
        expect(bufferDiscriminatorWithCount).to.have.length(1);
        expect(bufferDiscriminatorWithCount[0]).to.be.deep.equal(Buffer.from([]));
    };

    it('should get the repeating entries', () => {
        testRepeatingEntries(tlvData1, 1, 2);
        testRepeatingEntries(tlvData2, 2, 1);
        testRepeatingEntries(tlvData3, 4, 8);
        testRepeatingEntries(tlvData4, 8, 4);
    });

    const testDiscriminators = (tlvData: Buffer, discriminatorSize: number, lengthSize: LengthSize) => {
        const tlv = new TlvState(tlvData, discriminatorSize, lengthSize);
        const discriminators = tlv.discriminators();
        expect(discriminators).to.have.length(4);

        const type = Buffer.alloc(discriminatorSize);
        type[0] = 0;
        expect(discriminators[0]).to.be.deep.equal(type);
        type[0] = 1;
        expect(discriminators[1]).to.be.deep.equal(type);
        type[0] = 2;
        expect(discriminators[2]).to.be.deep.equal(type);
        type[0] = 0;
        expect(discriminators[3]).to.be.deep.equal(type);
    };

    it('should get the discriminators', () => {
        testDiscriminators(tlvData1, 1, 2);
        testDiscriminators(tlvData2, 2, 1);
        testDiscriminators(tlvData3, 4, 8);
        testDiscriminators(tlvData4, 8, 4);
    });
});
