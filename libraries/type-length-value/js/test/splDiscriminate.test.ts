import { expect } from 'chai';
import { splDiscriminate } from '../src/splDiscriminate';
import { createHash } from 'crypto';

describe('splDiscrimintor', () => {
    const testVectors = [
        'hello',
        'this-is-a-test',
        'test-namespace:this-is-a-test',
        'test-namespace:this-is-a-test:with-a-longer-name',
    ];

    const testExpectedBytes = testVectors.map((x) => {
        return createHash('sha256').update(x).digest();
    });

    const testSplDiscriminator = (length: number) => {
        for (let i = 0; i < testVectors.length; i++) {
            const discriminator = splDiscriminate(testVectors[i], length);
            const expectedBytes = testExpectedBytes[i].subarray(0, length);
            expect(discriminator).to.have.length(length);
            expect(discriminator).to.deep.equal(expectedBytes);
        }
    };

    it('should produce the expected bytes', () => {
        testSplDiscriminator(8);
        testSplDiscriminator(4);
        testSplDiscriminator(2);
    });

    it('should produce the same bytes as rust library', () => {
        const expectedBytes = Buffer.from([105, 37, 101, 197, 75, 251, 102, 26]);
        const discriminator = splDiscriminate('spl-transfer-hook-interface:execute');
        expect(discriminator).to.deep.equal(expectedBytes);
    });
});
